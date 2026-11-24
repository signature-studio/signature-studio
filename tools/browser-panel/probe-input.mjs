// browser-panel probe-input (#1362): where the pointer and the keys go while the Browser's face (the iframe over the desktop) is shown. It records only.
// Usage: node tools/browser-panel/probe-input.mjs <absolute path to the bundle directory raw/<name>> > probe-input.json   (needs app/node_modules and app/dist, as boot-bundle.mjs does). Exit 1 when error is set, 2 when no bundle directory is given
// Order, on the engine's first idle (bootBundle's afterIdle): Escape (the Welcome window), Escape (the "Problems loading" box), then the tabs 35,125 and 267,125 (the cloud tab's Navigate, as measure.mjs does).
// Then the counters are set on #desktop (the element that pointer.ts listens on: pointerdown, keydown), on the document (capture), and on the face's own document. Each step counts the events it added:
//   control-pointer: a press on the desktop outside the face (guest 800,600). Expected: desktop +1, document +1, face 0.
//   control-key: a key on the focused element (the desktop after the press). Expected: desktop +1, face 0.
//   face-pointer: a press at the centre of the face. Expected: face +1, desktop 0, document 0 (the face's events do not reach the page).
//   face-key: a key while the face has the focus. Expected: face +1, desktop 0.
// hits: the element at the control point and at the face's centre (document.elementFromPoint, before the presses).
// stdout: ONE JSON line {state, error, steps: [{name, delta: {desktop, document, face}, faceRect}], face: {count, rect, loaded}, hits: {control, face}, logTail}.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { bootBundle } from '../boot-check/boot-bundle.mjs';
import { sleep } from '../boot-check/cdp.mjs';

const WAIT_MS = 500;
const OPEN_LIMIT_MS = 120000;
const POLL_MS = 100;
const FACE_WAIT_MS = 15000;
const TAB_SETTLE_MS = 3000;
const TABS = [{ x: 35, y: 125 }, { x: 267, y: 125 }];
/** a point on the desktop (CSS px) that is not the face: the first of the fractions of #desktop's box that the element there is inside #desktop and outside the face */
const CONTROL = `(() => { const d = document.getElementById('desktop').getBoundingClientRect(); for (const fx of [0.9, 0.75, 0.6, 0.5]) for (const fy of [0.9, 0.7, 0.5, 0.3]) { const x = d.x + d.width * fx, y = d.y + d.height * fy; const e = document.elementFromPoint(x, y); if (e && e.closest('#desktop') && !e.closest('iframe[data-fl-webview2]')) return { x, y, tag: e.tagName, id: e.id || null }; } return null; })()`;

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, title: document.title }; })()`;
const RECT = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height }; })()`;
const DIMS = `(async () => { const d = await window.__app.deskDumpBase64(true); return { w: d.width, h: d.height }; })()`;
const FACE = `(() => { const f = document.querySelector('iframe[data-fl-webview2]'); if (!f) return { count: document.querySelectorAll('iframe[data-fl-webview2]').length, rect: null, loaded: false }; const r = f.getBoundingClientRect(); const d = f.contentDocument; return { count: document.querySelectorAll('iframe[data-fl-webview2]').length, rect: { x: r.x, y: r.y, w: r.width, h: r.height }, loaded: !!(d && d.readyState === 'complete' && d.body) }; })()`;
const ATTACH = `(() => {
  const desk = document.getElementById('desktop');
  const frame = document.querySelector('iframe[data-fl-webview2]');
  const c = { desktop: 0, document: 0, face: 0 };
  window.__inputProbe = c;
  desk.addEventListener('pointerdown', () => { c.desktop++; }, true);
  desk.addEventListener('keydown', () => { c.desktop++; }, true);
  document.addEventListener('pointerdown', () => { c.document++; }, true);
  const fd = frame.contentDocument;
  fd.addEventListener('pointerdown', () => { c.face++; }, true);
  fd.addEventListener('keydown', () => { c.face++; }, true);
  return true;
})()`;
const COUNTS = `window.__inputProbe ? { ...window.__inputProbe } : null`;
const HIT = (c) => `(() => { const e = document.elementFromPoint(${c.x}, ${c.y}); if (!e) return null; return { tag: e.tagName, id: e.id || null, inDesktop: !!e.closest('#desktop'), inFace: !!e.closest('iframe[data-fl-webview2]') || e.tagName === 'IFRAME' }; })()`;
const strip = (l) => String(l).replace(/^\[err\] /, '');
const log = (...a) => process.stderr.write(a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ') + '\n');

/** the CSS point of a guest pixel on #desktop (the room is scaled to the picture, as probe-navigate.mjs maps it) */

/** the CSS point of a guest pixel on #desktop (the room is scaled to the picture, as probe-navigate.mjs maps it) */
const cssAt = (r, dims, p) => ({ x: r.x + (p.x + 0.5) * (r.w / dims.w), y: r.y + (p.y + 0.5) * (r.h / dims.h) });

async function mouse(page, type, c, button, buttons, clickCount) {
  await page.send('Input.dispatchMouseEvent', { type, x: c.x, y: c.y, button, buttons, clickCount, pointerType: 'mouse' });
}

async function pressAt(page, c) {
  await mouse(page, 'mouseMoved', c, 'none', 0, 0);
  await sleep(60);
  await mouse(page, 'mousePressed', c, 'left', 1, 1);
  await sleep(50);
  await mouse(page, 'mouseReleased', c, 'left', 0, 1);
}

async function key(page, name, vk) {
  await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
  await sleep(40);
  await page.send('Input.dispatchKeyEvent', { type: 'keyUp', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
}

/** one step: the input, then WAIT_MS; the counters' difference and the engine's input lines added are recorded */
async function step(page, steps, name, run) {
  const before = await page.evaluate(COUNTS);
  await run();
  await sleep(WAIT_MS);
  const after = await page.evaluate(COUNTS);
  const delta = { desktop: after.desktop - before.desktop, document: after.document - before.document, face: after.face - before.face };
  const face = await page.evaluate(FACE);
  const rec = { name, delta, faceRect: face.rect };
  steps.push(rec);
  log('step', rec);
}

async function session(page, steps, out) {
  const titleBefore = (await page.evaluate(STATE)).title;
  await key(page, 'Escape', 27);
  const t0 = Date.now();
  for (;;) {
    const s = await page.evaluate(STATE);
    if (s && (s.state === 'error' || s.state === 'exited')) throw new Error(`state ${s.state} after Escape`);
    if (s && s.state === 'engine-idle' && s.title !== titleBefore) break;
    if (Date.now() - t0 >= OPEN_LIMIT_MS) throw new Error('the project did not open within 120 s after Escape');
    await sleep(POLL_MS);
  }
  await sleep(2000);
  await key(page, 'Escape', 27);
  await sleep(2000);
  const rect = await page.evaluate(RECT);
  const dims = await page.evaluate(DIMS);
  for (const p of TABS) {
    await pressAt(page, cssAt(rect, dims, p));
    await sleep(TAB_SETTLE_MS);
  }
  for (let t = 0; t < FACE_WAIT_MS; t += POLL_MS) {
    const f = await page.evaluate(FACE);
    if (f.count >= 1 && f.loaded) break;
    await sleep(POLL_MS);
  }
  const face = await page.evaluate(FACE);
  out.face = face;
  if (face.count < 1 || !face.rect) throw new Error('the face (iframe[data-fl-webview2]) is not on the page');
  await page.evaluate(ATTACH);
  const center = { x: face.rect.x + face.rect.w / 2, y: face.rect.y + face.rect.h / 2 };
  const control = await page.evaluate(CONTROL);
  if (!control) throw new Error('no point of the desktop outside the face');
  out.hits = { control, face: await page.evaluate(HIT(center)) };
  await step(page, steps, 'control-pointer', () => pressAt(page, control));
  await step(page, steps, 'control-key', () => key(page, 'A', 65));
  await step(page, steps, 'face-pointer', () => pressAt(page, center));
  await step(page, steps, 'face-key', () => key(page, 'A', 65));
}

async function main() {
  if (!process.argv[2]) {
    process.stderr.write('usage: node tools/browser-panel/probe-input.mjs <absolute path to the bundle directory>\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(process.argv[2]);
  const steps = [];
  const out = { face: null };
  let ran = false;
  const { out: boot, engineLog } = await bootBundle(bundleDir, {
    afterIdle: async (page) => {
      ran = true;
      await session(page, steps, out);
    },
  }).catch((e) => ({ out: { state: null, standIn: null, error: String(e?.message ?? e).slice(0, 500) }, engineLog: null }));
  const error = boot.error ?? (ran ? null : `the session did not run: state ${boot.state}`);
  const tail = engineLog ? engineLog.slice(-8).map((l) => strip(l).slice(0, 200)) : [];
  process.stdout.write(JSON.stringify({ state: boot.state, error, steps, face: out.face, hits: out.hits ?? null, logTail: tail, nproc: os.availableParallelism(), loadavg: os.loadavg() }) + '\n');
  process.exitCode = error ? 1 : 0;
}

// the probe runs when this file is the program
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
