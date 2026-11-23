// browser-panel probe-navigate (#1355): the FL-side trigger of WebView2's Navigate, by the operations a user makes. It records only: the WebView2 lines of each step (the engine's log since the step began) and, optionally, a PNG of the Browser's area after each picture step.
// Usage: node tools/browser-panel/probe-navigate.mjs <absolute path to the bundle directory raw/<name>> [<png prefix>] > probe.json   (needs app/node_modules and app/dist, as boot-bundle.mjs does). Exit 1 when error is set, 2 when no bundle directory is given.
// Order, on the engine's first idle (bootBundle's afterIdle). Every step waits WAIT_MS after its input, then reads the log since the step began:
//   idle: no input for 30 s (timers, the news download);
//   escape: Escape closes the Welcome window (flow B of tools/smoke/welcome-flow.mjs), and the project opens when the title has changed (waited up to 120 s);
//   dismiss: Escape closes the "Problems loading the project" box that the project's opening shows (it covers the Browser);
//   shot-a: a PNG of the Browser's area (guest x 0..399, y 0..1055) when <png prefix> is given (no input);
//   tabs: one click per point of TABS (guest px), each with its own step; shot-b: the PNG after the tabs;
//   f8: F8 (the Browser panel is shown or hidden); shot-c: the PNG after F8.
// stdout: ONE JSON line {state, standIn, error, steps: [{name, state (after the step), input, webview2Lines, navigateCalls, navigateLines, fired, lines (the first 12 webview2 lines of the step)}], logTail}. input: the number of the engine's `input:` lines of the step (0 in every run so far: the engine does not write them in this start-up, so a click is seen by its effect, the WebView2 lines and the pictures, not by this count). navigateCalls: the lines of Navigate, NavigateToString, ExecuteScript, PostWebMessageAsJson and PostWebMessageAsString. fired: the events of the fire lines (their first appearance).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { bootBundle } from '../boot-check/boot-bundle.mjs';
import { sleep } from '../boot-check/cdp.mjs';

const WAIT_MS = 2000;
const IDLE_MS = 30000;
const OPEN_LIMIT_MS = 120000;
const POLL_MS = 500;
// the icons of the Browser's tab row (guest px, from the picture of the run: the row at y 125; the selected one is the orange cloud at x 267)
const TABS = [{ x: 35, y: 125 }, { x: 91, y: 125 }, { x: 151, y: 125 }, { x: 208, y: 125 }, { x: 267, y: 125 }, { x: 325, y: 125 }];
const NAVIGATE = new Set(['Navigate', 'NavigateToString', 'ExecuteScript', 'PostWebMessageAsJson', 'PostWebMessageAsString']);
const WEBVIEW2_LINE = /^webview2: (\S+)/;
const WEBVIEW2_FIRE = /^webview2: fire (\S+)/;
const BROWSER_BOX = { x: 0, y: 0, w: 400, h: 1056 };

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, title: document.title, standIn: document.body.innerText.includes('STAND-IN') }; })()`;
const PICTURE = `window.__app.deskDumpBase64(true)`;
const RECT = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height }; })()`;
const DIMS = `(async () => { const d = await window.__app.deskDumpBase64(true); return { w: d.width, h: d.height }; })()`;
const LOG_LEN = `window.__app.engineLog().length`;
const LOG_FROM = (n) => `window.__app.engineLog().slice(${n})`;
const strip = (l) => String(l).replace(/^\[err\] /, '');
const log = (...a) => process.stderr.write(a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ') + '\n');

/** a PNG (RGBA, 8 bits) of the box of a picture dump: rows of the dump may be longer than width x 4 */
function writePng(file, d, box) {
  const bytes = Buffer.from(d.base64, 'base64');
  const stride = bytes.length / d.height;
  const w = Math.min(box.w, d.width - box.x), h = Math.min(box.h, d.height - box.y);
  const rowLen = w * 4 + 1;
  const raw = Buffer.alloc(rowLen * h);
  for (let y = 0; y < h; y++) bytes.copy(raw, y * rowLen + 1, (box.y + y) * stride + box.x * 4, (box.y + y) * stride + (box.x + w) * 4);
  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(zlib.crc32(td) >>> 0);
    return Buffer.concat([len, td, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  fs.writeFileSync(file, Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]));
}

/** the CSS point of a guest pixel on the box of #desktop (as feat-c6-popup-menus/measure.mjs maps it) */
const cssAt = (r, pic, p) => ({ x: r.x + (p.x + 0.5) * (r.w / pic.w), y: r.y + (p.y + 0.5) * (r.h / pic.h) });

async function mouse(page, type, c, button, buttons, clickCount) {
  await page.send('Input.dispatchMouseEvent', { type, x: c.x, y: c.y, button, buttons, clickCount, pointerType: 'mouse' });
}

async function clickAt(page, p) {
  const c = cssAt(await page.evaluate(RECT), await page.evaluate(DIMS), p);
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

async function picture(page) {
  return page.evaluate(PICTURE);
}

/** one step: the input (run), then WAIT_MS; the WebView2 lines since the step began are recorded */
async function step(page, steps, name, run) {
  const from = await page.evaluate(LOG_LEN);
  await run();
  await sleep(WAIT_MS);
  const span = (await page.evaluate(LOG_FROM(from))).map(strip);
  const wv = span.filter((l) => WEBVIEW2_LINE.test(l));
  const nav = wv.filter((l) => NAVIGATE.has(WEBVIEW2_LINE.exec(l)[1]));
  const fired = [...new Set(wv.filter((l) => WEBVIEW2_FIRE.test(l)).map((l) => WEBVIEW2_FIRE.exec(l)[1]))];
  const st = await page.evaluate(STATE);
  const rec = { name, state: st ? st.state : null, input: span.filter((l) => /^input: /.test(l)).length, webview2Lines: wv.length, navigateCalls: nav.length, navigateLines: nav.slice(0, 5).map((l) => l.slice(0, 240)), fired, lines: wv.slice(0, 12).map((l) => l.slice(0, 240)) };
  steps.push(rec);
  log('step', rec.name, { state: rec.state, input: rec.input, webview2Lines: rec.webview2Lines, navigateCalls: rec.navigateCalls, fired: rec.fired });
}

async function session(page, steps, pngPrefix) {
  await page.evaluate('window.__app.windowApiTrace(true)');
  await step(page, steps, 'idle30', () => sleep(IDLE_MS));
  await step(page, steps, 'escape', async () => {
    // flow B of tools/smoke/welcome-flow.mjs: Escape closes the Welcome window, and the project opens when the title has changed
    const before = await page.evaluate(STATE);
    await key(page, 'Escape', 27);
    const t0 = Date.now();
    for (;;) {
      const s = await page.evaluate(STATE);
      if (s && (s.state === 'error' || s.state === 'exited')) {
        const n = await page.evaluate(LOG_LEN);
        for (const l of (await page.evaluate(LOG_FROM(Math.max(0, n - 60)))).map(strip)) log('tail', l.slice(0, 300));
        throw new Error(`state ${s.state} after Escape`);
      }
      if (s && s.state === 'engine-idle' && s.title !== before.title) break;
      if (Date.now() - t0 >= OPEN_LIMIT_MS) throw new Error('the project did not open within 120 s after Escape');
      await sleep(POLL_MS);
    }
    await sleep(2000);
  });
  await step(page, steps, 'dismiss', () => key(page, 'Escape', 27));
  if (pngPrefix) writePng(`${pngPrefix}-a.png`, await picture(page), BROWSER_BOX);
  for (const p of TABS) await step(page, steps, `tab ${p.x},${p.y}`, () => clickAt(page, p));
  if (pngPrefix) writePng(`${pngPrefix}-b.png`, await picture(page), BROWSER_BOX);
  await step(page, steps, 'f8', () => key(page, 'F8', 119));
  if (pngPrefix) writePng(`${pngPrefix}-c.png`, await picture(page), BROWSER_BOX);
}

async function main() {
  if (!process.argv[2]) {
    process.stderr.write('usage: node tools/browser-panel/probe-navigate.mjs <absolute path to the bundle directory> [<png prefix>]\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(process.argv[2]);
  const pngPrefix = process.argv[3] ? path.resolve(process.argv[3]) : null;
  const steps = [];
  let ran = false;
  const { out, engineLog } = await bootBundle(bundleDir, {
    afterIdle: async (page) => {
      ran = true;
      await session(page, steps, pngPrefix);
    },
  }).catch((e) => ({ out: { state: null, standIn: null, error: String(e?.message ?? e).slice(0, 500) }, engineLog: null }));
  const error = out.error ?? (ran ? null : `the session did not run: state ${out.state}`);
  const tail = engineLog ? engineLog.slice(-12).map((l) => strip(l).slice(0, 200)) : [];
  process.stdout.write(JSON.stringify({ state: out.state, standIn: out.standIn, error, steps, logTail: tail, nproc: os.availableParallelism(), loadavg: os.loadavg() }) + '\n');
  process.exitCode = error ? 1 : 0;
}

// the probe runs when this file is the program
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
