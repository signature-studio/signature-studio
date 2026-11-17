// mt2: what the dirty-rectangle copy of a SHARED memory costs the desktop presenter (decision 0223 shows a view of the engine's memory with putImageData, no copy; ImageData refuses a view of a SharedArrayBuffer).
// In a Worker with an OffscreenCanvas 2d context: per presented rectangle, the time of the call(s) and of a flush (getImageData 1x1) after N rectangles.
// variants: cur = non-shared memory, ImageData over a view of it (decision 0223, no copy); rowcopy = shared memory, rows of the rectangle .set into a full-size ordinary ImageData, putImageData(dirty);
//           compact = shared, rows .set into a w*h ordinary ImageData, putImageData(x0,y0); fullcopy = shared, the whole picture .set into an ordinary ImageData, putImageData(dirty); gl = shared, texSubImage2D from the shared view (WebGL2, software GL in headless runs).
const OFF = 0x1000000;   // the picture lives 16 MiB into the memory
function fill(u8, n) { let x = 0x9E3779B9 | 0; for (let i = 0; i < n; i++) { x ^= x << 13; x ^= x >>> 17; x ^= x << 5; u8[i] = x & 255; } }
self.onmessage = async (ev) => {
  const out = { ua: navigator.userAgent, isolated: self.crossOriginIsolated, results: [] };
  const cases = ev.data.cases;   // [{W,H,x0,y0,w,h,iters,label}]
  for (const c of cases) {
    const bytes = c.W * c.H * 4, pages = Math.ceil((OFF + bytes) / 65536) + 1;
    const mem = new WebAssembly.Memory({ initial: pages, maximum: pages });
    const smem = new WebAssembly.Memory({ initial: pages, maximum: pages, shared: true });
    fill(new Uint8Array(mem.buffer, OFF, bytes), bytes);
    fill(new Uint8Array(smem.buffer, OFF, bytes), bytes);
    const canvas = new OffscreenCanvas(c.W, c.H); const ctx = canvas.getContext('2d');
    const view = new Uint8ClampedArray(mem.buffer, OFF, bytes), img = new ImageData(view, c.W, c.H);
    const sview = new Uint8Array(smem.buffer, OFF, bytes);
    const scratch = new ImageData(c.W, c.H), compact = new ImageData(c.w, c.h);
    const variants = {
      cur: () => { ctx.putImageData(img, 0, 0, c.x0, c.y0, c.w, c.h); },
      rowcopy: () => { for (let y = c.y0; y < c.y0 + c.h; y++) scratch.data.set(sview.subarray((y * c.W + c.x0) * 4, (y * c.W + c.x0 + c.w) * 4), (y * c.W + c.x0) * 4); ctx.putImageData(scratch, 0, 0, c.x0, c.y0, c.w, c.h); },
      compact: () => { for (let y = 0; y < c.h; y++) compact.data.set(sview.subarray(((c.y0 + y) * c.W + c.x0) * 4, ((c.y0 + y) * c.W + c.x0 + c.w) * 4), y * c.w * 4); ctx.putImageData(compact, c.x0, c.y0); },
      fullcopy: () => { scratch.data.set(sview); ctx.putImageData(scratch, 0, 0, c.x0, c.y0, c.w, c.h); },
    };
    const row = { label: c.label, W: c.W, H: c.H, rect: [c.x0, c.y0, c.w, c.h], iters: c.iters, v: {} };
    for (const [name, f] of Object.entries(variants)) {
      for (let i = 0; i < 8; i++) f();
      ctx.getImageData(0, 0, 1, 1);
      let inCall = 0; const t0 = performance.now();
      for (let i = 0; i < c.iters; i++) { const a = performance.now(); f(); inCall += performance.now() - a; }
      const t1 = performance.now(); ctx.getImageData(0, 0, 1, 1); const t2 = performance.now();
      row.v[name] = { in_call_ms: inCall / c.iters, loop_ms: (t1 - t0) / c.iters, with_flush_ms: (t2 - t0) / c.iters };
    }
    try {
      const gc = new OffscreenCanvas(c.W, c.H), gl = gc.getContext('webgl2');
      if (!gl) throw new Error('no webgl2');
      const tex = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, c.W, c.H, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, c.W); gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, c.x0); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, c.y0);
      const f = () => { gl.texSubImage2D(gl.TEXTURE_2D, 0, c.x0, c.y0, c.w, c.h, gl.RGBA, gl.UNSIGNED_BYTE, sview, 0 + (c.y0 * c.W + c.x0) * 0); };
      for (let i = 0; i < 4; i++) f(); gl.finish();
      let inCall = 0; const t0 = performance.now();
      for (let i = 0; i < c.iters; i++) { const a = performance.now(); f(); inCall += performance.now() - a; }
      const t1 = performance.now(); gl.finish(); const t2 = performance.now();
      row.v.gl = { in_call_ms: inCall / c.iters, loop_ms: (t1 - t0) / c.iters, with_flush_ms: (t2 - t0) / c.iters, note: 'texSubImage2D from the shared view (upload only, no draw); software GL in headless' };
    } catch (e) { row.v.gl = { error: String(e).slice(0, 120) }; }
    out.results.push(row);
  }
  self.postMessage(out);
};
