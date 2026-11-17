// srm1 (decision 0177): node run-serum.mjs mod=O2 n=1 state=3 blocks=40 reps=1 seed=small tag=name [notes=60,64,67,72] [procOnly=1]
// For each repetition: a fresh headless Chrome (Chrome 156), N Workers on ONE compiled WebAssembly.Module of the Serum 2 harness, a barrier, the harness main() in all Workers at once.
// CPU per Worker thread is read from /proc (utime+stime, 10 ms ticks) before/after the release of the barrier; the per-block times are the harness' own (in a Worker they are wall
// time: performance.now(), see lift/harness/vst3a/wasm_cpuclock.c); RSS of the Chrome process tree is read from /proc. Same method as mt1's run-scale.mjs (decision 0173).
import fs from 'node:fs';
import path from 'node:path';
import { withBrowser, threadCpu, cpuDiff, argv, OUT, sleep, pct } from './lib.mjs';
import { descendants } from '../cdp.mjs';
const mod = argv.mod || 'O2', state = argv.states ?? argv.state ?? '3', n = argv.states ? argv.states.split(',').length : +(argv.n ?? 1), blocks = +(argv.blocks || 40), reps = +(argv.reps || 1), tag = argv.tag || 'srm1', seed = argv.seed || 'small';
const rss = (root) => { let r = 0; for (const p of descendants(root)) { try { r += +fs.readFileSync(`/proc/${p}/status`, 'utf8').match(/VmRSS:\s+(\d+)/)[1]; } catch {} } return r / 1024; };
const pss = (root) => { let r = 0; for (const p of descendants(root)) { try { r += +fs.readFileSync(`/proc/${p}/smaps_rollup`, 'utf8').match(/^Pss:\s+(\d+)/m)[1]; } catch {} } return r / 1024; };
const peakRss = (root) => { let r = 0; for (const p of descendants(root)) { try { r += +fs.readFileSync(`/proc/${p}/status`, 'utf8').match(/VmHWM:\s+(\d+)/)[1]; } catch {} } return r / 1024; };
const rows = [];
for (let rep = 0; rep < reps; rep++) {
  const name = `${tag}-${mod}-s${String(state).replace(/,/g, '')}-n${n}-r${rep}`;
  const load0 = fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0];
  const q = `mod=${mod}` + (argv.states ? `&states=${argv.states}` : `&n=${n}&state=${state}`) + `&blocks=${blocks}&seed=${seed}` + (argv.notes ? `&notes=${argv.notes}` : '') + (argv.procOnly ? '&procOnly=1' : '');
  const o = await withBrowser({ name, page: `/coi/serum.html?${q}`, timeoutS: 1800, args: argv.jsflags ? [`--js-flags=${argv.jsflags}`] : [] }, async ({ evalJSON, waitDone, chrome, out }) => {
    for (let i = 0; i < 20000; i++) { if (await evalJSON('window.__ready === true')) break; const er = await evalJSON('window.__error || null'); if (er) throw new Error('page: ' + er); await sleep(100); }
    const rss0 = rss(chrome.proc.pid); out.pssMB_before_run = pss(chrome.proc.pid);
    const before = threadCpu(chrome.proc.pid);
    await evalJSON('window.__go = true; true');
    const r = await waitDone(1800);
    const after = threadCpu(chrome.proc.pid);
    out.rssMB_before_run = rss0; out.rssMB_after_run = rss(chrome.proc.pid); out.pssMB_after_run = pss(chrome.proc.pid); out.rssMB_peak_hwm = peakRss(chrome.proc.pid);
    await evalJSON('window.__terminate = true; true');
    out.cpu = cpuDiff(before, after).slice(0, n + 6);
    return r;
  });
  if (o.error) { console.error(name, 'ERROR', o.error.slice(0, 600)); continue; }
  const R = o.result.results;
  const blk = R.map((x) => {
    const b = [];
    for (const l of x.lines) { const m = /^vst: block (\d+)\s+cpu ([\d.]+) ms\s+wall ([\d.]+) ms/.exec(l); if (m) b[+m[1]] = +m[3]; }
    return b;
  });
  const per = blk.map((b) => pct(b.slice(1).filter((v) => v !== undefined)));
  const res = R.map((x) => (x.lines.find((l) => l.startsWith('RESULT blocks')) || '').slice(0, 300));
  const hash = R.map((x) => (x.lines.find((l) => l.startsWith('RESULT out_hash')) || '').slice(0, 40));
  const stage = R.map((x) => x.lines.filter((l) => /LoadLibraryW|factory\.\.initialize|setState|DllMain\.\.setActive|setupProcessing|setActive/.test(l)).map((l) => l.slice(0, 200)));
  const workerCpu = o.cpu.filter((c) => /Worker/i.test(c.name));
  const row = { name, mod, n, state, blocks, load_start: load0, load_end: o.loadavg_after.split(' ')[0], hc: o.result.hardwareConcurrency, wasmBytes: o.result.wasmBytes, contentBytes: o.result.contentBytes,
    fetchMs: Math.round(o.result.tFetch), compileMs: Math.round(o.result.tCompile), instMs: o.result.instMs.map(Math.round), wallMs: Math.round(o.result.wallMs), runMs: R.map((x) => Math.round(x.runMs)),
    perBlockMin: per.map((p) => p.min), perBlockP50: per.map((p) => p.p50), perBlockMax: per.map((p) => p.max), block0: blk.map((b) => b[0]), hash, hashesEqual: new Set(hash).size === 1,
    memBytesEnd: R.map((x) => x.memBytes), rssMB_before_run: Math.round(o.rssMB_before_run), rssMB_after_run: Math.round(o.rssMB_after_run), rssMB_peak_hwm: Math.round(o.rssMB_peak_hwm), pssMB_before_run: Math.round(o.pssMB_before_run), pssMB_after_run: Math.round(o.pssMB_after_run),
    workerCpuMs: workerCpu.map((c) => c.ms), err: R.map((x) => x.err), missing: R.map((x) => x.missing), result: res, stage };
  rows.push(row);
  console.log(JSON.stringify(row));
}
fs.mkdirSync(path.join(OUT, 'results'), { recursive: true });
fs.writeFileSync(path.join(OUT, 'results', `${tag}-${mod}-s${state}-n${n}-summary.json`), JSON.stringify(rows, null, 1));
