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

## Other native containers

Legacy OLE2 signatures and ZIP/OPC native containers return explicit unsupported
errors. `DocumentReader` can still produce an opaque inventory with the original
error. Do not pass these files directly to XML or attempt arbitrary zlib scans.

The [legacy ZLB research](legacy-zlb-rules.md) establishes a complete chunk wrapper
for already extracted OLE streams, including multiple members and an exact EOF
terminator. It does not implement CFB sector enumeration or legacy document layouts.

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
