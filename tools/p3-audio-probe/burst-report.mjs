// Burst report for a CDP CPU profile (#1352): the self time of each function by thread, the longest stretch of busy time that ran without a switch point for each function, and the longest switch-free runs of the whole profile.
// Usage: node tools/p3-audio-probe/burst-report.mjs <profile.json> <wasm-func-names.tsv> [--hyb DIR] [--switch A,B] [--from-ms MS] [--to-ms MS] [--top N] [--runs N] > report.json
// --switch: the names of the switch points (default fle_progress_slice,fle_loop_slice). A busy sample whose stack has one of them is a switch point; a switch-free run is a maximal run of consecutive busy samples without one (an idle sample ends it too).
// --hyb DIR: the per-module tables of fe-wasmgen hybrid (lift/out-app/hyb, with dir/): a function whose status is "not selected for the new backend (hybrid)" is C (fe-lift output), "ok" is wasm.
// The per-function stretch is the longest run of consecutive busy samples without a switch point whose stacks all contain the function (the sample resolution is the profile's interval).
// Times are ms from the start of the profile (the engine-idle of measure.mjs, for the Worker and the page alike). Sampled data: each sample's time is its interval to the next sample.
import fs from 'node:fs';
import path from 'node:path';

const argv = process.argv.slice(2);
const val = (name, def) => { const i = argv.indexOf(name); return i < 0 ? def : argv[i + 1]; };
const valueFlags = ['--switch', '--from-ms', '--to-ms', '--top', '--runs', '--hyb'];
const pos = argv.filter((a, i) => !a.startsWith('--') && !valueFlags.includes(argv[i - 1]));
if (pos.length !== 2) {
  process.stderr.write('usage: node tools/p3-audio-probe/burst-report.mjs <profile.json> <wasm-func-names.tsv> [--hyb DIR] [--switch A,B] [--from-ms MS] [--to-ms MS] [--top N] [--runs N]\n');
  process.exit(2);
}
const [profilePath, tablePath] = pos;
const switches = new Set(val('--switch', 'fle_progress_slice,fle_loop_slice').split(',').filter(Boolean));
const fromMs = Number(val('--from-ms', '0'));
const toMs = Number(val('--to-ms', '1e12'));
const top = Number(val('--top', '20'));
const runsN = Number(val('--runs', '10'));
const hybDir = val('--hyb', 'lift/out-app/hyb');

const table = new Map();
for (const line of fs.readFileSync(tablePath, 'utf8').split('\n').slice(1)) {
  if (!line) continue;
  const [index, kind, va, name] = line.split('\t');
  table.set(Number(index), { kind, va, name });
}

// the hybrid status of each VA: wasm ("ok") or C (fe-lift)
const status = new Map();
const hybFiles = [];
if (fs.existsSync(hybDir)) {
  for (const f of fs.readdirSync(hybDir)) if (f.endsWith('.tsv') && f !== 'wasm-func-names.tsv') hybFiles.push(path.join(hybDir, f));
  const dir = path.join(hybDir, 'dir');
  if (fs.existsSync(dir)) for (const f of fs.readdirSync(dir)) if (f.endsWith('.tsv')) hybFiles.push(path.join(dir, f));
}
for (const f of hybFiles) {
  for (const line of fs.readFileSync(f, 'utf8').split('\n').slice(1)) {
    const c = line.split('\t');
    if (c.length < 8 || !/^0x[0-9a-f]+$/.test(c[0])) continue;
    const st = c[7];
    const prev = status.get(c[0]);
    if (prev === undefined || st === 'ok') status.set(c[0], st);
  }
}
const whereOf = (name) => {
  const m = /^f_([0-9a-f]+)$/.exec(name);
  if (!m) return name.startsWith('fle_') || name.startsWith('poll_') ? 'C runtime' : name === '(program)' || name === '(garbage collector)' ? '-' : 'other';
  const st = status.get(`0x${m[1]}`);
  if (st === undefined) return 'C (fe-lift; no status row)';
  return st === 'ok' ? 'wasm' : `C (fe-lift: ${st})`;
};

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
  let l = m ? (row?.name ? row.name : `${fn} (not in the table)`) : fn;
  if (/^(fle_|poll_)/.test(l)) l = l.replace(/_\d+$/, '');
  labelOf.set(id, l);
  return l;
};
const isSwitch = (l) => switches.has(l);
// the stack of a node, from the leaf to the root (labels)
const stackOf = (id) => {
  const out = [];
  for (let cur = id; cur !== undefined; cur = parent.get(cur)) out.push(label(cur));
  return out;
};

const n = prof.samples.length;
const tStart = new Float64Array(n);
const dur = new Float64Array(n);
let acc = 0;
for (let i = 0; i < n; i++) {
  tStart[i] = acc;
  dur[i] = (prof.timeDeltas[i + 1] ?? 0) / 1000;
  acc += (prof.timeDeltas[i] ?? 0) / 1000;
}

const selfMs = new Map();          // label -> ms
const selfByThread = new Map();    // label -> Map(thread -> ms)
const longestOf = new Map();       // label -> { ms, startMs }
const runs = [];
let busyMs = 0, idleMs = 0, switchMs = 0;
const cur = new Map();             // label -> { ms, startMs }
let run = null;

const flushCur = (l) => {
  const c = cur.get(l);
  if (!c) return;
  const best = longestOf.get(l);
  if (!best || c.ms > best.ms) longestOf.set(l, { ms: c.ms, startMs: c.startMs });
  cur.delete(l);
};
const flushAll = () => { for (const l of [...cur.keys()]) flushCur(l); };
const endRun = () => {
  if (run) {
    runs.push(run);
    run = null;
  }
  flushAll();
};

for (let i = 0; i < n; i++) {
  const t = tStart[i];
  if (t < fromMs || t >= toMs) continue;
  const d = dur[i];
  const st = stackOf(prof.samples[i]);
  const leaf = st[0];
  if (leaf === '(idle)') {
    idleMs += d;
    endRun();
    continue;
  }
  busyMs += d;
  const root = st[st.length - 1];
  const thread = st.length >= 2 ? st[st.length - 2] : root;
  selfMs.set(leaf, (selfMs.get(leaf) ?? 0) + d);
  if (!selfByThread.has(leaf)) selfByThread.set(leaf, new Map());
  const bt = selfByThread.get(leaf);
  bt.set(thread, (bt.get(thread) ?? 0) + d);
  if (st.some(isSwitch)) {
    switchMs += d;
    endRun();
    continue;
  }
  // the functions on this stack continue their stretch; the others end theirs
  const on = new Set(st);
  for (const l of [...cur.keys()]) if (!on.has(l)) flushCur(l);
  for (const l of on) {
    const c = cur.get(l);
    if (c) c.ms += d;
    else cur.set(l, { ms: d, startMs: t });
  }
  if (!run) run = { ms: 0, startMs: t, leaves: new Map(), threads: new Map(), chain: null };
  run.ms += d;
  run.leaves.set(leaf, (run.leaves.get(leaf) ?? 0) + d);
  run.threads.set(thread, (run.threads.get(thread) ?? 0) + d);
  if (!run.chain) run.chain = st.slice().reverse().slice(0, 10);
}
endRun();

const round = (x, k = 1) => Math.round(x * 10 ** k) / 10 ** k;
const dominantThread = (l) => {
  const m = selfByThread.get(l);
  if (!m) return '';
  return [...m.entries()].sort((a, b) => b[1] - a[1])[0][0];
};
const ranked = [...selfMs.entries()].sort((a, b) => b[1] - a[1]).slice(0, top).map(([name, ms], k) => {
  const m = /^f_([0-9a-f]+)$/.exec(name);
  const row = [...table.values()].find((x) => x.name === name);
  const lo = longestOf.get(name);
  return {
    rank: k + 1,
    name,
    kind: row?.kind ?? (m ? 'f' : 'c'),
    va: row?.va ?? (m ? `0x${m[1]}` : ''),
    thread: dominantThread(name),
    threads: Object.fromEntries([...(selfByThread.get(name)?.entries() ?? [])].map(([k, v]) => [k, round(v)])),
    selfMs: round(ms),
    shareOfBusy: round(ms / busyMs, 3),
    longestNoSwitchMs: lo ? round(lo.ms) : 0,
    longestNoSwitchStartMs: lo ? round(lo.startMs) : null,
    where: whereOf(name),
  };
});
const longRuns = runs.slice().sort((a, b) => b.ms - a.ms).slice(0, runsN).map((r, k) => ({
  rank: k + 1,
  startMs: round(r.startMs),
  lengthMs: round(r.ms),
  threads: [...r.threads.entries()].sort((a, b) => b[1] - a[1]).map(([t, ms]) => ({ thread: t, ms: round(ms) })),
  leaves: [...r.leaves.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5).map(([name, ms]) => ({ name, ms: round(ms), where: whereOf(name) })),
  chainFromRoot: r.chain,
}));
process.stdout.write(JSON.stringify({
  profile: profilePath,
  windowMs: [round(fromMs), Math.min(round(toMs), round(acc))],
  switchFunctions: [...switches],
  busyMs: round(busyMs),
  idleMs: round(idleMs),
  switchMs: round(switchMs),
  top: ranked,
  longestRuns: longRuns,
}, null, 1) + '\n');
