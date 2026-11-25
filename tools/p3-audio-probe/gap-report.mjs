// Publish-gap report (#1375): the gaps of the engine's output ring against the CPU profile of the engine's Worker. A gap is the interval between two publishes of the output ring (fle_js_publish) with both ends in the window and both found the output running (the engine's own rule for maxPubGap). The publish log comes from measure.mjs --boot --pubs FILE, the profile from measure.mjs --boot --profile FILE (its profile.fleClock maps the Worker's performance.now() to the CDP clock).
// For each gap longer than --min-ms: the profile's samples that fall in it (self time by function, the switch-point samples in it, the most common stacks from the leaf up). A switch-point sample means a switch point was in the stack, so the gap had one there but no publish came of it (a point that did not refill the ring).
// Usage: node tools/p3-audio-probe/gap-report.mjs <profile.json> <wasm-func-names.tsv> <pubs.json> [--min-ms 40] [--top 10] [--switch A,B] > report.json
// Times are ms from the window start (w0 of the publish log). Stacks are labelled like burst-report.mjs (wasm-function[N] through the names table; fle_ and poll_ frames without the _N suffix).
import fs from 'node:fs';

const argv = process.argv.slice(2);
const val = (name, def) => { const i = argv.indexOf(name); return i < 0 ? def : argv[i + 1]; };
const valueFlags = ['--min-ms', '--top', '--switch'];
const pos = argv.filter((a, i) => !a.startsWith('--') && !valueFlags.includes(argv[i - 1]));
if (pos.length !== 3) {
  process.stderr.write('usage: node tools/p3-audio-probe/gap-report.mjs <profile.json> <wasm-func-names.tsv> <pubs.json> [--min-ms 40] [--top 10] [--switch A,B] [--all]\n');
  process.exit(2);
}
const whole = argv.includes('--all');
const [profilePath, tablePath, pubsPath] = pos;
const minMs = Number(val('--min-ms', '40'));
const top = Number(val('--top', '10'));
const switches = new Set(val('--switch', 'fle_progress_slice,fle_loop_slice').split(',').filter(Boolean));

const table = new Map();
for (const line of fs.readFileSync(tablePath, 'utf8').split('\n').slice(1)) {
  if (!line) continue;
  const [index, kind, va, name] = line.split('\t');
  table.set(Number(index), { kind, va, name });
}

const prof = JSON.parse(fs.readFileSync(profilePath, 'utf8'));
if (!prof.fleClock) {
  process.stderr.write('the profile has no fleClock (take it with measure.mjs --boot --profile)\n');
  process.exit(1);
}
const pubs = JSON.parse(fs.readFileSync(pubsPath, 'utf8'));
const entries = pubs.entries;
// --all: every publish of the log (from the engine-idle on); else the window of the probe (w0 to w1). Times are ms from the base: the window start, or the first publish with --all.
const w0 = whole ? entries[0][0] : pubs.w0;
const w1 = whole ? Infinity : pubs.w1;
const offsetUs = prof.fleClock.offsetUs;

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
const stackOf = (id) => {
  const out = [];
  for (let cur = id; cur !== undefined; cur = parent.get(cur)) out.push(label(cur));
  return out;
};

// sample k covers [start[k], start[k + 1]) in CDP microseconds (timeDeltas[k] is the interval from sample k-1)
const n = prof.samples.length;
const start = new Float64Array(n);
{
  let acc = prof.startTime;
  for (let k = 0; k < n; k++) {
    acc += prof.timeDeltas[k] ?? 0;
    start[k] = acc;
  }
}
const endOf = (k) => (k + 1 < n ? start[k + 1] : prof.endTime);
const firstAtOrAfter = (us) => {
  let lo = 0, hi = n;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (start[mid] < us) lo = mid + 1; else hi = mid;
  }
  return lo;
};
const cdpUs = (perfMs) => perfMs * 1000 + offsetUs;

const gaps = [];
for (let i = 1; i < entries.length; i++) {
  const [t, r] = entries[i];
  const [tp, rp] = entries[i - 1];
  if (tp < w0 || t > w1 || !r || !rp) continue;
  if (t - tp < minMs) continue;
  gaps.push({ tp, t, ms: t - tp });
}

const reportOf = (g) => {
  const a = cdpUs(g.tp), b = cdpUs(g.t);
  const self = new Map();
  const stacks = new Map();
  let covered = 0, idle = 0, switchSamples = 0, samples = 0;
  for (let k = firstAtOrAfter(a); k < n && start[k] < b; k++) {
    const s = Math.max(start[k], a), e = Math.min(endOf(k), b);
    if (e <= s) continue;
    const d = (e - s) / 1000;
    samples++;
    const st = stackOf(prof.samples[k]);
    if (st[0] === '(idle)') {
      idle += d;
      continue;
    }
    covered += d;
    self.set(st[0], (self.get(st[0]) ?? 0) + d);
    if (st.some((l) => switches.has(l))) switchSamples++;
    const chain = st.slice(0, 6).join(' < ');
    stacks.set(chain, (stacks.get(chain) ?? 0) + d);
  }
  const byMs = (m, k) => [...m.entries()].sort((x, y) => y[1] - x[1]).slice(0, k).map(([name, ms]) => [name, Math.round(ms * 10) / 10]);
  return {
    startMs: Math.round((g.tp - w0) * 10) / 10,
    lengthMs: Math.round(g.ms * 10) / 10,
    samples,
    busyMs: Math.round(covered * 10) / 10,
    idleMs: Math.round(idle * 10) / 10,
    switchSamples,
    leaves: byMs(self, 6),
    stacks: byMs(stacks, 3),
  };
};

const all = gaps.map(reportOf);
const hist = { '40-60': 0, '60-100': 0, '100-200': 0, '200+': 0 };
for (const g of all) {
  if (g.lengthMs < 60) hist['40-60']++;
  else if (g.lengthMs < 100) hist['60-100']++;
  else if (g.lengthMs < 200) hist['100-200']++;
  else hist['200+']++;
}
const out = {
  window: { probeW0: pubs.w0, probeW1: pubs.w1, base: w0, whole, probeMs: Math.round((pubs.w1 - pubs.w0) * 10) / 10 },
  publishesInWindow: entries.filter(([t]) => t >= w0 && t <= w1).length,
  gapsOver: { minMs, count: all.length, histogram: hist },
  longest: all.sort((x, y) => y.lengthMs - x.lengthMs).slice(0, top),
};
process.stdout.write(JSON.stringify(out, null, 1) + '\n');
