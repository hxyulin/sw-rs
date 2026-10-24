// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Versioned native metadata and cached Part geometry inspection CLI.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let first = args
        .next()
        .ok_or("usage: sw-dump [metadata|geometry|mates|inventory|references] MODEL | --version")?;
    if first == "--version" {
        println!("sw-rs {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let geometry = first == "geometry";
    let mates = first == "mates";
    let inventory = first == "inventory";
    let references = first == "references";
    let path = if geometry || mates || inventory || references || first == "metadata" {
        args.next().ok_or("missing native document path")?
    } else {
        first
    };
    let inspect_stream = if inventory { args.next() } else { None };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let file = std::fs::File::open(&path)?;
    use std::io::Read;
    let mut data = Vec::new();
    file.take((sw_rs::Limits::default().max_file_bytes as u64) + 1)
        .read_to_end(&mut data)?;
    let result = if references {
        let reader =
            sw_rs::DocumentReader::open(&data, &path.to_string_lossy(), sw_rs::Limits::default())?;
        serde_json::to_value(reader.references()?)?
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
