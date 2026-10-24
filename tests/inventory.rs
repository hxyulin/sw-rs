// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Source accounting, cache ownership and structural inspection regressions.
use std::io::Write;
use sw_rs::{
    ByteSpan, Container, ContainerRecordStatus as Status, DocumentReader, Limits, SourceFormat,
};
fn file(records: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = vec![0; 16];
    out[7] = 3;
    for (name, bytes) in records {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(bytes).unwrap();
        let packed = encoder.finish().unwrap();
        out.extend([20, 0, 6, 0, 8, 0]);
        for v in [
            1,
            crc32fast::hash(bytes),
            packed.len() as u32,
            bytes.len() as u32,
            name.len() as u32,
        ] {
            out.extend(v.to_le_bytes());
        }
        out.extend(name.bytes().map(|b| b.rotate_right(3)));
        out.extend(packed);
    }
    out
}
fn assert_accounted(reader: &DocumentReader<'_>) {
    let inventory = reader.inventory();
    let mut spans = inventory.verified_container_ranges.clone();
    spans.extend(&inventory.unclassified_ranges);
    spans.sort_by_key(|s| s.offset);
    let mut at = 0;
    for span in spans {
        assert_eq!(span.offset, at);
        at += span.length;
        assert!(span.slice(reader.source_bytes()).is_some());
    }
    assert_eq!(at, inventory.source_bytes);
    assert_eq!(
        inventory.verified_container_bytes
            + inventory
                .unclassified_ranges
                .iter()
                .map(|s| s.length)
                .sum::<usize>(),
        inventory.source_bytes
    );
}
#[test]
fn records_keep_offsets_hashes_duplicates_and_raw_byte_accounting() {
    let mut bytes = file(&[("未知", b"first"), ("未知", b"second"), ("empty", b"")]);
    bytes.extend(b"opaque tail");
    let legacy = Container::parse(&bytes, Limits::default()).unwrap();
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    assert_eq!(reader.container().unwrap().streams, legacy.streams);
    assert!(std::ptr::eq(
        reader.stream("未知").unwrap().unwrap(),
        reader.stream("未知").unwrap().unwrap()
    ));
    let inventory = reader.inventory();
    assert_eq!(inventory.format, SourceFormat::ModernInline);
    assert!(inventory.decoded_streams_available);
    assert_eq!(
        inventory
            .records
            .iter()
            .map(|r| r.status)
            .collect::<Vec<_>>(),
        [Status::Selected, Status::Duplicate, Status::Selected]
    );
    assert_ne!(
        inventory.records[0].decoded_sha256,
        inventory.records[1].decoded_sha256
    );
    assert_eq!(inventory.streams.len(), 2);
    assert_eq!(inventory.records[0].offset, 16);
    assert_eq!(
        inventory.records[0]
            .name_span
            .unwrap()
            .slice(&bytes)
            .unwrap()
            .iter()
            .map(|v| v.rotate_left(3))
            .collect::<Vec<_>>(),
        "未知".as_bytes()
    );
    assert!(
        inventory.records[0]
            .payload_span
            .unwrap()
            .slice(&bytes)
            .is_some()
    );
    assert!(
        inventory
            .streams
            .iter()
            .all(|s| s.semantic_status == "not_evaluated")
    );
    assert!(std::ptr::eq(inventory, reader.inventory()));
    assert_accounted(&reader);
}
#[test]
fn corrupt_first_record_does_not_hide_later_valid_record_or_opaque_bytes() {
    let mut bytes = file(&[("bad", b"bad"), ("good", b"good")]);
    bytes[26] ^= 1;
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    assert!(reader.stream("bad").unwrap().is_none());
    assert_eq!(reader.stream("good").unwrap(), Some(b"good".as_slice()));
    assert_eq!(reader.inventory().records[0].status, Status::Rejected);
    assert_eq!(
        reader.inventory().records[0].diagnostic.as_deref(),
        Some("CRC mismatch")
    );
    assert_accounted(&reader);
}
#[test]
fn unsupported_formats_and_all_invalid_files_remain_inspectable() {
    for (bytes, format) in [
        (
            b"\xd0\xcf\x11\xe0opaque".as_slice(),
            SourceFormat::LegacyOle2,
        ),
        (b"PK\x03\x04opaque", SourceFormat::ZipOpc),
        (b"unknown", SourceFormat::Unknown),
        (b"", SourceFormat::Unknown),
    ] {
        let reader = DocumentReader::open(bytes, "part.SLDPRT", Limits::default()).unwrap();
        assert_eq!(reader.inventory().format, format);
        assert!(reader.inventory().container_error.is_some());
        assert!(!reader.inventory().decoded_streams_available);
        assert!(reader.metadata().is_err());
        assert!(reader.stream("anything").is_err());
        assert_accounted(&reader);
    }
    let bytes = file(&[("bad", b"bad")]);
    let limited = Limits {
        max_stream_bytes: 1,
        ..Limits::default()
    };
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", limited).unwrap();
    assert!(
        reader
            .inventory()
            .container_error
            .as_ref()
            .unwrap()
            .contains("limit")
    );
    assert_accounted(&reader);
    assert!(
        DocumentReader::open(
            &bytes,
            "part",
            Limits {
                max_file_bytes: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        ByteSpan {
            offset: usize::MAX,
            length: 2
        }
        .slice(&bytes)
        .is_none()
    );
}
#[test]
fn metadata_and_finish_reuse_one_container_without_consuming_borrowed_streams() {
    let bytes = file(&[("docProps/core.xml", b"<core><title>Test</title></core>")]);
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    let first = reader.metadata().unwrap();
    assert!(std::ptr::eq(first, reader.metadata().unwrap()));
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(sw_rs::parse(&bytes, "part.SLDPRT").unwrap()).unwrap()
    );
    assert!(reader.geometry().is_err());
    assert!(reader.geometry().is_err());
    reader.inventory();
    let pointer = reader
        .stream("docProps/core.xml")
        .unwrap()
        .unwrap()
        .as_ptr();
    let output = reader.finish().unwrap();
    assert_eq!(
        pointer,
        output.container.streams["docProps/core.xml"].as_ptr()
    );
    assert!(output.metadata.is_some());
    assert!(output.geometry.unwrap().is_err());
    assert!(output.inventory.is_some());
}
#[test]
fn xml_inventory_preserves_namespaces_text_unknown_attributes_and_ambiguous_links() {
    let xml=br#"<swSolidWorks xmlns="http://www.solidworks.com/sw2003/schema" xmlns:q="urn:extra"><swFile id="a"/><swFile id="a"/><swModel id="b" swFileRef="a" swModelRef="missing" q:id="other" q:unknown="keep">A&amp;B<Unknown x="7"/>tail</swModel></swSolidWorks>"#;
    let bytes = file(&[("swXmlContents/Features", xml)]);
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    let inspection = reader
        .inspect_stream("swXmlContents/Features")
        .unwrap()
        .unwrap();
    assert_eq!(inspection.structure_status, "xml");
    assert_eq!(inspection.xml_elements.len(), 5);
    assert_eq!(inspection.xml_elements[4].parent, Some(3));
    assert_eq!(inspection.xml_elements[3].span.slice(xml).unwrap()[0], b'<');
    assert!(
        inspection.xml_elements[3]
            .attributes
            .iter()
            .any(|a| a.name == "unknown"
                && a.namespace.as_deref() == Some("urn:extra")
                && a.value == "keep")
    );
    assert_eq!(
        inspection
            .xml_text
            .iter()
            .map(|t| t.value.as_str())
            .collect::<Vec<_>>(),
        ["A&B", "tail"]
    );
    assert_eq!(inspection.xml_references[0].candidate_elements, [1, 2]);
    assert!(inspection.xml_references[1].candidate_elements.is_empty());
    assert!(!inspection.native_object_graph_decoded);
    assert!(reader.inspect_stream("missing").unwrap().is_none());
}
#[test]
fn binary_candidates_and_malformed_xml_never_claim_native_objects() {
    let mut binary = b"opaque".to_vec();
    binary.extend([255, 255, 1, 0, 7, 0]);
    binary.extend(b"Unknown");
    binary.extend([255, 254, 255, 2, b'A', 0, b'B', 0]);
    let bytes = file(&[("unknown", &binary), ("bad.xml", b"<invalid>")]);
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    let inspection = reader.inspect_stream("unknown").unwrap().unwrap();
    assert_eq!(inspection.structure_status, "binary_candidates");
    assert_eq!(inspection.class_declarations[0].name, "Unknown");
    assert_eq!(inspection.class_declarations[0].status, "candidate");
    assert_eq!(inspection.class_declarations[0].span.offset, 6);
    assert_eq!(inspection.framed_strings[0].value, "AB");
    assert!(!inspection.native_object_graph_decoded);
    let bad = reader.inspect_stream("bad.xml").unwrap().unwrap();
    assert_eq!(bad.structure_status, "invalid_xml");
    assert!(bad.xml_error.is_some());
}

#[test]
fn index_records_and_all_truncations_have_truthful_source_accounting() {
    let mut bytes = file(&[("good", b"good")]);
    let directory = bytes.len();
    bytes.extend([20, 0, 6, 0, 8, 0]);
    for v in [1_u32, 0, 0, 0, 3] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend([0; 14]);
    bytes.extend(b"dir".iter().map(|b| b.rotate_right(3)));
    bytes.extend([0; 6]);
    bytes.extend([20, 0, 6, 0, 8, 0]);
    for v in [1_u32, 8, 2, 4, 5] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend(b"cache".iter().map(|b| b.rotate_right(3)));
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    let inventory = reader.inventory();
    assert_eq!(inventory.records[1].status, Status::DirectoryIndex);
    assert_eq!(inventory.records[1].name.as_deref(), Some("dir"));
    assert_eq!(
        inventory.records[1].name_span.unwrap().offset,
        directory + 40
    );
    assert_eq!(inventory.records[2].status, Status::CacheIndex);
    assert!(inventory.records[2].payload_span.is_none());
    assert_accounted(&reader);
    for end in 0..bytes.len() {
        let reader = DocumentReader::open(&bytes[..end], "part.SLDPRT", Limits::default()).unwrap();
        assert_accounted(&reader);
    }
}
#[test]
fn empty_streams_are_not_reported_as_invalid_xml() {
    let bytes = file(&[("empty.xml", b"")]);
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    let inspection = reader.inspect_stream("empty.xml").unwrap().unwrap();
    assert_eq!(inspection.structure_status, "empty");
    assert!(inspection.xml_error.is_none());
}

#[test]
fn utf16_xml_and_keyword_wrappers_keep_original_byte_spans() {
    let xml = format!(
        "<root attr=\"{}\">纹理😀&amp;value</root>",
        "prefix".repeat(100)
    );
    for little in [true, false] {
        let mut raw = if little {
            vec![255, 254]
        } else {
            vec![254, 255]
        };
        for unit in xml.encode_utf16() {
            raw.extend(if little {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
        let bytes = file(&[("data.xml", &raw)]);
        let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
        let inspection = reader.inspect_stream("data.xml").unwrap().unwrap();
        assert_eq!(inspection.structure_status, "xml");
        assert_eq!(
            inspection.xml_elements[0].span,
            ByteSpan {
                offset: 2,
                length: raw.len() - 2
            }
        );
        assert_eq!(inspection.xml_text[0].value, "纹理😀&value");
        let text = inspection.xml_text[0].span.slice(&raw).unwrap();
        let units = text
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| {
                if little {
                    u16::from_le_bytes(*b)
                } else {
                    u16::from_be_bytes(*b)
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(String::from_utf16(&units).unwrap(), "纹理😀&amp;value");
    }
    let xml = b"<?xml version=\"1.0\"?><Keywords><Unknown/></Keywords>";
    let mut raw = vec![0x86];
    raw.extend(xml);
    let bytes = file(&[("swXmlContents/KeyWords", &raw)]);
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    let inspection = reader
        .inspect_stream("swXmlContents/KeyWords")
        .unwrap()
        .unwrap();
    assert_eq!(inspection.structure_status, "xml");
    assert_eq!(
        inspection.xml_payload_span,
        Some(ByteSpan {
            offset: 1,
            length: xml.len()
        })
    );
    assert_eq!(
        inspection.opaque_prefix,
        Some(ByteSpan {
            offset: 0,
            length: 1
        })
    );
    assert!(
        inspection.xml_elements[0]
            .span
            .slice(&raw)
            .unwrap()
            .starts_with(b"<Keywords>")
    );
}
#[test]
fn incidental_xml_prefixes_in_binary_data_do_not_claim_malformed_documents() {
    let bytes = file(&[("opaque", b"<\xffbinary")]);
    let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    let inspection = reader.inspect_stream("opaque").unwrap().unwrap();
    assert_eq!(inspection.structure_status, "binary_candidates");
    assert!(inspection.xml_error.is_none());
}
