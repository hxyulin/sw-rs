// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Native reference prefixes and explicitly provisional configuration framing evidence.
use crate::{BinaryReader, ByteSpan, FramedString};
use serde::Serialize;

mod context;
mod subtree;
pub use context::{NativeDocumentReferenceCandidate, NativeHandleClassEvidence};
pub use subtree::{
    NativeArchivePointer, NativeExternalObjectProfile, NativeSubtreeObject, NativeSubtreePayload,
    NativeSubtreeReader,
};

pub(crate) fn configuration_scope(source: &str) -> Option<(u32, &str)> {
    let base = source
        .strip_suffix("-MatesList")
        .or_else(|| source.strip_suffix("-ResolvedFeatures"))?;
    let digits = base.strip_prefix("Contents/Config-")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((digits.parse().ok()?, base))
}
fn is_face(class: &str) -> bool {
    class == "moFaceRef_c"
}

const COMPONENT_MARKER: [u8; 16] = [
    0x7d, 0xc3, 0x94, 0x25, 0xad, 0x49, 0xb2, 0x54, 0x7d, 0xc3, 0x94, 0x25, 0xad, 0x49, 0xb2, 0x54,
];

/// First unsupported position in an explicit native face-reference prefix.
/// No archive dictionary is inferred and no unknown payload is skipped.
#[derive(Clone, Debug, Serialize)]
pub struct NativeReferenceStop {
    /// Exact decoded source stream.
    pub source_stream: String,
    /// Start of the candidate face/edge reference declaration.
    pub prefix_offset: usize,
    /// First unsupported field's exact decoded-stream position.
    pub offset: usize,
    /// Expected class name or framing stage, not a decoded class assignment.
    pub stage: &'static str,
    /// Raw first word for a class-stage failure, if two bytes remain.
    pub raw_tag: Option<u16>,
    /// Explicit schema-1 class at that position, if its declaration is valid.
    pub explicit_class_candidate: Option<String>,
}

/// Saved document-handle evidence from a contiguous, explicit native reference prefix.
/// A path can be stale, external or blank. It is not proof of the current owner.
#[derive(Clone, Debug, Serialize)]
pub struct NativeDocumentReference {
    /// Exact decoded stream containing the prefix.
    pub source_stream: String,
    /// Explicit face/edge root class. Face is omitted in JSON for compatibility.
    #[serde(skip_serializing_if = "is_face")]
    pub reference_class: String,
    /// Complete checked prefix through the document CString, not an object boundary.
    pub prefix_span: ByteSpan,
    /// Native Config-N index for an exact mate-list or resolved-feature stream.
    /// Visual-state configuration/display-state ownership remains unknown.
    pub configuration_index: Option<u32>,
    /// Five saved header words, without interpreting counts, stamps or object IDs.
    pub raw_header: [u32; 5],
    /// Explicit saved surface/edge representation class, not a resolved map slot.
    pub entity_class: String,
    /// Exact saved document-handle text and its CString offset. Blank stays unknown.
    pub document: FramedString,
}

fn declaration(reader: &mut BinaryReader<'_>) -> Option<String> {
    let mut trial = *reader;
    if trial.u16()? != 0xffff || trial.u16()? != 1 {
        return None;
    }
    let length = usize::from(trial.u16()?);
    let name = trial.take(length)?;
    if name.is_empty()
        || !name
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return None;
    }
    let name = String::from_utf8(name.to_vec()).ok()?;
    *reader = trial;
    Some(name)
}

fn stop(
    bytes: &[u8],
    source_stream: &str,
    prefix_offset: usize,
    offset: usize,
    stage: &'static str,
    class: bool,
) -> NativeReferenceStop {
    let mut reader = BinaryReader::at(bytes, offset);
    NativeReferenceStop {
        source_stream: source_stream.into(),
        prefix_offset,
        offset,
        stage,
        raw_tag: class.then(|| reader.as_mut()?.u16()).flatten(),
        explicit_class_candidate: if class {
            BinaryReader::at(bytes, offset).and_then(|mut r| declaration(&mut r))
        } else {
            None
        },
    }
}

fn entity_prefix<'a>(
    bytes: &'a [u8],
    source: &str,
    offset: usize,
) -> Result<(BinaryReader<'a>, [u32; 5], String, String), NativeReferenceStop> {
    let failure = |at, stage, class| stop(bytes, source, offset, at, stage, class);
    let mut reader = BinaryReader::at(bytes, offset)
        .ok_or_else(|| failure(offset, "face_or_edge_reference_class", true))?;
    let reference_class = declaration(&mut reader)
        .filter(|name| matches!(name.as_str(), "moFaceRef_c" | "moEdgeRef_c"))
        .ok_or_else(|| failure(offset, "face_or_edge_reference_class", true))?;
    let header_offset = reader.position();
    let words = reader
        .words::<5>()
        .filter(|w| w[..2] == [1, 0])
        .ok_or_else(|| failure(header_offset, "reference_header", false))?;
    let marker_offset = reader.position();
    if reader.take(16) != Some(COMPONENT_MARKER.as_slice()) || reader.u16() != Some(0) {
        return Err(failure(marker_offset, "reference_marker", false));
    }
    let entity_offset = reader.position();
    let entity_class = declaration(&mut reader)
        .filter(|name| {
            name.starts_with("mo")
                && (name.ends_with("SurfIdRep_c")
                    || name.ends_with("SurfIdRep2_c")
                    || name.ends_with("EdgeIdRep_c"))
        })
        .ok_or_else(|| failure(entity_offset, "entity_class", true))?;
    let scalar_offset = reader.position();
    if reader.u16() != Some(0) {
        return Err(failure(scalar_offset, "entity_scalar", false));
    }
    for expected in ["moFR_c", "moExtObject_c"] {
        let at = reader.position();
        if declaration(&mut reader).as_deref() != Some(expected) {
            return Err(failure(at, expected, true));
        }
    }
    Ok((reader, words, entity_class, reference_class))
}

impl NativeDocumentReference {
    /// Read a supported moFaceRef_c/moEdgeRef_c prefix at an exact decoded-stream offset.
    /// Every class must be explicitly declared with schema 1. Unknown/reused classes
    /// and malformed prefixes return None; no slots or object lengths are guessed.
    pub fn parse_prefix(bytes: &[u8], source_stream: &str, offset: usize) -> Option<Self> {
        Self::inspect_prefix(bytes, source_stream, offset).ok()
    }

    /// Read an explicit prefix, retaining the first unsupported field on failure.
    /// This never resumes after an unknown class or assigns archive map slots.
    pub fn inspect_prefix(
        bytes: &[u8],
        source_stream: &str,
        offset: usize,
    ) -> Result<Self, NativeReferenceStop> {
        let (mut reader, raw_header, entity_class, reference_class) =
            entity_prefix(bytes, source_stream, offset)?;
        let at = reader.position();
        if declaration(&mut reader).as_deref() != Some("moCStringHandle_c") {
            return Err(stop(
                bytes,
                source_stream,
                offset,
                at,
                "moCStringHandle_c",
                true,
            ));
        }
        let at = reader.position();
        let document = reader
            .string()
            .ok_or_else(|| stop(bytes, source_stream, offset, at, "document_cstring", false))?;
        Ok(Self {
            source_stream: source_stream.into(),
            reference_class,
            prefix_span: ByteSpan {
                offset,
                length: reader.position() - offset,
            },
            configuration_index: configuration_scope(source_stream).map(|(index, _)| index),
            raw_header,
            entity_class,
            document,
        })
    }
}

impl NativeDocumentReferenceCandidate {
    pub(crate) fn parse(
        bytes: &[u8],
        source_stream: &str,
        offset: usize,
        evidence: &NativeHandleClassEvidence,
        protected_end: &mut usize,
    ) -> Option<Self> {
        let (configuration_index, base) = configuration_scope(source_stream)?;
        if evidence.source_stream != base {
            return None;
        }
        let (mut reader, raw_header, entity_class, reference_class) =
            entity_prefix(bytes, source_stream, offset).ok()?;
        let at = reader.position();
        let raw_tag = reader.u16()?;
        if raw_tag != evidence.raw_tag || !(0x8001..0xffff).contains(&raw_tag) {
            return None;
        }
        let mut counted = reader;
        counted.string_bytes()?;
        *protected_end = counted.position();
        let document = reader.string()?;
        Some(Self {
            source_stream: source_stream.into(),
            reference_class,
            prefix_span: ByteSpan {
                offset,
                length: reader.position() - offset,
            },
            configuration_index,
            raw_header,
            entity_class,
            document,
            unresolved_class: stop(bytes, source_stream, offset, at, "moCStringHandle_c", true),
            class_evidence: evidence.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
