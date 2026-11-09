'use strict';
// Feature tests for the browser port of flengine6.wasm. Runs in the page (scope 'window') and, started as a Worker, in a dedicated worker
// (scope 'worker'); the page collects both into window.__result. Nothing here plays sound: AudioWorklet is tried on an OfflineAudioContext
// (renders to a buffer), an AudioContext is created and closed without resume().
const PROBES = {
  simd: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x01,0x05,0x01,0x60,0x00,0x01,0x7b,0x03,0x02,0x01,0x00,0x0a,0x16,0x01,0x14,0x00,0xfd,0x0c,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0b],
  threads: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x01,0x05,0x01,0x60,0x00,0x01,0x7f,0x03,0x02,0x01,0x00,0x05,0x04,0x01,0x03,0x01,0x01,0x0a,0x0c,0x01,0x0a,0x00,0x41,0x00,0x41,0x01,0xfe,0x1e,0x02,0x00,0x0b],
  bulk: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x01,0x04,0x01,0x60,0x00,0x00,0x03,0x02,0x01,0x00,0x05,0x03,0x01,0x00,0x01,0x0a,0x0d,0x01,0x0b,0x00,0x41,0x00,0x41,0x00,0x41,0x00,0xfc,0x0b,0x00,0x0b],
  exc_legacy: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x01,0x04,0x01,0x60,0x00,0x00,0x03,0x02,0x01,0x00,0x0d,0x03,0x01,0x00,0x00,0x0a,0x08,0x01,0x06,0x00,0x06,0x40,0x19,0x0b,0x0b],
  memory64: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x05,0x03,0x01,0x04,0x01],
  tailcall: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x01,0x04,0x01,0x60,0x00,0x00,0x03,0x02,0x01,0x00,0x0a,0x06,0x01,0x04,0x00,0x12,0x00,0x0b],
  multivalue: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x01,0x06,0x01,0x60,0x00,0x02,0x7f,0x7f,0x03,0x02,0x01,0x00,0x0a,0x08,0x01,0x06,0x00,0x41,0x00,0x41,0x01,0x0b],
  reftypes: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x01,0x06,0x01,0x60,0x01,0x6f,0x01,0x6f,0x03,0x02,0x01,0x00,0x0a,0x06,0x01,0x04,0x00,0x20,0x00,0x0b],
  extconst: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x06,0x09,0x01,0x7f,0x00,0x41,0x01,0x41,0x02,0x6a,0x0b],
  multimem: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x05,0x05,0x02,0x00,0x01,0x00,0x01],
  relaxed: [0x00,0x61,0x73,0x6d,0x01,0x00,0x00,0x00,0x01,0x05,0x01,0x60,0x00,0x01,0x7b,0x03,0x02,0x01,0x00,0x0a,0x3d,0x01,0x3b,0x00,0xfd,0x0c,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0xfd,0x0c,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0xfd,0x0c,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0xfd,0x8b,0x02,0x0b],
};

async function features(scope) {
  const r = { scope, userAgent: navigator.userAgent, crossOriginIsolated: self.crossOriginIsolated, secureContext: self.isSecureContext,
    hardwareConcurrency: navigator.hardwareConcurrency, deviceMemory: navigator.deviceMemory };
  const tryit = async (f) => { try { return await f(); } catch (e) { return 'ERROR ' + (e && e.name) + ': ' + String(e && e.message).slice(0, 160); } };

  r.SharedArrayBuffer = { typeof: typeof SharedArrayBuffer, construct: await tryit(() => { const b = new SharedArrayBuffer(16); return b.byteLength; }) };
  r.Atomics = { typeof: typeof Atomics, wait_exists: typeof Atomics !== 'undefined' && typeof Atomics.wait, waitAsync: typeof Atomics !== 'undefined' && typeof Atomics.waitAsync };
  r.sharedWasmMemory = await tryit(() => { const m = new WebAssembly.Memory({ initial: 1, maximum: 2, shared: true }); return m.buffer instanceof SharedArrayBuffer; });

  r.wasm = { validate: {} };
  for (const [k, bytes] of Object.entries(PROBES)) r.wasm.validate[k] = await tryit(() => WebAssembly.validate(new Uint8Array(bytes)));
  r.wasm.compileStreaming = typeof WebAssembly.compileStreaming;
  r.wasm.instantiateStreaming = typeof WebAssembly.instantiateStreaming;
  r.wasm.memory = {};
  for (const pages of [16384, 32768, 65536]) {
    r.wasm.memory['initial ' + pages * 64 / 1024 + ' MiB, max 4096 MiB'] = await tryit(() => { const m = new WebAssembly.Memory({ initial: pages, maximum: 65536 }); const n = m.buffer.byteLength; return n; });
  }
  r.wasm.memory['maximum 65537 pages'] = await tryit(() => new WebAssembly.Memory({ initial: 1, maximum: 65537 }) && 'accepted');
  const localsModule = (n) => new Uint8Array([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 1, 4, 1, 0x60, 0, 0, 3, 2, 1, 0, 0x0a, 8, 1, 6, 1, 0x80 | (n & 0x7f), 0x80 | ((n >> 7) & 0x7f), n >> 14, 0x7f, 0x0b]);
  r.wasm.maxLocals = { '50000 locals validates': WebAssembly.validate(localsModule(50000)), '50001 locals validates': WebAssembly.validate(localsModule(50001)) };
  r.wasm.memory64 = await tryit(() => { const m = new WebAssembly.Memory({ initial: 1, maximum: 2, address: 'i64' }); return m.buffer.byteLength; });
  r.wasm.jspi = { Suspending: typeof WebAssembly.Suspending, promising: typeof WebAssembly.promising };

  r.OffscreenCanvas = { typeof: typeof OffscreenCanvas };
  if (typeof OffscreenCanvas !== 'undefined') {
    r.OffscreenCanvas.ctx2d = await tryit(() => { const c = new OffscreenCanvas(16, 16); const x = c.getContext('2d'); x.fillStyle = '#ff0000'; x.fillRect(0, 0, 16, 16); const d = x.getImageData(0, 0, 1, 1).data; return Array.from(d).join(','); });
    r.OffscreenCanvas.webgl2 = await tryit(() => { const c = new OffscreenCanvas(16, 16); const g = c.getContext('webgl2'); if (!g) return null; const dbg = g.getExtension('WEBGL_debug_renderer_info'); return { version: g.getParameter(g.VERSION), renderer: dbg ? g.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : null }; });
    r.OffscreenCanvas.webgl1 = await tryit(() => !!new OffscreenCanvas(16, 16).getContext('webgl'));
    r.OffscreenCanvas.convertToBlob = await tryit(async () => { const c = new OffscreenCanvas(8, 8); c.getContext('2d').fillRect(0, 0, 8, 8); return (await c.convertToBlob()).size > 0; });
  }
  if (scope === 'window') r.OffscreenCanvas.transferControlToOffscreen = typeof HTMLCanvasElement.prototype.transferControlToOffscreen;
  r.webgpu = { navigator_gpu: typeof navigator.gpu };
  if (navigator.gpu) r.webgpu.adapter = await tryit(async () => { const a = await navigator.gpu.requestAdapter(); return a ? 'adapter ok' : null; });

  r.OPFS = { getDirectory: typeof (navigator.storage && navigator.storage.getDirectory) };
  r.OPFS.roundtrip = await tryit(async () => {
    const root = await navigator.storage.getDirectory();
    const fh = await root.getFileHandle('probe.bin', { create: true });
    const out = { createWritable: typeof fh.createWritable, createSyncAccessHandle: typeof fh.createSyncAccessHandle };
    if (typeof fh.createWritable === 'function') {
      const w = await fh.createWritable(); await w.write(new Uint8Array([1, 2, 3, 4, 5])); await w.close();
      out.writable_read = (await (await fh.getFile()).arrayBuffer()).byteLength;
    }
    if (typeof fh.createSyncAccessHandle === 'function' && scope === 'worker') {
      const h = await fh.createSyncAccessHandle();
      h.truncate(0); const n = h.write(new Uint8Array(1 << 20), { at: 0 }); h.flush(); out.sync_write_bytes = n; out.sync_size = h.getSize(); h.close();
    }
    await root.removeEntry('probe.bin');
    return out;
  });
  r.storage = await tryit(async () => { const e = await navigator.storage.estimate(); return { quotaMB: Math.round(e.quota / 1048576), usageMB: Math.round(e.usage / 1048576) }; });
  r.persist = await tryit(() => navigator.storage.persisted());
  r.IndexedDB = { typeof: typeof indexedDB, cacheStorage: typeof caches };
  r.IndexedDB.roundtrip = await tryit(() => new Promise((res, rej) => {
    const q = indexedDB.open('probe', 1);
    q.onupgradeneeded = () => q.result.createObjectStore('s');
    q.onsuccess = () => { const db = q.result; const tx = db.transaction('s', 'readwrite'); tx.objectStore('s').put(new Uint8Array(1 << 20), 'k'); tx.oncomplete = () => { db.close(); indexedDB.deleteDatabase('probe'); res('1 MiB stored'); }; tx.onerror = () => rej(tx.error); };
    q.onerror = () => rej(q.error);
  }));
  r.IndexedDB.storeWasmModule = await tryit(() => new Promise((res) => {
    const m = new WebAssembly.Module(new Uint8Array(PROBES.multivalue));
    const q = indexedDB.open('probe2', 1);
    q.onupgradeneeded = () => q.result.createObjectStore('s');
    q.onsuccess = () => { const db = q.result; try { const tx = db.transaction('s', 'readwrite'); tx.objectStore('s').put(m, 'm'); tx.oncomplete = () => { db.close(); indexedDB.deleteDatabase('probe2'); res('stored'); }; tx.onerror = () => { db.close(); indexedDB.deleteDatabase('probe2'); res('ERROR ' + (tx.error && tx.error.name)); }; } catch (e) { db.close(); indexedDB.deleteDatabase('probe2'); res('ERROR ' + e.name); } };
  }));
  r.streams = { CompressionStream: typeof CompressionStream, DecompressionStream: typeof DecompressionStream };
  for (const f of ['gzip', 'deflate', 'brotli', 'zstd']) r.streams['Decompression ' + f] = await tryit(() => { new DecompressionStream(f); return 'constructible'; });

  if (scope === 'window') {
    r.audio = { AudioContext: typeof AudioContext, OfflineAudioContext: typeof OfflineAudioContext, AudioWorkletNode: typeof AudioWorkletNode };
    r.audio.audioContext = await tryit(async () => { const c = new AudioContext(); const o = { state: c.state, sampleRate: c.sampleRate, baseLatency: c.baseLatency, hasAudioWorklet: !!c.audioWorklet }; await c.close(); return o; });
    r.audio.offlineWorklet = await tryit(async () => {
      const code = "class P extends AudioWorkletProcessor{process(i,o){o[0][0].fill(0.5);return true}} registerProcessor('probe',P)";
      const url = URL.createObjectURL(new Blob([code], { type: 'text/javascript' }));
      const c = new OfflineAudioContext(1, 512, 48000);
      await c.audioWorklet.addModule(url);
      const n = new AudioWorkletNode(c, 'probe'); n.connect(c.destination);
      const buf = await c.startRendering();
      return { rendered_first_samples: Array.from(buf.getChannelData(0).slice(0, 3)), length: buf.length };
    });
    r.audio.sharedMemoryToWorklet = await tryit(async () => {
      const c = new OfflineAudioContext(1, 128, 48000);
      const code = "class P extends AudioWorkletProcessor{constructor(o){super();this.v=new Int32Array(o.processorOptions.sab);}process(i,o){Atomics.add(this.v,0,1);return true}} registerProcessor('sab',P)";
      await c.audioWorklet.addModule(URL.createObjectURL(new Blob([code], { type: 'text/javascript' })));
      const sab = new SharedArrayBuffer(4);
      const n = new AudioWorkletNode(c, 'sab', { processorOptions: { sab } }); n.connect(c.destination);
      await c.startRendering();
      return { process_calls_seen_through_SAB: Atomics.load(new Int32Array(sab), 0) };
    });
    r.worker = await new Promise((res) => {
      const w = new Worker('features.js');
      const t = setTimeout(() => res('TIMEOUT'), 30000);
      w.onmessage = (e) => { clearTimeout(t); res(e.data); w.terminate(); };
      w.onerror = (e) => { clearTimeout(t); res('ERROR ' + e.message); };
    });
  } else {
    r.audio = { AudioContext: typeof AudioContext, OfflineAudioContext: typeof OfflineAudioContext, AudioWorkletNode: typeof AudioWorkletNode };
    // cost of one emscripten_return_address (new Error().stack, split, index) at a call depth like the one of the a4 run (12 frames)
    r.returnAddressCost = (() => {
      const jsStackTrace = () => new Error().stack.toString();
      const ra = (level) => { const cs = jsStackTrace().split('\n'); if (cs[0] === 'Error') cs.shift(); return cs[level + 3]; };
      const deep = (n, f) => (n === 0 ? f() : deep(n - 1, f) + 0);
      const N = 20000;
      const t0 = performance.now();
      let x = 0;
      for (let i = 0; i < N; i++) x += deep(12, () => (ra(0) ? 1 : 0));
      const us = (performance.now() - t0) * 1000 / N;
      return { calls: N, microseconds_per_call: +us.toFixed(2), frames_parsed: 'depth 12 + 3', note: 'micro-benchmark in JS, not the wasm' };
    })();
  }
  return r;
}

if (typeof WorkerGlobalScope !== 'undefined' && self instanceof WorkerGlobalScope) {
  features('worker').then((r) => self.postMessage(r), (e) => self.postMessage({ error: String(e) }));
}
