// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Native JSON metadata inspection CLI.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: sw-dump MODEL.SLDPRT|MODEL.SLDASM|MODEL.SLDDRW")?;
    let data = std::fs::read(&path)?;
    let doc = sw_rs::parse(&data, &path.to_string_lossy())?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &doc)?;
    Ok(())
}
