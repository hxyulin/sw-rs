// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Strict CFB stream loading and exact legacy ZLB normalization.
use crate::{ByteSpan, Container, ContainerRecord, ContainerRecordStatus, Limits, ParseError};
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::{self, Cursor, Read, Seek, SeekFrom},
    path::Component,
    rc::Rc,
};

/// Encoding of one original OLE stream. All ZLB offsets are stream-relative.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OleCompression {
    /// Uncompressed bytes copied exactly, without an invented native checksum.
    Raw,
    /// Every member and the EOF terminator of the supported ZLB wrapper validated.
    Zlb {
        /// Exact member ranges in the extracted OLE stream and decoded output.
        chunks: Vec<crate::ZlbChunk>,
        /// Final eight zero bytes in the extracted OLE stream.
        terminator: ByteSpan,
    },
}

/// Source provenance for one decoded OLE stream, without conflating sector offsets
/// with logical stream offsets. Concatenating `ranges` reconstructs encoded bytes.
#[derive(Clone, Debug, Serialize)]
pub struct OleStreamSource {
    /// Exact original CFB path, without a leading slash, before suffix normalization.
    pub path: String,
    /// Original extracted stream length, before any ZLB decoding.
    pub encoded_bytes: usize,
    /// SHA-256 of original extracted bytes, not an on-disk integrity checksum.
    pub encoded_sha256: String,
    /// Physical source-file byte ranges in logical stream order; excludes padding.
    pub ranges: Vec<ByteSpan>,
    /// Supported encoding after validation; None for a rejected ZLB wrapper.
    pub compression: Option<OleCompression>,
}

fn error(message: impl std::fmt::Display) -> ParseError {
    ParseError(format!("legacy OLE2: {message}"))
}

// Observe only physical reads made while extracting a stream. CFB keeps its
// allocation tables in memory. Verify the observation against the extracted bytes
// before publishing it, so library buffering cannot silently corrupt provenance.
struct SourceReader<'a> {
    cursor: Cursor<&'a [u8]>,
    ranges: Rc<RefCell<Vec<ByteSpan>>>,
    // One allocation-metadata pass plus one nonoverlapping stream-data pass.
    // Bound malformed DIFAT amplification before the CFB reader allocates it.
    remaining: usize,
}
impl Read for SourceReader<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if !out.is_empty() && self.remaining == 0 {
            return Err(io::Error::other("OLE physical read budget exceeded"));
        }
        let offset = self.cursor.position() as usize;
        let room = out.len().min(self.remaining);
        let length = self.cursor.read(&mut out[..room])?;
        self.remaining -= length;
        if length != 0 {
            let mut ranges = self.ranges.borrow_mut();
            if let Some(last) = ranges.last_mut()
                && last.offset + last.length == offset
            {
                last.length += length;
            } else {
                ranges.push(ByteSpan { offset, length });
            }
        }
        Ok(length)
    }
}
impl Seek for SourceReader<'_> {
    fn seek(&mut self, at: SeekFrom) -> io::Result<u64> {
        self.cursor.seek(at)
    }
}

pub(crate) fn parse(
    data: &[u8],
    limits: Limits,
    mut trace: Option<&mut Vec<ContainerRecord>>,
) -> Result<Container, ParseError> {
    let ranges = Rc::new(RefCell::new(Vec::new()));
    let source = SourceReader {
        cursor: Cursor::new(data),
        ranges: Rc::clone(&ranges),
        remaining: data.len().saturating_mul(2),
    };
    let mut file = cfb::OpenOptions::new()
        .strict()
        .max_buffer_size(8192)
        .open_with(source)
        .map_err(error)?;
    // Count storages too: a deeply nested empty tree is still parsing work.
    let mut entries = Vec::new();
    let mut records = 0usize;
    for entry in file.walk().filter(|entry| !entry.is_root()) {
        if records >= limits.max_records {
            return Err(error("entry count limit exceeded"));
        }
        records += 1;
        if entry.is_stream() {
            entries.push(entry);
        }
    }
    let mut container = Container {
        streams: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    let mut total = 0usize;
    let mut occupied = BTreeMap::<usize, usize>::new();
    // Keep allocation metadata distinct from stream-owned bytes. Its reads can
    // overlap each other; their union cannot be claimed by a native stream.
    let mut metadata_ranges = std::mem::take(&mut *ranges.borrow_mut());
    metadata_ranges.sort_by_key(|span| span.offset);
    for span in metadata_ranges {
        if let Some(mut last) = occupied.last_entry()
            && *last.get() >= span.offset
        {
            *last.get_mut() = (*last.get()).max(span.offset + span.length);
        } else {
            occupied.insert(span.offset, span.offset + span.length);
        }
    }
    for entry in entries {
        let path = entry
            .path()
            .components()
            .filter_map(|part| match part {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/");
        let zlb = path.ends_with("__ZLB");
        let name = path.strip_suffix("__ZLB").unwrap_or(&path).to_owned();
        if name.is_empty() || name.ends_with('/') {
            return Err(error("empty normalized stream name"));
        }
        let length = usize::try_from(entry.len()).map_err(error)?;
        let budget = limits.max_total_bytes.saturating_sub(total);
        if length > data.len()
            || (zlb && length > limits.max_compressed_bytes)
            || (!zlb && (length > limits.max_stream_bytes || length > budget))
        {
            return Err(error(format!(
                "encoded or decoded byte limit exceeded ({path})"
            )));
        }
        ranges.borrow_mut().clear();
        let mut encoded = Vec::new();
        file.open_stream(entry.path())
            .map_err(error)?
            .take(entry.len().saturating_add(1))
            .read_to_end(&mut encoded)
            .map_err(error)?;
        if encoded.len() != length {
            return Err(error(format!("truncated stream ({path})")));
        }
        let physical = std::mem::take(&mut *ranges.borrow_mut());
        let mut logical = 0usize;
        for span in &physical {
            let end = logical
                .checked_add(span.length)
                .ok_or_else(|| error("range overflow"))?;
            if span.slice(data) != encoded.get(logical..end) {
                return Err(error("physical stream provenance mismatch"));
            }
            logical = end;
            let end = span.offset + span.length;
            if occupied
                .range(..end)
                .next_back()
                .is_some_and(|(_, prior_end)| *prior_end > span.offset)
            {
                return Err(error("overlapping OLE stream ranges"));
            }
            occupied.insert(span.offset, end);
        }
        if logical != length {
            return Err(error("incomplete physical stream provenance"));
        }
        let mut record = ContainerRecord::candidate(0);
        record.name = Some(name.clone());
        record.ole_source = Some(OleStreamSource {
            path: path.clone(),
            encoded_bytes: length,
            encoded_sha256: crate::inventory::hash(&encoded),
            ranges: physical,
            compression: (!zlb).then_some(OleCompression::Raw),
        });
        let decoded = if zlb {
            let decoded = crate::decode_zlb(
                &encoded,
                Limits {
                    max_records: limits.max_records.saturating_sub(records),
                    max_total_bytes: budget,
                    ..limits
                },
            );
            let decoded = match decoded {
                Ok(value) => value,
                Err(failure) => {
                    record.diagnostic = Some(failure.to_string());
                    if let Some(trace) = trace.as_deref_mut() {
                        trace.push(record);
                    }
                    return Err(error(format!("{path}: {failure}")));
                }
            };
            records += decoded.chunks.len();
            record
                .ole_source
                .as_mut()
                .expect("OLE provenance")
                .compression = Some(OleCompression::Zlb {
                chunks: decoded.chunks,
                terminator: decoded.terminator,
            });
            decoded.data
        } else {
            encoded
        };
        total = total
            .checked_add(decoded.len())
            .ok_or_else(|| error("total byte overflow"))?;
        record.decoded_bytes = Some(decoded.len());
        record.decoded_sha256 = Some(crate::inventory::hash(&decoded));
        let collision = container.streams.contains_key(&name);
        record.status = if collision {
            ContainerRecordStatus::Duplicate
        } else {
            ContainerRecordStatus::Selected
        };
        if collision {
            record.diagnostic = Some(format!("normalized stream name collision: {name}"));
        }
        if let Some(trace) = trace.as_deref_mut() {
            trace.push(record);
        }
        if collision {
            return Err(error(format!("normalized stream name collision: {name}")));
        }
        container.streams.insert(name, decoded);
    }
    if container.streams.is_empty() {
        return Err(error("no streams found"));
    }
    Ok(container)
}
