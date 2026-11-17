// R4-5d: the medians over the runs of each group (verification only). Reads a TSV written by tools/verify-kit/tools/r4-5d-segments.mjs (--out) and prints, for the section b-play-menu (playback) and each input, the median over the runs of each column, and the range. The groups are taken from the run folder names: co-1 (r42c-cop2r*-chrome-1), co-2 (r42c-cop2r*-chrome-2), rt1 (r42c-rt1p2r*), rt (r42c-rtp2r*). Usage: node tools/verify-kit/tools/r4-5d-groups.mjs SEGMENTS.tsv
import fs from 'node:fs';

const file = process.argv[2];
if (!file) { process.stderr.write('usage: node tools/verify-kit/tools/r4-5d-groups.mjs SEGMENTS.tsv\n'); process.exit(2); }
const rows = fs.readFileSync(file, 'utf8').split('\n').filter((l) => l && !l.startsWith('#')).map((l) => l.split('\t'));
const head = rows.shift();
const col = (name) => head.indexOf(name);
const groupOf = (run) => (/-cop2r\d+-chrome-1$/.test(run) ? 'co-1' : /-cop2r\d+-chrome-2$/.test(run) ? 'co-2' : /-rt1p2r\d+/.test(run) ? 'rt1' : /-rtp2r\d+/.test(run) ? 'rt' : null);
const med = (v) => { const s = [...v].sort((a, b) => a - b); return s[Math.floor((s.length - 1) / 2)]; };
const cols = ['T_med', 'page_med', 'queue_med', 'engine_med', 'present_med', 'engine_p90'];
const out = [['section', 'input', 'group', 'runs', ...cols.map((c) => `${c}(median; min-max over runs)`)].join('\t')];
const keys = new Map();
for (const r of rows) {
  const g = groupOf(r[col('run')]);
  if (!g) continue;
  const k = `${r[col('section')]}\t${r[col('input')]}\t${g}`;
  if (!keys.has(k)) keys.set(k, []);
  keys.get(k).push(r);
}
for (const [k, list] of keys) {
  const [section, input, g] = k.split('\t');
  const cells = cols.map((c) => {
    const v = list.map((r) => Number(r[col(c)]));
    return `${med(v).toFixed(2)} (${Math.min(...v).toFixed(2)}-${Math.max(...v).toFixed(2)})`;
  });
  out.push([section, input, g, list.length, ...cells].join('\t'));
}
process.stdout.write(out.join('\n') + '\n');
