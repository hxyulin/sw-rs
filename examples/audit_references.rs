// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Reproducible saved-identity census; writes JSONL, never extracts private CAD.
#[cfg(feature = "archive")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{collections::BTreeMap, io::Read};
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or("usage: audit_references ARCHIVE.zip")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().to_owned();
        let lower = name.to_ascii_lowercase();
        if ![".sldprt", ".sldasm", ".slddrw"]
            .iter()
            .any(|ext| lower.ends_with(ext))
            || name.rsplit('/').next().is_some_and(|n| n.starts_with("~$"))
        {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let reader = sw_rs::DocumentReader::open(&bytes, &name, sw_rs::Limits::unlimited())?;
        let value = match reader.references() {
            Err(error) => {
                serde_json::json!({"path":name,"source_sha256":reader.source_sha256(),"error":error.to_string()})
            }
            Ok(graph) => {
                let mut counts = BTreeMap::<String, BTreeMap<String, usize>>::new();
                for link in &graph.links {
                    *counts
                        .entry(
                            serde_json::to_value(link.relation)?
                                .as_str()
                                .unwrap()
                                .into(),
                        )
                        .or_default()
                        .entry(
                            serde_json::to_value(link.resolution)?
                                .as_str()
                                .unwrap()
                                .into(),
                        )
                        .or_default() += 1;
                }
                let mut ids = BTreeMap::<u32, usize>::new();
                for feature in &graph.features.records {
                    if let Some(id) = feature.source_id {
                        *ids.entry(id).or_default() += 1;
                    }
                }
                serde_json::json!({"path":name,"source_sha256":reader.source_sha256(),"features_status":graph.features.status,"features":graph.features.records.len(),"feature_source_ids":ids.len(),"duplicate_feature_ids":ids.values().filter(|&&n|n>1).count(),"nodes":graph.nodes.len(),"links":counts,"native_object_graph_decoded":graph.native_object_graph_decoded,"topology_ownership_decoded":graph.topology_ownership_decoded})
            }
        };
        println!("{value}");
    }
    Ok(())
}
#[cfg(not(feature = "archive"))]
fn main() {
    eprintln!("enable --features archive");
}
