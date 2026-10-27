// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored CFB fixtures: fragmented source ranges, wrappers, limits and corruption.
use std::io::{Cursor, Seek, SeekFrom, Write};
use sw_rs::{
    Container, ContainerRecordStatus, DocumentReader, Limits, OleCompression, SourceFormat,
};

fn compound(version: cfb::Version, streams: &[(&str, &[u8])]) -> Vec<u8> {
    let mut file =
        cfb::CompoundFile::create_with_version(version, Cursor::new(Vec::new())).unwrap();
    for (name, bytes) in streams {
        if let Some((storage, _)) = name.rsplit_once('/')
            && !storage.is_empty()
        {
            file.create_storage_all(storage).unwrap();
        }
        file.create_stream(name).unwrap().write_all(bytes).unwrap();
    }
    file.into_inner().into_inner()
}
fn zlb(chunks: &[&[u8]]) -> Vec<u8> {
    let mut out = vec![
        0x23, 0x1d, 0xd5, 0x71, 0xda, 0x81, 0x48, 0xa2, 0xa8, 0x58, 0x98, 0xb2, 0x1b, 0x89, 0xef,
        0x99,
    ];
    for bytes in chunks {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(bytes).unwrap();
        let packed = encoder.finish().unwrap();
        out.extend((bytes.len() as u32).to_le_bytes());
        out.extend((packed.len() as u32).to_le_bytes());
        out.extend(packed);
    }
    out.extend([0; 8]);
    out
}
fn assert_accounted(reader: &DocumentReader<'_>) {
    let inventory = reader.inventory();
    let mut all = inventory.verified_container_ranges.clone();
    all.extend(&inventory.unclassified_ranges);
    all.sort_by_key(|span| span.offset);
    let mut end = 0;
    for span in all {
        assert_eq!(span.offset, end);
        assert!(span.slice(reader.source_bytes()).is_some());
        end += span.length;
    }
    assert_eq!(end, reader.source_bytes().len());
}

#[test]
fn raw_and_multimember_streams_preserve_unicode_paths_and_physical_provenance() {
    let packed = zlb(&[b"first", b"second"]);
    let empty = zlb(&[]);
    for version in [cfb::Version::V3, cfb::Version::V4] {
        let bytes = compound(
            version,
            &[
                ("/Contents/DisplayLists__ZLB", &packed),
                ("/swXmlContents/KeyWords__ZLB", &empty),
                ("/未知/資料", b"native"),
                ("/\u{5}SummaryInformation", b"property set"),
                ("/empty", b""),
                ("/opaque", &packed), // Signature alone does not authorize decompression.
            ],
        );
        let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
        let inventory = reader.inventory();
        assert_eq!(inventory.format, SourceFormat::LegacyOle2);
        assert!(
            inventory.container_error.is_none(),
            "{:?}",
            inventory.container_error
        );
        assert_eq!(
            reader.stream("Contents/DisplayLists").unwrap(),
            Some(b"firstsecond".as_slice())
        );
        assert_eq!(reader.stream("Contents/DisplayLists__ZLB").unwrap(), None);
        assert_eq!(
            reader.stream("swXmlContents/KeyWords").unwrap(),
            Some(b"".as_slice())
        );
        assert_eq!(
            reader.stream("未知/資料").unwrap(),
            Some(b"native".as_slice())
        );
        assert_eq!(
            reader.stream("\u{5}SummaryInformation").unwrap(),
            Some(b"property set".as_slice())
        );
        assert_eq!(reader.stream("opaque").unwrap(), Some(packed.as_slice()));
        for record in &inventory.records {
            assert_eq!(record.status, ContainerRecordStatus::Selected);
            assert!(record.payload_span.is_none());
            assert!(record.name_span.is_none());
            assert!(record.expected_crc32.is_none());
            let source = record.ole_source.as_ref().unwrap();
            let encoded: Vec<u8> = source
                .ranges
                .iter()
                .flat_map(|span| span.slice(&bytes).unwrap())
                .copied()
                .collect();
            assert_eq!(encoded.len(), source.encoded_bytes);
            if source.path == "Contents/DisplayLists__ZLB" {
                assert_eq!(encoded, packed);
                assert!(
                    matches!(&source.compression, Some(OleCompression::Zlb { chunks, .. }) if chunks.len() == 2)
                );
            }
        }
        assert!(inventory.verified_container_bytes < bytes.len());
        assert_accounted(&reader);
        assert!(std::ptr::eq(
            reader.container().unwrap(),
            reader.container().unwrap()
        ));
        assert!(std::ptr::eq(
            reader.metadata().unwrap(),
            reader.metadata().unwrap()
        ));
        let out = reader.finish().unwrap();
        assert!(out.metadata.is_some());
        assert!(out.inventory.is_some());
    }
}

#[test]
fn fragmented_regular_and_mini_streams_reconstruct_in_logical_order() {
    for size in [64, 5120] {
        let mut file =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new()))
                .unwrap();
        let prefix = vec![17; size];
        let suffix = vec![29; size];
        file.create_stream("/a")
            .unwrap()
            .write_all(&prefix)
            .unwrap();
        file.create_stream("/b")
            .unwrap()
            .write_all(&prefix)
            .unwrap();
        let mut a = file.open_stream("/a").unwrap();
        a.seek(SeekFrom::End(0)).unwrap();
        a.write_all(&suffix).unwrap();
        drop(a);
        let bytes = file.into_inner().into_inner();
        let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
        assert_eq!(
            reader.stream("a").unwrap(),
            Some([prefix.clone(), suffix].concat().as_slice())
        );
        let record = reader
            .inventory()
            .records
            .iter()
            .find(|r| r.name.as_deref() == Some("a"))
            .unwrap();
        assert!(record.ole_source.as_ref().unwrap().ranges.len() >= 2);
        assert_accounted(&reader);
    }
}

#[test]
fn zlb_display_cache_reaches_the_shared_geometry_decoder_with_source_units() {
    let class = b"uoTempFaceTessData_c";
    let mut face = vec![255, 255, 1, 0];
    face.extend((class.len() as u16).to_le_bytes());
    face.extend(class);
    face.extend(1u32.to_le_bytes()); // One triangle.
    face.extend(1u32.to_le_bytes()); // One strip.
    for (size, kind, count, payload) in [
        (4u32, 8u32, 1u32, 3u32.to_le_bytes().to_vec()),
        (
            12,
            100,
            3,
            [0.0f32, 0.0, 0.0, 0.01, 0.0, 0.0, 0.0, 0.01, 0.0]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect(),
        ),
        (12, 100, 3, vec![0; 36]),
        (4, 8, 4, vec![0; 16]),
        (4, 8, 1, 4u32.to_le_bytes().to_vec()),
        (1, 8, 4, vec![0; 4]),
    ] {
        for word in [size, kind, 2, count] {
            face.extend(word.to_le_bytes());
        }
        face.extend(payload);
    }
    let bytes = compound(
        cfb::Version::V3,
        &[("/Contents/DisplayLists__ZLB", &zlb(&[&face]))],
    );
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    let geometry = reader.geometry().unwrap();
    let mesh = &geometry.model.tessellations[0];
    assert_eq!(mesh.source_stream, "Contents/DisplayLists");
    assert_eq!(mesh.triangles, [[0, 1, 2]]);
    assert!((mesh.vertices[1][0] - 10.0).abs() < 0.000001);
    assert_eq!(geometry.length_unit, "millimeter");
    assert!(std::ptr::eq(geometry, reader.geometry().unwrap()));
    assert!(reader.finish().unwrap().geometry.unwrap().is_ok());
}

#[test]
fn normalized_name_collisions_and_invalid_wrappers_fail_the_container() {
    let good = zlb(&[b"new"]);
    let bytes = compound(cfb::Version::V3, &[("/x", b"old"), ("/x__ZLB", &good)]);
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    assert!(
        reader
            .container()
            .unwrap_err()
            .to_string()
            .contains("collision")
    );
    assert!(
        reader
            .inventory()
            .records
            .iter()
            .any(|r| r.status == ContainerRecordStatus::Duplicate)
    );
    assert_accounted(&reader);
    for bad in [b"not ZLB".to_vec(), good[..good.len() - 1].to_vec(), {
        let mut bad = good.clone();
        let index = bad.len() - 9; // Last member's Adler checksum.
        bad[index] ^= 1;
        bad
    }] {
        let bytes = compound(
            cfb::Version::V3,
            &[("/a", b"kept only in inventory"), ("/bad__ZLB", &bad)],
        );
        let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
        assert!(reader.metadata().is_err());
        let inventory = reader.inventory();
        assert!(!inventory.decoded_streams_available);
        let rejected = inventory
            .records
            .iter()
            .find(|r| r.status == ContainerRecordStatus::Rejected)
            .unwrap();
        assert!(rejected.ole_source.as_ref().unwrap().compression.is_none());
        assert!(rejected.diagnostic.is_some());
        assert_accounted(&reader);
    }
}

#[test]
fn document_budgets_include_all_raw_streams_and_zlb_members() {
    let packed = zlb(&[b"1234", b"5678"]);
    let bytes = compound(cfb::Version::V3, &[("/a", b"1234"), ("/b__ZLB", &packed)]);
    let exact = Limits {
        max_stream_bytes: 8,
        max_total_bytes: 12,
        max_compressed_bytes: packed.len(),
        max_records: 4,
        ..Limits::default()
    };
    assert!(Container::parse(&bytes, exact).is_ok());
    for limit in [
        Limits {
            max_file_bytes: bytes.len() - 1,
            ..exact
        },
        Limits {
            max_stream_bytes: 7,
            ..exact
        },
        Limits {
            max_total_bytes: 11,
            ..exact
        },
        Limits {
            max_compressed_bytes: packed.len() - 1,
            ..exact
        },
        Limits {
            max_records: 3,
            ..exact
        },
    ] {
        assert!(Container::parse(&bytes, limit).is_err());
    }
    let raw = compound(cfb::Version::V3, &[("/a", b"1234"), ("/b", b"5678")]);
    assert!(
        Container::parse(
            &raw,
            Limits {
                max_total_bytes: 7,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let nested = compound(cfb::Version::V3, &[("/a/b/c", b"")]);
    assert!(
        Container::parse(
            &nested,
            Limits {
                max_records: 2,
                ..Limits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn malformed_headers_chains_and_truncated_payloads_are_rejected_without_panics() {
    let bytes = compound(cfb::Version::V3, &[("/regular", &vec![42; 6000])]);
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    let span = reader.inventory().records[0]
        .ole_source
        .as_ref()
        .unwrap()
        .ranges[0];
    let mut bad_shift = bytes.clone();
    bad_shift[30..32].copy_from_slice(&31u16.to_le_bytes());
    let mut cycle = bytes.clone();
    let fat = u32::from_le_bytes(bytes[76..80].try_into().unwrap()) as usize;
    let sector = span.offset / 512 - 1;
    let fat_entry = (fat + 1) * 512 + sector * 4;
    cycle[fat_entry..fat_entry + 4].copy_from_slice(&(sector as u32).to_le_bytes());
    let mut tree_cycle = bytes.clone();
    let directory = u32::from_le_bytes(bytes[48..52].try_into().unwrap()) as usize;
    let root_child = (directory + 1) * 512 + 76;
    tree_cycle[root_child..root_child + 4].copy_from_slice(&0u32.to_le_bytes());
    for malformed in [
        bytes[..100].to_vec(),
        bytes[..span.offset + span.length - 1].to_vec(),
        bad_shift,
        cycle,
        tree_cycle,
    ] {
        assert!(Container::parse(&malformed, Limits::default()).is_err());
        let reader = DocumentReader::open(&malformed, "part.SLDPRT", Limits::default()).unwrap();
        assert!(reader.inventory().container_error.is_some());
        assert_accounted(&reader);
    }
}

#[test]
fn shared_stream_sectors_and_repeated_fat_amplification_are_rejected() {
    let bytes = compound(
        cfb::Version::V3,
        &[("/a", &vec![17; 6000]), ("/b", &vec![29; 6000])],
    );
    let directory = u32::from_le_bytes(bytes[48..52].try_into().unwrap()) as usize;
    let directory_start = (directory + 1) * 512;
    let mut shared = bytes.clone();
    let first_start = bytes[directory_start + 128 + 116..directory_start + 128 + 120].to_vec();
    shared[directory_start + 256 + 116..directory_start + 256 + 120].copy_from_slice(&first_start);
    assert!(
        Container::parse(&shared, Limits::default())
            .unwrap_err()
            .to_string()
            .contains("overlapping")
    );

    // The FAT itself must not expand many copies of one small physical sector.
    let fat = &bytes[76..80];
    let mut repeated = bytes.clone();
    repeated[44..48].copy_from_slice(&109u32.to_le_bytes());
    for index in 0..109 {
        repeated[76 + index * 4..80 + index * 4].copy_from_slice(fat);
    }
    assert!(
        Container::parse(&repeated, Limits::default())
            .unwrap_err()
            .to_string()
            .contains("physical read budget")
    );
}

#[test]
fn model_header_reports_share_the_normalized_raw_and_zlb_ole_stream_path() {
    let mut header = vec![255, 255, 1, 0, 10, 0];
    header.extend(b"moHeader_c");
    header.extend([0; 90]);
    let packed = zlb(&[&header]);
    for version in [cfb::Version::V3, cfb::Version::V4] {
        let bytes = compound(
            version,
            &[
                ("/Contents/Config-0-ModelHeader", &header),
                ("/Contents/Config-1-ModelHeader__ZLB", &packed),
            ],
        );
        let reader = DocumentReader::open(&bytes, "assembly.SLDASM", Limits::default()).unwrap();
        let report = reader.model_headers().unwrap();
        assert_eq!(report.streams.len(), 2);
        assert_eq!(report.source_sha256, reader.source_sha256());
        for (index, stream) in report.streams.iter().enumerate() {
            assert_eq!(stream.configuration_index, Some(index as u32));
            assert_eq!(stream.status, sw_rs::NativeModelHeaderStatus::Decoded);
            assert_eq!(
                stream.source_stream,
                format!("Contents/Config-{index}-ModelHeader")
            );
            let decoding = stream.decoding.as_ref().unwrap();
            assert_eq!(
                decoding.objects[&2]
                    .span
                    .slice(reader.stream(&stream.source_stream).unwrap().unwrap()),
                Some(header.as_slice())
            );
        }
    }
}
