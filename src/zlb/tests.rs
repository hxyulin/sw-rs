// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored chunk wrappers and corruption cases; no legacy CAD fixtures.
use super::*;
use std::io::Write;
fn wrapper(parts: &[&[u8]]) -> Vec<u8> {
    let mut bytes = SIGNATURE.to_vec();
    for part in parts {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(part).unwrap();
        let packed = encoder.finish().unwrap();
        bytes.extend((part.len() as u32).to_le_bytes());
        bytes.extend((packed.len() as u32).to_le_bytes());
        bytes.extend(packed);
    }
    bytes.extend([0; 8]);
    bytes
}
#[test]
fn multiple_members_concatenate_and_spans_partition_the_encoded_stream() {
    let large = vec![17; 1_048_577]; // observed one-MiB chunk size is not an imposed cap.
    let bytes = wrapper(&[b"first", &large, b"last"]);
    let decoded = decode_zlb(&bytes, Limits::unlimited()).unwrap();
    assert_eq!(
        decoded.data,
        [b"first".as_slice(), large.as_slice(), b"last".as_slice()].concat()
    );
    assert_eq!(decoded.chunks.len(), 3);
    let mut encoded_offset = 16;
    let mut output_offset = 0;
    for chunk in &decoded.chunks {
        assert_eq!(chunk.header.offset, encoded_offset);
        assert_eq!(chunk.compressed.offset, encoded_offset + 8);
        assert_eq!(chunk.decoded.offset, output_offset);
        encoded_offset = chunk.compressed.offset + chunk.compressed.length;
        output_offset += chunk.decoded.length;
    }
    assert_eq!(decoded.terminator.offset, encoded_offset);
    assert_eq!(encoded_offset + 8, bytes.len());
    assert_eq!(output_offset, decoded.data.len());
    let empty = decode_zlb(&wrapper(&[]), Limits::default()).unwrap();
    assert!(empty.data.is_empty());
    assert!(empty.chunks.is_empty());
}
#[test]
fn every_truncation_bad_signature_checksum_and_trailing_bytes_fail() {
    let bytes = wrapper(&[b"hello", b"world"]);
    for end in 0..bytes.len() {
        assert!(
            decode_zlb(&bytes[..end], Limits::default()).is_err(),
            "truncation {end}"
        );
    }
    let mut bad = bytes.clone();
    bad[0] ^= 1;
    assert!(decode_zlb(&bad, Limits::default()).is_err());
    let decoded = decode_zlb(&bytes, Limits::default()).unwrap();
    let chunk = &decoded.chunks[1];
    let mut bad = bytes.clone();
    bad[chunk.compressed.offset + chunk.compressed.length - 1] ^= 1;
    assert!(decode_zlb(&bad, Limits::default()).is_err());
    let mut tail = bytes.clone();
    tail.push(0);
    assert!(decode_zlb(&tail, Limits::default()).is_err());
}
#[test]
fn declared_lengths_do_not_trigger_large_preallocations_or_allow_member_padding() {
    let bytes = wrapper(&[b"hello"]);
    for declared in [0, 1, 4, 6, u32::MAX] {
        let mut bad = bytes.clone();
        bad[16..20].copy_from_slice(&declared.to_le_bytes());
        assert!(decode_zlb(&bad, Limits::unlimited()).is_err());
    }
    let mut padded = bytes.clone();
    let length = u32::from_le_bytes(padded[20..24].try_into().unwrap());
    padded[20..24].copy_from_slice(&(length + 1).to_le_bytes());
    padded.insert(24 + length as usize, 0);
    assert!(decode_zlb(&padded, Limits::unlimited()).is_err());
    let mut zero = bytes;
    zero[20..24].copy_from_slice(&0_u32.to_le_bytes());
    assert!(decode_zlb(&zero, Limits::unlimited()).is_err());
}
#[test]
fn cumulative_limits_apply_across_members_and_empty_wrappers_need_no_chunk_slot() {
    let bytes = wrapper(&[b"hello", b"world"]);
    let mut limits = Limits::unlimited();
    limits.max_stream_bytes = 9;
    assert!(decode_zlb(&bytes, limits).is_err());
    limits = Limits::unlimited();
    limits.max_total_bytes = 9;
    assert!(decode_zlb(&bytes, limits).is_err());
    limits = Limits::unlimited();
    limits.max_records = 1;
    assert!(decode_zlb(&bytes, limits).is_err());
    let first = decode_zlb(&bytes, Limits::unlimited()).unwrap().chunks[0]
        .compressed
        .length;
    limits = Limits::unlimited();
    limits.max_compressed_bytes = first;
    assert!(decode_zlb(&bytes, limits).is_err());
    limits = Limits::unlimited();
    limits.max_file_bytes = bytes.len() - 1;
    assert!(decode_zlb(&bytes, limits).is_err());
    limits = Limits::unlimited();
    limits.max_records = 0;
    assert!(decode_zlb(&wrapper(&[]), limits).is_ok());
}
