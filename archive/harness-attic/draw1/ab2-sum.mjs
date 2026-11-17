// draw1: summary of tools/ab.sh runs made with --open-ms 9000 --mid-ms 4000 --close-ms 4000: per variant (median over all reps of all rounds, load given):
//  first 4 s after F8: Worker CPU s, F8-window changed draws (frames of the animation); the following 5 s (the F8 window is static): Worker cores (CPU s / wall s), draws per second of all windows (identical repaints) and of the F8 window; close phase 4 s: Worker CPU s; idle background cores of the same session.
import fs from 'node:fs';
const dir = process.argv[2];
const med = (a) => { const s = a.filter((x) => Number.isFinite(x)).sort((x, y) => x - y); return s.length ? (s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2) : NaN; };
const rows = {};
for (const d of fs.readdirSync(dir).sort()) {
  const f = `${dir}/${d}/result.json`; if (!fs.existsSync(f)) continue;
  const r = JSON.parse(fs.readFileSync(f)); const v = d.replace(/-r\d+$/, '');
  const idleCores = r.idle ? r.idle.workerCpuS / r.idle.wallS : NaN;
  for (const rep of r.reps_ ?? []) {
    const tl = rep.timelineOpen;
    const f8first = tl.filter((x) => x[4] === 1920 && x[6] > 0 && x[0] < 4000).length;
    const f8rest = tl.filter((x) => x[4] === 1920 && x[0] >= 4000).length;
    const allRest = tl.filter((x) => x[0] >= 4000).length;
    const lastChange = Math.max(0, ...tl.filter((x) => x[4] === 1920 && x[6] > 0).map((x) => x[0]));
    (rows[v] ??= []).push({ first: rep.openFirst.workerCpuS, rest: rep.openRest.workerCpuS / rep.openRest.wallS, f8first, f8restPerS: f8rest / rep.openRest.wallS, allRestPerS: allRest / rep.openRest.wallS, close: rep.close.workerCpuS, closeWall: rep.close.wallS, lastChange, load: rep.load0, idle: idleCores });
  }
}
const f = (x, n = 2) => (Number.isFinite(x) ? x.toFixed(n) : '-');
console.log('variant | n | 0-4 s Worker CPU s | F8 changed frames in 0-4 s | last F8 change at ms | static phase 4-9 s: Worker cores | F8 repaints/s | all-window draws/s | close 4 s Worker CPU s | idle bg cores | load0 med/max');
for (const [v, a] of Object.entries(rows)) console.log(`${v} | ${a.length} | ${f(med(a.map((x) => x.first)))} | ${f(med(a.map((x) => x.f8first)), 0)} | ${f(med(a.map((x) => x.lastChange)), 0)} | ${f(med(a.map((x) => x.rest)))} | ${f(med(a.map((x) => x.f8restPerS)), 1)} | ${f(med(a.map((x) => x.allRestPerS)), 1)} | ${f(med(a.map((x) => x.close)))} | ${f(med(a.map((x) => x.idle)))} | ${f(med(a.map((x) => x.load)), 1)}/${f(Math.max(...a.map((x) => x.load)), 1)}`);
