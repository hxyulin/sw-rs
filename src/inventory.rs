// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Source-aware inventory; structural candidates never imply semantic coverage.
use crate::{Container, FramedString, mesh};
use serde::Serialize;
use sha2::{Digest, Sha256};
mod xml;

/// A checked contiguous range, in bytes, in the source named by its enclosing record.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct ByteSpan {
    /// Zero-based starting byte offset.
    pub offset: usize,
    /// Number of bytes in this span.
    pub length: usize,
}
impl ByteSpan {
    /// Borrow this range without copying; reject overflow or out-of-bounds spans.
    pub fn slice(self, bytes: &[u8]) -> Option<&[u8]> {
        bytes.get(self.offset..self.offset.checked_add(self.length)?)
    }
    pub(crate) fn between(start: usize, end: usize) -> Self {
        Self {
            offset: start,
            length: end - start,
        }
    }
}
/// Recognized outer file signature, independent of runtime decoding support.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    /// Candidate modern inline chunk container; individual records still need validation.
    ModernInline,
    /// Legacy OLE2 structured storage; individual streams still require validation.
    LegacyOle2,
    /// Recognized ZIP/OPC signature; not decoded as a native document container.
    ZipOpc,
    /// No supported outer signature.
    Unknown,
}
/// Validation and selection outcome for one marker candidate in source-file bytes.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContainerRecordStatus {
    /// Validated modern payload/CRC or OLE stream; first selected decoded name.
    Selected,
    /// Validated duplicate name. Modern streams keep the first; OLE alias collisions fail.
    Duplicate,
    /// Rejected marker candidate or OLE stream; no verified byte ownership is implied.
    Rejected,
    /// Recognized directory-index framing; payload semantics remain unsupported.
    DirectoryIndex,
    /// Recognized cache-index framing; payload semantics remain unsupported.
    CacheIndex,
}
/// One modern marker candidate or OLE stream, including rejected and duplicate records.
#[derive(Clone, Debug, Serialize)]
pub struct ContainerRecord {
    /// Modern source-file marker offset; zero for OLE (use `ole_source.ranges`).
    pub offset: usize,
    /// Raw word immediately after the marker, when the inline-sized header exists.
    pub family_word: Option<u32>,
    /// Decoded stream name when its supported framing is available.
    pub name: Option<String>,
    /// Source-file span of the encoded name, when bounded.
    pub name_span: Option<ByteSpan>,
    /// Source-file span of a bounded candidate compressed payload; validation is separate.
    pub payload_span: Option<ByteSpan>,
    /// OLE stream provenance, including fragmented physical source ranges.
    /// Modern records omit this field; OLE records have no inline header spans.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ole_source: Option<crate::OleStreamSource>,
    /// Raw inline-header compressed-size field, not an established index-record size.
    pub declared_compressed_bytes: Option<usize>,
    /// Raw inline-header decoded-size field.
    pub declared_decoded_bytes: Option<usize>,
    /// Raw inline-header CRC word; only selected/duplicate records were verified.
    pub expected_crc32: Option<u32>,
    /// Validation/selection result, never inferred from a filename alone.
    pub status: ContainerRecordStatus,
    /// Actual produced byte count when decompression was attempted.
    pub decoded_bytes: Option<usize>,
    /// SHA-256 of fully validated decoded bytes, including validated duplicates.
    pub decoded_sha256: Option<String>,
    /// Local rejection/index/duplicate diagnostic, when applicable.
    pub diagnostic: Option<String>,
}
impl ContainerRecord {
    pub(crate) fn candidate(offset: usize) -> Self {
        Self {
            offset,
            family_word: None,
            name: None,
            name_span: None,
            payload_span: None,
            ole_source: None,
            declared_compressed_bytes: None,
            declared_decoded_bytes: None,
            expected_crc32: None,
            status: ContainerRecordStatus::Rejected,
            decoded_bytes: None,
            decoded_sha256: None,
            diagnostic: None,
        }
    }
}
/// A first-valid stream descriptor; decoder availability is not execution coverage.
#[derive(Clone, Debug, Serialize)]
pub struct StreamInventory {
    /// Exact decoded stream name.
    pub name: String,
    /// Index into the enclosing source-file record list.
    pub source_record_index: usize,
    /// Validated decoded byte count.
    pub decoded_bytes: usize,
    /// SHA-256 of the exact decoded byte buffer.
    pub sha256: String,
    /// Category suggested by the stream path, not an assertion of native semantics.
    pub family_hint: &'static str,
    /// Existing partial semantic decoders applicable by path; not execution results.
    pub available_decoders: Vec<&'static str>,
    /// Always `not_evaluated`: inventory does not claim semantic parsing coverage.
    pub semantic_status: &'static str,
}
/// Source-file inventory. Verified inline records and OLE stream bytes are accounted.
#[derive(Clone, Debug, Serialize)]
pub struct DocumentInventory {
    /// Versioned inventory interchange identity.
    pub schema: &'static str,
    /// Original source-file SHA-256.
    pub source_sha256: String,
    /// Original source-file byte length.
    pub source_bytes: usize,
    /// Outer signature classification, not inferred from the filename extension.
    pub format: SourceFormat,
    /// Fatal container error retained while unsupported/corrupt bytes remain inspectable.
    pub container_error: Option<String>,
    /// Whether selected decoded stream buffers are available through the reader.
    pub decoded_streams_available: bool,
    /// Modern marker candidates or OLE streams in directory traversal order.
    /// A hard error can stop the scan.
    pub records: Vec<ContainerRecord>,
    /// First-valid stream descriptors, sorted by exact name; buffers may be unavailable after a fatal error.
    pub streams: Vec<StreamInventory>,
    /// Nonoverlapping verified inline-record or OLE encoded-stream byte ranges.
    /// OLE allocation metadata and unused sector padding remain unclassified.
    pub verified_container_ranges: Vec<ByteSpan>,
    /// Complement of verified ranges: prologue, indices, rejected data and opaque gaps.
    /// These ranges are not assumed to be padding or absent records.
    pub unclassified_ranges: Vec<ByteSpan>,
    /// Number of bytes covered by verified container ranges; not semantic coverage.
    pub verified_container_bytes: usize,
}
/// An explicitly framed class-declaration candidate, not an instantiated object.
#[derive(Clone, Debug, Serialize)]
pub struct ClassDeclaration {
    /// Candidate declaration span within the decoded stream.
    pub span: ByteSpan,
    /// Supported declaration schema marker, currently 1.
    pub schema: u16,
    /// Exact ASCII native class name.
    pub name: String,
    /// Always `candidate`; framing may occur inside opaque data.
    pub status: &'static str,
}
/// A saved XML attribute; namespaces and duplicate local names remain distinct.
#[derive(Clone, Debug, Serialize)]
pub struct XmlAttribute {
    /// Local attribute name.
    pub name: String,
    /// Namespace URI when explicitly namespaced.
    pub namespace: Option<String>,
    /// Decoded XML value; never fetched or executed.
    pub value: String,
}
/// A structural XML element in preorder, independent of a native semantic decoder.
#[derive(Clone, Debug, Serialize)]
pub struct XmlElement {
    /// Zero-based element index in this stream's XML collection.
    pub index: usize,
    /// Parent element index; None for the XML document element.
    pub parent: Option<usize>,
    /// Local element name.
    pub name: String,
    /// Namespace URI when present.
    pub namespace: Option<String>,
    /// Complete element range in decoded-stream bytes, including nested content.
    pub span: ByteSpan,
    /// Exact parsed attributes with their namespaces.
    pub attributes: Vec<XmlAttribute>,
}
/// Parsed XML text, including mixed content, with an original decoded-byte span.
#[derive(Clone, Debug, Serialize)]
pub struct XmlText {
    /// Parent element index, or None for document-level text.
    pub parent: Option<usize>,
    /// Original XML byte range, before entity expansion/normalization.
    pub span: ByteSpan,
    /// XML-decoded text; source bytes remain accessible separately.
    pub value: String,
}
/// A syntactic SolidWorks XML reference candidate, not persistent topology ownership.
#[derive(Clone, Debug, Serialize)]
pub struct XmlReference {
    /// Referring element index.
    pub from_element: usize,
    /// Exact supported unqualified reference attribute name.
    pub attribute: String,
    /// Saved reference token.
    pub reference: String,
    /// Matching unqualified `id` element indices; empty/multiple stay unresolved.
    pub candidate_elements: Vec<usize>,
}
/// On-demand structural inspection of one decoded stream. This may be large.
#[derive(Clone, Debug, Serialize)]
pub struct StreamInspection {
    /// Exact enclosing decoded stream name; all child offsets use this stream.
    pub source_stream: String,
    /// `xml`, `invalid_xml`, `binary_candidates`, or `empty`; never semantic decoding.
    pub structure_status: &'static str,
    /// XML parse error, when XML-looking content is unsupported or malformed.
    pub xml_error: Option<String>,
    /// Source XML encoding when structural parsing succeeds.
    pub xml_encoding: Option<&'static str>,
    /// Original decoded-stream XML payload range, including its BOM when present.
    pub xml_payload_span: Option<ByteSpan>,
    /// Uninterpreted leading bytes of a supported native XML wrapper, when present.
    pub opaque_prefix: Option<ByteSpan>,
    /// Structurally parsed XML elements, retaining arbitrary names and attributes.
    pub xml_elements: Vec<XmlElement>,
    /// Parsed text nodes, including whitespace and mixed content.
    pub xml_text: Vec<XmlText>,
    /// Supported literal XML ID-reference candidates; no cross-stream resolution.
    pub xml_references: Vec<XmlReference>,
    /// Class-framing candidates, without guessed object boundaries or reused tags.
    pub class_declarations: Vec<ClassDeclaration>,
    /// Framed UTF-16 string candidates, with no semantic or ownership inference.
    pub framed_strings: Vec<FramedString>,
    /// False: a complete native record/object graph was not decoded by this inspection.
    pub native_object_graph_decoded: bool,
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub(crate) fn format(bytes: &[u8]) -> SourceFormat {
    if bytes.starts_with(&[0xd0, 0xcf, 0x11, 0xe0]) {
        SourceFormat::LegacyOle2
    } else if bytes.starts_with(b"PK\x03\x04") {
        SourceFormat::ZipOpc
    } else if bytes
        .get(..bytes.len().min(64))
        .is_some_and(|b| b.windows(6).any(|w| w == [20, 0, 6, 0, 8, 0]))
    {
        SourceFormat::ModernInline
    } else {
        SourceFormat::Unknown
    }
}
fn family(name: &str) -> (&'static str, Vec<&'static str>) {
    let leaf = name.rsplit('/').next().unwrap_or(name);
    if name.starts_with("ImportedComp/") || name.starts_with("VirtualComp/") {
        ("embedded_document", vec![])
    } else if leaf == "DisplayLists" {
        (
            "display_cache",
            vec!["geometry.cached_part", "appearance.cached_properties"],
        )
    } else if leaf == "VisualStates" {
        ("appearance", vec!["appearance.native_partial"])
    } else if leaf.ends_with("-MatesList") {
        ("mates", vec!["mates.inventory_partial"])
    } else if leaf == "MATERIALTREE" {
        ("materials", vec!["material.saved_definitions"])
    } else if leaf == "COMPINSTANCETREE" {
        ("occurrences", vec!["metadata.components_partial"])
    } else if leaf == "CMgrHdr" || leaf == "CMgrHdr2" {
        (
            "configuration",
            vec!["metadata.configuration_names_partial"],
        )
    } else if leaf.ends_with("Partition") || leaf == "LocalBodies" || leaf == "StockBodies" {
        ("body_geometry", vec![])
    } else if leaf.ends_with("ResolvedFeatures")
        || leaf == "History"
        || leaf == "Biography"
        || leaf == "Features"
    {
        ("feature_history", vec![])
    } else if leaf == "User Units Table" {
        ("units", vec![])
    } else if leaf == "View Orientation Data" {
        ("saved_views", vec![])
    } else if leaf.ends_with("PreviewPNG") || leaf == "PreviewPNG" {
        ("preview", vec!["metadata.png_bytes"])
    } else if name.starts_with("docProps/") {
        let available = match leaf {
            "core.xml" => vec!["metadata.core_properties"],
            "custom.xml" | "ISolidWorksInformation.xml" => vec!["metadata.properties_partial"],
            _ if leaf.starts_with("Config-") && leaf.ends_with("-Cutlist-Properties.xml") => {
                vec!["metadata.cut_list_partial"]
            }
            _ if leaf.starts_with("Config-") && leaf.ends_with("-Properties.xml") => {
                vec!["metadata.properties_partial"]
            }
            _ => vec![],
        };
        ("document_properties", available)
    } else {
        ("unknown", vec![])
    }
}
pub(crate) fn inventory(
    bytes: &[u8],
    source_hash: &str,
    records: &[ContainerRecord],
    result: &Result<Container, crate::ParseError>,
) -> DocumentInventory {
    let mut verified = records
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                ContainerRecordStatus::Selected | ContainerRecordStatus::Duplicate
            )
        })
        .flat_map(|r| {
            if let Some(source) = &r.ole_source {
                source.ranges.clone()
            } else {
                r.payload_span
                    .map(|p| ByteSpan::between(r.offset, p.offset + p.length))
                    .into_iter()
                    .collect()
            }
        })
        .collect::<Vec<_>>();
    verified.sort_by_key(|s| s.offset);
    let mut gaps = Vec::new();
    let mut end = 0;
    for span in &verified {
        if end < span.offset {
            gaps.push(ByteSpan::between(end, span.offset));
        }
        end = end.max(span.offset + span.length);
    }
    if end < bytes.len() {
        gaps.push(ByteSpan::between(end, bytes.len()));
    }
    let streams = records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.status == ContainerRecordStatus::Selected)
        .map(|(index, r)| {
            let name = r.name.clone().expect("verified selected name");
            let (family_hint, available_decoders) = family(&name);
            StreamInventory {
                name,
                source_record_index: index,
                decoded_bytes: r.decoded_bytes.expect("verified size"),
                sha256: r.decoded_sha256.clone().expect("traced hash"),
                family_hint,
                available_decoders,
                semantic_status: "not_evaluated",
            }
        })
        .collect::<Vec<_>>();
    let mut out = DocumentInventory {
        schema: "sw-rs-inventory-v1",
        source_sha256: source_hash.into(),
        source_bytes: bytes.len(),
        format: format(bytes),
        container_error: result.as_ref().err().map(ToString::to_string),
        decoded_streams_available: result.is_ok(),
        records: records.to_vec(),
        streams,
        verified_container_bytes: verified.iter().map(|s| s.length).sum(),
        verified_container_ranges: verified,
        unclassified_ranges: gaps,
    };
    out.streams.sort_by(|a, b| a.name.cmp(&b.name));
    out
}
pub(crate) fn inspect(name: &str, bytes: &[u8]) -> StreamInspection {
    let mut out = StreamInspection {
        source_stream: name.into(),
        structure_status: "binary_candidates",
        xml_error: None,
        xml_encoding: None,
        xml_payload_span: None,
        opaque_prefix: None,
        xml_elements: Vec::new(),
        xml_text: Vec::new(),
        xml_references: Vec::new(),
        class_declarations: Vec::new(),
        framed_strings: Vec::new(),
        native_object_graph_decoded: false,
    };
    if bytes.is_empty() {
        out.structure_status = "empty";
        return out;
    }
    if xml::inspect(name, bytes, &mut out) {
        return out;
    }
    out.class_declarations = mesh::class_intervals(bytes)
        .into_iter()
        .map(|c| ClassDeclaration {
            span: ByteSpan::between(c.class_offset, c.content.start),
            schema: 1,
            name: c.name,
            status: "candidate",
        })
        .collect();
    let mut at = 0;
    while at < bytes.len() {
        if let Some((value, end)) = crate::visual::native_string_at(bytes, at) {
            out.framed_strings.push(value);
            at = end;
        } else {
            at += 1;
        }
    }
    out
}
