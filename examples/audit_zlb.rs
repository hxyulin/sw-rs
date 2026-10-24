// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Verify extracted legacy stream wrappers; no OLE dependency or CAD extraction.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    fn hash(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    let mut args = std::env::args_os().skip(1);
    let input = std::path::PathBuf::from(
        args.next()
            .ok_or("usage: audit_zlb STREAM.zlb | DIRECTORY")?,
    );
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let mut paths = if input.is_dir() {
        std::fs::read_dir(input)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("zlb"))
            })
            .collect()
    } else {
        vec![input]
    };
    paths.sort();
    for path in paths {
        let bytes = std::fs::read(&path)?;
        let result = match sw_rs::decode_zlb(&bytes, sw_rs::Limits::unlimited()) {
            Ok(stream) => {
                serde_json::json!({"decoded_bytes":stream.data.len(),"decoded_sha256":hash(&stream.data),"chunks":stream.chunks,"terminator":stream.terminator})
            }
            Err(error) => serde_json::json!({"error":error.to_string()}),
        };
        println!(
            "{}",
            serde_json::json!({"path":path,"encoded_bytes":bytes.len(),"encoded_sha256":hash(&bytes),"result":result})
        );
    }
    Ok(())
}
