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

/// Explicit applied visual assignment, independent of physical-material assignment.
#[derive(Clone, Debug, Serialize)]
pub struct AppearanceBinding {
    /// Target owned by this document or assembly.
    pub target: AppearanceTarget,
    /// Saved configuration scope, preserving unknown versus explicitly universal.
    pub configuration: PresentationScope,
    /// Saved display-state scope, independent of configuration scope.
    pub display_state: PresentationScope,
    /// Resolved document-local appearance index; None preserves an unresolved link.
    pub appearance_index: Option<usize>,
    /// Exact saved appearance reference, including unresolved references.
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
    /// Whether applied visual ownership is decoded (currently false).
    pub appearance_bindings_decoded: bool,
    /// Whether display-state visual scopes are decoded (currently false).
    pub display_states_decoded: bool,
    /// Whether native texture projections/UV mappings are decoded (currently false).
    pub texture_mappings_decoded: bool,
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
    /// Explicit visual assignments; currently empty because ownership is not decoded.
    pub appearance_bindings: Vec<AppearanceBinding>,
    /// Known and unknown decoding capabilities.
    pub coverage: PresentationCoverage,
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
            coverage: PresentationCoverage {
                material_tree_decoded: self.materials.is_some(),
                body_material_assignments_decoded: false,
                appearance_bindings_decoded: false,
                display_states_decoded: false,
                texture_mappings_decoded: false,
            },
        };
        let Some(materials) = &self.materials else {
            return result;
        };
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
