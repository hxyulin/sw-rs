// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored native reference strings and bounded-window regressions.
use super::*;
fn reference(text: &str) -> SavedSurfaceReference {
    SavedSurfaceReference::parse(&FramedString {
        value: text.into(),
        offset: 12,
    })
    .unwrap()
}
fn set(strings: &[&str]) -> CachedSurfaceReferences {
    CachedSurfaceReferences {
        metadata_span: ByteSpan {
            offset: 0,
            length: 0,
        },
        references: strings.iter().map(|s| reference(s)).collect(),
    }
}
#[test]
fn signed_tail_bits_and_full_identity_length_are_preserved() {
    let reference = reference("moFromSktEnt3IntSurfIdRep_c,120,0,-1,-2147483648,2147483647, \n");
    assert_eq!(reference.status, SurfaceReferenceStatus::Complete);
    assert_eq!(reference.source.offset, 12);
    assert!(reference.terminal_comma);
    let identity = reference.identity.unwrap();
    assert_eq!(identity.feature_source_id, 120);
    assert_eq!(identity.local_surface_id, 0);
    assert_eq!(
        identity.trailing_fields,
        [u32::MAX, 0x8000_0000, 0x7fff_ffff]
    );
    assert_eq!(reference.tail_tokens, ["-1", "-2147483648", "2147483647"]);
    assert_ne!(identity, reference_identity("moSimpleSurfIdRep_c,120,0,"));
}
fn reference_identity(text: &str) -> SavedSurfaceIdentity {
    reference(text).identity.unwrap()
}
#[test]
fn invalid_and_opaque_surface_tokens_survive_without_truncating_identity() {
    for text in [
        "moSimpleSurfIdRep_c,0,1,",
        "moSimpleSurfIdRep_c,4294967295,1,",
        "moSimpleSurfIdRep_c,7,",
        "moSimpleSurfIdRep_c,7,no,",
        "moSimpleSurfIdRep_c",
    ] {
        let r = reference(text);
        assert_eq!(r.status, SurfaceReferenceStatus::Invalid);
        assert!(r.identity.is_none());
        assert_eq!(r.source.value, text);
    }
    for tail in [
        "opaque",
        "4294967295",
        "2147483648",
        "-2147483649",
        "",
        " 1",
    ] {
        let text = format!("moSimpleSurfIdRep_c,7,2,{tail},");
        let r = reference(&text);
        assert_eq!(r.status, SurfaceReferenceStatus::OpaqueTail);
        assert!(r.identity.is_none());
        assert_eq!(r.feature_source_id, Some(7));
        assert_eq!(r.tail_tokens, [tail]);
    }
    assert!(
        SavedSurfaceReference::parse(&FramedString {
            offset: 0,
            value: "moBodyIdRep_c,7,2,".into()
        })
        .is_none()
    );
}
#[test]
fn duplicate_consensus_never_discards_conflicting_tails_or_higher_order_evidence() {
    let identical = set(&["moSimpleSurfIdRep_c,7,2,-1,", "moSimpleSurfIdRep_c,7,2,-1,"]);
    assert!(matches!(
        identical.identity_resolution(),
        SurfaceIdentityResolution::Consistent { .. }
    ));
    assert_eq!(identical.references.len(), 2);
    let conflict = set(&["moSimpleSurfIdRep_c,7,2,-1,", "moSimpleSurfIdRep_c,7,2,"]);
    assert_eq!(
        conflict.identity_resolution(),
        SurfaceIdentityResolution::Conflicting
    );
    assert_eq!(conflict.feature_source_id(), Some(7));
    let opaque = set(&[
        "moSimpleSurfIdRep_c,7,2,",
        "moSimpleSurfIdRep_c,7,2,opaque,",
    ]);
    assert_eq!(
        opaque.identity_resolution(),
        SurfaceIdentityResolution::OpaqueTail
    );
    assert_eq!(opaque.feature_source_id(), Some(7));
    let invalid = set(&["moSimpleSurfIdRep_c,7,2,", "moSimpleSurfIdRep_c,7,no,"]);
    assert_eq!(
        invalid.identity_resolution(),
        SurfaceIdentityResolution::InvalidReference
    );
    assert_eq!(invalid.feature_source_id(), None);
    let different = set(&["moSimpleSurfIdRep_c,7,2,", "moSimpleSurfIdRep_c,8,2,"]);
    assert_eq!(
        different.identity_resolution(),
        SurfaceIdentityResolution::Conflicting
    );
    assert_eq!(different.feature_source_id(), None);
    assert_eq!(
        set(&[]).identity_resolution(),
        SurfaceIdentityResolution::NotDecoded
    );
}
fn framed(value: &str) -> Vec<u8> {
    let units = value.encode_utf16().collect::<Vec<_>>();
    let mut bytes = vec![255, 254, 255];
    if units.len() < 254 {
        bytes.push(units.len() as u8);
    } else {
        bytes.push(254);
        bytes.extend((units.len() as u16).to_le_bytes());
    }
    bytes.extend(units.into_iter().flat_map(u16::to_le_bytes));
    bytes
}
#[test]
fn metadata_scanning_is_bounded_and_supports_extended_unicode_frames() {
    let mut bytes = vec![0; 9];
    let start = bytes.len();
    let text = format!("moSimpleSurfIdRep_c,7,2,{},", "opaque😀".repeat(70));
    bytes.extend(framed(&text));
    let end = bytes.len();
    bytes.extend(framed("moSimpleSurfIdRep_c,99,2,"));
    let found = CachedSurfaceReferences::inspect(
        &bytes,
        ByteSpan {
            offset: start,
            length: end - start,
        },
    );
    assert_eq!(found.references.len(), 1);
    assert_eq!(found.references[0].source.offset, start);
    assert_eq!(found.references[0].source.value, text);
    assert_eq!(
        found.references[0].status,
        SurfaceReferenceStatus::OpaqueTail
    );
    let short = CachedSurfaceReferences::inspect(
        &bytes,
        ByteSpan {
            offset: start,
            length: end - start - 1,
        },
    );
    assert!(short.references.is_empty());
    assert!(
        CachedSurfaceReferences::inspect(
            &bytes,
            ByteSpan {
                offset: usize::MAX,
                length: 2
            }
        )
        .references
        .is_empty()
    );
}
#[test]
fn every_truncated_frame_and_invalid_surrogates_remain_unresolved() {
    let bytes = framed("moSimpleSurfIdRep_c,7,2,-1,");
    for end in 0..bytes.len() {
        assert!(
            CachedSurfaceReferences::inspect(
                &bytes,
                ByteSpan {
                    offset: 0,
                    length: end
                }
            )
            .references
            .is_empty()
        );
    }
    let bad = [255, 254, 255, 1, 0, 216];
    assert!(
        CachedSurfaceReferences::inspect(
            &bad,
            ByteSpan {
                offset: 0,
                length: bad.len()
            }
        )
        .references
        .is_empty()
    );
}
