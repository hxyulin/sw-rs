// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Approximate visualization presets from explicit saved configuration assignments.
//! These presets are neither original CAD appearances nor measured optical properties.
use serde::Serialize;

/// A procedural visualization preset backed by one resolved saved assignment.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct MaterialAppearance {
    /// Original saved material name, without relabeling the source evidence.
    pub name: String,
    /// Stable visualization family: `aluminum`, `carbon_fiber`, or `generic`.
    pub family: String,
    /// Display label explicitly distinguishing the preset from original CAD appearance.
    pub label: String,
    /// Approximate sRGB channel values in [0, 1], decoded to linear light by the shader.
    /// Saved RGB swatches have no recovered color profile; sRGB is a viewer convention.
    pub base_color: [f32; 3],
    /// Artistic roughness in [0, 1], not a measured surface parameter.
    pub roughness: f32,
    /// Artistic metallic blend in [0, 1], not a measured material property.
    pub metallic: f32,
    /// Procedural pattern: `brushed_aluminum`, `carbon_weave`, or `none`.
    pub pattern: String,
    /// Approximation of the saved material's default shader, when present.
    /// This remains separate from its physical family and from face overrides.
    pub saved_surface: Option<SavedSurface>,
    /// Exact selected configuration name supporting this preset.
    pub configuration: String,
    /// Exact saved material key supporting this preset.
    pub material_reference: String,
    /// Container stream supplying the material evidence.
    pub source_stream: String,
    /// XML element path of the saved definition.
    pub definition_source_path: String,
    /// XML element path of the explicit configuration assignment.
    pub assignment_source_path: String,
}

/// Viewer reconstruction of a saved material appearance reference.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SavedSurface {
    /// Exact saved shader name or path; never a physical-material inference.
    pub name: String,
    /// Approximate sRGB color, using the saved swatch when available.
    pub base_color: [f32; 3],
    /// Artistic GGX roughness; the source shader implementation is unavailable.
    pub roughness: f32,
    /// Approximate metal response inferred from the explicit shader identifier.
    pub metallic: f32,
    /// Procedural reconstruction, without original texture pixels or UVs.
    pub pattern: String,
    /// Saved diffuse coefficient when valid, otherwise the viewer default.
    pub diffuse: f32,
    /// Saved specularity coefficient when valid, otherwise the viewer default.
    pub specularity: f32,
    /// Saved emission coefficient when valid, otherwise zero.
    pub emission: f32,
    /// Raw referenced texture paths; no files are opened or downloaded.
    pub texture_references: Vec<String>,
}

fn saved_surface(definition: &sw_rs::SavedMaterial) -> Option<SavedSurface> {
    let saved = definition.appearance.as_ref()?;
    // Saved PhotoWorks paths/names are more specific than generic realtime cg
    // fallbacks (the library contains satin aluminum paired with defaultplastic).
    let mut chosen = None;
    for names in [["pwshader2", "pwshader"], ["cgshader2", "cgshader"]] {
        let candidates: std::collections::BTreeSet<_> = saved
            .shaders
            .iter()
            .filter(|r| names.contains(&r.element.as_str()))
            .filter_map(|r| {
                let name = r.attributes.get("name").filter(|s| !s.trim().is_empty());
                let path = r.attributes.get("path").filter(|s| !s.trim().is_empty());
                name.or(path)
                    .map(|label| (label.clone(), path.cloned().unwrap_or_default()))
            })
            .collect();
        if candidates.len() > 1 {
            return None;
        }
        if let Some(value) = candidates.into_iter().next() {
            chosen = Some(value);
            break;
        }
    }
    let (name, path) = chosen.unwrap_or_else(|| ("Saved optical swatch".into(), String::new()));
    let identifier = format!("{name} {path}")
        .to_lowercase()
        .replace([' ', '_', '-', '\\', '/'], "");
    let color = definition
        .saved_swatch_rgb
        .as_deref()
        .and_then(swatch)
        .unwrap_or([0.55, 0.59, 0.64]);
    let cast_texture = saved.texture_references.iter().any(|r| {
        r.attributes.get("path").is_some_and(|p| {
            let p = p.to_ascii_lowercase();
            p.contains("cast_fine") || p.contains("cast_rough")
        })
    });
    let (metallic, roughness, pattern, base_color) =
        if identifier.contains("carbonfiber") || identifier.contains("carbonfibre") {
            (0.05, 0.36, "carbon_weave", color.map(|v| v * 0.16))
        } else if identifier.contains("glassfiber") || identifier.contains("glassfibre") {
            (0., 0.5, "fiber_weave", color)
        } else if identifier.contains("aluminum") || identifier.contains("aluminium") {
            (
                0.88,
                0.38,
                if cast_texture {
                    "fine_grain"
                } else {
                    "brushed_aluminum"
                },
                color,
            )
        } else if identifier.contains("steel")
            || identifier.contains("chrome")
            || identifier.contains("chromium")
            || identifier.contains("gold")
            || identifier.contains("iron")
        {
            let polished = identifier.contains("polished")
                || identifier.contains("chrome")
                || identifier.contains("chromium");
            (
                0.92,
                if polished { 0.18 } else { 0.5 },
                if polished { "none" } else { "fine_grain" },
                color,
            )
        } else if identifier.contains("textured") || identifier.contains("mt11050") {
            (0., 0.62, "fine_grain", color)
        } else {
            (
                0.,
                if identifier.contains("lowgloss") || identifier.contains("matte") {
                    0.72
                } else {
                    0.5
                },
                "none",
                color,
            )
        };
    let optical = |key: &str, fallback: f32| {
        if saved.optical.len() != 1 {
            return fallback;
        }
        saved.optical[0]
            .attributes
            .get(key)
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
            .unwrap_or(fallback)
    };
    Some(SavedSurface {
        name,
        base_color,
        roughness,
        metallic,
        pattern: pattern.into(),
        diffuse: optical("Diffuse", 1.),
        specularity: optical("Specularity", 1.),
        emission: optical("Emission", 0.),
        texture_references: saved
            .texture_references
            .iter()
            .filter_map(|r| r.attributes.get("path").cloned())
            .collect(),
    })
}

/// Choose an approximate visualization only for an exact, uniquely named configuration
/// with an explicit resolved material reference. Unassigned definitions, custom properties,
/// filenames, shader names and density values never establish a material assignment.
/// Configuration assignment does not imply recovered face/body appearance ownership.
pub fn for_configuration(doc: &sw_rs::Document, name: &str) -> Option<MaterialAppearance> {
    let definition = doc.physical_material_for_configuration(name)?;
    let configuration = doc.configurations.iter().find(|c| c.name == name)?;
    let assignment = configuration.material.as_ref()?;
    let materials = doc.materials.as_ref()?;
    let labels = std::iter::once(definition.name.as_str())
        .chain(definition.classifications.iter().map(String::as_str))
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    let carbon = labels.iter().any(|s| {
        ["碳纤维", "carbon fiber", "carbon fibre", "thornel mat vma"]
            .iter()
            .any(|token| s.contains(token))
    });
    let aluminum = labels.iter().any(|s| {
        ["铝合金", "aluminum", "aluminium"]
            .iter()
            .any(|token| s.contains(token))
    });
    let swatch = definition.saved_swatch_rgb.as_deref().and_then(swatch);
    let (family, label, base_color, roughness, metallic, pattern) = if carbon {
        (
            "carbon_fiber",
            "Approximate carbon fiber visualization",
            [0.09, 0.105, 0.12],
            0.36,
            0.05,
            "carbon_weave",
        )
    } else if aluminum {
        (
            "aluminum",
            "Approximate aluminum visualization",
            swatch.unwrap_or([0.72, 0.75, 0.78]),
            0.38,
            0.88,
            "brushed_aluminum",
        )
    } else {
        (
            "generic",
            "Approximate saved-swatch visualization",
            swatch.unwrap_or([0.55, 0.59, 0.64]),
            0.6,
            0.0,
            "none",
        )
    };
    Some(MaterialAppearance {
        name: definition.name.clone(),
        family: family.into(),
        label: label.into(),
        base_color,
        roughness,
        metallic,
        pattern: pattern.into(),
        saved_surface: saved_surface(definition),
        configuration: name.into(),
        material_reference: assignment.material_reference.clone(),
        source_stream: materials.source_stream.clone(),
        definition_source_path: definition.source_path.clone(),
        assignment_source_path: assignment.source_path.clone(),
    })
}

fn swatch(value: &str) -> Option<[f32; 3]> {
    if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let rgb = u32::from_str_radix(value, 16).ok()?;
    Some([
        ((rgb >> 16) & 255) as f32 / 255.0,
        ((rgb >> 8) & 255) as f32 / 255.0,
        (rgb & 255) as f32 / 255.0,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc() -> sw_rs::Document {
        let xml = r#"<m:materials xmlns:m="http://www.solidworks.com/sldmaterials">
          <classification name="铝合金"><material name="库|6061" matid="21"><shaders><pwshader2 name="satin finish aluminum" path="metal/aluminum/satin finish aluminum.p2m"/><cgshader2 name="defaultplastic"/><swtexture path="images/textures/metal/cast/cast_fine.jpg"/></shaders><swatchcolor RGB="aBcD00"><Optical Diffuse="0.7" Specularity="0.5" Emission="0.05"/></swatchcolor></material></classification>
          <classification name="碳纤维"><material name="自定义材料|VMA"><shaders><pwshader2 name="black low gloss plastic"/></shaders><swatchcolor RGB="707070"/></material></classification>
          <classification name="Steel"><material name="library|Carbon Steel"><swatchcolor RGB="804020"/></material></classification>
          <configurationslist><configuration name="默认" material="库|6061|21"/><configuration name="碳板" material="自定义材料|VMA"/><configuration name="steel" material="library|Carbon Steel"/><configuration name="broken" material="missing"/></configurationslist>
        </m:materials>"#;
        let tree = r#"<swSolidWorks><swModelList><swModel swConfigurationId="0" swConfigurationName="默认"/><swModel swConfigurationId="1" swConfigurationName="碳板"/><swModel swConfigurationId="2" swConfigurationName="steel"/><swModel swConfigurationId="3" swConfigurationName="broken"/><swModel swConfigurationId="4" swConfigurationName="unassigned"/></swModelList></swSolidWorks>"#;
        sw_rs::metadata_from_container(
            sw_rs::Container {
                streams: [
                    ("swXmlContents/MATERIALTREE".into(), xml.as_bytes().to_vec()),
                    ("docProps/Config-0-Properties.xml".into(), vec![]),
                    ("docProps/Config-1-Properties.xml".into(), vec![]),
                    ("docProps/Config-2-Properties.xml".into(), vec![]),
                    ("docProps/Config-3-Properties.xml".into(), vec![]),
                    ("docProps/Config-4-Properties.xml".into(), vec![]),
                    (
                        "swXmlContents/COMPINSTANCETREE".into(),
                        tree.as_bytes().to_vec(),
                    ),
                ]
                .into(),
                diagnostics: vec![],
            },
            "carbon-fiber-aluminum.SLDPRT",
        )
    }
    #[test]
    fn exact_unicode_assignments_classify_and_retain_evidence() {
        let d = doc();
        let aluminum = for_configuration(&d, "默认").unwrap();
        assert_eq!(aluminum.family, "aluminum");
        assert_eq!(aluminum.pattern, "brushed_aluminum");
        assert_eq!(aluminum.base_color, [171. / 255., 205. / 255., 0.]);
        assert_eq!(aluminum.material_reference, "库|6061|21");
        assert!(aluminum.assignment_source_path.contains("configuration[1]"));
        assert_eq!(
            for_configuration(&d, "碳板").unwrap().family,
            "carbon_fiber"
        );
        assert!(
            serde_json::to_value(aluminum).unwrap()["label"]
                .as_str()
                .unwrap()
                .starts_with("Approximate")
        );
    }
    #[test]
    fn saved_shader_takes_precedence_over_generic_realtime_hint_and_physical_family() {
        let d = doc();
        let aluminum = for_configuration(&d, "默认").unwrap();
        let saved = aluminum.saved_surface.unwrap();
        assert_eq!(saved.name, "satin finish aluminum");
        assert_eq!(saved.pattern, "fine_grain");
        assert_eq!(saved.diffuse, 0.7);
        assert_eq!(saved.specularity, 0.5);
        assert_eq!(saved.emission, 0.05);
        assert!(saved.texture_references[0].ends_with("cast_fine.jpg"));
        let carbon = for_configuration(&d, "碳板").unwrap();
        assert_eq!(carbon.family, "carbon_fiber");
        assert_eq!(carbon.pattern, "carbon_weave");
        let saved = carbon.saved_surface.unwrap();
        assert_eq!(saved.name, "black low gloss plastic");
        assert_eq!(saved.pattern, "none");
        assert_eq!(saved.metallic, 0.0);
        assert_eq!(saved.base_color, [112. / 255.; 3]);
    }
    #[test]
    fn conflicting_shader_records_are_not_arbitrarily_selected() {
        let mut d = doc();
        let saved = d.materials.as_mut().unwrap().definitions[0]
            .appearance
            .as_mut()
            .unwrap();
        let mut extra = saved.shaders[0].clone();
        extra
            .attributes
            .insert("name".into(), "polished steel".into());
        saved.shaders.push(extra);
        assert!(
            for_configuration(&d, "默认")
                .unwrap()
                .saved_surface
                .is_none()
        );
    }
    #[test]
    fn generic_swatch_and_carbon_steel_do_not_become_carbon_fiber() {
        let material = for_configuration(&doc(), "steel").unwrap();
        assert_eq!(material.family, "generic");
        assert_eq!(material.pattern, "none");
        assert_eq!(material.base_color, [128. / 255., 64. / 255., 32. / 255.]);
        assert_eq!(swatch("#abcdef"), None);
        assert_eq!(swatch("zzzzzz"), None);
    }
    #[test]
    fn missing_unassigned_unresolved_and_wrong_case_are_not_materials() {
        let d = doc();
        for name in ["", "missing", "unassigned", "broken", "Steel"] {
            assert!(for_configuration(&d, name).is_none(), "{name}");
        }
    }
    #[test]
    fn stale_public_indices_do_not_override_saved_reference() {
        let mut d = doc();
        d.configurations[0]
            .material
            .as_mut()
            .unwrap()
            .definition_index = Some(1);
        assert!(for_configuration(&d, "默认").is_none());
    }
}
