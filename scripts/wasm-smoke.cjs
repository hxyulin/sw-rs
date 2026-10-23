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
assert.equal(doc.document_type, 'assembly');
assert.equal(doc.configurations[0].name, '默认');
assert.throws(() => parse_json(new Uint8Array([1, 2, 3]), 'bad.SLDPRT'), /unrecognized/);
console.log('WASM Uint8Array → Rust parser → JSON passed; invalid input throws.');
