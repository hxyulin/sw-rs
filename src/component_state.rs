// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! One supported explicit component-reference prefix; no global grounding inference.
use crate::{BinaryReader, ByteSpan, Component, FramedString};
use serde::Serialize;

/// Saved component fixation, bound to an exact occurrence in its owning assembly.
#[derive(Clone, Debug, Serialize)]
pub struct ComponentFixation {
    /// Saved fixation relative to the owning assembly.
    /// False can still be fully constrained by mates; floating ancestors can
    /// move an occurrence whose local fixation is true.
    pub fixed_in_owner: bool,
    /// Exact decoded Config-N carrier in this document.
    pub source_stream: String,
    /// Explicit moReference_c header and checked fields, not a complete object boundary.
    pub prefix_span: ByteSpan,
    /// Native boolean u16 offset in the same source stream.
    pub flag_offset: usize,
    /// Saved source ID matched uniquely to the owning XML occurrence's swID.
    pub reference_source_id: u32,
    /// Exact saved instance label matched to component name and reference number.
    pub reference_name: FramedString,
    /// Raw reference-configuration text; internal flexible aliases are not rewritten.
    pub saved_reference_configuration: FramedString,
}

pub(crate) fn apply(components: &mut [Component], bytes: &[u8], source: &str) {
    let Some(evidence) = first_reference(bytes, source) else {
        return;
    };
    let mut matches = components
        .iter()
        .enumerate()
        .filter(|(_, c)| c.native_id == Some(evidence.reference_source_id));
    let Some((index, component)) = matches.next() else {
        return;
    };
    if matches.next().is_some() || component.is_suppressed {
        return;
    }
    let (Some(name), Some(number)) = (&component.component_name, component.reference_number) else {
        return;
    };
    if number == 0 || evidence.reference_name.value != format!("{name}-{number}") {
        return;
    }
    components[index].fixation = Some(evidence);
}

fn declaration(cursor: &mut BinaryReader<'_>, expected: &str) -> Option<()> {
    if cursor.u16()? != u16::MAX
        || cursor.u16()? != 1
        || usize::from(cursor.u16()?) != expected.len()
        || cursor.take(expected.len())? != expected.as_bytes()
    {
        return None;
    }
    Some(())
}

fn first_reference(bytes: &[u8], source: &str) -> Option<ComponentFixation> {
    let mut cursor = BinaryReader::new(bytes);
    declaration(&mut cursor, "moAssembly_c")?;
    cursor.u32()?; // Opaque enclosing-header word, not a version or physical value.
    if cursor.u32()? != 0 || ![3400, 10000].contains(&cursor.u32()?) {
        return None;
    }
    declaration(&mut cursor, "moNodeName_c")?;
    cursor.string()?;
    if cursor.take(16)? != [0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0, 0, 0, 0, 1, 0, 0] {
        return None;
    }
    cursor.string()?;
    if cursor.u32()? != 1 || matches!(cursor.u16()?, 0 | u16::MAX) {
        return None;
    }
    let reference_offset = cursor.position();
    // Do not infer a reused class slot from marker counts or a matching basename.
    declaration(&mut cursor, "moReference_c")?;
    if !(0x8001..0xffff).contains(&cursor.u16()?) {
        return None;
    }
    let reference_name = cursor.string()?;
    let header = cursor.take(16)?;
    if header[0] != 0
        || ![0, 1].contains(&header[1])
        || header[2..7] != [0; 5]
        || ![0x40, 0xc0].contains(&header[7])
        || header[12..16] != [0; 4]
    {
        return None;
    }
    let source_id = u32::from_le_bytes(header[8..12].try_into().ok()?);
    if matches!(source_id, 0 | u32::MAX) {
        return None;
    }
    cursor.string()?;
    let body = cursor.take(55)?;
    if body[..20] != [0; 20] || body[20..24] != 101_u32.to_le_bytes() {
        return None;
    }
    cursor.string()?;
    cursor.take(84)?; // Observed common-feature scalar block, without assigning semantics.
    for _ in 0..12 {
        if !cursor.f64()?.is_finite() {
            return None;
        }
    }
    cursor.u16()?; // Two raw words precede the independently changed fixation word.
    cursor.u16()?;
    let flag_offset = cursor.position();
    let fixed_in_owner = match cursor.u16()? {
        0 => false,
        1 => true,
        _ => return None,
    };
    cursor.u32()?;
    let saved_reference_configuration = cursor.string()?;
    if reference_name.value.is_empty() || saved_reference_configuration.value.is_empty() {
        return None;
    }
    Some(ComponentFixation {
        fixed_in_owner,
        source_stream: source.into(),
        prefix_span: ByteSpan {
            offset: reference_offset,
            length: cursor.position() - reference_offset,
        },
        flag_offset,
        reference_source_id: source_id,
        reference_name,
        saved_reference_configuration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn class(bytes: &mut Vec<u8>, name: &str) {
        bytes.extend([255, 255, 1, 0]);
        bytes.extend((name.len() as u16).to_le_bytes());
        bytes.extend(name.as_bytes());
    }
    fn text(bytes: &mut Vec<u8>, value: &str) {
        let units: Vec<_> = value.encode_utf16().collect();
        bytes.extend([255, 254, 255, units.len() as u8]);
        bytes.extend(units.into_iter().flat_map(u16::to_le_bytes));
    }
    fn component(id: u32, name: &str, number: u32) -> Component {
        Component {
            name: "editable label is not the identity".into(),
            component_name: Some(name.into()),
            reference_number: Some(number),
            native_id: Some(id),
            is_flexible: None,
            fixation: None,
            path: "reused-part.SLDPRT".into(),
            ref_config: "Default".into(),
            component_reference: String::new(),
            is_suppressed: false,
            is_hidden: false,
            exclude_from_bom: false,
            transform: Some([
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ]),
        }
    }
    fn fixture(fixed: bool) -> (Vec<u8>, usize, usize) {
        let mut bytes = Vec::new();
        class(&mut bytes, "moAssembly_c");
        for word in [123_u32, 0, 10000] {
            bytes.extend(word.to_le_bytes());
        }
        class(&mut bytes, "moNodeName_c");
        text(&mut bytes, "assembly😀");
        bytes.extend([0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0, 0, 0, 0, 1, 0, 0]);
        text(&mut bytes, "root note");
        bytes.extend(1_u32.to_le_bytes());
        bytes.extend(2_u16.to_le_bytes());
        class(&mut bytes, "moReference_c");
        bytes.extend(0x8006_u16.to_le_bytes());
        text(&mut bytes, "anchor😀-2");
        bytes.extend([0, 0, 0, 0, 0, 0, 0, 0x40]);
        bytes.extend(28_u32.to_le_bytes());
        bytes.extend([0; 4]);
        text(&mut bytes, "reference note");
        let body = bytes.len();
        bytes.extend([0; 55]);
        bytes[body + 20..body + 24].copy_from_slice(&101_u32.to_le_bytes());
        text(&mut bytes, "metadata😀");
        bytes.extend([0; 84]);
        let doubles = bytes.len();
        for _ in 0..12 {
            bytes.extend(0.0_f64.to_le_bytes());
        }
        bytes.extend(4_u16.to_le_bytes());
        bytes.extend(3_u16.to_le_bytes());
        let flag = bytes.len();
        bytes.extend(u16::from(fixed).to_le_bytes());
        bytes.extend(163_u32.to_le_bytes());
        text(&mut bytes, "Default-_flexible1");
        (bytes, flag, doubles)
    }
    #[test]
    fn explicit_fixation_uses_exact_owner_identity_and_retains_offsets() {
        for fixed in [false, true] {
            let (bytes, flag, _) = fixture(fixed);
            let mut components = vec![component(29, "anchor😀", 1), component(28, "anchor😀", 2)];
            apply(&mut components, &bytes, "Contents/Config-7");
            assert!(components[0].fixation.is_none()); // Same file and identity pose do not imply fixed/floating.
            let evidence = components[1].fixation.as_ref().unwrap();
            assert_eq!(evidence.fixed_in_owner, fixed);
            assert_eq!(evidence.flag_offset, flag);
            assert_eq!(evidence.source_stream, "Contents/Config-7");
            assert_eq!(evidence.reference_source_id, 28);
            assert_eq!(evidence.reference_name.value, "anchor😀-2");
            assert_eq!(
                evidence.saved_reference_configuration.value,
                "Default-_flexible1"
            );
            assert_eq!(components[1].ref_config, "Default"); // No suffix-based configuration rewriting.
            assert_eq!(
                evidence.prefix_span.offset + evidence.prefix_span.length,
                bytes.len()
            );
        }
    }
    #[test]
    fn absent_ambiguous_or_mismatched_occurrence_identity_stays_unknown() {
        let (bytes, _, _) = fixture(true);
        for mut components in [
            vec![component(29, "anchor😀", 2)],
            vec![component(28, "renamed", 2)],
            vec![component(28, "anchor😀", 1)],
            vec![component(28, "anchor😀", 2), component(28, "other", 3)],
        ] {
            apply(&mut components, &bytes, "Contents/Config-0");
            assert!(components.iter().all(|c| c.fixation.is_none()));
        }
        for missing in [0, 1, 2, 3] {
            let mut components = vec![component(28, "anchor😀", 2)];
            match missing {
                0 => components[0].native_id = None,
                1 => components[0].component_name = None,
                2 => components[0].reference_number = None,
                _ => components[0].is_suppressed = true,
            }
            apply(&mut components, &bytes, "Contents/Config-0");
            assert!(components[0].fixation.is_none());
        }
    }
    #[test]
    fn truncated_nonfinite_unknown_or_reused_reference_prefixes_are_not_fixed_flags() {
        let (bytes, flag, doubles) = fixture(true);
        for end in 0..bytes.len() {
            assert!(first_reference(&bytes[..end], "s").is_none());
        }
        for (at, replacement) in [
            (2, 2_u16.to_le_bytes().to_vec()),
            (flag, 2_u16.to_le_bytes().to_vec()),
            (doubles, f64::NAN.to_le_bytes().to_vec()),
        ] {
            let mut invalid = bytes.clone();
            invalid[at..at + replacement.len()].copy_from_slice(&replacement);
            assert!(first_reference(&invalid, "s").is_none());
        }
        let evidence = first_reference(&bytes, "s").unwrap();
        let start = evidence.prefix_span.offset;
        let mut reused = bytes.clone();
        reused.splice(
            start..start + 6 + "moReference_c".len(),
            0x8008_u16.to_le_bytes(),
        );
        assert!(first_reference(&reused, "s").is_none());
    }
    #[test]
    fn multiple_configurations_or_alias_carriers_do_not_gain_fixation_by_index_guessing() {
        let (bytes, _, _) = fixture(true);
        let xml = r#"<tree><swHeader><swFile id="1" swPath="root.SLDASM"/><swFile id="2" swPath="part.SLDPRT"/></swHeader><swModelList>
        <swModel id="3" swFileRef="1" swConfigurationId="0" swConfigurationName="First"><swReference swName="label" swComponentName="anchor😀" swReferenceNumber="2" swID="28" swSuppressed="NO" swModelRef="5"/></swModel>
        <swModel id="5" swFileRef="2" swConfigurationId="0" swConfigurationName="Default"/></swModelList></tree>"#;
        let streams = std::collections::BTreeMap::from([
            ("Contents/Config-0".into(), bytes.clone()),
            (
                "swXmlContents/COMPINSTANCETREE".into(),
                xml.as_bytes().to_vec(),
            ),
        ]);
        let decode = |streams| {
            crate::metadata_from_container(
                crate::Container {
                    streams,
                    diagnostics: vec![],
                },
                "root.SLDASM",
            )
        };
        let doc = decode(streams.clone());
        assert!(
            doc.configurations[0].components[0]
                .fixation
                .as_ref()
                .unwrap()
                .fixed_in_owner
        );
        for extra in ["Contents/Config-9", "Contents/Config-00"] {
            let mut streams = streams.clone();
            streams.insert(extra.into(), bytes.clone());
            let doc = decode(streams);
            assert!(
                doc.configurations
                    .iter()
                    .flat_map(|c| &c.components)
                    .all(|c| c.fixation.is_none()),
                "extra carrier {extra}"
            );
        }
        // An inactive XML configuration without a cached Config-N stream also
        // prevents proving that this document has only one configuration.
        let other =
            r#"<swModel id="4" swFileRef="1" swConfigurationId="9" swConfigurationName="Other"/>"#;
        let mut streams = streams;
        streams.insert(
            "swXmlContents/COMPINSTANCETREE".into(),
            xml.replace("</swModelList>", &format!("{other}</swModelList>"))
                .into_bytes(),
        );
        let doc = decode(streams);
        assert!(doc.configurations[0].components[0].fixation.is_none());
    }
}
