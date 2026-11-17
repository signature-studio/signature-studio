'use strict';
// Drawing, page side. Autonomous part (window.__result): env, worker_offscreen, main_canvas, postpixels. Driver-controlled part: window.presentStart(style, ms)
// (run-draw.mjs takes screenshots of the displayed canvas while the Worker draws in a given loop style).
const W = 1920, H = 1080, BYTES = W * H * 4;
const Q = new URLSearchParams(location.search);
const N = +(Q.get('n') || 200);
const WANT = (Q.get('tests') || 'env,worker_offscreen,main_canvas,postpixels').split(',');
const log = (s) => { (window.__log = window.__log || []).push(s); };

function summarize(arr, qs) {
  qs = qs || [50, 95, 99];
  const a = Float64Array.from(arr).sort();
  const n = a.length;
  const o = { n };
  if (!n) return o;
  o.min = a[0]; o.max = a[n - 1]; o.mean = a.reduce((s, x) => s + x, 0) / n;
  for (const q of qs) o['p' + q] = a[Math.min(n - 1, Math.floor((q / 100) * n))];
  return o;
}
function touch(u32, frame) {
  const o = (frame * 977) % (W * H - 4096);
  for (let k = 0; k < 4096; k++) u32[o + k] = 0xff000000 | (frame * 2654435761 + k) >>> 0;
}
function workerRun(msg, transfer, onmsg) {
  const w = new Worker('draw-worker.js');
  const p = new Promise((res, rej) => {
    w.onmessage = (e) => { if (e.data.type === 'result') res(e.data.result); else if (onmsg) onmsg(e.data); };
    w.onerror = (e) => rej(new Error('worker error: ' + e.message));
  });
  w.postMessage(msg, transfer || []);
  return { w, p };
}

async function mainCanvas(n) {
  const cv = document.getElementById('cvm');
  const ctx = cv.getContext('2d');
  const buf = new Uint8ClampedArray(BYTES);
  const u32 = new Uint32Array(buf.buffer);
  u32.fill(0xff303030);
  const full = new ImageData(buf, W, H);
  const R = { size: [W, H], frames: n };
  const run = (label, fn) => {
    for (let k = 0; k < 20; k++) { touch(u32, k); fn(k); }
    const ts = [];
    const t0 = performance.now();
    for (let k = 0; k < n; k++) { touch(u32, k + 100); const t = performance.now(); fn(k); ts.push(performance.now() - t); }
    R[label] = { fps_total: +(n * 1000 / (performance.now() - t0)).toFixed(1), ms_per_call: summarize(ts) };
  };
  run('put_full', () => ctx.putImageData(full, 0, 0));
  for (const [dw, dh] of [[960, 540], [480, 270], [256, 256], [64, 64], [16, 16]]) run(`put_dirty_${dw}x${dh}`, () => ctx.putImageData(full, 0, 0, 0, 0, dw, dh));
  {
    const bc = document.getElementById('cvb').getContext('bitmaprenderer');
    const ts = [];
    for (let k = 0; k < Math.min(n, 100); k++) { touch(u32, k); const t = performance.now(); const bmp = await createImageBitmap(full); bc.transferFromImageBitmap(bmp); ts.push(performance.now() - t); }
    R.createImageBitmap_bitmaprenderer = { ms_per_call_incl_await: summarize(ts) };
  }
  // frame cadence: how many putImageData(full) fit in rAF frames (the display refresh the page is given)
  {
    const stamps = [];
    await new Promise((res) => { let k = 0; const step = (t) => { touch(u32, k); ctx.putImageData(full, 0, 0); stamps.push(t); if (++k < 120) requestAnimationFrame(step); else res(); }; requestAnimationFrame(step); });
    const d = []; for (let k = 1; k < stamps.length; k++) d.push(stamps[k] - stamps[k - 1]);
    R.raf_with_put_full = { interval_ms: summarize(d), fps: +(1000 / (d.reduce((a, b) => a + b, 0) / d.length)).toFixed(1) };
  }
  return R;
}

async function postPixels(n) {
  const cv = document.getElementById('cvm');
  const ctx = cv.getContext('2d');
  const lat = [];
  const drawMs = [];
  let got = 0;
  let firstT = 0, lastT = 0;
  let done;
  const fin = new Promise((r) => { done = r; });
  const wk = workerRun({ mode: 'postpixels', n, interval: 0 }, [], (d) => {
    if (d.type !== 'frame') return;
    const now = performance.timeOrigin + performance.now();
    lat.push(now - d.t);
    const t = performance.now();
    ctx.putImageData(new ImageData(new Uint8ClampedArray(d.buf), W, H), 0, 0);
    drawMs.push(performance.now() - t);
    if (!got) firstT = performance.now();
    lastT = performance.now();
    if (++got === n) done();
  });
  await fin;
  const r = await wk.p;
  wk.w.terminate();
  return { n, worker: r, post_to_receive_ms: summarize(lat), main_putImageData_ms: summarize(drawMs), fps_main_receive_and_draw: +((n - 1) * 1000 / (lastT - firstT)).toFixed(1) };
}

// ---- driver-controlled presentation test
window.presentStart = (style, ms) => {
  window.scrollTo(0, 0);
  document.getElementById('host').innerHTML = '';
  const cv = document.createElement('canvas');
  cv.width = W; cv.height = H; cv.style.width = '960px'; cv.style.height = '540px';
  document.getElementById('host').appendChild(cv);
  window.__present = { style, done: false, info: null, bitmaps: 0 };
  if (style === 'bitmap') {
    const bc = cv.getContext('bitmaprenderer');
    const wk = workerRun({ mode: 'bitmap', ms }, [], (d) => { if (d.type === 'bitmap') { bc.transferFromImageBitmap(d.bmp); window.__present.bitmaps++; } });
    wk.p.then((r) => { window.__present.info = r; window.__present.done = true; wk.w.terminate(); });
  } else if (style === 'mainthread_put') {
    // the page itself draws (for comparison): rAF loop, putImageData of a full frame
    const ctx = cv.getContext('2d');
    const buf = new Uint8ClampedArray(BYTES);
    const u32 = new Uint32Array(buf.buffer);
    u32.fill(0xff202020);
    const img = new ImageData(buf, W, H);
    const t0 = performance.now();
    let k = 0;
    const step = () => {
      { const v = 0xff000000 | (k & 255) | 0x00800000; for (let y = 0; y < 64; y++) u32.fill(v, y * W, y * W + 64); }
      touch(u32, k);
      ctx.putImageData(img, 0, 0); k++;
      if (performance.now() - t0 < ms) requestAnimationFrame(step); else { window.__present.info = { frames: k, elapsed_ms: performance.now() - t0 }; window.__present.done = true; }
    };
    requestAnimationFrame(step);
  } else {
    const off = cv.transferControlToOffscreen();
    const wk = workerRun({ mode: 'present', style, ms, canvas: off }, [off]);
    wk.p.then((r) => { window.__present.info = r; window.__present.done = true; wk.w.terminate(); });
  }
  return true;
};
window.decodeShot = async (b64) => {
  const bin = atob(b64);
  const u8 = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
  const bmp = await createImageBitmap(new Blob([u8], { type: 'image/png' }));
  const c = new OffscreenCanvas(bmp.width, bmp.height);
  const x = c.getContext('2d');
  x.drawImage(bmp, 0, 0);
  const d = x.getImageData(2, 0, 1, 1).data;
  return [d[0], d[1], d[2], d[3], bmp.width, bmp.height];
};

async function main() {
  const R = { env: {
    crossOriginIsolated: self.crossOriginIsolated, OffscreenCanvas: typeof OffscreenCanvas,
    transferControlToOffscreen: typeof HTMLCanvasElement.prototype.transferControlToOffscreen,
    offscreen2dCommit: typeof (self.OffscreenCanvasRenderingContext2D && OffscreenCanvasRenderingContext2D.prototype.commit),
    offscreenTransferToImageBitmap: typeof (self.OffscreenCanvas && OffscreenCanvas.prototype.transferToImageBitmap),
    devicePixelRatio, userAgent: navigator.userAgent,
    gl: (() => { try { const c = document.createElement('canvas'); const g = c.getContext('webgl2') || c.getContext('webgl'); if (!g) return null; const e = g.getExtension('WEBGL_debug_renderer_info'); return e ? g.getParameter(e.UNMASKED_RENDERER_WEBGL) : g.getParameter(g.RENDERER); } catch (e) { return String(e); } })(),
  } };
  log('env ' + JSON.stringify(R.env));
  if (WANT.includes('worker_offscreen')) { const wk = workerRun({ mode: 'offscreen', n: N }); R.worker_offscreen = await wk.p; wk.w.terminate(); log('worker_offscreen done'); }
  if (WANT.includes('main_canvas')) { R.main_canvas = await mainCanvas(N); log('main_canvas done'); }
  if (WANT.includes('postpixels')) { R.postpixels = await postPixels(Math.min(N, 120)); log('postpixels done'); }
  window.__result = R;
  window.__done = true;
}
main().catch((e) => { window.__error = String(e && e.stack || e); });
