// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Synthetic geometry diagnostics; private CAD is never embedded in tests.
use std::io::Write;

fn file(streams: &[(&str, &[u8])]) -> Vec<u8> {
    let mut bytes = vec![0; 16];
    bytes[7] = 3;
    for (name, data) in streams {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();
        bytes.extend([0x14, 0, 6, 0, 8, 0]);
        for n in [
            1,
            crc32fast::hash(data),
            compressed.len() as u32,
            data.len() as u32,
            name.len() as u32,
        ] {
            bytes.extend(n.to_le_bytes());
        }
        bytes.extend(name.bytes().map(|b| b.rotate_right(3)));
        bytes.extend(compressed);
    }
    bytes
}
fn error(streams: &[(&str, &[u8])]) -> String {
    sw_rs::geometry(&file(streams), "part.SLDPRT")
        .unwrap_err()
        .to_string()
}

#[test]
fn absent_faces_are_distinguished_from_invalid_or_unsupported_face_records() {
    let environment_only = b"\xff\xff\x01\x00\x10\x00uiUserModelEnv_c";
    let message = error(&[("Contents/DisplayLists", environment_only)]);
    assert!(
        message.contains("no uoTempFaceTessData_c records"),
        "{message}"
    );
    // A retained B-Rep payload cannot be used as an invented tessellation.
    assert_eq!(
        message,
        error(&[
            ("Contents/DisplayLists", environment_only),
            ("Config-0-FeatureBodies/LocalBodies", b"opaque saved body"),
        ])
    );
    let message = error(&[(
        "Contents/DisplayLists",
        b"\xff\xff\x01\x00\x12\x00uoTempFaceTessData_c\x01\x00\x00\x00",
    )]);
    assert!(message.contains("face-class markers exist"), "{message}");
    assert!(message.contains("unsupported or invalid"), "{message}");
}

#[test]
fn missing_and_ambiguous_display_carriers_are_distinct() {
    assert_eq!(
        error(&[("other", b"data")]),
        "Part has no DisplayLists carrier"
    );
    assert_eq!(
        error(&[
            ("Contents/DisplayLists", b"one"),
            ("Other/DisplayLists", b"two"),
        ]),
        "ambiguous Part DisplayLists carriers: found 2"
    );
}
