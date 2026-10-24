<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Provisional native appearance interpretation

This records the 2026-10-24 investigation without requiring a SolidWorks
installation. Binary fields stay available as saved. Hypotheses are opt-in and
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

The library also contains 93 legacy OLE documents unsupported by this runtime.
These counts include supported cache properties for Parts whose geometry can be
decoded. They are not counts of unique appearances, applied surfaces, or complete
native records.

All 470 supported extensions have the same candidate vector/origin grouping:
`U=(0,1,0)`, `V=(0,0,1)`, `origin=(0,0,0)`. Size-like values include approximately
`0.008382`, `0.0254`, `0.00635`, `0.010922`, and `1`. Values such as `0.0254` and
`0.00635` are consistent with metre representations of inch-based defaults, but
this does not establish units. **No nondefault direction or origin was found in
these supported records**, so their order, signs, or even their semantic identity
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
