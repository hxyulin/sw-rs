// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Inspect assembly renderability without a browser or GPU; output is JSONL.
use sw_rs::archive::{ArchiveLimits, FileSet};
use sw_viewer::project::Project;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let filename = args
        .next()
        .ok_or("usage: inspect_project ARCHIVE [PATH_FILTER]")?;
    let filter = args.next().unwrap_or_default();
    let bytes = std::fs::read(&filename)?;
    let files = FileSet::from_archive(&bytes, &filename, ArchiveLimits::unlimited())?;
    eprintln!(
        "{} documents, {} ignored members",
        files.documents.len(),
        files.ignored_files
    );
    drop(bytes);
    let paths: Vec<_> = files
        .documents
        .keys()
        .filter(|p| p.to_ascii_lowercase().ends_with(".sldasm") && p.contains(&filter))
        .cloned()
        .collect();
    let mut project = Project::new(files);
    for path in paths {
        eprintln!("Inspecting {path}");
        let scene = project.scene(&path, 0, false);
        println!(
            "{}",
            serde_json::json!({"path":path,
            "configurations":scene.configurations,
            "triangles":scene.vertices.len()/27,"occurrences":scene.occurrences,
            "missing_references":scene.missing_references,"visited_nodes":scene.visited_nodes,
            "diagnostics":scene.diagnostics,"parser_diagnostics":scene.metadata["diagnostics"],"reference_replacements":scene.metadata["reference_replacements"]})
        );
    }
    Ok(())
}
