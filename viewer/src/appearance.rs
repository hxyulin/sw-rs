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

/// Choose an approximate visualization only for an exact, uniquely named configuration
/// with an explicit resolved material reference. Unassigned definitions, custom properties,
/// filenames, shader names and density values never establish a material assignment.
/// Configuration assignment does not imply recovered face/body appearance ownership.
pub fn for_configuration(doc: &sw_rs::Document, name: &str) -> Option<MaterialAppearance> {
    if name.is_empty() {
        return None;
    }
    let mut configurations = doc.configurations.iter().filter(|c| c.name == name);
    let configuration = configurations.next()?;
    if configurations.next().is_some() {
        return None;
    }
    let assignment = configuration.material.as_ref()?;
    if assignment.configuration != name {
        return None;
    }
    let materials = doc.materials.as_ref()?;
    let definition = materials.definitions.get(assignment.definition_index?)?;
    // Recheck the public records rather than trusting arbitrary caller-mutated indices.
    let key = match &definition.material_id {
        Some(id) => format!("{}|{id}", definition.name),
        None => definition.name.clone(),
    };
    if key.is_empty() || assignment.material_reference != key {
        return None;
    }
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
          <classification name="铝合金"><material name="库|6061" matid="21"><swatchcolor RGB="aBcD00"/></material></classification>
          <classification name="碳纤维"><material name="自定义材料|VMA"/></classification>
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
