// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Versioned native metadata and cached Part geometry inspection CLI.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let first = args
        .next()
        .ok_or("usage: sw-dump [metadata|geometry|geometry-status|mates|inventory|references|model-headers|assembly-analysis|stream] MODEL [STREAM] | --version")?;
    if first == "--version" {
        println!("sw-rs {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let geometry = first == "geometry";
    let geometry_status = first == "geometry-status";
    let mates = first == "mates";
    let inventory = first == "inventory";
    let references = first == "references";
    let model_headers = first == "model-headers";
    let stream = first == "stream";
    let assembly_analysis = first == "assembly-analysis";
    let path = if geometry
        || geometry_status
        || mates
        || inventory
        || references
        || model_headers
        || stream
        || assembly_analysis
        || first == "metadata"
    {
        args.next().ok_or("missing native document path")?
    } else {
        first
    };
    let inspect_stream = if inventory { args.next() } else { None };
    let raw_stream = if stream {
        Some(args.next().ok_or("missing decoded stream name")?)
    } else {
        None
    };
    let include_geometry = if references {
        match args.next() {
            Some(flag) if flag == "--geometry" => true,
            Some(_) => return Err("expected --geometry".into()),
            None => false,
        }
    } else {
        false
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let file = std::fs::File::open(&path)?;
    use std::io::Read;
    let mut data = Vec::new();
    file.take((sw_rs::Limits::default().max_file_bytes as u64) + 1)
        .read_to_end(&mut data)?;
    if geometry_status {
        let reader =
            sw_rs::DocumentReader::open(&data, &path.to_string_lossy(), sw_rs::Limits::default())?;
        serde_json::to_writer(std::io::stdout().lock(), &reader.inspect_geometry_cache()?)?;
        return Ok(());
    }
    if let Some(name) = raw_stream {
        let reader =
            sw_rs::DocumentReader::open(&data, &path.to_string_lossy(), sw_rs::Limits::default())?;
        let bytes = reader
            .stream(&name.to_string_lossy())?
            .ok_or("decoded stream not found")?;
        use std::io::Write;
        std::io::stdout().lock().write_all(bytes)?;
        return Ok(());
    }
    if model_headers {
        let reader =
            sw_rs::DocumentReader::open(&data, &path.to_string_lossy(), sw_rs::Limits::default())?;
        serde_json::to_writer(std::io::stdout().lock(), reader.model_headers()?)?;
        return Ok(());
    }
    let result = if assembly_analysis {
        let reader =
            sw_rs::DocumentReader::open(&data, &path.to_string_lossy(), sw_rs::Limits::default())?;
        let doc = reader.metadata()?;
        let configurations: Vec<_> = doc
            .configurations
            .iter()
            .map(|config| {
                let mates = doc
                    .mates
                    .configurations
                    .iter()
                    .find(|m| m.configuration_index == config.index);
                serde_json::json!({
                    "configuration_index": config.index,
                    "name": config.name,
                    "derived_from_index": config.parent_index,
                    "components": config.components,
                    "mates": mates,
                    "mate_reference_graph": mates.map(sw_rs::ConfigurationMates::reference_graph),
                })
            })
            .collect();
        serde_json::json!({
            "schema": "sw-rs-assembly-analysis-v1",
            "source_sha256": reader.source_sha256(),
            "doc_type": doc.document_type,
            "version": doc.version,
            "coordinate_frame": doc.coordinate_frame,
            "configurations": configurations,
            "reference_replacements": doc.reference_replacements,
            "references": reader.references()?,
            "coverage": {
                "scope": "direct_saved_occurrences",
                "transforms": "saved_column_major_local_to_owner",
                "grounding_decoded": false,
                "limit_bounds_decoded": false,
                "mate_entity_ownership_decoded": false,
                "constraint_solving": false,
            },
            "diagnostics": doc.diagnostics,
        })
    } else if references {
        let reader =
            sw_rs::DocumentReader::open(&data, &path.to_string_lossy(), sw_rs::Limits::default())?;
        if include_geometry {
            serde_json::to_value(reader.references_with_geometry()?)?
        } else {
            serde_json::to_value(reader.references()?)?
        }
    } else if inventory {
        let reader =
            sw_rs::DocumentReader::open(&data, &path.to_string_lossy(), sw_rs::Limits::default())?;
        let inspection = inspect_stream
            .as_ref()
            .map(|name| reader.inspect_stream(&name.to_string_lossy()))
            .transpose()?;
        serde_json::json!({"inventory":reader.inventory(),"inspection":inspection.flatten()})
    } else if geometry {
        match sw_rs::geometry(&data, &path.to_string_lossy()) {
            Ok(doc) => {
                serde_json::json!({"schema":"sw-rs-geometry-v1","geometry":doc,"diagnostics":[]})
            }
            Err(error) => {
                serde_json::to_writer(
                    std::io::stdout().lock(),
                    &serde_json::json!({"schema":"sw-rs-geometry-v1","geometry":null,"diagnostics":[{"code":"unsupported_or_invalid_geometry","message":error.to_string()}]}),
                )?;
                return Err(error.into());
            }
        }
    } else {
        let doc = sw_rs::parse(&data, &path.to_string_lossy())?;
        if mates {
            serde_json::json!({"schema":"sw-rs-mates-v1","mates":doc.mates})
        } else {
            serde_json::to_value(doc)?
        }
    };
    serde_json::to_writer(std::io::stdout().lock(), &result)?;
    Ok(())
}
