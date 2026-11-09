// draw1: summary of tools/ab.sh runs: per variant, the Worker CPU of the F8 open phase (6 s window) and of the close phase, the F8 window's (lid with 1920x1080 draws) changed draws (= frames) and the CPU per frame, median over reps, with the load.
import fs from 'node:fs';
const dir = process.argv[2];
const med = (a) => { const s = [...a].sort((x, y) => x - y); return s.length ? (s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2) : NaN; };
const rows = {};
for (const d of fs.readdirSync(dir).sort()) {
  const f = `${dir}/${d}/result.json`; if (!fs.existsSync(f)) continue;
  const r = JSON.parse(fs.readFileSync(f)); const v = d.replace(/-r\d+$/, '');
  for (const rep of r.reps_ ?? []) {
    const lids = Object.entries(rep.open.perWindow ?? {});
    const f8 = lids.find(([, e]) => e.maxW === 1920 && e.maxH === 1080)?.[1];
    const mn = lids.find(([, e]) => e.maxW === 1930)?.[1];
    (rows[v] ??= []).push({ run: d, openCpu: rep.open.workerCpuS, closeCpu: rep.close.workerCpuS, f8Frames: f8?.changedDraws ?? 0, mainDraws: mn?.draws ?? 0, openDraws: rep.open.draws, load: rep.load0, idleCpu: r.idle?.workerCpuS });
  }
}
console.log('variant | n | open CPU s (median, min-max) | close CPU s (median) | F8 frames in 6 s (median) | open CPU per F8 frame ms (median) | load0 (median, max)');
for (const [v, a] of Object.entries(rows)) {
  const per = a.filter((x) => x.f8Frames > 0).map((x) => 1000 * x.openCpu / x.f8Frames);
  console.log(`${v} | ${a.length} | ${med(a.map((x) => x.openCpu)).toFixed(2)} (${Math.min(...a.map((x) => x.openCpu)).toFixed(2)}-${Math.max(...a.map((x) => x.openCpu)).toFixed(2)}) | ${med(a.map((x) => x.closeCpu)).toFixed(2)} | ${med(a.map((x) => x.f8Frames))} | ${med(per).toFixed(0)} | ${med(a.map((x) => x.load)).toFixed(1)}, ${Math.max(...a.map((x) => x.load)).toFixed(1)}`);
}
