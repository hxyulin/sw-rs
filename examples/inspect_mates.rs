// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Reproducible mate inventory and reference-evidence graph for private ZIP archives.
//! `mbx run --release --features archive --example inspect_mates -- ARCHIVE.zip`
#[cfg(feature = "archive")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or("usage: inspect_mates ARCHIVE.zip")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name()?.into_owned();
        if !name.to_ascii_lowercase().ends_with(".sldasm")
            || name.rsplit('/').next().is_some_and(|n| n.starts_with("~$"))
        {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let hash = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let value = match sw_rs::parse_with_limits(&bytes, &name, sw_rs::Limits::unlimited()) {
            Ok(doc) => serde_json::json!({
                "path":name,"sha256":hash,"mates":doc.mates,
                "reference_graphs":doc.mates.configurations.iter().map(|c| serde_json::json!({
                    "source_stream":c.source_stream,"configuration_index":c.configuration_index,
                    "nodes":c.reference_graph(),
                })).collect::<Vec<_>>(),
            }),
            Err(error) => serde_json::json!({"path":name,"sha256":hash,"error":error.to_string()}),
        };
        println!("{value}");
    }
    Ok(())
}
#[cfg(not(feature = "archive"))]
fn main() {
    eprintln!("enable --features archive");
}
