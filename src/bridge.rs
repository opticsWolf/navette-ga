// SPDX-License-Identifier: LGPL-3.0-or-later
//! G1/G4 — fitness bridge and needle-memetic operator.
//!
//! The bridge turns a decoded genome into a navette `DesignStack` (plus any
//! environments), evaluates the joint merit, and returns the scalar fitness
//! with the per-target breakdown used by the QD descriptors. A full generation
//! is batched so stack setup is amortised.
//!
//! The memetic operator runs the needle pipeline over a genome: program-level
//! (`run_needle`) first, then the staged A1 API (`scan/insert/polish/cleanup`)
//! once it lands. Lamarckian write-back is the default; the Baldwinian variant
//! is kept for the V21 ablation.

use std::sync::Arc;

use navette::lsq::backends::LsqResult;
use navette::lsq::lm::LmConfig;
use navette::smatrix::angle::Radians;
use navette::smatrix::synthesis::config::ThinLayerPolicy;
use navette::smatrix::synthesis::context::DesignContext;
use navette::smatrix::synthesis::cycle::{
    InsertRules, Insertion, MaterialPool, NeedleCycleConfig, NeedleRules,
};
use navette::smatrix::synthesis::environments::{CompiledEnvironments, Gap};
use navette::smatrix::synthesis::evaluator::SmatrixContext;
use navette::smatrix::synthesis::merit::{MeritSpec, SimCurves};
use navette::smatrix::synthesis::pipeline::SpectralInputs;
use navette::smatrix::synthesis::staged::{PolishReport, PolishScope};
use navette::smatrix::synthesis::structure::{ClampReport, DesignStack, EdgeMaterials, LayerSpec};

use crate::decoder::{AdjacencyGrammar, DecodedStack, MaterialPool as DecoderPool, Pins};

/// May a needle of pool entry `needle` sit on the joint boundary locus
/// `gap` of a multi-environment design?
///
/// This is the public door to navette's joint rule: the same host,
/// neighbour, edge-list, and interface-slice checks used by the needle
/// sweep, applied to every environment's placement of the locus. `stacks`
/// must be the K expanded assemblies from `CompiledEnvironments::expand`.
/// The query is read-only; it never inserts or moves a film.
pub fn joint_gap_accepts(
    envs: &CompiledEnvironments,
    stacks: &[DesignStack],
    pool: &MaterialPool,
    gap: Gap,
    needle: usize,
) -> Result<bool, String> {
    pool.gap_accepts_needle(envs, stacks, gap, needle)
}

/// One accepted joint-boundary needle site: a locus and a pool entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JointGapSite {
    pub gap: Gap,
    pub needle: usize,
}

/// Enumerate every pool entry accepted at every joint boundary locus.
///
/// This is the structural half of a joint boundary scan: it applies the
/// same host, neighbour, edge-list, and interface-slice rule as the
/// single-environment staged scan, but over every environment's placement
/// and without a spectral gain estimate. Use it to build a candidate list
/// for [`joint_gap_insert`] or to audit a rule set before a run.
pub fn joint_gap_sites(
    envs: &CompiledEnvironments,
    stacks: &[DesignStack],
    pool: &MaterialPool,
) -> Result<Vec<JointGapSite>, String> {
    let mut sites = Vec::new();
    for gap in envs.gaps() {
        for needle in 0..pool.len() {
            if pool.gap_accepts_needle(envs, stacks, gap, needle)? {
                sites.push(JointGapSite { gap, needle });
            }
        }
    }
    Ok(sites)
}

/// Insert a needle at a joint boundary locus and return the new slot plus
/// the K expanded assemblies.
///
/// The acceptance query runs first on the current design, so a film the
/// joint rule refuses is never inserted. On success the film is inserted
/// into every environment's template and the primary design stack, as a
/// new shared design slot; the routing table shifts with it. The caller
/// must pass a `design` that is the primary (environment 0) stack of
/// `envs`.
pub fn joint_gap_insert(
    envs: &mut CompiledEnvironments,
    design: &mut DesignStack,
    pool: &MaterialPool,
    gap: Gap,
    needle: usize,
    d_nm: f64,
) -> Result<(usize, Vec<DesignStack>), String> {
    if !(d_nm.is_finite() && d_nm > 0.0) {
        return Err(format!(
            "joint gap insertion: needle thickness must be positive and finite (got {d_nm} nm)"
        ));
    }
    let current = envs.expand(design)?;
    if !joint_gap_accepts(envs, &current, pool, gap, needle)? {
        return Err(format!(
            "joint gap insertion: pool entry {needle:?} is not accepted at gap {gap:?}"
        ));
    }
    let mut film = pool[needle].clone();
    film.d_nm = d_nm;
    film.coherent = true;
    film.rough_type = 0;
    film.rough_val = 0.0;
    film.optimize = true;
    film.needle = true;
    // Both objects are replaced only after the design and every template
    // have accepted the film; a slice or routing refusal leaves the caller
    // with the original state.
    let mut trial_envs = envs.clone();
    let mut trial_design = design.clone();
    trial_envs.boundary_design(&mut trial_design, gap, &film)?;
    let slot = trial_envs.insert_boundary(gap, &film)?;
    let stacks = trial_envs.expand(&trial_design)?;
    *envs = trial_envs;
    *design = trial_design;
    Ok((slot, stacks))
}

/// A material-boundary contract for a stack template.
///
/// Reuses navette's `InsertRules` (host label -> accepted needle labels) and
/// its outer/inner edge lists. `seed_edges` is deliberately absent: it inserts
/// a film and changes the decoded layer count, so this bridge uses decoder pins
/// for single-material edge lists and assembly-time validation for the rest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BoundaryRules {
    /// Host label -> the labels of needles it accepts. `None`: unrestricted.
    pub insert: Option<InsertRules>,
    /// Allowed materials (template labels) of the ambient-side film.
    pub outer: Option<Vec<String>>,
    /// Allowed materials (template labels) of the substrate-side film.
    pub inner: Option<Vec<String>>,
}

impl BoundaryRules {
    /// No insert or edge restriction.
    pub fn is_free(&self) -> bool {
        self.insert.is_none() && self.outer.is_none() && self.inner.is_none()
    }

    /// Reuse a navette [`NeedleRules`] value. Edge seeding is refused here;
    /// it inserts a film and changes the decoded layer count, so the bridge
    /// uses decoder pins for a single-material edge list.
    pub fn from_needle_rules(rules: NeedleRules) -> Result<Self, String> {
        if rules.seed_edges.is_some() {
            return Err(
                "boundary rules: seed_edges is not supported; seeding inserts a film \
                 and changes the decoded layer count. Use a single-material edge list \
                 as decoder pins, or let assemble validate the initial edge"
                    .into(),
            );
        }
        Ok(Self {
            insert: rules.insert,
            outer: rules.outer,
            inner: rules.inner,
        })
    }

    /// Resolve every label and reject structurally impossible rules before a
    /// template is built. An empty edge list is rejected because no film can
    /// satisfy it.
    pub fn validate(&self, materials: &[LayerSpec]) -> Result<(), String> {
        if self.is_free() {
            return Ok(());
        }
        if self
            .insert
            .as_ref()
            .is_some_and(|rules| rules.contains_key("*"))
        {
            return Err(
                "boundary rules: the \"*\" host is not supported by StackTemplate; \
                 name each host material explicitly"
                    .into(),
            );
        }
        for (which, list) in [("outer", &self.outer), ("inner", &self.inner)] {
            if list.as_ref().is_some_and(|l| l.is_empty()) {
                return Err(format!(
                    "boundary rules: the {which} edge list is empty; \
                     omit it or name at least one material"
                ));
            }
        }
        let pool = MaterialPool::from(materials.to_vec());
        if let Some(rules) = &self.insert {
            pool.clone().with_insert_rules(rules)?;
        }
        if self.outer.is_some() || self.inner.is_some() {
            pool.edge_materials(self.outer.as_deref(), self.inner.as_deref())?;
        }
        Ok(())
    }
}

/// A material pool plus the fixed ambient/substrate half-spaces.
///
/// `materials[i]` is the optical template for decoder material index `i`.
/// The template's thickness is ignored; every assembled film takes its
/// thickness from the decoded layer. `optimize` and `needle` are template
/// defaults for unpinned films; pinned films always have both switched off.
#[derive(Clone, Debug)]
pub struct StackTemplate {
    pub ambient: LayerSpec,
    pub substrate: LayerSpec,
    pub materials: Vec<LayerSpec>,
    boundary: BoundaryRules,
}

impl StackTemplate {
    pub fn new(
        mut ambient: LayerSpec,
        mut substrate: LayerSpec,
        materials: Vec<LayerSpec>,
    ) -> Result<Self, String> {
        if materials.is_empty() {
            return Err("stack template needs at least one material".into());
        }
        let num_wavs = ambient.nk.len();
        if num_wavs == 0 {
            return Err("ambient nk is empty".into());
        }
        if substrate.nk.len() != num_wavs {
            return Err(format!(
                "substrate nk length {} != ambient nk length {}",
                substrate.nk.len(),
                num_wavs
            ));
        }
        for (index, material) in materials.iter().enumerate() {
            if material.nk.len() != num_wavs {
                return Err(format!(
                    "material {index} ({:?}) nk length {} != ambient nk length {}",
                    material.material,
                    material.nk.len(),
                    num_wavs
                ));
            }
        }
        ambient.optimize = false;
        ambient.needle = false;
        substrate.optimize = false;
        substrate.needle = false;
        Ok(Self {
            ambient,
            substrate,
            materials,
            boundary: BoundaryRules::default(),
        })
    }

    /// Attach validated material-boundary rules.
    pub fn with_boundary_rules(mut self, rules: BoundaryRules) -> Result<Self, String> {
        rules.validate(&self.materials)?;
        self.boundary = rules;
        Ok(self)
    }

    /// Attach the rule subset of navette's [`NeedleRules`]. `seed_edges` is
    /// refused; see [`BoundaryRules::from_needle_rules`].
    pub fn with_needle_rules(self, rules: NeedleRules) -> Result<Self, String> {
        self.with_boundary_rules(BoundaryRules::from_needle_rules(rules)?)
    }

    pub fn boundary_rules(&self) -> &BoundaryRules {
        &self.boundary
    }

    /// Resolve the configured edge lists against the template materials.
    pub fn edge_materials(&self) -> Result<EdgeMaterials, String> {
        let pool = MaterialPool::from(self.materials.clone());
        pool.edge_materials(
            self.boundary.outer.as_deref(),
            self.boundary.inner.as_deref(),
        )
    }

    /// Turn single-material edge lists into decoder pins, so a GA starts with
    /// compliant edges. A list naming several materials cannot be pinned; that
    /// edge returns `None` and is still validated by [`Self::assemble`].
    pub fn edge_pins(&self, decoder: &DecoderPool) -> Result<Pins, String> {
        let resolve = |list: &Option<Vec<String>>, which: &str| -> Result<Option<usize>, String> {
            let Some(list) = list else {
                return Ok(None);
            };
            if list.len() != 1 {
                return Ok(None);
            }
            let name = &list[0];
            let mut hits = decoder
                .materials()
                .iter()
                .enumerate()
                .filter(|(_, material)| material.name() == name)
                .map(|(index, _)| index);
            match (hits.next(), hits.next()) {
                (Some(index), None) => Ok(Some(index)),
                (None, _) => Err(format!(
                    "edge pins: the {which} list names {name:?}, which is not a decoder material"
                )),
                (Some(_), Some(_)) => Err(format!(
                    "edge pins: the decoder has more than one material named {name:?}; names must be unique"
                )),
            }
        };
        Ok(Pins::with_edges(
            resolve(&self.boundary.outer, "outer")?,
            resolve(&self.boundary.inner, "inner")?,
        ))
    }

    /// Check the host→needle rules against the decoder's adjacency grammar.
    ///
    /// A split needle inside host `H` produces `H | N | H`, so an allowed
    /// `H → N` pair needs both `H → N` and `N → H` in the grammar. This is a
    /// consistency check, not a derivation: the grammar remains the decoder's
    /// single source of adjacency truth. The `"*"` rule is skipped because it
    /// names no concrete host.
    pub fn check_grammar(
        &self,
        decoder: &DecoderPool,
        grammar: &AdjacencyGrammar,
    ) -> Result<(), String> {
        let Some(rules) = &self.boundary.insert else {
            return Ok(());
        };
        let lookup = |label: &str, which: &str| -> Result<usize, String> {
            let mut hits = decoder
                .materials()
                .iter()
                .enumerate()
                .filter(|(_, material)| material.name() == label)
                .map(|(index, _)| index);
            match (hits.next(), hits.next()) {
                (Some(index), None) => Ok(index),
                (None, _) => Err(format!(
                    "insert/grammar check: {which} {label:?} is not a decoder material"
                )),
                (Some(_), Some(_)) => Err(format!(
                    "insert/grammar check: the decoder has more than one material named \
                     {label:?}; names must be unique"
                )),
            }
        };
        for (host, needles) in rules {
            let h = lookup(host, "host")?;
            for needle in needles {
                let n = lookup(needle, "needle")?;
                if !grammar.allows(h, n) || !grammar.allows(n, h) {
                    return Err(format!(
                        "insert/grammar conflict: {host:?} accepts {needle:?}, but the \
                         adjacency grammar does not allow both directions; a split needle \
                         creates {host} | {needle} | {host}"
                    ));
                }
            }
        }
        Ok(())
    }

    /// Build a navette stack from decoded layers.
    pub fn assemble(&self, decoded: &DecodedStack) -> Result<DesignStack, String> {
        let mut films = Vec::with_capacity(decoded.len());
        for (index, layer) in decoded.layers().iter().enumerate() {
            let mut film = self
                .materials
                .get(layer.material)
                .ok_or_else(|| {
                    format!(
                        "decoded layer {index}: material index {} is outside the template",
                        layer.material
                    )
                })?
                .clone();
            film.d_nm = layer.thickness_nm;
            film.optimize = film.optimize && !layer.pinned;
            film.needle = film.needle && !layer.pinned;
            // The template material is a pool entry; its `material` is the
            // label insert/edge rules resolve. Keep the assembled film on that
            // label even if the template carried a custom display label, so a
            // host never falls back to ambiguous index matching.
            film.label = Some(Arc::from(film.material.as_ref()));
            films.push(film);
        }
        let mut stack =
            DesignStack::with_films(self.ambient.clone(), self.substrate.clone(), films)?;
        if self.boundary.outer.is_some() || self.boundary.inner.is_some() {
            stack.set_edge_materials(self.edge_materials()?)?;
        }
        Ok(stack)
    }

    /// The template materials as a needle pool with the insert rules applied.
    ///
    /// Pool entries are the same `LayerSpec`s; their thickness and flags are
    /// ignored by the needle pass, which only reads material label, nk and
    /// label/insert rules.
    pub fn needle_pool(&self) -> Result<MaterialPool, String> {
        let pool = MaterialPool::from(self.materials.clone());
        match &self.boundary.insert {
            Some(rules) => pool.with_insert_rules(rules),
            None => Ok(pool),
        }
    }
}

/// Context knobs for the fitness evaluator.
#[derive(Clone, Debug)]
pub struct EvaluatorOptions {
    pub clamp_min_nm: f64,
    pub clamp_max_nm: f64,
    pub lm: LmConfig,
    pub thin_layer_policy: ThinLayerPolicy,
}

impl Default for EvaluatorOptions {
    fn default() -> Self {
        Self {
            clamp_min_nm: 2.0,
            clamp_max_nm: 1000.0,
            lm: LmConfig::default(),
            thin_layer_policy: ThinLayerPolicy::Remove,
        }
    }
}

/// A fixed spectral/angular problem plus the navette synthesis context.
///
/// One evaluator is reused for a whole GA run. `evaluate` is pure (no stack
/// mutation); `refine` is the LM finish and mutates the stack in place.
#[derive(Clone)]
pub struct FitnessEvaluator {
    ctx: SmatrixContext,
}

impl FitnessEvaluator {
    pub fn new(
        wavelengths: Vec<f64>,
        angles_deg: &[f64],
        spec: MeritSpec,
        options: EvaluatorOptions,
    ) -> Result<Self, String> {
        if wavelengths.is_empty() {
            return Err("wavelength grid is empty".into());
        }
        for (index, wavelength) in wavelengths.iter().enumerate() {
            if !wavelength.is_finite() || *wavelength <= 0.0 {
                return Err(format!(
                    "wavelength[{index}] = {wavelength} is not finite and positive"
                ));
            }
            if index > 0 && *wavelength <= wavelengths[index - 1] {
                return Err(format!(
                    "wavelength grid must be strictly increasing at index {index}: \
                     {} then {wavelength}",
                    wavelengths[index - 1]
                ));
            }
        }
        if angles_deg.is_empty() {
            return Err("angle grid is empty".into());
        }
        let angles: Vec<Radians> = angles_deg
            .iter()
            .map(|&angle| Radians::from_degrees(angle))
            .collect::<Result<_, _>>()?;

        if !options.clamp_min_nm.is_finite()
            || !options.clamp_max_nm.is_finite()
            || options.clamp_min_nm < 0.0
            || options.clamp_max_nm < options.clamp_min_nm
        {
            return Err(format!(
                "invalid clamp bounds [{}, {}] nm",
                options.clamp_min_nm, options.clamp_max_nm
            ));
        }

        // Refuse unsupported demands before any solve, not after a silent
        // missing-curve penalty.
        SmatrixContext::check_spec(&spec)?;

        let sin_theta = angles.iter().map(|angle| angle.sin()).collect();
        Ok(Self {
            ctx: SmatrixContext {
                wavls: wavelengths,
                sin_theta,
                spec,
                clamp_min_nm: options.clamp_min_nm,
                clamp_max_nm: options.clamp_max_nm,
                thin_layer_policy: options.thin_layer_policy,
                lm: options.lm,
                clamp_accumulator: ClampReport::default(),
                envs: None,
            },
        })
    }

    /// Scalar joint merit of one stack.
    pub fn evaluate(&self, stack: &DesignStack) -> Result<f64, String> {
        self.ctx.evaluate_merit(stack)
    }

    /// Evaluate a generation in input order through navette's A3 batch door.
    ///
    /// A3 lives on navette's `dev_ga` branch and is compiled in by the local
    /// path dependency. The call fans the population out through Rayon while
    /// each solve keeps its own grid parallelism. A published navette release
    /// containing A3 is required before this crate can be published; see the
    /// release-gate note on the navette dependency in `Cargo.toml`.
    pub fn evaluate_batch(&self, stacks: &[DesignStack]) -> Result<Vec<f64>, String> {
        self.ctx.evaluate_many(stacks)
    }

    /// Bounded LM thickness finish, in place. Returns the post-finish merit.
    pub fn refine(&mut self, stack: &mut DesignStack) -> Result<f64, String> {
        self.ctx.refine_thicknesses(stack)
    }

    /// Bounded LM finish plus the solver's termination report.
    ///
    /// `None` means there was nothing to optimize (no free films); otherwise
    /// `LsqResult::termination` says whether the run converged or hit a
    /// budget/stall. This is the certificate hook for a terminal memetic step.
    pub fn refine_report(
        &mut self,
        stack: &mut DesignStack,
    ) -> Result<(f64, Option<LsqResult>), String> {
        self.ctx.refine_thicknesses_report(stack)
    }

    pub fn simulate(&self, stack: &DesignStack) -> Result<SimCurves, String> {
        self.ctx.simulate(stack)
    }

    pub fn context(&self) -> &SmatrixContext {
        &self.ctx
    }

    pub fn context_mut(&mut self) -> &mut SmatrixContext {
        &mut self.ctx
    }
}

/// G4(b): the A1 needle stages as a memetic operator.
///
/// One step is: scan the current stack, insert the best improving candidate,
/// then run a bounded LM polish. The operator never runs the macro-loop's
/// internal insertion cycle; it is the composable version the GA calls.
#[derive(Clone, Debug)]
pub struct MemeticOperator {
    pub spectral: SpectralInputs,
    pub pool: MaterialPool,
    pub cfg: NeedleCycleConfig,
    /// Which free films the polish step may move.
    pub polish: PolishScope,
}

/// One insertion and its polish.
#[derive(Clone, Debug)]
pub struct MemeticStep {
    pub insertion: Insertion,
    pub merit_after_insert: f64,
    pub polish: PolishReport,
}

/// What a memetic call did.
#[derive(Clone, Debug, Default)]
pub struct MemeticReport {
    pub initial_merit: f64,
    pub final_merit: f64,
    pub scans: usize,
    pub steps: Vec<MemeticStep>,
}

impl MemeticOperator {
    pub fn new(spectral: SpectralInputs, pool: MaterialPool, cfg: NeedleCycleConfig) -> Self {
        Self {
            spectral,
            pool,
            cfg,
            polish: PolishScope::All,
        }
    }

    /// Build the operator from a template, so the template's insert rules are
    /// applied to the needle pool. Equivalent to `new` with
    /// [`StackTemplate::needle_pool`].
    pub fn from_template(
        spectral: SpectralInputs,
        template: &StackTemplate,
        cfg: NeedleCycleConfig,
    ) -> Result<Self, String> {
        Ok(Self::new(spectral, template.needle_pool()?, cfg))
    }

    /// Scan the current stack through the A1 stage.
    pub fn scan(
        &self,
        evaluator: &FitnessEvaluator,
        stack: &DesignStack,
    ) -> Result<navette::smatrix::synthesis::staged::NeedleScan, String> {
        stack.needle_scan(evaluator.context(), &self.spectral, &self.pool, &self.cfg)
    }

    /// Apply at most `max_insertions` scan/insert/polish steps.
    ///
    /// A candidate is accepted only after a trial insertion and a real merit
    /// evaluation shows an improvement. The analytic P sweep is a predictor;
    /// accepting it untested could make the stack worse when several
    /// candidates interact. The first candidate in scan order that passes the
    /// trial is inserted; if none passes, the loop stops.
    pub fn apply(
        &self,
        evaluator: &mut FitnessEvaluator,
        stack: &mut DesignStack,
        max_insertions: usize,
    ) -> Result<MemeticReport, String> {
        let initial_merit = evaluator.evaluate(stack)?;
        let mut report = MemeticReport {
            initial_merit,
            final_merit: initial_merit,
            ..MemeticReport::default()
        };

        for _ in 0..max_insertions {
            let scan =
                stack.needle_scan(evaluator.context(), &self.spectral, &self.pool, &self.cfg)?;
            report.scans += 1;
            let mut accepted = None;
            for candidate in &scan.candidates {
                if !candidate.is_improving() {
                    break;
                }
                let mut trial = stack.clone();
                trial.insert_needle_candidate(&self.pool, candidate)?;
                let merit = evaluator.evaluate(&trial)?;
                if merit < scan.current_merit {
                    accepted = Some((candidate.clone(), merit));
                    break;
                }
            }
            let Some((candidate, merit_after_insert)) = accepted else {
                break;
            };
            let insertion = stack.insert_needle_candidate(&self.pool, &candidate)?;
            let polish = stack.polish_needles(evaluator.context_mut(), self.polish.clone())?;
            report.steps.push(MemeticStep {
                insertion,
                merit_after_insert,
                polish,
            });
        }

        report.final_merit = evaluator.evaluate(stack)?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use navette::smatrix::synthesis::design_config::{
        DesignRequest, LayerRow, MaterialDef, StructureCfg, TabData,
    };
    use navette::smatrix::synthesis::environments::{
        build_environments, DesignSegmentCfg, EnvSegmentCfg, EnvironmentCfg, FixedLayerRow,
    };
    use navette::smatrix::synthesis::merit::{
        ConstraintKind, CurveId, MeritKey, MeritTarget, SimTransform,
    };

    use crate::decoder::{
        AdjacencyGrammar, Decoder, Genome, LayerGene, Material, MaterialPool, Pins,
    };

    fn constant(name: &str, n: f64, d_nm: f64, num_wavs: usize) -> LayerSpec {
        LayerSpec::constant(name, n, 0.0, d_nm, num_wavs)
    }

    fn fixed_stack(films: Vec<LayerSpec>, num_wavs: usize) -> DesignStack {
        let mut ambient = constant("air", 1.0, 0.0, num_wavs);
        ambient.optimize = false;
        ambient.needle = false;
        let mut substrate = constant("substrate", 1.52, 0.0, num_wavs);
        substrate.optimize = false;
        substrate.needle = false;
        DesignStack::with_films(ambient, substrate, films).unwrap()
    }

    fn zero_reflectance_spec(angle_deg: f64, wavelength: f64) -> MeritSpec {
        let mut spec = MeritSpec::new();
        let key = spec.add_key(MeritKey {
            angle: Radians::from_degrees(angle_deg).unwrap(),
            curve: CurveId::Rs,
        });
        spec.add_target(MeritTarget {
            key_idx: key as u32,
            env_idx: 0,
            wavelengths: vec![wavelength].into(),
            kind: ConstraintKind::Exact,
            transform: SimTransform::Linear,
            norm_factor: 1.0,
            normalized_targets: vec![0.0].into(),
            tolerances: vec![1e-3].into(),
            band: Vec::new().into(),
            phase: false,
            differential_passes: None,
            retardance: false,
            ratio: None,
            dispersion: None,
            stokes: None,
            point_weights: Vec::new().into(),
            integral: false,
            weight: 1.0,
            count_norm: None,
        })
        .unwrap();
        spec
    }

    fn evaluator(spec: MeritSpec) -> FitnessEvaluator {
        FitnessEvaluator::new(vec![550.0], &[0.0], spec, EvaluatorOptions::default()).unwrap()
    }

    fn reflectance(stack: &DesignStack) -> f64 {
        let ev = evaluator(zero_reflectance_spec(0.0, 550.0));
        let sim = ev.simulate(stack).unwrap();
        sim.curve(CurveId::Rs).unwrap()[0]
    }

    #[test]
    fn quarter_wave_layer_is_an_ideal_ar_at_design_wavelength() {
        let n_substrate = 1.52_f64;
        let n_film = n_substrate.sqrt();
        let d_nm = 550.0 / (4.0 * n_film);
        let stack = fixed_stack(vec![constant("film", n_film, d_nm, 1)], 1);
        let ev = evaluator(zero_reflectance_spec(0.0, 550.0));

        assert!(reflectance(&stack) < 1e-24);
        assert!(ev.evaluate(&stack).unwrap() < 1e-24);
    }

    #[test]
    fn two_layer_v_coat_is_zero_at_design_wavelength() {
        // Two quarter-wave layers satisfy n1^2 * n_substrate / n2^2 = 1.
        let n_substrate = 1.52_f64;
        let n1 = 1.38;
        let n2 = n1 * n_substrate.sqrt();
        let d1 = 550.0 / (4.0 * n1);
        let d2 = 550.0 / (4.0 * n2);
        let stack = fixed_stack(
            vec![constant("n1", n1, d1, 1), constant("n2", n2, d2, 1)],
            1,
        );

        assert!(reflectance(&stack) < 1e-24);
    }

    #[test]
    fn template_and_decoder_assemble_the_same_design() {
        let n_substrate = 1.52_f64;
        let n_film = n_substrate.sqrt();
        let d_nm = 550.0 / (4.0 * n_film);

        let pool = MaterialPool::new(vec![Material::new("film", d_nm, d_nm).unwrap()]).unwrap();
        let decoder = Decoder::without_constraints(pool).unwrap();
        let genome = Genome::from(vec![LayerGene::new(0.2, 0.9, 0.9)]);
        let decoded = decoder.decode(&genome).unwrap();
        assert_eq!(decoded.len(), 1);

        let template = StackTemplate::new(
            constant("air", 1.0, 0.0, 1),
            constant("substrate", n_substrate, 0.0, 1),
            vec![constant("film", n_film, 0.0, 1)],
        )
        .unwrap();
        let stack = template.assemble(&decoded).unwrap();

        assert!((stack.films()[0].d_nm - d_nm).abs() < 1e-12);
        assert!(reflectance(&stack) < 1e-24);
    }

    fn joint_row(code: &str, d_nm: f64) -> LayerRow {
        LayerRow {
            material_code: code.to_string(),
            thickness_nm: d_nm,
            coherent: true,
            roughness_nm: 0.0,
            rough_type: 0,
            inhomogen: false,
            inh_delta: 0.1,
            interface: false,
            interface_thickness_nm: 0.0,
            optimize: true,
            needle: true,
            layer_type: 1,
            gradient: None,
        }
    }

    fn joint_material(name: &str, n: f64) -> MaterialDef {
        MaterialDef {
            name: name.to_string(),
            code: None,
            model: "TableMaterial".to_string(),
            params: BTreeMap::new(),
            n_data: Some(TabData {
                wavelengths: vec![500.0, 600.0],
                values: vec![n, n],
            }),
            k_data: None,
        }
    }

    fn joint_fixed_row(code: &str, d_nm: f64) -> FixedLayerRow {
        FixedLayerRow {
            material_code: code.to_string(),
            thickness_nm: d_nm,
            coherent: true,
            roughness_nm: 0.0,
            rough_type: 0,
            inhomogen: false,
            inh_delta: 0.1,
            interface: false,
            interface_thickness_nm: 0.0,
            optimize: false,
            needle: false,
            layer_type: 1,
            gradient: None,
        }
    }

    fn joint_design_segment() -> EnvSegmentCfg {
        EnvSegmentCfg {
            layers: None,
            design: Some("coat".to_string()),
            repeat: None,
            inverted: false,
        }
    }

    fn joint_fixed_segment(code: &str, d_nm: f64) -> EnvSegmentCfg {
        EnvSegmentCfg {
            layers: Some(vec![joint_fixed_row(code, d_nm)]),
            design: None,
            repeat: None,
            inverted: false,
        }
    }

    #[test]
    fn joint_gap_accepts_checks_environments_and_edge_lists() {
        let insert: InsertRules = [
            ("H".to_string(), vec!["G".to_string()]),
            ("L".to_string(), vec!["G".to_string()]),
        ]
        .into_iter()
        .collect();
        let request = DesignRequest {
            structure: StructureCfg {
                label: "joint-gap".to_string(),
                layers: Vec::new(),
                groups: Vec::new(),
            },
            library: vec![
                joint_material("H", 2.2),
                joint_material("L", 1.46),
                joint_material("G", 1.8),
            ],
            materials: Some(vec!["H".to_string(), "L".to_string(), "G".to_string()]),
            insert_rules: Some(insert),
            outer: None,
            inner: None,
            seed_edges: None,
            film_flags: BTreeMap::new(),
            per_film_flags: BTreeMap::new(),
            ambient_name: "air".to_string(),
            substrate_name: "sub".to_string(),
            design: [(
                "coat".to_string(),
                DesignSegmentCfg {
                    layers: vec![joint_row("H", 80.0), joint_row("L", 100.0)],
                    outer: Some(vec!["H".to_string()]),
                    inner: Some(vec!["L".to_string()]),
                },
            )]
            .into_iter()
            .collect(),
            environments: vec![
                EnvironmentCfg {
                    name: "front".to_string(),
                    stack: vec![joint_design_segment()],
                },
                EnvironmentCfg {
                    name: "back".to_string(),
                    stack: vec![joint_fixed_segment("H", 300.0), joint_design_segment()],
                },
            ],
        };
        let (mut envs, pool, warnings) = build_environments(&request, &[550.0]).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(envs.n_envs(), 2);
        let stacks = envs.expand(&envs.stacks()[0]).unwrap();
        assert_eq!(stacks.len(), 2);
        assert_eq!(stacks[0].films().len(), 2);
        assert_eq!(stacks[1].films().len(), 3);
        let g = pool
            .iter()
            .position(|entry| entry.material.as_ref() == "G")
            .unwrap();
        let thicknesses: Vec<Vec<f64>> = stacks
            .iter()
            .map(|stack| stack.films().iter().map(|film| film.d_nm).collect())
            .collect();

        // The top edge is H, the bottom L, and G is on neither list.
        assert!(!joint_gap_accepts(&envs, &stacks, &pool, Gap { seg: 0, k: 0 }, g).unwrap());
        // Between H and L both hosts accept G, in every environment.
        assert!(joint_gap_accepts(&envs, &stacks, &pool, Gap { seg: 0, k: 1 }, g).unwrap());
        assert!(!joint_gap_accepts(&envs, &stacks, &pool, Gap { seg: 0, k: 2 }, g).unwrap());

        let sites = joint_gap_sites(&envs, &stacks, &pool).unwrap();
        assert!(sites.contains(&JointGapSite {
            gap: Gap { seg: 0, k: 1 },
            needle: g,
        }));
        assert!(!sites.iter().any(|site| site.needle == g
            && (site.gap == Gap { seg: 0, k: 0 } || site.gap == Gap { seg: 0, k: 2 })));

        // A multi-material edge list is a set of legal edge films, not a
        // pin: G joins each list and is accepted at both segment edges.
        let mut multi = request.clone();
        let coat = multi.design.get_mut("coat").unwrap();
        coat.outer = Some(vec!["H".to_string(), "G".to_string()]);
        coat.inner = Some(vec!["L".to_string(), "G".to_string()]);
        let (mut multi_envs, multi_pool, _) = build_environments(&multi, &[550.0]).unwrap();
        let multi_stacks = multi_envs.expand(&multi_envs.stacks()[0]).unwrap();
        let multi_g = multi_pool
            .iter()
            .position(|entry| entry.material.as_ref() == "G")
            .unwrap();
        for gap in [Gap { seg: 0, k: 0 }, Gap { seg: 0, k: 2 }] {
            assert!(
                joint_gap_accepts(&multi_envs, &multi_stacks, &multi_pool, gap, multi_g).unwrap()
            );
        }
        let mut multi_design = multi_envs.stacks()[0].clone();
        let (slot, inserted) = joint_gap_insert(
            &mut multi_envs,
            &mut multi_design,
            &multi_pool,
            Gap { seg: 0, k: 0 },
            multi_g,
            15.0,
        )
        .unwrap();
        assert_eq!(slot, 0);
        assert_eq!(inserted[0].films()[0].material.as_ref(), "G");
        assert_eq!(inserted[0].films()[0].d_nm, 15.0);
        assert_eq!(inserted[1].films()[0].d_nm, 300.0);
        assert_eq!(inserted[1].films()[1].material.as_ref(), "G");
        assert_eq!(inserted[1].films()[1].d_nm, 15.0);

        let after: Vec<Vec<f64>> = stacks
            .iter()
            .map(|stack| stack.films().iter().map(|film| film.d_nm).collect())
            .collect();
        assert_eq!(after, thicknesses, "a gap query must not move a film");

        let mut design = envs.stacks()[0].clone();
        let mut refused_envs = envs.clone();
        let mut refused_design = design.clone();
        let refused_gap = Gap { seg: 0, k: 0 };
        assert!(joint_gap_insert(
            &mut refused_envs,
            &mut refused_design,
            &pool,
            refused_gap,
            g,
            25.0
        )
        .is_err());
        assert_eq!(refused_design.films().len(), 2);
        assert_eq!(refused_envs.gaps(), envs.gaps());

        let insertion_gap = Gap { seg: 0, k: 1 };
        let (slot, inserted) =
            joint_gap_insert(&mut envs, &mut design, &pool, insertion_gap, g, 25.0).unwrap();
        assert_eq!(slot, 1);
        assert_eq!(inserted.len(), 2);
        assert_eq!(inserted[0].films().len(), 3);
        assert_eq!(inserted[0].films()[1].d_nm, 25.0);
        assert_eq!(inserted[0].films()[1].material.as_ref(), "G");
        assert_eq!(inserted[1].films().len(), 4);
        assert_eq!(inserted[1].films()[2].d_nm, 25.0);
        assert_eq!(inserted[1].films()[2].material.as_ref(), "G");
        assert!(!joint_gap_accepts(&envs, &inserted, &pool, insertion_gap, g).unwrap());
    }

    #[test]
    fn boundary_rules_reject_bad_labels_and_seed_edges() {
        let template = || {
            StackTemplate::new(
                constant("air", 1.0, 0.0, 1),
                constant("substrate", 1.52, 0.0, 1),
                vec![constant("H", 2.2, 0.0, 1), constant("L", 1.46, 0.0, 1)],
            )
            .unwrap()
        };
        let bad_insert = BoundaryRules {
            insert: Some(
                [("H".to_string(), vec!["Z".to_string()])]
                    .into_iter()
                    .collect(),
            ),
            ..BoundaryRules::default()
        };
        assert!(template().with_boundary_rules(bad_insert).is_err());

        let bad_edge = BoundaryRules {
            outer: Some(vec!["Z".to_string()]),
            ..BoundaryRules::default()
        };
        assert!(template().with_boundary_rules(bad_edge).is_err());

        let empty_edge = BoundaryRules {
            outer: Some(Vec::new()),
            ..BoundaryRules::default()
        };
        assert!(template().with_boundary_rules(empty_edge).is_err());

        let seeded = navette::smatrix::synthesis::cycle::NeedleRules {
            seed_edges: Some(5.0),
            ..navette::smatrix::synthesis::cycle::NeedleRules::default()
        };
        assert!(BoundaryRules::from_needle_rules(seeded).is_err());

        let wildcard = BoundaryRules {
            insert: Some(
                [("*".to_string(), vec!["L".to_string()])]
                    .into_iter()
                    .collect(),
            ),
            ..BoundaryRules::default()
        };
        assert!(template().with_boundary_rules(wildcard).is_err());
    }

    #[test]
    fn multi_material_edges_are_validated_not_pinned() {
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0, 1),
            constant("substrate", 1.52, 0.0, 1),
            vec![
                constant("H", 2.2, 0.0, 1),
                constant("L", 1.46, 0.0, 1),
                constant("G", 1.8, 0.0, 1),
            ],
        )
        .unwrap()
        .with_boundary_rules(BoundaryRules {
            outer: Some(vec!["H".to_string(), "G".to_string()]),
            inner: Some(vec!["L".to_string()]),
            ..BoundaryRules::default()
        })
        .unwrap();
        let decoder_pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 300.0).unwrap(),
            Material::new("L", 10.0, 200.0).unwrap(),
            Material::new("G", 15.0, 250.0).unwrap(),
        ])
        .unwrap();
        let pins = template.edge_pins(&decoder_pool).unwrap();
        assert_eq!(pins.outer, None, "a two-material list cannot be pinned");
        assert_eq!(pins.inner, Some(1));
    }

    #[test]
    fn boundary_rules_apply_to_pool_and_assembled_edges() {
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0, 1),
            constant("substrate", 1.52, 0.0, 1),
            vec![
                constant("H", 2.2, 0.0, 1),
                constant("L", 1.46, 0.0, 1),
                constant("G", 1.8, 0.0, 1),
            ],
        )
        .unwrap();
        let rules = BoundaryRules {
            insert: Some(
                [
                    ("H".to_string(), vec!["L".to_string()]),
                    ("L".to_string(), Vec::new()),
                ]
                .into_iter()
                .collect(),
            ),
            outer: Some(vec!["H".to_string()]),
            inner: Some(vec!["L".to_string()]),
        };
        let template = template.with_boundary_rules(rules).unwrap();

        let pool = template.needle_pool().unwrap();
        assert_eq!(pool.len(), 3);
        let edges = template.edge_materials().unwrap();
        assert_eq!(edges.outer.as_ref().unwrap()[0].0.as_ref(), "H");
        assert_eq!(edges.inner.as_ref().unwrap()[0].0.as_ref(), "L");

        let decoder_pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 300.0).unwrap(),
            Material::new("L", 10.0, 200.0).unwrap(),
        ])
        .unwrap();
        let pins = template.edge_pins(&decoder_pool).unwrap();
        assert_eq!(pins.outer, Some(0));
        assert_eq!(pins.inner, Some(1));
        let decoder = Decoder::without_constraints(decoder_pool).unwrap();
        let good = decoder
            .decode(&Genome::from(vec![
                LayerGene::new(0.1, 0.5, 0.9),
                LayerGene::new(0.9, 0.5, 0.9),
            ]))
            .unwrap();
        let stack = template.assemble(&good).unwrap();
        assert!(!stack.edge_materials().is_free());
        assert_eq!(
            stack.edge_materials().outer.as_ref().unwrap()[0].0.as_ref(),
            "H"
        );
        assert_eq!(
            stack.edge_materials().inner.as_ref().unwrap()[0].0.as_ref(),
            "L"
        );

        let bad = decoder
            .decode(&Genome::from(vec![
                LayerGene::new(0.9, 0.5, 0.9),
                LayerGene::new(0.1, 0.5, 0.9),
            ]))
            .unwrap();
        assert!(template.assemble(&bad).is_err());
    }

    #[test]
    fn boundary_needles_use_insert_rules_and_edge_lists() {
        let spec = zero_reflectance_spec(0.0, 450.0);
        let ev = evaluator(spec.clone());
        let stack = fixed_stack(
            vec![constant("H", 2.2, 40.0, 1), constant("L", 1.46, 40.0, 1)],
            1,
        );
        let insert: navette::smatrix::synthesis::cycle::InsertRules = [
            ("H".to_string(), vec!["G".to_string()]),
            ("L".to_string(), vec!["G".to_string()]),
        ]
        .into_iter()
        .collect();
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0, 1),
            constant("substrate", 1.52, 0.0, 1),
            vec![
                constant("H", 2.2, 0.0, 1),
                constant("L", 1.46, 0.0, 1),
                constant("G", 1.8, 0.0, 1),
            ],
        )
        .unwrap()
        .with_boundary_rules(BoundaryRules {
            insert: Some(insert.clone()),
            ..BoundaryRules::default()
        })
        .unwrap();
        let spectral = SpectralInputs::from_spec(&spec, &[Radians::ZERO], &[450.0]).unwrap();
        let cfg = NeedleCycleConfig {
            max_needles: 1,
            scan_step_nm: 10.0,
            boundary_needles: true,
            ..NeedleCycleConfig::default()
        };
        let op = MemeticOperator::from_template(spectral, &template, cfg).unwrap();
        let scan = op.scan(&ev, &stack).unwrap();
        let boundary: Vec<_> = scan
            .candidates
            .iter()
            .filter(|candidate| candidate.site.boundary)
            .collect();
        assert!(!boundary.is_empty(), "{scan:?}");
        assert!(
            boundary
                .iter()
                .all(|candidate| candidate.material.as_ref() == "G"
                    && candidate.site.film_idx == 2),
            "{scan:?}"
        );

        let restricted = template
            .with_boundary_rules(BoundaryRules {
                insert: Some(insert),
                outer: Some(vec!["H".to_string()]),
                inner: Some(vec!["L".to_string()]),
            })
            .unwrap();
        let mut stack = stack;
        stack
            .set_edge_materials(restricted.edge_materials().unwrap())
            .unwrap();
        let spectral = SpectralInputs::from_spec(&spec, &[Radians::ZERO], &[450.0]).unwrap();
        let cfg = NeedleCycleConfig {
            max_needles: 1,
            scan_step_nm: 10.0,
            boundary_needles: true,
            ..NeedleCycleConfig::default()
        };
        let op = MemeticOperator::from_template(spectral, &restricted, cfg).unwrap();
        let scan = op.scan(&ev, &stack).unwrap();
        assert!(
            scan.candidates
                .iter()
                .all(|candidate| !candidate.site.boundary),
            "{scan:?}"
        );
    }

    #[test]
    fn insert_rules_must_match_both_grammar_directions() {
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0, 1),
            constant("substrate", 1.52, 0.0, 1),
            vec![
                constant("H", 2.2, 0.0, 1),
                constant("L", 1.46, 0.0, 1),
                constant("G", 1.8, 0.0, 1),
            ],
        )
        .unwrap();
        let rules = BoundaryRules {
            insert: Some(
                [("H".to_string(), vec!["L".to_string()])]
                    .into_iter()
                    .collect(),
            ),
            ..BoundaryRules::default()
        };
        let template = template.with_boundary_rules(rules).unwrap();
        let decoder_pool = MaterialPool::new(vec![
            Material::new("H", 20.0, 300.0).unwrap(),
            Material::new("L", 10.0, 200.0).unwrap(),
        ])
        .unwrap();

        let open =
            AdjacencyGrammar::from_allowed(vec![vec![true, true], vec![true, true]]).unwrap();
        template.check_grammar(&decoder_pool, &open).unwrap();

        let no_h_to_l =
            AdjacencyGrammar::from_allowed(vec![vec![true, false], vec![true, true]]).unwrap();
        let err = template
            .check_grammar(&decoder_pool, &no_h_to_l)
            .unwrap_err();
        assert!(err.contains("insert/grammar conflict"), "{err}");

        let no_l_to_h =
            AdjacencyGrammar::from_allowed(vec![vec![true, true], vec![false, true]]).unwrap();
        assert!(template.check_grammar(&decoder_pool, &no_l_to_h).is_err());
    }

    #[test]
    fn boundary_rules_restrict_the_memetic_scan() {
        let spec = zero_reflectance_spec(0.0, 550.0);
        let ev = evaluator(spec.clone());
        let stack = fixed_stack(
            vec![constant("H", 2.2, 80.0, 1), constant("L", 1.46, 120.0, 1)],
            1,
        );
        let template = StackTemplate::new(
            constant("air", 1.0, 0.0, 1),
            constant("substrate", 1.52, 0.0, 1),
            vec![
                constant("H", 2.2, 0.0, 1),
                constant("L", 1.46, 0.0, 1),
                constant("G", 1.8, 0.0, 1),
            ],
        )
        .unwrap();
        let rules = BoundaryRules {
            insert: Some(
                [
                    ("H".to_string(), vec!["L".to_string()]),
                    ("L".to_string(), Vec::new()),
                    ("G".to_string(), Vec::new()),
                ]
                .into_iter()
                .collect(),
            ),
            ..BoundaryRules::default()
        };
        let template = template.with_boundary_rules(rules).unwrap();
        let spectral = SpectralInputs::from_spec(&spec, &[Radians::ZERO], &[550.0]).unwrap();
        let cfg = NeedleCycleConfig {
            max_needles: 1,
            scan_step_nm: 10.0,
            ..NeedleCycleConfig::default()
        };
        let op = MemeticOperator::from_template(spectral, &template, cfg).unwrap();
        let scan = op.scan(&ev, &stack).unwrap();

        assert!(!scan.is_empty(), "{scan:?}");
        assert!(
            scan.candidates
                .iter()
                .all(|candidate| candidate.material.as_ref() == "L"),
            "{scan:?}"
        );
    }

    #[test]
    fn pinned_layers_are_not_optimized_or_needle_hosts() {
        let pool = MaterialPool::new(vec![
            Material::new("L", 10.0, 200.0).unwrap(),
            Material::new("H", 20.0, 300.0).unwrap(),
        ])
        .unwrap();
        let decoder = Decoder::new(
            pool,
            Pins::with_edges(Some(0), Some(1)),
            AdjacencyGrammar::all_allowed(2),
        )
        .unwrap();
        let genome = Genome::from(vec![
            LayerGene::new(0.0, 0.5, 0.9),
            LayerGene::new(0.9, 0.5, 0.9),
        ]);
        let decoded = decoder.decode(&genome).unwrap();

        let template = StackTemplate::new(
            constant("air", 1.0, 0.0, 1),
            constant("substrate", 1.52, 0.0, 1),
            vec![constant("L", 1.38, 0.0, 1), constant("H", 2.2, 0.0, 1)],
        )
        .unwrap();
        let stack = template.assemble(&decoded).unwrap();
        assert!(!stack.films()[0].optimize);
        assert!(!stack.films()[0].needle);
        assert!(!stack.films()[1].optimize);
        assert!(!stack.films()[1].needle);
    }

    #[test]
    fn lm_finish_improves_a_detuned_quarter_wave() {
        let n_substrate = 1.52_f64;
        let n_film = n_substrate.sqrt();
        let d_nm = 550.0 / (4.0 * n_film);
        let mut stack = fixed_stack(vec![constant("film", n_film, 0.8 * d_nm, 1)], 1);
        let mut ev = evaluator(zero_reflectance_spec(0.0, 550.0));

        let before = ev.evaluate(&stack).unwrap();
        let after = ev.refine(&mut stack).unwrap();
        let d_after = stack.films()[0].d_nm;

        assert!(after < before, "merit {before} -> {after}");
        assert!(
            (d_after - d_nm).abs() < 2.0,
            "LM finished at {d_after} nm, design {d_nm} nm"
        );
    }

    #[test]
    fn batch_preserves_input_order() {
        let ev = evaluator(zero_reflectance_spec(0.0, 550.0));
        let stacks = [
            fixed_stack(vec![constant("a", 1.2, 100.0, 1)], 1),
            fixed_stack(vec![constant("b", 1.4, 120.0, 1)], 1),
        ];
        let batch = ev.evaluate_batch(&stacks).unwrap();
        let serial: Vec<f64> = stacks.iter().map(|st| ev.evaluate(st).unwrap()).collect();
        assert_eq!(batch, serial);
    }

    #[test]
    fn memetic_operator_inserts_and_polishes() {
        let spec = zero_reflectance_spec(0.0, 550.0);
        let mut ev = evaluator(spec.clone());
        let mut stack = fixed_stack(
            vec![constant("H", 2.2, 80.0, 1), constant("L", 1.46, 120.0, 1)],
            1,
        );
        let pool = navette::smatrix::synthesis::cycle::MaterialPool::from(vec![
            constant("H", 2.2, 0.0, 1),
            constant("L", 1.46, 0.0, 1),
        ]);
        let spectral = SpectralInputs::from_spec(&spec, &[Radians::ZERO], &[550.0]).unwrap();
        let cfg = NeedleCycleConfig {
            max_needles: 1,
            scan_step_nm: 10.0,
            ..NeedleCycleConfig::default()
        };
        let op = MemeticOperator::new(spectral, pool, cfg);
        let before = stack.films().len();
        let report = op.apply(&mut ev, &mut stack, 1).unwrap();

        assert!(report.scans >= 1, "{report:?}");
        assert!(report.final_merit <= report.initial_merit + 1e-12);
        assert_eq!(report.steps.len(), 1, "{report:?}");
        assert!(stack.films().len() > before);
        let step = &report.steps[0];
        assert!(step.polish.merit_after <= step.polish.merit_before + 1e-12);
    }

    #[test]
    fn invalid_inputs_are_refused_before_a_solve() {
        let spec = zero_reflectance_spec(0.0, 550.0);
        assert!(FitnessEvaluator::new(
            Vec::new(),
            &[0.0],
            spec.clone(),
            EvaluatorOptions::default()
        )
        .is_err());
        assert!(
            FitnessEvaluator::new(vec![550.0], &[], spec.clone(), EvaluatorOptions::default())
                .is_err()
        );
        assert!(FitnessEvaluator::new(
            vec![550.0, 500.0],
            &[0.0],
            spec,
            EvaluatorOptions::default()
        )
        .is_err());
    }

    #[test]
    fn template_refuses_a_wrong_nk_grid() {
        let result = StackTemplate::new(
            constant("air", 1.0, 0.0, 2),
            constant("substrate", 1.52, 0.0, 2),
            vec![constant("film", 1.4, 100.0, 1)],
        );
        assert!(result.is_err());
    }
}
