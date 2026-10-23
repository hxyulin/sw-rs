// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Private-corpus stream inventory. Run with `--features archive --example investigate_geometry -- ARCHIVE OUTPUT`.
#[cfg(feature = "archive")]
use std::{collections::BTreeMap, io::Read};
#[cfg(feature = "archive")]
use sw_rs::{Container, Limits};
#[cfg(feature = "archive")]
fn inspect(path: &str, bytes: &[u8], output: &std::path::Path, dump: bool) {
    let container = match Container::parse(bytes, Limits::unlimited()) {
        Ok(container) => container,
        Err(error) => {
            println!(
                "{}",
                serde_json::json!({"path":path,"container_error":error.to_string()})
            );
            return;
        }
    };
    if path.to_ascii_lowercase().ends_with(".sldprt") {
        use sha2::{Digest, Sha256};
        let sha256 = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let result = sw_rs::geometry_with_limits(bytes, path, Limits::unlimited());
        let streams: BTreeMap<_, _> = container
            .streams
            .iter()
            .map(|(name, data)| {
                let classes: Vec<_> = data
                    .windows(4)
                    .enumerate()
                    .filter(|(_, w)| *w == [255, 255, 1, 0])
                    .filter_map(|(at, _)| {
                        let len =
                            u16::from_le_bytes(data.get(at + 4..at + 6)?.try_into().ok()?) as usize;
                        let name = std::str::from_utf8(data.get(at + 6..at + 6 + len)?).ok()?;
                        Some(serde_json::json!({"offset":at,"name":name}))
                    })
                    .collect();
                (
                    name.clone(),
                    serde_json::json!({"bytes":data.len(),"classes":classes}),
                )
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({"path":path,"sha256":sha256,"error":result.as_ref().err().map(ToString::to_string),"tables":result.as_ref().ok().map(|g|g.model.tessellations.len()),"triangles":result.as_ref().ok().map(|g|g.model.tessellations.iter().map(|t|t.triangles.len()).sum::<usize>()),"streams":streams})
        );
        if dump && result.is_err() {
            let id = Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let dir = output.join(id);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("path.txt"), path).unwrap();
            for (name, data) in &container.streams {
                std::fs::write(dir.join(name.replace('/', "__")), data).unwrap();
            }
        }
    } else {
        for (name, data) in container.streams.iter().filter(|(_, d)| d.len() > 22) {
            if [".sldprt", ".sldasm"]
                .iter()
                .any(|ext| name.to_ascii_lowercase().ends_with(ext))
                && data[..64.min(data.len())]
                    .windows(6)
                    .any(|w| w == [0x14, 0, 6, 0, 8, 0])
            {
                inspect(&format!("{path}/@embedded/{name}"), data, output, dump);
            }
        }
    }
}
#[cfg(feature = "archive")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        return Err("usage: investigate_geometry ARCHIVE OUTPUT [--dump]".into());
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&args[1])?)?;
    let output = std::path::Path::new(&args[2]);
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if ![".sldprt", ".sldasm"]
            .iter()
            .any(|ext| entry.name().to_ascii_lowercase().ends_with(ext))
        {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        inspect(
            entry.name(),
            &bytes,
            output,
            args.iter().any(|s| s == "--dump"),
        );
    }
    Ok(())
}

#[cfg(not(feature = "archive"))]
fn main() {
    eprintln!("enable --features archive");
}
