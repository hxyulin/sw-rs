// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Probe one exact subtree with an explicitly EMPTY archive context; never guess slots.
use sw_rs::{
    ArchiveContext, BinaryReader, DocumentReader, Limits, NativeEdgeReferenceProfile,
    NativeExternalObjectProfile, NativeFaceReferenceProfile, NativeModelHeaderProfile,
    NativeModelHeaderTailProfile, NativeSubtreeReader,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path=args.next().ok_or("usage: inspect_native_subtree MODEL STREAM OFFSET two_strings|three_strings|three_strings_short_tail|three_strings_short_block [array|model_header|model_header_word|model_header_full|model_header_full_word32|model_header_full_word64|model_header_full_word68|model_header_full_direct28|model_header_full_direct32|external|surface|face_three_words|face_five_words|edge_two_words|edge_four_words] [COUNT]")?;
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
        Some("three_strings_short_tail") => NativeExternalObjectProfile::ThreeStringsShortTail,
        Some("three_strings_short_block") => NativeExternalObjectProfile::ThreeStringsShortBlock,
        _ => return Err("unknown external-object profile".into()),
    };
    let kind = args.next().unwrap_or_else(|| "array".into());
    let count = args
        .next()
        .map(|s| {
            s.to_str()
                .ok_or("invalid count")?
                .parse::<usize>()
                .map_err(|_| "invalid count")
        })
        .transpose()?
        .unwrap_or(1);
    if !(1..=10_000).contains(&count) {
        return Err("count must be between 1 and 10000".into());
    }
    if count > 1
        && !matches!(
            kind.to_str(),
            Some(
                "face_three_words"
                    | "face_five_words"
                    | "edge_two_words"
                    | "edge_four_words"
                    | "model_header_full"
                    | "model_header_full_word32"
                    | "model_header_full_word64"
                    | "model_header_full_word68"
                    | "model_header_full_direct28"
                    | "model_header_full_direct32"
            )
        )
    {
        return Err("multiple reads require a complete model-header or reference profile".into());
    }
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let bytes = std::fs::read(&path)?;
    let reader = DocumentReader::open(&bytes, &path.to_string_lossy(), Limits::default())?;
    let data = reader.stream(&stream)?.ok_or("stream absent")?;
    let mut cursor = BinaryReader::at(data, offset).ok_or("offset outside stream")?;
    let mut subtree = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    let mut roots = Vec::new();
    let mut error = None;
    for _ in 0..count {
        let result = match kind.to_str() {
            Some("array") => subtree.read_reference_array_prefix(&mut cursor, &stream),
            Some("model_header_full_direct28") => subtree.read_model_header_with_tail(
                &mut cursor,
                &stream,
                NativeModelHeaderProfile::DirectArrays,
                NativeModelHeaderTailProfile::Extension28,
            ),
            Some("model_header_full_direct32") => subtree.read_model_header_with_tail(
                &mut cursor,
                &stream,
                NativeModelHeaderProfile::DirectArrays,
                NativeModelHeaderTailProfile::Extension32,
            ),
            Some("model_header_full_word32") => subtree.read_model_header_with_tail(
                &mut cursor,
                &stream,
                NativeModelHeaderProfile::LeadingWord,
                NativeModelHeaderTailProfile::Extension32,
            ),
            Some("model_header_full_word64") => subtree.read_model_header_with_tail(
                &mut cursor,
                &stream,
                NativeModelHeaderProfile::LeadingWord,
                NativeModelHeaderTailProfile::Extension64,
            ),
            Some("model_header_full_word68") => subtree.read_model_header_with_tail(
                &mut cursor,
                &stream,
                NativeModelHeaderProfile::LeadingWord,
                NativeModelHeaderTailProfile::Extension68,
            ),
            Some("model_header_full") => subtree.read_model_header(
                &mut cursor,
                &stream,
                NativeModelHeaderProfile::DirectArrays,
            ),
            Some("model_header") => subtree.read_model_header_prefix(
                &mut cursor,
                &stream,
                NativeModelHeaderProfile::DirectArrays,
            ),
            Some("model_header_word") => subtree.read_model_header_prefix(
                &mut cursor,
                &stream,
                NativeModelHeaderProfile::LeadingWord,
            ),
            Some("external") => subtree.read_external_object(&mut cursor, &stream),
            Some("surface") => subtree.read_surface(&mut cursor, &stream),
            Some("face_three_words") => subtree.read_face_reference(
                &mut cursor,
                &stream,
                NativeFaceReferenceProfile::ThreeWords,
            ),
            Some("face_five_words") => subtree.read_face_reference(
                &mut cursor,
                &stream,
                NativeFaceReferenceProfile::FiveWords,
            ),
            Some("edge_two_words") => subtree.read_edge_reference(
                &mut cursor,
                &stream,
                NativeEdgeReferenceProfile::TwoWords,
            ),
            Some("edge_four_words") => subtree.read_edge_reference(
                &mut cursor,
                &stream,
                NativeEdgeReferenceProfile::FourWords,
            ),
            _ => return Err("unknown subtree kind".into()),
        };
        match result {
            Ok(root) => roots.push(root),
            Err(failure) => {
                error =
                    Some(serde_json::json!({"offset":failure.offset,"message":failure.message}));
                break;
            }
        }
    }
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "source_sha256":reader.source_sha256(),"source_stream":stream,"offset":offset,
            "context":"empty archive; no inherited class or object map was supplied",
            "profile":profile,"root":roots.first(),"roots":roots,"error":error,"cursor_end":cursor.position(),
            "archive":subtree.archive().entries(),"objects":subtree.objects(),
            "native_object_graph_decoded":false,"topology_ownership_decoded":false,
        }),
    )?;
    Ok(())
}
