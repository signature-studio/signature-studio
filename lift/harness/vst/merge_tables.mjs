#!/usr/bin/env node
// The dispatch table of one lifted module (the node replacement of the merge of gen_all.rs, which is the merge_tables_n.py of the older builds).
// usage: node merge_tables.mjs TABLES.txt ROWS.txt OUT.c
//   TABLES.txt  PREFIX-tables.txt of `fe-lift lift FILE PREFIX --split-bytes N` (the dispatch table and the response table)
//   ROWS.txt    extra rows "<0x pseudo-VA> <function>" (# starts a comment); the VA must be >= 0x7FF9_0000_0000 and unused
// OUT.c gets fle_dispatch_table (VA -> function, sorted by VA), fle_dispatch_count and the response table fle_resp_table of TABLES.txt.
// R, FL and X are not defined here: the runtime (fle_runtime.h with FLE_LOWREGS) defines them.
import fs from 'node:fs';

const [tablesPath, rowsPath, outPath] = process.argv.slice(2);
if (!outPath) {
  console.error('usage: node merge_tables.mjs TABLES.txt ROWS.txt OUT.c');
  process.exit(64);
}
const RESP_BASE = 0x7ff90000_0000n;

function parseRow(line) {
  const m = /^ {4}\{ UINT64_C\(0x([0-9a-f]+)\), ([A-Za-z0-9_]+) \},$/.exec(line);
  return m ? { va: BigInt('0x' + m[1]), name: m[2] } : null;
}

const rows = new Map();
const count = new Map();
let resp = null;
const lines = fs.readFileSync(tablesPath, 'utf8').split('\n');
let intab = false;
for (const l of lines) {
  if (l.startsWith('const fle_fn_entry fle_dispatch_table[]')) {
    intab = true;
    continue;
  }
  if (!intab) continue;
  const r = parseRow(l);
  if (r) {
    count.set(r.va, (count.get(r.va) ?? 0) + 1);
    const old = rows.get(r.va);
    if (old !== undefined && old !== r.name) throw new Error(`VA 0x${r.va.toString(16)} is ${old} and ${r.name}`);
    if (old === undefined) rows.set(r.va, r.name);
  } else if (l.startsWith('};')) {
    intab = false;
  }
}
{
  const i = lines.findIndex((l) => l.startsWith('const fle_resp_entry fle_resp_table[] = {'));
  if (i < 0) throw new Error('no response table');
  const j = lines.findIndex((l, k) => k >= i && l.startsWith('const uint64_t fle_resp_count'));
  if (j < 0) throw new Error('no response count');
  resp = lines.slice(i, j + 1).join('\n');
}
const coll = [...count].filter(([va, c]) => c > 1 && va < RESP_BASE).map(([va]) => va);
if (coll.length) throw new Error(`f_<VA> collision between modules: ${coll.length} (first 0x${coll[0].toString(16)})`);

if (rowsPath) {
  for (const [no, raw] of fs.readFileSync(rowsPath, 'utf8').split('\n').entries()) {
    const t = raw.split('#')[0].trim();
    if (!t) continue;
    const parts = t.split(/\s+/);
    const bad = (why) => new Error(`${rowsPath}:${no + 1}: ${why}`);
    if (parts.length !== 2) throw bad('want "<0x VA> <name>"');
    if (!/^0x[0-9a-f]+$/.test(parts[0])) throw bad('bad VA');
    const va = BigInt(parts[0]);
    const name = parts[1];
    if (va < RESP_BASE || rows.has(va) || [...rows.values()].includes(name)) throw bad('VA or name already used, or below the pseudo-VA region');
    rows.set(va, name);
  }
}

let s = '/* merged dispatch table of one module (lift/harness/vst/merge_tables.mjs) */\n#include "fle_runtime.h"\n#include "fle_sse.h"\n';
s += '#include "fle_dispatch.h"\n#include "fle_modules.h"\n\n';
const sorted = [...rows].sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
for (const name of new Set(sorted.map(([, n]) => n))) s += `void ${name}(void);\n`;
s += '\nconst fle_fn_entry fle_dispatch_table[] = {\n';
for (const [va, name] of sorted) s += `    { UINT64_C(0x${va.toString(16)}), ${name} },\n`;
s += `};\nconst uint64_t fle_dispatch_count = ${sorted.length};\n\n`;
s += resp + '\n';
fs.writeFileSync(outPath, s);
console.error(`merge_tables: ${sorted.length} dispatch rows`);
