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
