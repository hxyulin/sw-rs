// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Run after wasm-bindgen --target nodejs --out-dir pkg/node <wasm>.
const assert = require('node:assert/strict');
const { deflateRawSync } = require('node:zlib');
const { parse_json } = require('../pkg/node/sw_rs.js');
function crc32(bytes) {
  let crc = 0xffffffff;
  for (const b of bytes) {
    crc ^= b;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
const header = Buffer.alloc(16);
header[7] = 3;
const name = Buffer.from('swXmlContents/COMPINSTANCETREE');
const data = Buffer.from('<swSolidWorks><swModelList><swModel swConfigurationId="0" swConfigurationName="默认"/></swModelList></swSolidWorks>');
const compressed = deflateRawSync(data);
const chunk = Buffer.alloc(26);
Buffer.from([0x14, 0, 6, 0, 8, 0]).copy(chunk);
[1, crc32(data), compressed.length, data.length, name.length].forEach((n, i) => chunk.writeUInt32LE(n, 6 + i * 4));
const encoded = Buffer.from(name.map(b => ((b >>> 3) | (b << 5)) & 255));
const file = Buffer.concat([header, chunk, encoded, compressed]);
const doc = JSON.parse(parse_json(new Uint8Array(file), 'synthetic.SLDASM'));
assert.equal(doc.doc_type, 'assembly');
assert.equal(doc.configurations[0].name, '默认');
assert.throws(() => parse_json(new Uint8Array([1, 2, 3]), 'bad.SLDPRT'), /unrecognized/);
console.log('WASM Uint8Array → Rust parser → JSON passed; invalid input throws.');

const { geometry_json } = require('../pkg/node/sw_rs.js');
function container(streamName, payload) {
  const n = Buffer.from(streamName);
  const packed = deflateRawSync(payload);
  const h = Buffer.alloc(26);
  Buffer.from([0x14, 0, 6, 0, 8, 0]).copy(h);
  [1, crc32(payload), packed.length, payload.length, n.length].forEach((v, i) => h.writeUInt32LE(v, 6 + i * 4));
  return Buffer.concat([header, h, Buffer.from(n.map(b => ((b >>> 3) | (b << 5)) & 255)), packed]);
}
function descriptor(size, kind, count, bytes) {
  const h = Buffer.alloc(16);
  [size, kind, 2, count].forEach((n, i) => h.writeUInt32LE(n, i * 4));
  return Buffer.concat([h, bytes]);
}
const positions = Buffer.alloc(36);
[0, 0, 0, 1, 0, 0, 0, 1, 0].forEach((n, i) => positions.writeFloatLE(n, i * 4));
const className = Buffer.from('uoTempFaceTessData_c');
const classHeader = Buffer.alloc(6);
Buffer.from([255, 255, 1, 0]).copy(classHeader);
classHeader.writeUInt16LE(className.length, 4);
const counts = Buffer.alloc(8);
counts.writeUInt32LE(1, 0); counts.writeUInt32LE(1, 4);
const payload = Buffer.concat([classHeader, className, counts,
  descriptor(4, 8, 1, Buffer.from([3, 0, 0, 0])),
  descriptor(12, 100, 3, positions), descriptor(12, 100, 3, Buffer.alloc(36)),
  descriptor(4, 8, 4, Buffer.alloc(16)), descriptor(4, 8, 1, Buffer.from([4, 0, 0, 0])),
  descriptor(1, 8, 4, Buffer.alloc(4))]);
const geometry = JSON.parse(geometry_json(container('Contents/DisplayLists', payload), 'synthetic.SLDPRT'));
assert.equal(geometry.schema, 'sw-rs-geometry-v1');
assert.deepEqual(geometry.geometry.model.tessellations[0].vertices, [[0, 0, 0], [1000, 0, 0], [0, 1000, 0]]);
assert.deepEqual(geometry.geometry.model.tessellations[0].triangles, [[0, 1, 2]]);
assert.throws(() => geometry_json(file, 'assembly.SLDASM'), /Parts only/);
console.log('WASM Part geometry, units, winding, source hashing and errors passed.');
const { geometry_references_json, references_json } = require('../pkg/node/sw_rs.js');
function framed(value) {
  const raw = Buffer.from(value, 'utf16le');
  assert.ok(raw.length / 2 < 254);
  return Buffer.concat([Buffer.from([255, 254, 255, raw.length / 2]), raw]);
}
const descriptorGroup = 'moSimpleSurfIdRep_c,7,2, \nmoSimpleSurfIdRep_c,8,3, \n';
const cacheGraph = JSON.parse(geometry_references_json(container('Contents/DisplayLists', Buffer.concat([payload, framed(descriptorGroup)])), 'grouped.SLDPRT'));
const saved = cacheGraph.nodes.find(node => node.record.kind === 'cached_surface').record.surface_references.references;
assert.deepEqual(saved.map(ref => ref.feature_source_id), [7, 8]);
assert.equal(saved[1].source.offset, payload.length);
assert.ok(saved[1].text_span.offset > 0);
assert.equal(cacheGraph.topology_ownership_decoded, false);
const mateClass = Buffer.from('moMateCoincident');
const mateClassHeader = Buffer.from([255, 255, 1, 0, mateClass.length, 0]);
const mate = Buffer.concat([Buffer.from([193, 0, 0, 0, 1, 0]), mateClassHeader, mateClass,
  Buffer.from([12, 128]), framed('Edited name'), Buffer.from([0, 1, 0, 0, 0, 0, 0, 64, 27, 0, 0, 0, 0, 0, 0, 0]),
  framed(''), framed(descriptorGroup), Buffer.from([1, 0, 0, 0]), framed('Part-1@Root')]);
const mateGraph = JSON.parse(references_json(container('Contents/Config-7-MatesList', mate), 'grouped.SLDASM'));
const selection = mateGraph.links.find(link => link.relation === 'mate_selection');
assert.deepEqual(selection.selection_surface_references.map(ref => ref.feature_source_id), [7, 8]);
assert.equal(selection.selection_surface_reference, undefined);
assert.equal(selection.selection_descriptor.value, descriptorGroup);
assert.equal(selection.resolution, 'unresolved');
console.log('WASM grouped mate/cache references preserve all rows without resolving topology.');
function nativeClass(name) {
  const raw = Buffer.from(name);
  const h = Buffer.from([255, 255, 1, 0, 0, 0]);
  h.writeUInt16LE(raw.length, 4);
  return Buffer.concat([h, raw]);
}
const nativeHeader = Buffer.alloc(20);
[1, 0, 490, 768, 42].forEach((word, index) => nativeHeader.writeUInt32LE(word, index * 4));
const nativePrefix = Buffer.concat([nativeClass('moFaceRef_c'), nativeHeader,
  Buffer.from('7dc39425ad49b2547dc39425ad49b254', 'hex'), Buffer.alloc(2),
  nativeClass('moSimpleSurfIdRep_c'), Buffer.alloc(2), nativeClass('moFR_c'),
  nativeClass('moExtObject_c'), nativeClass('moCStringHandle_c'), framed('../source/Part.SLDPRT')]);
const nativeGraph = JSON.parse(references_json(container('ThirdPtyStore/VisualStates', nativePrefix), 'reference.SLDPRT'));
const handle = nativeGraph.links.find(link => link.relation === 'native_document_handle');
assert.equal(handle.reference, '../source/Part.SLDPRT');
assert.equal(handle.resolution, 'unresolved');
assert.deepEqual(handle.candidates, []);
const prefixRecord = nativeGraph.nodes[handle.from].record.reference;
assert.deepEqual(prefixRecord.raw_header, [1, 0, 490, 768, 42]);
assert.equal(prefixRecord.prefix_span.length, nativePrefix.length);
assert.equal(nativeGraph.native_object_graph_decoded, false);
assert.equal(nativeGraph.topology_ownership_decoded, false);
console.log('WASM native document handles retain checked source prefixes without assigning ownership.');
const handleClassAt = nativePrefix.indexOf(nativeClass('moCStringHandle_c'));
const reusedPrefix = Buffer.concat([nativePrefix.subarray(0, handleClassAt),
  Buffer.from([16, 128]), framed('../source/Part.SLDPRT')]);
const contextFrame = Buffer.concat([nativeClass('moExtFeature_c'),
  nativeClass('moCStringHandle_c'), framed('different/stale/assembly.SLDASM'),
  Buffer.from([16, 128]), framed('saved configuration text')]);
const context = Buffer.concat([nativeClass('moAssembly_c'), contextFrame]);
function candidateFile(contextName, contextPayload) {
  return Buffer.concat([container('Contents/Config-7-MatesList', reusedPrefix),
    container(contextName, contextPayload).subarray(header.length)]);
}
const candidateGraph = JSON.parse(references_json(candidateFile('Contents/Config-7', context), 'candidate.SLDASM'));
const candidateLink = candidateGraph.links.find(link => link.relation === 'native_document_handle_candidate');
assert.equal(candidateLink.reference, '../source/Part.SLDPRT');
assert.equal(candidateLink.resolution, 'unresolved');
assert.deepEqual(candidateLink.candidates, []);
const candidate = candidateGraph.nodes[candidateLink.from].record;
assert.equal(candidate.kind, 'native_document_reference_candidate');
assert.equal(candidate.reference.class_evidence.source_stream, 'Contents/Config-7');
assert.equal(candidate.reference.class_evidence.raw_tag, 0x8010);
assert.equal(candidate.reference.prefix_span.length, reusedPrefix.length);
assert.equal(candidateGraph.native_reference_stops[0].stage, 'moCStringHandle_c');
assert.equal(candidateGraph.native_reference_stops[0].offset, handleClassAt);
assert.equal(candidateGraph.native_object_graph_decoded, false);
assert.equal(candidateGraph.topology_ownership_decoded, false);
for (const [contextName, bytes] of [['Contents/Config-8', context],
  ['Contents/Config-7', Buffer.concat([context, contextFrame])]]) {
  const rejected = JSON.parse(references_json(candidateFile(contextName, bytes), 'candidate.SLDASM'));
  assert.equal(rejected.nodes.some(node => node.record.kind === 'native_document_reference_candidate'), false);
  assert.equal(rejected.native_reference_stops.length, 1);
}
console.log('WASM handle candidates preserve class-map blockers and reject ambiguous or cross-config context.');
const edgePrefix = Buffer.from(reusedPrefix);
edgePrefix.write('moEdgeRef_c', 6, 'ascii');
const edgeFile = Buffer.concat([candidateFile('Contents/Config-7', context),
  container('Contents/Config-7-ResolvedFeatures', edgePrefix).subarray(header.length)]);
const edgeGraph = JSON.parse(references_json(edgeFile, 'edge.SLDASM'));
const edgeNode = edgeGraph.nodes.find(node => node.record.reference?.reference_class === 'moEdgeRef_c');
assert.equal(edgeNode.record.kind, 'native_document_reference_candidate');
assert.equal(edgeNode.record.reference.source_stream, 'Contents/Config-7-ResolvedFeatures');
assert.equal(edgeNode.record.reference.configuration_index, 7);
assert.equal(edgeNode.record.reference.document.value, '../source/Part.SLDPRT');
assert.equal(edgeGraph.links.find(link => link.from === edgeNode.index).resolution, 'unresolved');
assert.equal(edgeGraph.native_reference_stops.length, 2);
const unrelatedEdge = Buffer.concat([container('Contents/Config-7-ResolvedFeatures', edgePrefix),
  container('Contents/Config-7', context).subarray(header.length)]);
const unrelatedGraph = JSON.parse(references_json(unrelatedEdge, 'edge.SLDASM'));
assert.equal(unrelatedGraph.nodes.some(node => node.record.reference?.reference_class === 'moEdgeRef_c'), false);
assert.equal(edgeGraph.native_object_graph_decoded, false);
assert.equal(edgeGraph.topology_ownership_decoded, false);
console.log('WASM edge-prefix evidence retains configuration scope without assigning face owners.');

if (process.argv[2]) {
  const path = process.argv[2];
  const source = require('node:fs').readFileSync(path);
  const wasm = JSON.parse(geometry_json(source, path));
  const native = JSON.parse(require('node:child_process').execFileSync(process.argv[3], ['geometry', path], {maxBuffer: 128 * 1024 * 1024}));
  assert.deepEqual(wasm.geometry, native.geometry);
  const wasmMetadata = JSON.parse(parse_json(source, path));
  const nativeMetadata = JSON.parse(require('node:child_process').execFileSync(process.argv[3], ['metadata', path]));
  assert.deepEqual(wasmMetadata, nativeMetadata);
  const wasmInventory = JSON.parse(require('../pkg/node/sw_rs.js').inventory_json(source, path));
  const nativeInventory = JSON.parse(require('node:child_process').execFileSync(process.argv[3], ['inventory', path], {maxBuffer: 128 * 1024 * 1024})).inventory;
  assert.deepEqual(wasmInventory, nativeInventory);
  console.log('Real native Part: WASM/native geometry, metadata and source inventory matched exactly.');
}
