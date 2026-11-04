// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Compare rendered material fallback with source-backed display-cache appearances.
//! Per-document output can contain private model names; keep reports out of Git.
use std::collections::BTreeMap;
use sw_rs::{
    CachedPropertyResolution, DocumentReader, Limits, PresentationScope,
    archive::{ArchiveLimits, FileSet},
};
use sw_viewer::project::Project;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let archive = args
        .next()
        .ok_or("usage: audit_appearances ARCHIVE ASSEMBLY_PATH")?;
    let root = args.next().ok_or("missing exact assembly path")?;
    let bytes = std::fs::read(&archive)?;
    let files = FileSet::from_archive(&bytes, &archive, ArchiveLimits::unlimited())?;
    drop(bytes);
    let mut project = Project::new(files);
    let scene = project.scene(&root, 0, false);
    let mut instances = BTreeMap::<(String, String), usize>::new();
    for node in &scene.hierarchy {
        if node["kind"] == "part" && node["status"] == "available" {
            let path = node["path"].as_str().ok_or("missing node path")?;
            let config = node["configuration"]
                .as_str()
                .ok_or("missing configuration")?;
            *instances.entry((path.into(), config.into())).or_default() += 1;
        }
    }
    let mut totals = BTreeMap::<String, usize>::new();
    let mut colors = BTreeMap::<String, usize>::new();
    let mut documents = Vec::new();
    for ((path, name), occurrences) in instances {
        let Some(bytes) = project.files.documents.get(&path) else {
            *totals
                .entry("embedded_occurrences_not_audited".into())
                .or_default() += occurrences;
            continue;
        };
        let reader = DocumentReader::open(bytes, &path, Limits::unlimited())?;
        let doc = reader.metadata()?;
        let geometry = reader.geometry()?;
        let p = &geometry.presentation;
        let reason = if sw_viewer::appearance::for_configuration(doc, &name).is_some() {
            "resolved_physical_material"
        } else if doc.materials.is_none() {
            "no_material_tree"
        } else if doc
            .configurations
            .iter()
            .find(|c| c.name == name)
            .and_then(|c| c.material.as_ref())
            .is_none()
        {
            "no_configuration_material_assignment"
        } else {
            "unresolved_physical_material_reference"
        };
        *totals.entry(format!("{reason}_occurrences")).or_default() += occurrences;
        let states: std::collections::BTreeSet<_> = p
            .display_states
            .iter()
            .filter_map(|s| match (&s.configuration, &s.display_state) {
                (
                    PresentationScope::Named { name: c },
                    PresentationScope::Named { name: state },
                ) if c == &name => Some(state.as_str()),
                _ => None,
            })
            .collect();
        let document_rgb = if states.len() == 1 {
            p.document_appearance_for_state(&name, states.first().copied().unwrap_or_default())
                .and_then(|a| a.native.as_ref())
                .and_then(|a| a.color_rgb)
        } else {
            None
        };
        let mut counts = BTreeMap::<String, usize>::new();
        for mesh in &geometry.model.tessellations {
            let triangles = mesh
                .triangles
                .iter()
                .filter(|t| {
                    let points = t.map(|i| mesh.vertices[i as usize].map(|v| v / 1000.));
                    let a = std::array::from_fn::<_, 3, _>(|i| points[1][i] - points[0][i]);
                    let b = std::array::from_fn::<_, 3, _>(|i| points[2][i] - points[0][i]);
                    let n = [
                        a[1] * b[2] - a[2] * b[1],
                        a[2] * b[0] - a[0] * b[2],
                        a[0] * b[1] - a[1] * b[0],
                    ];
                    n.iter().map(|x| x * x).sum::<f64>().sqrt() >= 1e-20
                        && points.iter().flatten().all(|x| x.is_finite())
                })
                .count();
            let (category, rgb) = match p.cached_property(&mesh.source_stream, mesh.source_offset) {
                CachedPropertyResolution::Resolved {
                    appearance_index, ..
                } => {
                    let rgb = p
                        .appearances
                        .get(appearance_index)
                        .and_then(|a| a.native.as_ref())
                        .and_then(|a| a.color_rgb);
                    (
                        if rgb.is_some() {
                            "cached_rgb"
                        } else {
                            "cached_without_rgb"
                        },
                        rgb,
                    )
                }
                CachedPropertyResolution::NotDecoded => (
                    if document_rgb.is_some() {
                        "document_rgb"
                    } else {
                        "no_scoped_rgb"
                    },
                    document_rgb,
                ),
                CachedPropertyResolution::Ambiguous { .. } => ("ambiguous_cache", None),
                CachedPropertyResolution::Unresolved { .. } => ("unresolved_cache", None),
            };
            *counts.entry(category.into()).or_default() += triangles;
            if let Some([r, g, b]) = rgb {
                *colors.entry(format!("{r:02X}{g:02X}{b:02X}")).or_default() +=
                    triangles * occurrences;
            }
            *totals
                .entry(format!("{reason}_{category}_triangles"))
                .or_default() += triangles * occurrences;
        }
        let configuration_index = doc
            .configurations
            .iter()
            .position(|c| c.name == name)
            .ok_or("missing named configuration")?;
        documents.push(serde_json::json!({"path":path.clone(),"configuration":name,"occurrences":occurrences,
            "physical_material_status":reason,"source_triangle_categories":counts,"document_rgb":document_rgb,
            "material_definitions":doc.materials.as_ref().map_or(0,|m|m.definitions.len()),
            "material_assignments":doc.materials.as_ref().map(|m| &m.assignments)}));
        drop(reader);
        let rendered = project.scene(&path, configuration_index, false);
        let mut selected_colors = BTreeMap::<String, usize>::new();
        let mut unresolved = 0;
        for batch in &rendered.batches {
            if let Some(rgb) = batch.saved_color {
                let [r, g, b] = rgb.map(|v| (v * 255.).round() as u8);
                *selected_colors
                    .entry(format!("{r:02X}{g:02X}{b:02X}"))
                    .or_default() += batch.vertex_count / 3;
            } else if batch.texture_index.is_none() {
                unresolved += batch.vertex_count / 3;
            }
        }
        let entry = documents.last_mut().expect("just appended document");
        entry["rendered_rgb_triangle_counts"] = serde_json::json!(selected_colors);
        entry["unresolved_saved_triangles"] = unresolved.into();
    }
    let material_triangles: usize = scene
        .batches
        .iter()
        .filter(|b| b.material_index.is_some())
        .map(|b| b.vertex_count / 3)
        .sum();
    let fallback_triangles: usize = scene
        .batches
        .iter()
        .filter(|b| b.material_index.is_none())
        .map(|b| b.vertex_count / 3)
        .sum();
    let saved_rgb_triangles: usize = scene
        .batches
        .iter()
        .filter(|b| b.saved_color.is_some())
        .map(|b| b.vertex_count / 3)
        .sum();
    let saved_fallback_triangles: usize = scene
        .batches
        .iter()
        .filter(|b| {
            b.saved_color.is_none() && b.material_index.is_none() && b.texture_index.is_none()
        })
        .map(|b| b.vertex_count / 3)
        .sum();
    let mut rendered_colors = BTreeMap::<String, usize>::new();
    let mut textured_triangles = 0;
    for batch in &scene.batches {
        if let Some(rgb) = batch.saved_color {
            let [r, g, b] = rgb.map(|v| (v * 255.).round() as u8);
            *rendered_colors
                .entry(format!("{r:02X}{g:02X}{b:02X}"))
                .or_default() += batch.vertex_count / 3;
        }
        if batch.texture_index.is_some() {
            textured_triangles += batch.vertex_count / 3;
        }
    }
    println!(
        "{}",
        serde_json::json!({"assembly":root,"rendered_triangles":scene.vertices.len()/27,
        "physical_material_triangles":material_triangles,"physical_unassigned_triangles":fallback_triangles,
        "saved_rgb_triangles":saved_rgb_triangles,"saved_diagnostic_triangles":saved_fallback_triangles,
        "rendered_rgb_triangle_counts":rendered_colors,"textured_triangles":textured_triangles,"material_presets":scene.materials.len(),"bitmap_images":scene.textures.len(),
        "rendered_occurrences":scene.occurrences,"totals":totals,"source_rgb_triangle_counts":colors,"documents":documents})
    );
    Ok(())
}
