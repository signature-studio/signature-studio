// Decision 0240 (gpu1): the WebGL2 presenter (src/engine/desk-gl.ts) driven DIRECTLY in a real browser, on whatever WebGL2 the headless Chrome has (SwiftShader), against a JS version of the compositor's formulas
// (lift/runtime/fle_desk.c draw_rect: opaque, LWA_ALPHA / LWA_COLORKEY, UpdateLayeredWindow's premultiplied alpha with div255 rounding, decision 0244's extended frame (glass) and window region). The product chooses between WebGL2 and the 2D picture by what
// the platform answers (webgl2Usable: failIfMajorPerformanceCaveat, MAX_TEXTURE_SIZE); a software renderer may answer no (the answer of the browser in use is printed as usableByThePlatformRule), so a test of the GL path does not go through the product page:
// this is the separate entry (no switch is in the product's code).
// The scene description is written into a fake engine memory the way the runtime writes it (strided B, G, R, X surfaces at addresses, windows bottom first, written rectangles).
// usage: node test/e2e/desk-gl-presenter.mjs [OUTDIR]     (not part of run.mjs: it needs a browser)
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { openBrowser } from './browser.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const OUT = path.resolve(process.argv[2] ?? '/tmp/opencode/desk-gl-presenter');
fs.mkdirSync(OUT, { recursive: true });
const bundle = (await build({ entryPoints: [path.join(HERE, '../../src/engine/desk-gl.ts')], bundle: true, format: 'iife', globalName: 'DeskGl', platform: 'browser', write: false, logLevel: 'silent' })).outputFiles[0].text;

const b = await openBrowser(OUT, 'gl');
const page = await b.newPage({ width: 400, height: 300, dpr: 1 });
await page.goto('about:blank');
const res = await page.ev(`(async () => {
  ${bundle}
  const W = 96, H = 64, MEM = 1 << 20, SC = 4096;
  const mem = new ArrayBuffer(MEM), u8 = new Uint8Array(mem), dv = new DataView(mem);
  const div255 = (x) => { x += 128; return (x + (x >> 8)) >> 8; };
  let seed = 4242; const rnd = () => { seed = (seed * 1664525 + 1013904223) >>> 0; return seed >>> 8; };
  // windows: surfaces live at addresses in the fake memory, with a stride larger than the width (as the runtime's surfaces may have)
  let nextAddr = 0x10000;
  const mkWin = (lid, x, y, w, h, layer, alpha, key) => { const stride = w * 4 + 16; const bits = nextAddr; nextAddr += stride * h + 64; for (let i = 0; i < stride * h; i++) u8[bits + i] = rnd() & 255; return { lid, x, y, w, h, bits, stride, layer: layer || 0, alpha: alpha ?? 255, key: key ?? 0, fresh: true, g: [0, 0, 0, 0], c: [0, 0, w, h] }; };
  const LAYERED = 8, A = 1, K = 2, P = 4, GLASS = 16;
  const keyRGB = 0x00c81e0a;   // COLORREF 0x00BBGGRR: R 0x0a, G 0x1e, B 0xc8
  const wins = [mkWin(1, 0, 0, 96, 64, 0), mkWin(2, 10, 8, 40, 30, 0), mkWin(3, 30, 20, 50, 36, LAYERED | A, 128), mkWin(4, 5, 30, 36, 30, LAYERED | K, 255, keyRGB), mkWin(5, 40, 5, 44, 30, LAYERED | P | A, 200), mkWin(6, 12, 40, 50, 22, GLASS), mkWin(7, 60, 36, 34, 26, GLASS)];
  wins[5].g = [8, 6, 42, 16];                         // glass with margins: a frame of premultiplied ARGB round an opaque inner box
  wins[6].c = [4, 3, 30, 20];                         // a sheet of glass (inner box empty) kept to a window region
  wins[2].c = [6, 5, 44, 30];                         // a constant-alpha layered window with a region
  // make some pixels of window 4 the key colour, window 5 premultiplied (colour <= alpha, some transparent)
  const px = (w, x, y) => w.bits + y * w.stride + x * 4;
  for (let y = 0; y < wins[3].h; y++) for (let x = 0; x < wins[3].w; x++) if ((rnd() & 3) === 0) { const p = px(wins[3], x, y); u8[p] = 0xc8; u8[p + 1] = 0x1e; u8[p + 2] = 0x0a; }
  const premulFill = (w) => { for (let y = 0; y < w.h; y++) for (let x = 0; x < w.w; x++) { const p = px(w, x, y), a = rnd() & 255; const m = (rnd() & 3) === 0 ? 0 : a; u8[p] = Math.min(u8[p], m); u8[p + 1] = Math.min(u8[p + 1], m); u8[p + 2] = Math.min(u8[p + 2], m); u8[p + 3] = a; if ((rnd() & 15) === 0) { u8[p] = u8[p + 1] = u8[p + 2] = 0; u8[p + 3] = 0; } } };
  premulFill(wins[4]); premulFill(wins[5]); premulFill(wins[6]);
  const writeScene = (flags, upds, ws) => {
    dv.setUint32(SC, 0x4e454353, true); dv.setUint32(SC + 4, 2, true); dv.setUint32(SC + 8, W, true); dv.setUint32(SC + 12, H, true); dv.setUint32(SC + 16, 1, true);
    dv.setUint32(SC + 20, ws.length, true); dv.setUint32(SC + 24, upds.length, true); dv.setUint32(SC + 28, flags, true); dv.setUint32(SC + 32, 5 | (7 << 8) | (10 << 16), true);
    ws.forEach((w, i) => { const o = SC + 64 + i * 80; [w.lid, w.x, w.y, w.w, w.h, w.bits, w.stride, w.layer, w.alpha, w.key, w.fresh ? 1 : 0, 0, ...w.g, ...w.c].forEach((v, k) => dv.setInt32(o + k * 4, v, true)); });
    upds.forEach((u, i) => { const o = SC + 64 + 64 * 80 + i * 20; [u.lid, u.x0, u.y0, u.x1, u.y1].forEach((v, k) => dv.setInt32(o + k * 4, v, true)); });
  };
  // the reference compositor (fle_desk.c draw_rect on a W x H picture, background 5 7 10)
  const reference = (ws) => {
    const pic = new Uint8Array(W * H * 4); for (let i = 0; i < W * H; i++) { pic[4 * i] = 5; pic[4 * i + 1] = 7; pic[4 * i + 2] = 10; pic[4 * i + 3] = 255; }
    for (const w of ws) {
      const glass = (w.layer & LAYERED) === 0 && (w.layer & GLASS) !== 0;
      const layered = (w.layer & LAYERED) !== 0, keyed = layered && (w.layer & K) !== 0, premul = layered && (w.layer & P) !== 0, alpha = (w.layer & A) ? w.alpha & 255 : 255;
      const kr = w.key & 255, kg = (w.key >> 8) & 255, kb = (w.key >> 16) & 255;
      for (let y = 0; y < w.h; y++) for (let x = 0; x < w.w; x++) {
        if (x < w.c[0] || y < w.c[1] || x >= w.c[2] || y >= w.c[3]) continue;          // outside the window region
        const X = w.x + x, Y = w.y + y; if (X < 0 || Y < 0 || X >= W || Y >= H) continue;
        const s = w.bits + y * w.stride + x * 4, b0 = u8[s], g0 = u8[s + 1], r0 = u8[s + 2], a0 = u8[s + 3], d = (Y * W + X) * 4;
        if (glass) { if (y >= w.g[1] && y < w.g[3] && x >= w.g[0] && x < w.g[2]) { pic[d] = r0; pic[d + 1] = g0; pic[d + 2] = b0; } else { if (a0 === 0 && (b0 | g0 | r0) === 0) continue; const k = 65025 - a0 * 255; const f = (c, dd) => Math.min(255, Math.floor((c * 255 * 255 + dd * k + 32512) / 65025)); pic[d] = f(r0, pic[d]); pic[d + 1] = f(g0, pic[d + 1]); pic[d + 2] = f(b0, pic[d + 2]); } }
        else if (!layered || (alpha === 255 && !keyed && !premul)) { pic[d] = r0; pic[d + 1] = g0; pic[d + 2] = b0; }
        else if (premul) { if (a0 === 0 && (b0 | g0 | r0) === 0) continue; const k = 65025 - a0 * alpha; const f = (c, dd) => Math.min(255, Math.floor((c * alpha * 255 + dd * k + 32512) / 65025)); pic[d] = f(r0, pic[d]); pic[d + 1] = f(g0, pic[d + 1]); pic[d + 2] = f(b0, pic[d + 2]); }
        else { if (keyed && r0 === kr && g0 === kg && b0 === kb) continue; if (alpha === 255) { pic[d] = r0; pic[d + 1] = g0; pic[d + 2] = b0; } else { const ia = 255 - alpha; pic[d] = div255(r0 * alpha + pic[d] * ia); pic[d + 1] = div255(g0 * alpha + pic[d + 1] * ia); pic[d + 2] = div255(b0 * alpha + pic[d + 2] * ia); } }
      }
    }
    return pic;
  };
  const compare = (name, got, want) => { let n = 0, mx = 0; for (let i = 0; i < W * H; i++) { const d = Math.max(Math.abs(got[4*i] - want[4*i]), Math.abs(got[4*i+1] - want[4*i+1]), Math.abs(got[4*i+2] - want[4*i+2])); if (d) { n++; if (d > mx) mx = d; } } return { name, differing: n, maxChannelDiff: mx }; };
  const canvas = new OffscreenCanvas(W, H);
  const api = { sceneMode() {}, resync() {}, describe: () => '', scene: () => SC };
  const p = new DeskGl.GlDeskPresenter(canvas, api, () => mem);
  const out = [];
  const full = (ws) => ws.map((w) => ({ lid: w.lid, x0: 0, y0: 0, x1: w.w, y1: w.h }));
  // 1. everything is new
  writeScene(3, full(wins), wins); p.present(); out.push(compare('all seven windows (opaque, constant alpha, colour key, premultiplied, extended frame, window region), strided surfaces', p.readPixels().data, reference(wins)));
  // 2. a move and a restack: no pixel is uploaded
  const before = p.stats.uploads; const moved = wins.map((w) => ({ ...w, fresh: false })); moved[1].x = 20; moved[1].y = 14; const re = [moved[0], moved[2], moved[1], moved[3], moved[4], moved[5], moved[6]];
  writeScene(2, [], re); const shown = p.present(); out.push({ name: 'a move and a restack upload nothing', uploads: p.stats.uploads - before, shown }); out.push(compare('after the move and restack', p.readPixels().data, reference(re)));
  // 3. a written rectangle inside window 2: only that rectangle is uploaded, and the picture is the new one
  for (let y = 3; y < 9; y++) for (let x = 4; x < 17; x++) { const q = px(re[2], x, y); u8[q] = rnd() & 255; u8[q + 1] = rnd() & 255; u8[q + 2] = rnd() & 255; }
  const before2 = p.stats.pixels; writeScene(0, [{ lid: 2, x0: 4, y0: 3, x1: 17, y1: 9 }], re.map((w) => ({ ...w, fresh: false }))); p.present(); out.push({ name: 'a written rectangle uploads only its pixels', pixels: p.stats.pixels - before2 }); out.push(compare('after the written rectangle', p.readPixels().data, reference(re)));
  // 4. a window hidden
  const hid = re.filter((w) => w.lid !== 3).map((w) => ({ ...w, fresh: false })); writeScene(2, [], hid); p.present(); out.push(compare('after a window is hidden', p.readPixels().data, reference(hid)));
  // 5. the extended frame's inner box and a window region change (decision 0244): the description changes, no pixel is uploaded
  const before3 = p.stats.uploads; const reg = hid.map((w) => ({ ...w, g: w.lid === 6 ? [14, 10, 30, 14] : w.g, c: w.lid === 2 ? [3, 4, 20, 16] : w.lid === 7 ? [0, 0, 34, 26] : w.c }));
  writeScene(2, [], reg); p.present(); out.push({ name: 'a changed inner box and region upload nothing', uploads: p.stats.uploads - before3 }); out.push(compare('after the inner box and the region changed', p.readPixels().data, reference(reg)));
  const gl = canvas.getContext('webgl2'); void gl;
  return { renderer: (() => { const t = new OffscreenCanvas(1, 1).getContext('webgl2'); const e = t && t.getExtension('WEBGL_debug_renderer_info'); return e ? t.getParameter(e.UNMASKED_RENDERER_WEBGL) : null; })(), usableByThePlatformRule: DeskGl.webgl2Usable(() => new OffscreenCanvas(1, 1)), steps: out };
})()`, 120000);
fs.writeFileSync(path.join(OUT, 'result.json'), JSON.stringify(res, null, 1));
console.log(JSON.stringify(res, null, 1));
await b.close();
const bad = (res.steps ?? []).filter((s) => s.differing > 0 || (s.name.includes('upload') && s.name.includes('nothing') && s.uploads !== 0));
process.exit(bad.length ? 1 : 0);
