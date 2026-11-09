// Stage 3 measurement (decision 0216, opfs1): how the guest's files can be OPFS files that a Worker reads and writes SYNCHRONOUSLY (FileSystemSyncAccessHandle), and what each way of opening / enumerating costs.
// Runs in a dedicated Worker; the page (bench.html) posts {cmd, ...} and gets {ok, result}. Nothing here is the engine: it is a stand-in loop that times the browser's own calls.
// Commands: caps, sync (open latency + read/write throughput), lock (what a second opener / the page sees while a handle is open), persist (write, flush, close; read back after a reload), bridge-server (helper side of the SAB bridge),
// bridge-client (blocks with Atomics.wait while the helper does the async work), jspi (WebAssembly.Suspending around a promise: cost per suspension, if the browser has it).
const now = () => performance.now();
const med = (a) => { const s = [...a].sort((x, y) => x - y); return s.length ? s[Math.floor(s.length / 2)] : null; };
const p95 = (a) => { const s = [...a].sort((x, y) => x - y); return s.length ? s[Math.min(s.length - 1, Math.floor(s.length * 0.95))] : null; };
async function dir(path, create = true) { let d = await navigator.storage.getDirectory(); for (const s of path.split('/').filter(Boolean)) d = await d.getDirectoryHandle(s, { create }); return d; }

const C = {
  async caps() {
    return {
      createSyncAccessHandle: typeof FileSystemFileHandle !== 'undefined' && 'createSyncAccessHandle' in FileSystemFileHandle.prototype,
      sharedArrayBuffer: typeof SharedArrayBuffer !== 'undefined', crossOriginIsolated: self.crossOriginIsolated === true, atomicsWait: typeof Atomics !== 'undefined' && typeof Atomics.wait === 'function', atomicsWaitAsync: typeof Atomics !== 'undefined' && typeof Atomics.waitAsync === 'function',
      jspiSuspending: typeof WebAssembly !== 'undefined' && typeof WebAssembly.Suspending === 'function', jspiPromising: typeof WebAssembly !== 'undefined' && typeof WebAssembly.promising === 'function',
      move: typeof FileSystemFileHandle !== 'undefined' && 'move' in FileSystemFileHandle.prototype, userAgent: navigator.userAgent,
    };
  },
  async sync({ files = 100, big = 64, block = 1 }) {
    const d = await dir('bench/sync');
    const open = [], handles = [];
    for (let i = 0; i < files; i++) {
      const t0 = now();
      const fh = await d.getFileHandle('f' + i, { create: true });
      const h = await fh.createSyncAccessHandle();
      open.push(now() - t0);
      handles.push(h);
    }
    for (const h of handles) h.close();
    const reopen = [];
    for (let i = 0; i < files; i++) { const t0 = now(); const h = await (await d.getFileHandle('f' + i)).createSyncAccessHandle(); reopen.push(now() - t0); h.close(); }
    // one big file: sequential write, flush, sequential read
    const fh = await d.getFileHandle('big', { create: true });
    const h = await fh.createSyncAccessHandle();
    h.truncate(0);
    const MB = 1024 * 1024, buf = new Uint8Array(block * MB);
    for (let i = 0; i < buf.length; i += 4096) buf[i] = i >> 12;
    let t0 = now();
    for (let off = 0; off < big * MB; off += buf.length) h.write(buf, { at: off });
    const tw = now() - t0;
    t0 = now(); h.flush(); const tf = now() - t0;
    t0 = now();
    let sum = 0;
    for (let off = 0; off < big * MB; off += buf.length) { const n = h.read(buf, { at: off }); sum += buf[0] + n; }
    const tr = now() - t0;
    // random 4 KiB reads and writes
    const small = new Uint8Array(4096), n = 20000, rr = [], rw = [];
    const size = h.getSize();
    let seed = 12345;
    const rnd = () => { seed = (seed * 1103515245 + 12345) >>> 0; return seed; };
    t0 = now();
    for (let i = 0; i < n; i++) { h.read(small, { at: (rnd() % (size / 4096 | 0)) * 4096 }); }
    const tr4 = now() - t0;
    t0 = now();
    for (let i = 0; i < n; i++) { h.write(small, { at: (rnd() % (size / 4096 | 0)) * 4096 }); }
    const tw4 = now() - t0;
    t0 = now(); for (let i = 0; i < 2000; i++) h.getSize(); const tgs = now() - t0;
    h.close();
    void rr; void rw; void sum;
    return {
      files, openMedianMs: med(open), openP95Ms: p95(open), reopenMedianMs: med(reopen), reopenP95Ms: p95(reopen),
      bigMiB: big, blockMiB: block, seqWriteMiBs: big / (tw / 1000), flushMs: tf, seqReadMiBs: big / (tr / 1000),
      random4KReadUs: (tr4 / n) * 1000, random4KWriteUs: (tw4 / n) * 1000, getSizeUs: (tgs / 2000) * 1000,
    };
  },
  async sabview() {
    // the engine's memory is a SharedArrayBuffer (THREADS=1): can a sync access handle read into and write from a view of one?
    const d = await dir('bench/sabview');
    const h = await (await d.getFileHandle('s', { create: true })).createSyncAccessHandle();
    const sab = new SharedArrayBuffer(1 << 20), v = new Uint8Array(sab);
    for (let i = 0; i < 4096; i++) v[i] = (i * 7) & 255;
    const out = {};
    try { out.writeBytes = h.write(v.subarray(0, 4096), { at: 0 }); h.flush(); } catch (e) { out.writeError = `${e.name}: ${e.message}`.slice(0, 120); }
    v.fill(0);
    try { out.readBytes = h.read(v.subarray(0, 4096), { at: 0 }); out.readBack = v[1] === 7 && v[4095] === ((4095 * 7) & 255); } catch (e) { out.readError = `${e.name}: ${e.message}`.slice(0, 120); }
    // and through a growable / sub-range view at an offset (as HEAPU8.subarray(ptr, ptr + n) is)
    try { const w = new Uint8Array(sab, 8192, 4096); out.readOffsetView = h.read(w, { at: 0 }) === 4096 && w[1] === 7; } catch (e) { out.readOffsetError = `${e.name}: ${e.message}`.slice(0, 120); }
    h.close();
    return out;
  },
  async lock() {
    const d = await dir('bench/lock');
    const out = {};
    const fh = await d.getFileHandle('l', { create: true });
    const h = await fh.createSyncAccessHandle();
    h.write(new TextEncoder().encode('hello'), { at: 0 }); h.flush();
    const tryit = async (name, f) => { try { const r = await f(); out[name] = r === undefined ? 'ok' : r; } catch (e) { out[name] = `${e.name}`; } };
    await tryit('secondSyncHandle', async () => { const h2 = await fh.createSyncAccessHandle(); h2.close(); });
    await tryit('createWritable', async () => { const w = await fh.createWritable(); await w.close(); });
    await tryit('getFile.size', async () => (await fh.getFile()).size);
    await tryit('getFile.text', async () => await (await fh.getFile()).text());
    await tryit('removeEntry', async () => { await d.removeEntry('l'); });
    await tryit('move', async () => { await fh.move('l2'); });
    h.close();
    await tryit('afterClose.removeEntry', async () => { await d.removeEntry('l').catch(async () => d.removeEntry('l2')); });
    return out;
  },
  async persist({ phase }) {
    const d = await dir('bench/persist');
    if (phase === 'write') {
      const fh = await d.getFileHandle('p', { create: true });
      const h = await fh.createSyncAccessHandle();
      h.truncate(0); h.write(new TextEncoder().encode('persisted-by-sync-handle'), { at: 0 }); h.flush(); h.close();
      const fh2 = await d.getFileHandle('p-noclose', { create: true });
      const h2 = await fh2.createSyncAccessHandle();
      h2.truncate(0); h2.write(new TextEncoder().encode('flushed-not-closed'), { at: 0 }); h2.flush(); // NOT closed: the Worker is terminated by the page
      return 'written';
    }
    const o = {};
    for (const n of ['p', 'p-noclose']) { try { o[n] = await (await (await d.getFileHandle(n)).getFile()).text(); } catch (e) { o[n] = e.name; } }
    return o;
  },
  async 'bridge-server'({ sab }) {
    // the helper: waits for requests in the SAB (Atomics.waitAsync where there is one, else a polling Atomics.wait with a timeout) and does the async OPFS work
    const ctl = new Int32Array(sab, 0, 16), data = new Uint8Array(sab, 64);
    const d = await dir('bench/bridge');
    const files = new Map();
    for (;;) {
      // ctl[0]: 0 idle, 1 request posted; ctl[1]: op; ctl[2]: arg; ctl[3]: result; ctl[4]: length
      await (Atomics.waitAsync ? Atomics.waitAsync(ctl, 0, 0).value : new Promise((r) => setTimeout(r, 0)));
      if (Atomics.load(ctl, 0) !== 1) continue;
      const op = ctl[1], arg = ctl[2];
      try {
        if (op === 1) { const fh = await d.getFileHandle('b' + arg, { create: true }); const h = await fh.createSyncAccessHandle(); files.set(arg, h); ctl[3] = arg; }
        else if (op === 2) { const h = files.get(arg); const n = h.read(data.subarray(0, 4096), { at: 0 }); ctl[3] = n; }
        else if (op === 3) { for (const h of files.values()) h.close(); files.clear(); ctl[3] = 0; }
        else if (op === 9) { ctl[3] = 0; Atomics.store(ctl, 0, 2); Atomics.notify(ctl, 0); return 'closed'; }
        else ctl[3] = 0;   // op 0: no-op
      } catch (e) { ctl[3] = -1; }
      Atomics.store(ctl, 0, 2);
      Atomics.notify(ctl, 0);
    }
  },
  async 'bridge-client'({ sab, n = 300 }) {
    const ctl = new Int32Array(sab, 0, 16);
    const call = (op, arg) => {
      ctl[1] = op; ctl[2] = arg; Atomics.store(ctl, 0, 1); Atomics.notify(ctl, 0);
      while (Atomics.load(ctl, 0) !== 2) Atomics.wait(ctl, 0, 1, 1000);
      Atomics.store(ctl, 0, 0);
      return ctl[3];
    };
    const noop = [], opens = [], reads = [];
    call(0, 0);
    for (let i = 0; i < n; i++) { const t0 = now(); call(0, 0); noop.push(now() - t0); }
    for (let i = 0; i < Math.min(n, 100); i++) { const t0 = now(); call(1, i); opens.push(now() - t0); }
    for (let i = 0; i < n; i++) { const t0 = now(); call(2, i % 100); reads.push(now() - t0); }
    call(3, 0);
    call(9, 0);
    return { noopMedianUs: med(noop) * 1000, noopP95Us: p95(noop) * 1000, openMedianMs: med(opens), openP95Ms: p95(opens), readMedianUs: med(reads) * 1000, readP95Us: p95(reads) * 1000 };
  },
  async jspi({ wasm, n = 2000 }) {
    if (typeof WebAssembly.Suspending !== 'function' || typeof WebAssembly.promising !== 'function') return { available: false };
    const d = await dir('bench/jspi');
    let opens = 0;
    const imports = { env: {
      resolved: new WebAssembly.Suspending(() => Promise.resolve(1)),
      open: new WebAssembly.Suspending(async (i) => { const fh = await d.getFileHandle('j' + (i % 50), { create: true }); const h = await fh.createSyncAccessHandle(); h.close(); opens++; return i; }),
    } };
    const { instance } = await WebAssembly.instantiate(wasm, imports);
    const runResolved = WebAssembly.promising(instance.exports.run_resolved), runOpen = WebAssembly.promising(instance.exports.run_open);
    let t0 = now(); await runResolved(n); const tr = now() - t0;
    t0 = now(); await runOpen(100); const to = now() - t0;
    return { available: true, resolvedPerSuspendUs: (tr / n) * 1000, openPerCallMs: to / 100, opens };
  },
};
self.onmessage = async (e) => {
  const { id, cmd, ...args } = e.data;
  try { self.postMessage({ id, ok: true, result: await C[cmd](args) }); } catch (err) { self.postMessage({ id, ok: false, error: `${err && err.name}: ${err && err.message}` }); }
};
