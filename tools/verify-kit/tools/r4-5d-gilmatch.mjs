// R4-5d: match the lock waits of the diagnostic build (branch r4-5d-rt, the lines "R4-5d gilwait", "R4-5d slice", "R4-5d stat" of the engine log) with the input records of one run (lat-b-play-menu.json: tPage, tListener, tInject, tIdle, frame). Verification only; reads files, writes nothing unless --out is given. Usage: node tools/verify-kit/tools/r4-5d-gilmatch.mjs RUNDIR [--out FILE.tsv]. RUNDIR holds lat-b-play-menu.json and the log saved by saveLog (*.log). For each answered File click and Esc of the playback section it prints, in ms: T (frame - tPage), page (tListener - tPage), queue (tInject - tListener), engine (tIdle - tInject), and the time Worker 0 spent waiting for the giant lock (gilwait lines with w=0) that overlaps the interval queue (tListener..tInject), engine (tInject..tIdle) and the whole T (tPage..frame), and the time the other Workers spent in slices (slice lines, w != 0) that overlap the engine interval. The last lines give the cumulative counters of the final "stat" line of each Worker. All times are epoch ms (the page and the Worker both use performance.timeOrigin + performance.now(); the log uses Date.now()).
import fs from 'node:fs';
import path from 'node:path';

const argv = process.argv.slice(2);
let outFile = null;
let dir = null;
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === '--out') outFile = argv[++i];
  else dir = argv[i];
}
if (!dir) { process.stderr.write('usage: node tools/verify-kit/tools/r4-5d-gilmatch.mjs RUNDIR [--out FILE.tsv]\n'); process.exit(2); }

const logFile = fs.readdirSync(dir).find((f) => f.endsWith('.log') && f.startsWith('engine'));
const logLines = logFile ? fs.readFileSync(path.join(dir, logFile), 'utf8').split('\n') : [];
const waits = [], slices = [], stats = [];
for (const l of logLines) {
  let m;
  if ((m = /R4-5d gilwait w=(\d+) start=([\d.]+) dur=([\d.]+) sleep=([\d.]+) sleeps=(\d+) spins=(\d+) holder=(\d+)/.exec(l))) waits.push({ w: +m[1], s: +m[2], d: +m[3], sleep: +m[4], sleeps: +m[5], spins: +m[6], holder: +m[7] });
  else if ((m = /R4-5d slice w=(\d+) start=([\d.]+) dur=([\d.]+)/.exec(l))) slices.push({ w: +m[1], s: +m[2], d: +m[3] });
  else if ((m = /R4-5d stat t=([\d.]+) w=(\d+) acq=(\d+) slow=(\d+) slow_ms=([\d.]+) spins=(\d+) sleeps=(\d+) sleep_ms=([\d.]+) slices=(\d+) slice_ms=([\d.]+)/.exec(l)))
    stats.push({ t: +m[1], w: +m[2], acq: +m[3], slow: +m[4], slowMs: +m[5], spins: +m[6], sleeps: +m[7], sleepMs: +m[8], slices: +m[9], sliceMs: +m[10] });
}
const lat0 = JSON.parse(fs.readFileSync(path.join(dir, 'lat-b-play-menu.json'), 'utf8')).records;
// the same filter as lat-ab.py load(): only the records after the last seq of the sections before the playback section
const before = ['a-idle-menu', 'c-mixer-drag', 'd-space'].map((x) => path.join(dir, `lat-${x}.json`)).filter((p) => fs.existsSync(p));
const lastBefore = before.length ? Math.max(...before.map((p) => Math.max(...JSON.parse(fs.readFileSync(p, 'utf8')).records.map((r) => r.seq)))) : -1;
const lat = lat0.filter((r) => r.seq > lastBefore);

const overlap = (a0, a1, b0, b1) => Math.max(0, Math.min(a1, b1) - Math.max(a0, b0));
const sumOverlap = (list, t0, t1, pred) => list.reduce((acc, x) => (pred(x) ? acc + overlap(x.s, x.s + x.d, t0, t1) : acc), 0);
const med = (v) => { if (!v.length) return null; const s = [...v].sort((a, b) => a - b); return s[Math.floor((s.length - 1) / 2)]; };
const f = (x) => (x === null || x === undefined ? '-' : x.toFixed(2));

const header = ['kind', 'seq', 'T', 'page', 'queue', 'engine', 'w0_wait_in_queue', 'w0_wait_in_engine', 'w0_wait_in_T', 'other_slices_in_engine'];
const out = [header.join('\t')];
const rows = [];
for (const r of lat) {
  if (r.kind !== 2 && r.kind !== 5) continue;
  if (r.dropped || !r.frames || r.tIdle == null || r.tInject == null || r.tListener == null) continue;
  const ids = Object.keys(r.frames).map(Number).sort((a, b) => a - b);
  if (!ids.length) continue;
  const frame = r.frames[String(ids[0])];
  const T = frame - r.tPage;
  const q = [r.tListener, r.tInject], e = [r.tInject, r.tIdle], all = [r.tPage, frame];
  const row = {
    kind: r.kind === 2 ? 'File' : 'Esc', seq: r.seq, T, page: r.tListener - r.tPage, queue: r.tInject - r.tListener, engine: r.tIdle - r.tInject,
    w0q: sumOverlap(waits, q[0], q[1], (x) => x.w === 0), w0e: sumOverlap(waits, e[0], e[1], (x) => x.w === 0), w0T: sumOverlap(waits, all[0], all[1], (x) => x.w === 0),
    osl: sumOverlap(slices, e[0], e[1], (x) => x.w !== 0),
  };
  rows.push(row);
  out.push([row.kind, row.seq, f(row.T), f(row.page), f(row.queue), f(row.engine), f(row.w0q), f(row.w0e), f(row.w0T), f(row.osl)].join('\t'));
}
const summary = [];
for (const k of ['File', 'Esc']) {
  const rs = rows.filter((x) => x.kind === k);
  summary.push(`# ${k}: n=${rs.length} T_med=${f(med(rs.map((x) => x.T)))} queue_med=${f(med(rs.map((x) => x.queue)))} engine_med=${f(med(rs.map((x) => x.engine)))} w0_wait_in_engine_med=${f(med(rs.map((x) => x.w0e)))} w0_wait_in_engine_mean=${f(rs.length ? rs.reduce((a, x) => a + x.w0e, 0) / rs.length : null)} other_slices_in_engine_mean=${f(rs.length ? rs.reduce((a, x) => a + x.osl, 0) / rs.length : null)}`);
}
out.push(...summary);
out.push(`# log: ${logFile ?? 'none'} gilwait lines=${waits.length} (w0=${waits.filter((x) => x.w === 0).length}) slice lines=${slices.length} stat lines=${stats.length}`);
const lastStat = {};
for (const s of stats) lastStat[s.w] = s;
for (const w of Object.keys(lastStat).sort()) {
  const s = lastStat[w];
  out.push(`# stat w=${w} acq=${s.acq} slow=${s.slow} slow_ms=${s.slowMs.toFixed(1)} spins=${s.spins} sleeps=${s.sleeps} sleep_ms=${s.sleepMs.toFixed(1)} slices=${s.slices} slice_ms=${s.sliceMs.toFixed(1)} (t=${s.t.toFixed(0)})`);
}
const text = out.join('\n') + '\n';
if (outFile) fs.writeFileSync(outFile, text);
process.stdout.write(text);
