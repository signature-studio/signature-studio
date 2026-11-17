// R4-5d: split the input latency T of the recorded runs into its segments (verification only; reads the lat JSON files, writes nothing unless --out is given). Usage: node tools/verify-kit/tools/r4-5d-segments.mjs [--out FILE.tsv] DIR... where DIR is a folder of lat-*.json files (one run each, e.g. evidence/R4-5c/p2/lat/r42c-rtp2r3-chrome-1). For each answered record of a section (a-idle-menu, b-play-menu) and each input (kind 2 = File click, kind 5 = Esc) it computes, in ms: T = frame - tPage (the definition of lat-ab.py: the first frame of the record), and its segments: page = tListener - tPage (the page handled the event), queue = tInject - tListener (the page gave it to the engine), engine = tIdle - tInject (the engine was idle again after it), present = frame - tIdle (from the engine being idle to the presented frame). The records that were dropped or have no frame are not counted. Per run: the median and the 90th percentile of each segment. The sum page + queue + engine + present = T exactly.
import fs from 'node:fs';
import path from 'node:path';

const argv = process.argv.slice(2);
let outFile = null;
const dirs = [];
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === '--out') { outFile = argv[++i]; continue; }
  dirs.push(argv[i]);
}
if (!dirs.length) { process.stderr.write('usage: node tools/verify-kit/tools/r4-5d-segments.mjs [--out FILE.tsv] DIR...\n'); process.exit(2); }

const med = (v) => { if (!v.length) return null; const s = [...v].sort((a, b) => a - b); return s[Math.floor((s.length - 1) / 2)]; };
const p90 = (v) => { if (!v.length) return null; const s = [...v].sort((a, b) => a - b); return s[Math.min(s.length - 1, Math.max(0, Math.ceil(0.9 * s.length) - 1))]; };
const f = (x) => (x === null || x === undefined ? '-' : x.toFixed(2));

const header = ['run', 'section', 'input', 'answered', 'T_med', 'T_p90', 'page_med', 'queue_med', 'engine_med', 'present_med', 'engine_p90', 'present_p90'];
const lines = [header.join('\t')];
const out = [];
out.push('# r4-5d-segments: input latency T split into page / queue / engine / present (ms; medians and 90th percentiles over the answered records of one run)');
out.push(header.join('\t'));
// the same filter as tools/verify-kit/tools/lat-ab.py load(): a dump holds the last 400 inputs, so a section counts only the records after the last seq of the sections before it
const ORDER = ['a-idle-menu', 'c-mixer-drag', 'd-space', 'b-play-menu'];
function lastSeqBefore(dir) {
  const last = {};
  let prev = -1;
  for (const x of ORDER) {
    const p = path.join(dir, `lat-${x}.json`);
    if (!fs.existsSync(p)) continue;
    last[x] = prev;
    const recs = JSON.parse(fs.readFileSync(p, 'utf8')).records;
    prev = Math.max(prev, ...recs.map((r) => r.seq));
  }
  return last;
}
for (const dir of dirs) {
  const run = path.basename(dir);
  const last = lastSeqBefore(dir);
  for (const section of ['a-idle-menu', 'b-play-menu']) {
    const file = path.join(dir, `lat-${section}.json`);
    if (!fs.existsSync(file)) continue;
    const records = JSON.parse(fs.readFileSync(file, 'utf8')).records;
    for (const [kind, name] of [[2, 'File click'], [5, 'Esc']]) {
      const seg = { T: [], page: [], queue: [], engine: [], present: [] };
      for (const r of records) {
        if (r.kind !== kind || r.dropped || !r.frames || r.seq <= (last[section] ?? -1)) continue;
        const ids = Object.keys(r.frames);
        if (!ids.length || r.tIdle == null || r.tInject == null || r.tListener == null) continue;
        const lid = ids.map(Number).sort((a, b) => a - b)[0];
        const frame = r.frames[String(lid)];
        if (frame == null) continue;
        seg.T.push(frame - r.tPage);
        seg.page.push(r.tListener - r.tPage);
        seg.queue.push(r.tInject - r.tListener);
        seg.engine.push(r.tIdle - r.tInject);
        seg.present.push(frame - r.tIdle);
      }
      const row = [run, section, name, seg.T.length, f(med(seg.T)), f(p90(seg.T)), f(med(seg.page)), f(med(seg.queue)), f(med(seg.engine)), f(med(seg.present)), f(p90(seg.engine)), f(p90(seg.present))];
      out.push(row.join('\t'));
      lines.push(row.join('\t'));
    }
  }
}
const text = out.join('\n') + '\n';
if (outFile) fs.writeFileSync(outFile, text);
process.stdout.write(text);
