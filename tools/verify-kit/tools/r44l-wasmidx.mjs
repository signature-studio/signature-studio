// R4-4l: reads a wasm binary (no compilation, no change) and writes a JSON index: the function imports count, the defined function count, the export names of functions, the names of the name section (when present), and for every defined function its body offset (the byte offset of the body after its size field, the same offset Chrome puts in a CPU profile's columnNumber), its size and the SHA-256 of its bytes.
// Usage: node tools/verify-kit/tools/r44l-wasmidx.mjs <wasm file (absolute)> <out.json (absolute)>
import fs from 'node:fs';
import crypto from 'node:crypto';

const [wasmPath, outPath] = process.argv.slice(2);
if (!wasmPath || !outPath) {
  process.stderr.write('usage: node tools/verify-kit/tools/r44l-wasmidx.mjs <wasm file> <out.json>\n');
  process.exit(2);
}
const buf = fs.readFileSync(wasmPath);
let pos = 0;
const u8 = () => buf[pos++];
const u32 = () => {
  let result = 0; let shift = 0; let b;
  do { b = buf[pos++]; result += (b & 0x7f) * 2 ** shift; shift += 7; } while (b & 0x80);
  return result;
};
const bytes = (n) => { const s = buf.subarray(pos, pos + n); pos += n; return s; };
const name = () => bytes(u32()).toString('utf8');

if (buf.readUInt32LE(0) !== 0x6d736100) throw new Error('not a wasm file');
pos = 8;
let funcImports = 0;
let defs = 0;
const exports = [];
const names = new Map();
let codeStart = null; let codeCount = 0; let codeFrom = 0;
while (pos < buf.length) {
  const id = u8();
  const size = u32();
  const end = pos + size;
  if (id === 0) {
    const nm = name();
    if (nm === 'name') {
      while (pos < end) {
        const sub = u8(); const subSize = u32(); const subEnd = pos + subSize;
        if (sub === 1) {
          const n = u32();
          for (let i = 0; i < n; i++) { const idx = u32(); names.set(idx, name()); }
        }
        pos = subEnd;
      }
    }
  } else if (id === 2) {
    const n = u32();
    for (let i = 0; i < n; i++) {
      name(); name();
      const kind = u8();
      if (kind === 0) { u32(); funcImports++; }
      else if (kind === 1) { u8(); const flags = u8(); u32(); if (flags & 1) u32(); }
      else if (kind === 2) { const flags = u8(); u32(); if (flags & 1) u32(); }
      else if (kind === 3) { u8(); u8(); }
      else if (kind === 4) { u8(); u32(); }
      else throw new Error(`import kind ${kind}`);
    }
  } else if (id === 3) {
    defs = u32();
  } else if (id === 7) {
    const n = u32();
    for (let i = 0; i < n; i++) {
      const nm = name(); const kind = u8(); const idx = u32();
      if (kind === 0) exports.push([idx, nm]);
    }
  } else if (id === 10) {
    codeCount = u32();
    codeStart = pos;
    codeFrom = funcImports;
  }
  pos = end;
}
if (codeStart === null) throw new Error('no code section');
if (codeCount !== defs) throw new Error(`code count ${codeCount} != function count ${defs}`);

const bodies = [];
pos = codeStart;
for (let i = 0; i < codeCount; i++) {
  const size = u32();
  const off = pos;
  const body = buf.subarray(pos, pos + size);
  const sha = crypto.createHash('sha256').update(body).digest('hex');
  bodies.push([codeFrom + i, off, size, sha]);
  pos += size;
}

const out = {
  wasm: wasmPath,
  bytes: buf.length,
  funcImports,
  defs,
  hasNameSection: names.size > 0,
  nameCount: names.size,
  exports: exports.sort((a, b) => a[0] - b[0]),
  names: [...names.entries()].sort((a, b) => a[0] - b[0]),
  bodies,
};
fs.writeFileSync(outPath, JSON.stringify(out));
process.stdout.write(`funcImports=${funcImports} defs=${defs} exports(func)=${exports.length} names=${names.size} bodies=${bodies.length} -> ${outPath}\n`);
