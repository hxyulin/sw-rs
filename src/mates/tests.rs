// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored mate streams; no proprietary CAD fixtures.
use super::*;
fn text(out: &mut Vec<u8>, value: &str) {
    let units: Vec<_> = value.encode_utf16().collect();
    out.extend([255, 254, 255]);
    if units.len() < 255 {
        out.push(units.len() as u8);
    } else {
        out.push(255);
        out.extend((units.len() as u16).to_le_bytes());
    }
    out.extend(units.into_iter().flat_map(u16::to_le_bytes));
}
fn class(out: &mut Vec<u8>, name: &str) {
    out.extend([255, 255, 1, 0]);
    out.extend((name.len() as u16).to_le_bytes());
    out.extend(name.as_bytes());
}
fn header(count: u16) -> Vec<u8> {
    [
        193_u32.to_le_bytes().as_slice(),
        count.to_le_bytes().as_slice(),
    ]
    .concat()
}
fn record(out: &mut Vec<u8>, kind: Option<&str>, name: &str, identifier: u32) -> usize {
    let start = out.len();
    if let Some(kind) = kind {
        class(out, kind);
    } else {
        out.extend(0x80c1_u16.to_le_bytes());
    }
    out.extend(0x800c_u16.to_le_bytes());
    text(out, name);
    out.extend([0, 1, 0, 0, 0, 0, 0, 0x40]);
    out.extend(identifier.to_le_bytes());
    out.extend([0; 4]);
    text(out, "note@not-an-endpoint");
    start
}
fn reference(out: &mut Vec<u8>, descriptor: &str, path: &str) {
    text(out, descriptor);
    out.extend(1_u32.to_le_bytes());
    text(out, path);
}

fn parameter(out: &mut Vec<u8>, kind: Option<&str>, value: f64) -> usize {
    if let Some(kind) = kind {
        class(out, kind);
    } else {
        out.extend(0x81a7_u16.to_le_bytes());
    }
    out.extend(0x8006_u16.to_le_bytes());
    text(out, "renamed parameter");
    out.extend([0, 0, 0, 0, 0, 0, 0, 0x40]);
    out.extend(u32::MAX.to_le_bytes());
    out.extend([0; 4]);
    text(out, "parameter note");
    out.extend([0; 2]);
    let offset = out.len();
    out.extend(value.to_le_bytes());
    offset
}

fn tolerance(out: &mut Vec<u8>, upper: f64, lower: f64) -> (usize, usize) {
    out.push(0);
    let declaration = out.len();
    class(out, "moDimTolerance_c");
    let payload = out.len();
    out.extend(3_u32.to_le_bytes());
    out.extend(0_u32.to_le_bytes());
    out.extend(upper.to_le_bytes());
    out.extend(lower.to_le_bytes());
    (declaration, payload)
}

fn reused_record(out: &mut Vec<u8>, index: u16, identifier: u32) -> usize {
    let at = record(out, None, "same editable label", identifier);
    out[at..at + 2].copy_from_slice(&(0x8000 | index).to_le_bytes());
    at
}

fn reused_parameter(out: &mut Vec<u8>, index: u16, value: f64) -> usize {
    let at = out.len();
    let scalar = parameter(out, None, value);
    out[at..at + 2].copy_from_slice(&(0x8000 | index).to_le_bytes());
    scalar
}

fn state_body(bytes: &mut Vec<u8>, suppressed: bool, alignment: u16) -> (usize, usize) {
    let base = bytes.len();
    bytes.extend([0; 55]);
    bytes[base + 12..base + 16].copy_from_slice(&u32::from(suppressed).to_le_bytes());
    bytes[base + 20..base + 24].copy_from_slice(&101_u32.to_le_bytes());
    text(bytes, "metadata😀");
    bytes.extend([0; 84]);
    let alignment_at = bytes.len();
    bytes.extend(alignment.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.push(0);
    bytes.extend(2_u32.to_le_bytes());
    bytes.extend(0_u32.to_le_bytes());
    bytes.extend(0_u16.to_le_bytes()); // First geometry pointer, not decoded here.
    (base, alignment_at)
}

fn rotation_tail(bytes: &mut Vec<u8>, locked: bool) -> usize {
    text(bytes, "");
    text(bytes, "");
    let at = bytes.len();
    bytes.extend(u32::from(locked).to_le_bytes());
    bytes.extend(u32::MAX.to_le_bytes());
    at
}

#[test]
fn saved_suppression_and_alignment_follow_frames_not_names_or_the_header_flag() {
    for header_flag in [0, 1] {
        for suppressed in [false, true] {
            for (raw, expected) in [
                (0_u16, MateAlignment::Closest),
                (1, MateAlignment::Aligned),
                (2, MateAlignment::AntiAligned),
            ] {
                let mut bytes = header(1);
                record(&mut bytes, Some("moMateCoincident"), "renamed😀", 7);
                let marker = bytes
                    .windows(8)
                    .position(|w| w == [0, 1, 0, 0, 0, 0, 0, 0x40])
                    .unwrap();
                bytes[marker + 1] = header_flag;
                let (base, alignment_at) = state_body(&mut bytes, suppressed, raw);
                let config = parse_stream(&bytes, "Contents/Config-7-MatesList", 7);
                let state = config.records[0].saved_state.as_ref().unwrap();
                assert_eq!(state.suppression.value, suppressed);
                assert_eq!(state.suppression.source_offset, base + 12);
                assert_eq!(state.feature_format_offset, base + 20);
                let alignment = state.alignment.as_ref().unwrap();
                assert_eq!(alignment.value, Some(expected));
                assert_eq!(alignment.raw_value, raw);
                assert_eq!(alignment.source_offset, alignment_at);
                assert!(state.rotation_lock.is_none());
                assert!(!config.parameters_decoded);
            }
        }
    }
}

#[test]
fn unsupported_state_fields_remain_unknown_and_do_not_cross_record_boundaries() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateCoincident"), "mate", 1);
    let (base, alignment_at) = state_body(&mut bytes, false, 1);
    for (at, value) in [(base, 1_u32), (base + 12, 2), (base + 20, 102)] {
        let mut invalid = bytes.clone();
        invalid[at..at + 4].copy_from_slice(&value.to_le_bytes());
        assert!(
            parse_stream(&invalid, "Contents/Config-0-MatesList", 0).records[0]
                .saved_state
                .is_none()
        );
    }
    let mut unknown_alignment = bytes.clone();
    unknown_alignment[alignment_at..alignment_at + 2].copy_from_slice(&3_u16.to_le_bytes());
    let config = parse_stream(&unknown_alignment, "Contents/Config-0-MatesList", 0);
    let alignment = config.records[0]
        .saved_state
        .as_ref()
        .unwrap()
        .alignment
        .as_ref()
        .unwrap();
    assert!(alignment.value.is_none());
    assert_eq!(alignment.raw_value, 3);
    for (at, replacement) in [
        (alignment_at + 2, 3_u16.to_le_bytes().to_vec()),
        (alignment_at + 4, vec![1]),
        (alignment_at + 5, 0_u32.to_le_bytes().to_vec()),
        (alignment_at + 5, 5_u32.to_le_bytes().to_vec()),
        (alignment_at + 9, 1_u32.to_le_bytes().to_vec()),
    ] {
        let mut invalid = bytes.clone();
        invalid[at..at + replacement.len()].copy_from_slice(&replacement);
        let config = parse_stream(&invalid, "Contents/Config-0-MatesList", 0);
        assert!(
            config.records[0]
                .saved_state
                .as_ref()
                .unwrap()
                .alignment
                .is_none()
        );
    }
    bytes.truncate(base + 16);
    bytes[4..6].copy_from_slice(&2_u16.to_le_bytes());
    record(&mut bytes, Some("moMateConcentric"), "next", 2);
    state_body(&mut bytes, true, 2);
    let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert_eq!(config.records.len(), 2);
    assert!(config.records[0].saved_state.is_none());
    assert!(
        config.records[1]
            .saved_state
            .as_ref()
            .unwrap()
            .suppression
            .value
    );
    bytes[4..6].copy_from_slice(&3_u16.to_le_bytes());
    assert!(
        parse_stream(&bytes, "Contents/Config-0-MatesList", 0)
            .records
            .iter()
            .all(|r| r.saved_state.is_none())
    );
}

#[test]
fn concentric_rotation_locks_use_a_checked_class_tail_for_each_record() {
    let mut bytes = header(2);
    record(&mut bytes, Some("moMateConcentric"), "first", 1);
    state_body(&mut bytes, false, 1);
    let first_at = rotation_tail(&mut bytes, false);
    reused_record(&mut bytes, 193, 2);
    state_body(&mut bytes, false, 2);
    let second_at = rotation_tail(&mut bytes, true);
    let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    for (r, expected, offset) in [
        (&config.records[0], false, first_at),
        (&config.records[1], true, second_at),
    ] {
        let lock = r
            .saved_state
            .as_ref()
            .unwrap()
            .rotation_lock
            .as_ref()
            .unwrap();
        assert_eq!(lock.value, expected);
        assert_eq!(lock.source_offset, offset);
    }
    for (at, replacement) in [
        (first_at - 8, vec![0]),
        (first_at, 2_u32.to_le_bytes().to_vec()),
        (first_at + 4, 0_u32.to_le_bytes().to_vec()),
    ] {
        let mut invalid = bytes.clone();
        invalid[at..at + replacement.len()].copy_from_slice(&replacement);
        let config = parse_stream(&invalid, "Contents/Config-0-MatesList", 0);
        assert!(
            config.records[0]
                .saved_state
                .as_ref()
                .unwrap()
                .rotation_lock
                .is_none()
        );
        assert!(
            config.records[1]
                .saved_state
                .as_ref()
                .unwrap()
                .rotation_lock
                .as_ref()
                .unwrap()
                .value
        );
    }
    for end in second_at - 8..bytes.len() {
        let config = parse_stream(&bytes[..end], "Contents/Config-0-MatesList", 0);
        assert!(
            config.records[1]
                .saved_state
                .as_ref()
                .unwrap()
                .rotation_lock
                .is_none()
        );
    }
    let mut bytes = header(1);
    record(
        &mut bytes,
        Some("moMateCoincident"),
        "same tail, different class",
        1,
    );
    state_body(&mut bytes, false, 1);
    rotation_tail(&mut bytes, true);
    let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert!(
        config.records[0]
            .saved_state
            .as_ref()
            .unwrap()
            .rotation_lock
            .is_none()
    );
}

#[test]
fn future_mate_classes_do_not_inherit_alignment_or_concentric_lock_semantics() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateFutureUnknown"), "Concentric1", 1);
    state_body(&mut bytes, false, 1);
    rotation_tail(&mut bytes, false);
    let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    let state = config.records[0].saved_state.as_ref().unwrap();
    assert!(!state.suppression.value);
    assert!(state.alignment.is_none());
    assert!(state.rotation_lock.is_none());
}

#[test]
fn saved_state_is_scoped_to_the_literal_stream_not_a_shared_feature_id_or_label() {
    let mut streams = BTreeMap::new();
    for (index, suppressed) in [(0, false), (9, true)] {
        let mut bytes = header(2);
        record(&mut bytes, Some("moMateCoincident"), "same name", 30);
        state_body(&mut bytes, suppressed, 1);
        reused_record(&mut bytes, 193, 31);
        state_body(&mut bytes, !suppressed, 2);
        streams.insert(format!("Contents/Config-{index}-MatesList"), bytes);
    }
    let data = parse(&streams);
    assert_eq!(data.configurations.len(), 2);
    for (config, index, expected) in [
        (&data.configurations[0], 0, false),
        (&data.configurations[1], 9, true),
    ] {
        assert_eq!(config.configuration_index, index);
        assert_eq!(
            config.source_stream,
            format!("Contents/Config-{index}-MatesList")
        );
        assert_eq!(config.records[0].saved_identifier, 30);
        assert_eq!(config.records[0].name.value, "same name");
        assert_eq!(
            config.records[0]
                .saved_state
                .as_ref()
                .unwrap()
                .suppression
                .value,
            expected
        );
        assert_eq!(
            config.records[1]
                .saved_state
                .as_ref()
                .unwrap()
                .suppression
                .value,
            !expected
        );
    }
}

#[test]
fn ordered_class_reuse_decodes_mixed_mates_and_parameter_units_with_shared_proofs() {
    let mut bytes = header(6);
    record(
        &mut bytes,
        Some("moMateDistanceDim_c"),
        "same editable label",
        1,
    );
    parameter(&mut bytes, Some("moLengthParameter_c"), 0.01);
    let angle_at = record(
        &mut bytes,
        Some("moMatePlanarAngleDim_c"),
        "same editable label",
        2,
    );
    parameter(&mut bytes, Some("moAngleParameter_c"), 1.);
    let coincident_at = record(
        &mut bytes,
        Some("moMateCoincident"),
        "same editable label",
        3,
    );
    reused_record(&mut bytes, 193, 4);
    let length_value = reused_parameter(&mut bytes, 210, 0.025);
    let angle_reuse = reused_record(&mut bytes, 220, 5);
    reused_parameter(&mut bytes, 260, 0.5);
    reused_record(&mut bytes, 280, 6);
    let config = parse_stream(&bytes, "Contents/Config-7-MatesList", 7);
    assert_eq!(
        config.records.iter().map(|r| r.kind).collect::<Vec<_>>(),
        [
            Some(MateKind::Distance),
            Some(MateKind::Angle),
            Some(MateKind::Coincident),
            Some(MateKind::Distance),
            Some(MateKind::Angle),
            Some(MateKind::Coincident),
        ]
    );
    assert_eq!(config.class_order_evidence.len(), 2);
    let resolution = config.records[4]
        .class_reference_resolution
        .as_ref()
        .unwrap();
    assert!(config.records[4].class_name.is_none());
    assert_eq!(resolution.class_index, 220);
    assert_eq!(resolution.declaration_offset, angle_at);
    assert_eq!(resolution.index_source_offset, angle_reuse);
    assert_eq!(resolution.declaration_order_evidence, Some(0));
    let proof = &config.class_order_evidence[0];
    assert_eq!(proof.prefix_family, "mates");
    assert_eq!(proof.declarations[2].declaration_offset, coincident_at);
    assert_eq!(proof.index_uses[0].class_index, 193);
    assert_eq!(proof.index_uses[0].source_offset, 0);
    let length = &config.records[3].parameter_candidates[0];
    assert_eq!(length.unit, Some("metres"));
    assert_eq!(length.value, 0.025);
    assert_eq!(length.value_offset, length_value);
    assert!(length.class_name.is_none());
    assert_eq!(
        length
            .class_reference_resolution
            .as_ref()
            .unwrap()
            .declaration_order_evidence,
        Some(1)
    );
    assert_eq!(
        config.records[4].parameter_candidates[0].unit,
        Some("radians")
    );
    assert_eq!(config.records_with_configured_limits, 0);
    assert!(!config.parameters_decoded);
}

#[test]
fn ordered_reuse_retains_ambiguity_unused_classes_and_inherited_tags() {
    let mut bytes = header(4);
    record(&mut bytes, Some("moMateCoincident"), "root", 1);
    record(&mut bytes, Some("moMateParallel"), "unused or reused", 2);
    record(
        &mut bytes,
        Some("moMateDistanceDim_c"),
        "unused or reused",
        3,
    );
    reused_record(&mut bytes, 250, 4);
    let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert!(config.records[3].class_reference_resolution.is_none());
    assert!(config.records[3].kind.is_none());
    // Earlier use before the later declaration makes the same class tag unique.
    let mut bytes = header(5);
    record(&mut bytes, Some("moMateCoincident"), "root", 1);
    record(&mut bytes, Some("moMateParallel"), "not a type label", 2);
    reused_record(&mut bytes, 250, 3);
    record(&mut bytes, Some("moMateDistanceDim_c"), "unused", 4);
    reused_record(&mut bytes, 180, 5); // An inherited class never joins the local map.
    let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert_eq!(config.records[2].kind, Some(MateKind::Parallel));
    assert!(config.records[4].kind.is_none());
    // Contradictory numbers of local tags invalidate the order proof.
    bytes[4..6].copy_from_slice(&6_u16.to_le_bytes());
    reused_record(&mut bytes, 251, 6);
    bytes[4..6].copy_from_slice(&7_u16.to_le_bytes());
    reused_record(&mut bytes, 252, 7);
    let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert!(config.records[2].kind.is_none());
    assert!(config.class_order_evidence.is_empty());
}

#[test]
fn parameter_order_inventory_keeps_nonfinite_declarations_and_rejects_unknown_layouts() {
    for class in ["moAngleParameter_c", "moFutureParameter_c"] {
        let mut bytes = header(1);
        record(&mut bytes, Some("moMateDistanceDim_c"), "mate", 1);
        parameter(&mut bytes, Some("moLengthParameter_c"), 0.01);
        parameter(&mut bytes, Some(class), f64::NAN);
        reused_parameter(&mut bytes, 250, 1.);
        let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
        let reused = config.records[0].parameter_candidates.last().unwrap();
        assert!(
            reused.unit.is_none(),
            "ambiguous or unknown declaration {class}"
        );
        assert!(reused.class_reference_resolution.is_none());
    }
}

#[test]
fn conflicting_mate_and_parameter_orders_do_not_create_local_class_identities() {
    let mut bytes = header(4);
    record(&mut bytes, Some("moMateCoincident"), "root", 1);
    parameter(&mut bytes, Some("moLengthParameter_c"), 0.01);
    record(&mut bytes, Some("moMateParallel"), "later", 2);
    reused_record(&mut bytes, 220, 3);
    // This parameter was declared BEFORE the parallel class, so its proven
    // index cannot follow the parallel class index in the same archive map.
    reused_parameter(&mut bytes, 250, 0.02);
    reused_record(&mut bytes, 193, 4);
    let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert!(config.records[2].kind.is_none());
    assert!(config.records[2].parameter_candidates[0].unit.is_none());
    assert_eq!(config.records[3].kind, Some(MateKind::Coincident));
    assert!(config.class_order_evidence.is_empty());
    assert!(
        config
            .diagnostics
            .iter()
            .any(|d| d.code == "class_order_conflict")
    );
}

#[test]
fn ordered_matches_equal_the_intersection_of_all_valid_injections() {
    fn assignments(
        n: usize,
        deadlines: &[usize],
        chosen: &mut Vec<usize>,
        all: &mut Vec<Vec<usize>>,
    ) {
        if chosen.len() == deadlines.len() {
            all.push(chosen.clone());
            return;
        }
        let first = chosen.last().map_or(0, |last| last + 1);
        for index in first..n.min(deadlines[chosen.len()]) {
            chosen.push(index);
            assignments(n, deadlines, chosen, all);
            chosen.pop();
        }
    }
    for n in 1_usize..=4 {
        for count in 1..=n + 1 {
            for encoded in 0..(n + 1).pow(count as u32) {
                let mut remainder = encoded;
                let deadlines: Vec<_> = (0..count)
                    .map(|_| {
                        let d = remainder % (n + 1);
                        remainder /= n + 1;
                        d
                    })
                    .collect();
                let proof = MateClassOrderEvidence {
                    prefix_family: "parameters",
                    leading_class_index: 193,
                    declarations: (0..n)
                        .map(|i| MateClassDeclaration {
                            class_name: format!("Class{i}"),
                            declaration_offset: (i + 1) * 10,
                        })
                        .collect(),
                    index_uses: deadlines
                        .iter()
                        .enumerate()
                        .map(|(i, d)| MateClassIndexUse {
                            class_index: 200 + i as u32,
                            source_offset: d * 10 + 1,
                        })
                        .collect(),
                };
                let mut all = Vec::new();
                assignments(n, &deadlines, &mut Vec::new(), &mut all);
                let actual = ordered_class_matches(&proof);
                if let Some(first) = all.first() {
                    let expected: BTreeMap<_, _> = first
                        .iter()
                        .enumerate()
                        .filter_map(|(i, d)| {
                            all.iter()
                                .all(|a| a[i] == *d)
                                .then_some((200 + i as u32, *d))
                        })
                        .collect();
                    assert_eq!(actual, Some(expected), "n={n}, deadlines={deadlines:?}");
                } else {
                    assert!(actual.is_none(), "n={n}, deadlines={deadlines:?}");
                }
            }
        }
    }
}

#[test]
fn configured_distance_bounds_decode_independent_deviations_with_source_offsets() {
    for (upper, lower, minimum, maximum) in [
        (0.05, -0.05, 0., 0.1),
        (0.05, -0.04, 0.01, 0.1),
        (0.04, -0.05, 0., 0.09),
    ] {
        let mut bytes = header(1);
        record(
            &mut bytes,
            Some("moMateLimitDistanceDim_c"),
            "any editable name",
            1,
        );
        let value_at = parameter(&mut bytes, Some("moLengthParameter_c"), 0.05);
        let (declaration, payload) = tolerance(&mut bytes, upper, lower);
        let config = parse_stream(&bytes, "Contents/Config-7-MatesList", 7);
        assert_eq!(config.records_with_configured_limits, 1);
        assert!(!config.limit_bounds_decoded); // Complete state/enable semantics remain unknown.
        let bounds = config.records[0].configured_limits.as_ref().unwrap();
        assert_eq!(bounds.unit, MateLimitUnit::Metres);
        assert!((bounds.minimum - minimum).abs() < 1e-15);
        assert!((bounds.maximum - maximum).abs() < 1e-15);
        assert_eq!(bounds.dimension_value, 0.05);
        assert_eq!(bounds.lower_deviation, lower);
        assert_eq!(bounds.upper_deviation, upper);
        assert_eq!(bounds.dimension_value_offset, value_at);
        assert_eq!(bounds.tolerance_declaration_offset, declaration);
        assert_eq!(bounds.upper_deviation_offset, payload + 8);
        assert_eq!(bounds.lower_deviation_offset, payload + 16);
        for end in declaration..bytes.len() {
            assert!(
                parse_stream(&bytes[..end], "Contents/Config-7-MatesList", 7).records[0]
                    .configured_limits
                    .is_none()
            );
        }
    }
}

#[test]
fn configured_angle_bounds_decode_independently_changed_limits_in_radians() {
    use std::f64::consts::{FRAC_PI_2, PI};
    for (minimum, maximum) in [(0., PI), (PI / 18., PI), (0., PI * 17. / 18.)] {
        let mut bytes = header(1);
        record(
            &mut bytes,
            Some("moMateLimitPlanarAngleDim_c"),
            "renamed",
            1,
        );
        let value_at = parameter(&mut bytes, Some("moAngleParameter_c"), FRAC_PI_2);
        let (declaration, payload) =
            tolerance(&mut bytes, maximum - FRAC_PI_2, minimum - FRAC_PI_2);
        let config = parse_stream(&bytes, "Contents/Config-2-MatesList", 2);
        assert_eq!(config.records_with_configured_limits, 1);
        let bounds = config.records[0].configured_limits.as_ref().unwrap();
        assert_eq!(bounds.unit, MateLimitUnit::Radians);
        assert!((bounds.minimum - minimum).abs() < 1e-15);
        assert!((bounds.maximum - maximum).abs() < 1e-15);
        assert_eq!(bounds.dimension_value, FRAC_PI_2);
        assert_eq!(bounds.dimension_value_offset, value_at);
        assert_eq!(bounds.tolerance_declaration_offset, declaration);
        assert_eq!(bounds.upper_deviation_offset, payload + 8);
        assert_eq!(bounds.lower_deviation_offset, payload + 16);
        assert!(!config.limit_bounds_decoded);
    }
}

#[test]
fn configured_bounds_do_not_promote_plain_dimensions_unknown_classes_or_other_units() {
    for (kind, parameter_class) in [
        (Some("moMateDistanceDim_c"), Some("moLengthParameter_c")),
        (Some("moMatePlanarAngleDim_c"), Some("moAngleParameter_c")),
        (
            Some("moMateLimitPlanarAngleDim_c"),
            Some("moLengthParameter_c"),
        ),
        (Some("moMateLimitPlanarAngleDim_c"), None),
        (Some("moMateLimitDistanceDim_c"), Some("moAngleParameter_c")),
        (Some("moMateLimitDistanceDim_c"), None),
        (None, Some("moLengthParameter_c")),
    ] {
        let mut bytes = header(1);
        record(&mut bytes, kind, "LimitDistance1", 1);
        parameter(&mut bytes, parameter_class, 0.05);
        tolerance(&mut bytes, 0.05, -0.05);
        let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
        assert!(config.records[0].configured_limits.is_none());
        assert_eq!(config.records_with_configured_limits, 0);
    }
    let mut bytes = header(1);
    record(
        &mut bytes,
        Some("moMateLimitDistanceDim_c"),
        "same label",
        1,
    );
    parameter(&mut bytes, Some("moLengthParameter_c"), 0.05);
    tolerance(&mut bytes, 0.05, -0.05);
    parameter(&mut bytes, Some("moLengthParameter_c"), 0.06);
    assert!(
        parse_stream(&bytes, "Contents/Config-0-MatesList", 0).records[0]
            .configured_limits
            .is_none()
    );
}

#[test]
fn configured_bounds_reject_invalid_modes_scalars_and_cross_record_reads() {
    let mut original = header(1);
    record(&mut original, Some("moMateLimitDistanceDim_c"), "mate", 1);
    let value = parameter(&mut original, Some("moLengthParameter_c"), 0.05);
    let (declaration, payload) = tolerance(&mut original, 0.05, -0.05);
    let mut mismatched_inventory = original.clone();
    mismatched_inventory[4..6].copy_from_slice(&2_u16.to_le_bytes());
    let config = parse_stream(&mismatched_inventory, "Contents/Config-0-MatesList", 0);
    assert!(!config.inventory_matches_declared_count);
    assert!(config.records[0].configured_limits.is_none());
    assert_eq!(config.records_with_configured_limits, 0);
    for (at, bytes) in [
        (payload, 2_u32.to_le_bytes().to_vec()),
        (payload + 4, 1_u32.to_le_bytes().to_vec()),
        (declaration, [255, 255, 2, 0].to_vec()),
        (value + 8, vec![1]),
        (payload + 8, f64::NAN.to_le_bytes().to_vec()),
        (payload + 16, f64::NEG_INFINITY.to_le_bytes().to_vec()),
        (payload + 8, (-0.01_f64).to_le_bytes().to_vec()),
        (payload + 16, 0.01_f64.to_le_bytes().to_vec()),
        (payload + 16, (-0.06_f64).to_le_bytes().to_vec()),
    ] {
        let mut invalid = original.clone();
        invalid[at..at + bytes.len()].copy_from_slice(&bytes);
        assert!(
            parse_stream(&invalid, "Contents/Config-0-MatesList", 0).records[0]
                .configured_limits
                .is_none()
        );
    }
    let mut overflow = original.clone();
    overflow[value..value + 8].copy_from_slice(&f64::MAX.to_le_bytes());
    overflow[payload + 8..payload + 16].copy_from_slice(&f64::MAX.to_le_bytes());
    assert!(
        parse_stream(&overflow, "Contents/Config-0-MatesList", 0).records[0]
            .configured_limits
            .is_none()
    );
    original.truncate(payload + 16); // Missing lower deviation, followed by another record.
    original[4..6].copy_from_slice(&2_u16.to_le_bytes());
    record(&mut original, Some("moMateCoincident"), "next mate", 2);
    let config = parse_stream(&original, "Contents/Config-0-MatesList", 0);
    assert_eq!(config.records.len(), 2);
    assert!(config.records[0].configured_limits.is_none());
    assert!(config.records[1].configured_limits.is_none());
}

#[test]
fn numerical_parameters_keep_names_units_offsets_and_record_scope() {
    let mut bytes = header(3);
    record(&mut bytes, Some("moMateDistanceDim_c"), "arbitrary name", 1);
    let length_at = parameter(&mut bytes, Some("moLengthParameter_c"), 0.01);
    record(
        &mut bytes,
        Some("moMatePlanarAngleDim_c"),
        "not localized",
        2,
    );
    let angle_at = parameter(
        &mut bytes,
        Some("moAngleParameter_c"),
        std::f64::consts::FRAC_PI_6,
    );
    record(&mut bytes, Some("moMateLimitDistanceDim_c"), "limit", 3);
    parameter(&mut bytes, None, -0.02);
    let config = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert_eq!(config.records.len(), 3);
    assert!(config.inventory_matches_declared_count);
    assert!(!config.parameters_decoded);
    assert!(!config.limit_bounds_decoded);
    let length = &config.records[0].parameter_candidates;
    assert_eq!(length.len(), 1);
    assert_eq!(length[0].value, 0.01);
    assert_eq!(length[0].value_offset, length_at);
    assert_eq!(length[0].unit, Some("metres"));
    assert_eq!(length[0].name.value, "renamed parameter");
    let angle = &config.records[1].parameter_candidates;
    assert_eq!(angle.len(), 1);
    assert_eq!(angle[0].value, std::f64::consts::FRAC_PI_6);
    assert_eq!(angle[0].value_offset, angle_at);
    assert_eq!(angle[0].unit, Some("radians"));
    let unknown = &config.records[2].parameter_candidates;
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].value, -0.02);
    assert_eq!(unknown[0].class_reference, Some(0x81a7));
    assert!(unknown[0].unit.is_none()); // A limit mate does not identify this scalar's role.
}

#[test]
fn numerical_parameters_reject_nonfinite_unknown_declarations_and_truncated_scalars() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut bytes = header(1);
        record(&mut bytes, Some("moMateDistanceDim_c"), "mate", 1);
        parameter(&mut bytes, Some("moLengthParameter_c"), value);
        assert!(
            parse_stream(&bytes, "Contents/Config-0-MatesList", 0).records[0]
                .parameter_candidates
                .is_empty()
        );
    }
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateDistanceDim_c"), "mate", 1);
    parameter(&mut bytes, Some("moFutureParameter_c"), 3.);
    assert!(
        parse_stream(&bytes, "Contents/Config-0-MatesList", 0).records[0]
            .parameter_candidates
            .is_empty()
    );
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateDistanceDim_c"), "mate", 1);
    let value_at = parameter(&mut bytes, Some("moLengthParameter_c"), 0.02);
    for end in value_at..bytes.len() {
        assert!(
            parse_stream(&bytes[..end], "Contents/Config-0-MatesList", 0).records[0]
                .parameter_candidates
                .is_empty()
        );
    }
    // A damaged marker must not be accepted as the known parameter layout.
    let marker = value_at - 2 - (4 + "parameter note".len() * 2) - 16;
    bytes[marker] = 1;
    assert!(
        parse_stream(&bytes, "Contents/Config-0-MatesList", 0).records[0]
            .parameter_candidates
            .is_empty()
    );
}

#[test]
fn multiline_selection_descriptors_keep_all_rows_without_choosing_an_endpoint() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateCoincident"), "edited name", 27);
    let descriptor = "moSimpleSurfIdRep_c,7,2, \nmoFromSktEntSurfIdRep_c,8,4, \n";
    reference(&mut bytes, descriptor, "Part-2@Root");
    let data = parse_stream(&bytes, "Contents/Config-7-MatesList", 7);
    let selection = &data.records[0].reference_candidates[0];
    assert_eq!(
        selection.geometry_descriptor.as_ref().unwrap().value,
        descriptor
    );
    assert!(selection.surface_reference().is_none());
    let surfaces = selection.surface_references();
    assert_eq!(surfaces.len(), 2);
    assert_eq!(surfaces[0].feature_source_id, Some(7));
    assert_eq!(surfaces[1].feature_source_id, Some(8));
    let doc = crate::metadata_from_container(
        crate::Container {
            streams: BTreeMap::from([("Contents/Config-7-MatesList".into(), bytes)]),
            diagnostics: vec![],
        },
        "part.SLDASM",
    );
    let graph =
        crate::ReferenceGraph::build("hash", &crate::FeatureCatalog::parse(None), &doc, &[]);
    let link = graph
        .links
        .iter()
        .find(|l| l.relation == crate::ReferenceRelation::MateSelection)
        .unwrap();
    assert!(link.selection_surface_reference.is_none());
    assert_eq!(link.selection_surface_references.len(), 2);
    assert_eq!(
        link.selection_descriptor.as_ref().unwrap().value,
        descriptor
    );
    assert!(link.candidates.is_empty());
    assert_eq!(link.resolution, crate::ReferenceResolution::Unresolved);
    assert!(!graph.topology_ownership_decoded);
}
#[test]
fn names_types_and_reference_graph_keep_occurrences_and_hyperedges_distinct() {
    let mut bytes = header(2);
    record(
        &mut bytes,
        Some("moMateWidth"),
        "not a localized type name",
        27,
    );
    for path in ["A-1@Root", "A-2@Root", "B-1@Root/Leaf-1@B", "B-2@Root"] {
        reference(&mut bytes, "moSimpleSurfIdRep_c,70,227,", path);
    }
    reference(&mut bytes, "moSimpleSurfIdRep_c,70,227,", "A-1@Root");
    let next = record(&mut bytes, None, "renamed mate", 31);
    reference(&mut bytes, "moSimpleSurfIdRep_c,70,227,", "A-1@Root");
    let data = parse_stream(&bytes, "Contents/Config-7-MatesList", 7);
    assert!(data.inventory_matches_declared_count);
    assert_eq!(data.configuration_index, 7);
    assert_eq!(data.records[0].kind, Some(MateKind::Width));
    assert_eq!(data.records[0].name.value, "not a localized type name");
    assert_eq!(data.records[0].evidence_end, next);
    assert_eq!(data.records[0].reference_candidates.len(), 5);
    assert!(
        data.records[0].reference_candidates[0]
            .geometry_descriptor
            .is_some()
    );
    assert_eq!(data.records[1].class_reference, Some(0x80c1));
    assert_eq!(data.records[1].kind, Some(MateKind::Width));
    assert!(data.records[1].class_name.is_none());
    let resolution = data.records[1].class_reference_resolution.as_ref().unwrap();
    assert_eq!(resolution.class_name, "moMateWidth");
    assert_eq!(resolution.class_index, 193);
    assert_eq!(resolution.index_source_offset, 0);
    assert_eq!(resolution.declaration_offset, 6);
    assert_eq!(resolution.reference_offset, next);
    let graph = data.reference_graph();
    assert_eq!(graph.len(), 4);
    assert_eq!(graph[0].reference, "A-1@Root");
    assert_eq!(graph[0].record_indices, [0, 1]);
    assert!(!data.entity_ownership_decoded);
    assert!(!data.parameters_decoded);
}

#[test]
fn leading_class_anchor_is_configuration_local_and_does_not_resolve_other_slots() {
    let mut streams = BTreeMap::new();
    for (index, class_name) in [(2, "moMateCoincident"), (7, "moMateConcentric")] {
        let mut bytes = header(3);
        record(&mut bytes, Some(class_name), "same editable name", 1);
        record(&mut bytes, None, "unrelated label", 2);
        let other = record(&mut bytes, None, "same editable name", 3);
        bytes[other..other + 2].copy_from_slice(&0x80c2_u16.to_le_bytes());
        streams.insert(format!("Contents/Config-{index}-MatesList"), bytes);
    }
    let data = parse(&streams);
    assert_eq!(
        data.configurations[0].records[1].kind,
        Some(MateKind::Coincident)
    );
    assert_eq!(
        data.configurations[1].records[1].kind,
        Some(MateKind::Concentric)
    );
    for config in data.configurations {
        assert!(config.records[0].class_reference_resolution.is_none());
        assert!(config.records[2].kind.is_none());
        assert!(config.records[2].class_reference_resolution.is_none());
        assert!(!config.entity_ownership_decoded);
        assert!(!config.limit_bounds_decoded);
    }
}

#[test]
fn leading_class_anchor_requires_supported_index_first_declaration_and_complete_inventory() {
    let mut original = header(2);
    record(
        &mut original,
        Some("moMateCoincident"),
        "does not imply a type",
        1,
    );
    record(&mut original, None, "Concentric1", 2);
    for index in [0_u32, 12, 0x7fff, 0x1_00c1] {
        let mut bytes = original.clone();
        bytes[..4].copy_from_slice(&index.to_le_bytes());
        let parsed = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
        assert!(parsed.records[1].class_reference_resolution.is_none());
        assert!(parsed.records[1].kind.is_none());
    }
    let mut mismatch = original.clone();
    mismatch[4..6].copy_from_slice(&3_u16.to_le_bytes());
    let parsed = parse_stream(&mismatch, "Contents/Config-0-MatesList", 0);
    assert!(parsed.records[1].kind.is_none());
    let mut displaced = original.clone();
    displaced.splice(6..6, [0, 0]);
    let parsed = parse_stream(&displaced, "Contents/Config-0-MatesList", 0);
    assert!(parsed.records[1].kind.is_none());
    let mut undeclared = header(2);
    record(&mut undeclared, None, "same label", 1);
    record(&mut undeclared, None, "same label", 2);
    assert!(
        parse_stream(&undeclared, "Contents/Config-0-MatesList", 0)
            .records
            .iter()
            .all(|r| r.kind.is_none())
    );
    let mut future = header(2);
    record(&mut future, Some("moMateFutureUnknown"), "any name", 1);
    record(&mut future, None, "Distance1", 2);
    let parsed = parse_stream(&future, "Contents/Config-0-MatesList", 0);
    assert!(parsed.records[1].kind.is_none());
    assert_eq!(
        parsed.records[1]
            .class_reference_resolution
            .as_ref()
            .unwrap()
            .class_name,
        "moMateFutureUnknown"
    );
}
#[test]
fn nested_dimension_parameters_are_not_extra_mates_and_types_are_not_guessed() {
    let mut bytes = header(3);
    record(&mut bytes, Some("moMateDistanceDim_c"), "same name", 91);
    record(&mut bytes, Some("moLengthParameter_c"), "D1", u32::MAX);
    record(&mut bytes, Some("moMateFutureUnknown"), "same name", 92);
    record(&mut bytes, Some("moMateCamTangent"), "same name", 93);
    let data = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert!(data.inventory_matches_declared_count);
    assert_eq!(data.records.len(), 3);
    assert_eq!(data.records[0].kind, Some(MateKind::Distance));
    assert_eq!(
        data.records[1].class_name.as_deref(),
        Some("moMateFutureUnknown")
    );
    assert_eq!(data.records[1].kind, None);
    assert_eq!(data.records[2].kind, Some(MateKind::CamTangent));
    assert_ne!(data.records[0].source_offset, data.records[1].source_offset);
}
#[test]
fn stream_scope_empty_missing_and_unsupported_are_distinct() {
    let data = parse(&BTreeMap::from([
        ("Contents/Config-0-MatesList".into(), header(0)),
        ("Contents/Config-9-MatesList".into(), vec![0; 3]),
        ("Contents/Config-10-MatesList".into(), header(u16::MAX)),
        ("Contents/Config-bad-MatesList".into(), header(0)),
        ("Contents/Config-11-MatesList-extra".into(), header(0)),
    ]));
    assert_eq!(data.configurations.len(), 3);
    let empty = data
        .configurations
        .iter()
        .find(|c| c.configuration_index == 0)
        .unwrap();
    assert_eq!(empty.declared_count, Some(0));
    assert!(empty.inventory_matches_declared_count);
    for unsupported in data
        .configurations
        .iter()
        .filter(|c| c.configuration_index != 0)
    {
        assert!(!unsupported.inventory_matches_declared_count);
        assert_eq!(unsupported.diagnostics[0].code, "unsupported_mate_header");
    }
    assert!(parse(&BTreeMap::new()).configurations.is_empty());
}
#[test]
fn truncation_and_oversized_strings_remain_partial_without_panics() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateConcentric"), "同心_名称", 42);
    reference(&mut bytes, "moSimpleSurfIdRep_c,1,2,", "A-1@Root");
    for end in 0..bytes.len() {
        let data = parse_stream(&bytes[..end], "Contents/Config-0-MatesList", 0);
        assert!(!data.entity_ownership_decoded);
    }
    let mut malformed = header(1);
    malformed.extend([255, 254, 255, 255]);
    malformed.extend(u32::MAX.to_le_bytes());
    let data = parse_stream(&malformed, "Contents/Config-0-MatesList", 0);
    assert!(!data.inventory_matches_declared_count);
    assert!(
        data.diagnostics
            .iter()
            .any(|d| d.code == "mate_count_mismatch")
    );
}
#[test]
fn long_unicode_references_round_trip_and_notes_and_file_paths_are_excluded() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateLock_c"), "锁定", 42);
    let value = format!("{}-1@Root", "零件".repeat(150));
    reference(&mut bytes, "moSimpleSurfIdRep_c,1,2,", &value);
    text(&mut bytes, "C:\\email@example.com\\part.SLDPRT");
    let data = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert_eq!(data.records[0].reference_candidates.len(), 1);
    let reference = &data.records[0].reference_candidates[0].reference;
    assert_eq!(reference.value, value);
    assert_eq!(
        native_string_at(&bytes, reference.offset).unwrap().0.value,
        value
    );
    assert_eq!(
        serde_json::to_value(&data).unwrap()["records"][0]["kind"],
        "lock"
    );
}

#[test]
fn metadata_exposes_native_configuration_indices_and_mate_coverage() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateConcentric"), "Concentric1", 42);
    let doc = crate::metadata_from_container(
        crate::Container {
            streams: BTreeMap::from([
                ("Contents/Config-7-MatesList".into(), bytes),
                ("Contents/Config-2-MatesList".into(), header(0)),
            ]),
            diagnostics: Vec::new(),
        },
        "test.SLDASM",
    );
    let json = serde_json::to_value(doc).unwrap();
    assert_eq!(json["configurations"][0]["index"], 2);
    assert_eq!(json["configurations"][1]["index"], 7);
    assert_eq!(json["mates"]["configurations"][1]["configuration_index"], 7);
    assert_eq!(
        json["mates"]["configurations"][1]["records"][0]["kind"],
        "concentric"
    );
    assert_eq!(
        json["mates"]["configurations"][1]["entity_ownership_decoded"],
        false
    );
}

#[test]
fn cached_planes_keep_assembly_frame_values_without_assigning_endpoints() {
    let mut bytes = header(2);
    record(
        &mut bytes,
        Some("moMateCoincident"),
        "edited plane mate",
        41,
    );
    class(&mut bytes, "moAbsoluteRefPlnData_c");
    let parameters_offset = bytes.len();
    for value in [0.01_f64, -0.02, 0.03, 0.0, -1.0, 0.0, 0.0, 0.0] {
        bytes.extend(value.to_le_bytes());
    }
    // A reused tag is not enough to identify another plane object's class.
    bytes.extend(0x80c7_u16.to_le_bytes());
    bytes.extend([0; 64]);
    record(&mut bytes, Some("moMateCoincident"), "same name", 42);
    let data = parse_stream(&bytes, "Contents/Config-9-MatesList", 9);
    assert!(data.inventory_matches_declared_count);
    let planes = &data.records[0].cached_plane_geometry;
    assert_eq!(planes.len(), 1);
    assert_eq!(planes[0].point_metres, [0.01, -0.02, 0.03]);
    assert_eq!(planes[0].normal, [0.0, -1.0, 0.0]);
    assert_eq!(planes[0].parameters_offset, parameters_offset);
    assert!(data.records[1].cached_plane_geometry.is_empty());
    assert!(!data.entity_ownership_decoded);
    assert!(data.records[0].reference_candidates.is_empty());

    bytes[4..6].copy_from_slice(&3_u16.to_le_bytes());
    let incomplete = parse_stream(&bytes, "Contents/Config-9-MatesList", 9);
    assert!(
        incomplete
            .records
            .iter()
            .all(|r| r.cached_plane_geometry.is_empty())
    );
}

#[test]
fn cached_planes_reject_unknown_truncated_and_nonplanar_prefixes() {
    let make = |class_name: &str, values: [f64; 8]| {
        let mut bytes = header(1);
        record(&mut bytes, Some("moMateCoincident"), "plane mate", 41);
        let declaration_offset = bytes.len();
        class(&mut bytes, class_name);
        let parameters_offset = bytes.len();
        for value in values {
            bytes.extend(value.to_le_bytes());
        }
        (bytes, declaration_offset, parameters_offset)
    };
    let valid = [0.01, 0.02, 0.03, 0.0, 0.0, 1.0, 0.0, 0.0];
    let (bytes, declaration_offset, parameters_offset) = make("moAbsoluteRefPlnData_c", valid);
    for end in parameters_offset..bytes.len() {
        let data = parse_stream(&bytes[..end], "Contents/Config-0-MatesList", 0);
        assert!(
            data.records[0].cached_plane_geometry.is_empty(),
            "truncation at {end}"
        );
    }
    for (index, value) in [
        (0, f64::NAN),
        (3, f64::INFINITY),
        (5, 0.0),
        (5, 2.0),
        (6, 1.0),
        (7, f64::NEG_INFINITY),
    ] {
        let mut values = valid;
        values[index] = value;
        let (bytes, _, _) = make("moAbsoluteRefPlnData_c", values);
        assert!(
            parse_stream(&bytes, "Contents/Config-0-MatesList", 0).records[0]
                .cached_plane_geometry
                .is_empty(),
            "field {index} = {value}"
        );
    }
    let (unknown, _, _) = make("moFutureRefPlnData_c", valid);
    assert!(
        parse_stream(&unknown, "Contents/Config-0-MatesList", 0).records[0]
            .cached_plane_geometry
            .is_empty()
    );
    let mut schema = bytes.clone();
    schema[declaration_offset + 2] = 2;
    assert!(
        parse_stream(&schema, "Contents/Config-0-MatesList", 0).records[0]
            .cached_plane_geometry
            .is_empty()
    );

    let mut embedded = header(1);
    record(&mut embedded, Some("moMateCoincident"), "plane mate", 41);
    let units: Vec<_> = bytes[declaration_offset..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect();
    text(&mut embedded, &String::from_utf16(&units).unwrap());
    assert!(
        parse_stream(&embedded, "Contents/Config-0-MatesList", 0).records[0]
            .cached_plane_geometry
            .is_empty()
    );

    // A following mate must not supply missing doubles to this prefix.
    let mut crossing = bytes[..parameters_offset + 40].to_vec();
    record(&mut crossing, Some("moMateCoincident"), "next mate", 42);
    crossing[4..6].copy_from_slice(&2_u16.to_le_bytes());
    assert!(
        parse_stream(&crossing, "Contents/Config-0-MatesList", 0)
            .records
            .iter()
            .all(|r| r.cached_plane_geometry.is_empty())
    );
}
