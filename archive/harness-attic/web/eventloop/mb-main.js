'use strict';
// M-B page side. ?tests=env,rtt,bench  ?n=10000
const Q = new URLSearchParams(location.search);
const N = +(Q.get('n') || 10000);
const WANT = (Q.get('tests') || 'env,rtt,bench').split(',');
const OPTS = (Q.get('opts') || 'O2').split(',');
const log = (s) => { (window.__log = window.__log || []).push(s); };

function summarize(arr, qs) {
  qs = qs || [50, 95, 99];
  const a = Float64Array.from(arr).sort();
  const n = a.length;
  const o = { n, min: a[0], max: a[n - 1], mean: a.reduce((s, x) => s + x, 0) / n };
  for (const q of qs) o['p' + q] = a[Math.min(n - 1, Math.floor((q / 100) * n))];
  return o;
}

async function rtt(variant, depth, n) {
  const w = new Worker('mb-worker.js');
  let pending = null;
  let ready;
  const rd = new Promise((r) => { ready = r; });
  const fin = new Promise((res, rej) => {
    w.onmessage = (e) => {
      const d = e.data;
      if (d && d.type === 'ready') ready();
      else if (d && d.type === 'result') res(d.result);
      else if (typeof d === 'number' && pending && pending.k === d) pending.res();
    };
    w.onerror = (e) => rej(new Error('worker error: ' + e.message));
  });
  w.postMessage({ variant, op: 'echo', depth });
  await Promise.race([rd, fin.then((r) => { throw new Error('worker ended: ' + JSON.stringify(r)); })]);
  const one = (k) => new Promise((res) => { pending = { k, res }; w.postMessage({ type: 1, a: k, b: 0, t: 0 }); });
  for (let k = 1; k <= 200; k++) await one(1000000 + k);
  const v = new Array(n);
  for (let k = 1; k <= n; k++) {
    const t0 = performance.now();
    await one(k);
    v[k - 1] = (performance.now() - t0) * 1000;
  }
  w.postMessage({ type: 9, a: 0, b: 0, t: 0 });
  const r = await fin;
  w.terminate();
  return { variant, depth, unit: 'us', rtt: summarize(v), worker: r };
}

async function bench(variant, cfg) {   // also used for op:'depth'
  const w = new Worker('mb-worker.js');
  const fin = new Promise((res, rej) => {
    w.onmessage = (e) => { if (e.data.type === 'result') res(e.data.result); };
    w.onerror = (e) => rej(new Error('worker error: ' + e.message));
  });
  w.postMessage(Object.assign({ variant, op: 'bench' }, cfg));
  const r = await fin;
  w.terminate();
  return r;
}

async function main() {
  const R = { env: {
    crossOriginIsolated: self.crossOriginIsolated, SharedArrayBuffer: typeof SharedArrayBuffer,
    JSPI: { Suspending: typeof (self.WebAssembly && WebAssembly.Suspending), promising: typeof (self.WebAssembly && WebAssembly.promising) },
    userAgent: navigator.userAgent,
  } };
  const modes = (Q.get('modes') || 'asyncify,jspi').split(',');
  if (WANT.includes('rtt')) {
    R.rtt = [];
    for (const opt of OPTS) for (const mode of modes) for (const depth of [0, 40]) {
      R.rtt.push(Object.assign(await rtt(`mb_${mode}_${opt}`, depth, N), { opt, mode }));
      log(`rtt ${mode} ${opt} depth ${depth} done`);
    }
  }
  if (WANT.includes('bench')) {
    R.bench = {};
    for (const opt of OPTS) for (const mode of ['sync', ...modes]) {
      const cfg = mode === 'sync'
        ? { depths: [0, 10, 40, 160], kinds: [0], wides: [0, 1], nNop: 200000, nMicro: 0, nTask: 0 }
        : { depths: [0, 10, 40, 160], kinds: [0, 1, 2], wides: [0, 1], nNop: 200000, nMicro: 20000, nTask: 3000 };
      R.bench[`${mode}_${opt}`] = await bench(`mb_${mode}_${opt}`, cfg);
      log(`bench ${mode} ${opt} done`);
    }
  }
  if (WANT.includes('depth')) {
    R.depth = {};
    for (const opt of OPTS) for (const mode of ['sync', ...modes]) {
      R.depth[`${mode}_${opt}`] = await bench(`mb_${mode}_${opt}`, { op: 'depth' }).catch((e) => ({ error: String(e) }));
      log(`depth ${mode} ${opt} done`);
    }
  }
  window.__result = R;
  window.__done = true;
}
main().catch((e) => { window.__error = String(e && e.stack || e); });
