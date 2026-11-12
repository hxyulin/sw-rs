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

fn triangle_cache() -> Vec<u8> {
    let class = b"uoTempFaceTessData_c";
    let mut bytes = vec![255, 255, 1, 0];
    bytes.extend((class.len() as u16).to_le_bytes());
    bytes.extend(class);
    bytes.extend(1_u32.to_le_bytes()); // Triangle count.
    bytes.extend(1_u32.to_le_bytes()); // Strip count.
    let positions: Vec<_> = [0_f32, 0., 0., 0.01, 0., 0., 0., 0.01, 0.]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    for (size, kind, payload) in [
        (4_u32, 8_u32, 3_u32.to_le_bytes().to_vec()),
        (12, 100, positions),
        (12, 100, vec![]),
        (4, 8, vec![]),
        (4, 8, 4_u32.to_le_bytes().to_vec()),
        (1, 8, vec![]),
    ] {
        for word in [size, kind, 2, payload.len() as u32 / size] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend(payload);
    }
    bytes
}

#[test]
fn cache_inspection_keeps_supported_faces_when_configuration_assignment_is_ambiguous() {
    use sw_rs::{DocumentReader, GeometryCacheStatus, Limits};
    let cache = triangle_cache();
    let single = file(&[
        ("Contents/DisplayLists", &cache),
        ("docProps/Config-7-Properties.xml", b"<Properties/>"),
    ]);
    let reader = DocumentReader::open(&single, "part.SLDPRT", Limits::default()).unwrap();
    let report = reader.inspect_geometry_cache().unwrap();
    assert_eq!(
        report.status,
        GeometryCacheStatus::SupportedSingleConfiguration
    );
    assert_eq!(report.source_sha256, reader.source_sha256());
    assert_eq!(report.assigned_configuration_index, Some(7));
    let face = &report.carriers[0].faces[0];
    assert_eq!(face.vertices, 3);
    assert_eq!(face.triangles, 1);
    assert_eq!(
        reader.geometry().unwrap().model.tessellations[0].source_offset,
        face.source_offset
    );
    assert!(!report.cache_completeness_verified);
    assert!(!report.body_ownership_decoded);

    let multi = file(&[
        ("Contents/DisplayLists", &cache),
        ("docProps/Config-0-Properties.xml", b"<Properties/>"),
        ("docProps/Config-7-Properties.xml", b"<Properties/>"),
    ]);
    let reader = DocumentReader::open(&multi, "part.SLDPRT", Limits::default()).unwrap();
    let report = reader.inspect_geometry_cache().unwrap();
    assert_eq!(report.status, GeometryCacheStatus::AmbiguousConfiguration);
    assert_eq!(report.assigned_configuration_index, None);
    assert_eq!(
        report
            .configurations
            .iter()
            .map(|c| c.index)
            .collect::<Vec<_>>(),
        [0, 7]
    );
    assert_eq!(report.carriers[0].faces[0].triangles, 1);
    assert!(reader.finish().unwrap().geometry.is_none()); // Counts do not install a selected mesh.
}

#[test]
fn cache_inspection_distinguishes_absent_unsupported_and_ambiguous_carriers() {
    use sw_rs::{DocumentReader, GeometryCacheStatus, Limits};
    for (streams, status) in [
        (
            vec![("other", b"bytes".as_slice())],
            GeometryCacheStatus::CarrierAbsent,
        ),
        (
            vec![("Contents/DisplayLists", b"environment only".as_slice())],
            GeometryCacheStatus::NoSupportedFaceRecords,
        ),
        (
            vec![("Contents/DisplayLists", b"uoTempFaceTessData_c".as_slice())],
            GeometryCacheStatus::UnsupportedOrInvalidFaceRecords,
        ),
        (
            vec![
                ("Contents/DisplayLists", b"first".as_slice()),
                ("Other/DisplayLists", b"second".as_slice()),
            ],
            GeometryCacheStatus::AmbiguousCarriers,
        ),
    ] {
        let bytes = file(&streams);
        let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
        let report = reader.inspect_geometry_cache().unwrap();
        assert_eq!(report.status, status);
        assert_eq!(report.assigned_configuration_index, None);
        assert!(report.carriers.iter().all(|c| c.faces.is_empty()));
    }
    let bytes = file(&[("other", b"bytes")]);
    assert!(
        DocumentReader::open(&bytes, "assembly.SLDASM", Limits::default())
            .unwrap()
            .inspect_geometry_cache()
            .is_err()
    );
    assert!(
        DocumentReader::open(b"invalid", "part.SLDPRT", Limits::default())
            .unwrap()
            .inspect_geometry_cache()
            .is_err()
    );
}
