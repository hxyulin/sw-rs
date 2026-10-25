<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Saved identities and reference decoding

SolidWorks has persistent references. `GetPersistReference3` returns an opaque
array associated with an object and model; `GetObjectByPersistReference3`
resolves one back to an object. Its documentation explicitly allows the byte
representation to change across rebuilds/releases while reference lookup remains
consistent. An API persistent reference is therefore not equivalent to a feature
name, an arbitrary recovered integer, or our source-record index.

Primary source: [GetPersistReference3](https://help.solidworks.com/2023/english/api/sldworksapi/SolidWorks.interop.sldworks~SolidWorks.interop.sldworks.IModelDocExtension~GetPersistReference3.html).
This API contract does not specify the native on-disk encoding or prove that its
returned bytes occur verbatim in a saved file.

## Current interfaces

`DocumentReader::features()` decodes `swXmlContents/KeyWords` into a cached
`FeatureCatalog`. Records retain the saved XML tag, name, type, raw attributes,
raw ID, positive numeric source ID when valid, source element index/byte span,
and nearest containing feature. Duplicate IDs and reserved `-1`/FFFFFFFF/zero
tokens remain visible. Unknown tags remain saved records rather than inferred
operations. Configurations and dimensions are not feature identities. XML nesting
is not a dependency edge and saved order is not guaranteed regeneration order.

`DocumentReader::references()` builds a cached `sw-rs-references-v1` graph from
that catalog, metadata's existing mate/visual evidence, and structural XML in
`swXmlContents/Features` and `swXmlContents/COMPINSTANCETREE`. It uses the same
decoded container and does not request geometry. `finish()` moves requested
projections to `ReadOutput`. Convenience metadata/geometry output is unchanged;
these new projections are opt-in.

```sh
mbx run --locked --bin sw-dump -- references part.SLDPRT
```

WASM exports `references_json(bytes, filename)` with the same graph. Graph node
indices are local to a source SHA-256 and schema, not cross-save persistent IDs.
Source locators refer to decoded stream bytes, never compressed file offsets.

## Native document-handle prefixes

The opt-in graph inspects `ThirdPtyStore/VisualStates`, exact
`Contents/Config-N-MatesList` streams, and matching `Config-N-ResolvedFeatures`
streams when the same literal configuration has a mate stream. It reads supported
explicit `moFaceRef_c` and `moEdgeRef_c` prefixes. `NativeDocumentReference::parse_prefix` exposes the same checked decoder
at a caller-supplied decoded-stream offset.

The observed prefix has five u32 words beginning with `1, 0`, the repeated
16-byte component marker and a zero u16. An explicit surface/edge representation
class follows, then a zero u16 and explicit `moFR_c`, `moExtObject_c` and
`moCStringHandle_c` declarations. The final UTF-16 CString is retained exactly.
Every declaration must use schema 1. Reused class tags and unknown layouts stop
the prefix; no archive slots are guessed and no unknown payload is skipped.

`ReferenceRecord::NativeDocumentReference` retains the entire checked prefix span,
raw words, entity class, saved document handle and its CString offset.
`reference_class` identifies edge roots; face roots omit this field in JSON for
compatibility. For supported configuration streams, the native index comes only
from the exact stream name.
Visual-state configuration ownership remains unknown. The prefix ends at the
CString, not at a validated complete native-object boundary.

Its `native_document_handle` link remains unresolved. Saved paths can be stale,
external or blank. They are not opened, rewritten, matched to a basename, or
assigned to a nearby appearance or mate. Both native ownership flags remain false.
Ordinary metadata and geometry loading do not request this additional scan.

The 2026-10-25 public audit finds 67 complete visual-state prefixes: 64 nonblank
handles and three blank handles. All prefix words, class names, strings and spans
agree with an independent byte walker. All 344 modern metadata source entries
remain identical to their prior projection; those entries include three duplicate
source hashes. The 46 explicit mate face-reference candidates all stop at an
unknown reused class: 41 at the handle class, four at the entity class and one at
`moFR_c`. A missing supported prefix is not proof that a file has no references.

These are the same pinned public files used by the
[grouped-descriptor audit](surface-reference-rules.md#grouped-descriptor-records).
Per-file evidence and the independent walker remain ignored under
`local-assets/modern-reference-investigation/`. Complete class-map reconstruction
and current entity/occurrence ownership remain separate work.

### Reused handle candidates and stopping diagnostics

The opt-in reference graph now exposes `native_reference_stops` when a scanned
prefix reaches an unsupported field. Each stop retains its source stream,
prefix offset, failed-field offset, stage, raw class tag when available, and an
explicit class candidate when present. `NativeDocumentReference::inspect_prefix`
returns the same diagnostic; `parse_prefix` still accepts only explicit prefixes.

For the handle-class branch, the graph can additionally retain a
`native_document_reference_candidate` record. Its separate
`native_document_handle_candidate` link stays unresolved. This is a framed-text
candidate, not reused-class decoding or a reconstructed archive map.

The candidate requires all preceding classes to be explicit schema-1 declarations,
then a short class tag and valid CString. The exact corresponding `Contents/Config-N`
stream must start with `moAssembly_c` or `moRouteAssembly_c` and contain exactly
one supported contiguous frame:

```text
explicit moExtFeature_c
explicit moCStringHandle_c
CString
short class tag
CString
```

The intervening configuration tag must equal the unknown handle tag. No path or
name comparison supplies this relationship. The record retains the candidate
CString, its complete prefix-framing span, raw header words, explicit entity
class, and the original unresolved-class diagnostic. `class_evidence` retains
the configuration stream, root class, declaration/frame/string spans and exact
tag offset. Configuration text is not copied into every candidate.

Missing, multiple or malformed context frames reject the candidate. A model
header, manager stream or another configuration cannot supply the context.
Object/null tags, unsupported classes earlier in the prefix, wide tags and
invalid strings remain unsupported. Counted string contents are excluded from
nested declaration scans, including when a later frame field fails.

The initial candidate validation retains all 67 explicit visual prefixes and adds 41 mate
handle candidates, including one blank string. All 46 class stops remain visible:
a handle candidate does not remove its class-map blocker. An independent byte
walker checks both source intervals and the exact saved candidate text. Ordinary
metadata remains identical across the 344 modern baseline entries.

The remaining implementation blocker is the complete serialization call sequence
through `Config-N` and `Config-N-ResolvedFeatures`. Matching frame tags cannot
establish object registrations, local class slots, archive order or current
selection ownership. No synthetic empty map slots or declaration-order dictionary
are inserted. See the [public prefix investigation](archive-reference-rules.md#public-mate-prefix-follow-up-2026-10-25).

## Sequential reference subtrees

`NativeSubtreeReader` implements supported reference payloads using a caller-supplied
`ArchiveContext`. It resolves class and object tags through that actual shared map,
including wide tags. It does not infer slots from scanned declarations, saved
paths, the leading mate-stream word, or the conditional trial dictionaries.

The supported payloads are:

- `moCStringHandle_c`: one checked CString.
- `moExtObject_c`: two independently serialized handle pointers, a checked
  saved byte (2, 3 or 5), a checked 16-bit word (0 or 0x0100), a raw u32, saved CStrings, fixed raw scalar blocks and a
  configuration-looking CString. That text does not establish ownership.
- `moFR_c`: an external-object pointer and two raw u32 words.
- `moSimpleSurfIdRep_c`, `moFromSktEntSurfIdRep_c`, `moFilletSurfIdRep_c`
  and `moThinOffsetSurfIdRep_c`:
  word(0), an FR pointer and one raw trailing word.
- `moEndFaceSurfIdRep_c`: the same prefix with two trailing words.
- `moDerivedSurfIdRep_c`: word(0), an FR pointer, a recursive representation
  pointer and one trailing word.

- `moEndFace3IntSurfIdRep_c` and `moFromSktEnt3IntSurfIdRep_c`: word(0),
  an FR pointer and three raw trailing words.
- `moLPatternSurfIdRep_c`: word(0), an FR pointer, a recursive representation
  pointer and two raw trailing words.

The trailing words retain their saved values, including `0xffffffff`; their
meanings are not inferred from class names. `moSideFace3IntSurfIdRep_c` and other
unlisted families remain unsupported.

Reference, handle and log classes require saved schema one. The model-header
string arrays require schema zero. Nulls and duplicate pointers are retained.
A backreference must point to a complete object of the expected type already
consumed by this reader. A known class header without a decoded payload is
insufficient. Cycles into in-progress payloads remain unsupported and stop.

`NativeExternalObjectProfile` is explicit; no universal version gate is claimed:

| Profile | Saved CStrings | Raw bytes before configuration CString | Raw trailing bytes |
| --- | ---: | ---: | ---: |
| `TwoStrings` | 2 | 17 | 12 |
| `ThreeStrings` | 3 | 18 | 16 |
| `ThreeStringsShortTail` | 3 | 18 | 12 |
| `ThreeStringsShortBlock` | 3 | 17 | 12 |

Both handle pointers precede these fields. The scalar blocks remain raw because
their field meanings and some byte boundaries are unverified. Both profiles are
checked against independently located FR-field boundaries in projections of the
five public samples. Those private projections rewrite archive tags; one also
substitutes an unavailable handle reference. They verify field consumption, not
original archive-map continuity or identity.

`ThreeStringsShortTail` preserves the following FR words in controlled SW2022
Part samples. Choosing `ThreeStrings` there consumes the FR's first word as part
of the external object's tail. A structurally successful prefix read alone cannot
establish which profile applies. Select the profile from independently established
field and sibling boundaries; do not infer it from version, blank document text
or the number of saved strings alone.

`read_reference_array_prefix` consumes the supported face/edge header, component
marker, zero word and the declared surface-pointer count. It retains null entries
and recursive pointers in source order. It **does not consume the enclosing
face/edge object's trailer**. Its arena record has `complete_payload: false`;
the checked span is a prefix, and it cannot satisfy a complete-object backreference.
The caller must decode the remaining parent fields before advancing to a sibling.
`read_surface` and `read_external_object` consume their complete supported payloads.

`read_face_reference` consumes a complete supported `moFaceRef_c` payload. It
requires saved header word three (zero-based) to equal 768 and an explicitly
selected `NativeFaceReferenceProfile`:

| Profile | Checked trailer words |
| --- | --- |
| `ThreeWords` | `[0, 0, 0]` |
| `FiveWords` | `[0, 0, raw, 0, 0]` |

The raw middle word is retained without assigning it a version meaning. Complete
faces use the `FaceReference` payload and `complete_payload: true`. The next object
can be read at the returned cursor without scanning. A later complete-face
backreference must use the same trailer profile. A previously recorded array
prefix remains incomplete and cannot satisfy that backreference. These profiles
apply only to the supported face header. Edge objects use their own profiles.

Conditional local relocations of 145 public stream segments consume two adjacent
face objects and both trailers: 79 with `TwoStrings`/`ThreeWords`, 25 with
`ThreeStrings`/`FiveWords`, and 41 with `ThreeStringsShortTail`/`FiveWords`. The
relocations rewrite only checked local pointer positions into an authored map;
they do not recover or import the original inherited native context. See
[face-reference trailers](archive-reference-rules.md#face-reference-trailers-and-short-external-tails).


`read_edge_reference` consumes the supported complete `moEdgeRef_c` layout with
saved header word three (zero-based) equal to 512. It uses an explicit
`NativeEdgeReferenceProfile`:

| Profile | Checked trailer words |
| --- | --- |
| `TwoWords` | `[0, 0]` |
| `FourWords` | `[0, 0, raw, 0]` |

The `EdgeReference` payload retains all header, surface-pointer and trailer fields.
The object is complete and can satisfy later edge backreferences with the same
selected profile. Face and edge profiles cannot be exchanged. Header words 0,
518, 768, 772 and 774 also occur in edge declaration candidates; the complete-edge
reader rejects them rather than masking bits or guessing their trailers. Unknown
child classes still stop. Two public source intervals support complete adjacent
edge reads under local tag relocation: one `TwoStrings`/`TwoWords` pair and one
`ThreeStrings`/`FourWords` pair. Neither establishes the inherited native map.

Every public read rolls back the cursor, shared-map registrations, decoded values
and pointer budget after failure. Previously decoded objects remain unchanged.
The default cumulative budget is 100,000 pointer occurrences and depth 64;
`with_limits` can reduce these, and depth is always capped at 64. Unknown schemas,
classes, wrong pointer types, missing values, invalid strings, excessive counts
and unsupported layouts stop without resynchronizing.

`archive_mut` lets an enclosing sequential decoder append known class/object or
explicit MapObject operations for layouts outside this reader. It must preserve
the context's identity and must not guess missing registrations. `read_string_handle`
can consume handles directly in a configuration payload before later references
reuse them. Adopted map entries without decoded values still stop on backreference.

The reader stores definitions once by archive object slot. Pointer occurrences
retain their own source stream and header span. Definitions retain their defining
stream and checked span, so cross-stream backreferences do not acquire the later
stream's provenance. These are archive identities, not current geometry owners.

### Model-header prefixes

`read_model_header_prefix` walks the saved `moHeader_c` prefix with the same typed
archive reader. It consumes two `su_CStringArray` pointers, a `suObList` of
`moLogs_c` records, two raw scalar words and a `moExtObjectList_c` pointer. Each
log contains a short-count stamp collection, a raw word and a CString label.
Each `moStamp_c` contains ten raw bytes and a CString. External objects use the
caller's explicit `NativeExternalObjectProfile`.

`NativeModelHeaderProfile::DirectArrays` starts directly with the two string arrays.
`LeadingWord` preserves one raw u32 before them. Neither profile establishes a
version gate. Schema-zero string arrays retain blanks, duplicates, order and exact
frame offsets. Log labels, numeric words and stamp bytes remain saved evidence;
they do not assign current users, dates, features or owners. All collections in
this profile use supported short counts; extended counts stop.

`ModelHeaderPrefix` records are incomplete because fields after the external-object
list remain unread. Child arrays, logs, stamps, lists and supported external objects
have complete typed definitions. Later known serialization operations can reuse
them through the same `NativeSubtreeReader`. The cursor, map, values and budget
roll back together if any field fails. No intervening class is scanned or skipped.

Python and Rust independently agree on 922 explicit profile trials across 349
original public model-header streams: 169 use direct arrays and 180 use a leading
word. Multiple external profiles can describe the same prefix when its external
list is null. Prefix success does not choose a universal external-object profile.
These walks preserve source hashes, object/class slots, pointer occurrences,
strings, scalar bytes and exact spans without rewriting tags or importing slots.
They establish a local model-header map prefix, not a whole-file native graph.

The configuration archive still has a separate bootstrap problem. In 347 checked
matching resolved-feature streams, the first supported common name tag equals
`0x8000 | (saved external-list count + 4)`; two other checked headers lack that
stream. This correlation does not prove the intervening `MapObject` calls or their
order. It is not used to seed a configuration or mate map.

### Complete direct-array model headers

`read_model_header` consumes the established `DirectArrays`/`TwoStrings` layout
through its final scalar words. Other profile combinations remain unsupported by
this complete read. The prefix reader continues to support both initial layouts.

After the shared fields, the complete reader consumes a raw u32 and another
external-object pointer, 42 raw scalar bytes, a null or empty `suObList`, three
scalar words, an optional 80-byte block and three final scalar words. The third
word before the optional block must be zero or one. Numeric block bytes are
retained exactly; no geometry, units or owner is inferred. Nonempty trailing lists
and unknown presence words stop without skipping bytes.

`ModelHeader` records have `complete_payload: true`, shared `NativeModelHeaderFields`
and a `NativeModelHeaderTail` with its exact source span. Prefix fields stay flat
in JSON. A complete root can satisfy later same-profile backreferences; an
incomplete prefix cannot. The reader consumes the object layout without requiring
EOF, so a caller can continue with the next known serialization operation.

On original public streams, Python and Rust independently consume 166 complete
headers through EOF with matching slots, pointer occurrences, fields and spans.
The optional block is present in 163 and absent in three. These complete objects
are local to their model-header archive. They do not establish the configuration
handoff, mate endpoints, current dependencies or appearance ownership.

`read_model_header_with_tail` also consumes three established leading-word layouts.
It requires the saved leading word to equal one and an explicit
`NativeModelHeaderTailProfile`:

| Header profile | External-object profile | Tail profile | Extension words |
| --- | --- | --- | ---: |
| `DirectArrays` | `TwoStrings` | `NoExtension` | 0 |
| `DirectArrays` | `ThreeStringsShortBlock` | `Extension28` | 7 |
| `DirectArrays` | `ThreeStringsShortTail` | `Extension32` | 8 |
| `LeadingWord` | `ThreeStringsShortTail` | `Extension32` | 8 |
| `LeadingWord` | `ThreeStringsShortTail` | `Extension64` | 16 |
| `LeadingWord` | `ThreeStrings` | `Extension68` | 17 |

The extension follows the common three final words. Its first word is checked as
one for `Extension28` and `Extension32`, two for `Extension64`, and zero/one/two for `Extension68`.
Subsequent words must be zero or one. All values remain raw; no flag, version or
count meaning is assigned. Unknown combinations or words stop transactionally.
Complete-header backreferences must match both the initial and tail profiles.

Python and Rust consume 176 direct-array and 182 leading-word headers through
EOF, covering all 358 model-header streams in the pinned public modern corpus.
This includes two direct-array headers with 28-byte extensions and one with a
32-byte extension. Other saves and the configuration handoff remain open.

Saved external objects retain `raw_header_byte` for observed values 2, 3 and 5,
and `raw_prefix_word` for zero or 0x0100. Neither field is assigned an API enum,
virtual-component state or current owner. The explicit CString/scalar profiles
retain every saved string instead of folding extra CString frames into raw bytes.

### Select complete streams through the reader

`DocumentReader::model_headers()` and `sw-dump model-headers MODEL` inspect the
supported layouts without requiring the caller to choose one. Selection requires
exactly one complete non-null `moHeader_c` object consuming EOF. Six attempts run
with independent fresh contexts. Unsupported/ambiguous streams remain reported
with no selected definitions; there is no prefix fallback or version-based guess.

`NativeModelHeaderReport` retains literal source names, original source hash,
attempt outcomes and the unique successful layout's root, archive slots and typed
objects. The result is cached independently of metadata and geometry and moves
through `ReadOutput`. See [the reader contract](reader-inventory.md#complete-model-header-reports)
and [CLI inspection](cli.md#complete-model-header-inspection).

### Probe a fresh archive only

```sh
cargo run --locked --example inspect_native_subtree -- model.SLDASM 'Contents/Config-0-ResolvedFeatures' 8378 two_strings
```

This example deliberately supplies an empty archive. Its JSON labels that context
and reports either the checked definitions or an exact error. It does not fill
missing slots. An arbitrary saved stream offset normally needs an inherited map;
the command is a diagnostic probe, not a whole-file graph decoder. The optional
kind selects `array` (a prefix), `model_header` (direct arrays),
`model_header_word` (leading word), `model_header_full` (complete direct-array
layout), `model_header_full_word32`, `model_header_full_word64`,
`model_header_full_word68`, `model_header_full_direct28`,
`model_header_full_direct32`, `surface`, `external`, `face_three_words` or
`face_five_words`, `edge_two_words` or `edge_four_words`. The complete
header/reference kinds accept a final count from 1 through 10,000
for sequential reads with one shared map. Other kinds accept only one read, so
an incomplete prefix cannot silently skip to a sibling. `root` retains the first
successful pointer; `roots` lists all successful reads. A failed later read rolls
back that object while preserving earlier completed objects.

### Public-file result

Extending prefix inspection to edge roots and matching resolved-feature carriers
retains 53 handle candidates: 47 face prefixes and six edge prefixes, with two
blank strings. This adds 12 candidates to the initial 41. The 67 fully explicit
visual prefixes remain unchanged. There are now 70 class stops across the wider
scan; candidates do not remove their unresolved class tags.

All five earlier edge chains now retain their own saved-handle candidates.
Those records are not assigned to the later blocked face references. The five
original face stops remain unchanged. Starting each edge subtree with an empty
map stops at its first inherited handle class and rolls back all registrations.
Metadata remains unchanged across all 344 modern baseline entries. Private audits
and projected payloads live in `local-assets/native-subtree-implementation/`.

Remaining work includes the verified `Config-N` / resolved-feature archive
handoff, unsupported face/edge header and trailer variants, and other surface
families. The payload reader supplies supported complete face layouts and typed
backreferences; it does not reconstruct that missing context.

## Resolution rules

- `resolved`: XML feature containment, or a unique correctly typed same-stream
  `swFileRef`/`swModelRef`. This establishes only the indicated syntactic relation.
- `candidate`: a unique matching feature source ID for a `moCompFeature_c` visual
  source word. This is a research join, not native identity
  proof. Timestamp, class semantics and configuration scope need verification.
- `ambiguous`: multiple saved records match. No first match is selected.
- `unresolved`: missing/wrong-type ID, undecoded native owner or mate selection.
  Editable occurrence-looking strings are retained without splitting or matching
  names. Body and mate-header IDs never enter the feature-ID namespace.

XML IDs never cross streams. Native feature matches never become face/body
assignments, component joints, renderer overrides or solved constraints.
`native_object_graph_decoded` and `topology_ownership_decoded` remain false.
This release establishes a common representation and measurable research links;
it does not complete persistent-reference decoding.

## Work needed for native resolution

The subsequent [archive-rule investigation](archive-reference-rules.md) confirms
shared MFC map accounting for the configuration-header prefix and introduces a
checked `ArchiveContext`. Complete mate/visual payload walks remain pending.

The intended runtime resolves saved structural references. Numeric/name matching
is only an investigation tool. The next decoding stages are:

1. Walk supported binary object layouts sequentially and reconstruct each archive's
   class/object reuse map. An inline class declaration gives framing, not the
   boundary of an entire object; scanning later declarations cannot recover that
   map reliably. Unknown payloads must stop or remain explicitly unsupported.
2. Decode typed feature/body/surface identity records, including stamps and scope,
   then resolve references through those maps. Serialized object indexes, feature
   source IDs and persistent topological IDs are separate namespaces.
3. Connect native geometry and occurrence context to the references carried by
   appearances and mate selections. Cached display-table offsets are not B-Rep IDs.
4. Verify against controlled SolidWorks saves/API exports: rename/reopen/rebuild,
   configuration changes, repeated component instances, and face splits/deletions.
   API persistent-reference bytes provide an external oracle, not an assumed wire
   layout. FIXME: these controlled comparisons remain unavailable.

## Corpus check

Opt-in cache records and structured mate surface descriptors are now available;
see [surface-reference rules and the expanded census](surface-reference-rules.md).
`surface_references()` caches bounded metadata inspection;
`references_with_geometry()` composes it with the base graph. Source-ID matches
remain candidates and native topology flags remain false.

`audit_references` was run on the same 340 EGA documents and 240 design-library
documents used by the reader inventory; all source SHA-256 values match that
inventory. All 487 supported modern documents have a decoded Keywords catalog:
23,390 saved feature-tree records, 22,932 distinct numeric source IDs summed per
document, and no duplicate numeric IDs within these catalogs. The 93 legacy OLE
documents were excluded from that snapshot; see the subsequent
[OLE loading results](parsing-coverage.md#ole-follow-up-2026-10-25).
These counts do not establish cross-save stability.

The XML graphs contain 11,496 resolved typed stream-local reference links and 27 saved
feature-containment links. Visual feature source words produce 31 unique catalog
candidates and 15 unmatched references. No candidate establishes timestamp or
topology ownership. An investigative comparison found zero catalog matches for
the 2,675 mate header identifiers; the implementation therefore retains those
words in their own undecoded namespace rather than joining them to feature IDs.
All 5,413 mate selection references remain unresolved endpoints.

```sh
mbx run --release --locked --features archive --example audit_references -- PRIVATE.zip
```

Only counts and authored synthetic fixtures belong in Git. Private per-file
audit JSONL, hashes and logs remain in ignored `local-assets/reference-investigation/`.
