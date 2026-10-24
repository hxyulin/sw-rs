<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Metadata and saved records

`Document` represents a saved snapshot, not a regenerated model. A filename hint
selects Part, assembly or drawing interpretation; it is never opened by the core.
A native version word controls supported field gates. Unknown or malformed
records produce diagnostics rather than invented values.

## Configurations

`Contents/CMgrHdr2` (or the older header fallback) supplies the supported
configuration-manager profile. Sequential MFC class/object bookkeeping validates
`dmConfigMgrHeader_c` and `dmConfigHeader_c` headers; names use framed strings.
Native index, parent evidence and version-gated fields follow. A leading raw
word is not substituted for the configuration index. Supported schema 0/1 does
not imply arbitrary schema support. [Archive-map research](archive-reference-rules.md)
documents exact slots, opaque trailers and tested native versions.

Other saved records provide global/configuration properties, preview PNGs and
raw mass-property tokens. Names and paths preserve Unicode. Missing optional
fields remain absent. COM, volume, mass and inertia have unverified source units,
frame, sign convention and cache freshness; do not use them directly in physics.

## Assembly occurrences

`swXmlContents/COMPINSTANCETREE` and supported saved records expose occurrence
names, external paths, referenced configurations, component-reference strings,
visibility, suppression and BOM exclusion. Repeated occurrences remain separate
from shared document definitions. A saved 16-value transform is optional;
missing/malformed data is `None` rather than an invented identity matrix.

Transforms follow the openswx-derived column-major interpretation. Agreement
with that implementation is not independent validation of every placement.
Viewer composition uses these saved poses; mates do not drive positioning.

Saved `ReplaceRef` string pairs retain old-to-new paths only when the complete
supported stream decodes; partial pairs do not become mappings. Header reference
lists and embedded `ImportedComp`/`VirtualComp` documents remain evidence with
owner context, not unrestricted filesystem resolution.

## XML structure and features

Structural inspection accepts supported UTF-8/UTF-16 XML, preserving namespaces,
attributes, source spans and text. File/model reference links use explicit
stream-local IDs and require unique targets. Malformed XML is reported; a valid
XML tree is not automatically a semantically decoded SolidWorks object.

The `KeyWords` feature catalog retains names, native type tokens, raw attributes,
XML containment, source IDs and invalid/reserved ID tokens. Valid numeric IDs
are document-local evidence. Features are not evaluated, and no new geometry is
regenerated. [Reference decoding](references.md) describes candidate joins and
scope guards.

## Materials, weldments and drawings

`MATERIALTREE` supplies physical definitions, exact configuration references,
density tokens, swatches and raw shader/texture/optical elements. Assignments
resolve only unique exact references; a material definition alone is not an
assignment. Physical material and visual appearance are separate: see
[presentation](presentation.md).

Saved weldment cut lists expose names, quantities and properties. Drawing
metadata exposes basic sheet/view names and referenced documents/configurations.
There is no drawing renderer, BOM document resolver or sketch/feature evaluator.
Neither a thumbnail nor a metadata-only drawing is claimed as recovered geometry.

## Inspect unsupported payloads

Use `DocumentReader::inventory()` for provenance and opaque accounting, then
`inspect_stream()` for structural XML or binary class/string candidates. Binary
candidates are framed observations, not complete payload boundaries. Raw native
body/history/visual data stays available for future checked decoders.
