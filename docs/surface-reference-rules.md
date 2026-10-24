<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Surface references: additional rules and measured coverage

The archive-map implementation improves reference correctness; it does not itself
decode feature payloads, native topology or extra mesh carriers. This follow-up
investigates typed identifiers in cache metadata and mate geometry descriptors.

## Confirmed supported token profile

A framed UTF-16 string can contain:

```text
mo…SurfIdRep_c,feature_source_id,local_surface_id[,signed_tail_field…],
```

The Apache-2.0 [pinned cadmpeg implementation](https://github.com/cadmpeg/cadmpeg/blob/4df21312c62cc1207a4c2274e29b4118fd232ec6/crates/cadmpeg-codec-sldprt/src/tessellation.rs)
interprets this family separately from MFC map slots. We adapted that profile,
retaining the complete source record and testing it against both private corpora.
It is not a universal schema for every native selection class or an API
`GetPersistReference3` byte array.

- A positive feature source ID excludes the absent zero/FFFFFFFF markers.
- The local surface ID is unsigned; a saved zero is retained.
- Numeric tail fields are signed 32-bit decimal values. Preserve their ordered
  u32 bit patterns: `-1` becomes FFFFFFFF. Tail length also belongs to the tuple.
- Opaque suffixes retain source/local evidence but cannot produce a full tuple.
  Invalid required fields remain recognized, explicitly invalid records.
- Duplicated strings must all agree before reporting a consistent tuple.
  Conflicting, opaque or invalid records block full identity agreement.
- An agreed feature prefix can survive different/opaque tails. It can identify a
  feature-reference candidate without establishing a particular face.

Full framed strings, source offsets, class names, raw tail tokens and terminal
comma presence remain available. Extended UTF-16 count forms are supported by
the shared checked reader. A complete tuple is lexical interpretation, not proof
of cross-save stability or an existing face/body.

## Native measurements

The census uses the same 340 EGA and 240 design-library native entries as the
previous inventory. Every source SHA-256 matches. Only canonical supported Part
geometry is eligible for bounded cache-metadata inspection: 248 EGA Parts and
79 library Parts. The 93 legacy OLE library documents remain unsupported; mirror,
embedded and configuration-ambiguous geometry limitations remain in place.

Across 218,500 validated cache tables, 433,680 recognized surface-reference
strings were recovered. Repetition is intentional; these are not unique faces.
Of those strings, 410,220 carry complete supported numeric tuples, 19,858 have
opaque tails, and 3,602 have invalid required fields.

Per-table agreement is:

| Metadata interpretation | Tables |
| --- | ---: |
| All recovered full tuples agree | 205,092 |
| Opaque suffix prevents full agreement | 9,929 |
| Invalid required fields | 1,801 |
| No supported reference recovered | 1,669 |
| Conflicting full tuples | 9 |

215,030 tables have a common valid feature-source prefix. Every one of these
prefixes uniquely matches a saved Keywords feature in its own source document.
These links are still `candidate`: timestamps, cache freshness and current native
topology ownership are not established. A table is not asserted to be one B-Rep
face merely because its metadata contains a tuple.

There are 451 per-document source/local pairs that occur with different complete
numeric tails. Dropping the tail would collapse distinct saved identities in real
files. The nine conflicting metadata windows remain unresolved; window ownership
and possible unrecognized intervening records need further investigation.

In the inspected cache windows, 216,822 contain two recognized strings, nine
contain four, and 1,669 contain none. Every recognized cache string retains the
terminal comma. Complete numeric tails have zero, one or two fields in this
corpus; the parser preserves longer supported tuples rather than treating these
observations as universal limits.

Mate reference evidence contains 4,983 recognized surface descriptor strings:
4,217 complete tuples, 568 opaque tails and 198 invalid records. These are string
occurrences, not resolved mate endpoints. Other geometry classes and missing
adjacent descriptors remain raw. Explicit mate-family coverage is unchanged:
149 of 2,675 headers; no constraints are solved.

An independently written Python decoder checked original decoded byte windows
in two distinct native Parts covering complete, invalid, opaque and conflicting
cases: 3,145 tables and 6,296 strings. All offsets, raw text, classifications and
numeric tuples agree with Rust. Synthetic tests additionally exercise malformed
Unicode, extended counts, truncation, duplicate conflicts and integer bounds.

## Opt-in reader and graph interfaces

`DocumentReader::surface_references()` returns cached `CachedSurfaceData` with
source hash, canonical configuration and per-table bounded reference records.
It reuses existing geometry/carrier buffers without reinflating or copying mesh
arrays. Normal `geometry()` and viewer loading do not perform this string scan.
The tiny `Tessellation::metadata_span` locator is omitted from ordinary geometry
JSON, retaining its existing serialized fields.

`references_with_geometry()` returns an owned graph with these cache records.
`ReferenceGraph::with_surfaces()` also accepts same-source inspection data; source
hash/schema mismatches and incompatible cached configuration scopes are rejected.
Repeated composition retains existing table locators. `references()` itself stays
independent of geometry. `finish()` transfers requested surface results alongside
the other cached projections.

`MateReferenceCandidate::surface_reference()` interprets its adjacent descriptor;
reference graph selection links include the resulting structured record without
changing their unresolved endpoint status.

```sh
mbx run --locked --bin sw-dump -- references part.SLDPRT --geometry
mbx run --release --locked --features archive --example audit_references -- PRIVATE.zip --geometry
```

Core WASM exports `geometry_references_json(bytes, filename)` for this composition.
No renderer assignments, UVs, native shaders or physics properties are fabricated
from these identifiers. Both native-object-graph and topology-ownership coverage
flags remain false.

## What this unlocks next

Additional research identified and implemented the separate
[legacy chunked ZLB wrapper](legacy-zlb-rules.md). This enables extracted-stream
decoding but does not enable the OLE document container in the runtime parser.

The saved identifiers now have a typed, loss-preserving representation shared by
mate descriptors and cache inspections. The next join is to complete native
`FACE_ID_2001` / body identities, preserving occurrence context, timestamps and
configuration scope. Separately, the inherited mate-stream map context remains
the [previously documented hypothesis](archive-reference-rules.md); this change
does not promote it to confirmed family decoding.

Private JSONL audits, source hashes, window probes and compatibility/performance
logs remain ignored in `local-assets/surface-investigation/`. Cache inspection
does not increase rendered Part/assembly counts; geometry coverage is a separate
measurement from the number of reference strings interpreted.
