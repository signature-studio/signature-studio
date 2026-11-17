// draw1: summary of tools/g3cpu.sh runs: user CPU seconds of the f8 script and of the control script per variant, median and range; F8 phase = median(f8) - median(control).
import fs from 'node:fs';
const dirs = process.argv.slice(2);
const rows = {};
for (const dir of dirs) for (const f of fs.readdirSync(dir)) {
  const m = /^(\w+)-(f8-control|f8)-r(\d+)\.txt$/.exec(f); if (!m) continue;
  const t = fs.readFileSync(`${dir}/${f}`, 'utf8'); const u = /User time \(seconds\): ([\d.]+)/.exec(t); if (!u) continue;
  (rows[m[1]] ??= { f8: [], ctl: [] })[m[2] === 'f8' ? 'f8' : 'ctl'].push(+u[1]);
}
const med = (a) => { const s = [...a].sort((x, y) => x - y); return s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2; };
console.log('variant | n(f8/ctl) | f8 script user s: median (min-max) | control: median (min-max) | F8 phase = median(f8) - median(control) | ratio to base');
let base = null;
for (const v of Object.keys(rows)) {
  const r = rows[v]; const d = med(r.f8) - med(r.ctl); if (v === 'F1xHn') base = d;
  console.log(`${v} | ${r.f8.length}/${r.ctl.length} | ${med(r.f8).toFixed(1)} (${Math.min(...r.f8).toFixed(1)}-${Math.max(...r.f8).toFixed(1)}) | ${med(r.ctl).toFixed(1)} (${Math.min(...r.ctl).toFixed(1)}-${Math.max(...r.ctl).toFixed(1)}) | ${d.toFixed(1)} | ${base ? (d / base).toFixed(2) : '-'}`);
}
