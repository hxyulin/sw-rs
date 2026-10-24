// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Locate unsupported explicit face-reference prefixes without constructing a map.
use serde_json::{Value, json};
use sw_rs::{BinaryReader, DocumentReader, Limits, NativeDocumentReference};

fn declaration(cursor: &mut BinaryReader<'_>) -> Option<String> {
    let mut trial = *cursor;
    if trial.u16()? != 0xffff || trial.u16()? != 1 {
        return None;
    }
    let length = usize::from(trial.u16()?);
    let bytes = trial.take(length)?;
    if bytes.is_empty()
        || !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return None;
    }
    let name = std::str::from_utf8(bytes).ok()?.to_owned();
    *cursor = trial;
    Some(name)
}

fn offsets(bytes: &[u8], name: &str) -> Vec<usize> {
    let mut marker = vec![255, 255, 1, 0];
    marker.extend_from_slice(&(name.len() as u16).to_le_bytes());
    marker.extend_from_slice(name.as_bytes());
    bytes
        .windows(marker.len())
        .enumerate()
        .filter_map(|(at, window)| (window == marker).then_some(at))
        .collect()
}

fn reference_declarations(bytes: &[u8]) -> Vec<Value> {
    bytes
        .windows(4)
        .enumerate()
        .filter_map(|(offset, tag)| {
            if tag != [255, 255, 1, 0] {
                return None;
            }
            let mut cursor = BinaryReader::at(bytes, offset)?;
            let name = declaration(&mut cursor)?;
            if !(name.ends_with("SurfIdRep_c")
                || name.ends_with("SurfIdRep2_c")
                || name.ends_with("EdgeIdRep_c")
                || matches!(
                    name.as_str(),
                    "moEdgeRef_c" | "moFaceRef_c" | "moFR_c" | "moExtObject_c"
                ))
            {
                return None;
            }
            Some(json!({"offset":offset,"end":cursor.position(),"class_name":name}))
        })
        .collect()
}

// This is a framing correlation, not a ReadObject walk or a recovered class slot.
fn handle_frames(bytes: &[u8]) -> Vec<Value> {
    offsets(bytes, "moCStringHandle_c")
        .into_iter()
        .filter_map(|offset| {
            let mut cursor = BinaryReader::at(bytes, offset)?;
            declaration(&mut cursor)?;
            let first = cursor.string()?;
            let tag_offset = cursor.position();
            let following_tag = cursor.u16()?;
            let second = cursor.string()?;
            Some(json!({"declaration_offset":offset, "first_string":first,
                "following_tag_offset":tag_offset, "following_tag":following_tag,
                "second_string":second}))
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or("usage: audit_mate_prefixes MODEL")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let bytes = std::fs::read(&path)?;
    let reader = DocumentReader::open(&bytes, &path.to_string_lossy(), Limits::default())?;
    let graph = reader.references()?;
    let mut configurations = Vec::new();
    for config in &reader.metadata()?.mates.configurations {
        let data = reader
            .stream(&config.source_stream)?
            .ok_or("missing mate stream")?;
        let mut candidates = Vec::new();
        let mut prefix_end = 0;
        for offset in offsets(data, "moFaceRef_c") {
            if offset < prefix_end {
                continue;
            }
            let explicit =
                NativeDocumentReference::parse_prefix(data, &config.source_stream, offset);
            if let Some(reference) = &explicit {
                prefix_end = reference.prefix_span.offset + reference.prefix_span.length;
            }
            let stop =
                NativeDocumentReference::inspect_prefix(data, &config.source_stream, offset).err();
            let candidate = graph.nodes.iter().find_map(|node| {
                if node.source_offset != Some(offset) || node.source.stream != config.source_stream
                {
                    return None;
                }
                match &node.record {
                    sw_rs::ReferenceRecord::NativeDocumentReferenceCandidate { reference } => {
                        Some(reference)
                    }
                    _ => None,
                }
            });
            if let Some(reference) = candidate {
                prefix_end = reference.prefix_span.offset + reference.prefix_span.length;
            }
            let lookahead = stop.as_ref().map(|stop| {
                let remaining = data.get(stop.offset..).unwrap_or_default();
                json!({"offset":stop.offset,"bytes":&remaining[..remaining.len().min(16)]})
            });
            candidates.push(
                json!({"offset":offset,"stop":stop,"explicit_prefix":explicit,
                "handle_candidate":candidate,"raw_stop_lookahead":lookahead}),
            );
        }
        if candidates.is_empty() {
            continue;
        }
        let base = config
            .source_stream
            .strip_suffix("-MatesList")
            .ok_or("invalid mate stream name")?
            .to_owned();
        let resolved_source = format!("{base}-ResolvedFeatures");
        let mut carriers = Vec::new();
        for name in [
            base.clone(),
            resolved_source.clone(),
            config.source_stream.clone(),
            format!("{base}-ModelHeader"),
            "Contents/CMgr".into(),
        ] {
            let Some(data) = reader.stream(&name)? else {
                continue;
            };
            carriers.push(json!({"stream":name,"handle_frames":handle_frames(data),
                "reference_class_declaration_candidates":reference_declarations(data),
                "mo_fr_declaration_candidates":offsets(data,"moFR_c"),
                "simple_surface_declaration_candidates":offsets(data,"moSimpleSurfIdRep_c")}));
        }
        let relevant = |source: &str| source == config.source_stream || source == resolved_source;
        let related = graph
            .nodes
            .iter()
            .filter(|node| {
                relevant(&node.source.stream)
                    && matches!(
                        &node.record,
                        sw_rs::ReferenceRecord::NativeDocumentReference { .. }
                            | sw_rs::ReferenceRecord::NativeDocumentReferenceCandidate { .. }
                    )
            })
            .collect::<Vec<_>>();
        let stops = graph
            .native_reference_stops
            .iter()
            .filter(|stop| relevant(&stop.source_stream))
            .collect::<Vec<_>>();
        configurations.push(
            json!({"stream":config.source_stream,"header_word":config.header_word,
            "candidates":candidates,"carrier_framing_evidence":carriers,
            "related_prefix_evidence":related,"native_reference_stops":stops}),
        );
    }
    serde_json::to_writer(
        std::io::stdout().lock(),
        &json!({
            "source_sha256":reader.source_sha256(),"configurations":configurations,
            "archive_map_recovered":false,"topology_ownership_decoded":false,
            "evidence_scope":"Scanned declarations and contiguous checked frames; tag equality is a correlation, not class or ownership resolution"
        }),
    )?;
    Ok(())
}
