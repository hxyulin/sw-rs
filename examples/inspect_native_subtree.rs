// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Probe one exact subtree with an explicitly EMPTY archive context; never guess slots.
use sw_rs::{
    ArchiveContext, BinaryReader, DocumentReader, Limits, NativeExternalObjectProfile,
    NativeSubtreeReader,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path=args.next().ok_or("usage: inspect_native_subtree MODEL STREAM OFFSET two_strings|three_strings [array|external|surface]")?;
    let stream = args
        .next()
        .ok_or("missing decoded stream name")?
        .into_string()
        .map_err(|_| "stream name must be UTF-8")?;
    let offset = args
        .next()
        .ok_or("missing offset")?
        .to_str()
        .ok_or("invalid offset")?
        .parse::<usize>()?;
    let profile = match args.next().ok_or("missing profile")?.to_str() {
        Some("two_strings") => NativeExternalObjectProfile::TwoStrings,
        Some("three_strings") => NativeExternalObjectProfile::ThreeStrings,
        _ => return Err("unknown external-object profile".into()),
    };
    let kind = args.next().unwrap_or_else(|| "array".into());
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let bytes = std::fs::read(&path)?;
    let reader = DocumentReader::open(&bytes, &path.to_string_lossy(), Limits::default())?;
    let data = reader.stream(&stream)?.ok_or("stream absent")?;
    let mut cursor = BinaryReader::at(data, offset).ok_or("offset outside stream")?;
    let mut subtree = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    let result = match kind.to_str() {
        Some("array") => subtree.read_reference_array_prefix(&mut cursor, &stream),
        Some("external") => subtree.read_external_object(&mut cursor, &stream),
        Some("surface") => subtree.read_surface(&mut cursor, &stream),
        _ => return Err("unknown subtree kind".into()),
    };
    let (root, error) = match result {
        Ok(root) => (Some(root), None),
        Err(error) => (
            None,
            Some(serde_json::json!({"offset":error.offset,"message":error.message})),
        ),
    };
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "source_sha256":reader.source_sha256(),"source_stream":stream,"offset":offset,
            "context":"empty archive; no inherited class or object map was supplied",
            "profile":profile,"root":root,"error":error,"cursor_end":cursor.position(),
            "archive":subtree.archive().entries(),"objects":subtree.objects(),
            "native_object_graph_decoded":false,"topology_ownership_decoded":false,
        }),
    )?;
    Ok(())
}
