// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Research-verified compressed-stream framing, also used by the OLE loader.
use crate::{BinaryReader, ByteSpan, Limits, ParseError};
use flate2::{Decompress, FlushDecompress, Status};
use serde::Serialize;

const SIGNATURE: [u8; 16] = [
    0x23, 0x1d, 0xd5, 0x71, 0xda, 0x81, 0x48, 0xa2, 0xa8, 0x58, 0x98, 0xb2, 0x1b, 0x89, 0xef, 0x99,
];

/// One exact zlib member inside the supported legacy stream wrapper.
#[derive(Clone, Debug, Serialize)]
pub struct ZlbChunk {
    /// Two little-endian size words in the encoded extracted stream.
    pub header: ByteSpan,
    /// Exact compressed member bytes, including its zlib header/checksum.
    pub compressed: ByteSpan,
    /// This member's range in the concatenated decoded result.
    pub decoded: ByteSpan,
}
/// Fully validated supported ZLB wrapper; does not imply native document parsing.
#[derive(Debug, Serialize)]
pub struct ZlbStream {
    /// Concatenated member payloads; not serialized as JSON byte arrays.
    #[serde(skip_serializing)]
    pub data: Vec<u8>,
    /// Source/member provenance in encounter order.
    pub chunks: Vec<ZlbChunk>,
    /// Final eight zero bytes in the encoded extracted stream.
    pub terminator: ByteSpan,
}

/// Decode the observed legacy `__ZLB` stream wrapper without opening OLE files.
/// Validates the 16-byte signature, size pairs, every complete zlib member and its
/// checksum, exact decoded lengths, and the final zero/zero terminator with no tail.
/// Source spans refer to the extracted stream, not fragmented OLE file sectors.
///
/// Existing limits bound encoded input, cumulative compressed/decoded bytes and
/// chunk count. No one-MiB policy limit is imposed. Decoding grows only with actual
/// output, never by preallocating an untrusted declared length. Any failed member
/// fails the entire operation. Callers processing multiple streams own the combined
/// resource budget; `max_total_bytes` here applies to this result only.
///
/// ```
/// # let mut bytes = vec![0x23,0x1d,0xd5,0x71,0xda,0x81,0x48,0xa2,0xa8,0x58,0x98,0xb2,0x1b,0x89,0xef,0x99];
/// # bytes.extend([0; 8]);
/// let empty = sw_rs::decode_zlb(&bytes, sw_rs::Limits::default())?;
/// assert!(empty.data.is_empty());
/// assert!(empty.chunks.is_empty());
/// # Ok::<(), sw_rs::ParseError>(())
/// ```
pub fn decode_zlb(bytes: &[u8], limits: Limits) -> Result<ZlbStream, ParseError> {
    let error = |message: &str| ParseError(format!("legacy ZLB: {message}"));
    if bytes.len() > limits.max_file_bytes {
        return Err(error("input size limit exceeded"));
    }
    let mut reader = BinaryReader::new(bytes);
    if reader.take(16) != Some(SIGNATURE.as_slice()) {
        return Err(error("unsupported or truncated signature"));
    }
    let mut data = Vec::new();
    let mut chunks = Vec::new();
    let mut compressed_total = 0_usize;
    loop {
        let header_offset = reader.position();
        let [decoded_len, compressed_len] = reader
            .words::<2>()
            .ok_or_else(|| error("truncated size pair or missing terminator"))?;
        if decoded_len == 0 && compressed_len == 0 {
            if !reader.remaining().is_empty() {
                return Err(error("unexpected bytes after terminator"));
            }
            return Ok(ZlbStream {
                data,
                chunks,
                terminator: ByteSpan {
                    offset: header_offset,
                    length: 8,
                },
            });
        }
        if decoded_len == 0 || compressed_len == 0 {
            return Err(error("incomplete zero size pair"));
        }
        if chunks.len() >= limits.max_records {
            return Err(error("chunk count limit exceeded"));
        }
        let decoded_len = usize::try_from(decoded_len)
            .map_err(|_| error("decoded size exceeds platform range"))?;
        let compressed_len = usize::try_from(compressed_len)
            .map_err(|_| error("compressed size exceeds platform range"))?;
        let final_len = data
            .len()
            .checked_add(decoded_len)
            .ok_or_else(|| error("decoded size overflow"))?;
        if final_len > limits.max_stream_bytes || final_len > limits.max_total_bytes {
            return Err(error("decoded byte limit exceeded"));
        }
        compressed_total = compressed_total
            .checked_add(compressed_len)
            .ok_or_else(|| error("compressed size overflow"))?;
        if compressed_total > limits.max_compressed_bytes {
            return Err(error("compressed byte limit exceeded"));
        }
        let compressed_offset = reader.position();
        let packed = reader
            .take(compressed_len)
            .ok_or_else(|| error("truncated compressed member"))?;
        let decoded_offset = data.len();
        let mut decoder = Decompress::new(true);
        let mut buffer = [0_u8; 8192];
        loop {
            let before_in = decoder.total_in();
            let before_out = decoder.total_out();
            let input =
                usize::try_from(before_in).map_err(|_| error("member input offset overflow"))?;
            let status = decoder
                .decompress(&packed[input..], &mut buffer, FlushDecompress::None)
                .map_err(|_| error("invalid zlib member or checksum"))?;
            let produced = usize::try_from(decoder.total_out() - before_out)
                .map_err(|_| error("member output size overflow"))?;
            if decoder.total_out() > decoded_len as u64 {
                return Err(error("member decoded length exceeds declaration"));
            }
            data.extend_from_slice(&buffer[..produced]);
            if status == Status::StreamEnd {
                if decoder.total_in() != compressed_len as u64 {
                    return Err(error("compressed member contains trailing bytes"));
                }
                if decoder.total_out() != decoded_len as u64 {
                    return Err(error("member decoded length differs from declaration"));
                }
                break;
            }
            if before_in == decoder.total_in() && before_out == decoder.total_out() {
                return Err(error("incomplete zlib member"));
            }
        }
        chunks.push(ZlbChunk {
            header: ByteSpan {
                offset: header_offset,
                length: 8,
            },
            compressed: ByteSpan {
                offset: compressed_offset,
                length: compressed_len,
            },
            decoded: ByteSpan {
                offset: decoded_offset,
                length: decoded_len,
            },
        });
    }
}

#[cfg(test)]
mod tests;
