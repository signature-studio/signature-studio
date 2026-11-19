// R4-4l: compares the function bodies of two wasm files at the same function index (read only). For each index: the body offset, size and the SHA-256 of the body in file A, the name of that index in file B (its name section, else its export), whether file B has a body with the same SHA-256 at the same index, and every index of file B whose body has the same SHA-256 (uniqueness). When the sizes are equal, the number of differing bytes between the two bodies and its share.
// Usage: node tools/verify-kit/tools/r44l-cmp.mjs <index A.json> <index B.json> <wasm A> <wasm B> <index list, comma separated>
// The index JSON files are written by tools/verify-kit/tools/r44l-wasmidx.mjs.
import fs from 'node:fs';

const [idxA, idxB, wasmA, wasmB, list] = process.argv.slice(2);
if (!list) {
  process.stderr.write('usage: node tools/verify-kit/tools/r44l-cmp.mjs <index A.json> <index B.json> <wasm A> <wasm B> <index list>\n');
  process.exit(2);
}
const A = JSON.parse(fs.readFileSync(idxA, 'utf8'));
const B = JSON.parse(fs.readFileSync(idxB, 'utf8'));
const bufA = fs.readFileSync(wasmA);
const bufB = fs.readFileSync(wasmB);
const bodyA = new Map(A.bodies.map((b) => [b[0], b]));
const bodyB = new Map(B.bodies.map((b) => [b[0], b]));
const byHash = new Map();
for (const [i, , , h] of B.bodies) { if (!byHash.has(h)) byHash.set(h, []); byHash.get(h).push(i); }
const expA = new Map(A.exports);
const expB = new Map(B.exports);
const nameB = new Map(B.names);

const rows = [];
for (const idx of list.split(',').map(Number)) {
  const a = bodyA.get(idx);
  const b = bodyB.get(idx);
  if (!a) { rows.push({ idx, error: 'not a defined function in A' }); continue; }
  const sameAt = b && b[3] === a[3];
  const row = {
    idx,
    exportA: expA.get(idx) ?? '-',
    offA: a[1],
    sizeA: a[2],
    shaA: a[3].slice(0, 16),
    nameB: nameB.get(idx) ?? expB.get(idx) ?? '-',
    sizeB: b ? b[2] : null,
    shaB: b ? b[3].slice(0, 16) : null,
    sameShaAtSameIndex: sameAt ? 'yes' : 'no',
    indicesInBWithSameSha: (byHash.get(a[3]) ?? []).join(',') || '-',
  };
  if (b && b[2] === a[2] && !sameAt) {
    let diff = 0;
    for (let k = 0; k < a[2]; k++) if (bufA[a[1] + k] !== bufB[b[1] + k]) diff++;
    row.diffBytes = diff;
    row.diffPct = (100 * diff / a[2]).toFixed(2);
  } else if (b && b[2] !== a[2]) {
    row.diffBytes = 'size differs';
    row.diffPct = '-';
  } else {
    row.diffBytes = sameAt ? 0 : '-';
    row.diffPct = sameAt ? '0.00' : '-';
  }
  rows.push(row);
}
const cols = ['idx', 'exportA', 'offA', 'sizeA', 'shaA', 'nameB', 'sizeB', 'shaB', 'sameShaAtSameIndex', 'indicesInBWithSameSha', 'diffBytes', 'diffPct'];
console.log(`| ${cols.join(' | ')} |`);
console.log(`|${cols.map(() => '---').join('|')}|`);
for (const r of rows) {
  if (r.error) { console.log(`| ${r.idx} | ${r.error} |`); continue; }
  console.log(`| ${cols.map((c) => r[c] ?? '-').join(' | ')} |`);
}
