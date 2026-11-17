// R4-5c: the input latency T recomputed from saved lat files. Usage: node tools/verify-kit/tools/r45c-lat-recompute.mjs <dir of run folders>. Each run folder holds lat-a-idle-menu.json, lat-c-mixer-drag.json, lat-d-space.json and lat-b-play-menu.json (the files of sdriveg2-lat.mjs). The definitions (ORDER, pct, load) are those of tools/verify-kit/tools/lat-ab.py, ported 1:1; the output goes to stdout (tab-separated, relative names only).
import fs from 'node:fs';
import path from 'node:path';

const ORDER = ['a-idle-menu', 'c-mixer-drag', 'd-space', 'b-play-menu'];
const SECTIONS = ['a-idle-menu', 'b-play-menu'];
const KINDS = { 2: 'File click (button down)', 5: 'Esc (key down)' };
const KEYS = [
  'a-idle-menu|File click (button down)',
  'a-idle-menu|Esc (key down)',
  'b-play-menu|File click (button down)',
  'b-play-menu|Esc (key down)',
];

function pct(v, p) {
  if (!v || v.length === 0) return null;
  const s = [...v].sort((a, b) => a - b);
  const i = Math.min(s.length - 1, Math.max(0, Math.ceil((p * s.length) / 100) - 1));
  return s[i];
}

function readRecords(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8')).records;
}

function load(d) {
  const last = {};
  let prev = -1;
  for (const x of ORDER) {
    const p = path.join(d, `lat-${x}.json`);
    if (fs.existsSync(p)) {
      last[x] = prev;
      const seqs = readRecords(p).map((r) => r.seq);
      prev = seqs.length ? Math.max(...seqs) : prev;
    }
  }
  const out = new Map();
  for (const sec of SECTIONS) {
    const p = path.join(d, `lat-${sec}.json`);
    if (!fs.existsSync(p)) continue;
    for (const r of readRecords(p)) {
      if (r.dropped || r.seq <= (last[sec] ?? -1) || r.kind === 1) continue;
      const k = KINDS[r.kind];
      if (!k) continue;
      const key = `${sec}|${k}`;
      if (!out.has(key)) out.set(key, { v: null, n: 0 });
      const o = out.get(key);
      o.n += 1;
      const fr = r.frames;
      if (fr && Object.keys(fr).length) {
        const lid = Object.keys(fr).sort()[0];
        o.v = (o.v || []).concat([fr[lid] - r.tPage]);
      }
    }
  }
  return out;
}

function median(xs) {
  const s = [...xs].sort((a, b) => a - b);
  const m = s.length >> 1;
  return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
}

function group(run) {
  if (/^r42c-rtp2r\d+-chrome-1$/.test(run)) return 'rt(実スレッド版)';
  if (/^r42c-rt1p2r\d+-chrome-1$/.test(run)) return 'rt1(対照)';
  if (/^r42c-cop2r\d+-chrome-1$/.test(run)) return 'co 束1(chrome-1)';
  if (/^r42c-cop2r\d+-chrome-2$/.test(run)) return 'co 束2(chrome-2)';
  return 'other';
}

const dir = process.argv[2];
if (!dir) {
  process.stderr.write('usage: node tools/verify-kit/tools/r45c-lat-recompute.mjs <dir of run folders>\n');
  process.exit(2);
}
const f3 = (x) => (x === null ? '-' : x.toFixed(3));
const runs = fs.readdirSync(dir).filter((n) => fs.statSync(path.join(dir, n)).isDirectory()).sort();
const lines = [];
lines.push(`# R4-5c: input latency T (ms) per run, recomputed from ${dir} (tools/verify-kit/tools/r45c-lat-recompute.mjs; the definitions of lat-ab.py)`);
lines.push(['run', 'section|input', 'n_sent', 'n_answered', 'p50_ms', 'p90_ms', 'p99_ms'].join('\t'));
const byGroup = new Map();
for (const run of runs) {
  const o = load(path.join(dir, run));
  for (const k of KEYS) {
    const e = o.get(k) || { v: null, n: 0 };
    const v = e.v || [];
    const p50 = pct(v, 50), p90 = pct(v, 90), p99 = pct(v, 99);
    lines.push([run, k, e.n, v.length, f3(p50), f3(p90), f3(p99)].join('\t'));
    const g = group(run);
    if (!byGroup.has(g)) byGroup.set(g, new Map());
    if (!byGroup.get(g).has(k)) byGroup.get(g).set(k, { p50: [], p90: [], p99: [] });
    const acc = byGroup.get(g).get(k);
    if (p50 !== null) { acc.p50.push(p50); acc.p90.push(p90); acc.p99.push(p99); }
  }
}
lines.push('');
lines.push('# medians over the runs of each group (the group is taken from the folder name)');
lines.push(['group', 'section|input', 'runs', 'median_p50', 'min_p50', 'max_p50', 'median_p90', 'median_p99'].join('\t'));
for (const [g, m] of byGroup) {
  for (const k of KEYS) {
    const acc = m.get(k);
    if (!acc || acc.p50.length === 0) continue;
    lines.push([g, k, acc.p50.length, f3(median(acc.p50)), f3(Math.min(...acc.p50)), f3(Math.max(...acc.p50)), f3(median(acc.p90)), f3(median(acc.p99))].join('\t'));
  }
}
process.stdout.write(lines.join('\n') + '\n');
