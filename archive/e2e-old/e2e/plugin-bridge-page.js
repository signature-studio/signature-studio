// Browser side of test/e2e/plugin-bridge.mjs (served as /coi/page.js).
import { PluginInstance } from '/coi/src/plugins/plugin-client.ts';
const q = new URLSearchParams(location.search);
const MODE = q.get('mode') || 'all';
const PERIOD = (4096 / 44100) * 1000;
const browserWorker = () => {
  const w = new Worker('/coi/src/plugins/plugin-worker.ts', { type: 'module' });
  return { post: (m) => w.postMessage(m), onMessage: (cb) => w.addEventListener('message', (e) => cb(e.data)), onError: (cb) => w.addEventListener('error', (e) => cb(new Error(e.message || 'worker error'))), terminate: () => w.terminate() };
};
const fetchBytes = async (u) => new Uint8Array(await (await fetch(u)).arrayBuffer());
const pct = (a, p) => { const s = [...a].sort((x, y) => x - y); return s[Math.min(s.length - 1, Math.floor((p / 100) * s.length))]; };
function drive(sabs, opts) {
  return new Promise((resolve, reject) => {
    const w = new Worker('/coi/src/plugins/driver-worker.ts', { type: 'module' });
    w.onmessage = (e) => { w.terminate(); e.data.type === 'driven' ? resolve(e.data.res) : reject(new Error(e.data.error)); };
    w.onerror = (e) => reject(new Error(e.message));
    w.postMessage({ type: 'drive', sabs, frames: 4096, topology: 'fanout', ...opts });
  });
}
(async () => {
  try {
    const t0 = performance.now();
    const [wasm, ott, seed] = await Promise.all(['vst-ext.wasm', 'ott.vst3', 'seed.tsv'].map((f) => fetchBytes('/coi/data/' + f)));
    const module = await WebAssembly.compile(wasm);
    const files = { '/ott.vst3': ott, '/seed.tsv': seed };
    const base = { module, files, pluginPath: '/ott.vst3', maxBlocks: 4096 };
    const start = (extra = {}) => PluginInstance.start(browserWorker, { ...base, ...extra });
    const one = async (opts, extra = {}) => {
      const t = performance.now();
      const p = await start(extra);
      const readyMs = performance.now() - t;
      const res = await drive([p.slot.sab], { blocks: opts.blocks ?? 40, periodMs: 0, ...opts });
      const out = await p.stop();
      return { hash: out.result.out_hash, lat: res.latencyMs, readyMs, rc: out.rc, served: out.served };
    };
    const result = { hardwareConcurrency: navigator.hardwareConcurrency, compileMs: performance.now() - t0 };
    if (MODE === 'all') {
      result.hashes = {};
      const firstBlocks = [];
      for (const [k, opts, extra] of [['default', {}, {}], ['scale05', { scale: 0.5 }, {}], ['silence', { silence: true }, {}], ['paramFirst', {}, { extraArgs: ['--param-first', '0.25'] }]]) {
        const r = await one(opts, extra);
        result.hashes[k] = r.hash;
        firstBlocks.push(r.lat[0]);
      }
      const ps = await Promise.all([0, 1, 2, 3].map(() => start()));
      const f = await drive(ps.map((p) => p.slot.sab), { blocks: 40, periodMs: 0 });
      const outs = await Promise.all(ps.map((p) => p.stop()));
      result.fanout = { hashes: outs.map((o) => o.result.out_hash), firstBlockMs: f.latencyMs[0] };
      firstBlocks.push(f.latencyMs[0]);
      const pp = await Promise.all([0, 1, 2, 3].map(() => start({ preroll: 6, prerollReset: 0 })));
      const blocks = 150;
      const d = await drive(pp.map((p) => p.slot.sab), { blocks, periodMs: PERIOD });
      await Promise.all(pp.map((p) => p.stop()));
      const steady = d.latencyMs.slice(5);
      result.paced = { blocks, periodMs: PERIOD, lateAll: d.late.length, lateAfterWarm: d.late.filter((b) => b >= 5).length, p50: pct(steady, 50), p99: pct(steady, 99), max: Math.max(...steady), first: d.latencyMs.slice(0, 5) };
      result.firstBlocks = firstBlocks;
    } else if (MODE === 'cold') {
      const r = await one({ blocks: 40 });
      Object.assign(result, { firstBlockMs: r.lat[0], secondBlockMs: r.lat[1], readyMs: r.readyMs, hash: r.hash });
    } else if (MODE === 'warm-instance') {
      const w = await one({ blocks: 12 }, { preroll: 0 });   // a throw-away instance of the same compiled module runs a few blocks
      const r = await one({ blocks: 40 });
      Object.assign(result, { firstBlockMs: r.lat[0], secondBlockMs: r.lat[1], readyMs: r.readyMs, hash: r.hash, warmupFirstMs: w.lat[0] });
    } else if (MODE === 'series') {
      // how long does the tier-up take? one instance, unpaced, per-block round trip in windows of 20 blocks
      const r = await one({ blocks: 400 });
      const win = [];
      for (let i = 0; i < r.lat.length; i += 20) { const w = r.lat.slice(i, i + 20); win.push(+pct(w, 50).toFixed(2)); }
      Object.assign(result, { hash: r.hash, firstBlockMs: r.lat[0], windowMedians20: win, first10: r.lat.slice(0, 10).map((x) => +x.toFixed(1)) });
    } else if (MODE === 'preroll8') {
      const r = await one({ blocks: 40 }, { preroll: 8, prerollReset: 0 });
      Object.assign(result, { firstBlockMs: r.lat[0], secondBlockMs: r.lat[1], readyMs: r.readyMs, hash: r.hash });
    }
    window.__result = result;
  } catch (e) { window.__error = String((e && e.stack) || e); }
  window.__done = true;
})();
