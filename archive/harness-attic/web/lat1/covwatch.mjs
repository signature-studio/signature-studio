// lat1: the fork-probe binary's children all write the same cov.txt at their exit (the per-probe file of decision 0184 is not in this binary), so each child's file is copied to cov.txt.<probe> as soon as the parent prints
// "g3-probe: <probe> exit 0" (the parent prints it right after the child ended; the next child needs at least a second to finish). usage: node tools/covwatch.mjs RUNDIR
import fs from 'node:fs';
import path from 'node:path';
const dir = process.argv[2];
const stdout = path.join(dir, 'stdout.txt');
const seen = new Set();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const t0 = Date.now();
let copied = 0;
while (Date.now() - t0 < 3 * 3600 * 1000) {
  let txt = '';
  try { txt = fs.readFileSync(stdout, 'latin1'); } catch { /* not yet */ }
  for (const m of txt.matchAll(/^g3-probe: (\S+) exit (\d+)/gm)) {
    if (seen.has(m[1])) continue;
    seen.add(m[1]);
    try { fs.copyFileSync(path.join(dir, 'cov.txt'), path.join(dir, `cov.txt.${m[1]}`)); copied++; } catch (e) { console.log('copy failed', m[1], String(e)); }
  }
  if (/^g3-snap: #\d+ end /m.test(txt) || /\nrc=/.test(fs.existsSync(path.join(dir, 'load.txt')) ? fs.readFileSync(path.join(dir, 'load.txt'), 'utf8') : '')) { console.log('done, copied', copied); break; }
  await sleep(100);
}
