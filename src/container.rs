// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Ported from openswx internal/modern_parser.cc and rol_codec.h.
// Copyright (c) 2026 openswx contributors; see LICENSE-openswx.
use serde::Serialize;
use std::{collections::BTreeMap, fmt};
const MARKER: &[u8] = &[0x14, 0, 6, 0, 8, 0];

/// Resource limits applied before and during decompression.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum input document bytes.
    pub max_file_bytes: usize,
    /// Maximum compressed bytes in one stream.
    pub max_compressed_bytes: usize,
    /// Maximum decoded bytes in one stream.
    pub max_stream_bytes: usize,
    /// Maximum cumulative decoded bytes, including duplicate streams.
    pub max_total_bytes: usize,
    /// Maximum modern marker candidates, or OLE entries plus ZLB members.
    pub max_records: usize,
}
impl Limits {
    /// Remove byte and record-count policy caps for caller-owned local parsing.
    /// Payload lengths, checksums and numeric validity are still verified.
    pub const fn unlimited() -> Self {
        Self {
            max_file_bytes: usize::MAX,
            max_compressed_bytes: usize::MAX,
            max_stream_bytes: usize::MAX,
            max_total_bytes: usize::MAX,
            max_records: usize::MAX,
        }
    }
    /// Remove byte policy caps for local imports while retaining the malformed
    /// record scan bound. Actual allocations remain limited by the platform.
    pub fn without_byte_limits() -> Self {
        Self {
            max_file_bytes: usize::MAX,
            max_compressed_bytes: usize::MAX,
            max_stream_bytes: usize::MAX,
            max_total_bytes: usize::MAX,
            ..Self::default()
        }
    }
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_bytes: 256 << 20,
            max_compressed_bytes: 64 << 20,
            max_stream_bytes: 128 << 20,
            max_total_bytes: 512 << 20,
            max_records: 100_000,
        }
    }
}
/// A hard container error. Individual corrupt records instead produce diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub(crate) String);
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ParseError {}
/// Evidence for an unselected or malformed record.
#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    /// Byte offset of the marker (or zero for document-level diagnostics).
    pub offset: usize,
    /// Human-readable reason; no malformed record is silently accepted.
    pub message: String,
}
/// Decoded primary streams, retaining the first valid occurrence of each name.
#[derive(Debug)]
pub struct Container {
    /// Uncompressed stream bytes, keyed by decoded Unicode paths.
    pub streams: BTreeMap<String, Vec<u8>>,
    /// Skipped records, index records, and duplicate names.
    pub diagnostics: Vec<Diagnostic>,
}
fn u32_at(data: &[u8], pos: usize) -> usize {
    u32::from_le_bytes(data[pos..pos + 4].try_into().expect("checked header")) as usize
}
impl Container {
    /// Structurally inspect one retained stream without decoding the container again.
    /// Returns owned XML records or binary candidates; raw bytes remain unchanged.
    pub fn inspect_stream(&self, name: &str) -> Option<crate::StreamInspection> {
        self.streams
            .get(name)
            .map(|bytes| crate::inventory::inspect(name, bytes))
    }

    /// Decode modern inline records or legacy OLE streams with checked ZLB wrappers.
    /// ZIP/OPC native documents remain unsupported.
    pub fn parse(data: &[u8], limits: Limits) -> Result<Self, ParseError> {
        Self::parse_traced(data, limits, None)
    }
    pub(crate) fn parse_traced(
        data: &[u8],
        limits: Limits,
        mut trace: Option<&mut Vec<crate::inventory::ContainerRecord>>,
    ) -> Result<Self, ParseError> {
        use crate::inventory::{ByteSpan, ContainerRecord, ContainerRecordStatus};
        let fail = |s: &str| ParseError(s.into());
        if data.len() > limits.max_file_bytes {
            return Err(fail("input size limit exceeded"));
        }
        if data.starts_with(&[0xd0, 0xcf, 0x11, 0xe0]) {
            return crate::ole::parse(data, limits, trace);
        }
        if data.starts_with(b"PK\x03\x04") {
            return Err(fail("ZIP/OPC container is unsupported"));
        }
        if data.len() < 22 || !data[..data.len().min(64)].windows(6).any(|w| w == MARKER) {
            return Err(fail("unrecognized SolidWorks container"));
        }
        let mut result = Self {
            streams: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        let mut pos = 0;
        let mut records = 0;
        let mut total = 0usize;
        while let Some(relative) = data[pos..].windows(6).position(|w| w == MARKER) {
            let at = pos + relative;
            pos = at + 6;
            records += 1;
            if records > limits.max_records {
                return Err(fail("record count limit exceeded"));
            }
            let mut record = trace.as_deref_mut().map(|records| {
                records.push(ContainerRecord::candidate(at));
                records.last_mut().expect("inserted candidate")
            });
            if data.len() - at < 26 {
                trace_warning(&mut result, &mut record, at, "truncated header");
                continue;
            }
            let crc = u32_at(data, at + 10) as u32;
            let compressed = u32_at(data, at + 14);
            let expected = u32_at(data, at + 18);
            let name_len = u32_at(data, at + 22);
            if let Some(record) = record.as_deref_mut() {
                record.family_word = Some(u32_at(data, at + 6) as u32);
                record.expected_crc32 = Some(crc);
                record.declared_compressed_bytes = Some(compressed);
                record.declared_decoded_bytes = Some(expected);
            }
            // Tail-directory names follow a 14-byte descriptor, rather than
            // the inline header. Recognize them before trying to inflate bytes.
            if crc == 0
                && expected == 0
                && name_len != 0
                && let Some(index_record) = data
                    .get(at..)
                    .and_then(|tail| tail.get(..name_len.checked_add(46)?))
                && index_record[40..40 + name_len]
                    .iter()
                    .all(|b| (0x20..0x7f).contains(&b.rotate_left(u32::from(data[7]) & 7)))
            {
                if let Some(record) = record.as_deref_mut() {
                    record.status = ContainerRecordStatus::DirectoryIndex;
                    record.name_span = Some(ByteSpan {
                        offset: at + 40,
                        length: name_len,
                    });
                    record.name = String::from_utf8(
                        data[at + 40..at + 40 + name_len]
                            .iter()
                            .map(|b| b.rotate_left(u32::from(data[7]) & 7))
                            .collect(),
                    )
                    .ok();
                }
                trace_warning(
                    &mut result,
                    &mut record,
                    at,
                    "directory index record (no inline payload)",
                );
                continue;
            }
            let name_start = at + 26;
            // Subtraction checks avoid overflow on wasm32.
            if name_len == 0 || name_len > data.len() - name_start {
                trace_warning(
                    &mut result,
                    &mut record,
                    at,
                    "invalid or truncated stream name",
                );
                continue;
            }
            if let Some(record) = record.as_deref_mut() {
                record.name_span = Some(ByteSpan {
                    offset: name_start,
                    length: name_len,
                });
            }
            let decoded: Vec<u8> = data[name_start..name_start + name_len]
                .iter()
                .map(|b| b.rotate_left(u32::from(data[7]) & 7))
                .collect();
            let Ok(name) = String::from_utf8(decoded) else {
                trace_warning(
                    &mut result,
                    &mut record,
                    at,
                    "invalid UTF-8 stream name or index record",
                );
                continue;
            };
            if name.chars().any(char::is_control) {
                trace_warning(
                    &mut result,
                    &mut record,
                    at,
                    "control character in stream name or index record",
                );
                continue;
            }
            if let Some(record) = record.as_deref_mut() {
                record.name = Some(name.clone());
            }
            // Non-inline record families must not be treated as deflate data.
            if expected != 0
                && crc == (expected as u32).wrapping_mul(2)
                && compressed == expected / 2
            {
                if let Some(record) = record.as_deref_mut() {
                    record.status = ContainerRecordStatus::CacheIndex;
                }
                trace_warning(
                    &mut result,
                    &mut record,
                    at,
                    "cache index record (no inline payload)",
                );
                continue;
            }
            let start = name_start + name_len;
            if compressed > limits.max_compressed_bytes || compressed > data.len() - start {
                trace_warning(
                    &mut result,
                    &mut record,
                    at,
                    "invalid or truncated compressed payload",
                );
                continue;
            }
            if let Some(record) = record.as_deref_mut() {
                record.payload_span = Some(ByteSpan {
                    offset: start,
                    length: compressed,
                });
            }
            if expected > limits.max_stream_bytes {
                return Err(fail("stream size limit exceeded"));
            }
            let budget = limits
                .max_stream_bytes
                .min(limits.max_total_bytes.saturating_sub(total));
            let mut output = Vec::new();
            let mut decoder = flate2::Decompress::new(false);
            let mut complete = false;
            loop {
                let mut chunk = [0u8; 8192];
                let room = chunk
                    .len()
                    .min(budget.saturating_sub(output.len()).saturating_add(1));
                let before_in = decoder.total_in();
                let before_out = decoder.total_out();
                let status = decoder.decompress(
                    &data[start + before_in as usize..start + compressed],
                    &mut chunk[..room],
                    flate2::FlushDecompress::None,
                );
                let produced = (decoder.total_out() - before_out) as usize;
                output.extend_from_slice(&chunk[..produced]);
                if output.len() > budget {
                    break;
                }
                match status {
                    Ok(flate2::Status::StreamEnd) => {
                        complete = true;
                        break;
                    }
                    Err(_) => break,
                    _ if before_in == decoder.total_in() && produced == 0 => break,
                    _ => {}
                }
            }
            if let Some(record) = record.as_deref_mut() {
                record.decoded_bytes = Some(output.len());
            }
            if output.len() > budget {
                return Err(ParseError(format!(
                    "decoded byte limit exceeded at {at} ({name}): total={total}, stream={}, budget={budget}",
                    output.len()
                )));
            }
            total += output.len();
            if !complete {
                trace_warning(
                    &mut result,
                    &mut record,
                    at,
                    "invalid or incomplete deflate stream",
                );
                continue;
            }
            if decoder.total_in() != compressed as u64 || output.len() != expected {
                trace_warning(
                    &mut result,
                    &mut record,
                    at,
                    "compressed or decoded size mismatch",
                );
                continue;
            }
            if crc32fast::hash(&output) != crc {
                trace_warning(&mut result, &mut record, at, "CRC mismatch");
                continue;
            }
            if let Some(record) = record.as_deref_mut() {
                record.decoded_sha256 = Some(crate::inventory::hash(&output));
                record.status = if result.streams.contains_key(&name) {
                    ContainerRecordStatus::Duplicate
                } else {
                    ContainerRecordStatus::Selected
                };
            }
            use std::collections::btree_map::Entry;
            match result.streams.entry(name) {
                Entry::Vacant(e) => {
                    e.insert(output);
                }
                Entry::Occupied(e) => {
                    let message = format!("duplicate stream: {} (first wins)", e.key());
                    if let Some(record) = record {
                        record.diagnostic = Some(message.clone());
                    }
                    result.diagnostics.push(Diagnostic {
                        offset: at,
                        message,
                    });
                }
            }
            pos = start + compressed;
        }
        if result.streams.is_empty() {
            return Err(fail("no valid inline streams found"));
        }
        Ok(result)
    }
}

fn trace_warning(
    container: &mut Container,
    record: &mut Option<&mut crate::inventory::ContainerRecord>,
    offset: usize,
    message: &str,
) {
    if let Some(record) = record.as_deref_mut() {
        record.diagnostic = Some(message.into());
    }
    container.diagnostics.push(Diagnostic {
        offset,
        message: message.into(),
    });
}
