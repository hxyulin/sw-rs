<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Source inventory and the shared document reader

The inventory locates saved data and records what is known about its framing.
It does not claim every native object, feature or dependency is understood.
A file can have verified container bytes and remain largely opaque semantically.

## Reader interface

Existing `parse`, `geometry`, their limit-aware variants, and
`metadata_from_container` remain available. For multiple views of one source:

```rust
fn inspect(bytes: &[u8]) -> Result<(), sw_rs::ParseError> {
let reader = sw_rs::DocumentReader::open(bytes, "part.SLDPRT", sw_rs::Limits::default())?;
let metadata = reader.metadata()?;
let geometry = reader.geometry()?;
let inventory = reader.inventory();
let original_feature_bytes = reader.stream("Contents/Config-0-ResolvedFeatures")?;
let structural_view = reader.inspect_stream("swXmlContents/Features")?;
let _ = (metadata, geometry, inventory, original_feature_bytes, structural_view);
Ok(())
}
```

`open` borrows original file bytes and eagerly validates/decompresses the
container once. It retains trace records and computes original-file/verified
stream SHA-256 identities. It does not perform lazy decompression or run all
semantic decoders.

- `source_bytes()` and `stream(name)` return borrowed byte slices.
- `metadata()` and `geometry()` initialize cached results on demand. They share
  the same decoded streams. Geometry errors are retained too.
- `inventory()` caches an owned source summary without invoking metadata,
  geometry, or detailed structural inspection.
- `inspect_stream(name)` explicitly builds an owned structural report. The
  library does not cache these potentially large reports automatically.
- `finish()` transfers retained buffers and requested projections into
  `ReadOutput`, without copying mesh buffers. Unrequested projections remain None.
- `Container::inspect_stream(name)` also works on an already owned container.

`DocumentReader` owns decoded buffers and immutable cached projections. It uses
single-owner `OnceCell` caches; use a reader per worker rather than sharing one
across threads. `finish` ends the original-byte borrow. Callers can then retain
only the output they need. Configurations are cloned into the existing owned
geometry DTO; raw carriers and mesh buffers are not cloned by this interface.

Unsupported/corrupt container errors are retained so an inventory can still be
produced. Semantic/stream access and `finish` return the original error. The
source file-size cap fails `open` before decoding/hashing; other hard limits can
stop the scan and leave a partial source inventory. Limits and validation rules
are unchanged. `Limits::unlimited()` remains an explicit caller choice.

The viewer uses this interface to request Part metadata and geometry from one
container pass, then drops unneeded decoded streams. The consuming `geometry`
convenience API can still release unrelated streams before mesh allocation.
Retaining a shared reader instead trades decoded-buffer memory for avoiding
repeated decompression; it is not a peak-memory reduction guarantee.

## Checked binary cursor

`BinaryReader` borrows a decoded stream and provides checked little-endian
primitive reads, finite float-array helpers, and UTF-16 CString framing with
short/u16/u32 counts. Failed compound reads roll back, making alternative record
profiles easier to probe. `take` borrows bytes; string decoding allocates only
the resulting String, without an intermediate UTF-16 Vec.

The existing `NativeString` API is retained as an alias of `FramedString`.
Appearance/mate framing now shares this cursor. Framed offsets refer to the slice
provided to the cursor; use `BinaryReader::at(full_stream, offset)` to retain
whole-stream coordinates when starting inside a record.

## Source inventory contract

`DocumentInventory` (`sw-rs-inventory-v1`) records:

- Source SHA-256, byte length and outer-signature classification.
- Inline marker candidates with original-file offsets, bounded name/payload spans,
  declared sizes/CRC, validation outcomes, hashes and diagnostics.
- Selected, validated duplicate, rejected, directory-index and cache-index records.
- Selected stream descriptors with decoded byte counts/hashes, source-record
  indices, path-based family hints and available partial decoder names.
- Nonoverlapping verified inline-record ranges and their unclassified complement.

Verified ranges include the encoded inline header, name and compressed payload.
Every original file byte belongs to either a verified range or an unclassified
range. Unclassified bytes can contain prologues, indices, rejected candidates,
unsupported structures or other content; they are never assumed to be padding.
Index framing is recorded, but index payload semantics are not decoded.

A raw header field does not become an interpreted size/type for an unsupported
record family. Duplicate payloads are validated and hashed, but the selected raw
stream API retains the existing first-valid-name behavior. Duplicate encoded
payloads remain accessible through source spans. If a hard error occurs after
some successful records, those descriptors can remain in the inventory while
`decoded_streams_available` is false.

`family_hint` and `available_decoders` describe path-based routing possibilities,
not decoder execution or success. `semantic_status` is deliberately
`not_evaluated`, regardless of which other reader projections were requested.
Inventory byte accounting must never be presented as a semantic coverage percentage.

## Structural stream inspection

`StreamInspection` distinguishes `xml`, `invalid_xml`, `empty`, and
`binary_candidates`. It does not establish a native object graph.

XML inspection retains preorder element/parent indices, names/namespaces,
attributes, text nodes and byte spans. UTF-8 and UTF-16 LE/BE are supported;
sparse source checkpoints map UTF-16 spans back to the original decoded-stream
bytes, including non-BMP characters. The observed one-byte `KeyWords` wrapper is
retained in `opaque_prefix`, and `xml_payload_span` locates its XML. Its flag
semantics remain unverified. Comments/declarations and other original syntax
remain available in the source buffer rather than separate typed records.

Only explicit `swFileRef`/`swModelRef` attributes in the SolidWorks namespace
produce syntactic XML-reference candidates. All matching unqualified `id` values
in that namespace are retained, so duplicate/missing targets stay ambiguous.
These links never establish persistent B-Rep ownership or cross-stream identities.
Other attributes, namespaces and elements remain in the structural inventory.

For binary data, supported schema-1 ASCII class declarations and framed UTF-16
strings are **candidates**. Such framing can occur inside opaque bytes. No complete
object interval, reused-class dictionary, entity identity or link is inferred.
Incidental `<` bytes in binary streams do not make them malformed XML documents.

## CLI, WASM and browser inspection

```sh
mbx run --locked --bin sw-dump -- inventory part.SLDPRT
mbx run --locked --bin sw-dump -- inventory part.SLDPRT swXmlContents/MATERIALTREE
mbx run --release --locked --features archive --example audit_inventory -- ARCHIVE.zip --structure
```

The CLI summary succeeds for recognized unsupported containers, with an explicit
`container_error` and opaque ranges. Selecting a stream requires an available
decoded container. `inventory_json(bytes, filename)` exposes the same summary
through the optional core WASM feature.

In the viewer, **Inspect source…** opens a native dialog. Filter streams, inspect
one stream and browse record pages. XML values and binary candidates render as
plain text. Opening/closing/inspecting does not request a scene or change geometry.
Unsupported native containers still receive opaque source inventories.

The viewer retains one inspected source and one detailed stream until close or
source replacement. It scans a selected stream once and pages the JSON report
in windows of 100 records per collection; original element indices and byte
offsets remain unchanged. All records remain accessible, and the core API is
unpaginated. This bounds record counts per UI message, not individual value sizes.
The viewer summary schema is `sw-rs-inventory-summary-v1`; its paged report is a
projection of the full library inventory, not the full inventory DTO.

## Validation and measured scope

Private EGA and design-library audits include 580 native archive entries:
340 + 147 modern supported files and 93 unsupported legacy OLE files. There are
27,955 selected modern streams. Every file's verified-plus-unclassified byte
accounting balances, and source hashes match prior audits. Original CAD and
per-file reports remain ignored under `local-assets/inventory-investigation/`.

The main EGA assembly retains 3,380,059 triangles, 615 rendered occurrences,
1 missing reference and 1,169 visited nodes. Native tests check CRC rejection,
duplicates, indices, truncation, limits, source accounting, transactional reads,
UTF-16 source mapping and output compatibility. Browser tests cover source
filtering, opaque files, pagination, stale/import reset, inert saved text and
unchanged scene state.

A reproducible two-Part benchmark compares separate metadata/geometry wrappers
with the shared reader, using seven alternating in-memory rounds and verifying
byte-identical serialized outputs before timing:

```sh
mbx run --release --locked --example benchmark_reader -- PART.SLDPRT 7
```

| Source bytes | Retained decoded bytes | Separate median | Shared median |
| ---: | ---: | ---: | ---: |
| 783317 | 1593253 | 8.26 ms | 5.41 ms |
| 1494112 | 5412962 | 15.49 ms | 10.64 ms |

These are local release measurements for two samples, including allocations and
drops. They exclude file/archive IO, detailed inventory inspection, browser mesh
upload and rendering. They do not establish whole-project speed or peak RSS.

## Next decoder work

The inventory makes the next unknowns visible; it does not eliminate them.
Prioritize shared MFC dictionaries and native object references, feature/history
relationships, body/configuration ownership, units and saved views. Register
new partial decoders only with source-backed routing and explicit coverage.
Native mate solving, feature regeneration and Parasolid interpretation remain
separate capabilities.
