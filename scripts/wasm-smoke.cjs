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
