// node run-vfs.mjs cases="shared:4,private:4,shared:1,private:1" mb=105 reps=2 tag=vfs ; RSS of the whole Chrome process tree (all Workers are threads of one renderer) before / after the Workers ran.
import fs from 'node:fs';
import path from 'node:path';
import { withBrowser, argv, OUT, sleep, threadCpu, cpuDiff } from './lib.mjs';
import { descendants } from '../cdp.mjs';
const cases = (argv.cases || 'shared:4,private:4').split(',').map((s) => s.split(':')), mb = +(argv.mb || 105), reps = +(argv.reps || 2), tag = argv.tag || 'vfs';
const rss = (root) => { let r = 0; for (const p of descendants(root)) { try { r += +fs.readFileSync(`/proc/${p}/status`, 'utf8').match(/VmRSS:\s+(\d+)/)[1]; } catch {} } return r / 1024; };
const rows = [];
for (let rep = 0; rep < reps; rep++) for (const [mode, n] of cases) {
  const name = `${tag}-${mode}-n${n}-r${rep}`;
  const o = await withBrowser({ name, page: `/coi/vfs.html?mode=${mode}&n=${n}&mb=${mb}`, timeoutS: 120 }, async ({ evalJSON, waitDone, chrome, out }) => {
    for (let i = 0; i < 600; i++) { if (await evalJSON('window.__ready === true')) break; const er = await evalJSON('window.__error || null'); if (er) throw new Error(er); await sleep(100); }
    out.rss_before = rss(chrome.proc.pid);
    const before = threadCpu(chrome.proc.pid);
    await evalJSON('window.__go = true; true');
    const r = await waitDone(120);
    out.rss_after = rss(chrome.proc.pid);
    const after = threadCpu(chrome.proc.pid);
    out.cpu = cpuDiff(before, after).filter((c) => /Worker/.test(c.name)).map((c) => c.ms);
    await evalJSON('window.__terminate = true; true');
    return r;
  });
  if (o.error) { console.error(name, 'ERROR', o.error.slice(0, 300)); continue; }
  const r = o.result;
  const row = { name, mode, n: +n, mb, load: o.loadavg_before.split(' ')[0], tAllocMs: Math.round(r.tAlloc), wallMs: Math.round(r.wall), rss_before_MB: Math.round(o.rss_before), rss_after_MB: Math.round(o.rss_after), rss_delta_MB: Math.round(o.rss_after - o.rss_before),
    tCopyMs_median: +r.workers.map((w) => w.tCopy).sort((a, b) => a - b)[Math.floor(r.workers.length / 2)].toFixed(1), tReadMs_median: +r.workers.map((w) => w.tRead).sort((a, b) => a - b)[Math.floor(r.workers.length / 2)].toFixed(1), MBps_median: +r.workers.map((w) => w.mbps).sort((a, b) => a - b)[Math.floor(r.workers.length / 2)].toFixed(0), workerCpuMs: o.cpu };
  rows.push(row); console.log(JSON.stringify(row));
}
fs.mkdirSync(path.join(OUT, 'results'), { recursive: true });
fs.writeFileSync(path.join(OUT, 'results', `${tag}-summary.json`), JSON.stringify(rows, null, 1));
