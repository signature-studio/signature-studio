// node run-pipe.mjs cases="par:1:0,par:4:1,chain:4:1" blocks=150 reps=2 tag=pipe   (case = topo:K:paced)
import fs from 'node:fs';
import path from 'node:path';
import { withBrowser, threadCpu, cpuDiff, argv, OUT, sleep } from './lib.mjs';
const cases = (argv.cases || 'par:1:0').split(',').map((s) => s.split(':')), blocks = +(argv.blocks || 150), reps = +(argv.reps || 1), tag = argv.tag || 'pipe', mod = argv.mod || 'mx9';
const rows = [];
for (let rep = 0; rep < reps; rep++) for (const [topo, k, paced] of cases) {
  const name = `${tag}-${mod}-${topo}-k${k}-p${paced}-r${rep}`;
  const o = await withBrowser({ name, page: `/coi/pipe.html?topo=${topo}&k=${k}&paced=${paced}&blocks=${blocks}&mod=${mod}`, timeoutS: 200 }, async ({ evalJSON, waitDone, chrome, out }) => {
    for (let i = 0; i < 2000; i++) { if (await evalJSON('window.__ready === true')) break; const er = await evalJSON('window.__error || null'); if (er) throw new Error('page: ' + er); await sleep(100); }
    const before = threadCpu(chrome.proc.pid);
    await evalJSON('window.__go = true; true');
    const r = await waitDone(200);
    const after = threadCpu(chrome.proc.pid);
    await evalJSON('window.__terminate = true; true');
    out.cpu = cpuDiff(before, after).filter((c) => /Worker/.test(c.name)).map((c) => c.ms);
    return r;
  });
  if (o.error) { console.error(name, 'ERROR', o.error.slice(0, 300)); continue; }
  const r = o.result;
  const row = { name, topo, k: +k, paced: +paced, blocks, load_start: o.loadavg_before.split(' ')[0], load_end: o.loadavg_after.split(' ')[0], hc: r.hardwareConcurrency, periodMs: +r.period.toFixed(2), wallMs: Math.round(r.wall),
    lat_p50: +r.lat.p50.toFixed(2), lat_p90: +r.lat.p90.toFixed(2), lat_p99: +r.lat.p99.toFixed(2), lat_max: +r.lat.max.toFixed(2), first: r.latFirst.map((x) => +x.toFixed(1)), start_late_p99: +r.late.p99.toFixed(2),
    missesSteady: r.missesSteady, missesAll: r.missesAll, over50pct: r.over50pct, pluginsOK: r.plugins.every((p) => p.rc === 0 && !p.err && p.done === blocks), workerCpuMs: o.cpu };
  rows.push(row); console.log(JSON.stringify(row));
}
fs.mkdirSync(path.join(OUT, 'results'), { recursive: true });
fs.writeFileSync(path.join(OUT, 'results', `${tag}-${mod}-summary.json`), JSON.stringify(rows, null, 1));
