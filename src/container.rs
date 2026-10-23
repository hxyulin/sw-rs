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
    /// Maximum marker candidates, bounding diagnostics and decode attempts.
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
#[derive(Debug, Serialize)]
pub struct Diagnostic {
    /// Byte offset of the marker (or zero for document-level diagnostics).
    pub offset: usize,
    /// Human-readable reason; no malformed record is silently accepted.
    pub message: String,
}
/// Decoded primary streams, retaining the first valid occurrence of each name.
#[derive(Debug)]
pub struct Container {
    /// Raw uncompressed stream bytes, keyed by decoded ASCII names.
    pub streams: BTreeMap<String, Vec<u8>>,
    /// Skipped records, index records, and duplicate names.
    pub diagnostics: Vec<Diagnostic>,
}
fn u32_at(data: &[u8], pos: usize) -> usize {
    u32::from_le_bytes(data[pos..pos + 4].try_into().expect("checked header")) as usize
}
impl Container {
    /// Decode inline raw-deflate records with CRC and size verification.
    /// OLE2 and ZIP/OPC are recognized but unsupported.
    pub fn parse(data: &[u8], limits: Limits) -> Result<Self, ParseError> {
        let fail = |s: &str| ParseError(s.into());
        if data.len() > limits.max_file_bytes {
            return Err(fail("input size limit exceeded"));
        }
        if data.starts_with(&[0xd0, 0xcf, 0x11, 0xe0]) {
            return Err(fail("legacy OLE2 container is unsupported"));
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
            let mut warn = |message: &str| {
                result.diagnostics.push(Diagnostic {
                    offset: at,
                    message: message.into(),
                })
            };
            if data.len() - at < 26 {
                warn("truncated header");
                continue;
            }
            let crc = u32_at(data, at + 10) as u32;
            let compressed = u32_at(data, at + 14);
            let expected = u32_at(data, at + 18);
            let name_len = u32_at(data, at + 22);
            // Tail-directory names follow a 14-byte descriptor, rather than
            // the inline header. Recognize them before trying to inflate bytes.
            if crc == 0
                && expected == 0
                && name_len != 0
                && let Some(record) = data
                    .get(at..)
                    .and_then(|tail| tail.get(..name_len.checked_add(46)?))
                && record[40..40 + name_len]
                    .iter()
                    .all(|b| (0x20..0x7f).contains(&b.rotate_left(u32::from(data[7]) & 7)))
            {
                warn("directory index record (no inline payload)");
                continue;
            }
            let name_start = at + 26;
            // Subtraction checks avoid overflow on wasm32.
            if name_len == 0 || name_len > data.len() - name_start {
                warn("invalid or truncated stream name");
                continue;
            }
            let decoded: Vec<u8> = data[name_start..name_start + name_len]
                .iter()
                .map(|b| b.rotate_left(u32::from(data[7]) & 7))
                .collect();
            let Ok(name) = String::from_utf8(decoded) else {
                warn("invalid UTF-8 stream name or index record");
                continue;
            };
            if name.chars().any(char::is_control) {
                warn("control character in stream name or index record");
                continue;
            }
            // Non-inline record families must not be treated as deflate data.
            if expected != 0
                && crc == (expected as u32).wrapping_mul(2)
                && compressed == expected / 2
            {
                warn("cache index record (no inline payload)");
                continue;
            }
            let start = name_start + name_len;
            if compressed > limits.max_compressed_bytes || compressed > data.len() - start {
                warn("invalid or truncated compressed payload");
                continue;
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
            if output.len() > budget {
                return Err(ParseError(format!(
                    "decoded byte limit exceeded at {at} ({name}): total={total}, stream={}, budget={budget}",
                    output.len()
                )));
            }
            total += output.len();
            if !complete {
                warn("invalid or incomplete deflate stream");
                continue;
            }
            if decoder.total_in() != compressed as u64 || output.len() != expected {
                warn("compressed or decoded size mismatch");
                continue;
            }
            if crc32fast::hash(&output) != crc {
                warn("CRC mismatch");
                continue;
            }
            use std::collections::btree_map::Entry;
            match result.streams.entry(name) {
                Entry::Vacant(e) => {
                    e.insert(output);
                }
                Entry::Occupied(e) => result.diagnostics.push(Diagnostic {
                    offset: at,
                    message: format!("duplicate stream: {} (first wins)", e.key()),
                }),
            }
            pos = start + compressed;
        }
        if result.streams.is_empty() {
            return Err(fail("no valid inline streams found"));
        }
        Ok(result)
    }
}
