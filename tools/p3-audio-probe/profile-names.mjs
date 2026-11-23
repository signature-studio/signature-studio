// Names for a CDP CPU profile of the engine's Worker (#1334). The profile's frames are wasm-function[N] (N: the function index of engine.wasm); the table lift/out-app/gen/wasm-func-names.tsv (written by fe-wasmgen hybrid, copied by fe-build) gives each index its kind, guest VA and name.
// Usage: node tools/p3-audio-probe/profile-names.mjs <profile.json> <wasm-func-names.tsv> [--switch NAME] [--top N] > report.json
// Self time is the sum of each sample's interval (to the next sample) by its leaf function. "(idle)" samples are not busy. A run without a switch point is a maximal run of busy samples whose stacks do not contain the switch function (NAME, default fle_progress_slice: the scheduler's slice, which the dispatch calls at a switch point); the longest one is the longest time the engine ran without a switch point (sampled: the resolution is the profile's interval).
// stdout: one JSON object. Exit 2 when the arguments are wrong.
import fs from 'node:fs';

const argv = process.argv.slice(2);
const val = (name, def) => { const i = argv.indexOf(name); return i < 0 ? def : argv[i + 1]; };
const pos = argv.filter((a, i) => !a.startsWith('--') && (i === 0 || !['--switch', '--top'].includes(argv[i - 1])));
if (pos.length !== 2) {
  process.stderr.write('usage: node tools/p3-audio-probe/profile-names.mjs <profile.json> <wasm-func-names.tsv> [--switch NAME] [--top N]\n');
  process.exit(2);
}
const [profilePath, tablePath] = pos;
const switchName = val('--switch', 'fle_progress_slice');
const top = Number(val('--top', '20'));

const table = new Map();
for (const line of fs.readFileSync(tablePath, 'utf8').split('\n').slice(1)) {
  if (!line) continue;
  const [index, kind, va, name] = line.split('\t');
  table.set(Number(index), { kind, va, name });
}

const prof = JSON.parse(fs.readFileSync(profilePath, 'utf8'));
const byId = new Map(prof.nodes.map((n) => [n.id, n]));
const parent = new Map();
for (const n of prof.nodes) for (const c of n.children ?? []) parent.set(c, n.id);

const labelOf = new Map();
const label = (id) => {
  if (labelOf.has(id)) return labelOf.get(id);
  const fn = byId.get(id).callFrame.functionName || '(anonymous)';
  const m = /^wasm-function\[(\d+)\]$/.exec(fn);
  const row = m ? table.get(Number(m[1])) : undefined;
  const l = m ? (row?.name ? row.name : `${fn} (not in the table)`) : fn;
  labelOf.set(id, l);
  return l;
};

const dur = new Float64Array(prof.samples.length);
const cum = new Float64Array(prof.samples.length);
for (let i = 0; i < prof.samples.length; i++) {
  dur[i] = (prof.timeDeltas[i + 1] ?? 0) / 1000;
  cum[i] = (i > 0 ? cum[i - 1] : 0) + (prof.timeDeltas[i] ?? 0) / 1000;
}
const stackHas = (id, name) => {
  for (let cur = id; cur !== undefined; cur = parent.get(cur)) if (label(cur) === name) return true;
  return false;
};

const selfMs = new Map();
let idleMs = 0;
let busyMs = 0;
let run = null;
let longest = { ms: 0 };
for (let i = 0; i < prof.samples.length; i++) {
  const id = prof.samples[i];
  const leaf = label(id);
  const d = dur[i];
  if (leaf === '(idle)') {
    idleMs += d;
    if (run) { if (run.ms > longest.ms) longest = run; run = null; }
    continue;
  }
  busyMs += d;
  selfMs.set(leaf, (selfMs.get(leaf) ?? 0) + d);
  if (stackHas(id, switchName)) {
    if (run) { if (run.ms > longest.ms) longest = run; run = null; }
    continue;
  }
  if (!run) run = { ms: 0, startMs: cum[i], leaves: new Map() };
  run.ms += d;
  run.leaves.set(leaf, (run.leaves.get(leaf) ?? 0) + d);
}
if (run && run.ms > longest.ms) longest = run;

const rowOf = (name) => {
  const m = /^f_([0-9a-f]+)$/.exec(name);
  const r = [...table.values()].find((x) => x.name === name);
  return r ? { kind: r.kind, va: r.va } : m ? { kind: 'f', va: `0x${m[1]}` } : { kind: '', va: '' };
};
const ranked = [...selfMs.entries()].sort((a, b) => b[1] - a[1]).slice(0, top).map(([name, ms], k) => ({
  rank: k + 1, name, ...rowOf(name), selfMs: Math.round(ms * 10) / 10, shareOfBusy: Math.round((1000 * ms) / busyMs) / 1000,
}));
const leavesOfLongest = longest.leaves ? [...longest.leaves.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5).map(([name, ms]) => ({ name, ms: Math.round(ms * 10) / 10 })) : [];
process.stdout.write(JSON.stringify({
  profile: profilePath,
  switchFunction: switchName,
  samples: prof.samples.length,
  intervalUs: Math.round(((prof.endTime - prof.startTime) / prof.samples.length) * 1000) / 1000,
  spanMs: Math.round((prof.endTime - prof.startTime) / 100) / 10,
  idleMs: Math.round(idleMs),
  busyMs: Math.round(busyMs),
  top: ranked,
  longestRunWithoutSwitch: { ms: Math.round(longest.ms * 10) / 10, startMs: longest.startMs === undefined ? null : Math.round(longest.startMs), leaves: leavesOfLongest },
}, null, 1) + '\n');
