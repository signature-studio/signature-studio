// Counting check for the boot window (#1334): reads the TSV of measure.mjs --trace (one row per 50 ms: the page's underrun and playing quanta, the engine's WR and RD in frames, its publish count and maxPubGapMs) and prints what the two counts measure.
// Usage: node tools/p3-audio-probe/trace-stats.mjs <trace.tsv> > stats.json
// The output (AudioWorklet) quantum is 128 frames, an underrun is a quantum that starts with fewer than 128 frames ahead of RD (fle-out.worklet.js), so "fill" = WR - RD. The predicted starvation of a publish streak (no publish between two samples) is the streak minus the time the output takes to read the frames above 128 (at the RD rate measured over the streak).
// Resolution: the samples are 50 ms apart, so streak and run lengths are good to about 50 ms at each end. The engine's own maxPubGapMs is exact (the last row).
import fs from 'node:fs';

const path = process.argv[2];
if (!path) {
  process.stderr.write('usage: node tools/p3-audio-probe/trace-stats.mjs <trace.tsv>\n');
  process.exit(2);
}
const QUANTUM = 128;
const rows = fs.readFileSync(path, 'utf8').split('\n').slice(1).filter((l) => l.length > 0).map((l) => {
  const [t, ur, play, ctx, pub, wr, rd, gap] = l.split('\t');
  return { t: Number(t), ur: Number(ur), play: Number(play), ctx, pub: Number(pub), wr: Number(wr), rd: Number(rd), gap: gap === '' ? null : Number(gap), fill: Number(wr) - Number(rd) };
}).filter((r) => Number.isFinite(r.t) && Number.isFinite(r.ur));
if (rows.length < 2) {
  process.stderr.write('trace: fewer than two rows\n');
  process.exit(2);
}
const rate = (a, b) => (b.rd - a.rd) / Math.max(1, b.t - a.t);

// the runs of consecutive intervals (i, i+1) that satisfy pred; returns the longest by ms
const longestRun = (pred) => {
  let best = { ms: 0, start: 0, end: 0, quanta: 0, intervals: 0 };
  let cur = null;
  for (let i = 0; i + 1 < rows.length; i++) {
    if (pred(i)) {
      if (!cur) cur = { start: i, end: i, quanta: 0, intervals: 0 };
      cur.end = i + 1;
      cur.quanta += rows[i + 1].ur - rows[i].ur;
      cur.intervals++;
    } else if (cur) {
      cur.ms = rows[cur.end].t - rows[cur.start].t;
      if (cur.ms > best.ms) best = cur;
      cur = null;
    }
  }
  if (cur) {
    cur.ms = rows[cur.end].t - rows[cur.start].t;
    if (cur.ms > best.ms) best = cur;
  }
  return best;
};

const urRun = longestRun((i) => rows[i + 1].ur > rows[i].ur);
const pubStreak = longestRun((i) => rows[i + 1].pub === rows[i].pub);
const fillLow = longestRun((i) => rows[i].fill < QUANTUM && rows[i + 1].fill < QUANTUM);

let pred = { ms: 0, start: 0 };
for (let i = 0; i + 1 < rows.length; i++) {
  if (rows[i + 1].pub !== rows[i].pub) continue;
  let j = i + 1;
  while (j + 1 < rows.length && rows[j + 1].pub === rows[i].pub) j++;
  const span = rows[j].t - rows[i].t;
  const r = rate(rows[i], rows[j]);
  if (span > 0 && r > 0) {
    const starve = span - Math.max(0, rows[i].fill - QUANTUM) / r;
    if (starve > pred.ms) pred = { ms: starve, start: rows[i].t, span, fill: rows[i].fill, rate: r };
  }
  i = j - 1;
}

const lowSamples = rows.filter((r) => r.fill < QUANTUM).length;
const fills = rows.map((r) => r.fill).sort((a, b) => a - b);
const r0 = rows[0];
const rl = rows[rows.length - 1];
const publishes = rl.pub - r0.pub;
process.stdout.write(JSON.stringify({
  file: path,
  rows: rows.length,
  spanMs: rl.t - r0.t,
  playingRows: rows.filter((r) => r.ctx === 'running').length,
  underrunQuantaDelta: rl.ur - r0.ur,
  playingQuantaDelta: rl.play - r0.play,
  publishesDelta: publishes,
  engineMaxPubGapMs: rl.gap,
  longestUnderrunRun: { ms: urRun.ms, startMs: rows[urRun.start].t, quanta: urRun.quanta, intervals: urRun.intervals },
  longestPublishStreak: { ms: pubStreak.ms, startMs: rows[pubStreak.start].t, fillAtStartFrames: rows[pubStreak.start].fill, rdFramesInStreak: rows[pubStreak.end].rd - rows[pubStreak.start].rd },
  longestPredictedStarvation: { ms: Math.round(pred.ms * 10) / 10, startMs: pred.start === undefined ? null : pred.start, streakMs: pred.span, fillAtStartFrames: pred.fill ?? null, rdRateFramesPerMs: pred.rate === undefined ? null : Math.round(pred.rate * 1000) / 1000 },
  fillBelowQuantumSamples: lowSamples,
  longestFillBelowQuantumMs: fillLow.ms,
  fillFrames: { min: fills[0], median: fills[Math.floor(fills.length / 2)], max: fills[fills.length - 1] },
}, null, 1) + '\n');
