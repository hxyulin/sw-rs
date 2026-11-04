// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Inspect repeated visual-property candidates without assigning them to entities.
//! Output includes private source strings; keep reports outside version control.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: inspect_visual_candidates MODEL [--dump-stream PATH]")?;
    let dump = match args.next().as_deref() {
        None => None,
        Some("--dump-stream") => Some(args.next().ok_or("missing stream output path")?),
        Some(_) => return Err("unknown argument".into()),
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let bytes = std::fs::read(&path)?;
    let reader = sw_rs::DocumentReader::open(&bytes, &path, sw_rs::Limits::unlimited())?;
    let name = "ThirdPtyStore/VisualStates";
    let stream = reader.stream(name)?.ok_or("no VisualStates stream")?;
    if let Some(path) = dump {
        std::fs::write(path, stream)?;
    }
    let inspection = reader.inspect_stream(name)?.ok_or("no inspection")?;
    let doc = reader.metadata()?;
    let keywords = reader.inspect_stream("swXmlContents/KeyWords")?;
    let surfaces = reader.surface_references().ok();
    let mut candidates = Vec::new();
    // Candidate-only matching: two contiguous CString frames, six finite doubles,
    // and the fixed shader-identifier position seen in supported property prefixes.
    // This is not an MFC object-reference decoder or an ownership relationship.
    for pair in inspection.framed_strings.windows(2) {
        let texture = &pair[0];
        let material = &pair[1];
        if !texture.value.is_empty() || material.offset != texture.offset + 4 || texture.offset < 12
        {
            continue;
        }
        let material_units = material.value.encode_utf16().count();
        if material_units >= 254 {
            continue;
        } // Only the short CString frame is supported here.
        let optical_start = material.offset + 4 + material_units * 2;
        let end = optical_start + 48;
        let Some(raw) = stream.get(optical_start..end) else {
            continue;
        };
        let optical: Vec<_> = raw
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_le_bytes(*b))
            .collect();
        if !optical.iter().all(|v| v.is_finite()) {
            continue;
        }
        let Some(shader) = inspection
            .framed_strings
            .iter()
            .find(|s| s.offset == end + 44 && !s.value.is_empty())
        else {
            continue;
        };
        let start = texture.offset - 12;
        let packed = u32::from_le_bytes(
            stream[start..start + 4]
                .try_into()
                .expect("bounded color word"),
        );
        let rgb = (packed >> 24 == 0).then_some([
            packed as u8,
            (packed >> 8) as u8,
            (packed >> 16) as u8,
        ]);
        let tag = start
            .checked_sub(2)
            .map(|at| u16::from_le_bytes([stream[at], stream[at + 1]]));
        let declared = doc
            .visuals
            .definitions
            .iter()
            .any(|d| d.source.stream == name && d.source.path == format!("@byte[{start}]"));
        // Correlate the observed source-ID prefix, without promoting an ID match
        // to native feature ownership or a display-cache surface assignment.
        let source_header = start.checked_sub(32).and_then(|at| {
            let raw = stream.get(at..start)?;
            let words: Vec<_> = raw[..24].as_chunks::<4>().0.iter()
                .map(|b| u32::from_le_bytes(*b)).collect();
            if !matches!(words[0], 17000 | 18000)
                || !matches!(words[3], 1 | 3) || words[4..] != [1, 2]
                || raw[24..] != [7, 128, 1, 0, 0, 0, 9, 128] { return None; }
            let id = words[1].to_string();
            let xml_matches: Vec<_> = keywords.as_ref().into_iter()
                .flat_map(|k| &k.xml_elements)
                .filter(|e| e.attributes.iter().any(|a| a.name == "id" && a.value == id))
                .collect();
            let cache_matches: Vec<_> = surfaces.into_iter().flat_map(|s| &s.records)
                .filter(|r| r.surface_references.feature_source_id() == Some(words[1]))
                .map(|r| serde_json::json!({"source_stream":r.source_stream,"source_offset":r.source_offset}))
                .collect();
            Some(serde_json::json!({"offset":at,"raw_words":words,"xml_id_candidates":xml_matches,
                "cache_source_id_candidates":cache_matches,
                "ownership":"unresolved_source_id_match"}))
        });
        candidates.push(serde_json::json!({"offset":start,"preceding_word":tag,"packed_color":packed,"rgb_candidate":rgb,
            "optical_values_candidate":optical,"shader_identifier_candidate":shader.value,"material_name_candidate":material.value,
            "source_header_candidate":source_header,"already_decoded_definition":declared,"ownership":"unresolved_candidate"}));
    }
    println!(
        "{}",
        serde_json::json!({"schema":"sw-rs-visual-candidates-v1","source_sha256":reader.source_sha256(),
        "decoded_visuals":doc.visuals,"candidate_properties":candidates,"inspection":inspection,
        "warning":"Pattern matches are candidates, not proven class instances or effective appearance assignments"})
    );
    Ok(())
}
