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
- Reused class-reference tags, with a leading-index or declaration-order proof when resolved.
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
perpendicular, tangent, cam/follower tangent, distance, planar angle, symmetric, width, lock, slot,
limit distance or limit angle when recognized. Other class names remain raw.
Later records reuse serialized class tags. The leading-index profile resolves
references to the first declared class. A separate declaration-order profile can
resolve some later classes when only one match is possible. Other references
remain unknown. The first header word is not a format version or mate type.

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

The leading-index profile alone does not recover subsequent class indices or
reused parameter classes. The declaration-order profile below can recover a subset.
Neither establishes entity ownership, alignment, suppression or rotation locks. A complete ModelHeader
ledger is not an automatic seed for MatesList: its slots can be objects or absent
where a mate tag expects a class. The full archive-context handoff remains open.

### Declaration-order profile

Class indices increase with declaration order in the shared class/object map.
A reused class must have been declared before its first use. Once the first
class index is anchored, a reference at or above that index belongs to a class
introduced locally. The reader uses these constraints within two separately
checked prefix families: common mate headers and named scalar parameters.
It does not count intervening object registrations or construct their map.

The reader retains every eligible declaration, including classes never reused.
It sorts distinct referenced indices and records each one's earliest use. Each
reference can match only an earlier declaration, and distinct indices must match
distinct declarations in order. A pair is resolved only when it occurs in every
valid order-preserving assignment. For example, with declarations A, B and C,
one later reference could mean B or C and stays unknown. If that reference first
appears before C is declared, it identifies B. Later unused declarations do not
invalidate that earlier proof.

The profile requires the same complete common-header count and leading anchor
as above. Duplicate declaration names or inconsistent assignments reject the
order proof. Indices below the anchor remain inherited and unresolved. Parameter
inventory includes known declarations even when their scalar is nonfinite;
unknown classes with the same parameter prefix prevent parameter resolution.
An omitted scalar must not remove a class and create false uniqueness.
Conflicting mate and parameter assignments reject the order profile together;
independently anchored leading-index matches remain available.

`class_order_evidence` stores declarations, distinct indices and earliest-use
offsets once per configuration. Resolved records and parameter candidates retain
`class_name: null`, their raw tag and `class_reference_resolution`. The optional
`declaration_order_evidence` index points into that configuration's shared proof.
`index_source_offset` points at the reused tag for this profile, or at zero for
the leading-index profile. Types and units come from matched runtime classes,
never edited labels, scalar magnitudes or the containing mate's family.

Four mixed six-mate controls were checked against reopened API readings: baseline,
unchanged control, one distance edit and one mate rename. All 24 mate types and
saved feature IDs matched; all 16 numerical values and units matched. The
parameter edit replaced that mate's feature ID, so the saved identifier is not a
permanent identity across edits. The unchanged and renamed cases retained it.
The saved count and byte framing, not names or COM order, drive production decoding.

A private four-assembly recheck identifies 593 of 594 mate families. Comparisons
of 577 resolved reused classes against earlier partial COM readings all matched.
Those larger native captures had open/reference and rebuild problems; this is not
a complete native-solver validation. One distance-looking record remains ambiguous
between two class declarations. Its scalar has a proven length parameter class,
but neither its editable name nor its 0.01 m value establishes the mate family.
The original yaw and pitch assemblies identify all 40 and 356 mate families.
All 396 types matched the earlier API readings when checked again.
Their inherited numerical parameter classes and active motion remain unresolved.

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
3. TODO(mate-parameters): extend saved-state profiles and decode the lock-reference
   phase, driving dimensions, full limit activation and advanced/mechanical mate
   options. Do not feed
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

Explicit `moLengthParameter_c` and `moAngleParameter_c` declarations, or unique
declaration-order matches to them, establish metres and radians respectively.
Unknown reused class references retain the raw
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

Complete travel-bound and enable-state decoding remains unsupported:
`limit_bounds_decoded` is false. Supported configured distance and angle bounds are now
available separately as described below.
A limit mate can contain a dimension, tolerance values, presentation coordinates
and other scalars. None is labelled minimum or maximum without controlled saves
changing each bound independently. `parameters_decoded` also remains false: complete dimension, activation and
mechanical-mate semantics are not decoded. Supported saved state appears separately. Numerical candidates must not automatically become simulation limits.

## Configured distance and angle bounds

`MateRecord::configured_limits` requires a complete recognized/saved header count
and a recognized limit-distance or limit-angle class. Exactly one explicit
`moLengthParameter_c` or `moAngleParameter_c` prefix must match the mate type and
be followed immediately by the supported explicit `moDimTolerance_c` prefix.
Its two observed u32 words must be 3 and 0. The two subsequent f64 values are
the upper and lower signed deviations from the saved dimension value:
`minimum = dimension_value + lower_deviation` and
`maximum = dimension_value + upper_deviation`.

The result has an explicit `unit`, either `metres` or `radians`, and retains the
original dimension, deviations, decoded-stream offsets and tolerance declaration
offset. The saved dimension is a tolerance reference, not a solved joint coordinate.
The supported profile requires finite values, a nonnegative interval and a
reference inside that interval. Multiple parameters, unknown/reused parameter or
tolerance classes, other tolerance words and truncated fields remain unsupported.
Ordinary distance and angle dimensions never produce configured travel bounds.

Eight SolidWorks 2025 SP5.0 controls were checked after reopen, rebuild and resave.
The four distance controls retained a 0.05 m dimension: baseline and unchanged
control had [0, 0.1] m bounds, the lower-only save had [0.01, 0.1] m, and the
upper-only save had [0, 0.09] m. Four angle controls retained a pi/2 rad dimension:
baseline and unchanged control had [0, pi] rad bounds, the lower-only save had
[pi/18, pi] rad, and the upper-only save had [0, 17pi/18] rad. Both sets changed
the corresponding signed deviation independently. The native API reported
advanced/limit mode for each. Absolute offsets vary with saved strings; decoding
follows framing rather than fixture offsets.

The attempted 0.06 m current-only save reopened at 0.05 m and remains unverified.
Current-only angle and disabled-mode controls were not produced. Screenshots do
not establish motion. Native files, transferred hashes, raw readings and scripts
remain private.

`records_with_configured_limits` counts supported prefixes per configuration and
in the assembly-analysis coverage summary. The broader `limit_bounds_decoded`
and `parameters_decoded` flags remain false: effective activation, separate
enable flags and other profiles are not decoded. Saved suppression is reported
separately and does not establish effective activation. A configured interval does not
establish a physical stop, usable stroke, collision-free travel, or which component
moves. Unknown bounds are omitted, never replaced with zeros or infinity.

## Saved mate state

`MateRecord::saved_state` reports supported saved fields in the enclosing literal
Config-N stream. `suppression` contains a boolean `value` and its u32
`source_offset`. Optional `alignment` contains its semantic value, original u16
`raw_value` and offset. Optional `rotation_lock` contains a boolean and offset
for supported concentric tails. An absent field means unknown or unsupported,
never false. `parameters_decoded` remains false.

Suppression requires a complete common-header inventory and the observed
common-feature framing after the name and note: words [0, 0, 0, suppression,
0, 101], with suppression restricted to 0 or 1. The word 101 is a layout gate,
not an application or file-version claim. The result retains
`feature_format_offset`. The earlier byte in the common feature header is not
an inverse suppression flag: captured unsuppressed mates can have either value.

Alignment follows 55 scalar bytes, a framed CString and 84 scalar bytes in this
profile. The decoder checks the following word/byte markers and the observed
selection counts 1 through 4 before retaining the native u16. Native values 0,
1 and 2 map to `closest`, `aligned` and `anti_aligned`. These differ from the
[API alignment enum](https://help.solidworks.com/2026/English/api/swconst/SolidWorks.Interop.swconst~SolidWorks.Interop.swconst.swMateAlign_e.html).
Other native codes retain their raw value and offset with a null semantic value.
Unknown mate families do not acquire alignment semantics from an editable name.

Concentric rotation lock requires a known concentric family and a checked tail:
two empty CStrings, a 0/1 u32 and an FFFFFFFF word. The check is bounded at the
next supported mate header or stream end, so each record is checked separately.
It does not reconstruct a complete object boundary or the lock-reference phase.
Other families never acquire a false rotation-lock value from a failed getter.

Seven new native controls were reopened, rebuilt, resaved and reopened again:
active/suppressed coincident, aligned/anti-aligned coincident and unlocked/locked
concentric, with unchanged controls for both families. Their saved fields match
the reported API readings. A follow-up checked all seven through typed coincident
and concentric definition interfaces; alignment and rotation-lock readings agree.
The seven files' hashes stayed unchanged during that check.

A separate two-configuration assembly retained an unsuppressed mate in Default
and a suppressed mate in SuppressedOnly through two reopen/rebuild cycles. The
reader retains unsuppressed state in `Config-0` and suppressed state in `Config-1`.
The API reported configuration IDs 0 and 1 respectively, but the native
configuration-header record format has not been independently decoded. That
agreement does not establish a general binding between names, API IDs and stream
suffixes. State remains scoped to the literal source stream; no name is assigned
by dropdown position or nearby strings.

The profiles also match earlier partial assembly readings: 990 suppression
values, 989 known-family alignment values and 181 available concentric lock
values. The remaining ambiguous mate family retains suppression but no alignment
semantics. Those native captures have the previously documented load, rebuild
and reference-resolution limitations. They are comparisons to saved API evidence,
not new native-solver or motion verification.

Saved suppression, alignment and rotation lock do not establish effective mate
activation, successful constraint solving, grounding, collision-free travel or
motion freedom. In particular, an unsuppressed feature can still be ineffective
because of its configuration, parents or unresolved references.

## Saved component state

`Component::fixation` reports supported saved fixation in the component's owning
assembly. `fixed_in_owner` is a boolean; `source_stream`, `prefix_span` and
`flag_offset` identify its bytes. `reference_source_id` and `reference_name`
retain the exact native identity used for the join. This does not establish a
world-space anchor or motion freedom. An absent field means unknown, not floating.
`grounding_decoded` remains false; `components_with_saved_fixation` counts the
supported subset.

The current profile reads only the first explicit `moReference_c` prefix in a
supported `moAssembly_c` Config-N envelope. It checks the root/node headers, a
positive short reference count, the common-feature 101 framing, a framed string,
84 scalar bytes, twelve finite doubles, two raw words, the 0/1 fixation u16,
a raw word and a nonempty configuration string. The opaque header and scalar
fields have no inferred physical or version meaning. Offsets follow framed
strings; fixture positions are not hardcoded. Unknown layouts and reused class
tags remain unsupported.

The source ID must match one unique direct XML occurrence's `swID`. Its saved
instance label must exactly match `swComponentName` plus `swReferenceNumber`.
Missing, duplicate, suppressed or mismatched identities are not joined by path,
basename, editable label or transform. The raw `saved_reference_configuration`
string is retained without rewriting internal flexible aliases. Fixation is
currently withheld unless the XML has one unambiguous root configuration and
there is one matching literal root carrier. This includes inactive configurations
without cached streams. Independent cross-configuration identity binding remains
unproven.

Six native controls compare fixed/floating anchors and rigid/flexible child
assemblies, with unchanged controls for both families. The supported first
reference matches all six final reopened COM fixation readings. The existing
`is_flexible` XML flag matches the child occurrence's rigid/flexible readings in
all three parent cases. Missing flexible flags remain unknown; a Part's raw
Solving value of zero does not itself establish subassembly behavior.

An independent four-case witness distinguishes fixation from constraint
completeness. The underdefined floating component and fully-mated floating
component both decode as not fixed. The unchanged fully-mated control also
decodes as not fixed. The unmated fixed component decodes as fixed. Native
`GetConstrainedStatus` reports fully constrained for both the fully-mated floating
and fixed cases, while `IsFixed` distinguishes them. All four match the reader
after both reopen/rebuild passes, with unchanged copied Part bytes. False fixation
therefore must not be promoted to motion freedom.

The same child anchor reports fixed when its file is opened alone and floating
when queried as a nested occurrence in the parent. Those are different contexts.
The parent root anchor and nested anchor also reuse a Part and IDs, so identity
must retain the owning document and ancestry. No global grounding is propagated
through a floating parent. Child and Part hashes stayed unchanged during parent
edits. Some opens reported [warning 32, NeedsRegen](https://help.solidworks.com/2021/english/api/swconst/SolidWorks.Interop.swconst~SolidWorks.Interop.swconst.swFileLoadWarning_e.html?id=2.10.1.345);
final rebuilds succeeded. These captures do not prove collision-free motion.

The fully-mated witness also captures three assembly-root plane selections.
Its text-only mate reference candidates contain component-side strings while
native readings identify a separate root endpoint. Those strings are not a
complete endpoint list. The subsequent plane corpus below measures local and
assembly-frame data; complete root selection binding remains unresolved.

## Cached plane geometry

`MateRecord.cached_plane_geometry` retains explicit schema-1
`moAbsoluteRefPlnData_c` prefixes as candidates. Each contains `point_metres` and
`normal`, plus `declaration_offset` and `parameters_offset` in the configuration's
decoded mate stream. Coordinates belong to the assembly where the mate resides,
not the Part frame or an expanded top-level scene. Tangential point coordinates
are not a unique origin or a joint pivot. These fields match the point and normal
layout documented by [MateEntity2.EntityParams](https://help.solidworks.com/2018/English/api/sldworksapi/SOLIDWORKS.Interop.sldworks~SOLIDWORKS.Interop.sldworks.IMateEntity2~EntityParams.html).

The profile requires a complete mate inventory, an explicit declaration outside
framed strings, eight finite doubles within the class and record intervals, a
unit normal within a squared-length tolerance of `1e-8`, and two zero trailing
parameters. It preserves the normal without renormalizing it. Unknown schemas,
reused class tags, nonplanar values and truncated prefixes are withheld.

Three native assemblies compare two occurrences of the same Part with an unchanged
control and a 10 mm tangential translation of one occurrence. Both reopen/rebuild
passes retain the distinct occurrence poses and zero mate errors. The first
explicit plane prefix matches that occurrence's native endpoint parameters in all
three assemblies. Part-local and owning-assembly plane directions differ. Four
earlier Tech Core captures also match their first explicit plane prefixes,
including negative-Y, negative-Z and an oblique direction. The two larger captures
had load errors, so those comparisons do not validate their complete assemblies.

Candidates retain source order but have no resolved endpoint index or occurrence
owner. They are not paired with nearby text references. In the controlled corpus,
the first explicit plane cache belongs to a component endpoint; the root endpoint
is separate. Reused cache objects remain unsupported, and saved cache data may be
stale. `entity_ownership_decoded` stays false. These prefixes alone cannot define
a joint or establish motion freedom.
