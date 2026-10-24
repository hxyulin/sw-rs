// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Sequential configuration-header map audit and explicitly provisional mate seed clues.
#[cfg(feature = "archive")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .ok_or("usage: audit_archive_rules ARCHIVE.zip")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().to_owned();
        if ![".sldprt", ".sldasm", ".slddrw"]
            .iter()
            .any(|ext| name.to_ascii_lowercase().ends_with(ext))
            || name.rsplit('/').next().is_some_and(|n| n.starts_with("~$"))
        {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let reader = sw_rs::DocumentReader::open(&bytes, &name, sw_rs::Limits::unlimited())?;
        let result = audit(&reader);
        println!(
            "{}",
            match result {
                Ok(report) =>
                    serde_json::json!({"path":name,"source_sha256":reader.source_sha256(),"archive":report}),
                Err(error) =>
                    serde_json::json!({"path":name,"source_sha256":reader.source_sha256(),"error":error.to_string()}),
            }
        );
    }
    Ok(())
}
#[cfg(feature = "archive")]
fn audit(
    reader: &sw_rs::DocumentReader<'_>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use sw_rs::{ArchiveContext, ArchiveEntry, BinaryReader};
    fn text(reader: &mut BinaryReader<'_>) -> Option<String> {
        if let Some(text) = reader.string() {
            return Some(text.value);
        }
        let mut trial = *reader;
        let length = usize::from(trial.take(1)?[0]);
        if length == 255 {
            return None;
        }
        let text = std::str::from_utf8(trial.take(length)?).ok()?.to_owned();
        *reader = trial;
        Some(text)
    }
    fn object(
        map: &mut ArchiveContext,
        cursor: &mut BinaryReader<'_>,
        expected: &str,
    ) -> Result<sw_rs::ArchiveObjectHeader, Box<dyn std::error::Error>> {
        let header = map.read_object(cursor)?;
        if !header.is_new
            || !matches!(header.class_index.and_then(|i|map.entry(i)),Some(ArchiveEntry::Class {name,schema:1,..}) if name==expected)
        {
            return Err("unexpected class/schema or object backreference".into());
        }
        Ok(header)
    }
    let doc = reader.metadata()?;
    let stream = if reader.stream("Contents/CMgrHdr2")?.is_some() {
        "Contents/CMgrHdr2"
    } else {
        "Contents/CMgrHdr"
    };
    let bytes = reader
        .stream(stream)?
        .ok_or("configuration header stream absent")?;
    let mut cursor = BinaryReader::new(bytes);
    let mut map = ArchiveContext::new();
    let root = object(&mut map, &mut cursor, "dmConfigMgrHeader_c")?;
    let count = match cursor.u16().ok_or("truncated count")? {
        65535 => cursor.u32().ok_or("truncated extended count")?,
        n => u32::from(n),
    };
    if u64::from(count) > cursor.remaining().len() as u64 {
        return Err("count exceeds remaining prefix bytes".into());
    }
    let mut records = Vec::new();
    for _ in 0..count {
        let header = object(&mut map, &mut cursor, "dmConfigHeader_c")?;
        let flags = cursor.u32().ok_or("missing raw header word")?;
        let name = text(&mut cursor).ok_or("unsupported name string")?;
        let index = cursor.u32().ok_or("missing configuration index")?;
        if doc.version >= 1341 {
            cursor.u32().ok_or("missing versioned word")?;
        }
        if doc.version >= 2037 {
            text(&mut cursor).ok_or("missing display string")?;
            cursor.u32().ok_or("missing parent index")?;
        }
        if doc.version >= 2136 {
            cursor.u32().ok_or("missing versioned word")?;
        }
        if doc.version >= 2610 {
            text(&mut cursor).ok_or("missing string")?;
            text(&mut cursor).ok_or("missing string")?;
        }
        for gate in [3928, 11142, 12155, 12216] {
            if doc.version >= gate {
                cursor.u32().ok_or("missing versioned word")?;
            }
        }
        records.push(serde_json::json!({"header":header,"configuration_index":index,"name":name,"raw_header_word":flags,"prefix_end":cursor.position()}));
    }
    let hints=doc.mates.configurations.iter().filter_map(|configuration| {
        let first=configuration.records.first()?;
        let seed=configuration.header_word.filter(|&n|n>0 && n<0x7fff)?;
        if first.source_offset!=6 {return None;}
        let class=first.class_name.as_ref()?;
        let records=configuration.records.iter().enumerate().filter(|(_,r)|r.class_reference==Some(0x8000|seed as u16)).map(|(i,_)|i).collect::<Vec<_>>();
        Some(serde_json::json!({"stream":configuration.source_stream,"header_word":seed,"first_class":class,"matching_record_indices":records,"status":"hypothesis_requires_verification","meaning":"header word may be next map slot inherited from preceding archive context"}))
    }).collect::<Vec<_>>();
    Ok(
        serde_json::json!({"stream":stream,"version":doc.version,"stream_bytes":bytes.len(),"root":root,"records":records,"map":map.entries(),"known_prefix_end":cursor.position(),"opaque_tail":{"offset":cursor.position(),"length":cursor.remaining().len()},"mate_seed_hypotheses":hints,"complete_native_graph_decoded":false}),
    )
}
#[cfg(not(feature = "archive"))]
fn main() {
    eprintln!("enable --features archive");
}
