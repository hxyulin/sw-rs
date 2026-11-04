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

pub(crate) type NativeCacheColors = std::collections::BTreeMap<(String, usize), Option<[u8; 3]>>;

/// Reconstruct supported saved cache colors through sequential owner/state links.
/// This is a viewer cache projection, not current B-Rep topology ownership.
pub(crate) fn archive_colors(
    doc: &sw_rs::Document,
    surfaces: &sw_rs::CachedSurfaceData,
    features: &sw_rs::FeatureCatalog,
) -> NativeCacheColors {
    use sw_rs::{NativeSubtreePayload, SurfaceIdentityResolution};
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
        graph.properties.get(&state.property_object?)
    };
    // Local face payloads are retained, but their surface-selection semantics are
    // not yet implemented. They must block lower visual guesses.
    let unknown_face = graph
        .components
        .iter()
        .any(|c| c.class_name == "moCompFace_c" && property(c).is_some());
    let bodies: Vec<_> = graph
        .components
        .iter()
        .filter(|c| c.class_name == "moCompSolidBody_c" && property(c).is_some())
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
    for table in &surfaces.records {
        let color = (|| {
            if unknown_face || unknown_scope {
                return None;
            }
            if !local_properties {
                let [document] = documents.as_slice() else {
                    return None;
                };
                return property(document)?.color_rgb;
            }
            let SurfaceIdentityResolution::Consistent { identity } =
                table.surface_references.identity_resolution()
            else {
                return None;
            };
            let owners: Vec<_> = graph
                .components
                .iter()
                .filter(|c| {
                    c.source_identity
                        .is_some_and(|p| p[0] == identity.feature_source_id)
                })
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
                    .filter(|f| f.source_id == Some(identity.feature_source_id))
                    .count()
                    != 1
                {
                    return None;
                }
                if let Some(native) = property(owner) {
                    return native.color_rgb;
                }
            }
            let mut body_color = None;
            let mut body_match = false;
            for body in &bodies {
                let reference = graph.reference_objects.get(&body.reference_object?)?;
                let NativeSubtreePayload::FaceReference { records, .. } = &reference.payload else {
                    return None;
                };
                let matches = records
                    .iter()
                    .filter_map(|p| p.header.object_index())
                    .any(|index| {
                        let Some(surface) = graph.reference_objects.get(&index) else {
                            return false;
                        };
                        let NativeSubtreePayload::Surface {
                            feature_reference,
                            nested: None,
                            raw_tail,
                        } = &surface.payload
                        else {
                            return false;
                        };
                        let Some(feature) = feature_reference
                            .header
                            .object_index()
                            .and_then(|i| graph.reference_objects.get(&i))
                        else {
                            return false;
                        };
                        let NativeSubtreePayload::FeatureReference { raw_words, .. } =
                            &feature.payload
                        else {
                            return false;
                        };
                        raw_words[0] == identity.feature_source_id
                            && raw_tail.first() == Some(&identity.local_surface_id)
                            && raw_tail[1..] == identity.trailing_fields
                            && table
                                .surface_references
                                .references
                                .iter()
                                .all(|r| r.class_name == surface.class_name)
                    });
                if matches {
                    let next = property(body)?.color_rgb;
                    if body_match && body_color != next {
                        return None;
                    }
                    body_color = next;
                    body_match = true;
                }
            }
            if body_match {
                return body_color;
            }
            // A body reference is not a complete body-to-cache partition. Its
            // unmatched tables cannot inherit the document color by exclusion.
            if !bodies.is_empty() {
                return None;
            }
            let [document] = documents.as_slice() else {
                return None;
            };
            property(document)?.color_rgb
        })();
        result.insert((table.source_stream.clone(), table.source_offset), color);
    }
    result
}
