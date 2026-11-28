// Issue #1339: the share of a profile's instructions that run in direct wasm (M7: the direct-wasm share during P3 playback). The input is the TSV of tools/instr-coverage/profile-counts.mjs: va, module, instructions (a comment line starting with # and the header are skipped). A row is direct when its va is a VA that lift/harness/hyb/lists/lowered-*.txt lists (one VA per line): the functions that the build lowers to direct wasm.
// Issue #1340: coveragePct is over the lifted functions only: the rows whose va is a VA (instrVa). The rows that are not lifted functions (the runtime's C, js, v8, unnamed) are left out of it. coveragePctAll is the #1339 definition, over every row (instrTotal).
// Usage: node tools/instr-coverage/measure.mjs <inputs/p3-profile.tsv> > cov.json
// stdout: one JSON line {"instrTotal","instrVa","instrDirect","coveragePct","baselinePct","coveragePctAll","baselinePctAll"}. instrTotal: the sum of instructions of all rows; instrVa: the sum of the rows whose va is a VA; instrDirect: the sum of the direct rows; coveragePct: 100 x instrDirect / instrVa (2 decimals); coveragePctAll: 100 x instrDirect / instrTotal (2 decimals).
// baselinePct, baselinePctAll: the coveragePct and coveragePctAll of the first computation, recorded next to the TSV (<name>.baseline.json: {"baselinePct","baselinePctAll"}); the file is written when it is missing, and read after that (the #1340 judgement compares with it).
// stderr: the rows per module (the instructions, the direct share of each module). Exit 2 when the argument is wrong, 1 when the TSV has no rows, the lists are empty, or no row is a VA.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const USAGE = 'usage: node tools/instr-coverage/measure.mjs <inputs/p3-profile.tsv> > cov.json\n';
const argv = process.argv.slice(2);
if (argv.length !== 1 || argv[0].startsWith('--')) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const tsvPath = path.resolve(argv[0]);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const LISTS_DIR = path.join(repo, 'lift', 'harness', 'hyb', 'lists');
const VA = /^0x[0-9a-f]+$/i;

const direct = new Set();
for (const f of fs.readdirSync(LISTS_DIR).sort()) {
  if (!/^lowered-.*\.txt$/.test(f)) continue;
  for (const line of fs.readFileSync(path.join(LISTS_DIR, f), 'utf8').split('\n')) {
    const va = line.trim().toLowerCase();
    if (VA.test(va)) direct.add(va);
  }
}
if (direct.size === 0) {
  process.stderr.write('no VA in lift/harness/hyb/lists/lowered-*.txt\n');
  process.exit(1);
}

const perModule = new Map();
let instrTotal = 0;
let instrVa = 0;
let instrDirect = 0;
let rowCount = 0;
for (const line of fs.readFileSync(tsvPath, 'utf8').split('\n')) {
  if (line === '' || line.startsWith('#') || line === 'va\tmodule\tinstructions') continue;
  const [va, module, count] = line.split('\t');
  const n = Number(count);
  if (!Number.isFinite(n) || n < 0) {
    process.stderr.write(`bad row: ${line.slice(0, 200)}\n`);
    process.exit(1);
  }
  rowCount++;
  const isVa = VA.test(va);
  const isDirect = isVa && direct.has(va.toLowerCase());
  instrTotal += n;
  if (isVa) instrVa += n;
  if (isDirect) instrDirect += n;
  const m = perModule.get(module) ?? { total: 0, direct: 0 };
  m.total += n;
  if (isDirect) m.direct += n;
  perModule.set(module, m);
}
if (rowCount === 0 || instrTotal === 0 || instrVa === 0) {
  process.stderr.write(`no instructions of lifted functions in ${tsvPath}\n`);
  process.exit(1);
}
const pct = (d, t) => Math.round((10000 * d) / t) / 100;
const coveragePct = pct(instrDirect, instrVa);
const coveragePctAll = pct(instrDirect, instrTotal);

const baselinePath = tsvPath.replace(/\.tsv$/, '') + '.baseline.json';
if (!fs.existsSync(baselinePath)) {
  fs.writeFileSync(baselinePath, JSON.stringify({ baselinePct: coveragePct, baselinePctAll: coveragePctAll, recordedFrom: path.basename(tsvPath), instrTotal, instrVa, instrDirect }, null, 1) + '\n');
  process.stderr.write(`baseline recorded: ${baselinePath}\n`);
}
const baseline = JSON.parse(fs.readFileSync(baselinePath, 'utf8'));
const baselinePct = baseline.baselinePct;
const baselinePctAll = baseline.baselinePctAll;

for (const [module, m] of [...perModule].sort((a, b) => b[1].total - a[1].total)) {
  process.stderr.write(`${module}\t${m.total}\tdirect ${m.direct}\t${pct(m.direct, m.total)}%\tshare of all ${pct(m.total, instrTotal)}%\n`);
}
process.stdout.write(JSON.stringify({ instrTotal, instrVa, instrDirect, coveragePct, baselinePct, coveragePctAll, baselinePctAll }) + '\n');
