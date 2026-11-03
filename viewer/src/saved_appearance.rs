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
