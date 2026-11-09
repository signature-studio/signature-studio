// chrome-count.mjs OUTFILE [interval ms]: every interval, one line "epoch_ms<TAB>others<TAB>mine<TAB>total" = the number of Chrome browser processes (those with --remote-debugging-port, i.e. one per automated browser) that are not ci1's (their --user-data-dir is not under lift/out-ci1/kit/), ci1's, and the total. For the overlap column of the pure/lat tables (PM 14:58): a run with 2 or more other Chrome instances is a reference (overlap) run.
import fs from 'node:fs';
const out = process.argv[2], iv = Number(process.argv[3] ?? 2000);
function count() {
  let others = 0, mine = 0;
  for (const d of fs.readdirSync('/proc')) {
    if (!/^\d+$/.test(d)) continue;
    let c; try { c = fs.readFileSync(`/proc/${d}/cmdline`, 'utf8'); } catch { continue; }
    if (!c.includes('--remote-debugging-port') || c.includes('--type=')) continue;
    if (!c.includes('/chrome')) continue;
    if (c.includes('/lift/out-ci1/kit/')) mine++; else others++;
  }
  return [others, mine];
}
for (;;) { const [o, m] = count(); fs.appendFileSync(out, `${Date.now()}\t${o}\t${m}\t${o + m}\n`); await new Promise((r) => setTimeout(r, iv)); }
