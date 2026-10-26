// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Renderer-independent physical materials and visual appearance relationships.
//!
//! This is a document-local projection of decoded evidence, not a rendering recipe.
//! Material defaults are definitions, never fabricated face/body assignments.
use crate::{Document, MaterialDensity, SavedAppearance, SavedMaterial};
use serde::Serialize;

/// Location of a saved record in the decoded native container.
#[derive(Clone, Debug, Serialize)]
pub struct PresentationSource {
    /// Exact decoded stream name.
    pub stream: String,
    /// Exact element path within that stream.
    pub path: String,
}

/// Physical properties and identity, independent of visual appearance.
#[derive(Clone, Debug, Serialize)]
pub struct PhysicalMaterialDefinition {
    /// Exact saved material name, including its library prefix when present.
    pub name: String,
    /// Exact saved database material identifier.
    pub material_id: Option<String>,
    /// Outer-to-inner saved classifications, without inferred families.
    pub classifications: Vec<String>,
    /// Saved density token, with units unknown unless explicitly supplied.
    pub density: Option<MaterialDensity>,
    /// Index into `PresentationData::appearances`, not an applied visual override.
    pub default_appearance_index: Option<usize>,
    /// Saved definition provenance.
    pub source: PresentationSource,
}

/// Origin of a visual definition, separate from where it is applied.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppearanceOrigin {
    /// Default look linked to a physical material; not proof of an applied override.
    MaterialDefault {
        /// Index into the same document's physical-material definitions.
        material_index: usize,
    },
    /// Independently saved visual appearance; no physical material is implied.
    SavedAppearance,
}

/// Renderer-independent visual evidence; no PBR conversion or shader implementation.
#[derive(Clone, Debug, Serialize)]
pub struct AppearanceDefinition {
    /// Relationship to a physical-material definition, if any.
    pub origin: AppearanceOrigin,
    /// Exact RGB swatch token, without assuming a color space.
    pub swatch_rgb: Option<String>,
    /// Raw shader, optical and external texture-reference records.
    /// References are inert strings, not loaded texture assets.
    pub saved: Option<SavedAppearance>,
    /// Independently decoded binary visual properties, not a material-default inference.
    pub native: Option<crate::NativeAppearance>,
    /// Saved definition provenance.
    pub source: PresentationSource,
}

/// Scope of an explicit visual assignment. Entity identifiers remain opaque.
///
/// Face/feature/body identity and component occurrence paths must come from saved
/// ownership records, never geometry order or guessed filenames.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppearanceTarget {
    /// Whole native document.
    Document,
    /// A validated saved display table; never a native B-Rep face identity.
    Tessellation {
        /// Exact decoded display carrier name.
        source_stream: String,
        /// Descriptor-table byte offset within that carrier.
        source_offset: usize,
    },
    /// An assembly occurrence, distinct from its referenced document.
    Component {
        /// Saved occurrence identity within its owning assembly.
        occurrence: String,
    },
    /// A saved solid or surface body.
    Body {
        /// Opaque native body identifier.
        entity: String,
    },
    /// A saved modeling feature.
    Feature {
        /// Opaque native feature identifier.
        entity: String,
    },
    /// A saved topological face, not a tessellation triangle index.
    Face {
        /// Opaque native face identifier.
        entity: String,
    },
}

/// Saved configuration/display-state scope, distinguishing unknown from universal.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PresentationScope {
    /// Scope is not decoded; must not be treated as applying universally.
    Unknown,
    /// Explicitly applies to all configurations or display states.
    All,
    /// Applies only to the exact saved configuration or display-state name.
    Named {
        /// Exact native name, without case folding or configuration inheritance.
        name: String,
    },
}

/// Native precedence layer of an appearance link, independent of its target identity.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppearanceLayer {
    /// Saved whole-document default.
    Document,
    /// Body-default appearance in a saved display cache.
    CachedBody,
    /// Face-local property slot in a saved display cache.
    CachedFace,
}

/// Explicit applied visual assignment, independent of physical-material assignment.
#[derive(Clone, Debug, Serialize)]
pub struct AppearanceBinding {
    /// Where this assignment sits in the supported native appearance hierarchy.
    pub layer: AppearanceLayer,
    /// Target owned by this document or assembly.
    pub target: AppearanceTarget,
    /// Saved configuration scope, preserving unknown versus explicitly universal.
    pub configuration: PresentationScope,
    /// Saved display-state scope, independent of configuration scope.
    pub display_state: PresentationScope,
    /// Resolved document-local appearance index; None preserves an unresolved link.
    pub appearance_index: Option<usize>,
    /// Saved textual reference, or an explicit decoded-stream locator for binary links.
    pub reference: String,
    /// Saved assignment provenance.
    pub source: PresentationSource,
}

/// Explicit physical assignment to a saved configuration, including unresolved links.
#[derive(Clone, Debug, Serialize)]
pub struct PhysicalMaterialBinding {
    /// Exact saved configuration name.
    pub configuration: String,
    /// Exact saved material key.
    pub reference: String,
    /// Unique document-local material index; None means incomplete or ambiguous.
    pub material_index: Option<usize>,
    /// Saved assignment provenance.
    pub source: PresentationSource,
}

/// Decoding coverage, so empty collections cannot be mistaken for absent overrides.
#[derive(Clone, Debug, Serialize)]
pub struct PresentationCoverage {
    /// Whether a supported material tree was decoded, including an empty tree.
    pub material_tree_decoded: bool,
    /// Whether body-level physical assignments are decoded (currently false).
    pub body_material_assignments_decoded: bool,
    /// Whether complete native applied visual ownership is decoded (currently false).
    pub appearance_bindings_decoded: bool,
    /// Whether the complete display-state table/graph is decoded (currently false).
    pub display_states_decoded: bool,
    /// Whether native texture projections/UV mappings are decoded (currently false).
    pub texture_mappings_decoded: bool,
    /// At least one uniquely scoped whole-document visual binding was decoded.
    pub document_appearance_bindings_decoded: bool,
    /// Supported cached property slots were inspected; not complete native ownership.
    pub cached_appearance_bindings_decoded: bool,
    /// At least one framed configuration/display-state key was decoded.
    pub display_state_keys_decoded: bool,
}

/// A renderer-independent, document-local snapshot. Indices never cross documents.
///
/// Recompute after changing public document records. Original parser records remain
/// authoritative; renderer-specific approximations belong outside this structure.
#[derive(Clone, Debug, Serialize)]
pub struct PresentationData {
    /// Versioned projection schema, independent of native-file and renderer versions.
    pub schema: &'static str,
    /// Saved physical definitions, retaining source order and duplicates.
    pub physical_materials: Vec<PhysicalMaterialDefinition>,
    /// Saved physical configuration references, retaining unresolved records.
    pub physical_bindings: Vec<PhysicalMaterialBinding>,
    /// Visual definitions; their presence alone never establishes applied ownership.
    pub appearances: Vec<AppearanceDefinition>,
    /// Supported scoped document/cache assignments, without inferred native topology.
    pub appearance_bindings: Vec<AppearanceBinding>,
    /// Known and unknown decoding capabilities.
    pub coverage: PresentationCoverage,
    /// Saved visual-state keys, retaining ambiguous/unknown configuration scope.
    pub display_states: Vec<crate::SavedDisplayState>,
    /// Decoded source identities which cannot yet be linked to native entities.
    pub unresolved_visual_bindings: Vec<crate::UnresolvedVisualBinding>,
    /// Unsupported layouts and incomplete ownership, with decoded-source offsets.
    pub visual_diagnostics: Vec<crate::VisualDiagnostic>,
}

impl Document {
    /// Separate saved physical and visual evidence without selecting a renderer.
    pub fn presentation(&self) -> PresentationData {
        let mut result = PresentationData {
            schema: "sw-rs-presentation-v1",
            physical_materials: Vec::new(),
            physical_bindings: Vec::new(),
            appearances: Vec::new(),
            appearance_bindings: Vec::new(),
            display_states: self.visuals.display_states.clone(),
            unresolved_visual_bindings: Vec::new(),
            visual_diagnostics: self.visuals.diagnostics.clone(),
            coverage: PresentationCoverage {
                material_tree_decoded: self.materials.is_some(),
                body_material_assignments_decoded: false,
                appearance_bindings_decoded: false,
                display_states_decoded: false,
                texture_mappings_decoded: false,
                document_appearance_bindings_decoded: !self.visuals.bindings.is_empty(),
                cached_appearance_bindings_decoded: false,
                display_state_keys_decoded: !self.visuals.display_states.is_empty(),
            },
        };
        if let Some(materials) = &self.materials {
            let source = |path: &str| PresentationSource {
                stream: materials.source_stream.clone(),
                path: path.into(),
            };
            for (material_index, definition) in materials.definitions.iter().enumerate() {
                let default_appearance_index =
                    if definition.appearance.is_some() || definition.saved_swatch_rgb.is_some() {
                        let index = result.appearances.len();
                        result.appearances.push(AppearanceDefinition {
                            origin: AppearanceOrigin::MaterialDefault { material_index },
                            swatch_rgb: definition.saved_swatch_rgb.clone(),
                            saved: definition.appearance.clone(),
                            native: None,
                            source: source(&definition.source_path),
                        });
                        Some(index)
                    } else {
                        None
                    };
                result.physical_materials.push(PhysicalMaterialDefinition {
                    name: definition.name.clone(),
                    material_id: definition.material_id.clone(),
                    classifications: definition.classifications.clone(),
                    density: definition.density.clone(),
                    default_appearance_index,
                    source: source(&definition.source_path),
                });
            }
            for binding in &materials.assignments {
                result.physical_bindings.push(PhysicalMaterialBinding {
                    configuration: binding.configuration.clone(),
                    reference: binding.material_reference.clone(),
                    material_index: unique_material(
                        materials,
                        &binding.configuration,
                        &binding.material_reference,
                    ),
                    source: source(&binding.source_path),
                });
            }
        }
        result.append_visuals(&self.visuals);
        result
    }

    /// Resolve physical material evidence only for a unique exact configuration and
    /// unique consistent saved reference. This does not resolve applied appearances.
    pub fn physical_material_for_configuration(&self, name: &str) -> Option<&SavedMaterial> {
        if name.is_empty()
            || self
                .configurations
                .iter()
                .filter(|c| c.name == name)
                .count()
                != 1
        {
            return None;
        }
        let configuration = self.configurations.iter().find(|c| c.name == name)?;
        let binding = configuration.material.as_ref()?;
        if binding.configuration != name {
            return None;
        }
        let materials = self.materials.as_ref()?;
        let mut bindings = materials
            .assignments
            .iter()
            .filter(|b| b.configuration == name);
        let saved = bindings.next()?;
        if bindings.next().is_some() || saved.material_reference != binding.material_reference {
            return None;
        }
        let index = unique_material(materials, name, &binding.material_reference)?;
        if binding.definition_index != Some(index) || saved.definition_index != Some(index) {
            return None;
        }
        materials.definitions.get(index)
    }
}

fn unique_material(
    materials: &crate::MaterialData,
    configuration: &str,
    reference: &str,
) -> Option<usize> {
    if configuration.is_empty() || reference.is_empty() {
        return None;
    }
    let mut matches = materials.definitions.iter().enumerate().filter(|(_, d)| {
        !d.name.is_empty()
            && match &d.material_id {
                Some(id) => format!("{}|{id}", d.name) == reference,
                None => d.name == reference,
            }
    });
    let (index, _) = matches.next()?;
    matches.next().is_none().then_some(index)
}

impl PresentationData {
    pub(crate) fn append_visuals(&mut self, visuals: &crate::VisualData) {
        let base = self.appearances.len();
        self.appearances.extend(visuals.definitions.iter().cloned());
        self.appearance_bindings
            .extend(visuals.bindings.iter().cloned().map(|mut binding| {
                binding.appearance_index = binding.appearance_index.map(|index| base + index);
                binding
            }));
        self.unresolved_visual_bindings
            .extend(
                visuals
                    .unresolved_bindings
                    .iter()
                    .cloned()
                    .map(|mut binding| {
                        binding.appearance_index =
                            binding.appearance_index.map(|index| base + index);
                        binding
                    }),
            );
    }
}

/// Result of selecting a property stored with a particular display-cache table.
/// This is not a complete effective native appearance: external feature/component
/// overrides and native texture mappings may remain undecoded.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CachedPropertyResolution {
    /// No property for this exact table was decoded; not proof of no appearance.
    NotDecoded,
    /// Exactly one definition at the highest decoded cache layer.
    Resolved {
        /// Document-local appearance definition index.
        appearance_index: usize,
        /// Supported source cache precedence layer.
        layer: AppearanceLayer,
    },
    /// Conflicting definitions at the highest decoded cache layer.
    Ambiguous {
        /// Layer whose conflict prevents a lower-layer fallback.
        layer: AppearanceLayer,
    },
    /// An incomplete/invalid definition link at the highest decoded cache layer.
    Unresolved {
        /// Layer whose unresolved link prevents a lower-layer fallback.
        layer: AppearanceLayer,
    },
}

impl PresentationData {
    /// Select a decoded cache property by exact carrier and descriptor offset.
    /// Face-local properties outrank cached body defaults. Conflicts or unresolved
    /// higher-layer records block lower-layer fallback; table order is never an ID.
    pub fn cached_property(&self, stream: &str, offset: usize) -> CachedPropertyResolution {
        let matches = self.appearance_bindings.iter().filter(|binding| {
            matches!(&binding.target, AppearanceTarget::Tessellation {source_stream,source_offset}
                if source_stream == stream && *source_offset == offset)
        }).collect::<Vec<_>>();
        let layer = if matches
            .iter()
            .any(|b| b.layer == AppearanceLayer::CachedFace)
        {
            AppearanceLayer::CachedFace
        } else if matches
            .iter()
            .any(|b| b.layer == AppearanceLayer::CachedBody)
        {
            AppearanceLayer::CachedBody
        } else {
            return CachedPropertyResolution::NotDecoded;
        };
        let mut indices = std::collections::BTreeSet::new();
        for binding in matches.iter().filter(|b| b.layer == layer) {
            match binding.appearance_index {
                Some(index) if index < self.appearances.len() => {
                    indices.insert(index);
                }
                _ => return CachedPropertyResolution::Unresolved { layer },
            }
        }
        let mut indices = indices.into_iter();
        match (indices.next(), indices.next()) {
            (Some(appearance_index), None) => CachedPropertyResolution::Resolved {
                appearance_index,
                layer,
            },
            _ => CachedPropertyResolution::Ambiguous { layer },
        }
    }

    /// Look up only the saved document-level default for an exact configuration and
    /// display-state name. This does not resolve face/body/component overrides.
    /// Ambiguous, unknown-scoped, or invalid document links return None.
    pub fn document_appearance_for_state(
        &self,
        configuration: &str,
        display_state: &str,
    ) -> Option<&AppearanceDefinition> {
        if configuration.is_empty() || display_state.is_empty() {
            return None;
        }
        let matches = |scope: &PresentationScope, name: &str| match scope {
            PresentationScope::Unknown => None,
            PresentationScope::All => Some(true),
            PresentationScope::Named { name: saved } => Some(saved == name),
        };
        let mut index = None;
        for binding in self.appearance_bindings.iter().filter(|b| {
            b.layer == AppearanceLayer::Document && matches!(b.target, AppearanceTarget::Document)
        }) {
            let configuration_matches = matches(&binding.configuration, configuration);
            let display_matches = matches(&binding.display_state, display_state);
            if configuration_matches == Some(false) || display_matches == Some(false) {
                continue;
            }
            if !configuration_matches? || !display_matches? {
                continue;
            }
            let next = binding.appearance_index?;
            self.appearances.get(next)?;
            if index.is_some_and(|index| index != next) {
                return None;
            }
            index = Some(next);
        }
        self.appearances.get(index?)
    }
}
