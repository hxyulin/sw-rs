// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Archive integrity, resource bounds and virtual-path resolution.
#![cfg(feature = "archive")]
use std::io::{Cursor, Write};
use sw_rs::archive::{ArchiveLimits, FileSet, Resolution, normalize_path};
fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        writer
            .start_file(
                *name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
fn tar(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, bytes) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(bytes.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        builder.append_data(&mut h, *name, *bytes).unwrap();
    }
    builder.into_inner().unwrap()
}
#[test]
fn supported_archives_retain_exact_native_bytes_and_skip_other_files() {
    let files = [
        ("cad/部件.SLDPRT", b"native".as_slice()),
        ("readme.txt", b"ignored".as_slice()),
    ];
    let raw = tar(&files);
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&raw).unwrap();
    for (name, bytes) in [
        ("model.zip", zip(&files)),
        ("model.tar", raw),
        ("model.tar.gz", gzip.finish().unwrap()),
    ] {
        let set = FileSet::from_archive(&bytes, name, ArchiveLimits::default()).unwrap();
        assert_eq!(set.documents["cad/部件.SLDPRT"].as_ref(), b"native");
        assert_eq!(set.ignored_files, 1);
    }
}
#[test]
fn unsafe_duplicate_and_link_members_are_rejected() {
    for path in [
        "../a.SLDPRT",
        "/a.SLDPRT",
        "C:\\a.SLDPRT",
        "x/../../a",
        "x:stream",
        "a\0b",
    ] {
        assert!(normalize_path(path).is_err(), "{path:?}");
    }
    assert_eq!(
        normalize_path("./cad\\part.SLDPRT").unwrap(),
        "cad/part.SLDPRT"
    );
    let z = zip(&[("a.SLDPRT", b"1"), ("A.sldprt", b"2")]);
    assert!(FileSet::from_archive(&z, "a.zip", ArchiveLimits::default()).is_err());
    let z = zip(&[("../a.SLDPRT", b"1")]);
    assert!(FileSet::from_archive(&z, "a.zip", ArchiveLimits::default()).is_err());
    let mut builder = tar::Builder::new(Vec::new());
    let mut h = tar::Header::new_gnu();
    h.set_entry_type(tar::EntryType::Symlink);
    h.set_size(0);
    h.set_mode(0o777);
    h.set_link_name("outside").unwrap();
    h.set_cksum();
    builder.append_data(&mut h, "link", &[][..]).unwrap();
    assert!(
        FileSet::from_archive(
            &builder.into_inner().unwrap(),
            "a.tar",
            ArchiveLimits::default()
        )
        .unwrap_err()
        .to_string()
        .contains("links")
    );
}
#[test]
fn limits_and_damaged_archives_fail_atomically() {
    let z = zip(&[("a.SLDPRT", &[0; 1024]), ("ignored.txt", &[1; 1024])]);
    for limits in [
        ArchiveLimits {
            entries: 1,
            ..Default::default()
        },
        ArchiveLimits {
            member_bytes: 512,
            ..Default::default()
        },
        ArchiveLimits {
            total_bytes: 1500,
            ..Default::default()
        },
        ArchiveLimits {
            input_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(FileSet::from_archive(&z, "a.zip", limits).is_err());
    }
    for end in [0, 10, z.len() / 2, z.len() - 10] {
        assert!(FileSet::from_archive(&z[..end], "a.zip", Default::default()).is_err());
    }
    assert!(FileSet::from_archive(&[0; 100], "a.tar", Default::default()).is_err());
    assert!(
        FileSet::from_archive(&[0; 10], "a.tar.zst", Default::default())
            .unwrap_err()
            .to_string()
            .contains("supported archives")
    );
    assert!(
        FileSet::from_archive(&zip(&[("readme", b"x")]), "a.zip", Default::default())
            .unwrap_err()
            .to_string()
            .contains("no native")
    );
}
#[test]
fn virtual_resolution_prefers_owner_relative_and_never_selects_ambiguous_names() {
    let files = zip(&[
        ("cad/root.SLDASM", b"a"),
        ("cad/a/p.SLDPRT", b"b"),
        ("cad/b/p.SLDPRT", b"c"),
    ]);
    let s = FileSet::from_archive(&files, "a.zip", Default::default()).unwrap();
    assert_eq!(
        s.resolve("cad/root.SLDASM", "a\\p.sldprt"),
        Resolution::Found("cad/a/p.SLDPRT".into())
    );
    assert_eq!(
        s.resolve("cad/root.SLDASM", "C:\\old\\b\\p.SLDPRT"),
        Resolution::Found("cad/b/p.SLDPRT".into())
    );
    assert!(matches!(
        s.resolve("cad/root.SLDASM", "p.SLDPRT"),
        Resolution::Ambiguous(_)
    ));
    assert_eq!(
        s.resolve("cad/root.SLDASM", "../../p.SLDPRT"),
        Resolution::Missing
    );
}

#[test]
fn crc_failure_is_checked_even_for_ignored_members() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "ignored.txt",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(b"checksum-protected").unwrap();
    writer
        .start_file(
            "p.SLDPRT",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(b"source").unwrap();
    let mut bytes = writer.finish().unwrap().into_inner();
    let name = u16::from_le_bytes(bytes[26..28].try_into().unwrap()) as usize;
    let extra = u16::from_le_bytes(bytes[28..30].try_into().unwrap()) as usize;
    bytes[30 + name + extra] ^= 1;
    assert!(
        FileSet::from_archive(&bytes, "a.zip", Default::default())
            .unwrap_err()
            .to_string()
            .contains("damaged archive member")
    );
}

#[test]
fn unlimited_import_reports_validated_entries_for_each_archive_format() {
    let files = [
        ("part.SLDPRT", b"CAD".as_slice()),
        ("ignored.txt", b"text".as_slice()),
    ];
    let raw = tar(&files);
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&raw).unwrap();
    for (name, bytes, total) in [
        ("a.zip", zip(&files), Some(2)),
        ("a.tar", raw, None),
        ("a.tar.gz", gzip.finish().unwrap(), None),
    ] {
        let mut events = Vec::new();
        let set =
            FileSet::from_archive_with_progress(&bytes, name, ArchiveLimits::unlimited(), |p| {
                events.push((p.completed_entries, p.total_entries, p.path.to_owned()))
            })
            .unwrap();
        assert_eq!(
            events,
            vec![
                (1, total, "part.SLDPRT".into()),
                (2, total, "ignored.txt".into())
            ]
        );
        assert_eq!(set.documents["part.SLDPRT"].as_ref(), b"CAD");
    }
}
#[test]
fn local_import_can_exceed_the_old_member_byte_cap() {
    let large = vec![0; (64 << 20) + 1];
    let z = zip(&[("large.SLDPRT", &large)]);
    assert!(FileSet::from_archive(&z, "large.zip", ArchiveLimits::default()).is_err());
    let set = FileSet::from_archive(&z, "large.zip", ArchiveLimits::unlimited()).unwrap();
    assert_eq!(set.documents["large.SLDPRT"].len(), large.len());
    // Removing policy caps must not disable path validation.
    let bad = zip(&[("../bad.SLDPRT", b"CAD")]);
    assert!(FileSet::from_archive(&bad, "bad.zip", ArchiveLimits::unlimited()).is_err());
}
