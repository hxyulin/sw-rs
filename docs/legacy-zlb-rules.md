<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Legacy compressed-stream framing

This research identifies the `__ZLB` wrapper used by legacy OLE documents in the
design library. The new pure Rust decoder handles extracted compressed streams;
`Container` still rejects OLE documents. No legacy document/geometry coverage
increase is claimed.

## Observed framing

All 490 inspected `__ZLB` streams from 93 actual native documents share this
16-byte prefix (its GUID-like semantics are not inferred):

```text
23 1D D5 71 DA 81 48 A2 A8 58 98 B2 1B 89 EF 99
```

Following it, the observed grammar is:

```text
repeat:
    decoded_size:u32 LE
    compressed_size:u32 LE
    if both sizes are zero: end (must be EOF)
    otherwise: compressed_size bytes containing exactly one zlib member
```

Both nonterminal sizes must be positive. The zlib member includes its header and
Adler checksum, unlike the raw DEFLATE payload used by the modern inline
container. Members concatenate to form one decoded logical stream. Source
offsets here refer to extracted OLE stream bytes, not physical file offsets.

The corpus contains 107 empty wrappers, 379 single-member wrappers, and four
multi-member wrappers (7, 9, 3 and 3 members): 401 members total. Every member's
declared sizes match actual compression boundaries and output length, and every
wrapper ends with exactly eight zero bytes. The largest observed decoded member
is 1,048,576 bytes. That is an observation, not a parser-imposed chunk cap.

Total decoded data is 45,323,897 bytes. An independent Python zlib/OLE probe and
Rust agree on all 490 encoded/decoded hashes, member spans and terminator spans.
A probe that decodes only the first zlib member would truncate the four
multi-member streams; searching for a zlib header is insufficient validation.

## Implemented boundary

`decode_zlb(bytes, limits)` returns concatenated bytes plus per-member source and
decoded spans. It validates the signature, all size pairs, zlib checksums, exact
input/output consumption and terminator. Malformed later members fail the entire
operation. It grows output from actual decompression rather than allocating the
declared length up front. Existing caller-supplied byte/count limits apply across
the wrapper; callers processing multiple wrappers own the combined budget.

The implementation uses the existing Rust flate2 backend, with no new dependency,
filesystem calls, native runtime or proprietary assets. Synthetic tests cover
empty/multiple members, members larger than the observed one-MiB size, truncation,
checksums, declared-length mismatches, member padding and cumulative limits.

```sh
mbx run --release --locked --example audit_zlb -- EXTRACTED_STREAM.zlb
mbx run --release --locked --example audit_zlb -- EXTRACTED_ZLB_DIRECTORY
```

The audit emits hashes, member counts and spans, without serializing raw payloads.
Private source probes and extracted streams remain ignored under
`local-assets/surface-investigation/`.

## Next layer

OLE is a separate structured-storage format with a published
[Microsoft CFB specification](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/53989ce4-7b05-4f8d-829b-d08d6148375b).
The [MIT-licensed rust-cfb library](https://github.com/mdsteele/rust-cfb) is a
candidate for an OLE-container experiment; it has not been added or validated
for this project's WASM build. Container integration needs stream provenance,
sector/range accounting, cumulative budgets and explicit duplicate-name behavior.
Decoded stream names/legacy native layouts then need compatibility checks.

This wrapper rule is a prerequisite for complete legacy stream inventory. It does
not decode Parasolid bodies, recover native topology, or resolve the modern EGA
mirror/embedded Parts whose supported face caches are absent.
