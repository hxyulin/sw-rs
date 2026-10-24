<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Materials and presentation

`Document::materials` retains material definitions and configuration references.
`Configuration::material` selects only a unique exact saved reference;
`definition_index` resolves a unique definition within that document. Definitions
alone do not prove assignment, and no parent-configuration or body assignment is
inferred. Density retains its raw token and explicitly saved units, if any.
Each definition also retains saved shader records, texture references and optical
attributes with their source paths. These are metadata; external paths are never
opened. Duplicate or unknown appearance records remain available for inspection.
Physical material labels and display appearances are separate. The viewer maps
explicit aluminum/carbon-fiber assignments to its own procedural brushed-metal
and woven-composite presets; other assignments use a generic saved-swatch preset.
These are approximate visualizations, not recovered SolidWorks shaders or UVs.
Unassigned parts retain diagnostic colors, and a toolbar control switches the
whole scene back to diagnostic colors.
The **Appearance style** selector defaults to approximate saved material
appearances and can switch to physical material-family presets. Saved PhotoWorks
identities take precedence over generic realtime shader hints. This does not
recover final face/body overrides from binary visual states.

## Renderer-independent model

`Document::presentation()` projects decoded records into `sw-rs-presentation-v1`:
physical definitions/bindings and visual definitions/bindings are separate. Each
physical definition may reference a material-default appearance. This relationship
is not an applied face/body/component override. Source stream/element paths and
unresolved references are retained; indices are local to one document snapshot.
Recompute the projection after mutating a document's public source records.
`physical_material_for_configuration(name)` validates exact, unique configuration
and definition references before returning physical evidence.

Appearance targets distinguish document, component occurrence, body, feature,
face and **cached tessellation** identities. Cached tables use their carrier name
and exact descriptor byte offset; they never impersonate native B-Rep faces.
Configuration and display-state scopes distinguish unknown, universal and named
states. Binary source locations use `@byte[N]` within the decoded stream.

`Document::visuals` decodes supported `ThirdPtyStore/VisualStates` profiles:
framed UTF-16 state keys; packed color words; variable-length texture/material
strings; six legacy optical values (and exact hexadecimal bits); supported shader
identifiers and appearance/surface-finish references. Conditional texture blocks
and mapping numbers are retained raw. Their axes, projection types and physical
units are not established, so `texture_mappings_decoded` remains false.
`shader_parameters` additionally preserves supported raw shader groups.
`NativeAppearanceExtension::texture_mapping_hypothesis()` offers an opt-in,
explicitly provisional grouping of plausible directions/origin/repeat dimensions;
negative/default dimensions remain unresolved. It does not produce a verified UV
transform. See [appearance research and verification TODOs](appearance-research.md).

Verified part-default class chains can produce document-level bindings scoped to
one uniquely matched configuration/display-state key. Ambiguous prefixes,
multiple keys, unsupported versions, standalone definitions and assembly context
chains do not become whole-document bindings. Feature source IDs/timestamps and
some duplicated body identity words are preserved as unresolved evidence until
current history/topology associations can be verified. This is a partial profile,
not a general MFC object-graph decoder or complete display-state inventory.

`GeometryDocument::presentation` adds face-local and body-default property slots
from the validated display cache without changing mesh order or geometry.
`cached_property(stream, offset)` selects only the stored cache property: a
face-local property outranks a cached body default, and conflicting/unresolved
higher-layer links block fallback. `document_appearance_for_state(config, state)`
looks up only a document default. Neither method claims the complete effective
native appearance when component/feature/face ownership is still unknown.

The viewer exports `presentations` keyed by document path, including embedded
paths. Hierarchy occurrences refer to that path and document-local
`physical_material_index`/`material_default_appearance_index` when resolved.
Repeated occurrences share a catalog. Existing `materials`, `material_index`
and `batches` remain the viewer's approximate rendering adapter; recovered native
links do not silently replace that adapter while higher-level ownership remains
incomplete. The WASM `presentation_json(bytes, filename)` entry point exposes the
metadata projection; geometry JSON additionally exposes cache-local bindings.

Use `mbx run --release --features archive --example inspect_appearances -- ARCHIVE --cached` to audit locally, with per-file source hashes and explicit unsupported
containers/geometry. Add `--hypotheses` for raw mapping records and provisional
interpretations. Original CAD and reports stay outside Git. Legacy OLE
containers remain unsupported by the runtime.

The next verification needs controlled SolidWorks saves: an unchanged baseline,
one part/body/feature/face override at a time, an override on just one of two
repeated assembly occurrences, two configurations and two display states, and
one texture-mapping parameter changed per save. Those comparisons are needed to
verify the remaining reference graph and projection semantics.
