// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Inspector labels for explicit saved configuration material assignments.
use serde_json::Value;

pub(crate) fn lines(metadata: &Value, configuration: usize) -> Vec<String> {
    let materials = &metadata["materials"];
    let assignment = &metadata["configurations"][configuration]["material"];
    let Some(reference) = assignment["material_reference"].as_str() else {
        return vec![if materials.is_object() {
            "Material: no unique saved assignment for this configuration".into()
        } else {
            "Material: no supported saved assignment".into()
        }];
    };
    let definition = assignment["definition_index"]
        .as_u64()
        .and_then(|i| usize::try_from(i).ok())
        .and_then(|i| materials["definitions"].as_array()?.get(i));
    let Some(definition) = definition else {
        return vec![format!(
            "Saved material reference: {reference} (unresolved)"
        )];
    };
    let mut result = vec![format!(
        "Saved material: {}",
        definition["name"].as_str().unwrap_or(reference)
    )];
    if let Some(classifications) = definition["classifications"].as_array() {
        let path = classifications
            .iter()
            .filter_map(Value::as_str)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" / ");
        if !path.is_empty() {
            result.push(format!("Material classification: {path}"));
        }
    }
    if let Some(value) = definition["density"]["value"].as_str() {
        let units = definition["density"]["units"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or("units unverified");
        result.push(format!("Saved density token: {value} ({units})"));
    }
    for (field, label) in [
        ("shaders", "Saved shader"),
        ("texture_references", "Saved texture reference"),
    ] {
        if let Some(records) = definition["appearance"][field].as_array() {
            for record in records {
                let attributes = &record["attributes"];
                if let Some(value) = attributes["name"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .or_else(|| attributes["path"].as_str().filter(|s| !s.is_empty()))
                {
                    result.push(format!("{label}: {value}"));
                }
            }
        }
    }
    result.push("Material source: saved MATERIALTREE configuration reference".into());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn uses_only_the_selected_configurations_explicit_reference() {
        let metadata = json!({"materials":{"definitions":[
            {"name":"library|Aluminum", "classifications":["Metals","Aluminum"],
             "density":{"value":"2700","units":null}},
            {"name":"library|Composite", "density":{"value":"1600","units":"kg/m³"}}
        ]},"configurations":[
            {"material":{"material_reference":"library|Composite|2","definition_index":1}},
            {"material":null}
        ]});
        let assigned = lines(&metadata, 0).join("\n");
        assert!(assigned.contains("library|Composite"));
        assert!(assigned.contains("1600 (kg/m³)"));
        assert!(!assigned.contains("Aluminum"));
        assert!(lines(&metadata, 1)[0].contains("no unique saved assignment"));
        assert!(lines(&metadata, 2)[0].contains("no unique saved assignment"));
    }

    #[test]
    fn missing_and_unresolved_records_do_not_invent_materials_or_units() {
        assert!(lines(&Value::Null, 0)[0].contains("no supported saved assignment"));
        let mut metadata = json!({"materials":{"definitions":[
            {"name":"6061", "density":{"value":"2700"}}
        ]},"configurations":[{"material":{
            "material_reference":"missing|1", "definition_index":null
        }}]});
        assert_eq!(
            lines(&metadata, 0),
            ["Saved material reference: missing|1 (unresolved)"]
        );
        metadata["configurations"][0]["material"]["definition_index"] = 0.into();
        assert!(
            lines(&metadata, 0)
                .join("\n")
                .contains("2700 (units unverified)")
        );
    }
}
