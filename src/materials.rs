// SPDX-License-Identifier: LGPL-3.0-or-later
//! G3 — rung-2 material-pool loader.
//!
//! Navette owns materials-as-data ([`MaterialDef`]) and evaluates a library on
//! the design wavelength grid. This module turns that ordered evaluation into
//! the two objects the GA needs: a decoder [`MaterialPool`] with per-material
//! thickness bounds, and a matching list of [`LayerSpec`]s for a
//! [`StackTemplate`]. The pool is sorted by a cheap rung-2 similarity key
//! (mean index, then dispersion slope, then label) so adjacent material keys
//! are optically close and an adjacent swap is a small move.
//!
//! Rung 3 (latent material geometry) is deliberately absent: that is a later
//! offline embedding step, not a different loader contract.

use std::collections::BTreeMap;
use std::sync::Arc;

use navette::smatrix::synthesis::design_config::{evaluate_library, MaterialDef};
use navette::smatrix::synthesis::structure::LayerSpec;

use crate::bridge::StackTemplate;
use crate::decoder::{DecodeError, Material, MaterialPool};

/// Closed thickness interval for one material, in nm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThicknessBounds {
    pub min_nm: f64,
    pub max_nm: f64,
}

impl ThicknessBounds {
    pub fn new(min_nm: f64, max_nm: f64) -> Result<Self, String> {
        if !min_nm.is_finite() || !max_nm.is_finite() || min_nm < 0.0 || max_nm < min_nm {
            return Err(format!("invalid thickness bounds [{min_nm}, {max_nm}] nm"));
        }
        Ok(Self { min_nm, max_nm })
    }
}

/// One evaluated material, ready for both sides of the bridge.
#[derive(Clone, Debug)]
pub struct LoadedMaterial {
    /// Library code, or the name when no code is supplied.
    pub label: String,
    pub film: LayerSpec,
    pub min_nm: f64,
    pub max_nm: f64,
}

impl LoadedMaterial {
    pub fn decoder_material(&self) -> Result<Material, DecodeError> {
        Material::new(self.label.clone(), self.min_nm, self.max_nm)
    }
}

/// Load and order a material library on `wavelengths`.
///
/// `bounds` is keyed by library label (the code when present, otherwise the
/// name). Every library entry must have a unique label and a bounds entry;
/// missing or duplicate data is an error rather than a silent default.
pub fn load_library(
    library: &[MaterialDef],
    wavelengths: &[f64],
    bounds: &BTreeMap<String, ThicknessBounds>,
) -> Result<Vec<LoadedMaterial>, String> {
    if library.is_empty() {
        return Err("material library is empty".into());
    }
    if wavelengths.is_empty() {
        return Err("material wavelength grid is empty".into());
    }

    let evaluated = evaluate_library(library, wavelengths)?;
    let mut loaded = Vec::with_capacity(evaluated.len());
    for (index, (label, nk)) in evaluated.into_iter().enumerate() {
        let def = &library[index];
        let bounds = bounds
            .get(&label)
            .or_else(|| bounds.get(&def.name))
            .copied()
            .ok_or_else(|| format!("material {label:?} has no thickness bounds"))?;
        let mean_n = nk.iter().map(|value| value.re).sum::<f64>() / nk.len() as f64;
        let dispersion = if nk.len() > 1 {
            (nk[0].re - nk[nk.len() - 1].re) / (wavelengths[wavelengths.len() - 1] - wavelengths[0])
        } else {
            0.0
        };
        loaded.push((
            mean_n,
            dispersion,
            LoadedMaterial {
                label,
                film: LayerSpec {
                    material: Arc::from(
                        library[index]
                            .code
                            .as_deref()
                            .unwrap_or(&library[index].name),
                    ),
                    nk: nk.into(),
                    d_nm: 0.0,
                    coherent: true,
                    rough_type: 0,
                    rough_val: 0.0,
                    optimize: true,
                    needle: true,
                    label: None,
                },
                min_nm: bounds.min_nm,
                max_nm: bounds.max_nm,
            },
        ));
    }

    loaded.sort_by(|(a_mean, a_disp, a), (b_mean, b_disp, b)| {
        a_mean
            .total_cmp(b_mean)
            .then_with(|| a_disp.total_cmp(b_disp))
            .then_with(|| a.label.cmp(&b.label))
    });
    Ok(loaded
        .into_iter()
        .map(|(_, _, material)| material)
        .collect())
}

/// Build the decoder pool in the loader's similarity order.
pub fn decoder_pool(materials: &[LoadedMaterial]) -> Result<MaterialPool, String> {
    let pool = materials
        .iter()
        .map(LoadedMaterial::decoder_material)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    MaterialPool::new(pool).map_err(|error| error.to_string())
}

/// The material films, in the same order as [`decoder_pool`].
pub fn films(materials: &[LoadedMaterial]) -> Vec<LayerSpec> {
    materials
        .iter()
        .map(|material| material.film.clone())
        .collect()
}

/// A stack template whose material order matches [`decoder_pool`].
pub fn stack_template(
    ambient: LayerSpec,
    substrate: LayerSpec,
    materials: &[LoadedMaterial],
) -> Result<StackTemplate, String> {
    StackTemplate::new(ambient, substrate, films(materials))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use navette::smatrix::synthesis::design_config::{MaterialDef, TabData};

    fn table(name: &str, n: f64) -> MaterialDef {
        MaterialDef {
            name: name.to_string(),
            code: None,
            model: "TableMaterial".to_string(),
            params: BTreeMap::new(),
            n_data: Some(TabData {
                wavelengths: vec![400.0, 700.0],
                values: vec![n, n],
            }),
            k_data: None,
        }
    }

    fn bounds(entries: &[(&str, f64, f64)]) -> BTreeMap<String, ThicknessBounds> {
        entries
            .iter()
            .map(|(name, min, max)| (name.to_string(), ThicknessBounds::new(*min, *max).unwrap()))
            .collect()
    }

    #[test]
    fn loader_orders_by_index_and_builds_matching_pool() {
        let materials = load_library(
            &[table("H", 2.3), table("L", 1.45)],
            &[400.0, 700.0],
            &bounds(&[("H", 20.0, 300.0), ("L", 10.0, 200.0)]),
        )
        .unwrap();
        assert_eq!(materials.len(), 2);
        assert_eq!(materials[0].label, "L");
        assert_eq!(materials[1].label, "H");
        assert_eq!(materials[0].film.nk[0].re, 1.45);
        assert_eq!(materials[1].film.nk[1].re, 2.3);

        let pool = decoder_pool(&materials).unwrap();
        assert_eq!(pool.materials()[0].name(), "L");
        assert_eq!(pool.materials()[0].min_thickness_nm(), 10.0);
        assert_eq!(pool.materials()[1].max_thickness_nm(), 300.0);
    }

    #[test]
    fn loader_refuses_missing_bounds_and_empty_library() {
        let err = load_library(
            &[table("H", 2.3)],
            &[400.0, 700.0],
            &bounds(&[("L", 10.0, 200.0)]),
        )
        .unwrap_err();
        assert!(err.contains("no thickness bounds"), "{err}");

        let err = load_library(&[], &[400.0, 700.0], &bounds(&[])).unwrap_err();
        assert!(err.contains("empty"), "{err}");
    }
}
