// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Reproducible private-corpus appearance audit; writes JSONL, never source assets.
//! Run with `--features archive --example inspect_appearances -- ARCHIVE [--cached] [--hypotheses]`.
#[cfg(feature = "archive")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut args = std::env::args().skip(1);
    let archive = args
        .next()
        .ok_or("usage: inspect_appearances ARCHIVE [--cached] [--hypotheses]")?;
    let mut cached = false;
    let mut hypotheses = false;
    for argument in args {
        match argument.as_str() {
            "--cached" => cached = true,
            "--hypotheses" => hypotheses = true,
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().to_owned();
        if !name.to_ascii_lowercase().ends_with(".sldprt")
            && !name.to_ascii_lowercase().ends_with(".sldasm")
        {
            continue;
        }
        if name.rsplit('/').next().is_some_and(|n| n.starts_with("~$")) {
            continue;
        }
        let mut bytes = vec![];
        entry.read_to_end(&mut bytes)?;
        let hash = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let doc = match sw_rs::parse_with_limits(&bytes, &name, sw_rs::Limits::unlimited()) {
            Ok(doc) => doc,
            Err(error) => {
                println!(
                    "{}",
                    serde_json::json!({"path":name,"sha256":hash,"error":error.to_string()})
                );
                continue;
            }
        };
        let mut presentation = doc.presentation();
        let mut cache_error = None;
        let mut tables = None;
        let mut triangles = None;
        if cached && doc.document_type == "part" {
            match sw_rs::geometry_with_limits(&bytes, &name, sw_rs::Limits::unlimited()) {
                Ok(geometry) => {
                    tables = Some(geometry.model.tessellations.len());
                    triangles = Some(
                        geometry
                            .model
                            .tessellations
                            .iter()
                            .map(|m| m.triangles.len())
                            .sum::<usize>(),
                    );
                    presentation = geometry.presentation;
                }
                Err(error) => cache_error = Some(error.to_string()),
            }
        }
        let document_bindings = presentation
            .appearance_bindings
            .iter()
            .filter(|b| matches!(b.target, sw_rs::AppearanceTarget::Document))
            .count();
        let cached_bindings = presentation.appearance_bindings.len() - document_bindings;
        let native = presentation
            .appearances
            .iter()
            .filter_map(|a| a.native.as_ref())
            .collect::<Vec<_>>();
        let extensions = native.iter().filter(|a| a.extension.is_some()).count();
        let unknown_states = presentation
            .display_states
            .iter()
            .filter(|s| matches!(s.configuration, sw_rs::PresentationScope::Unknown))
            .count();
        let mapping_hypotheses = hypotheses.then(|| {
            presentation
                .appearances
                .iter()
                .enumerate()
                .filter_map(|(index, appearance)| {
                    let extension = appearance.native.as_ref()?.extension.as_ref()?;
                    Some(serde_json::json!({
                        "appearance_index": index, "source": appearance.source,
                        "raw": extension, "hypothesis": extension.texture_mapping_hypothesis(),
                    }))
                })
                .collect::<Vec<_>>()
        });
        println!(
            "{}",
            serde_json::json!({"path":name,"sha256":hash,"mapping_hypotheses":mapping_hypotheses,"configuration_names":doc.configurations.iter().map(|c| &c.name).collect::<Vec<_>>(),
            "definitions":native.len(),"extensions":extensions,"display_keys":presentation.display_states,"unknown_state_keys":unknown_states,
            "document_bindings":document_bindings,"cached_bindings":cached_bindings,"unresolved":presentation.unresolved_visual_bindings,
            "diagnostics":presentation.visual_diagnostics,"tables":tables,"triangles":triangles,"cache_error":cache_error})
        );
    }
    Ok(())
}
#[cfg(not(feature = "archive"))]
fn main() {
    eprintln!("enable --features archive");
}
