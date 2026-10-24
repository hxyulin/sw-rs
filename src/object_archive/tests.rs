// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored tag/map sequences, not scanned native-object boundaries.
use super::*;
fn class(name: &str, schema: u16) -> Vec<u8> {
    [
        0xffff_u16.to_le_bytes().as_slice(),
        schema.to_le_bytes().as_slice(),
        (name.len() as u16).to_le_bytes().as_slice(),
        name.as_bytes(),
    ]
    .concat()
}
#[test]
fn mixed_class_object_slots_preserve_reuse_null_and_cyclic_references() {
    let mut bytes = class("Root", 7);
    bytes.extend(class("Base", 3)); // SerializeClass: class only, slot 3.
    bytes.extend(0x8001_u16.to_le_bytes()); // new Root object, slot 4.
    bytes.extend(2_u16.to_le_bytes()); // already registered root.
    bytes.extend(0_u16.to_le_bytes());
    let mut reader = BinaryReader::new(&bytes);
    let mut map = ArchiveContext::new();
    let root = map.read_object(&mut reader).unwrap();
    assert_eq!(
        (root.class_index, root.object_index, root.is_new),
        (Some(1), Some(2), true)
    );
    assert_eq!(map.read_class(&mut reader).unwrap(), 3);
    let next = map.read_object(&mut reader).unwrap();
    assert_eq!(
        (next.class_index, next.object_index, next.is_new),
        (Some(1), Some(4), true)
    );
    let cycle = map.read_object(&mut reader).unwrap();
    assert_eq!(
        (cycle.class_index, cycle.object_index, cycle.is_new),
        (Some(1), Some(2), false)
    );
    let null = map.read_object(&mut reader).unwrap();
    assert_eq!(
        (null.class_index, null.object_index, null.is_new),
        (None, None, false)
    );
    assert_eq!(map.entries().len(), 4);
    assert!(reader.remaining().is_empty());
}
#[test]
fn configuration_headers_use_class_slot_three_not_declaration_ordinal() {
    let mut bytes = class("dmConfigMgrHeader_c", 1);
    bytes.extend(class("dmConfigHeader_c", 1));
    bytes.extend(0x8003_u16.to_le_bytes());
    let mut reader = BinaryReader::new(&bytes);
    let mut map = ArchiveContext::new();
    map.read_object(&mut reader).unwrap();
    let first = map.read_object(&mut reader).unwrap();
    let second = map.read_object(&mut reader).unwrap();
    assert_eq!((first.class_index, first.object_index), (Some(3), Some(4)));
    assert_eq!(
        (second.class_index, second.object_index),
        (Some(3), Some(5))
    );
    assert!(
        matches!(map.entry(3),Some(ArchiveEntry::Class {name,schema:1,..}) if name=="dmConfigHeader_c")
    );
}
#[test]
fn wide_indexes_and_external_objects_use_the_same_map() {
    let mut map = ArchiveContext::new();
    for expected in 1..=0x7ffe {
        assert_eq!(map.map_external_object().unwrap(), expected);
    }
    let mut bytes = class("Wide", 2);
    bytes.extend(0x7fff_u16.to_le_bytes());
    bytes.extend(0x8000_7fff_u32.to_le_bytes());
    bytes.extend(0x7fff_u16.to_le_bytes());
    bytes.extend(0x8000_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    let mut reader = BinaryReader::new(&bytes);
    let first = map.read_object(&mut reader).unwrap();
    assert_eq!(
        (first.class_index, first.object_index),
        (Some(0x7fff), Some(0x8000))
    );
    let next = map.read_object(&mut reader).unwrap();
    assert_eq!(
        (next.class_index, next.object_index),
        (Some(0x7fff), Some(0x8001))
    );
    let reference = map.read_object(&mut reader).unwrap();
    assert_eq!(reference.object_index, Some(0x8000));
    assert!(!reference.is_new);
    let external = map.read_object(&mut reader).unwrap();
    assert_eq!(external.object_index, Some(1));
    assert_eq!(external.class_index, None);
}
#[test]
fn failures_leave_cursor_and_map_unchanged_without_resynchronizing() {
    let declaration = class("Test", 1);
    for end in 0..declaration.len() {
        let mut reader = BinaryReader::new(&declaration[..end]);
        let mut map = ArchiveContext::new();
        assert!(map.read_object(&mut reader).is_err());
        assert_eq!(reader.position(), 0);
        assert!(map.entries().is_empty());
    }
    for raw in [
        vec![0, 128],
        vec![1, 128],
        vec![1, 0],
        vec![255, 127, 0, 0, 0, 128],
        vec![255, 127, 255, 255, 255, 255],
        vec![255, 255, 1, 0, 255, 255],
    ] {
        let mut reader = BinaryReader::new(&raw);
        let mut map = ArchiveContext::new();
        assert!(map.read_object(&mut reader).is_err());
        assert_eq!(reader.position(), 0);
        assert!(map.entries().is_empty());
    }
    let mut map = ArchiveContext::new();
    let mut reader = BinaryReader::new(&declaration);
    map.read_object(&mut reader).unwrap();
    for raw in [0x8002_u16.to_le_bytes(), 1_u16.to_le_bytes()] {
        let mut reader = BinaryReader::new(&raw);
        assert!(map.read_object(&mut reader).is_err());
        assert_eq!(reader.position(), 0);
        assert_eq!(map.entries().len(), 2);
    }
    let raw = 2_u16.to_le_bytes();
    let mut reader = BinaryReader::new(&raw);
    assert!(map.read_class(&mut reader).is_err());
    assert_eq!(reader.position(), 0);
    assert_eq!(map.entries().len(), 2);
}
#[test]
fn context_reset_and_explicit_class_headers_do_not_create_object_slots() {
    let raw = class("Base", 5);
    let mut reader = BinaryReader::new(&raw);
    let mut map = ArchiveContext::new();
    assert_eq!(map.read_class(&mut reader).unwrap(), 1);
    assert_eq!(map.entries().len(), 1);
    let reuse = 0x8001_u16.to_le_bytes();
    let mut reader = BinaryReader::new(&reuse);
    assert_eq!(map.read_class(&mut reader).unwrap(), 1);
    assert_eq!(map.entries().len(), 1);
    let mut fresh = ArchiveContext::new();
    let mut reader = BinaryReader::new(&reuse);
    assert!(fresh.read_class(&mut reader).is_err());
}
