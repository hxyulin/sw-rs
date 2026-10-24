// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Compare metadata+geometry wrappers with a shared decode-once reader on one Part.
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .ok_or("usage: benchmark_reader PART.SLDPRT [ROUNDS]")?;
    let rounds = args
        .next()
        .map(|s| s.to_string_lossy().parse::<usize>())
        .transpose()?
        .unwrap_or(5);
    if rounds == 0 || args.next().is_some() {
        return Err("rounds must be positive; no extra arguments".into());
    }
    let bytes = std::fs::read(&path)?;
    let name = path.to_string_lossy();
    let limits = sw_rs::Limits::unlimited();
    let legacy_doc = sw_rs::parse_with_limits(&bytes, &name, limits)?;
    let legacy_geo = sw_rs::geometry_with_limits(&bytes, &name, limits)?;
    let reader = sw_rs::DocumentReader::open(&bytes, &name, limits)?;
    if serde_json::to_vec(&legacy_doc)? != serde_json::to_vec(reader.metadata()?)?
        || serde_json::to_vec(&legacy_geo)? != serde_json::to_vec(reader.geometry()?)?
    {
        return Err("output mismatch".into());
    }
    let source_sha256 = reader.source_sha256().to_owned();
    let decoded_bytes: usize = reader.container()?.streams.values().map(Vec::len).sum();
    drop((reader, legacy_doc, legacy_geo));
    let mut separate = Vec::new();
    let mut shared = Vec::new();
    for round in 0..rounds {
        for is_shared in if round % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let start = Instant::now();
            if is_shared {
                let reader = sw_rs::DocumentReader::open(black_box(&bytes), &name, limits)?;
                black_box(reader.metadata()?);
                black_box(reader.geometry()?);
            } else {
                black_box(sw_rs::parse_with_limits(black_box(&bytes), &name, limits)?);
                black_box(sw_rs::geometry_with_limits(
                    black_box(&bytes),
                    &name,
                    limits,
                )?);
            }
            let ms = start.elapsed().as_secs_f64() * 1000.;
            if is_shared {
                shared.push(ms);
            } else {
                separate.push(ms);
            }
        }
    }
    let median = |values: &[f64]| {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        sorted[sorted.len() / 2]
    };
    println!(
        "{}",
        serde_json::json!({"source_sha256":source_sha256,"source_bytes":bytes.len(),"retained_decoded_bytes":decoded_bytes,"rounds":rounds,
        "output_equal":true,"separate_ms":separate,"shared_ms":shared,"separate_median_ms":median(&separate),"shared_median_ms":median(&shared),
        "scope":"in-memory metadata plus geometry, including allocations and drops; no peak-RSS claim"})
    );
    Ok(())
}
