// node run-scale.mjs mod=mx9 n=1,2,4,8,16 blocks=200 reps=3 tag=name
// For each N and repetition: a fresh headless Chrome, N Workers on ONE compiled WebAssembly.Module, a barrier, the OTT harness main() in all Workers at once.
// CPU per Worker thread is read from /proc (utime+stime, 10 ms ticks) before/after the release of the barrier; the harness' per-block wall time (performance.now) is taken inside the Worker.
import fs from 'node:fs';
import path from 'node:path';
import { withBrowser, threadCpu, cpuDiff, argv, OUT, sleep, pct } from './lib.mjs';
import { descendants } from '../cdp.mjs';
const mod = argv.mod || 'mx9', ns = (argv.n || '1,2,4,8,16').split(',').map(Number), blocks = +(argv.blocks || 200), reps = +(argv.reps || 3), tag = argv.tag || 'scale';
const rss = (root) => { let r = 0; for (const p of descendants(root)) { try { r += +fs.readFileSync(`/proc/${p}/status`, 'utf8').match(/VmRSS:\s+(\d+)/)[1]; } catch {} } return r / 1024; };
const rows = [];
for (let rep = 0; rep < reps; rep++) for (const n of ns) {
  const name = `${tag}-${mod}-n${n}-r${rep}`;
  const load0 = fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0];
  const o = await withBrowser({ name, page: `/coi/scale.html?n=${n}&mod=${mod}&blocks=${blocks}`, timeoutS: 900, args: argv.jsflags ? [`--js-flags=${argv.jsflags}`] : [] }, async ({ evalJSON, waitDone, chrome, out }) => {
    for (let i = 0; i < 4000; i++) { if (await evalJSON('window.__ready === true')) break; const er = await evalJSON('window.__error || null'); if (er) throw new Error('page: ' + er); await sleep(100); }
    const rss0 = rss(chrome.proc.pid);
    const before = threadCpu(chrome.proc.pid);
    await evalJSON('window.__go = true; true');
    const r = await waitDone(900);
    const after = threadCpu(chrome.proc.pid);
    await evalJSON('window.__terminate = true; true');
    out.rssMB_before = rss0; out.rssMB_after = rss(chrome.proc.pid);
    out.cpu = cpuDiff(before, after).slice(0, n + 6);
    return r;
  });
  if (o.error) { console.error(name, 'ERROR', o.error.slice(0, 300)); continue; }
  const R = o.result.results;
  const med = R.map((x) => { const b = x.blocks.slice(1).filter((v) => v !== undefined); return pct(b); });
  const hash = R.map((x) => (x.lines.find((l) => l.startsWith('RESULT out_hash')) || '').slice(0, 40));
  const workerCpu = o.cpu.filter((c) => /Worker/i.test(c.name));
  const row = { name, mod, n, blocks, load_start: load0, load_end: o.loadavg_after.split(' ')[0], hc: o.result.hardwareConcurrency, compileMs: Math.round(o.result.tCompile), instMsMedian: pct(o.result.instMs).p50, wallMs: Math.round(o.result.wallMs),
    perBlockMedianOfMedians: pct(med.map((m) => m.p50)).p50, perBlockP50min: Math.min(...med.map((m) => m.p50)), perBlockP50max: Math.max(...med.map((m) => m.p50)), perBlockP99worst: Math.max(...med.map((m) => m.p99)), perBlockMax: Math.max(...med.map((m) => m.max)),
    hashesEqual: new Set(hash).size === 1, hash: hash[0], rssMB_before: Math.round(o.rssMB_before), rssMB_after: Math.round(o.rssMB_after), workerCpuMs: workerCpu.map((c) => c.ms), cpuTop: o.cpu.slice(0, n + 3).map((c) => c.name + ':' + c.ms) };
  rows.push(row);
  console.log(JSON.stringify(row));
}
fs.mkdirSync(path.join(OUT, 'results'), { recursive: true });
fs.writeFileSync(path.join(OUT, 'results', `${tag}-${mod}-summary.json`), JSON.stringify(rows, null, 1));
