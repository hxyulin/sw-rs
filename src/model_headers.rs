// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Whole-stream model-header selection; no configuration map seeding or ownership joins.
use crate::{
    ArchiveContext, ArchiveEntry, BinaryReader, Container, NativeArchivePointer,
    NativeExternalObjectProfile, NativeModelHeaderProfile, NativeModelHeaderTailProfile,
    NativeSubtreeObject, NativeSubtreePayload, NativeSubtreeReader,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// One supported explicit combination of header, external-object and tail framing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct NativeModelHeaderLayout {
    /// Initial model-header framing.
    pub header_profile: NativeModelHeaderProfile,
    /// Saved external-object framing used by this archive.
    pub external_profile: NativeExternalObjectProfile,
    /// Scalar extension after the common header tail.
    pub tail_profile: NativeModelHeaderTailProfile,
}
const LAYOUTS: [NativeModelHeaderLayout; 6] = [
    NativeModelHeaderLayout {
        header_profile: NativeModelHeaderProfile::DirectArrays,
        external_profile: NativeExternalObjectProfile::TwoStrings,
        tail_profile: NativeModelHeaderTailProfile::NoExtension,
    },
    NativeModelHeaderLayout {
        header_profile: NativeModelHeaderProfile::LeadingWord,
        external_profile: NativeExternalObjectProfile::ThreeStringsShortTail,
        tail_profile: NativeModelHeaderTailProfile::Extension32,
    },
    NativeModelHeaderLayout {
        header_profile: NativeModelHeaderProfile::LeadingWord,
        external_profile: NativeExternalObjectProfile::ThreeStringsShortTail,
        tail_profile: NativeModelHeaderTailProfile::Extension64,
    },
    NativeModelHeaderLayout {
        header_profile: NativeModelHeaderProfile::LeadingWord,
        external_profile: NativeExternalObjectProfile::ThreeStrings,
        tail_profile: NativeModelHeaderTailProfile::Extension68,
    },
    NativeModelHeaderLayout {
        header_profile: NativeModelHeaderProfile::DirectArrays,
        external_profile: NativeExternalObjectProfile::ThreeStringsShortBlock,
        tail_profile: NativeModelHeaderTailProfile::Extension28,
    },
    NativeModelHeaderLayout {
        header_profile: NativeModelHeaderProfile::DirectArrays,
        external_profile: NativeExternalObjectProfile::ThreeStringsShortTail,
        tail_profile: NativeModelHeaderTailProfile::Extension32,
    },
];

/// Result of one bounded, fresh-context layout attempt.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeModelHeaderAttemptOutcome {
    /// A non-null complete model-header definition consumes the entire stream.
    Complete,
    /// A layout field or root definition is unsupported or malformed.
    Rejected {
        /// Exact failed offset in the decoded stream.
        offset: usize,
        /// Concrete framing, type or resource-limit failure.
        message: String,
    },
    /// A complete object was read, but bytes remain in the carrier.
    TrailingBytes {
        /// End of the checked object in decoded-stream bytes.
        offset: usize,
        /// Number of unread bytes; no resynchronization is attempted.
        length: usize,
    },
}
/// Provenance and outcome of one candidate layout, including rejected alternatives.
#[derive(Debug, Serialize)]
pub struct NativeModelHeaderAttempt {
    /// Explicit layout attempted from byte zero with an empty archive.
    pub layout: NativeModelHeaderLayout,
    /// Exact whole-stream result.
    pub outcome: NativeModelHeaderAttemptOutcome,
}
/// Selection status for one exact saved model-header stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeModelHeaderStatus {
    /// Exactly one supported layout consumes the complete stream.
    Decoded,
    /// No supported layout consumes a complete non-null header and the entire stream.
    Unsupported,
    /// Multiple layouts consume the complete stream; no layout or graph is chosen.
    Ambiguous,
}
/// A uniquely selected complete header and its archive-local definitions.
#[derive(Debug, Serialize)]
pub struct NativeModelHeaderDecoding {
    /// Unique supported layout established by whole-stream framing.
    pub layout: NativeModelHeaderLayout,
    /// Saved root pointer occurrence; its object definition is complete.
    pub root: NativeArchivePointer,
    /// Archive-local class/object slots in registration order, starting with slot one.
    pub archive: Vec<ArchiveEntry>,
    /// Typed definitions stored once by slot, with original source spans and fields.
    pub objects: BTreeMap<u32, NativeSubtreeObject>,
}
/// Decode status and alternatives for one literal Config-N-ModelHeader carrier.
#[derive(Debug, Serialize)]
pub struct NativeModelHeaderStream {
    /// Index from the stream name when it fits u32; no current configuration is assigned.
    pub configuration_index: Option<u32>,
    /// Exact source name, preserving zero padding and configuration scope.
    pub source_stream: String,
    /// Total bytes in the verified decoded carrier.
    pub decoded_bytes: usize,
    /// Unique, unsupported or ambiguous whole-stream framing.
    pub status: NativeModelHeaderStatus,
    /// Available only when exactly one complete layout consumes the stream.
    pub decoding: Option<NativeModelHeaderDecoding>,
    /// Results for every supported layout, including partial or failed alternatives.
    pub attempts: Vec<NativeModelHeaderAttempt>,
}
/// Opt-in complete model-header report, independent of metadata and geometry decoding.
#[derive(Debug, Serialize)]
pub struct NativeModelHeaderReport {
    /// Versioned JSON contract for this inspection.
    pub schema: &'static str,
    /// SHA-256 of original source bytes, before decompression.
    pub source_sha256: String,
    /// Exact matching streams in source-name order; absence means unavailable evidence.
    pub streams: Vec<NativeModelHeaderStream>,
    /// Always false: complete local headers do not establish a whole-file native graph.
    pub native_object_graph_decoded: bool,
    /// Always false: no face, component, mate or appearance ownership is assigned.
    pub topology_ownership_decoded: bool,
}
impl NativeModelHeaderReport {
    pub(crate) fn inspect(container: &Container, source_hash: &str) -> Self {
        let streams = container
            .streams
            .iter()
            .filter_map(|(name, bytes)| {
                let identifier = name
                    .strip_prefix("Contents/Config-")?
                    .strip_suffix("-ModelHeader")?;
                if identifier.is_empty() || !identifier.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                Some(inspect_stream(
                    bytes,
                    name,
                    identifier.parse().ok(),
                    &LAYOUTS,
                ))
            })
            .collect();
        Self {
            schema: "sw-rs-model-headers-v1",
            source_sha256: source_hash.into(),
            streams,
            native_object_graph_decoded: false,
            topology_ownership_decoded: false,
        }
    }
}
fn inspect_stream(
    bytes: &[u8],
    name: &str,
    index: Option<u32>,
    layouts: &[NativeModelHeaderLayout],
) -> NativeModelHeaderStream {
    let mut attempts = Vec::new();
    let mut decoding = None;
    let mut complete = 0;
    for &layout in layouts {
        let mut cursor = BinaryReader::new(bytes);
        let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), layout.external_profile);
        let result = reader.read_model_header_with_tail(
            &mut cursor,
            name,
            layout.header_profile,
            layout.tail_profile,
        );
        let outcome = match result {
            Err(error) => NativeModelHeaderAttemptOutcome::Rejected {
                offset: error.offset,
                message: error.message.into(),
            },
            Ok(root) => {
                let valid_root = root
                    .header
                    .object_index()
                    .and_then(|slot| reader.objects().get(&slot))
                    .is_some_and(|object| {
                        object.complete_payload
                            && matches!(object.payload, NativeSubtreePayload::ModelHeader { .. })
                    });
                if !valid_root {
                    NativeModelHeaderAttemptOutcome::Rejected {
                        offset: 0,
                        message: "expected a complete non-null model-header definition".into(),
                    }
                } else if !cursor.remaining().is_empty() {
                    NativeModelHeaderAttemptOutcome::TrailingBytes {
                        offset: cursor.position(),
                        length: cursor.remaining().len(),
                    }
                } else {
                    complete += 1;
                    if complete == 1 {
                        let (archive, objects) = reader.into_definitions();
                        decoding = Some(NativeModelHeaderDecoding {
                            layout,
                            root,
                            archive,
                            objects,
                        });
                    } else {
                        // Drop the first graph too: ambiguity must not expose a winner.
                        decoding = None;
                    }
                    NativeModelHeaderAttemptOutcome::Complete
                }
            }
        };
        attempts.push(NativeModelHeaderAttempt { layout, outcome });
    }
    let status = match complete {
        0 => NativeModelHeaderStatus::Unsupported,
        1 => NativeModelHeaderStatus::Decoded,
        _ => NativeModelHeaderStatus::Ambiguous,
    };
    NativeModelHeaderStream {
        configuration_index: index,
        source_stream: name.into(),
        decoded_bytes: bytes.len(),
        status,
        decoding,
        attempts,
    }
}

#[cfg(test)]
mod tests;
