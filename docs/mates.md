<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Saved assembly mates and relation evidence

sw-rs now inventories supported native mate headers and their saved reference
strings. This is the first step toward understanding constraints; it does not
solve them or change assembly transforms.

## What a mate represents

A mate describes a relation among selected geometry in an assembly context.
Its participants can be faces, edges, axes, planes, points or component frames;
a component pair alone is not enough to reconstruct the constraint.
[IMate2](https://help.solidworks.com/2024/english/api/sldworksapi/SolidWorks.Interop.sldworks~SolidWorks.Interop.sldworks.IMate2_members.html)
exposes type, alignment and entity access, and
[GetMateEntityCount](https://help.solidworks.com/2025/English/api/sldworksapi/SOLIDWORKS.Interop.sldworks~SOLIDWORKS.Interop.sldworks.IMate2~GetMateEntityCount.html)
is separate from entity retrieval. The published
[swMateType_e](https://help.solidworks.com/2026/english/api/swconst/SolidWorks.Interop.swconst~SolidWorks.Interop.swconst.swMateType_e.html)
includes geometric, dimensioned and mechanical families.

| Family | Relationship to understand | Additional data needed for solving |
| --- | --- | --- |
| Coincident / distance | Relative placement of selected geometry | Entity types, local geometry, alignment, distance |
| Concentric | Shared geometric axis | Cylinder/axis references, orientation, rotation lock |
| Parallel / perpendicular / angle | Relative directions | Local directions, alignment, angle and limits |
| Tangent | Contact of selected geometry | Surface/curve definitions and solution branch |
| Width / symmetric | A relation across several selections | Ordered roles, reference planes, mode/offset |
| Lock | Locked relative placement | Component frames and relative transform |
| Slot / hinge / slider | Constrained relative movement | Selected geometry, free coordinates, modes/limits |
| Gear / rack-pinion / screw / coupler | Coupled motion | Axes, ratio/pitch, signs, phase |

These are explanatory API-level semantics, not claims that each corresponding
native binary layout is implemented. Multiple mates can constrain the same
components; remaining freedom comes from the combined constraints. A concentric
mate alone is not automatically a revolute joint, and a lock mate is not proof
that a component is grounded. Higher-order mates must not become arbitrary
pairwise edges or cliques.

## Current parser contract

`Document::mates.configurations` contains one entry per exact
`Contents/Config-N-MatesList` stream. `configuration_index` is the native N,
not a dropdown position or guessed configuration name. Each entry retains:

- The exact stream name, raw first word and supported saved short count.
- Common mate headers with exact UTF-16 names, offsets and saved identifier words.
- Explicit class names and known families, where an inline declaration exists.
- Reused class-reference tags, with provenance when the leading-index profile resolves them.
- Occurrence/selection-looking framed strings inside each source interval, plus
  adjacent geometry descriptor strings when their framing is supported.
- Count agreement and diagnostics, with entity ownership and parameter coverage
  explicitly false.

A record's `source_offset` identifies its declaration/reference tag; `name.offset`
identifies its string frame. `evidence_end` is bounded by the next supported mate
header or EOF. It is not a validated serialized-object boundary. All offsets are
within the decoded source stream, not the compressed file.

The supported common feature prefix has two flag bytes, a packed marker,
an identifier and a zero word, followed by a framed note. Nested dimension
parameter objects with the FFFFFFFF identifier sentinel are excluded. Exact
framing is required; mate types never come from localized names such as
`Concentric1` or `同心1`. Names are user-editable and can repeat.

A direct class declaration establishes coincident, concentric, parallel,
perpendicular, tangent, distance, planar angle, symmetric, width, lock, slot,
limit distance or limit angle when recognized. Other class names remain raw.
Later records reuse serialized class tags. The supported leading-index profile
can resolve references to the first declared class in the same stream; other
references remain unknown. The first header word is not a format version or mate type.

### Leading class-index profile

For the supported short-count stream profile, the leading u32 supplies the
archive-local index of the first inline mate class declaration at byte six.
A later short class tag equal to `0x8000 | index` can therefore name that class
without reconstructing intervening object payloads. The parser requires a complete
header inventory, an inline first declaration at byte six, an index in the short
class-tag range and an inherited feature-class tag preceding that index.
Other layouts remain unsupported. The map is scoped to this literal Config-N
stream; no indices are shared across configurations or documents.

Resolved reused records retain `class_name: null` because no declaration exists
at that record. Their `kind` comes from the first declaration's exact runtime
class name. `class_reference_resolution` records that name, class index, index
source offset, declaration offset and reused-tag offset. Unknown future class
names retain this provenance with `kind: null`. Names and saved feature labels
never determine a type.

The profile was checked against 197 available readings in previously collected
SolidWorks API evidence: 24 yaw concentric mates, 145 pitch coincident mates and
28 main-assembly coincident mates. All matched. Original pitch/main source hashes
were checked against the evidence; the yaw source matches the recorded baseline
hash. These are comparisons to saved API readings, not new live native solves.
The remaining 147 main-assembly anchor matches were not captured in that partial
API export and are not included in the 197-check count.

This does not recover subsequent class indices, reused parameter classes, entity
ownership, alignment, suppression or rotation locks. A complete ModelHeader
ledger is not an automatic seed for MatesList: its slots can be objects or absent
where a mate tag expects a class. The full archive-context handoff remains open.

`reference_candidates` are not a complete list of mate entities. A string such
as `A-1@Root/B-2@A` retains its occurrence context and instance numbers. A plane
selection can contain additional `@` separators. The parser does not strip them,
rewrite stale names, match basenames, resolve external paths, or infer topology
from coincident meshes. Reused object references may have no new string at all.

A geometry descriptor can contain several surface records in one CString.
`MateReferenceCandidate::surface_references()` preserves all top-level rows;
the singular accessor returns None for a group. The reference graph exposes
these as `selection_surface_references` while retaining the full raw descriptor.
No row is selected as the mate endpoint. See [grouped descriptor records](surface-reference-rules.md#grouped-descriptor-records).

`ConfigurationMates::reference_graph()` produces a bipartite evidence graph:
each distinct exact saved string lists the mate-record indices mentioning it.
Duplicate mentions in one record are deduplicated only in this graph projection;
the source candidate list retains duplicates. Nodes are saved strings, not
validated current component identities, and edges do not imply active constraints.

An absent stream means unavailable evidence. A saved zero count, malformed or
unsupported header, and a count mismatch remain distinct states. Count agreement
only checks the inventory; it never promotes ownership or solver coverage.

## Inspection

```sh
mbx run --locked --bin sw-dump -- mates assembly.SLDASM
mbx run --release --locked --features archive --example inspect_mates -- assembly-archive.zip
```

The archive example writes per-document SHA-256, configuration inventories and
reference graphs as JSONL. It reports unsupported containers separately. The same
records are available through ordinary metadata JSON and its WASM entry point.
The web viewer's selected-document metadata exports them; the inspector shows
recognized/saved header counts for the selected native configuration.

## Private-corpus audit, 2026-10-24

| Corpus | Supported assembly entries | Mate streams | Saved / recognized headers | Reference-string candidates | Paired geometry descriptors |
| --- | ---: | ---: | ---: | ---: | ---: |
| EGA mechanisms | 45 | 45 | 2571 / 2571 | 5223 | 4830 |
| Design library | 12 | 13 | 104 / 104 | 190 | 189 |

All 58 stream counts agree, including 13 saved-empty streams. Six additional
library assemblies used legacy OLE containers excluded from that runtime census. Counts
include duplicate archive entries/documents and are not deduplicated designs.

Explicit class declarations establish families for 130 EGA records and 19 library
records. The remaining 2441 and 85 records retain unresolved reused class tags.
The main `底盘总装V2-哨ver-0517.SLDASM` contains 441 recognized headers and
249 distinct saved reference strings. These are **not 249 verified current
components**: nested selections and stale source names remain in the evidence.

Per-file hashes, source offsets, raw names, classes, references and reproducible
research scripts are in ignored `local-assets/mate-investigation/`. Source CAD
is not distributed. Tests and the browser fixture are authored synthetic streams.
The inspected cadmpeg SolidWorks implementation did not provide an assembly
MatesList decoder to port; this profile was implemented from the corpus framing
and public API semantics, without a SolidWorks SDK or new runtime dependency.

## Next steps and verification TODOs

The [native document-handle investigation](references.md#native-document-handle-prefixes)
validates a complete explicit prefix in visual states. All 46 explicit face-reference
candidates in the public mate streams still encounter unknown reused classes.
The shared archive map remains necessary before decoding those branches. The
[public prefix follow-up](archive-reference-rules.md#public-mate-prefix-follow-up-2026-10-25)
finds matching handle-tag evidence in all 41 affected configuration streams and
separates inherited-context candidates from local reuse. Run
`cargo run --locked --example audit_mate_prefixes -- assembly.SLDASM` to inspect
the first unsupported position and raw tag in another sample. The opt-in reference
graph now retains 41 of these cases as separate saved-handle candidates with
configuration framing evidence. All 46 class-map stops remain visible; neither
mate families nor selected entities are resolved from these candidates. The
[five earlier stops](archive-reference-rules.md#the-five-earlier-stops-edge-reference-context)
all have earlier edge-reference chains; four also appear to reuse objects rather
than write new handle strings. Those branches need object payloads and map
accounting, beyond class-name reuse. The
[sequential subtree reader](references.md#sequential-reference-subtrees) now
implements the supported child layouts and typed backreferences. It requires
a verified archive context; the five original face stops remain unresolved.

1. TODO(mate-object-map): reconstruct the external/shared MFC class and object
   dictionary. This should recover reused mate families and reused selections,
   instead of relying on string-only evidence.
2. TODO(mate-ownership): resolve persistent selection descriptions to the exact
   configuration, component occurrence and entity. Include assembly-root planes,
   repeated Parts, nested/flexible subassemblies and stale renamed references.
3. TODO(mate-parameters): decode suppression, alignment, lock-rotation, driving
   dimensions, limit bounds and advanced/mechanical mate options. Do not feed
   opaque scalar fields to a physics engine as measured quantities.
4. TODO(mate-verification): controlled saves should include an unchanged baseline,
   one mate addition/removal/type change, renamed mates, one suppression/alignment
   change, repeated occurrences, and one dimension/limit changed per save.
5. Once those contracts are established, build a separate constraint adapter.
   Keep parsing independent of solving. Validate residuals against saved poses
   before considering kinematic joints or a general assembly solver.

## Controlled-save check, 2026-11-03

Eight authored assembly saves from SolidWorks 2025 SP5.0 (API revision 33.5.0)
were inspected with `sw-dump mates` in this session. The unmated baseline and
unchanged reopen/save control each report zero records and a saved count of zero.
The six single-mate variants each report one record, matching the saved count,
the expected explicit class family and two textual reference candidates:
coincident, concentric, two distances and two angles. Reopened API evidence gives
distances of 10/20 mm and angles of 30/60 degrees. At that checkpoint the parser
did not read those numeric values or resolve the selected entities. Reference strings and a
known numeric API value alone do not establish a native dimension-object layout.

Per-file reports remain ignored under `local-assets/controlled-saves-2026-11-03/`.
These checks confirm the existing header inventory on controlled examples; they
do not promote endpoint, parameter or solver coverage.

## Numerical parameter prefixes

`MateRecord::parameter_candidates` retains named feature-derived scalar prefixes
inside each record's evidence interval. The supported layout requires a framed
name, the saved parameter marker and sentinel identifier, a framed note, a zero
word and a finite little-endian f64. Parameter names are not used to infer their
meaning. Truncated, nonfinite and unsupported explicit parameter classes are not
reported as supported values.

Explicit `moLengthParameter_c` and `moAngleParameter_c` declarations establish
metres and radians respectively. Unknown reused class references retain the raw
scalar and tag with a null unit. Every result retains its name offset and
`value_offset` in the enclosing decoded stream. These are partial parameter
prefixes, not complete dimension objects or resolved driving equations.

On 2026-11-12 the four controlled dimension saves above were checked again.
Distance values decoded as 0.01/0.02 metres at stream offset 4531. Angle values
decoded as 0.5235987755982988/1.0471975511965976 radians at offset 4798. The
baseline, control, coincident and concentric saves yielded no parameter candidates.
These offsets describe those fixtures; the parser follows framing, not fixed
offsets. Synthetic tests cover renamed parameters, reused classes, record scope,
damaged markers, truncation and nonfinite values.

Numerical travel bounds remain unsupported: `limit_bounds_decoded` is false.
A limit mate can contain a dimension, tolerance values, presentation coordinates
and other scalars. None is labelled minimum or maximum without controlled saves
changing each bound independently. `parameters_decoded` also remains false:
alignment, suppression, rotation locks and complete dimension semantics are not
decoded. Numerical candidates must not automatically become simulation limits.
