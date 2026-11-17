// R4-5d: the lock activity of each Worker inside the playback window of one run (verification only; reads the files of one run folder, writes nothing unless --out). Usage: node tools/verify-kit/tools/r4-5d-window.mjs RUNDIR [--out FILE.tsv]. The playback window is the time from the first answered record of lat-b-play-menu.json (after the same filter as lat-ab.py) to its last frame (epoch ms). The stat lines of the engine log are cumulative counters printed by Worker 0 about every 5 s (step function: the last stat line at or before each instant). The window's increase of each counter for each Worker is printed, with the wait lines of Worker 0 (gilwait, w=0) that fall into the window (count and total ms). The window is the time the clicks were in progress; the counters are step-sampled, so the increase is only as exact as the 5 s sampling.
import fs from 'node:fs';
import path from 'node:path';

const argv = process.argv.slice(2);
let outFile = null, dir = null;
for (let i = 0; i < argv.length; i++) { if (argv[i] === '--out') outFile = argv[++i]; else dir = argv[i]; }
if (!dir) { process.stderr.write('usage: node tools/verify-kit/tools/r4-5d-window.mjs RUNDIR [--out FILE.tsv]\n'); process.exit(2); }

const logFile = fs.readdirSync(dir).find((f) => f.endsWith('.log') && f.startsWith('engine'));
const lines = logFile ? fs.readFileSync(path.join(dir, logFile), 'utf8').split('\n') : [];
const stats = [], waits = [];
for (const l of lines) {
  let m;
  if ((m = /R4-5d stat t=([\d.]+) w=(\d+) acq=(\d+) slow=(\d+) slow_ms=([\d.]+) spins=(\d+) sleeps=(\d+) sleep_ms=([\d.]+) slices=(\d+) slice_ms=([\d.]+)/.exec(l)))
    stats.push({ t: +m[1], w: +m[2], acq: +m[3], slow: +m[4], slowMs: +m[5], spins: +m[6], sleeps: +m[7], sleepMs: +m[8], slices: +m[9], sliceMs: +m[10] });
  else if ((m = /R4-5d gilwait w=(\d+) start=([\d.]+) dur=([\d.]+)/.exec(l))) waits.push({ w: +m[1], s: +m[2], d: +m[3] });
}
const ORDER = ['a-idle-menu', 'c-mixer-drag', 'd-space', 'b-play-menu'];
const last = {};
let prev = -1;
for (const x of ORDER) {
  const p = path.join(dir, `lat-${x}.json`);
  if (!fs.existsSync(p)) continue;
  last[x] = prev;
  const recs = JSON.parse(fs.readFileSync(p, 'utf8')).records;
  prev = Math.max(prev, ...recs.map((r) => r.seq));
}
const recs = JSON.parse(fs.readFileSync(path.join(dir, 'lat-b-play-menu.json'), 'utf8')).records
  .filter((r) => (r.kind === 2 || r.kind === 5) && !r.dropped && r.frames && r.seq > (last['b-play-menu'] ?? -1));
const t0 = Math.min(...recs.map((r) => r.tPage));
const t1 = Math.max(...recs.map((r) => Math.max(...Object.values(r.frames))));

const at = (w, t) => { let v = null; for (const s of stats) if (s.w === w && s.t <= t && (v === null || s.t > v.t)) v = s; return v; };
const out = [];
out.push(`# r4-5d-window: ${path.basename(dir)}  playback window ${t0.toFixed(0)} .. ${t1.toFixed(0)} epoch ms (${((t1 - t0) / 1000).toFixed(1)} s, ${recs.length} answered-or-not records)`);
out.push(['w', 'acq', 'slow', 'slow_ms', 'spins', 'sleeps', 'sleep_ms', 'slices', 'slice_ms', 'stat_t0', 'stat_t1'].join('\t'));
for (let w = 0; w < 8; w++) {
  const a = at(w, t0), b = at(w, t1);
  if (!a || !b) { out.push(`${w}\t-`); continue; }
  const d = (k) => (b[k] - a[k]);
  out.push([w, d('acq'), d('slow'), (b.slowMs - a.slowMs).toFixed(1), d('spins'), d('sleeps'), (b.sleepMs - a.sleepMs).toFixed(1), d('slices'), (b.sliceMs - a.sliceMs).toFixed(1), a.t.toFixed(0), b.t.toFixed(0)].join('\t'));
}
const w0 = waits.filter((x) => x.w === 0 && x.s >= t0 && x.s <= t1);
out.push(`# worker 0 waits >= 0.5 ms in the window: n=${w0.length} total_ms=${w0.reduce((a, x) => a + x.d, 0).toFixed(2)} max_ms=${w0.reduce((a, x) => Math.max(a, x.d), 0).toFixed(2)}`);
const all0 = waits.filter((x) => x.w === 0);
out.push(`# worker 0 waits >= 0.5 ms in the whole run: n=${all0.length} total_ms=${all0.reduce((a, x) => a + x.d, 0).toFixed(2)}`);
const text = out.join('\n') + '\n';
if (outFile) fs.writeFileSync(outFile, text);
process.stdout.write(text);
