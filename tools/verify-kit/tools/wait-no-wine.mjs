// wait-no-wine.mjs [MAXMIN]: exits 0 when no FL64.exe / wineserver / wine process runs (opt1's Wine original FL takes 2-3 cores), or 1 after MAXMIN minutes (the run then starts as a reference). Prints one line.
import fs from 'node:fs';
const maxMin = Number(process.argv[2] ?? 15), t0 = Date.now();
function wine() {
  const n = [];
  for (const d of fs.readdirSync('/proc')) {
    if (!/^\d+$/.test(d)) continue;
    let c; try { c = fs.readFileSync(`/proc/${d}/cmdline`, 'utf8'); } catch { continue; }
    if (/FL64\.exe|wineserver|wine64|wine-preloader/.test(c)) n.push(d);
  }
  return n;
}
for (;;) {
  const w = wine();
  if (w.length === 0) { console.log(`no Wine process after ${Math.round((Date.now() - t0) / 1000)} s`); process.exit(0); }
  if ((Date.now() - t0) / 60000 > maxMin) { console.log(`Wine still running after ${maxMin} min (${w.length} processes): the run is a reference`); process.exit(1); }
  await new Promise((r) => setTimeout(r, 5000));
}
