// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Reproducible saved-identity census; writes JSONL, never extracts private CAD.
#[cfg(feature = "archive")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{collections::BTreeMap, io::Read};
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .ok_or("usage: audit_references ARCHIVE.zip [--geometry]")?;
    let include_geometry = match args.next().as_deref() {
        None => false,
        Some(flag) if flag == "--geometry" => true,
        _ => return Err("expected --geometry".into()),
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name()?.into_owned();
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
            Ok(base) => {
                let geometry = if include_geometry && lower.ends_with(".sldprt") {
                    Some(reader.geometry())
                } else {
                    None
                };
                let graph = match &geometry {
                    Some(Ok(_)) => std::borrow::Cow::Owned(
                        base.clone().with_surfaces(reader.surface_references()?)?,
                    ),
                    _ => std::borrow::Cow::Borrowed(base),
                };
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
                let mut cache_status = BTreeMap::<String, usize>::new();
                let mut reference_status = BTreeMap::<String, usize>::new();
                let mut classes = BTreeMap::<String, usize>::new();
                let mut reference_counts = BTreeMap::<usize, usize>::new();
                let mut tail_arities = BTreeMap::<usize, usize>::new();
                let mut unterminated = 0;
                let mut pair_tails =
                    BTreeMap::<(u32, u32), std::collections::BTreeSet<Vec<u32>>>::new();
                for node in &graph.nodes {
                    if let sw_rs::ReferenceRecord::CachedSurface {
                        surface_references, ..
                    } = &node.record
                    {
                        *reference_counts
                            .entry(surface_references.references.len())
                            .or_default() += 1;
                        let resolution =
                            serde_json::to_value(surface_references.identity_resolution())?;
                        *cache_status
                            .entry(resolution["status"].as_str().unwrap().into())
                            .or_default() += 1;
                        for reference in &surface_references.references {
                            unterminated += usize::from(!reference.terminal_comma);
                            *reference_status
                                .entry(
                                    serde_json::to_value(reference.status)?
                                        .as_str()
                                        .unwrap()
                                        .into(),
                                )
                                .or_default() += 1;
                            *classes.entry(reference.class_name.clone()).or_default() += 1;
                            if let Some(identity) = &reference.identity {
                                *tail_arities
                                    .entry(identity.trailing_fields.len())
                                    .or_default() += 1;
                                pair_tails
                                    .entry((identity.feature_source_id, identity.local_surface_id))
                                    .or_default()
                                    .insert(identity.trailing_fields.clone());
                            }
                        }
                    }
                }
                let mut mate_surface_status = BTreeMap::<String, usize>::new();
                for link in &graph.links {
                    for reference in link
                        .selection_surface_reference
                        .iter()
                        .chain(&link.selection_surface_references)
                    {
                        *mate_surface_status
                            .entry(
                                serde_json::to_value(reference.status)?
                                    .as_str()
                                    .unwrap()
                                    .into(),
                            )
                            .or_default() += 1;
                    }
                }
                let geometry=geometry.map(|result|match result {Ok(g)=>serde_json::json!({"tables":g.model.tessellations.len(),"triangles":g.model.tessellations.iter().map(|t|t.triangles.len()).sum::<usize>(),"vertices":g.model.tessellations.iter().map(|t|t.vertices.len()).sum::<usize>()}),Err(e)=>serde_json::json!({"error":e.to_string()})});
                serde_json::json!({"path":name,"source_sha256":reader.source_sha256(),"features_status":graph.features.status,"features":graph.features.records.len(),"feature_source_ids":ids.len(),"duplicate_feature_ids":ids.values().filter(|&&n|n>1).count(),"nodes":graph.nodes.len(),"links":counts,"native_object_graph_decoded":graph.native_object_graph_decoded,"topology_ownership_decoded":graph.topology_ownership_decoded,"geometry":geometry,"cache_surface_resolution":cache_status,"cache_surface_reference_status":reference_status,"cache_surface_reference_classes":classes,"same_pair_distinct_tail_groups":pair_tails.values().filter(|v|v.len()>1).count(),"mate_surface_reference_status":mate_surface_status,"cache_surface_reference_counts":reference_counts,"cache_surface_tail_arities":tail_arities,"cache_surface_unterminated":unterminated})
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
