// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Transactional cursor and Unicode framing regressions.
use sw_rs::BinaryReader;
#[test]
fn checked_borrows_and_failed_compound_reads_are_transactional() {
    let bytes = 1_u32.to_le_bytes();
    let mut reader = BinaryReader::new(&bytes);
    assert!(std::ptr::eq(reader.take(2).unwrap(), &bytes[..2]));
    assert_eq!(reader.position(), 2);
    assert!(reader.take(usize::MAX).is_none());
    assert_eq!(reader.position(), 2);
    assert!(reader.words::<2>().is_none());
    assert_eq!(reader.position(), 2);
    assert!(BinaryReader::at(&bytes, usize::MAX).is_none());
    let nan = f64::NAN.to_le_bytes();
    let mut reader = BinaryReader::new(&nan);
    assert!(reader.doubles::<1>().is_none());
    assert_eq!(reader.position(), 0);
    assert!(reader.f64().unwrap().is_nan());
}
#[test]
fn utf16_surrogates_extended_lengths_and_malformed_frames_keep_offsets() {
    let value = "纹理😀".repeat(100);
    let units = value.encode_utf16().collect::<Vec<_>>();
    let mut bytes = vec![0; 7];
    bytes.extend([255, 254, 255, 255]);
    bytes.extend((units.len() as u32).to_le_bytes());
    bytes.extend(units.into_iter().flat_map(u16::to_le_bytes));
    let mut reader = BinaryReader::at(&bytes, 7).unwrap();
    let text = reader.string().unwrap();
    assert_eq!(text.offset, 7);
    assert_eq!(text.value, value);
    assert!(reader.remaining().is_empty());
    for bad in [
        vec![255, 254, 255, 1, 0, 0xd8],
        vec![255, 254, 255, 255, 255, 255, 255, 255],
    ] {
        let mut reader = BinaryReader::new(&bad);
        assert!(reader.string().is_none());
        assert_eq!(reader.position(), 0);
    }
}
