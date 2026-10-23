// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Filesystem-free assembly traversal of saved native display caches.
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use sw_rs::{
    Document,
    archive::{FileSet, Resolution},
};
const IDENTITY: [f64; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];
#[derive(Default)]
pub struct Scene {
    /// Interleaved world positions (metres), face normals, diagnostic RGB.
    pub vertices: Vec<f32>,
    pub occurrences: usize,
    /// Unavailable referenced occurrences omitted while available siblings render.
    pub missing_references: usize,
    /// Total document visits, including unsupported documents and cycles.
    pub visited_nodes: usize,
    pub diagnostics: Vec<String>,
    pub metadata: serde_json::Value,
    pub configurations: Vec<String>,
}
pub struct Project {
    pub files: FileSet,
    metadata: BTreeMap<String, Document>,
    embedded_documents: BTreeMap<String, Arc<[u8]>>,
    embedded_files: BTreeMap<String, FileSet>,
    geometry: BTreeMap<String, Result<sw_rs::GeometryDocument, String>>,
}
impl Project {
    pub fn new(files: FileSet) -> Self {
        Self {
            files,
            metadata: BTreeMap::new(),
            embedded_documents: BTreeMap::new(),
            embedded_files: BTreeMap::new(),
            geometry: BTreeMap::new(),
        }
    }
    fn document(&mut self, path: &str) -> Result<&Document, String> {
        if !self.metadata.contains_key(path) {
            let bytes = self
                .files
                .documents
                .get(path)
                .or_else(|| self.embedded_documents.get(path))
                .cloned()
                .ok_or("document not present")?;
            let mut container = sw_rs::Container::parse(&bytes, sw_rs::Limits::unlimited())
                .map_err(|e| e.to_string())?;
            let stream_names = container.streams.keys().cloned().collect();
            let names: Vec<_> = container
                .streams
                .keys()
                .filter(|name| {
                    name.starts_with("ImportedComp/") || name.starts_with("VirtualComp/")
                })
                .cloned()
                .collect();
            let mut embedded = FileSet::default();
            for stream in names {
                let name = if let Some(name) = stream.strip_prefix("ImportedComp/") {
                    name.to_owned()
                } else {
                    let name = stream.strip_prefix("VirtualComp/").unwrap();
                    if let Some(stem) = name.strip_suffix("-sldasm") {
                        format!("{stem}.SLDASM")
                    } else if let Some(stem) = name.strip_suffix("-sldprt") {
                        format!("{stem}.SLDPRT")
                    } else {
                        continue;
                    }
                };
                let Ok(name) = sw_rs::archive::normalize_path(&name) else {
                    continue;
                };
                if ![".sldprt", ".sldasm"]
                    .iter()
                    .any(|ext| name.to_ascii_lowercase().ends_with(ext))
                {
                    continue;
                }
                let bytes: Arc<[u8]> = container.streams.remove(&stream).unwrap().into();
                self.embedded_documents
                    .insert(format!("{path}/@embedded/{name}"), bytes.clone());
                embedded.documents.insert(name, bytes);
            }
            self.embedded_files.insert(path.into(), embedded);
            let mut doc = sw_rs::metadata_from_container(container, path);
            doc.stream_names = stream_names;
            // The inspector does not display PNGs; do not retain duplicate caches.
            doc.preview_png = Vec::new();
            for config in &mut doc.configurations {
                config.preview_png = Vec::new();
            }
            self.metadata.insert(path.into(), doc);
        }
        Ok(&self.metadata[path])
    }
    pub fn scene(&mut self, path: &str, config: usize, show_hidden: bool) -> Scene {
        let mut scene = Scene::default();
        match self.document(path) {
            Ok(doc) => {
                scene.metadata = serde_json::to_value(doc).unwrap_or_default();
                scene.configurations = doc.configurations.iter().map(|c| c.name.clone()).collect();
            }
            Err(e) => {
                scene.diagnostics.push(e);
                return scene;
            }
        }
        let name = scene
            .configurations
            .get(config)
            .cloned()
            .unwrap_or_default();
        self.visit(
            path,
            &name,
            IDENTITY,
            show_hidden,
            &mut BTreeSet::new(),
            &mut scene,
        );
        scene
    }
    fn visit(
        &mut self,
        path: &str,
        name: &str,
        pose: [f64; 16],
        show_hidden: bool,
        ancestors: &mut BTreeSet<(String, String)>,
        scene: &mut Scene,
    ) {
        scene.visited_nodes += 1;
        if ancestors.contains(&(path.into(), name.into())) {
            scene.diagnostics.push(format!("Cyclic reference: {path}"));
            return;
        }
        let doc = match self.document(path) {
            Ok(d) => d,
            Err(e) => {
                scene.diagnostics.push(format!("{path}: {e}"));
                return;
            }
        };
        let kind = doc.document_type.clone();
        if kind == "drawing" {
            scene
                .diagnostics
                .push("Drawing metadata is available; drawing rendering is not supported.".into());
            return;
        }
        let matching: Vec<_> = doc
            .configurations
            .iter()
            .filter(|c| c.name == name)
            .collect();
        if matching.len() != 1 {
            scene
                .diagnostics
                .push(format!("Missing/ambiguous configuration: {path} / {name}"));
            return;
        }
        if kind == "assembly" {
            let replacements = doc.reference_replacements.clone();
            let children: Vec<_> = matching[0]
                .components
                .iter()
                .map(|c| {
                    (
                        c.name.clone(),
                        c.path.clone(),
                        c.ref_config.clone(),
                        c.is_suppressed,
                        c.is_hidden,
                        c.transform,
                    )
                })
                .collect();
            ancestors.insert((path.into(), name.into()));
            for (label, reference, config, suppressed, hidden, transform) in children {
                if suppressed || (hidden && !show_hidden) {
                    continue;
                }
                let Some(local) = transform.filter(rigid) else {
                    scene
                        .diagnostics
                        .push(format!("Unchecked transform: {label}"));
                    continue;
                };
                let mut resolved = resolve_reference(&self.files, path, &reference, &replacements);
                if resolved == Resolution::Missing {
                    let mut scope = Some(path);
                    while let Some(owner) = scope {
                        if let Some(files) = self.embedded_files.get(owner) {
                            let virtual_reference = virtual_reference(owner, &reference);
                            let local_reference =
                                virtual_reference.as_deref().unwrap_or(&reference);
                            resolved = match files.resolve("", local_reference) {
                                Resolution::Found(name) => {
                                    Resolution::Found(format!("{owner}/@embedded/{name}"))
                                }
                                other => other,
                            };
                            if resolved != Resolution::Missing {
                                break;
                            }
                        }
                        scope = owner.rsplit_once("/@embedded/").map(|(root, _)| root);
                    }
                }
                match resolved {
                    Resolution::Found(child) => self.visit(
                        &child,
                        &config,
                        multiply(pose, local),
                        show_hidden,
                        ancestors,
                        scene,
                    ),
                    Resolution::Missing => {
                        scene.missing_references += 1;
                        scene.diagnostics.push(format!(
                            "Missing reference (skipped): {label} · {reference}"
                        ));
                    }
                    Resolution::Ambiguous(_) => scene
                        .diagnostics
                        .push(format!("Ambiguous reference: {label}")),
                }
            }
            ancestors.remove(&(path.into(), name.into()));
            return;
        }
        if !self.geometry.contains_key(path) {
            let retained: usize = self
                .geometry
                .values()
                .filter_map(|g| g.as_ref().ok())
                .flat_map(|g| &g.model.tessellations)
                .map(|m| m.vertices.len())
                .sum();
            if retained > 2_000_000 {
                self.geometry.clear();
            }

            let bytes = self
                .files
                .documents
                .get(path)
                .or_else(|| self.embedded_documents.get(path))
                .expect("parsed document source");
            self.geometry.insert(
                path.into(),
                sw_rs::geometry_with_limits(bytes, path, sw_rs::Limits::unlimited())
                    .map(|mut g| {
                        for c in &mut g.configurations {
                            c.preview_png = Vec::new();
                        }
                        g
                    })
                    .map_err(|e| e.to_string()),
            );
        }
        let g = match &self.geometry[path] {
            Ok(g) => g,
            Err(e) => {
                scene.diagnostics.push(format!("{path}: {e}"));
                return;
            }
        };
        let mut hash = 0u32;
        for b in path.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(u32::from(b));
        }
        let palette = [
            [0.15, 0.72, 0.69],
            [0.52, 0.65, 0.85],
            [0.85, 0.68, 0.4],
            [0.62, 0.74, 0.71],
            [0.73, 0.58, 0.72],
        ];
        let color = palette[hash as usize % palette.len()];
        scene.occurrences += 1;
        for mesh in &g.model.tessellations {
            for triangle in &mesh.triangles {
                let p = triangle.map(|i| point(pose, mesh.vertices[i as usize]));
                let a = std::array::from_fn::<_, 3, _>(|i| p[1][i] - p[0][i]);
                let b = std::array::from_fn::<_, 3, _>(|i| p[2][i] - p[0][i]);
                let n = [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ];
                let length = n.iter().map(|x| x * x).sum::<f64>().sqrt();
                if length < 1e-20 || p.iter().flatten().any(|x| !x.is_finite()) {
                    continue;
                }
                for v in p {
                    scene.vertices.extend(v.map(|x| x as f32));
                    scene.vertices.extend(n.map(|x| (x / length) as f32));
                    scene.vertices.extend(color);
                }
            }
        }
    }
}
fn virtual_reference(owner: &str, reference: &str) -> Option<String> {
    let reference = reference.replace('\\', "/");
    let basename = reference.rsplit('/').next()?;
    let (name, parent) = basename.rsplit_once('^')?;
    let (parent, extension) = parent.rsplit_once('.')?;
    let owner = owner.rsplit('/').next()?.rsplit_once('.')?.0;
    (parent.to_lowercase() == owner.to_lowercase()
        && matches!(extension.to_ascii_lowercase().as_str(), "sldprt" | "sldasm"))
    .then(|| format!("{name}.{extension}"))
}
fn resolve_reference(
    files: &FileSet,
    owner: &str,
    reference: &str,
    replacements: &[sw_rs::ReferenceReplacement],
) -> Resolution {
    let normalize = |p: &str| p.replace('\\', "/").to_lowercase();
    let mut target = reference.to_owned();
    let mut seen = BTreeSet::new();
    loop {
        let original = normalize(&target);
        if !seen.insert(original.clone()) {
            return Resolution::Ambiguous(seen.into_iter().collect());
        }
        let targets: BTreeSet<_> = replacements
            .iter()
            .filter(|r| normalize(&r.original_path) == original)
            .map(|r| normalize(&r.replacement_path))
            .collect();
        match targets.len() {
            0 => return files.resolve(owner, &target),
            1 => {
                let next = targets.first().unwrap();
                if *next == original {
                    return files.resolve(owner, &target);
                }
                target = next.clone();
            }
            _ => return Resolution::Ambiguous(targets.into_iter().collect()),
        }
    }
}
fn point(m: [f64; 16], p: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|r| {
        m[r] * p[0] * 0.001 + m[4 + r] * p[1] * 0.001 + m[8 + r] * p[2] * 0.001 + m[12 + r]
    })
}
fn multiply(a: [f64; 16], b: [f64; 16]) -> [f64; 16] {
    std::array::from_fn(|i| {
        let r = i % 4;
        let c = i / 4;
        (0..4).map(|k| a[4 * k + r] * b[4 * c + k]).sum()
    })
}
fn rigid(m: &[f64; 16]) -> bool {
    if m.iter().any(|v| !v.is_finite())
        || [3, 7, 11, 15]
            .into_iter()
            .zip([0., 0., 0., 1.])
            .any(|(i, v)| (m[i] - v).abs() > 1e-6)
    {
        return false;
    }
    for i in 0..3 {
        for j in 0..3 {
            if ((0..3).map(|k| m[i * 4 + k] * m[j * 4 + k]).sum::<f64>()
                - if i == j { 1. } else { 0. })
            .abs()
                > 1e-6
            {
                return false;
            }
        }
    }
    let determinant = m[0] * (m[5] * m[10] - m[6] * m[9]) - m[4] * (m[1] * m[10] - m[2] * m[9])
        + m[8] * (m[1] * m[6] - m[2] * m[5]);
    (determinant.abs() - 1.).abs() < 1e-6
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_frame_conversion_and_composition() {
        let mut a = IDENTITY;
        a[12] = 2.;
        let mut b = IDENTITY;
        b[13] = 3.;
        assert_eq!(point(multiply(a, b), [1000., 0., 0.]), [3., 3., 0.]);
        assert!(rigid(&a));
        a[0] = -1.;
        assert!(rigid(&a)); // Saved mirrored occurrences remain valid isometries.
        a[3] = 1.;
        assert!(!rigid(&a));
    }
}

#[cfg(test)]
mod traversal_tests {
    use super::*;
    fn doc(kind: &str, components: Vec<sw_rs::Component>) -> Document {
        Document {
            document_type: kind.into(),
            version: 0,
            global_properties: Default::default(),
            configurations: vec![sw_rs::Configuration {
                index: 0,
                name: "Default".into(),
                parent_index: None,
                properties: Default::default(),
                mass_properties: None,
                components,
                cut_list: vec![],
                preview_png: vec![],
            }],
            preview_png: vec![],
            stream_names: vec![],
            diagnostics: vec![],
            sheets: vec![],
            reference_replacements: vec![],
            replacement_references: vec![],
            header_references: vec![],
            schema: "sw-rs-metadata-v1",
        }
    }
    fn component(path: &str, transform: Option<[f64; 16]>, hidden: bool) -> sw_rs::Component {
        sw_rs::Component {
            name: path.into(),
            path: path.into(),
            ref_config: "Default".into(),
            component_reference: String::new(),
            is_suppressed: false,
            is_hidden: hidden,
            exclude_from_bom: false,
            transform,
        }
    }
    fn fixture(children: Vec<sw_rs::Component>) -> Project {
        let mut files = FileSet::default();
        files
            .documents
            .insert("root.SLDASM".into(), Vec::new().into());
        files
            .documents
            .insert("part.SLDPRT".into(), Vec::new().into());
        let mut p = Project::new(files);
        p.metadata
            .insert("root.SLDASM".into(), doc("assembly", children));
        p.metadata.insert("part.SLDPRT".into(), doc("part", vec![]));
        p.geometry.insert(
            "part.SLDPRT".into(),
            Ok(sw_rs::GeometryDocument {
                source: sw_rs::GeometrySource {
                    sha256: "synthetic".into(),
                },
                length_unit: "millimeter",
                configurations: vec![],
                model: sw_rs::GeometryModel {
                    tessellations: vec![sw_rs::Tessellation {
                        source_stream: "synthetic".into(),
                        source_offset: 0,
                        vertices: vec![[0., 0., 0.], [1000., 0., 0.], [0., 1000., 0.]],
                        triangles: vec![[0, 1, 2]],
                        strip_lengths: vec![3],
                        normals: vec![],
                    }],
                },
                limitations: vec!["synthetic fixture"],
            }),
        );
        p
    }
    #[test]
    fn repeated_hidden_occurrences_keep_source_poses_and_shared_decode() {
        let mut second = IDENTITY;
        second[12] = 2.;
        let mut p = fixture(vec![
            component("part.SLDPRT", Some(IDENTITY), false),
            component("part.SLDPRT", Some(second), true),
        ]);
        let visible = p.scene("root.SLDASM", 0, false);
        assert_eq!(visible.occurrences, 1);
        assert_eq!(visible.vertices.len(), 27);
        let all = p.scene("root.SLDASM", 0, true);
        assert_eq!(all.occurrences, 2);
        assert_eq!(all.vertices[27], 2.);
        assert_eq!(p.geometry.len(), 1);
    }
    #[test]
    fn missing_references_do_not_discard_available_siblings() {
        let mut p = fixture(vec![
            component("missing.SLDPRT", Some(IDENTITY), false),
            component("part.SLDPRT", Some(IDENTITY), false),
            component("missing-subassembly.SLDASM", Some(IDENTITY), false),
            component("part.SLDPRT", Some(IDENTITY), false),
        ]);
        let s = p.scene("root.SLDASM", 0, false);
        assert_eq!(s.missing_references, 2);
        assert_eq!(s.occurrences, 2);
        assert_eq!(s.vertices.len(), 54);
        assert_eq!(s.diagnostics.len(), 2);
        assert!(
            s.diagnostics
                .iter()
                .all(|d| d.contains("Missing reference (skipped)"))
        );
    }
    #[test]
    fn explicit_export_chains_resolve_without_fuzzy_filename_guesses() {
        let files = FileSet {
            documents: [("new.SLDPRT".into(), Vec::new().into())].into(),
            ignored_files: 0,
        };
        let mut pairs = vec![
            sw_rs::ReferenceReplacement {
                original_path: "C:/old.SLDPRT".into(),
                replacement_path: "C:/middle.SLDPRT".into(),
            },
            sw_rs::ReferenceReplacement {
                original_path: "C:/middle.SLDPRT".into(),
                replacement_path: "C:/new.SLDPRT".into(),
            },
        ];
        assert_eq!(
            resolve_reference(&files, "root.SLDASM", "C:/old.SLDPRT", &pairs),
            Resolution::Found("new.SLDPRT".into())
        );
        pairs.push(sw_rs::ReferenceReplacement {
            original_path: "C:/middle.SLDPRT".into(),
            replacement_path: "C:/other.SLDPRT".into(),
        });
        assert!(matches!(
            resolve_reference(&files, "root.SLDASM", "C:/old.SLDPRT", &pairs),
            Resolution::Ambiguous(_)
        ));
        pairs.pop();
        pairs[1].replacement_path = "C:/old.SLDPRT".into();
        assert!(matches!(
            resolve_reference(&files, "root.SLDASM", "C:/old.SLDPRT", &pairs),
            Resolution::Ambiguous(_)
        ));
        assert_eq!(
            files.resolve("root.SLDASM", "new-old.SLDPRT"),
            Resolution::Missing
        );
    }
    #[test]
    fn large_valid_scenes_are_not_cut_off_at_old_triangle_or_occurrence_caps() {
        let mut p = fixture(
            (0..2000)
                .map(|_| component("part.SLDPRT", Some(IDENTITY), false))
                .collect(),
        );
        let g = p.geometry.get_mut("part.SLDPRT").unwrap().as_mut().unwrap();
        g.model.tessellations[0].triangles = vec![[0, 1, 2]; 300];
        let s = p.scene("root.SLDASM", 0, false);
        assert_eq!(s.occurrences, 2000);
        assert_eq!(s.vertices.len() / 27, 600_000);
        assert!(s.diagnostics.is_empty());
    }
    #[test]
    fn cycles_and_unknown_transforms_remain_explicit() {
        let mut p = fixture(vec![
            component("root.SLDASM", Some(IDENTITY), false),
            component("part.SLDPRT", None, false),
        ]);
        let s = p.scene("root.SLDASM", 0, false);
        assert_eq!(s.occurrences, 0);
        assert_eq!(s.diagnostics.len(), 2);
        assert!(s.diagnostics.iter().any(|s| s.contains("Cyclic")));
        assert!(
            s.diagnostics
                .iter()
                .any(|s| s.contains("Unchecked transform"))
        );
    }
    #[test]
    fn traversal_does_not_truncate_large_assemblies() {
        let mut p = fixture(
            (0..5000)
                .map(|_| component("missing.SLDPRT", Some(IDENTITY), false))
                .collect(),
        );
        // A missing file is reported at resolution time; repeated assemblies
        // with unsupported leaves must also terminate without successful meshes.
        p.metadata.get_mut("root.SLDASM").unwrap().configurations[0].components = (0..5000)
            .map(|_| component("part.SLDPRT", Some(IDENTITY), false))
            .collect();
        p.geometry
            .insert("part.SLDPRT".into(), Err("unsupported".into()));
        let s = p.scene("root.SLDASM", 0, false);
        assert_eq!(s.visited_nodes, 5001);
        assert_eq!(s.occurrences, 0);
    }
}
