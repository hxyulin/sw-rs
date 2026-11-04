// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Shared selection of supported saved colors and bitmap appearances.
use sw_rs::{NativeAppearance, PresentationData, PresentationScope};

fn document_native<'a>(
    presentation: &'a PresentationData,
    configuration: &str,
) -> Option<&'a NativeAppearance> {
    let states: std::collections::BTreeSet<_> = presentation
        .display_states
        .iter()
        .filter_map(|s| match (&s.configuration, &s.display_state) {
            (PresentationScope::Named { name }, PresentationScope::Named { name: state })
                if name == configuration =>
            {
                Some(state.as_str())
            }
            _ => None,
        })
        .collect();
    // No active-state guess when multiple or unknown scopes are saved.
    if states.len() != 1 {
        return None;
    }
    presentation
        .document_appearance_for_state(configuration, states.first().copied().unwrap_or_default())
        .and_then(|a| a.native.as_ref())
}

pub(crate) fn mesh_native<'a>(
    presentation: &'a PresentationData,
    mesh: &sw_rs::Tessellation,
    configuration: &str,
) -> Option<&'a NativeAppearance> {
    match presentation.cached_property(&mesh.source_stream, mesh.source_offset) {
        sw_rs::CachedPropertyResolution::NotDecoded => document_native(presentation, configuration),
        sw_rs::CachedPropertyResolution::Resolved {
            appearance_index, ..
        } => presentation
            .appearances
            .get(appearance_index)
            .and_then(|a| a.native.as_ref()),
        // A conflicting, invalid, or sentinel cache property cannot inherit a document color.
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CacheAppearance {
    Property(u32),
    Inherited {
        color: [u8; 3],
        surface: Option<u32>,
    },
}
pub(crate) type NativeCacheAppearances =
    std::collections::BTreeMap<(String, usize), Option<CacheAppearance>>;

/// Reconstruct supported saved cache appearance links through sequential owner/state links.
/// This is a viewer cache projection, not current B-Rep topology ownership.
pub(crate) fn archive_appearances(
    doc: &sw_rs::Document,
    surfaces: &sw_rs::CachedSurfaceData,
    features: &sw_rs::FeatureCatalog,
) -> NativeCacheAppearances {
    use sw_rs::NativeSubtreePayload;
    let mut result = std::collections::BTreeMap::new();
    let Some(graph) = doc.visuals.archive.as_ref().filter(|g| g.complete) else {
        return result;
    };
    let rows: Vec<_> = graph
        .configuration_rows
        .iter()
        .filter(|r| r[0] == surfaces.configuration_index)
        .collect();
    let [row] = rows.as_slice() else {
        return result;
    };
    let key_id = row[1];
    if !graph
        .keys
        .iter()
        .any(|k| k.id == key_id && k.configuration_index == surfaces.configuration_index)
    {
        return result;
    }
    let property = |owner: &sw_rs::VisualArchiveComponent| {
        let state = owner.states.iter().find(|s| s.key_id == key_id)?;
        state
            .property_object
            .filter(|id| graph.properties.contains_key(id))
    };
    // Controlled six-face saves establish the first reference as the selected
    // face. Later entries describe its context, not additional selected faces.
    let faces: Vec<_> = graph
        .components
        .iter()
        .filter(|c| c.class_name == "moCompFace_c" && property(c).is_some())
        .collect();
    let face_identity = |owner: &sw_rs::VisualArchiveComponent| {
        let reference = graph.reference_objects.get(&owner.reference_object?)?;
        let NativeSubtreePayload::FaceReference { records, .. } = &reference.payload else {
            return None;
        };
        native_surface_path(graph, records.first()?.header.object_index()?, 0)
    };
    let mut face_properties = std::collections::BTreeMap::new();
    for face in &faces {
        if let Some(key) = face_identity(face) {
            let selected = property(face);
            face_properties
                .entry(key)
                .and_modify(|previous| {
                    if *previous != selected {
                        *previous = None;
                    }
                })
                .or_insert(selected);
        }
    }
    let mut unknown_face_sources = std::collections::BTreeSet::new();
    let mut unknown_face = false;
    for face in &faces {
        if face_identity(face).is_some() {
            continue;
        }
        let mut pointer = face
            .reference_object
            .and_then(|i| graph.reference_objects.get(&i))
            .and_then(|o| match &o.payload {
                NativeSubtreePayload::FaceReference { records, .. } => {
                    records.first()?.header.object_index()
                }
                _ => None,
            });
        let mut depth = 0;
        while let Some(index) = pointer {
            depth += 1;
            if depth > 64 {
                unknown_face = true;
                break;
            }
            let Some(surface) = graph.reference_objects.get(&index) else {
                unknown_face = true;
                break;
            };
            let NativeSubtreePayload::Surface {
                feature_reference,
                nested,
                ..
            } = &surface.payload
            else {
                unknown_face = true;
                break;
            };
            let feature = feature_reference
                .header
                .object_index()
                .and_then(|i| graph.reference_objects.get(&i));
            let Some(NativeSubtreePayload::FeatureReference { raw_words, .. }) =
                feature.map(|o| &o.payload)
            else {
                unknown_face = true;
                break;
            };
            unknown_face_sources.insert(raw_words[0]);
            pointer = nested.as_ref().and_then(|p| p.header.object_index());
        }
        if depth == 0 {
            unknown_face = true;
        }
    }
    let bodies: Vec<_> = graph
        .components
        .iter()
        .filter(|c| {
            matches!(
                c.class_name.as_str(),
                "moCompSolidBody_c" | "moCompSurfaceBody_c"
            ) && property(c).is_some()
        })
        .collect();
    let documents: Vec<_> = graph
        .components
        .iter()
        .filter(|c| c.class_name == "moUnitComponent_c")
        .collect();
    let local_properties = graph
        .components
        .iter()
        .any(|c| c.class_name != "moUnitComponent_c" && property(c).is_some());
    let unknown_scope = graph
        .components
        .iter()
        .any(|c| !c.states.iter().any(|s| s.key_id == key_id));
    let mut body_signatures =
        std::collections::BTreeMap::<SurfacePath, std::collections::BTreeSet<[u8; 3]>>::new();
    let mut signature_properties =
        std::collections::BTreeMap::<SurfacePath, std::collections::BTreeSet<u32>>::new();
    let mut group_properties =
        std::collections::BTreeMap::<u32, std::collections::BTreeSet<u32>>::new();
    let mut current_ids = std::collections::BTreeSet::new();
    let mut group_colors =
        std::collections::BTreeMap::<u32, std::collections::BTreeSet<[u8; 3]>>::new();
    for owner in graph.components.iter().filter(|c| {
        matches!(
            c.class_name.as_str(),
            "moCompSolidBody_c" | "moCompSurfaceBody_c"
        )
    }) {
        let Some(id) = owner
            .raw_common
            .get(7..11)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
        else {
            continue;
        };
        current_ids.insert(id);
        let Some(property_id) = property(owner) else {
            continue;
        };
        let Some(color) = graph.properties[&property_id].render_color_rgb() else {
            continue;
        };
        group_colors.entry(id).or_default().insert(color);
        group_properties.entry(id).or_default().insert(property_id);
        let reference = owner
            .reference_object
            .and_then(|i| graph.reference_objects.get(&i));
        if let Some(NativeSubtreePayload::FaceReference { records, .. }) =
            reference.map(|r| &r.payload)
        {
            for record in records {
                if let Some(path) = record
                    .header
                    .object_index()
                    .and_then(|i| native_surface_path(graph, i, 0))
                {
                    signature_properties
                        .entry(path.clone())
                        .or_default()
                        .insert(property_id);
                    body_signatures.entry(path).or_default().insert(color);
                }
            }
        }
    }
    // Historical body references can identify an inherited color on a newly
    // generated body. A checked carrier-local style reference shares that color
    // with its siblings. Color equality never licenses inheriting bitmap/UV data.
    for table in &surfaces.records {
        let Some(id) = table.saved_body_id.filter(|id| !current_ids.contains(id)) else {
            continue;
        };
        for reference in &table.surface_references.references {
            if let Some(ids) = cache_surface_path(&reference.source.value, 0)
                .and_then(|p| signature_properties.get(&p))
            {
                group_properties
                    .entry(id)
                    .or_default()
                    .extend(ids.iter().copied());
            }
            if let Some(colors) =
                cache_surface_path(&reference.source.value, 0).and_then(|p| body_signatures.get(&p))
            {
                group_colors
                    .entry(id)
                    .or_default()
                    .extend(colors.iter().copied());
            }
        }
    }
    let mut style_colors =
        std::collections::BTreeMap::<(&str, u16), std::collections::BTreeSet<[u8; 3]>>::new();
    let mut style_properties =
        std::collections::BTreeMap::<(&str, u16), std::collections::BTreeSet<u32>>::new();
    for table in &surfaces.records {
        if let (Some(id), Some(style)) = (table.saved_body_id, table.saved_body_style)
            && let Some(colors) = group_colors.get(&id).filter(|c| c.len() == 1)
        {
            if let Some(ids) = group_properties.get(&id) {
                style_properties
                    .entry((&table.source_stream, style))
                    .or_default()
                    .extend(ids.iter().copied());
            }
            style_colors
                .entry((&table.source_stream, style))
                .or_default()
                .extend(colors.iter().copied());
        }
    }
    for table in &surfaces.records {
        let selected = (|| {
            if unknown_face || unknown_scope {
                return None;
            }
            if !local_properties {
                let [document] = documents.as_slice() else {
                    return None;
                };
                return property(document).map(CacheAppearance::Property);
            }
            let references = &table.surface_references.references;
            let paths: Vec<_> = references
                .iter()
                .map(|r| cache_surface_path(&r.source.value, 0))
                .collect::<Option<_>>()?;
            let path = paths.first()?;
            if paths
                .iter()
                .any(|p| unknown_face_sources.contains(&p.source_id()))
            {
                return None;
            }
            // A cache table may retain several historical names for one face.
            // Missing annotations on an alias do not negate an explicit match;
            // two matched appearances must still agree exactly.
            let mut face_property = None;
            for selected in paths.iter().filter_map(|p| face_properties.get(p)) {
                let selected = (*selected)?;
                if face_property.is_some_and(|previous| previous != selected) {
                    return None;
                }
                face_property = Some(selected);
            }
            if face_property.is_some() {
                return face_property.map(CacheAppearance::Property);
            }
            let source_id = path.source_id();
            let aliases = paths.iter().any(|p| p != path);
            let source_aliases = paths.iter().any(|p| p.source_id() != source_id);
            if source_aliases
                && graph.components.iter().any(|owner| {
                    property(owner).is_some()
                        && owner
                            .source_identity
                            .is_some_and(|id| paths.iter().any(|p| p.source_id() == id[0]))
                })
            {
                return None;
            }
            let owners: Vec<_> = graph
                .components
                .iter()
                .filter(|c| !source_aliases && c.source_identity.is_some_and(|p| p[0] == source_id))
                .collect();
            if owners.len() > 1 {
                return None;
            }
            if let [owner] = owners.as_slice() {
                if owner
                    .source_identity
                    .is_some_and(|p| p[1] == 0 || p[1] == u32::MAX)
                {
                    return None;
                }
                // Require a unique feature in this exact native document. Names,
                // table order and cross-document filename guesses are never IDs.
                if features
                    .records
                    .iter()
                    .filter(|f| f.source_id == Some(source_id))
                    .count()
                    != 1
                {
                    return None;
                }
                if let Some(native) = property(owner) {
                    return Some(CacheAppearance::Property(native));
                }
            }
            if let Some(body_id) = table.saved_body_id {
                let mut selected = None;
                for body in &bodies {
                    let current = u32::from_le_bytes(body.raw_common.get(7..11)?.try_into().ok()?);
                    if current == body_id {
                        let candidate = property(body)?;
                        if selected.is_some_and(|previous| previous != candidate) {
                            return None;
                        }
                        selected = Some(candidate);
                    }
                }
                if selected.is_some() {
                    return selected.map(CacheAppearance::Property);
                }
                let inherited = table.saved_body_style.and_then(|style| {
                    style_colors
                        .get(&(table.source_stream.as_str(), style))
                        .or_else(|| group_colors.get(&body_id))
                });
                if let Some(colors) = inherited {
                    if colors.len() != 1 {
                        return None;
                    }
                    let ids = table.saved_body_style.and_then(|style| {
                        style_properties
                            .get(&(table.source_stream.as_str(), style))
                            .or_else(|| group_properties.get(&body_id))
                    });
                    // A shared style may carry optical settings only when every
                    // anchor agrees. Bitmap ownership remains independent.
                    let surface = ids.and_then(|ids| {
                        let first = *ids.first()?;
                        let expected =
                            crate::appearance::native_surface(&graph.properties[&first])?;
                        ids.iter()
                            .all(|id| {
                                crate::appearance::native_surface(&graph.properties[id]).as_ref()
                                    == Some(&expected)
                            })
                            .then_some(first)
                    });
                    return Some(CacheAppearance::Inherited {
                        color: *colors.first()?,
                        surface,
                    });
                }
                if table.saved_body_style.is_some() {
                    return None;
                }
                let [document] = documents.as_slice() else {
                    return None;
                };
                return property(document).map(CacheAppearance::Property);
            }
            if aliases {
                return None;
            }
            let mut body_property = None;
            let mut body_match = false;
            for body in &bodies {
                let reference = graph.reference_objects.get(&body.reference_object?)?;
                let NativeSubtreePayload::FaceReference { records, .. } = &reference.payload else {
                    return None;
                };
                let matches = records
                    .iter()
                    .filter_map(|p| p.header.object_index())
                    .any(|index| native_surface_path(graph, index, 0).as_ref() == Some(path));
                if matches {
                    let next = property(body);
                    if body_match && body_property != next {
                        return None;
                    }
                    body_property = next;
                    body_match = true;
                }
            }
            if body_match {
                return body_property.map(CacheAppearance::Property);
            }
            // A body reference is not a complete body-to-cache partition. Its
            // unmatched tables cannot inherit the document color by exclusion.
            if !bodies.is_empty() {
                return None;
            }
            let [document] = documents.as_slice() else {
                return None;
            };
            property(document).map(CacheAppearance::Property)
        })();
        result.insert((table.source_stream.clone(), table.source_offset), selected);
    }
    result
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SurfacePath {
    Leaf(String, sw_rs::SavedSurfaceIdentity),
    Indirect(u32, Box<SurfacePath>),
    Pattern(String, u32, Vec<u32>, Box<SurfacePath>),
}
impl SurfacePath {
    fn source_id(&self) -> u32 {
        match self {
            Self::Leaf(_, id) => id.feature_source_id,
            Self::Indirect(id, _) | Self::Pattern(_, id, _, _) => *id,
        }
    }
}

fn cache_surface_path(text: &str, depth: usize) -> Option<SurfacePath> {
    if depth > 64 {
        return None;
    }
    let text = text.trim();
    let (class, rest) = text.split_once(',')?;
    if matches!(
        class,
        "moIndirectSurfIdRep_c" | "moMirPatternSurfIdRep_c" | "moCirPatternSurfIdRep2_c"
    ) {
        let (source, rest) = rest.split_once(',')?;
        let source: u32 = source.parse().ok()?;
        if source == 0 || source == u32::MAX {
            return None;
        }
        let (parameters, nested) = if class == "moCirPatternSurfIdRep2_c" {
            let (a, rest) = rest.split_once(',')?;
            let (b, nested) = rest.split_once(',')?;
            (vec![a.parse().ok()?, b.parse().ok()?], nested)
        } else {
            (Vec::new(), rest)
        };
        let nested = nested.strip_suffix(',').unwrap_or(nested).trim();
        let inner = nested.strip_prefix('<')?.strip_suffix('>')?;
        let nested = Box::new(cache_surface_path(inner, depth + 1)?);
        return Some(if class == "moIndirectSurfIdRep_c" {
            SurfacePath::Indirect(source, nested)
        } else {
            SurfacePath::Pattern(class.into(), source, parameters, nested)
        });
    }
    let parsed = sw_rs::SavedSurfaceReference::parse(&sw_rs::NativeString {
        offset: 0,
        value: text.into(),
    })?;
    Some(SurfacePath::Leaf(parsed.class_name, parsed.identity?))
}

fn native_surface_path(
    graph: &sw_rs::VisualArchive,
    index: u32,
    depth: usize,
) -> Option<SurfacePath> {
    use sw_rs::NativeSubtreePayload;
    if depth > 64 {
        return None;
    }
    let surface = graph.reference_objects.get(&index)?;
    let NativeSubtreePayload::Surface {
        feature_reference,
        nested,
        raw_tail,
    } = &surface.payload
    else {
        return None;
    };
    let feature = graph
        .reference_objects
        .get(&feature_reference.header.object_index()?)?;
    let NativeSubtreePayload::FeatureReference { raw_words, .. } = &feature.payload else {
        return None;
    };
    let source = raw_words[0];
    if source == 0 || source == u32::MAX {
        return None;
    }
    if surface.class_name == "moIndirectSurfIdRep_c" && raw_tail.is_empty() {
        return Some(SurfacePath::Indirect(
            source,
            Box::new(native_surface_path(
                graph,
                nested.as_ref()?.header.object_index()?,
                depth + 1,
            )?),
        ));
    }
    if matches!(
        surface.class_name.as_str(),
        "moMirPatternSurfIdRep_c" | "moCirPatternSurfIdRep2_c"
    ) {
        let expected = if surface.class_name == "moCirPatternSurfIdRep2_c" {
            2
        } else {
            0
        };
        if raw_tail.len() != expected {
            return None;
        }
        return Some(SurfacePath::Pattern(
            surface.class_name.clone(),
            source,
            raw_tail.clone(),
            Box::new(native_surface_path(
                graph,
                nested.as_ref()?.header.object_index()?,
                depth + 1,
            )?),
        ));
    }
    if nested.is_some() {
        return None;
    }
    Some(SurfacePath::Leaf(
        surface.class_name.clone(),
        sw_rs::SavedSurfaceIdentity {
            feature_source_id: source,
            local_surface_id: *raw_tail.first()?,
            trailing_fields: raw_tail[1..].to_vec(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patterned_surface_paths_keep_instance_parameters_and_nested_owners() {
        let text = "moCirPatternSurfIdRep2_c,9,3,0,<moFromSktEnt3IntSurfIdRep_c,7,1,-1,0>,";
        let path = cache_surface_path(text, 0).unwrap();
        assert_eq!(path.source_id(), 9);
        assert_ne!(
            cache_surface_path(&text.replace(",3,0,<", ",4,0,<"), 0).unwrap(),
            path
        );
        let mirrored = cache_surface_path(
            "moMirPatternSurfIdRep_c,9,<moFromSktEnt3IntSurfIdRep_c,7,1,-1,0>,",
            0,
        )
        .unwrap();
        assert_ne!(path, mirrored);
        assert!(
            cache_surface_path(
                "moCirPatternSurfIdRep2_c,9,3,<moFromSktEnt3IntSurfIdRep_c,7,1,-1,0>,",
                0
            )
            .is_none()
        );
    }
    #[test]
    fn indirect_surface_paths_preserve_both_source_ids_class_and_suffix() {
        let path = cache_surface_path(
            "moIndirectSurfIdRep_c,9,<moEndFace3IntSurfIdRep_c,7,1,-1,0>, \n",
            0,
        )
        .unwrap();
        assert_eq!(path.source_id(), 9);
        for other in [
            "moIndirectSurfIdRep_c,8,<moEndFace3IntSurfIdRep_c,7,1,-1,0>,",
            "moIndirectSurfIdRep_c,9,<moEndFace3IntSurfIdRep_c,8,1,-1,0>,",
            "moIndirectSurfIdRep_c,9,<moEndFace3IntSurfIdRep_c,7,1,0,0>,",
        ] {
            assert_ne!(cache_surface_path(other, 0).unwrap(), path);
        }
        assert!(
            cache_surface_path(
                "moIndirectSurfIdRep_c,9,<moEndFace3IntSurfIdRep_c,7,1,-1,0>,garbage",
                0
            )
            .is_none()
        );
        assert!(
            cache_surface_path(
                "moIndirectSurfIdRep_c,0,<moEndFace3IntSurfIdRep_c,7,1,-1,0>,",
                0
            )
            .is_none()
        );
    }
}
