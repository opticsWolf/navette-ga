// SPDX-License-Identifier: LGPL-3.0-or-later
//! G2/G3 — genome decoder and material pool.
//!
//! Owns the variable-length random-key genome, the material-pool mapping,
//! per-material thickness bounds, pinned layer indices, the adjacency grammar,
//! and the operator masks that say which keys an operator may touch. One
//! decoder serves the GA, the needle seeding path, and repair.
//!
//! Physics-backed tables (bounds, pins, grammar) are derived from navette's
//! A4 API at startup; navette stays the single owner of legality. This module
//! deliberately knows nothing about optics: it maps keys to material indices
//! and thicknesses, and the bridge turns the result into a `DesignStack`.

use std::fmt;

/// Default cap on decoded films.
pub const DEFAULT_MAX_LAYERS: usize = 80;

/// Default presence threshold: a gene is active when `presence >= threshold`.
pub const DEFAULT_PRESENCE_THRESHOLD: f64 = 0.5;

/// One variable-length layer gene, all fields in the unit interval.
///
/// The keys are deliberately raw `f64`s: crossover and mutation operate on
/// keys, and the decoder is the only place where a key becomes a material or
/// a physical thickness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerGene {
    /// Selects a material from the pool.
    pub material: f64,
    /// Selects a thickness inside that material's bounds.
    pub thickness: f64,
    /// Below the decoder's presence threshold the gene is absent.
    pub presence: f64,
}

impl LayerGene {
    pub fn new(material: f64, thickness: f64, presence: f64) -> Self {
        Self {
            material,
            thickness,
            presence,
        }
    }

    fn validate(self, gene: usize) -> Result<(), DecodeError> {
        for (field, value) in [
            ("material", self.material),
            ("thickness", self.thickness),
            ("presence", self.presence),
        ] {
            if !value.is_finite() {
                return Err(DecodeError::InvalidKey { gene, field, value });
            }
        }
        Ok(())
    }
}

/// A variable-length genome: one [`LayerGene`] per slot.
///
/// Insertion and deletion are ordinary `Vec` operations; no fixed-width
/// encoding is imposed on the GA. The decoder decides which genes are present.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Genome {
    genes: Vec<LayerGene>,
}

impl Genome {
    pub fn new(genes: Vec<LayerGene>) -> Self {
        Self { genes }
    }

    pub fn len(&self) -> usize {
        self.genes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.genes.is_empty()
    }

    pub fn genes(&self) -> &[LayerGene] {
        &self.genes
    }

    pub fn genes_mut(&mut self) -> &mut [LayerGene] {
        &mut self.genes
    }

    pub fn push(&mut self, gene: LayerGene) {
        self.genes.push(gene);
    }

    pub fn remove(&mut self, index: usize) -> Option<LayerGene> {
        (index < self.genes.len()).then(|| self.genes.remove(index))
    }

    pub fn iter(&self) -> impl Iterator<Item = &LayerGene> {
        self.genes.iter()
    }
}

impl From<Vec<LayerGene>> for Genome {
    fn from(genes: Vec<LayerGene>) -> Self {
        Self::new(genes)
    }
}

/// One material available to the decoder.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    name: String,
    min_thickness_nm: f64,
    max_thickness_nm: f64,
}

impl Material {
    /// Build a material with a closed thickness interval `[min, max]` in nm.
    pub fn new(
        name: impl Into<String>,
        min_thickness_nm: f64,
        max_thickness_nm: f64,
    ) -> Result<Self, DecodeError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DecodeError::InvalidMaterial {
                name,
                reason: "material name is empty".to_string(),
            });
        }
        if !min_thickness_nm.is_finite()
            || !max_thickness_nm.is_finite()
            || min_thickness_nm < 0.0
            || max_thickness_nm < min_thickness_nm
        {
            return Err(DecodeError::InvalidMaterial {
                name,
                reason: format!(
                    "invalid thickness bounds [{min_thickness_nm}, {max_thickness_nm}] nm"
                ),
            });
        }
        Ok(Self {
            name,
            min_thickness_nm,
            max_thickness_nm,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn min_thickness_nm(&self) -> f64 {
        self.min_thickness_nm
    }

    pub fn max_thickness_nm(&self) -> f64 {
        self.max_thickness_nm
    }

    /// Map a unit-interval key into this material's bounds.
    ///
    /// Out-of-range finite keys are clamped; non-finite keys are rejected by
    /// the decoder before this is called. `key = 1.0` lands on the upper bound.
    pub fn thickness_for_key(&self, key: f64) -> f64 {
        let k = if key.is_finite() {
            key.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.min_thickness_nm + k * (self.max_thickness_nm - self.min_thickness_nm)
    }
}

/// The ordered material pool. Order is the similarity order used by the GA:
/// index 0 is the first pool member, and keys select by position.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialPool {
    materials: Vec<Material>,
}

impl MaterialPool {
    pub fn new(materials: Vec<Material>) -> Result<Self, DecodeError> {
        if materials.is_empty() {
            return Err(DecodeError::EmptyPool);
        }
        Ok(Self { materials })
    }

    pub fn len(&self) -> usize {
        self.materials.len()
    }

    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Material> {
        self.materials.get(index)
    }

    pub fn materials(&self) -> &[Material] {
        &self.materials
    }

    /// Map a unit-interval key to a pool index.
    ///
    /// Clamping is intentional: a GA may temporarily produce keys outside
    /// `[0, 1)` after crossover or mutation, and a clamp is a legal repair.
    pub fn index_for_key(&self, key: f64) -> usize {
        let k = if key.is_finite() {
            key.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let last = self.materials.len().saturating_sub(1);
        ((k * self.materials.len() as f64).floor() as usize).min(last)
    }
}

/// Fixed edge materials. A pinned film is a decoder constant: it is never
/// mutated, never optimized, and never removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pins {
    /// Material index for the ambient-side film (layer 0).
    pub outer: Option<usize>,
    /// Material index for the substrate-side film (last layer).
    pub inner: Option<usize>,
}

impl Pins {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_edges(outer: Option<usize>, inner: Option<usize>) -> Self {
        Self { outer, inner }
    }
}

/// Which material may follow which. An all-allowed grammar is the default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdjacencyGrammar {
    allowed: Vec<Vec<bool>>,
}

impl AdjacencyGrammar {
    pub fn all_allowed(materials: usize) -> Self {
        Self {
            allowed: vec![vec![true; materials]; materials],
        }
    }

    pub fn from_allowed(allowed: Vec<Vec<bool>>) -> Result<Self, DecodeError> {
        let materials = allowed.len();
        if materials == 0 {
            return Err(DecodeError::GrammarShape {
                materials,
                rows: 0,
                row: 0,
                cols: 0,
            });
        }
        for (row, cols) in allowed.iter().enumerate() {
            if cols.len() != materials {
                return Err(DecodeError::GrammarShape {
                    materials,
                    rows: allowed.len(),
                    row,
                    cols: cols.len(),
                });
            }
        }
        Ok(Self { allowed })
    }

    pub fn len(&self) -> usize {
        self.allowed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.allowed.is_empty()
    }

    /// Is `next` legal after `previous`? Out-of-range indices are illegal.
    pub fn allows(&self, previous: usize, next: usize) -> bool {
        self.allowed
            .get(previous)
            .and_then(|row| row.get(next))
            .copied()
            .unwrap_or(false)
    }
}

/// Why a genome could not be decoded.
#[derive(Clone, Debug, PartialEq)]
pub enum DecodeError {
    EmptyPool,
    InvalidMaterial {
        name: String,
        reason: String,
    },
    InvalidKey {
        gene: usize,
        field: &'static str,
        value: f64,
    },
    InvalidPresenceThreshold {
        value: f64,
    },
    GenomeTooLong {
        len: usize,
        max: usize,
    },
    ConflictingPins,
    PinsWithoutLayers,
    GrammarShape {
        materials: usize,
        rows: usize,
        row: usize,
        cols: usize,
    },
    UnrepairableGrammar {
        position: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::EmptyPool => write!(f, "the material pool is empty"),
            DecodeError::InvalidMaterial { name, reason } => {
                write!(f, "material {name:?}: {reason}")
            }
            DecodeError::InvalidKey { gene, field, value } => {
                write!(f, "gene {gene}: {field} key is not finite ({value})")
            }
            DecodeError::InvalidPresenceThreshold { value } => {
                write!(
                    f,
                    "presence threshold must be finite and in [0, 1], got {value}"
                )
            }
            DecodeError::GenomeTooLong { len, max } => {
                write!(f, "genome has {len} genes, decoder cap is {max}")
            }
            DecodeError::ConflictingPins => {
                write!(f, "one layer cannot satisfy two different edge pins")
            }
            DecodeError::PinsWithoutLayers => {
                write!(
                    f,
                    "edge pins are configured but the genome decoded no layers"
                )
            }
            DecodeError::GrammarShape {
                materials,
                rows,
                row,
                cols,
            } => write!(
                f,
                "adjacency grammar must be {materials}x{materials}; got {rows} rows, \
                 row {row} has {cols} columns"
            ),
            DecodeError::UnrepairableGrammar { position } => {
                write!(f, "no legal material repair at layer boundary {position}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// A material change made by grammar repair, for the run log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repair {
    pub gene_index: usize,
    pub position: usize,
    pub from: usize,
    pub to: usize,
}

/// One decoded film.
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedLayer {
    /// Index of the gene that produced this film.
    pub gene_index: usize,
    pub material: usize,
    pub material_name: String,
    pub thickness_nm: f64,
    /// Pinned films are decoder constants: no mutation, no optimization,
    /// no removal.
    pub pinned: bool,
}

/// A decoded variable-length stack.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecodedStack {
    layers: Vec<DecodedLayer>,
    repairs: Vec<Repair>,
}

impl DecodedStack {
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    pub fn layers(&self) -> &[DecodedLayer] {
        &self.layers
    }

    pub fn repairs(&self) -> &[Repair] {
        &self.repairs
    }

    pub fn materials(&self) -> Vec<usize> {
        self.layers.iter().map(|layer| layer.material).collect()
    }

    pub fn thicknesses(&self) -> Vec<f64> {
        self.layers.iter().map(|layer| layer.thickness_nm).collect()
    }

    /// Which genes may be touched by an operator.
    ///
    /// Present, unpinned films are fully mutable. Pinned genes are frozen.
    /// Absent genes keep all three keys mutable so an operator can bring a
    /// dormant slot back or remove it.
    pub fn operator_mask(&self, genome_len: usize) -> Vec<GeneMask> {
        let mut mask = vec![GeneMask::all(); genome_len];
        for layer in &self.layers {
            if layer.pinned && layer.gene_index < genome_len {
                mask[layer.gene_index] = GeneMask::none();
            }
        }
        mask
    }
}

/// Which of a gene's three keys an operator may mutate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneMask {
    pub material: bool,
    pub thickness: bool,
    pub presence: bool,
}

impl GeneMask {
    pub const fn all() -> Self {
        Self {
            material: true,
            thickness: true,
            presence: true,
        }
    }

    pub const fn none() -> Self {
        Self {
            material: false,
            thickness: false,
            presence: false,
        }
    }
}

/// The one decoder used by the GA, needle seeding, and repair.
#[derive(Clone, Debug)]
pub struct Decoder {
    pool: MaterialPool,
    pins: Pins,
    grammar: AdjacencyGrammar,
    max_layers: usize,
    presence_threshold: f64,
}

impl Decoder {
    pub fn new(
        pool: MaterialPool,
        pins: Pins,
        grammar: AdjacencyGrammar,
    ) -> Result<Self, DecodeError> {
        if pool.is_empty() {
            return Err(DecodeError::EmptyPool);
        }
        if grammar.len() != pool.len() {
            return Err(DecodeError::GrammarShape {
                materials: pool.len(),
                rows: grammar.len(),
                row: 0,
                cols: grammar.len(),
            });
        }
        for (which, pin) in [("outer", pins.outer), ("inner", pins.inner)] {
            if let Some(index) = pin {
                if index >= pool.len() {
                    return Err(DecodeError::InvalidMaterial {
                        name: format!("{which} pin"),
                        reason: format!(
                            "material index {index} is outside the pool (len {})",
                            pool.len()
                        ),
                    });
                }
            }
        }
        Ok(Self {
            pool,
            pins,
            grammar,
            max_layers: DEFAULT_MAX_LAYERS,
            presence_threshold: DEFAULT_PRESENCE_THRESHOLD,
        })
    }

    /// A decoder with no pins and an all-allowed grammar.
    pub fn without_constraints(pool: MaterialPool) -> Result<Self, DecodeError> {
        let grammar = AdjacencyGrammar::all_allowed(pool.len());
        Self::new(pool, Pins::none(), grammar)
    }

    pub fn with_max_layers(mut self, max_layers: usize) -> Self {
        self.max_layers = max_layers.max(1);
        self
    }

    pub fn with_presence_threshold(mut self, threshold: f64) -> Result<Self, DecodeError> {
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err(DecodeError::InvalidPresenceThreshold { value: threshold });
        }
        self.presence_threshold = threshold;
        Ok(self)
    }

    pub fn pool(&self) -> &MaterialPool {
        &self.pool
    }

    pub fn pins(&self) -> Pins {
        self.pins
    }

    pub fn grammar(&self) -> &AdjacencyGrammar {
        &self.grammar
    }

    pub fn max_layers(&self) -> usize {
        self.max_layers
    }

    pub fn presence_threshold(&self) -> f64 {
        self.presence_threshold
    }

    /// Decode a genome into a legal, variable-length stack.
    pub fn decode(&self, genome: &Genome) -> Result<DecodedStack, DecodeError> {
        if genome.len() > self.max_layers {
            return Err(DecodeError::GenomeTooLong {
                len: genome.len(),
                max: self.max_layers,
            });
        }

        let mut layers = Vec::with_capacity(genome.len());
        for (gene_index, gene) in genome.iter().enumerate() {
            gene.validate(gene_index)?;
            if gene.presence < self.presence_threshold {
                continue;
            }
            let material = self.pool.index_for_key(gene.material);
            let thickness_nm = self.pool.materials()[material].thickness_for_key(gene.thickness);
            layers.push(DecodedLayer {
                gene_index,
                material,
                material_name: self.pool.materials()[material].name().to_string(),
                thickness_nm,
                pinned: false,
            });
        }

        if layers.is_empty() {
            if self.pins.outer.is_some() || self.pins.inner.is_some() {
                return Err(DecodeError::PinsWithoutLayers);
            }
            return Ok(DecodedStack {
                layers,
                repairs: Vec::new(),
            });
        }

        self.apply_pins(&mut layers, genome)?;
        let repairs = self.repair_grammar(&mut layers, genome)?;

        Ok(DecodedStack { layers, repairs })
    }

    fn apply_pins(&self, layers: &mut [DecodedLayer], genome: &Genome) -> Result<(), DecodeError> {
        let n = layers.len();
        if n == 0 {
            return if self.pins.outer.is_some() || self.pins.inner.is_some() {
                Err(DecodeError::PinsWithoutLayers)
            } else {
                Ok(())
            };
        }

        if n == 1 {
            if let (Some(outer), Some(inner)) = (self.pins.outer, self.pins.inner) {
                if outer != inner {
                    return Err(DecodeError::ConflictingPins);
                }
            }
        }

        if let Some(material) = self.pins.outer {
            self.set_material(&mut layers[0], material, genome);
            layers[0].pinned = true;
        }
        if let Some(material) = self.pins.inner {
            let index = n - 1;
            self.set_material(&mut layers[index], material, genome);
            layers[index].pinned = true;
        }
        Ok(())
    }

    fn set_material(&self, layer: &mut DecodedLayer, material: usize, genome: &Genome) {
        layer.material = material;
        layer.material_name = self.pool.materials()[material].name().to_string();
        let key = genome.genes()[layer.gene_index].thickness;
        layer.thickness_nm = self.pool.materials()[material].thickness_for_key(key);
    }

    /// Repair illegal adjacencies deterministically.
    ///
    /// Pins win: an interior film is changed first, then the film before the
    /// violation. A violation with pinned films on both sides is an error,
    /// never a silent illegal stack.
    fn repair_grammar(
        &self,
        layers: &mut [DecodedLayer],
        genome: &Genome,
    ) -> Result<Vec<Repair>, DecodeError> {
        let n = layers.len();
        if n < 2 {
            return Ok(Vec::new());
        }

        let mut repairs = Vec::new();
        for _ in 0..n {
            let mut changed = false;
            for i in 1..n {
                if self
                    .grammar
                    .allows(layers[i - 1].material, layers[i].material)
                {
                    continue;
                }

                let left = layers[i - 1].material;
                let right = layers.get(i + 1).map(|layer| layer.material);

                // First choice: replace the right film if it may legally
                // stand between both neighbours.
                let replacement = (0..self.pool.len()).find(|&candidate| {
                    self.grammar.allows(left, candidate)
                        && right.is_none_or(|r| self.grammar.allows(candidate, r))
                });
                if let Some(candidate) = replacement {
                    if !layers[i].pinned {
                        let from = layers[i].material;
                        self.set_material(&mut layers[i], candidate, genome);
                        repairs.push(Repair {
                            gene_index: layers[i].gene_index,
                            position: i,
                            from,
                            to: candidate,
                        });
                        changed = true;
                        continue;
                    }
                }

                // Second choice: replace the left film, keeping the right
                // one fixed.
                let left_left = i.checked_sub(2).map(|j| layers[j].material);
                let replacement = (0..self.pool.len()).find(|&candidate| {
                    left_left.is_none_or(|l| self.grammar.allows(l, candidate))
                        && self.grammar.allows(candidate, layers[i].material)
                });
                if let Some(candidate) = replacement {
                    if !layers[i - 1].pinned {
                        let from = layers[i - 1].material;
                        self.set_material(&mut layers[i - 1], candidate, genome);
                        repairs.push(Repair {
                            gene_index: layers[i - 1].gene_index,
                            position: i - 1,
                            from,
                            to: candidate,
                        });
                        changed = true;
                        continue;
                    }
                }

                return Err(DecodeError::UnrepairableGrammar { position: i });
            }
            if !changed {
                break;
            }
        }

        for i in 1..n {
            if !self
                .grammar
                .allows(layers[i - 1].material, layers[i].material)
            {
                return Err(DecodeError::UnrepairableGrammar { position: i });
            }
        }
        Ok(repairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> MaterialPool {
        MaterialPool::new(vec![
            Material::new("L", 10.0, 200.0).unwrap(),
            Material::new("H", 20.0, 300.0).unwrap(),
            Material::new("M", 5.0, 100.0).unwrap(),
        ])
        .unwrap()
    }

    fn gene(material: f64, thickness: f64, presence: f64) -> LayerGene {
        LayerGene::new(material, thickness, presence)
    }

    #[test]
    fn keys_map_to_material_and_bound_thickness() {
        let decoder = Decoder::without_constraints(pool()).unwrap();
        let genome = Genome::from(vec![
            gene(0.0, 0.0, 0.9),
            gene(0.5, 0.5, 0.9),
            gene(1.0, 1.0, 0.9),
        ]);
        let decoded = decoder.decode(&genome).unwrap();

        assert_eq!(decoded.materials(), vec![0, 1, 2]);
        assert_eq!(decoded.len(), 3);
        assert!((decoded.layers()[0].thickness_nm - 10.0).abs() < 1e-12);
        assert!((decoded.layers()[1].thickness_nm - 160.0).abs() < 1e-12);
        assert!((decoded.layers()[2].thickness_nm - 100.0).abs() < 1e-12);
    }

    #[test]
    fn absence_threshold_drops_genes_and_keeps_source_indices() {
        let decoder = Decoder::without_constraints(pool())
            .unwrap()
            .with_presence_threshold(0.5)
            .unwrap();
        let genome = Genome::from(vec![
            gene(0.0, 0.5, 0.49),
            gene(0.5, 0.5, 0.50),
            gene(0.9, 0.5, 0.20),
        ]);
        let decoded = decoder.decode(&genome).unwrap();

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded.layers()[0].gene_index, 1);
        assert_eq!(decoded.layers()[0].material, 1);
        assert_eq!(decoded.operator_mask(3).len(), 3);
    }

    #[test]
    fn out_of_range_keys_are_clamped_but_nan_is_rejected() {
        let decoder = Decoder::without_constraints(pool()).unwrap();
        let decoded = decoder
            .decode(&Genome::from(vec![gene(-3.0, 9.0, 2.0)]))
            .unwrap();
        assert_eq!(decoded.layers()[0].material, 0);
        assert!((decoded.layers()[0].thickness_nm - 200.0).abs() < 1e-12);

        let err = decoder
            .decode(&Genome::from(vec![gene(f64::NAN, 0.5, 0.9)]))
            .unwrap_err();
        assert!(matches!(
            err,
            DecodeError::InvalidKey {
                field: "material",
                ..
            }
        ));
    }

    #[test]
    fn pins_override_edges_and_freeze_their_genes() {
        let parser = pool();
        let decoder = Decoder::new(
            parser,
            Pins::with_edges(Some(0), Some(2)),
            AdjacencyGrammar::all_allowed(3),
        )
        .unwrap();
        let genome = Genome::from(vec![
            gene(0.9, 0.5, 0.9),
            gene(0.5, 0.5, 0.9),
            gene(0.0, 0.5, 0.9),
        ]);
        let decoded = decoder.decode(&genome).unwrap();

        assert_eq!(decoded.materials(), vec![0, 1, 2]);
        assert!(decoded.layers()[0].pinned);
        assert!(decoded.layers()[2].pinned);

        let mask = decoded.operator_mask(3);
        assert_eq!(mask[0], GeneMask::none());
        assert_eq!(mask[1], GeneMask::all());
        assert_eq!(mask[2], GeneMask::none());
    }

    #[test]
    fn one_layer_cannot_satisfy_two_different_pins() {
        let decoder = Decoder::new(
            pool(),
            Pins::with_edges(Some(0), Some(1)),
            AdjacencyGrammar::all_allowed(3),
        )
        .unwrap();
        let err = decoder
            .decode(&Genome::from(vec![gene(0.5, 0.5, 0.9)]))
            .unwrap_err();
        assert_eq!(err, DecodeError::ConflictingPins);
    }

    #[test]
    fn grammar_repair_fixes_an_illegal_adjacency() {
        // L cannot touch L; H is legal on both sides.
        let mut allowed = vec![vec![true; 3]; 3];
        allowed[0][0] = false;
        let decoder = Decoder::new(
            pool(),
            Pins::none(),
            AdjacencyGrammar::from_allowed(allowed).unwrap(),
        )
        .unwrap();
        let genome = Genome::from(vec![
            gene(0.0, 0.5, 0.9),
            gene(0.0, 0.5, 0.9),
            gene(0.0, 0.5, 0.9),
        ]);
        let decoded = decoder.decode(&genome).unwrap();

        assert_eq!(decoded.len(), 3);
        for pair in decoded.layers().windows(2) {
            assert!(decoder.grammar().allows(pair[0].material, pair[1].material));
        }
        assert!(!decoded.repairs().is_empty());
    }

    #[test]
    fn grammar_with_no_legal_successor_is_an_error() {
        let allowed = vec![vec![false; 3]; 3];
        let decoder = Decoder::new(
            pool(),
            Pins::none(),
            AdjacencyGrammar::from_allowed(allowed).unwrap(),
        )
        .unwrap();
        let genome = Genome::from(vec![gene(0.0, 0.5, 0.9), gene(0.5, 0.5, 0.9)]);
        let err = decoder.decode(&genome).unwrap_err();
        assert!(matches!(err, DecodeError::UnrepairableGrammar { .. }));
    }

    #[test]
    fn pinned_violation_is_not_repaired_away() {
        let mut allowed = vec![vec![true; 3]; 3];
        allowed[0][1] = false; // outer pin L cannot touch inner pin H
        let decoder = Decoder::new(
            pool(),
            Pins::with_edges(Some(0), Some(1)),
            AdjacencyGrammar::from_allowed(allowed).unwrap(),
        )
        .unwrap();
        let genome = Genome::from(vec![gene(0.5, 0.5, 0.9), gene(0.5, 0.5, 0.9)]);
        let err = decoder.decode(&genome).unwrap_err();
        assert!(matches!(err, DecodeError::UnrepairableGrammar { .. }));
    }

    #[test]
    fn empty_genome_is_empty_and_pins_require_layers() {
        let decoder = Decoder::without_constraints(pool()).unwrap();
        assert!(decoder.decode(&Genome::default()).unwrap().is_empty());

        let pinned = Decoder::new(
            pool(),
            Pins::with_edges(Some(0), None),
            AdjacencyGrammar::all_allowed(3),
        )
        .unwrap();
        assert_eq!(
            pinned.decode(&Genome::default()).unwrap_err(),
            DecodeError::PinsWithoutLayers
        );
    }

    #[test]
    fn genome_length_cap_is_enforced() {
        let decoder = Decoder::without_constraints(pool())
            .unwrap()
            .with_max_layers(1);
        let err = decoder
            .decode(&Genome::from(vec![
                gene(0.0, 0.5, 0.9),
                gene(0.1, 0.5, 0.9),
            ]))
            .unwrap_err();
        assert_eq!(err, DecodeError::GenomeTooLong { len: 2, max: 1 });
    }
}
