<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Provisional native appearance interpretation

This records the 2026-10-24 corpus investigation and the 2026-11-03 controlled-save
follow-up below. Binary fields stay available as saved. Hypotheses are opt-in and
carry `status: "provisional"`; they do not set decoding-coverage flags.

## Other implementations and public APIs

- [cadmpeg](https://github.com/cadmpeg/cadmpeg), Apache-2.0: compared its appearance
  implementation at `4df21312c62cc1207a4c2274e29b4118fd232ec6` with current main
  `97524b7e4ad511cd500e80e2004f3ea2d976dbba`. The inspected appearance and layout
  files are unchanged. Its feature-resolution approach requires feature history
  and topology inputs that sw-rs does not yet decode; copying only the final
  source-ID match would discard those preconditions. Existing adapted framing
  remains attributed in [NOTICE-cadmpeg.md](https://github.com/hxyulin/sw-rs/blob/main/NOTICE-cadmpeg.md).
- [sldkit](https://github.com/monozukuri-ai/sldkit), inspected at
  `474c097afcd8a471a5cae75eb1a0560824d9d9f0`: its current license notice places new
  material under PolyForm Noncommercial, with older MIT material and separate
  third-party licenses. No implementation was ported from this checkout.
- [native-cad-viewer](https://github.com/jfk-solutions/native-cad-viewer/blob/main/FORMAT_SUPPORT.md)
  describes a separate SolidWorks reader, but the guessed public reader repository
  was unavailable. Its support description is not a verified binary layout.
- The public [IRenderMaterial API](https://help.solidworks.com/2017/english/api/sldworksapi/solidworks.interop.sldworks~solidworks.interop.sldworks.irendermaterial_methods.html)
  exposes U/V directions and a mapping center. Its
  [mapping-type property](https://help.solidworks.com/2022/English/api/sldworksapi/SolidWorks.Interop.sldworks~SolidWorks.Interop.sldworks.IRenderMaterial~MappingType.html)
  lists surface, projection, spherical, cylindrical and automatic modes. These API
  names motivate candidates; they do not establish native byte offsets or prove
  that a saved flag word uses the API enumeration.

## Corpus evidence

Reproduce the native audit with either private archive:

```sh
mbx run --release --locked --features archive --example inspect_appearances -- ARCHIVE --cached --hypotheses
```

The output includes document hashes, definition indices, exact source locators,
raw extension fields and a candidate interpretation or null. Original CAD and
per-file reports stay in ignored `local-assets/visual-decoding/`.

| Corpus | Modern documents | Supported extensions | Positive size pairs | Unresolved size pairs |
| --- | ---: | ---: | ---: | ---: |
| EGA mechanisms | 340 | 360 | 157 | 203 |
| Design library | 147 | 110 | 22 | 88 |

That survey excluded 93 legacy OLE documents. Subsequent OLE loading results
are recorded in [coverage](parsing-coverage.md#ole-follow-up-2026-10-25).
These counts include supported cache properties for Parts whose geometry can be
decoded. They are not counts of unique appearances, applied surfaces, or complete
native records.

All 470 supported extensions have the same candidate vector/origin grouping:
`U=(0,1,0)`, `V=(0,0,1)`, `origin=(0,0,0)`. Size-like values include approximately
`0.008382`, `0.0254`, `0.00635`, `0.010922`, and `1`. Values such as `0.0254` and
`0.00635` are consistent with metre representations of inch-based defaults, but
this does not establish units. **No nondefault direction or origin was found in
these historical supported records**, so their order, signs, or even their semantic identity
cannot be independently confirmed from this sample.

The available map sources are STEP exports. A line-oriented scan for explicit
style/color/texture entity declarations found:

| File | STYLED_ITEM | SURFACE_STYLE_FILL_AREA | COLOUR_RGB |
| --- | ---: | ---: | ---: |
| RMUC2027_V1.0.0.stp | 177505 | 62148 | 373 |
| RMUC2026_V1.2.0.step (historical research) | 189283 | 55 | 55 |

Neither scan found `IMAGE_TEXTURE`, `PIXEL_TEXTURE`, `TEXTURE_MAP`,
`SURFACE_TEXTURE`, `EXTERNALLY_DEFINED_SURFACE_STYLE`, `SURFACE_STYLE_RENDERING`,
or `SURFACE_STYLE_RENDERING_WITH_PROPERTIES` declarations. This is evidence about
these entity types in these exports, not a complete STEP appearance parser or
proof that every possible texture representation is absent. STEP cannot establish
SolidWorks-native state/object ownership offsets in the original unsupplied file.

## Implemented interpretation

`NativeAppearanceExtension::shader_parameters` now preserves the previously
skipped label, selector words, discriminator, packed words, ten-float group,
mode, five-float group, flags and finish values. Their framing is decoded; their
renderer meanings are not guessed.

`NativeAppearanceExtension::texture_mapping_hypothesis()` groups the fourteen
mapping floats as follows only when the first two vectors are approximately unit
length and perpendicular, and all values are finite:

| Raw indices | Candidate meaning | Verification |
| --- | --- | --- |
| 0..3 | U direction | TODO: rotate U alone; check axis order and sign |
| 3..6 | V direction | TODO: rotate V alone; check handedness |
| 6..9 | Mapping origin | TODO: translate alone; check model/component frame and units |
| 9..12 | Uninterpreted | TODO: change angle/scale independently; do not assume `90` is an angle |
| 12..14 | Repeat width/height, if both positive | TODO: change each dimension separately; check units and negative defaults |

The result carries a source offset and required verification steps. Invalid
vectors return null; negative, zero or mixed-sign size pairs remain unresolved.
Raw values remain intact. Neither projection mode nor a UV transform is emitted.
The three mapping flag words and earlier two scalars remain uninterpreted.

## Remaining ownership TODOs

The subsequent [native document-handle decoder](references.md#native-document-handle-prefixes)
retains explicit reference-prefix file context. It does not assign those paths to
appearance definitions or prove that the saved file still owns the referenced face.

- TODO(native-state): decode reused MFC objects and key associations before choosing
  active configuration/display state or treating unknown scope as universal.
- TODO(native-feature): connect source ID/timestamp through current history and
  topology before attaching a feature definition to cached faces.
- TODO(native-body-face): establish entity identity separately from cache-table
  identity; do not use geometry order, coincident shapes or file names.
- TODO(native-component): verify two occurrences of one Part with only one
  overridden, including nested assembly ownership.
- TODO(native-mapping): compare an unchanged baseline and one changed parameter
  per save. Include nondefault directions/origins, independent dimensions,
  rotation, projection mode, component transforms and a document-unit change.

Current rendering presets continue to work while these hypotheses remain
available to future renderer adapters. No proprietary shader code or texture
pixels are supplied by this work.

## Controlled bitmap-texture saves, 2026-11-03

A locally authored SolidWorks 2025 SP5.0 corpus (API revision 33.5.0) supplies
34 textured Parts, split evenly between a block and cylinder. Each shape includes
an unchanged reopen/save control, a material-reapplication control, setter-only
negative controls and independent reapplication variants. Reopened API snapshots
provide ground truth; filenames alone do not establish that an edit persisted.
The complete local collection contains 80 native documents and 139 payload files.
All 139 payload hashes and the enclosing ZIP hash were checked this session.
Original CAD, appearance resources, paths and per-file evidence remain ignored
under `local-assets/controlled-saves-2026-11-03/`.

### Additional supported framing

The previous reader recovered an appearance prefix from every textured Part but
rejected all 34 extensions. The legacy block following the texture reference has
a two-byte word and a zero u32. Support now retains that word as `reference_flags`
and accepts only these observed word/selector pairs:

| Reference word | Six trailing selectors |
| --- | --- |
| 0 | 0, 1, 1, 0, 1, 1 (previous profile) |
| 1 | 0, 1, 0, 1, 0, 1 (controlled bitmap profile) |

Mixed pairs and other values remain unsupported. This adds field framing, not a
new claim of complete visual-object decoding. The full collection now yields 38
extensions, including all 34 texture Parts, compared with three before this change.

### Parameters established by the controlled saves

`NativeAppearanceExtension::bitmap_texture_parameters()` exposes a limited
source interpretation for the reference-word-one profile with shader `texture`,
selector words `[1,1,33,0]` or `[1,1,35,0]`, and discriminator 3 or 6.
Other profiles return None. Every field also remains available in its raw group.

| Source field | Controlled observation | Exposed meaning |
| --- | --- | --- |
| Legacy doubles 0 and 1 | Width-only and height-only edits change 25 to approximately 14.084507 and 33.333332 to 18.867924 respectively | Reciprocal scales per metre |
| Legacy double 12 | Setters 30 and 0.47 save approximately 330 and 359.53; reopened getters return radians | Stored degrees, without a universal handedness conversion |
| Shader floats 6..9 | Center setter (0.011, 0.013, 0.017) changes this group in both shapes | Texture center in metres; assembly occurrence frame remains open |

For 32 mapping-type-4 snapshots, reciprocals of the saved scales agree with the
reopened API Width and Height. The two mapping-type-2 saves retain those same
stored scales while API dimensions change with projection geometry. The API
mapping enum is therefore not inferred from these scales or the shader mode.
The accessor deliberately exposes reciprocal scales, not universal repeat sizes.
All 34 center and circular rotation comparisons match the reopened evidence.
These are measured source parameters, not generated UVs or verified rendering.

The earlier fourteen-float hypothesis is contradicted for texture-bearing
extensions: the center edit leaves its candidate origin unchanged, and dimension
edits leave its candidate sizes at negative sentinels. The independent orthogonal
direction edit also produces a first group that fails the proposed basis test.
`texture_mapping_hypothesis()` now returns None whenever a legacy texture block is
present. Its output for other profiles remains provisional. This investigation
does not establish U/V ordering or projection semantics for those profiles.

The separate unit-only fixture was previously reopened with unchanged API mapping
values. It supports the use of system units but does not establish the native
coordinate frame of nested occurrence textures. Complete texture coverage stays
false, and no renderer or native face assignments change.

### Reproduce the comparison

```sh
cargo run --locked --features archive --example inspect_appearances -- CORPUS.zip --hypotheses > REPORT.jsonl
python3 scripts/audit-controlled-textures.py CORPUS.zip REPORT.jsonl
```

The script validates all payload hashes, joins the texture report to reopened API
snapshots, and checks parameter agreement. It prints counts without exposing
paths or CAD data. Authored synthetic tests cover the additional framing,
truncation at every byte, unsupported combinations and semantic profile gates.
Mates, display-state visibility, nested overrides and persistent-reference change
experiments are present in the local collection; their native object relationships
are not promoted to decoded ownership by these texture comparisons. The face-split
fixture, independent display-state appearances, portable reference resolution and
visual rendering remain unverified.

## EGA renderer fallback audit, 2026-11-03

The browser bitmap implementation was committed at `a0b8de6` before this audit.
The initial main assembly selection renders 3,380,059 triangles in 615 Part
occurrences, using 171 distinct external Part/configuration pairs. The audit's
occurrence-weighted valid source-triangle counts sum to the exact rendered total.
Hidden, suppressed, missing and unsupported occurrences are excluded from these
ratios. The report stays ignored at `local-assets/ega-appearance-audit.json`.

At the audited commit, the renderer built ordinary material batches through
`appearance::for_configuration`, which requires a resolved physical-material
assignment. Outside the bitmap path it did not select the parser's cache or
document visual RGB records. A Part without a physical assignment therefore received
a path-palette color even when a scoped visual color is already decoded.

| Audited physical-material selection | Rendered occurrences | Triangles |
| --- | ---: | ---: |
| Resolved configuration assignment | 226 | 467,912 |
| Material tree parsed, no assignment for selected configuration | 363 | 2,240,090 |
| No material-tree stream | 26 | 672,057 |

All 14 distinct source Parts in the last group were checked independently: their
`swXmlContents/MATERIALTREE` stream is absent. Of the 66 source Parts without a
configuration assignment, 53 have no material definitions; 13 retain definitions
but only blank configuration/reference records. A raw XML inspection confirms an
empty `<configuration>` record in a representative definition-bearing Part.
No fallback occurrence in this selection has a nonempty physical reference that
fails to resolve. Unassigned definitions must not become physical assignments.

| Decoded visual evidence on diagnostic-color triangles | Triangles |
| --- | ---: |
| Unique highest-layer cache RGB | 751,966 |
| Unique scoped document RGB, with no cache property | 2,040,997 |
| No usable scoped RGB | 119,184 |

Thus 2,792,963 of the 2,912,147 diagnostic-color triangles (95.9%) already have
source-backed RGB evidence that ordinary shading did not consume. This was a
renderer selection gap, not evidence that those Parts need a physical material
invented or a texture downloaded. Many source colors are white or gray. These
counts establish usable saved defaults/cache colors, not complete effective
SolidWorks appearance ownership or verified original shading.

Saved-appearance mode now selects a supported cache color for its exact table,
otherwise a uniquely scoped document default, before falling back to the existing
physical-material preset and diagnostic palette. Ambiguous, unresolved and
sentinel cache records block document-color fallback. Cache colors and bitmap
selection share one selector, and batch merging includes the saved color.
Material-family mode continues to use only physical assignments. Shader response,
unsupported appearance scopes and the remaining 119,184 triangles need separate
handling.

The updated scene contains 3,260,875 triangles with selected saved RGB, and 119,184
triangles still using diagnostic colors. All remaining triangles belong to four
occurrences of one Part. That Part has a decoded blue-gray definition, one named
configuration/display-state key, two unscoped state keys, and no proven document
appearance binding. Applying its lone color would guess ownership. The raw
records remain available for further class-graph decoding.

### Installed material definitions

Read-only inspection of the installed graphics library found 631 `.p2m` files,
plus image and scene resources. Three sampled `.p2m` files are small text parameter
lists: the generic color shader, satin-finish aluminum, and black low-gloss
plastic. They expose shader identifiers, colors, roughness, reflectivity, diffuse
and specular factors, transparency and optional bump references. The generic
color definition has no bitmap reference. The sampled aluminum and plastic files
have empty bump references. These shaders therefore do not require bitmap pixels
for a basic surface approximation.

These definitions can inform a future explicit shader adapter or user-supplied
library import. They do not recover native entity ownership, missing display-state
links, UVs or renderer equivalence. No installed material files or images are
bundled with the project. Captured files, inventory and model-specific evidence
stay under ignored `local-assets/`.

Reproduce with the exact assembly path from the caller's private archive:

```sh
cargo run --release --locked --manifest-path viewer/Cargo.toml --example audit_appearances -- ARCHIVE ASSEMBLY_PATH > REPORT.json
```

The report includes per-document names and references; keep it private. The tool
counts only rendered external Parts, preserves repeated occurrences and reports
embedded occurrences separately rather than inferring their source documents.


## Offline class-reference investigation, 2026-11-04

Further analysis uses the existing native corpus and recorded API results. No new
SolidWorks save trials or installed application access are required.

The unresolved EGA Part contains four property-shaped blocks, rather than only
the single definition returned by the current decoder. The explicit declaration
has RGB `(202,209,238)`. Three further blocks have candidate RGB `(229,234,237)`,
`(57,57,57)` and `(0,49,255)`. Each additional block is preceded by word `0x8009`
and has the same short CString, finite six-double optical prefix and fixed shader
identifier placement. These are candidate instances, not proven entity links.

The same candidate inspection was run on all 80 native files in the existing
controlled corpus. It found 31 matching prefixes. For all 25 already-decoded
matches, candidate RGB and optical values exactly match the current decoder.
The six additional candidates include the green body and yellow face variants:
their candidate colors match the independently recorded reopened API values.
Four other candidates occur in the configuration/state Part family. Those API
experiments did not prove independent display-state appearance assignment.

This narrows the missing-color work to two stages: decode MFC class/object reuse,
then connect those instances to body/face/feature/state owners and exact cached
surfaces. Repeated colors or proximity to an entity record cannot establish those
links. In particular, applying the unresolved Part's first blue-gray definition
to every triangle could hide its dark and blue local appearances.

`inspect_visual_candidates` is a read-only research example. Its fixed prefix
patterns deliberately remain separate from the production appearance decoder.
It emits candidate offsets, preceding words, optical values, shader identifiers,
existing visual records and structural inspection. It does not assign candidates
or alter browser rendering. It handles only empty texture strings and short
material-name CString frames in this candidate profile; absence of a candidate
is not evidence of absent appearance data.

```sh
cargo run --locked --example inspect_visual_candidates -- MODEL > REPORT.json
```

Reports contain private strings and paths. The local results are stored under
ignored `local-assets/remaining-visual-candidates.json` and
`local-assets/offline-corpus-visual-candidates.json`.


### Feature-prefix and cache-source correlation

The next offline pass found two repeated blocks inside the declared
`moCompFeature_c` interval. Their prefixes contain version word 18000, source IDs
98 and 92, the same saved timestamp, selectors `[3,1,2]`, and the previously
supported attribute/property tag sequence. Both source IDs have unique matches
in the same Part's feature XML. The cached-surface catalog independently contains
17 records agreeing on source ID 98 and 64 agreeing on source ID 92.

The decoder previously accepted only selectors `[1,1,2]` in this feature-prefix
scan. It now also accepts the observed `[3,1,2]` profile and preserves the dark-gray
and blue definitions with their source ID/timestamp evidence. It does not infer a
meaning for selector 3 or accept other selector values. A regression test covers
recovery of the new profile, retained unresolved ownership, and rejection of an
unobserved selector.

These definitions remain unresolved visual bindings. The matching cache records
establish source-ID correlations, not timestamp continuity, the active display
state, native archive-map continuity, or complete precedence over face/body
properties. No browser colors are changed by this decoder extension. The light-gray
candidate still lacks this supported feature prefix.

The research example also emits the source-header words, feature-XML candidates
and exact carrier/offset pairs for matching cache-source IDs. Optional
`--dump-stream PATH` writes the decoded visual stream for byte-level inspection;
keep both stream dumps and JSON reports private.
