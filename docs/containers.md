<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Containers and compression

## Modern inline-stream profile

The current reader recognizes the six-byte marker `14 00 06 00 08 00` near the
start of a modern native file, then scans candidate records. Numeric fields are
little-endian. The supported inline record header relative to its marker is:

| Offset | Bytes | Interpretation |
| --- | ---: | --- |
| `+0` | 6 | Marker |
| `+6` | 4 | Family word retained by inventory; not a universal payload schema |
| `+10` | 4 | Expected decoded CRC32 |
| `+14` | 4 | Compressed payload length |
| `+18` | 4 | Expected decoded length |
| `+22` | 4 | Encoded stream-name length |
| `+26` | variable | Encoded stream name, then compressed payload |

Each name byte is rotated left by `file[7] & 7`; the result must be valid UTF-8
without control characters. Payloads use raw DEFLATE, not zlib headers. The
reader requires a completed stream, exact compressed/decoded consumption and
matching CRC32. Output grows from actual decompression, under the caller's
budget; declared lengths do not authorize unchecked allocation.

The scan is a recovery profile, not a complete native directory parser. A marker
candidate can be rejected. The first valid record for each decoded name is
selected; later duplicates are recorded and count toward decoded-byte budgets.
Malformed candidates retain diagnostics. Inventory preserves original source
hashes/spans and decoded hashes rather than pretending all source bytes have a
semantic interpretation.

## Index records and unknown ranges

Supported directory-index candidates have a different descriptor/name position
(`+40`), zero CRC/decoded-size words and no inline payload. Cache-index candidates
use a recognized size/CRC relationship (`crc == decoded_size * 2` with wrapping
u32 arithmetic, `compressed_size == decoded_size / 2`); they are reported rather
than passed to DEFLATE. These are implemented recognition rules, not complete
index semantics. See `src/container.rs` and the [inventory contract](reader-inventory.md).

Unknown record families and bytes outside validated spans remain diagnostic or
opaque. Successfully recovering a document's named streams is not equivalent
to interpreting every cache, binary object or historical revision.

## Legacy OLE structured storage

The reader loads OLE/CFB v3 and v4 from borrowed bytes using `cfb` 0.15 with
strict validation. It enumerates nested storages, extracts ordinary and mini
streams, and preserves Unicode paths, including standard property-set control
characters. No host files, clock or native CAD library are required.

A stream whose leaf ends in `__ZLB` must pass the complete
[ZLB wrapper decoder](legacy-zlb-rules.md). Its selected name removes that suffix;
for example, `Contents/DisplayLists__ZLB` becomes `Contents/DisplayLists`.
Other streams remain exact bytes, even when their contents resemble ZLB.
Raw/compressed names that normalize to the same path fail the container rather
than choosing a value. A malformed wrapper also fails the container; the source
inventory retains earlier validated streams and the rejected wrapper's provenance.

Input, per-stream and cumulative decoded-byte limits apply. `max_compressed_bytes`
also bounds the entire extracted ZLB wrapper before allocation. `max_records`
counts non-root storages, streams and ZLB members across the document. Physical
reads are bounded by twice the input length: one allocation-metadata pass and
one nonoverlapping stream-data pass. This prevents repeated FAT references from
amplifying allocations. Stream bytes cannot overlap each other or read allocation
metadata as a native payload.

`ContainerRecord::ole_source` preserves original paths, encoded lengths/hashes,
physical source ranges in logical order and validated ZLB member/terminator
spans. Concatenating those physical ranges reconstructs the encoded stream.
ZLB spans instead refer to that reconstructed stream. Inline header fields remain
absent. Verified byte accounting includes only validated encoded stream bytes;
allocation metadata and unused sector padding remain unclassified. Raw OLE
streams have no invented CRC: their SHA-256 records identity, not an on-disk checksum.

Loading streams does not establish support for every legacy native layout.
Existing metadata and cached-mesh profiles run on compatible streams. Binary OLE
property sets, native topology and feature evaluation remain opaque. Strict CFB
validation can reject malformed files that another application repairs.

## Other native containers

ZIP/OPC native containers remain unsupported. `DocumentReader` retains an opaque
inventory and the original error. Outer project ZIP archives are a separate layer.

## Outer project archives

With feature `archive`, `FileSet` accepts ZIP (stored/deflated), raw TAR and
TAR.GZ without extracting to disk. XZ/Zstandard TAR, encrypted ZIP, symlinks and
special members are unsupported. Member paths reject absolute paths, traversal,
Windows drive/alternate-stream syntax and case-insensitive duplicates. Ignored
non-CAD members still undergo integrity checks.

`FileSet::resolve(owner, reference)` searches inside uploaded documents, using
owner-relative paths then a unique longest Windows path suffix. Ambiguous
basenames remain ambiguous. Viewer project composition additionally follows
saved `ReplaceRef` chains and assembly-owned embedded documents; those are not
host filesystem searches. See [viewer](viewer.md) and the
[archive Rust API](https://hxyulin.github.io/sw-rs/api/sw_rs/archive/index.html).
