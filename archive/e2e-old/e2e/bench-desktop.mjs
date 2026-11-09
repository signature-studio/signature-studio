// Which way to put the windows on the desktop, measured (decision 0141, section "layers or one composite canvas"):
//   layers     one canvas per window, absolutely positioned and stacked by CSS; an update is a putImageData of the dirty rectangle into that window's canvas and the browser composites the layers
//   composite  one 1920x1080 canvas; every update redraws the window (drawImage of its own off-screen canvas) and every window above it, because the windows overlap
// Both are driven with the same updates (the dirty rectangles of a 16 ms animation of the main window plus a dialog on top) in the same headless Chrome; the numbers are the page's main-thread time
// per update (performance.now() around the calls and a 1x1 getImageData of every canvas touched, which makes the canvas run its recorded drawing commands - without it only the recording is timed) and the rate of
// requestAnimationFrame callbacks. The layered version's CSS compositing happens outside the page's main thread and is not in these numbers. Headless Chrome with --disable-gpu composites in software: a real GPU will differ.
import fs from 'node:fs';
import path from 'node:path';
import { openBrowser } from './browser.mjs';
import { freePort, start } from '../helpers/proc.mjs';
import { APP } from '../helpers/wrangler.mjs';

const out = process.argv[2] || '/tmp/opencode/app2/bench';
fs.mkdirSync(out, { recursive: true });
const port = await freePort();
const srv = start(path.join(APP, 'node_modules/.bin/vite'), ['--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env }, ready: /Local:\s+http/, label: 'vite dev' });
await srv.ready;
const browser = await openBrowser(out, 'bench');
const page = await browser.newPage({ width: 1920, height: 1080 });
const script = `(async () => {
  const wins = [ { w: 1930, h: 1092, x: -5, y: -6 }, { w: 668, h: 900, x: 626, y: 90 }, { w: 874, h: 570, x: 523, y: 255 } ];
  const mk = (w, h) => { const c = document.createElement('canvas'); c.width = w; c.height = h; return c; };
  const fill = (ctx, w, h, k) => { const d = ctx.createImageData(w, h); for (let i = 0; i < d.data.length; i += 4) { d.data[i] = (i >> 2) + k; d.data[i + 1] = k * 7; d.data[i + 2] = 90; d.data[i + 3] = 255; } return d; };
  const rect = { w: 200, h: 120 };   // a dirty rectangle (the engine's frame of the animated part of the main window)
  const N = 120;
  async function frames(update) {
    const times = []; let raf = 0; const t0 = performance.now();
    await new Promise((res) => { let n = 0; const tick = () => { const a = performance.now(); const fl = update(n); for (const c of fl) c.getImageData(0, 0, 1, 1); times.push(performance.now() - a); raf++; if (++n >= N) res(); else requestAnimationFrame(tick); }; requestAnimationFrame(tick); });
    const total = performance.now() - t0;
    times.sort((a, b) => a - b);
    return { updatesPerSec: +(N / (total / 1000)).toFixed(1), mainThreadMsMedian: +times[N >> 1].toFixed(3), mainThreadMsP95: +times[Math.floor(N * 0.95)].toFixed(3), mainThreadMsMax: +times[N - 1].toFixed(3) };
  }
  // layers
  const root = document.createElement('div'); root.style.cssText = 'position:relative;width:1920px;height:1080px;overflow:hidden'; document.body.append(root);
  const layers = wins.map((s, i) => { const c = mk(s.w, s.h); c.style.cssText = 'position:absolute;left:' + s.x + 'px;top:' + s.y + 'px;width:' + s.w + 'px;height:' + s.h + 'px;z-index:' + i; root.append(c); const ctx = c.getContext('2d'); ctx.putImageData(fill(ctx, s.w, s.h, i), 0, 0); return { c, ctx }; });
  const dirty = layers.map((l, i) => fill(l.ctx, rect.w, rect.h, 40 + i));
  const L = await frames((n) => { layers[0].ctx.putImageData(dirty[0], (n * 13) % 1500, (n * 7) % 900); return [layers[0].ctx]; });
  const L2 = await frames((n) => { layers[0].ctx.putImageData(dirty[0], (n * 13) % 1500, (n * 7) % 900); layers[2].ctx.putImageData(dirty[2], 40, 40); return [layers[0].ctx, layers[2].ctx]; });
  root.remove();
  // composite
  const screen = mk(1920, 1080); document.body.append(screen); const sctx = screen.getContext('2d');
  const srcs = wins.map((s, i) => { const c = mk(s.w, s.h); const ctx = c.getContext('2d'); ctx.putImageData(fill(ctx, s.w, s.h, i), 0, 0); return { c, ctx }; });
  const draw = (upto) => { for (let i = 0; i <= 2; i++) sctx.drawImage(srcs[i].c, wins[i].x, wins[i].y); };
  const C = await frames((n) => { srcs[0].ctx.putImageData(dirty[0], (n * 13) % 1500, (n * 7) % 900); draw(); return [sctx]; });
  const C2 = await frames((n) => { srcs[0].ctx.putImageData(dirty[0], (n * 13) % 1500, (n * 7) % 900); srcs[2].ctx.putImageData(dirty[2], 40, 40); draw(); return [sctx]; });
  // composite with clipping to the dirty rectangle only (what a smarter composite would do): redraw the layers inside the rectangle
  const C3 = await frames((n) => { const x = (n * 13) % 1500, y = (n * 7) % 900; srcs[0].ctx.putImageData(dirty[0], x, y); sctx.save(); sctx.beginPath(); sctx.rect(x - 5, y - 6, rect.w, rect.h); sctx.clip(); draw(); sctx.restore(); return [sctx]; });
  return { layers_oneWindowUpdates: L, layers_twoWindowUpdates: L2, composite_fullRedraw_oneWindow: C, composite_fullRedraw_twoWindows: C2, composite_clippedRedraw_oneWindow: C3 };
})()`;
try {
  await page.goto(`http://127.0.0.1:${port}/`);
  await page.waitFor('document.readyState === "complete"', 20000);
  const r = await page.ev(script, 120000);
  console.log(JSON.stringify(r, null, 1));
  fs.writeFileSync(path.join(out, 'bench-desktop.json'), JSON.stringify({ chrome: browser.version, result: r }, null, 1));
} finally {
  await browser.close();
  await srv.stop();
}
