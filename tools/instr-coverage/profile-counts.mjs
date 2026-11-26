// Issue #1339: the per-function counts of a CDP CPU profile of the engine's Worker (tools/p3-audio-probe/measure.mjs --profile, in a run), as the TSV that tools/instr-coverage/measure.mjs reads: va, module, instructions. The column instructions is the number of samples of the function as the leaf frame (self time, 1 sample = one interval of the profile, about 0.35 ms here), not an instruction count. (idle) is not counted; every other sample is (the engine's wasm functions, its C functions, the page's JS and V8 internals).
// va: the guest VA (0x...) of a lifted function (kind f or ia in wasm-func-names.tsv); otherwise the function's name (a C function: fle_*, unnamed_N; a JS function: js:<name>@<url>; the V8 frames: (program) etc.).
// module: the engine module of a guest VA (the n0-<module>.tsv of the hybrid build, lift/out-app/hyb); runtime (a C function of the engine), unnamed (a defined wasm function without a name), js (a page's JS frame), v8 ((program), (garbage collector), (root)).
// Usage: node tools/instr-coverage/profile-counts.mjs <profile.json> <wasm-func-names.tsv> <hyb directory> [--note TEXT] > inputs/p3-profile.tsv
// stdout: the TSV (a comment line, the header, one row per va, by instructions). stderr: the total and the rows per module. Exit 2 when the arguments are wrong.
import fs from 'node:fs';
import path from 'node:path';

const USAGE = 'usage: node tools/instr-coverage/profile-counts.mjs <profile.json> <wasm-func-names.tsv> <hyb directory> [--note TEXT] > inputs/p3-profile.tsv\n';
const argv = process.argv.slice(2);
const noteAt = argv.indexOf('--note');
const note = noteAt < 0 ? null : argv[noteAt + 1];
const pos = argv.filter((a, i) => !a.startsWith('--') && (noteAt < 0 || i !== noteAt + 1));
if (pos.length !== 3 || (noteAt >= 0 && (note === undefined || note.startsWith('--')))) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const [profilePath, namesPath, hybDir] = pos.map((p) => path.resolve(p));

const VA = /^0x[0-9a-f]+$/i;

// the engine's function table: index -> { kind, va, name } (the rows of wasm-func-names.tsv)
const table = new Map();
for (const line of fs.readFileSync(namesPath, 'utf8').split('\n').slice(1)) {
  if (!line) continue;
  const [index, kind, va, name] = line.split('\t');
  table.set(Number(index), { kind, va, name });
}

// the module of each guest VA: the hybrid build's per-module table (n0-<module>.tsv; a name with a dot is a separate plugin module, not in the engine). A VA that the module lists as ok (direct) is that module's; else the first module that lists it.
const moduleOf = new Map();
for (const f of fs.readdirSync(hybDir).sort()) {
  const m = /^n0-([a-z0-9]+)\.tsv$/.exec(f);
  if (!m) continue;
  const mod = m[1];
  for (const line of fs.readFileSync(path.join(hybDir, f), 'utf8').split('\n').slice(1)) {
    const cols = line.split('\t');
    const va = (cols[0] ?? '').toLowerCase();
    if (!VA.test(va)) continue;
    const ok = cols[7] === 'ok';
    const prev = moduleOf.get(va);
    if (prev === undefined || (ok && !prev.ok)) moduleOf.set(va, { module: mod, ok });
  }
}

const prof = JSON.parse(fs.readFileSync(profilePath, 'utf8'));
const byId = new Map(prof.nodes.map((n) => [n.id, n]));
const samples = new Map();
for (const id of prof.samples) samples.set(id, (samples.get(id) ?? 0) + 1);

// a JS frame's URL without the page's origin (its port changes per run); a blob URL (its id changes per run) is blob
const urlKey = (u) => (!u ? '' : u.startsWith('blob:') ? 'blob' : u.replace(/^https?:\/\/[^/]+/, ''));

// one row per key: the key is the va column, the module is the module column
const rows = new Map();
const add = (key, module, n) => {
  const r = rows.get(key) ?? { module, n: 0 };
  r.n += n;
  rows.set(key, r);
};
let total = 0;
let unknownIndex = 0;
for (const [id, n] of samples) {
  const f = byId.get(id).callFrame;
  const fn = f.functionName || '(anonymous)';
  if (fn === '(idle)') continue;
  total += n;
  const w = /^wasm-function\[(\d+)\]$/.exec(fn);
  if (w) {
    const row = table.get(Number(w[1]));
    if (!row) { unknownIndex += n; add(`wasm-function[${w[1]}]`, 'unknown', n); continue; }
    if (row.kind === 'f' || row.kind === 'ia') {
      const va = row.va.toLowerCase();
      add(va, moduleOf.get(va)?.module ?? 'unknown', n);
    } else if (row.kind === 'c' && /^unnamed_\d+$/.test(row.name)) {
      add(row.name, 'unnamed', n);
    } else if (row.kind === 'c') {
      add(row.name, 'runtime', n);
    } else {
      add(row.name, 'import', n);
    }
    continue;
  }
  if (/^\((program|garbage collector|root)\)$/.test(fn)) { add(fn, 'v8', n); continue; }
  add(`js:${fn}@${urlKey(f.url)}`, 'js', n);
}
if (unknownIndex > 0) process.stderr.write(`warning: ${unknownIndex} samples of wasm-function indices that the table does not have\n`);

const out = [];
out.push(`# instructions は命令数ではなく、サンプル数(その関数が葉にあったサンプルの数。(idle) を除く。1 サンプルは CDP の CPU プロファイルの 1 区間で、約 0.35 ms)。直接 wasm の判定は va が lift/harness/hyb/lists/lowered-*.txt に在るか。${note === null ? '' : ' ' + note}`);
out.push('va\tmodule\tinstructions');
const sorted = [...rows.entries()].sort((a, b) => b[1].n - a[1].n || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
for (const [key, r] of sorted) out.push(`${key}\t${r.module}\t${r.n}`);
process.stdout.write(out.join('\n') + '\n');

const perModule = new Map();
for (const r of rows.values()) perModule.set(r.module, (perModule.get(r.module) ?? 0) + r.n);
process.stderr.write(`samples (not idle): ${total}; rows: ${rows.size}\n`);
for (const [m, n] of [...perModule].sort((a, b) => b[1] - a[1])) process.stderr.write(`  ${m}\t${n}\n`);
