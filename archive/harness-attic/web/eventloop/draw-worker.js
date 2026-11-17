'use strict';
// Drawing part of the event-loop study: how fast can a pixel buffer that lives in a Worker's memory (the DIB section of lift/runtime's virtual DC)
// be put on a canvas. importScripts('ring.js') gives summarize/spin/Ring. Messages: {mode, ...}; result via postMessage({type:'result'}).
importScripts('ring.js');
const W = 1920, H = 1080, BYTES = W * H * 4;

function bigMem() {
  const mem = new WebAssembly.Memory({ initial: 160 + 1 });  // 10.3 MiB > 8.3 MB pixel block + 1 MiB base
  return mem;
}
// the "engine" touches a small changing region per frame (so successive frames differ) - not part of the measured cost
function touch(u32, frame) {
  const o = (frame * 977) % (W * H - 4096);
  for (let k = 0; k < 4096; k++) u32[o + k] = 0xff000000 | (frame * 2654435761 + k) >>> 0;
}
const time = (fn) => { const t = performance.now(); fn(); return (performance.now() - t); };

onmessage = async (e) => {
  const c = e.data;
  try {
    const r = await MODES[c.mode](c);
    postMessage({ type: 'result', result: r });
  } catch (err) {
    postMessage({ type: 'result', result: { error: String(err && err.stack || err) } });
  }
};

const MODES = {
  // standalone OffscreenCanvas (not shown): ImageData over the wasm memory and putImageData
  async offscreen(c) {
    const mem = bigMem();
    const base = 1 << 20;
    const u8 = new Uint8ClampedArray(mem.buffer, base, BYTES);
    const u32 = new Uint32Array(mem.buffer, base, W * H);
    u32.fill(0xff202020);
    const oc = new OffscreenCanvas(W, H);
    const ctx = oc.getContext('2d');
    const n = c.n;
    const R = { size: [W, H], frames: n, ctx2d: !!ctx };
    // ImageData construction over the linear memory: zero-copy?
    {
      const ts = [];
      for (let k = 0; k < 200; k++) ts.push(time(() => { new ImageData(u8, W, H); }));
      R.imageDataConstruct_ms = summarize(ts);
      const id = new ImageData(u8, W, H);
      u8[0] = 77;
      R.imageDataSharesMemory = id.data[0] === 77;
    }
    // shared source: ImageData refuses a SharedArrayBuffer view; copy into a non-shared array first
    {
      const sab = new SharedArrayBuffer(BYTES);
      let refused = null;
      try { new ImageData(new Uint8ClampedArray(sab), W, H); refused = 'accepted'; } catch (err) { refused = String(err.name + ': ' + err.message).slice(0, 120); }
      R.imageDataFromSharedView = refused;
      const dst = new Uint8ClampedArray(BYTES);
      const src = new Uint8ClampedArray(sab);
      const ts = [];
      for (let k = 0; k < 100; k++) ts.push(time(() => { dst.set(src); }));
      R.copyShared8MB_ms = summarize(ts);
      const ts2 = [];
      for (let k = 0; k < 100; k++) ts2.push(time(() => { dst.set(u8); }));
      R.copyLinear8MB_ms = summarize(ts2);
    }
    const full = new ImageData(u8, W, H);
    const run = (label, fn, flush) => {
      for (let k = 0; k < 20; k++) { touch(u32, k); fn(k); }   // warm-up
      if (flush) ctx.getImageData(0, 0, 1, 1);
      const ts = [];
      const t0 = performance.now();
      for (let k = 0; k < n; k++) { touch(u32, k + 100); const t = performance.now(); fn(k); if (flush) ctx.getImageData(0, 0, 1, 1); ts.push(performance.now() - t); }
      const total = performance.now() - t0;
      R[label] = { fps_total: +(n * 1000 / total).toFixed(1), ms_per_frame: summarize(ts) };
    };
    run('put_full', () => ctx.putImageData(full, 0, 0), false);
    run('put_full_flush1px', () => ctx.putImageData(full, 0, 0), true);
    for (const [dw, dh] of [[960, 540], [480, 270], [256, 256], [64, 64], [16, 16]]) {
      run(`put_dirty_${dw}x${dh}`, () => ctx.putImageData(full, 0, 0, 0, 0, dw, dh), false);
    }
    {
      const oc2 = new OffscreenCanvas(W, H);
      const c2 = oc2.getContext('2d');
      const ts = [];
      for (let k = 0; k < n; k++) { touch(u32, k); const t = performance.now(); c2.putImageData(full, 0, 0); const bmp = oc2.transferToImageBitmap(); ts.push(performance.now() - t); bmp.close(); }
      R.put_full_then_transferToImageBitmap = { ms_per_frame: summarize(ts) };
    }
    {
      const ts = [];
      for (let k = 0; k < Math.min(n, 100); k++) { touch(u32, k); const t = performance.now(); const bmp = await createImageBitmap(full); ts.push(performance.now() - t); bmp.close(); }
      R.createImageBitmap_from_ImageData = { ms_per_call_incl_await: summarize(ts) };
    }
    return R;
  },

  // pixels written into a SharedArrayBuffer by this worker, copied to a transferable and posted (what the page would need for a main-thread canvas)
  async postpixels(c) {
    const mem = bigMem();
    const base = 1 << 20;
    const u32 = new Uint32Array(mem.buffer, base, W * H);
    const u8 = new Uint8Array(mem.buffer, base, BYTES);
    const ts = [];
    for (let k = 0; k < c.n; k++) {
      touch(u32, k);
      const t = performance.now();
      const copy = u8.slice().buffer;           // copy out of the linear memory (it cannot be transferred itself)
      ts.push(performance.now() - t);
      postMessage({ type: 'frame', k, buf: copy, t: performance.timeOrigin + performance.now() }, [copy]);
      await new Promise((r) => setTimeout(r, c.interval || 0));
    }
    return { copy_ms: summarize(ts), n: c.n };
  },

  // draw to a placeholder canvas transferred from the page, in three loop styles; the page checks what is displayed
  async present(c) {
    const ctx = c.canvas.getContext('2d');
    const mem = bigMem();
    const base = 1 << 20;
    const u8 = new Uint8ClampedArray(mem.buffer, base, BYTES);
    const u32 = new Uint32Array(mem.buffer, base, W * H);
    const img = new ImageData(u8, W, H);
    u32.fill(0xff202020);
    const stamp = (k) => { const v = 0xff000000 | (k & 255) | (((k >> 8) & 255) << 8) | 0x00800000; for (let y = 0; y < 64; y++) u32.fill(v, y * W, y * W + 64); };   // the 64x64 block at the origin carries the frame number (R = k & 255)
    const info = { style: c.style, hasCommit: typeof ctx.commit, hasRAF: typeof requestAnimationFrame };
    const t0 = performance.now();
    let k = 0;
    let drawMs = 0;
    const frame = () => { stamp(k); touch(u32, k); const t = performance.now(); ctx.putImageData(img, 0, 0); drawMs += performance.now() - t; k++; if (c.style === 'blocking_commit' && ctx.commit) ctx.commit(); };
    if (c.style === 'raf') {
      await new Promise((res) => {
        const step = () => { frame(); if (performance.now() - t0 < c.ms) requestAnimationFrame(step); else res(); };
        requestAnimationFrame(step);
      });
    } else if (c.style === 'timeout') {
      await new Promise((res) => {
        const step = () => { frame(); if (performance.now() - t0 < c.ms) setTimeout(step, 16); else res(); };
        step();
      });
    } else {  // blocking: never returns to the event loop while it draws (like a lifted GetMessage loop)
      const i = new Int32Array(new SharedArrayBuffer(16));
      while (performance.now() - t0 < c.ms) { frame(); Atomics.wait(i, 0, 0, 16); }
    }
    info.frames = k;
    info.drawMs_per_frame = drawMs / k;
    info.elapsed_ms = performance.now() - t0;
    info.lastStamp = k - 1;
    return info;
  },

  // blocking loop that sends an ImageBitmap to the page each frame (postMessage from a blocked worker is delivered)
  async bitmap(c) {
    const oc = new OffscreenCanvas(W, H);
    const ctx = oc.getContext('2d');
    const mem = bigMem();
    const base = 1 << 20;
    const u8 = new Uint8ClampedArray(mem.buffer, base, BYTES);
    const u32 = new Uint32Array(mem.buffer, base, W * H);
    u32.fill(0xff202020);
    const img = new ImageData(u8, W, H);
    const t0 = performance.now();
    let k = 0;
    const i = new Int32Array(new SharedArrayBuffer(16));
    const ts = [];
    while (performance.now() - t0 < c.ms) {
      { const v = 0xff000000 | (k & 255) | 0x00800000; for (let y = 0; y < 64; y++) u32.fill(v, y * W, y * W + 64); }
      touch(u32, k);
      const t = performance.now();
      ctx.putImageData(img, 0, 0);
      const bmp = oc.transferToImageBitmap();
      postMessage({ type: 'bitmap', k, bmp }, [bmp]);
      ts.push(performance.now() - t);
      k++;
      Atomics.wait(i, 0, 0, 16);
    }
    return { frames: k, put_and_transfer_ms: summarize(ts), elapsed_ms: performance.now() - t0 };
  },
};
