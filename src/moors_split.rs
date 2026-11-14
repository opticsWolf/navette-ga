// SPDX-License-Identifier: LGPL-3.0-or-later
//! S6 — multi-objective split-off.
//!
//! `moors` owns the non-dominated sorting. This module only maps a navette
//! design's merit, total thickness, and layer count into the objective matrix,
//! then returns the fronts and the first front. That keeps the MOO decision
//! (which objectives, in which direction) in one place and leaves the
//! algorithm to the dependency.
//!
//! All objectives are minimised. The first front is the split-off candidate
//! set: a caller can hand it to [`crate::study::Campaign`] for a guided run or
//! report it directly.

use navette::smatrix::synthesis::structure::DesignStack;
use ndarray::Array2;

/// One candidate's objective vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DesignPoint {
    pub merit: f64,
    pub total_thickness_nm: f64,
    pub layer_count: usize,
}

impl DesignPoint {
    pub fn new(merit: f64, total_thickness_nm: f64, layer_count: usize) -> Result<Self, String> {
        if !merit.is_finite() || !total_thickness_nm.is_finite() || total_thickness_nm < 0.0 {
            return Err("MOO point needs finite merit and a non-negative thickness".into());
        }
        Ok(Self {
            merit,
            total_thickness_nm,
            layer_count,
        })
    }

    pub fn from_stack(merit: f64, stack: &DesignStack) -> Result<Self, String> {
        Self::new(
            merit,
            stack.films().iter().map(|film| film.d_nm).sum(),
            stack.films().len(),
        )
    }

    fn row(self) -> [f64; 3] {
        [self.merit, self.total_thickness_nm, self.layer_count as f64]
    }
}

/// Fronts in dominance order; `fronts[0]` is the non-dominated set.
#[derive(Clone, Debug, PartialEq)]
pub struct ParetoSplit {
    pub fronts: Vec<Vec<usize>>,
}

impl ParetoSplit {
    pub fn front(&self) -> &[usize] {
        self.fronts.first().map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn first_front_points<'a>(
        &'a self,
        points: &'a [DesignPoint],
    ) -> impl Iterator<Item = &'a DesignPoint> {
        self.front().iter().map(|&index| &points[index])
    }
}

/// Run `moors`' fast non-dominated sort on the objective matrix.
pub fn split(points: &[DesignPoint], min_survivors: usize) -> Result<ParetoSplit, String> {
    if points.is_empty() {
        return Err("MOO split needs at least one point".into());
    }
    let rows = points.len();
    let mut matrix = Vec::with_capacity(rows * 3);
    for point in points {
        matrix.extend(point.row());
    }
    let matrix = Array2::from_shape_vec((rows, 3), matrix)
        .map_err(|error| format!("MOO objective matrix: {error}"))?;
    let min_survivors = min_survivors.max(1).min(rows);
    Ok(ParetoSplit {
        fronts: moors::non_dominated_sorting::fast_non_dominated_sorting(&matrix, min_survivors),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_keeps_a_tradeoff_front_and_drops_dominated_points() {
        let points = [
            DesignPoint::new(1.0, 100.0, 4).unwrap(),
            DesignPoint::new(2.0, 50.0, 3).unwrap(),
            DesignPoint::new(2.0, 200.0, 8).unwrap(),
            DesignPoint::new(3.0, 40.0, 3).unwrap(),
        ];
        let split = split(&points, 4).unwrap();
        assert_eq!(split.front(), &[0, 1, 3]);
        assert!(!split.front().contains(&2));
        assert_eq!(split.front().len(), 3);
    }

    #[test]
    fn invalid_points_are_refused() {
        assert!(DesignPoint::new(f64::NAN, 10.0, 1).is_err());
        assert!(DesignPoint::new(1.0, -1.0, 1).is_err());
        assert!(split(&[], 1).is_err());
    }
}
