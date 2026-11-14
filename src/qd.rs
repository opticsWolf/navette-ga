// SPDX-License-Identifier: LGPL-3.0-or-later
//! S4/F1 — quality-diversity archive.
//!
//! The archive is deliberately small and dependency-free at the API boundary:
//! a two-dimensional MAP-Elites grid over *navette* design descriptors, with a
//! Pareto front inside each cell (the MOME feature the strategy asks for).
//!
//! Two descriptor families are kept separate, as E09 v2 §2.6 requires:
//!
//! * the **standard archive** uses layer count and total thickness, both read
//!   directly from a [`DesignStack`];
//! * the **gradient archive** can consume a caller-supplied descriptor vector
//!   (for example a thickness-Jacobian summary once navette's A2 door lands).
//!
//! Keeping the archive independent of a particular genotype also means the GA,
//! the memetic operator, and a future DQD loop all illuminate the same cells.

use std::collections::BTreeMap;

use navette::smatrix::synthesis::structure::DesignStack;

/// Grid definition for the standard archive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArchiveConfig {
    /// Number of layer-count bins. Must be at least one.
    pub depth_bins: usize,
    /// Number of total-thickness bins. Must be at least one.
    pub thickness_bins: usize,
    /// Upper edge of the layer-count axis. Values above are clamped into the
    /// last bin; the axis is inclusive at this value.
    pub max_layers: f64,
    /// Upper edge of the total-thickness axis, in nm.
    pub max_thickness_nm: f64,
    /// Maximum Pareto points retained in one cell.
    pub max_front: usize,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            depth_bins: 8,
            thickness_bins: 8,
            max_layers: 40.0,
            max_thickness_nm: 20_000.0,
            max_front: 32,
        }
    }
}

impl ArchiveConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.depth_bins == 0 || self.thickness_bins == 0 {
            return Err("QD archive needs at least one bin on each axis".into());
        }
        if !self.max_layers.is_finite() || self.max_layers <= 0.0 {
            return Err("QD archive max_layers must be finite and positive".into());
        }
        if !self.max_thickness_nm.is_finite() || self.max_thickness_nm <= 0.0 {
            return Err("QD archive max_thickness_nm must be finite and positive".into());
        }
        if self.max_front == 0 {
            return Err("QD archive max_front must be at least one".into());
        }
        Ok(())
    }
}

/// Behavioural descriptor read from a design.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DesignDescriptor {
    pub layer_count: usize,
    pub total_thickness_nm: f64,
}

impl DesignDescriptor {
    /// Read the standard descriptor directly from a navette stack.
    pub fn from_stack(stack: &DesignStack) -> Self {
        let total_thickness_nm = stack.films().iter().map(|film| film.d_nm).sum();
        Self {
            layer_count: stack.films().len(),
            total_thickness_nm,
        }
    }

    /// Map to the archive's integer cell key.
    pub fn key(&self, cfg: &ArchiveConfig) -> [usize; 2] {
        let depth = self.layer_count as f64;
        let depth_bin = ((depth / cfg.max_layers) * cfg.depth_bins as f64).floor() as isize;
        let thickness_bin = ((self.total_thickness_nm / cfg.max_thickness_nm)
            * cfg.thickness_bins as f64)
            .floor() as isize;
        [
            depth_bin.clamp(0, cfg.depth_bins as isize - 1) as usize,
            thickness_bin.clamp(0, cfg.thickness_bins as isize - 1) as usize,
        ]
    }
}

/// A2-derived descriptor for the DQD loop: two summaries of the thickness
/// residual Jacobian, both non-negative.
///
/// The Jacobian prices thickness moves only; it is not an insertion-position
/// gradient. The two axes are the mean and maximum column L2 norm, normalized
/// by the caller's chosen scale before they enter the archive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientDescriptor {
    pub mean_column_norm: f64,
    pub max_column_norm: f64,
}

impl GradientDescriptor {
    pub fn from_jacobian(values: &[f64], rows: usize, cols: usize) -> Result<Self, String> {
        if rows == 0 || cols == 0 || values.len() != rows * cols {
            return Err(format!(
                "gradient descriptor: {} values for {rows}x{cols}",
                values.len()
            ));
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err("gradient descriptor: Jacobian has a non-finite entry".into());
        }
        let mut norms = Vec::with_capacity(cols);
        for col in 0..cols {
            let norm = (0..rows)
                .map(|row| values[row * cols + col].powi(2))
                .sum::<f64>()
                .sqrt();
            norms.push(norm);
        }
        Ok(Self {
            mean_column_norm: norms.iter().sum::<f64>() / cols as f64,
            max_column_norm: norms.iter().copied().fold(0.0, f64::max),
        })
    }

    /// Normalize both axes into `[0, 1]` for [`MomeArchive::insert_at`].
    pub fn normalized(self, max_mean: f64, max_max: f64) -> Result<[f64; 2], String> {
        if !max_mean.is_finite() || !max_max.is_finite() || max_mean <= 0.0 || max_max <= 0.0 {
            return Err("gradient descriptor scales must be finite and positive".into());
        }
        Ok([
            (self.mean_column_norm / max_mean).clamp(0.0, 1.0),
            (self.max_column_norm / max_max).clamp(0.0, 1.0),
        ])
    }
}

/// One nondominated design in a cell.
#[derive(Clone, Debug)]
pub struct FrontPoint {
    pub merit: f64,
    pub total_thickness_nm: f64,
    pub layer_count: usize,
    pub stack: DesignStack,
}

impl FrontPoint {
    pub fn new(merit: f64, stack: DesignStack) -> Result<Self, String> {
        if !merit.is_finite() {
            return Err("QD point merit must be finite".into());
        }
        let descriptor = DesignDescriptor::from_stack(&stack);
        Ok(Self {
            merit,
            total_thickness_nm: descriptor.total_thickness_nm,
            layer_count: descriptor.layer_count,
            stack,
        })
    }

    /// Both objectives are minimised: merit and total thickness.
    fn objectives(&self) -> [f64; 2] {
        [self.merit, self.total_thickness_nm]
    }

    fn dominates(&self, other: &Self) -> bool {
        let a = self.objectives();
        let b = other.objectives();
        a[0] <= b[0] && a[1] <= b[1] && (a[0] < b[0] || a[1] < b[1])
    }
}

/// One occupied archive cell and its local Pareto front.
#[derive(Clone, Debug)]
pub struct ArchiveCell {
    pub key: [usize; 2],
    pub front: Vec<FrontPoint>,
}

impl ArchiveCell {
    pub fn best(&self) -> Option<&FrontPoint> {
        self.front.iter().min_by(|a, b| a.merit.total_cmp(&b.merit))
    }

    pub fn is_on_front(&self, point: &FrontPoint) -> bool {
        !self.front.iter().any(|held| held.dominates(point))
    }
}

/// MAP-Elites grid with a per-cell Pareto front.
#[derive(Clone, Debug)]
pub struct MomeArchive {
    cfg: ArchiveConfig,
    cells: BTreeMap<[usize; 2], ArchiveCell>,
}

impl MomeArchive {
    pub fn new(cfg: ArchiveConfig) -> Result<Self, String> {
        cfg.validate()?;
        Ok(Self {
            cfg,
            cells: BTreeMap::new(),
        })
    }

    pub fn config(&self) -> &ArchiveConfig {
        &self.cfg
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cells(&self) -> impl Iterator<Item = &ArchiveCell> {
        self.cells.values()
    }

    pub fn cell(&self, key: [usize; 2]) -> Option<&ArchiveCell> {
        self.cells.get(&key)
    }

    /// Insert one evaluated design. Returns whether it changed the archive.
    pub fn insert(&mut self, point: FrontPoint) -> bool {
        let descriptor = DesignDescriptor {
            layer_count: point.layer_count,
            total_thickness_nm: point.total_thickness_nm,
        };
        let key = descriptor.key(&self.cfg);
        let cell = self.cells.entry(key).or_insert_with(|| ArchiveCell {
            key,
            front: Vec::new(),
        });
        if cell.front.iter().any(|held| held.dominates(&point)) {
            return false;
        }
        let before = cell.front.len();
        cell.front.retain(|held| !point.dominates(held));
        cell.front.push(point);
        // The front is nondominated, so trimming must remove only the most
        // crowded end. Keep the lowest-merit half; the thickness axis still
        // spans the retained points.
        if cell.front.len() > self.cfg.max_front {
            cell.front.sort_by(|a, b| a.merit.total_cmp(&b.merit));
            cell.front.truncate(self.cfg.max_front);
        }
        before != cell.front.len() || cell.front.iter().any(|p| p.merit.is_finite())
    }

    /// Insert a point at a caller-supplied normalized descriptor. This is the
    /// DQD/gradient door; the standard [`Self::insert`] derives its key from
    /// layer count and total thickness instead.
    pub fn insert_at(&mut self, descriptor: [f64; 2], point: FrontPoint) -> bool {
        if descriptor
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return false;
        }
        let key = [
            ((descriptor[0] * self.cfg.depth_bins as f64).floor() as usize)
                .min(self.cfg.depth_bins - 1),
            ((descriptor[1] * self.cfg.thickness_bins as f64).floor() as usize)
                .min(self.cfg.thickness_bins - 1),
        ];
        let cell = self.cells.entry(key).or_insert_with(|| ArchiveCell {
            key,
            front: Vec::new(),
        });
        if cell.front.iter().any(|held| held.dominates(&point)) {
            return false;
        }
        cell.front.retain(|held| !point.dominates(held));
        cell.front.push(point);
        if cell.front.len() > self.cfg.max_front {
            cell.front.sort_by(|a, b| a.merit.total_cmp(&b.merit));
            cell.front.truncate(self.cfg.max_front);
        }
        true
    }

    /// Insert a stack directly; convenience for GA and memetic callers.
    pub fn insert_stack(&mut self, merit: f64, stack: DesignStack) -> Result<bool, String> {
        Ok(self.insert(FrontPoint::new(merit, stack)?))
    }

    /// Best merit anywhere in the archive.
    pub fn best(&self) -> Option<&FrontPoint> {
        self.cells
            .values()
            .filter_map(ArchiveCell::best)
            .min_by(|a, b| a.merit.total_cmp(&b.merit))
    }

    /// Sum of each occupied cell's best merit. For minimisation, lower is
    /// better; the score is reported together with [`Self::len`] rather than
    /// treated as a scalar fitness.
    pub fn qd_score(&self) -> f64 {
        self.cells
            .values()
            .filter_map(|cell| cell.best().map(|point| point.merit))
            .sum()
    }

    /// Fraction of the configured grid that is occupied.
    pub fn coverage(&self) -> f64 {
        let total = self.cfg.depth_bins * self.cfg.thickness_bins;
        self.cells.len() as f64 / total as f64
    }

    /// All front points, grouped by cell, in deterministic key order.
    pub fn front_points(&self) -> impl Iterator<Item = &FrontPoint> {
        self.cells.values().flat_map(|cell| cell.front.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use navette::smatrix::synthesis::structure::{DesignStack, LayerSpec};

    fn stack(layers: &[(&str, f64)]) -> DesignStack {
        let wavs = 1;
        let mut ambient = LayerSpec::constant("air", 1.0, 0.0, 0.0, wavs);
        ambient.optimize = false;
        ambient.needle = false;
        let mut substrate = LayerSpec::constant("glass", 1.52, 0.0, 0.0, wavs);
        substrate.optimize = false;
        substrate.needle = false;
        let films = layers
            .iter()
            .map(|(name, d)| LayerSpec::constant(name, 2.0, 0.0, *d, wavs))
            .collect();
        DesignStack::with_films(ambient, substrate, films).unwrap()
    }

    #[test]
    fn descriptor_bins_clamp_to_the_grid() {
        let cfg = ArchiveConfig {
            depth_bins: 4,
            thickness_bins: 2,
            max_layers: 40.0,
            max_thickness_nm: 100.0,
            ..ArchiveConfig::default()
        };
        assert_eq!(
            DesignDescriptor {
                layer_count: 0,
                total_thickness_nm: 0.0
            }
            .key(&cfg),
            [0, 0]
        );
        assert_eq!(
            DesignDescriptor {
                layer_count: 40,
                total_thickness_nm: 100.0
            }
            .key(&cfg),
            [3, 1]
        );
        assert_eq!(
            DesignDescriptor {
                layer_count: 400,
                total_thickness_nm: 10_000.0
            }
            .key(&cfg),
            [3, 1]
        );
    }

    #[test]
    fn a_cell_keeps_a_pareto_front_and_drops_dominated_points() {
        let mut archive = MomeArchive::new(ArchiveConfig {
            depth_bins: 1,
            thickness_bins: 1,
            max_layers: 100.0,
            max_thickness_nm: 1000.0,
            max_front: 8,
        })
        .unwrap();
        assert!(archive.insert_stack(10.0, stack(&[("H", 100.0)])).unwrap());
        assert!(archive.insert_stack(9.0, stack(&[("H", 300.0)])).unwrap());
        assert!(!archive.insert_stack(11.0, stack(&[("H", 400.0)])).unwrap());
        let cell = archive.cell([0, 0]).unwrap();
        assert_eq!(cell.front.len(), 2);
        assert_eq!(archive.best().unwrap().merit, 9.0);
        assert_eq!(archive.qd_score(), 9.0);
    }

    #[test]
    fn separate_cells_do_not_replace_each_other() {
        let mut archive = MomeArchive::new(ArchiveConfig {
            depth_bins: 4,
            thickness_bins: 4,
            max_layers: 40.0,
            max_thickness_nm: 400.0,
            max_front: 4,
        })
        .unwrap();
        archive.insert_stack(5.0, stack(&[("H", 50.0)])).unwrap();
        archive
            .insert_stack(4.0, stack(&[("H", 150.0), ("L", 150.0)]))
            .unwrap();
        assert_eq!(archive.len(), 2);
        assert_eq!(archive.front_points().count(), 2);
        assert!((archive.coverage() - 2.0 / 16.0).abs() < 1e-12);
    }

    #[test]
    fn gradient_descriptor_reads_jacobian_columns() {
        let descriptor = GradientDescriptor::from_jacobian(&[3.0, 4.0, 0.0, 0.0], 2, 2).unwrap();
        assert_eq!(descriptor.mean_column_norm, 3.5);
        assert_eq!(descriptor.max_column_norm, 4.0);
        let normalized = descriptor.normalized(7.0, 8.0).unwrap();
        assert!((normalized[0] - 0.5).abs() < 1e-12);
        assert!((normalized[1] - 0.5).abs() < 1e-12);
        assert!(GradientDescriptor::from_jacobian(&[1.0], 2, 2).is_err());
    }

    #[test]
    fn custom_descriptor_inserts_at_a_gradient_cell() {
        let mut archive = MomeArchive::new(ArchiveConfig {
            depth_bins: 2,
            thickness_bins: 2,
            max_layers: 10.0,
            max_thickness_nm: 1000.0,
            max_front: 4,
        })
        .unwrap();
        let point = FrontPoint::new(1.0, stack(&[("H", 10.0)])).unwrap();
        assert!(archive.insert_at([0.75, 0.75], point));
        assert!(archive.cell([1, 1]).is_some());
    }

    #[test]
    fn invalid_configs_are_refused() {
        let cfg = ArchiveConfig {
            depth_bins: 0,
            ..ArchiveConfig::default()
        };
        assert!(MomeArchive::new(cfg).is_err());
        assert!(FrontPoint::new(f64::NAN, stack(&[("H", 10.0)])).is_err());
    }
}
