// R4-5d: compare the Export WAVs of the runs of one or more bundles, sample by sample (verification only; reads files, writes nothing unless --out is given). Usage: node tools/verify-kit/tools/r4-5d-wavdiff.mjs [--out FILE.tsv] LABEL=PATH [LABEL=PATH ...]. Prints, per file, the byte size and the sha256 of the whole file, and per pair (every pair of the given files): the differing frames (a frame differs when one of its channels differs), the largest absolute difference (in LSB of the 16-bit samples), the number of differing samples, the first and the last differing frame in seconds, the number of runs of consecutive differing frames (a run continues over a gap of 0 frames), the longest run in frames, and the first five runs (start in s, length in frames, largest difference in the run). The format must be 16-bit PCM (the WAV written by the Export). The table is read by people; the numbers are the facts.
import fs from 'node:fs';
import crypto from 'node:crypto';

const argv = process.argv.slice(2);
let outFile = null;
const items = [];
for (let i = 0; i < argv.length; i++) {
  if (argv[i] === '--out') { outFile = argv[++i]; continue; }
  const m = /^([^=]+)=(.+)$/.exec(argv[i]);
  if (!m) { process.stderr.write(`r4-5d-wavdiff: bad argument ${argv[i]} (want LABEL=PATH)\n`); process.exit(2); }
  items.push({ label: m[1], path: m[2] });
}
if (items.length < 2) { process.stderr.write('usage: node tools/verify-kit/tools/r4-5d-wavdiff.mjs [--out FILE.tsv] LABEL=PATH LABEL=PATH [...]\n'); process.exit(2); }

function readWav(path) {
  const buf = fs.readFileSync(path);
  if (buf.toString('ascii', 0, 4) !== 'RIFF' || buf.toString('ascii', 8, 12) !== 'WAVE') throw new Error(`${path}: not a RIFF/WAVE file`);
  let p = 12, fmt = null, data = null;
  while (p + 8 <= buf.length) {
    const id = buf.toString('ascii', p, p + 4);
    const size = buf.readUInt32LE(p + 4);
    const body = p + 8;
    if (id === 'fmt ') fmt = { format: buf.readUInt16LE(body), channels: buf.readUInt16LE(body + 2), rate: buf.readUInt32LE(body + 4), bits: buf.readUInt16LE(body + 14) };
    if (id === 'data') { data = { off: body, size: Math.min(size, buf.length - body) }; break; }
    p = body + size + (size & 1);
  }
  if (!fmt || !data) throw new Error(`${path}: no fmt or data chunk`);
  if (fmt.bits !== 16) throw new Error(`${path}: ${fmt.bits}-bit samples; only 16-bit PCM is handled here`);
  const frames = Math.floor(data.size / (2 * fmt.channels));
  const samples = new Int16Array(buf.buffer.slice(buf.byteOffset + data.off, buf.byteOffset + data.off + frames * fmt.channels * 2));
  return { buf, fmt, frames, samples, bytes: buf.length, sha: crypto.createHash('sha256').update(buf).digest('hex') };
}

function compare(a, b) {
  const ch = a.fmt.channels, n = Math.min(a.frames, b.frames), rate = a.fmt.rate;
  let diffFrames = 0, diffSamples = 0, maxAbs = 0, first = -1, last = -1;
  const hist = { 1: 0, 2: 0, 3: 0, more: 0 };
  const runs = [];
  let cur = null;
  for (let f = 0; f < n; f++) {
    let fdiff = 0, fmax = 0;
    for (let c = 0; c < ch; c++) {
      const i = f * ch + c;
      const d = Math.abs(a.samples[i] - b.samples[i]);
      if (d) {
        fdiff++; diffSamples++;
        if (d > fmax) fmax = d;
        if (d === 1) hist[1]++; else if (d === 2) hist[2]++; else if (d === 3) hist[3]++; else hist.more++;
      }
    }
    if (fdiff) {
      diffFrames++;
      if (first < 0) first = f;
      last = f;
      if (fmax > maxAbs) maxAbs = fmax;
      if (cur && f === cur.end + 1) { cur.end = f; cur.len++; if (fmax > cur.max) cur.max = fmax; }
      else { if (cur) runs.push(cur); cur = { start: f, end: f, len: 1, max: fmax }; }
    }
  }
  if (cur) runs.push(cur);
  const longest = runs.reduce((m, r) => (r.len > m.len ? r : m), { len: 0, start: 0, max: 0 });
  return {
    frames: n, sameLength: a.frames === b.frames, diffFrames, diffSamples, maxAbs, hist,
    firstS: first < 0 ? null : first / rate, lastS: last < 0 ? null : last / rate,
    runs: runs.length, longestLen: longest.len, longestStartS: longest.len ? longest.start / rate : null,
    topRuns: runs.slice().sort((x, y) => y.len - x.len).slice(0, 5).map((r) => ({ startS: +(r.start / rate).toFixed(3), len: r.len, max: r.max })),
  };
}

const files = items.map((it) => ({ ...it, w: readWav(it.path) }));
const rows = [];
const fmt = (x, d = 3) => (x === null || x === undefined ? '-' : typeof x === 'number' ? (Number.isInteger(x) ? String(x) : x.toFixed(d)) : String(x));
const header = ['A', 'B', 'bytes_A', 'bytes_B', 'same_sha', 'frames', 'diff_frames', 'diff_samples', 'max_abs_LSB', 'hist_1_2_3_more', 'first_s', 'last_s', 'runs', 'longest_run_frames', 'longest_run_start_s', 'top5_runs(start_s:len:max)'];
rows.push(header.join('\t'));
const out = [];
out.push('# r4-5d-wavdiff: the Export WAVs, sample by sample (16-bit PCM). Read-only.');
for (const f of files) out.push(`${f.label}\t${f.path.split('/').slice(-3).join('/')}\tbytes ${f.w.bytes}\tsha256 ${f.w.sha}\trate ${f.w.fmt.rate}\tch ${f.w.fmt.channels}\tframes ${f.w.frames}`);
out.push(header.join('\t'));
for (let i = 0; i < files.length; i++) for (let j = i + 1; j < files.length; j++) {
  const A = files[i], B = files[j];
  const r = compare(A.w, B.w);
  const line = [A.label, B.label, A.w.bytes, B.w.bytes, A.w.sha === B.w.sha ? 'yes' : 'no', r.frames, r.diffFrames, r.diffSamples, r.maxAbs, `${r.hist[1]}_${r.hist[2]}_${r.hist[3]}_${r.hist.more}`, fmt(r.firstS), fmt(r.lastS), r.runs, r.longestLen, fmt(r.longestStartS), r.topRuns.map((t) => `${t.startS}:${t.len}:${t.max}`).join(' ')];
  out.push(line.join('\t'));
  rows.push(line.join('\t'));
}
const text = out.join('\n') + '\n';
if (outFile) fs.writeFileSync(outFile, text);
process.stdout.write(text);
