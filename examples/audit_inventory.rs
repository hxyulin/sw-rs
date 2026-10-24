// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Reproducible source/stream census for private native-document ZIP archives.
#[cfg(feature = "archive")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{collections::BTreeMap, io::Read};
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: audit_inventory ARCHIVE [--structure]")?;
    let structure = match args.next().as_deref() {
        None => false,
        Some("--structure") => true,
        _ => return Err("unknown argument".into()),
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
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
        let inventory = reader.inventory();
        let mut inspections = Vec::new();
        if structure && inventory.decoded_streams_available {
            for stream in &inventory.streams {
                let inspection = reader
                    .inspect_stream(&stream.name)?
                    .expect("selected stream");
                let mut classes = BTreeMap::<String, usize>::new();
                for class in inspection.class_declarations {
                    *classes.entry(class.name).or_default() += 1;
                }
                inspections.push(serde_json::json!({"stream":stream.name,"status":inspection.structure_status,"xml_elements":inspection.xml_elements.len(),"xml_references":inspection.xml_references.len(),"xml_error":inspection.xml_error,"classes":classes,"framed_strings":inspection.framed_strings.len()}));
            }
        }
        println!(
            "{}",
            serde_json::json!({"path":name,"source_sha256":inventory.source_sha256,"source_bytes":inventory.source_bytes,"format":inventory.format,"container_error":inventory.container_error,"records":inventory.records.len(),"record_statuses":inventory.records.iter().fold(BTreeMap::<String,usize>::new(),|mut counts,record|{*counts.entry(format!("{:?}",record.status)).or_default()+=1;counts}),"verified_container_bytes":inventory.verified_container_bytes,"unclassified_bytes":inventory.unclassified_ranges.iter().map(|s|s.length).sum::<usize>(),"streams":inventory.streams,"inspections":inspections})
        );
    }
    Ok(())
}
#[cfg(not(feature = "archive"))]
fn main() {
    eprintln!("enable --features archive");
}
