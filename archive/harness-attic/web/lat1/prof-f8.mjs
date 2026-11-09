// lat1: where does the engine Worker spend its time while the F8 plug-in list (full-screen window) opens and closes? V8 CPU profile (CDP Profiler) of the Worker over one F8 -> 6 s -> Escape -> 3 s, default entry (SHIPH).
// Self time is summed by kind: wasm (the lifted engine and runtime), JavaScript (the Worker's own code: pixel conversion, postMessage, the idle driver), (program), (idle), (garbage collector).
import fs from 'node:fs';
import { openServer, openSession, sleep, say, ROOT } from './lib.mjs';
const out = `${ROOT}/lift/out-lat1/runs/prof-f8`;
fs.mkdirSync(out, { recursive: true });
const srv = await openServer(out, 4500);
const S = await openSession(out, 'proff8');
const workers = [];
S.browser.cdp.on((m) => { if (m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') workers.push(m.params.sessionId); });
await S.page.S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
try {
  await S.open(`http://127.0.0.1:${srv.port}/${process.argv[2] ?? ''}`);
  await S.waitIdle(); await S.refreshRect(); await S.closeWizard(); await sleep(4000);
  if (!workers.length) throw new Error('no worker target attached');
  const W = (method, params) => S.browser.cdp.send(method, params, workers[0]);
  await W('Profiler.enable', {}); await W('Profiler.setSamplingInterval', { interval: 1000 });
  await W('Profiler.start', {});
  const t0 = Date.now();
  await S.tap('F8', 150); await sleep(6000);
  const tMid = Date.now();
  await S.tap('Escape', 150); await sleep(3000);
  const p = (await W('Profiler.stop', {})).profile;
  const byId = new Map(p.nodes.map((n) => [n.id, n]));
  const dt = p.timeDeltas; const self = new Map();
  for (let i = 0; i < p.samples.length; i++) self.set(p.samples[i], (self.get(p.samples[i]) ?? 0) + (dt[i] ?? 0));
  const kinds = {}; const fns = {};
  for (const [id, us] of self) {
    const n = byId.get(id); const cf = n.callFrame;
    const kind = cf.functionName.startsWith('(') ? cf.functionName : /wasm/.test(cf.url + cf.functionName) || (cf.url === '' && /^\d+$/.test(cf.functionName.replace(/[^\d]/g, '')) && cf.functionName === '') ? 'wasm' : (cf.url ? 'javascript' : 'other');
    kinds[kind] = (kinds[kind] ?? 0) + us;
    const key = `${kind}:${cf.functionName || '(anonymous)'}:${cf.url.split('/').pop()}:${cf.lineNumber}`;
    fns[key] = (fns[key] ?? 0) + us;
  }
  const total = Object.values(kinds).reduce((a, b) => a + b, 0);
  say(`profile ${(total / 1e6).toFixed(2)} s of samples in ${((Date.now() - t0) / 1000).toFixed(1)} s wall (F8 open ${((tMid - t0) / 1000).toFixed(1)} s, close ${((Date.now() - tMid) / 1000).toFixed(1)} s)`);
  for (const [k, v] of Object.entries(kinds).sort((a, b) => b[1] - a[1])) say(`  ${k}: ${(v / 1e6).toFixed(2)} s (${(100 * v / total).toFixed(1)}%)`);
  for (const [k, v] of Object.entries(fns).sort((a, b) => b[1] - a[1]).slice(0, 14)) say(`    ${(v / 1000).toFixed(0)} ms  ${k}`);
  fs.writeFileSync(`${out}/worker-f8.cpuprofile`, JSON.stringify(p));
  fs.writeFileSync(`${out}/summary.json`, JSON.stringify({ kinds, top: Object.entries(fns).sort((a, b) => b[1] - a[1]).slice(0, 30), total }, null, 1));
} finally { try { await S.page.close(); await S.browser.close(); } catch {} await srv.stop(); }
