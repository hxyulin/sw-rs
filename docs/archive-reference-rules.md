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
whole-document reference graph is claimed. All 93 legacy OLE documents remain
outside the runtime container profile. Native versions observed are 12000,
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

## Topology identity is another decoding layer

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
