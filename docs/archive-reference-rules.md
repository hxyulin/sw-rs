<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Binary reference rules: confirmed framing and open native layouts

This investigation separates documented MFC framing, native-file observations,
and unverified SolidWorks-specific interpretations. MFC documentation is not a
complete SolidWorks file specification. API persistent references are a fourth,
separate interface; their public contract does not define their on-disk encoding.

## Documented archive behavior

Microsoft's [TN002](https://learn.microsoft.com/en-us/cpp/mfc/tn002-persistent-object-data-format?view=msvc-170)
describes archive-local classes and objects sharing a sequential map. Null is
slot zero. A new class occupies one slot and its new object another. Reusing a
class allocates a fresh object; referencing an existing object allocates nothing.
Registration precedes payload serialization, so backreferences can form cycles.

Short little-endian tags distinguish null (`0000`), class declarations (`FFFF`),
old classes (`8000 | index`), and object references. `7FFF` introduces a wide
index with the class discriminator in bit 31. A declaration contains a 16-bit
schema, a 16-bit ASCII-name length and that name. The schema is not the document
version. These indices belong to an archive context, not a CAD entity namespace.

The [CArchive API](https://learn.microsoft.com/en-au/cpp/mfc/reference/carchive-class?view=msvc-170)
also exposes `SerializeClass`/`ReadClass`, which can register a class without an
object, and `MapObject`, which registers an external object without serializing
its payload. Correct call sequencing therefore matters; class-declaration order
alone does not establish class indices.

No general object-length field is supplied by these tags. Class-specific payload
readers must consume all nested serialization operations before advancing to the
next header. A marker scan cannot know which nested classes/objects were skipped.
Unknown payloads must stop a sequential walk or remain explicitly opaque.

## Confirmed native configuration-header profile

For `Contents/CMgrHdr2` in all 487 supported modern corpus documents, sequential
decoding establishes this map prefix:

```text
slot 1  class dmConfigMgrHeader_c
slot 2  root manager object
slot 3  class dmConfigHeader_c
slot 4  first configuration object
slot 5  second configuration object, header tag 0x8003
...
```

The root's count and each configuration's version-gated scalar/string fields
are read using the existing openswx-derived profile. The word preceding a
configuration's name remains raw evidence; it is not assumed to be the native
configuration index, which follows the name in this profile. Root and config
classes use saved schema 1 in this corpus. The metadata compatibility path also
retains its schema-0 fixture profile; unknown schemas/classes stop that walk.

The audit covers 2,512 configuration records, including 2,025 reused class tags,
974 class slots and 2,999 object slots. Independently written Python and Rust
prefix walkers agree on names, indices, offsets and raw header words. All file
hashes match the prior source inventory. These are source-entry counts, not
unique designs or cross-save identity tests.

There are still opaque trailers: 420 streams retain 8 bytes; 67 retain 12 bytes.
Those bytes are not silently skipped and no complete root-object boundary or
whole-document reference graph is claimed. At that snapshot, all 93 legacy OLE
documents remained outside the runtime container profile. Native versions observed are 12000,
13000, 14000, 16000, 17000 and 18000; no universal version support is inferred.

## Mate-stream context hypothesis

All 45 nonempty modern mate streams begin with two header fields followed by an
explicit first mate-class declaration at byte 6. In 33 of these streams, later
common mate headers reuse exactly `8000 | first_header_word`. This accounts for
1,089 reused records: 1,063 in EGA and 26 in the design library. There are no
contradictory inline declarations at those reused headers in the observed set.

The leading word may therefore preserve the next map index from an earlier
archive context: its value would identify the first new class in this segment.
This could explain why later tags reference classes that are never declared
in the mate stream itself. It is a strong corpus-backed hypothesis, not yet a
decoded checkpoint format or independently verified mate-family assignment.

The audit emits `hypothesis_requires_verification`; production `MateRecord.kind`
still requires an explicit native class declaration. No inferred families,
constraints, mate endpoints or solver behavior have been added. FIXME: establish
which preceding stream/context supplies the map, verify checkpoint behavior on
controlled saves, and reproduce subsequent class-slot assignments through all
intervening nested payloads. A fake initial array of empty slots would hide this
missing evidence rather than resolve it.

## Public mate-prefix follow-up, 2026-10-25

The pinned Talaria Robotics and GeoKylin public samples contain 46 explicit
`moFaceRef_c` candidates across 40 source files. Counts include separate
configuration streams in the same file. A checked walk of each supported prefix
stops at these class positions:

| First unsupported position | Candidates | Raw short class-slot candidates |
| --- | ---: | --- |
| `moCStringHandle_c` | 41 | 13 through 46, with gaps |
| Entity representation | 4 | 212, 215, 273, 312 |
| `moFR_c` | 1 | 257 |

These are archive-slot candidates extracted from raw tags, not CAD entity IDs.
No prefix in this set reaches a document handle through a fully explicit chain.

### The handle tags correlate with the matching configuration stream

For all 41 handle failures, the matching `Contents/Config-N` stream contains an
explicit `moCStringHandle_c` declaration followed contiguously by a framed string,
a short tag, and another framed string. That short tag equals the blocked mate
tag in **41 of 41** cases. Applying the same comparison to `Contents/CMgr` gives
one match; `Contents/Config-N-ModelHeader` gives zero matches. The declaration
alone would not establish this correspondence.

All 41 blocked mate tags are below their mate stream's leading word. Under the
existing next-map-slot hypothesis, these classes were registered before the mate
segment. This makes the matching `Config-N` stream the next target for a
sequential decoder. It does not prove its complete serialization call sequence
or establish which slots came from `MapObject` or class-only serialization.
Tag equality and adjacent strings remain framing evidence, not a recovered map.

### The earlier failures need feature and mate payload layouts

Three entity failures have tags above the mate stream's leading word: slot 312
versus 277, slot 212 versus 186, and slot 215 versus 189. Each stream has earlier
`moEdgeRef_c` and surface-representation declarations. Under the same hypothesis,
these are local reuse after edge-reference serialization. Skipping those
payloads would lose the object allocations needed to assign the later slots.

The other entity failure has slot 273 versus leading word 287. Its matching
`Contents/Config-1-ResolvedFeatures` contains an explicit `moSimpleSurfIdRep_c`
declaration at byte 9973. The `moFR_c` failure has slot 257 versus leading word
276; its matching `Contents/Config-0-ResolvedFeatures` contains an explicit
`moFR_c` declaration at byte 8461. These are candidate predecessor declarations.
No sequential walk yet binds them to those slots. The evidence supports
investigating `Config-N`, then `Config-N-ResolvedFeatures`, then `Config-N-MatesList`;
it does not establish that archive order as a format rule.

### Reproduce the stopping points

```sh
cargo run --locked --example audit_mate_prefixes -- assembly.SLDASM
```

The example reports source SHA-256, exact stream names, candidate offsets, the
first unsupported prefix position, raw tags and contiguous carrier frames.
The explicit-prefix walk stops at the unknown class. A separate candidate path
checks the following CString framing without asserting its class. Declaration
scans are labeled candidates; they are not object boundaries. Both map recovery
and topology ownership remain false. Ordinary metadata and geometry do not run
this scan.

An independent Python walker agrees with the Rust example on all 46 stopping
stages, offsets and tags, and on all 123 compared carrier frames. The probes,
per-source reports and extracted bytes remain ignored in
`local-assets/mate-prefix-investigation/`. No public sample bytes are distributed.
The opt-in graph now preserves the 41 handle cases as
[separate candidates](references.md#reused-handle-candidates-and-stopping-diagnostics),
with both source spans and the unresolved class stop. This does not recover a map.
Before adding reused-class support, decode and validate the intervening payloads
and map registrations. A controlled save can then test whether adding one object
moves the predicted slots without changing the selected entity.

## The five earlier stops: edge-reference context

A follow-up traced all five non-handle stops to an earlier explicit `moEdgeRef_c`
chain. Three chains occur earlier in the same mate stream; two occur in the
matching `ResolvedFeatures` stream. All five pinned source hashes and decoded
stream CRCs were checked again. The following are raw little-endian words at the
stopping position, not a decoded dictionary:

| Public sample | Stop offset | Raw continuation | Earlier edge declaration |
| --- | ---: | --- | --- |
| `Photo360/Study-Photo360.SLDASM` | 2124 | `8101 8103 8011`, then CString | ResolvedFeatures: 8378 |
| `Study-Toolbox.SLDASM` | 4842 | `8138 0000 8133 0136` | MatesList: 1776 |
| `Study-模具设计.SLDASM` | 5496 | `80D4 0000 80D6 00D9` | MatesList: 1774 |
| `Study-装配体2.SLDASM` | 5021 | `80D7 0000 80D9 00DC` | MatesList: 1760 |
| `terminalBlocks.SLDASM` | 15344 | `8111 0000 8113 8115 0117 0118` | ResolvedFeatures: 9918 |

The surrounding layouts suggest three different reuse forms:

- Photo360 reuses the FR, external-object and handle classes, followed by a new
  framed string. Its text differs from the earlier edge's saved document text.
  Reusing a class cannot justify copying the earlier object's handle.
- Toolbox, mold and assembly-2 appear to reuse an external object. At the
  hypothesized external-object pointer position, their words are respectively
  `0136`, `00D9` and `00DC`, which have the short object-reference tag form.
  Those slots are one above the candidate external-object class slots in the
  earlier edge chains. No new document CString follows those words.
- The terminal-block case appears to create a new external object using two
  existing string handles: `0117` and `0118`. Their positions follow the reused
  external-object class tag `8115`. The earlier edge chain contains two framed
  strings. Object-slot accounting is still needed to associate them.

These are structural hypotheses. The earlier surface-class declarations suggest
`moFromSktEntSurfIdRep_c` for Toolbox, `moDerivedSurfIdRep_c` for the mold sample,
`moFromSktEntSurfIdRep_c` for assembly-2 and `moSimpleSurfIdRep_c` for terminal
blocks. No production class assignment or saved-handle candidate is added from
these correlations. The original five stops remain unchanged.

### Conditional surface-array trials

Private trial walkers supplied the hypothesized class tags explicitly and consumed
only the proposed payload layouts, without resynchronizing after unknown bytes.
They are not archive-map reconstruction. In the reused external-object branch,
the trial leaf shape is:

```text
entity class tag, zero u16, FR class tag, external-object reference,
three raw u32 words
```

This consumes 20 bytes for the proposed FromSktEnt/Fillet leaf profiles. The
EndFace profile consumes one extra u32. The proposed Derived profile contains two
raw u32 words after the external-object reference, then a nested representation
and a trailing u32. None of those words is promoted to a topology ID or timestamp.

With these assumptions, Toolbox, mold and assembly-2 each consume five non-null
outer records and one null entry. Their third raw face-header word is six in each
case. The conditional walks end at decoded offsets 4952, 5706 and 5127. These are
array-trial endpoints, not complete native-object boundaries.

The terminal-block sample has a third raw header word of 157. After its opaque
first record, 155 consecutive 20-byte records occupy offsets 15439 through 18539,
followed by a zero word. This is consistent with one first record, 155 later
records and a null entry. The first record's external-object payload remains
opaque, so no complete sequential walk is claimed. The header word is still raw
in the public API; count semantics need broader and controlled validation.

### Next decoding target

The [sequential subtree reader](references.md#sequential-reference-subtrees) now
implements the supported child payloads and typed backreferences against a real
caller-supplied map. Array roots remain incomplete prefixes. The public five still
need their inherited context; the trial dictionary is not used in production.

The remaining target is the `moEdgeRef_c` subtree and its `moFR_c` /
`moExtObject_c` payloads. A reader must handle new classes, class reuse and object
backreferences separately, including both saved string handles. Then the leaf
and recursive surface layouts can be checked against a real shared map. Skipping
an opaque first object or seeding empty slots would invalidate the later indices.

`audit_mate_prefixes` now includes raw stop lookahead and reference-declaration
candidates in both mate and feature carriers. It never assigns those declarations
to raw slots. Per-source extracts, pointer-position probes and conditional array
reports remain ignored in `local-assets/mate-remaining-investigation/`.

## Topology identity is another decoding layer

The follow-up [surface-reference investigation](surface-reference-rules.md)
implements the saved tuple grammar and bounded cache inspection. It preserves
numeric/opaque tails and conflicts without claiming native topology ownership.

The pinned Apache-2.0 cadmpeg implementation distinguishes full surface identity
from archive slots: framed `SurfIdRep` strings can carry feature source, local
surface ID and ordered trailing fields; native `FACE_ID_2001` attributes provide
the corresponding geometric identity. A source/local pair can be insufficient
when complete identities differ in their tails. See the pinned
[appearance implementation](https://github.com/cadmpeg/cadmpeg/blob/4df21312c62cc1207a4c2274e29b4118fd232ec6/crates/cadmpeg-codec-sldprt/src/appearance.rs)
and [B-Rep model](https://github.com/cadmpeg/cadmpeg/blob/4df21312c62cc1207a4c2274e29b4118fd232ec6/crates/cadmpeg-codec-sldprt/src/brep.rs).

Our parser does not yet decode that B-Rep ownership graph. Class-map recovery
alone consequently cannot bind every appearance or mate selection to a face.
Feature timestamps, native body/face records, occurrence context and configuration
scope still matter. Cached display-table offsets remain cache identities.

## Independent prior art

[SWFormat](https://github.com/KenM76/swformat/tree/69b80c860241fdcbfa334f67cb951f76398c1a71)
is Apache-2.0 at inspected revision `69b80c860241fdcbfa334f67cb951f76398c1a71`.
Its `carchive/objmap.py` independently implements shared-slot bookkeeping, but
requires a caller-provided body-length/nested-index oracle. Its configuration
name reader scans supported record prefixes; it is not a complete sequential
native object graph decoder. No SWFormat source was copied into this change.
Our tag/map helper was independently implemented from Microsoft documentation
before inspecting that implementation. No Microsoft implementation source,
SolidWorks installation files, shaders, textures or CAD are distributed.

## Implemented interfaces and reproduction

`ArchiveContext` with `BinaryReader` now supports class-only headers, new objects,
class reuse, null/object backreferences, wide indices and explicitly mapped
external objects. Header failures roll back both cursor and map. Missing and
wrong-kind slots fail; no scan, guess or resynchronization is performed. New
objects are registered before their caller-decoded payload, supporting cycles.
The existing configuration-name parser now uses that map and validates the
expected payload class instead of accepting any high-bit tag.

```sh
mbx run --release --locked --features archive --example audit_archive_rules -- PRIVATE.zip
```

The JSONL report includes source hashes, map slots/header spans, configuration
records, opaque-tail ranges and separately labeled mate-context hypotheses.
Native backreferences, cyclic objects and wide indices were not reached in the
known configuration prefix; those primitives are currently validated by authored
synthetic tests. Private reports, the independent Python probe and research
checkout remain ignored in `local-assets/archive-investigation/` and
`local-assets/swformat-research/`.

The next target is the serialization chain preceding `Config-N-MatesList`, then
its nested feature/reference payloads. We can validate a class-map continuation
against its reused tags before attempting endpoint or appearance ownership.

### Additional surface payload boundaries

The sequential reader also accepts schema-one `moEndFace3IntSurfIdRep_c`,
`moFromSktEnt3IntSurfIdRep_c`, `moThinOffsetSurfIdRep_c` and
`moLPatternSurfIdRep_c`. All start with word(0) and an FR pointer. The two
`3Int` classes end with three raw u32 words. Thin-offset ends with one word.
Linear-pattern contains a recursive surface pointer followed by two words.
These words retain their saved values; neither signedness nor topology meaning
is inferred.

A survey of unique public sources found 89 explicit end-face `3Int` declarations,
122 sketch `3Int`, 12 thin-offset and 18 linear-pattern declarations in matching
mate/resolved-feature carriers. These are declaration candidates, not a native
object census. Two side-face `3Int` declarations remain unsupported.

In Photo360's `Contents/Config-0-ResolvedFeatures`, a conditional structural walk
consumes the four-record edge prefix at bytes 8378 through 8940. Its records are
end-face, end-face `3Int`, and two linear-pattern records containing sketch `3Int`
children. The second pattern and child reuse short class tags. Rewriting those
tags into a fresh authored archive allows the Rust reader to consume all four
records, including both recursive children, and stop before the parent trailer.
The external object's two handle tags are rewritten too. Saved strings and
scalar fields remain unchanged. This verifies payload consumption, not original
map continuity, object identity or a complete edge object.

Separate projections preserve the scalar bytes of 79 end-face `3Int`, 104 sketch
`3Int` and 12 thin-offset declaration candidates. Each has short FR/external tags
and a following declaration, null, or short-tag/zero-word boundary candidate.
The projections replace the FR class tag and external pointer with an explicit
FR declaration and null pointer. All 195 consume their exact projected interval.
They do not validate the original pointer targets. Probes and reports remain
ignored in `local-assets/surface-layout-investigation/`.

The inherited configuration map and unread face/edge parent trailers remain the
next blockers. The original five mate-face stopping points are unchanged.

### Face-reference trailers and short external tails

Comparing parent endings exposed two three-string external-object layouts.
Both contain an 18-byte scalar block before the configuration CString. One has
16 trailing bytes, as in terminalBlocks; the other has 12, as in the controlled
SW2022 sphere and torus. Selecting the 16-byte profile in those controlled samples
moves the saved FR word `31` into the external object's tail, shifts the two FR
words and consumes one word of the parent trailer. With the 12-byte profile, the
first FR words match the later FR that reuses the same external object. The sketch
surface's saved trailing word also matches. This establishes a field boundary;
it does not establish a universal version or empty-handle rule.

The supported complete face layout has header word three (zero-based) equal to
768. Its scalar trailer has either three zero words or five words of the form
`[0, 0, raw, 0, 0]`. A separate caller-selected profile is required. The middle word
is preserved without treating it as a document version. Edge-reference trailers
are not covered by these profiles.

The survey found 404 explicit face/edge declaration candidates in unique public
sources' mate/resolved-feature carriers. A conditional local relocation probe
rewrites pointer tags only as the payload walker consumes them. It hypothesizes
a base from a repeated local handle-class tag, rejects references outside the
local segment, and never seeds a production archive with guessed slots. The
original inherited map remains unverified.

Of those candidates, 145 distinct source intervals admit two sequential complete
face reads under one profile combination each:

| External-object profile | Face trailer | Adjacent pairs |
| --- | --- | ---: |
| `TwoStrings` | `ThreeWords` | 79 |
| `ThreeStrings` | `FiveWords` | 25 |
| `ThreeStringsShortTail` | `FiveWords` | 41 |

The probe checks both scalar trailers and the second root's reused class tag at
the exact first-object end. Rust independently consumes each rewritten pair using
the complete-face API. These are payload/boundary checks on local relocations,
not native archive-map recovery or topology ownership. Other trial profiles can
consume a prefix while leaving the wrong next field, so prefix success alone is
not counted as a complete object.

In the controlled sphere, the first array ends at byte 8883 and its five-word
trailer ends at 8903, exactly at the next face object. The second object ends at
8983. The corresponding torus boundaries are 8593, 8613 and 8693. The earlier
apparent 16-byte trailers resulted from selecting the longer external-object tail.
Research scripts, rewritten projections and reports remain ignored in
`local-assets/reference-trailer-investigation/`.

`NativeSubtreeReader::read_face_reference` marks these supported objects complete
and permits subsequent complete-face backreferences with the same selected
profile. The prefix API retains its existing incomplete contract. No document
reader now walks past unknown native fields: the inherited configuration context,
unsupported classes and edge trailers still stop a whole-stream decoder.

### Edge-reference scalar trailers

The complete `moEdgeRef_c` profile requires saved header word three (zero-based)
to equal 512. Its surface-array framing matches the supported prefix API, but its
scalar trailer is shorter than the face trailer:

| Edge profile | Saved trailer words |
| --- | --- |
| `TwoWords` | `[0, 0]` |
| `FourWords` | `[0, 0, raw, 0]` |

The raw word remains uninterpreted. Profile selection is explicit and does not
follow a document-version threshold. A shared scalar-tail helper enforces the
zero positions for both face and edge readers without exchanging their lengths.

Two distinct public source intervals establish adjacent edge boundaries under
conditional local relocation:

| Source | External / edge profile | First array end | First object end | Second object end |
| --- | --- | ---: | ---: | ---: |
| `轴承2.SLDPRT` | `TwoStrings` / `TwoWords` | 9263 | 9271 | 9399 |
| `DoveWrap_v11 Lidar.SLDPRT` | `ThreeStrings` / `FourWords` | 9643 | 9659 | 9795 |

Both root objects have header word 512. The second begins with the same reused
root-class tag at the exact end of the first trailer. It reuses local classes and
objects; both second trailers match their selected profile. Rust independently
consumes each rewritten pair and retains complete object spans. These probes
establish payload consumption and sibling boundaries, not native map continuity
or topology ownership. No missing inherited object values are supplied.

The broader private projection probe can replace an unavailable inherited handle
class with an explicit declaration for a payload trial. Neither successful pair
needs that replacement: both use only local tag relocation. Failed trials remain
failed and do not become source-map entries. Other edge candidates contain header
words 0, 518, 768, 772 or 774, unknown representation classes, or inherited pointers
outside the local segment. They are not covered by the complete-edge reader.
Probes and reports remain ignored in `local-assets/edge-trailer-investigation/`.

`read_edge_reference` marks supported roots complete and permits same-profile
backreferences. A previous `ReferenceArrayPrefix` stays incomplete. A face pointer,
a different trailer profile, a truncated/nonzero checked trailer field, or an
unsupported header rolls back the failed read and preserves earlier objects.

## Sequential model-header prefix and configuration bootstrap

`Contents/Config-N-ModelHeader` begins with a saved `moHeader_c` object. The
supported prefix contains two `su_CStringArray` objects with schema zero, not one.
In the direct-array profile, the native map starts as follows:

```text
slot 1   class moHeader_c, schema 1
slot 2   model-header root object
slot 3   class su_CStringArray, schema 0
slot 4   first string-array object
slot 5   second string-array object, reused class tag 0x8003
slot 6   class suObList, schema 1
slot 7   log-list object
slot 8   class moLogs_c, schema 1
slot 9   first log object
slot 10  class moStamp_c, schema 1
slot 11  first stamp object
...
```

The second string array explains the later log tag `0x8008` and stamp tag
`0x800a`. No initial map hole or external registration is needed. The leading-word
profile adds a raw u32 after the root header; it leaves the slot sequence unchanged.
Later classes and objects register through the same checked `ArchiveContext`.

The log list uses a short count. Each `moLogs_c` contains a short-count collection
of stamp pointers, a saved u32 and a CString label. Each `moStamp_c` contains ten
scalar bytes and a CString. Those scalars remain raw. After the log list, two raw
u32 words precede the external-object-list pointer. The list uses a short count
and the selected external-object payload profile. Nulls, class reuse and complete
object backreferences retain their actual serialization behavior.

An independent Python walker and Rust agree on 847 explicit profile trials across
312 unique original public model-header streams. There are 161 direct-array and
151 leading-word streams. Comparisons cover every registered slot, non-null
pointer occurrence, class/schema, object span, string frame, raw scalar and prefix
end. Source hashes and decompressed-stream CRCs match the pinned inventory. These
checks use original bytes and fresh contexts, without relocation or guessed slots.
Multiple external profiles can succeed when there is no external-object payload;
the audits do not treat that as evidence for a version-selection rule.

The reader retains a `ModelHeaderPrefix` with `complete_payload: false`. Native
fields after the external-object list remain unread. The typed child definitions
and map are available for a caller that knows the remaining serialization sequence.
Unknown layouts, extended collection counts, bad schemas and invalid/missing
backreferences stop and roll back the entire requested prefix.

The model-header list also sharpens the separate configuration-bootstrap
hypothesis. For 310 matching resolved-feature streams with a supported first
common name header, its raw short tag equals `0x8000 | (external-list count + 4)`.
Two other checked headers have no resolved-feature stream. All 310 comparisons
agree, but the result does not identify a serialization call, an external slot's
meaning or its registration order. It is consistent with pre-registering referenced
objects before reading `Config-N`; it does not prove that mechanism. Production
code does not turn this count into missing map entries or resolve mate classes
from it. The configuration handoff still needs independent verification.

Probes, reports and extracted bytes remain ignored in
`local-assets/config-context-investigation/`. Use the empty-context diagnostic
probe to reproduce a supported model-header prefix:

```sh
cargo run --locked --example inspect_native_subtree -- model.SLDASM 'Contents/Config-0-ModelHeader' 0 two_strings model_header
```

Choose `model_header_word` for the leading-word layout. The external-object
profile remains explicit. This is a prefix probe, not a complete configuration
or mate decoder.

### Complete direct-array model-header layout

The checked direct-array prefix continues with one raw u32, another external-object
pointer, 42 scalar bytes, a null or empty `suObList`, three scalar words, an optional
80-byte block and three final words. The third scalar word before the optional
block is a checked zero/one presence value. The 80 bytes remain raw; neither
bounding-box semantics nor model-frame ownership is assigned. Nonempty trailing
lists and other presence values remain unsupported.

The following reference exposes a second saved external-object leading byte:
values 2 and 3 both precede word(0) and the existing checked payload fields. The
reader retains this byte rather than assigning it a SolidWorks API document type.
Accepting the observed byte 3 also extends model-header prefix coverage to 349
original public streams, with 922 explicit profile trials. Of these, 169 use direct
arrays and 180 use a leading word. The configuration-bootstrap correlation now
agrees in 347 matching resolved-feature streams; two headers lack that stream.
The correlation remains excluded from production map construction.

Python and Rust independently consume 166 original direct-array headers through
EOF with `TwoStrings` external-object framing. All class/object slots, pointer
occurrences, strings, scalar bytes and source spans agree. The optional block is
present in 163 streams and absent in three. These walks use original source bytes,
fresh contexts and pinned SHA-256/CRC checks, without relocation or map seeding.

`NativeSubtreeReader::read_model_header` supports only this established
`DirectArrays`/`TwoStrings` combination. It marks the root complete and permits
later same-profile root backreferences. Prefix-only roots remain incomplete.
`NativeModelHeaderFields` shares the prefix representation between the two APIs;
JSON prefix fields retain their existing flat shape. `NativeModelHeaderTail`
records the exact interval and remaining fields. Bytes after the supported object
are left for the caller, so the API can read a header followed by another known
object instead of treating EOF as an object-length field.

The leading-word layout still has unparsed remaining fields. No whole-file map,
current document/occurrence assignment, mate endpoint or appearance ownership is
inferred from a complete model-header object. Research and reports remain ignored
in `local-assets/model-header-tail-investigation/`.

```sh
cargo run --locked --example inspect_native_subtree -- model.SLDASM 'Contents/Config-0-ModelHeader' 0 two_strings model_header_full
```

### Complete leading-word model-header layouts

The leading-word headers share the established core tail, then add scalar
extensions of 32, 64 or 68 bytes. `read_model_header_with_tail` selects these
layouts explicitly. Full leading-word reads require the initial saved u32 to be
one; the prefix reader still retains other initial values without claiming a
complete object.

| Initial profile | External-object profile | Extension profile | Original streams |
| --- | --- | --- | ---: |
| `LeadingWord` | `ThreeStringsShortTail` | `Extension32` | 39 |
| `LeadingWord` | `ThreeStringsShortTail` | `Extension64` | 10 |
| `LeadingWord` | `ThreeStrings` | `Extension68` | 130 |

The extension is eight, sixteen or seventeen little-endian u32 words after the
common final words. The first word is one in the 32-byte layout, two in the
64-byte layout, and zero, one or two in the 68-byte layout. Later words are zero
or one. The decoder checks these observed values and retains all words without
assigning flag, count, version or owner meanings. Other values are unsupported.
In the corpus, these layouts occur with saved document versions 15000, 16000 and
17000 respectively; this observation is not used as a universal version gate.

An independent walker and Rust consume all 179 original streams through EOF with
matching class/object slots, pointer occurrences, fields and spans. No byte
relocation, map seeding or declaration scan supplies the walk. Together with the
166 direct-array headers, 345 local model-header objects are complete under the
selected profiles. Failed alternative profile trials remain failures and do not
become native class-map entries.

`NativeModelHeaderTail` retains the extension profile and raw words. Its span
includes the extension. Later complete-root backreferences must match both the
initial header profile and extension profile. Truncation, unknown leading words,
unsupported scalar values and profile mismatches roll back the read. The older
`read_model_header` API retains its direct-array/no-extension contract.

Four of the previously checked model-header prefixes still lack a complete
supported layout. The configuration archive handoff and whole-file topology or
appearance ownership remain unverified. Research scripts, original-stream checks
and reports remain ignored in `local-assets/leading-header-investigation/`.

```sh
cargo run --locked --example inspect_native_subtree -- model.SLDPRT 'Contents/Config-0-ModelHeader' 0 three_strings_short_tail model_header_full_word32
```

The 64-byte layout uses `model_header_full_word64` with `three_strings_short_tail`.
The 68-byte layout uses `model_header_full_word68` with `three_strings`.

### Remaining public header layouts

The 13 previously unsupported model-header streams reduce to checked saved-field
variants and two older direct-array layouts:

- Seven assembly streams, including six routed examples, use external-object
  prefix word 0x0100 instead of zero. The same CString/scalar layout continues
  through the complete header.
- Three Part headers use leading byte 5. The byte remains raw; no SolidWorks API
  document type is assigned.
- Two direct-array headers contain three saved CStrings, a 17-byte scalar block,
  the configuration CString and 12 trailing bytes. `ThreeStringsShortBlock`
  preserves that third CString as a string. Their header extension is seven u32
  words (28 bytes), starting with one, followed by checked zero/one values.
- One direct-array header uses `ThreeStringsShortTail` and an eight-word (32-byte)
  extension, rather than the initial leading-word header layout.

These counts describe the observed failure groups, not a general relationship
between these values and document kinds.
`raw_prefix_word` retains the exact u16 alongside `raw_header_byte`. Values outside
zero/0x0100 and 2/3/5 respectively remain unsupported. No virtual-component,
occurrence or ownership meaning is inferred.

Treating a third blank CString as four opaque scalar bytes could make different
field interpretations reach the same boundary. The implemented profiles instead
retain every checked CString and distinguish the 17- and 18-byte scalar blocks.
The complete-reader catalog now has six supported combinations. The new tuples
are `DirectArrays`/`ThreeStringsShortBlock`/`Extension28` and
`DirectArrays`/`ThreeStringsShortTail`/`Extension32`; existing tuples are unchanged.

Independent fresh-context walks and Rust agree through EOF on all 358 public
model-header streams: 176 direct-array and 182 leading-word headers. The public
CLI uniquely selects the same layout for every stream across 341 unique source
files. Comparisons cover all class/object slots, pointer occurrences, exact source
spans, strings, scalar bytes and extensions. There are no ambiguous results in
this corpus. Original SHA-256 identities and decoded CRCs match the pinned
inventory. No rewritten tags, map seed holes or declaration scans supply these
walks.

These results establish complete local model-header layouts for the sampled
streams. The configuration archive handoff, whole-file graph and topology or
appearance ownership remain separate open requirements. Probes and original-byte
reports remain ignored in `local-assets/remaining-header-investigation/`.

```sh
cargo run --locked --example inspect_native_subtree -- model.SLDPRT 'Contents/Config-0-ModelHeader' 0 three_strings_short_block model_header_full_direct28
```

Use `model_header_full_direct32` with `three_strings_short_tail` for the additional
direct-array 32-byte extension. `sw-dump model-headers` checks all six supported
combinations and retains unsupported or ambiguous results instead of choosing a
partial prefix.
