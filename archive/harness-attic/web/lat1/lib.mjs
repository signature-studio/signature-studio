// lat1 (decision 0194) shared code: a vite preview of our own build, an isolated Chrome, the injected hooks, input helpers, collection and the join of the records.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import net from 'node:net';
import { PAGE_HOOK, installWorkerHook } from './hooks.mjs';

import { fileURLToPath } from 'node:url';
// LAT1_ROOT: the repository (default: four levels above this file); LAT1_APP: the app tree (default <root>/app); LAT1_DIST: its vite build (default <app>/dist); LAT1_ASSETS: the bundles (read only)
export const ROOT = process.env.LAT1_ROOT ?? fileURLToPath(new URL('../../../../', import.meta.url)).replace(/\/$/, '');
export const SRC = process.env.LAT1_APP ?? `${ROOT}/app`;
export const ASSETS = process.env.LAT1_ASSETS ?? `${ROOT}/lift/out-websrv/assets`;
export const DIST = process.env.LAT1_DIST ?? `${SRC}/dist`;
const { start } = await import(`${SRC}/test/helpers/proc.mjs`);
const { openBrowser, sleep } = await import(`${SRC}/test/e2e/browser.mjs`);
export { sleep };

export const say = (s) => console.log(`${new Date().toISOString()} ${s}`);
export const load1 = () => os.loadavg()[0];
const portFree = (p) => new Promise((res) => { const s = net.createServer(); s.once('error', () => res(false)); s.listen(p, '127.0.0.1', () => s.close(() => res(true))); });

/** one vite preview of our own build (dist-head = git archive HEAD, vite build) serving the default entry's assets read-only */
export async function openServer(out, basePort = 4410) {
  let port = basePort;
  while (!(await portFree(port))) port++;
  const env = { ...process.env, FL_ASSETS_DIR: ASSETS, FL_PERF_LOG: path.join(out, 'perf-log.jsonl') };
  const server = start(path.join(SRC, 'node_modules/.bin/vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort', '--outDir', DIST], { cwd: SRC, env, ready: /Local:\s+http/, label: 'vite preview' });
  await server.ready;
  return { port, stop: () => server.stop() };
}

const KEYS = {
  Space: { key: ' ', vk: 32, text: ' ', code: 'Space' }, Enter: { key: 'Enter', vk: 13, text: '\r', code: 'Enter' }, Escape: { key: 'Escape', vk: 27, code: 'Escape' },
  F8: { key: 'F8', vk: 119, code: 'F8' }, Left: { key: 'ArrowLeft', vk: 37, code: 'ArrowLeft' }, Right: { key: 'ArrowRight', vk: 39, code: 'ArrowRight' },
  Up: { key: 'ArrowUp', vk: 38, code: 'ArrowUp' }, Down: { key: 'ArrowDown', vk: 40, code: 'ArrowDown' }, F7: { key: 'F7', vk: 118, code: 'F7' },
};

/** an isolated Chrome with one page, the page hook, the worker hook; the URL is opened by session.open() */
export async function openSession(out, name, hook = PAGE_HOOK) {
  fs.mkdirSync(out, { recursive: true });
  const browser = await openBrowser(out, name);
  const page = await browser.newPage({ width: 2400, height: 1300, initScript: hook });
  const hookState = await installWorkerHook(page, browser);
  let rect = null;
  const refreshRect = async () => { rect = JSON.parse(await page.ev("JSON.stringify(document.getElementById('desktop').getBoundingClientRect().toJSON())")); S.rect = rect; return rect; };
  const toClient = (sx, sy) => ({ x: rect.x + (sx + 0.5) * (rect.width / 1920), y: rect.y + (sy + 0.5) * (rect.height / 1080) });
  const S = {
    page, browser, hookState, refreshRect,
    async mouse(type, sx, sy, o = {}) { const c = toClient(sx, sy); await page.S('Input.dispatchMouseEvent', { type, x: c.x, y: c.y, button: 'none', buttons: 0, clickCount: 0, pointerType: 'mouse', ...o }); },
    move: (sx, sy) => S.mouse('mouseMoved', sx, sy),
    down: (sx, sy, button = 'left') => S.mouse('mousePressed', sx, sy, { button, buttons: button === 'left' ? 1 : 2, clickCount: 1 }),
    up: (sx, sy, button = 'left') => S.mouse('mouseReleased', sx, sy, { button, buttons: 0, clickCount: 1 }),
    dragMove: (sx, sy) => S.mouse('mouseMoved', sx, sy, { button: 'left', buttons: 1 }),
    wheel: (sx, sy, dy) => S.mouse('mouseWheel', sx, sy, { deltaX: 0, deltaY: dy }),
    async keyDown(k) { const e = KEYS[k]; await page.S('Input.dispatchKeyEvent', { type: e.text ? 'keyDown' : 'rawKeyDown', code: e.code, key: e.key, windowsVirtualKeyCode: e.vk, nativeVirtualKeyCode: e.vk, ...(e.text ? { text: e.text } : {}) }); },
    async keyUp(k) { const e = KEYS[k]; await page.S('Input.dispatchKeyEvent', { type: 'keyUp', code: e.code, key: e.key, windowsVirtualKeyCode: e.vk, nativeVirtualKeyCode: e.vk }); },
    async tap(k, hold = 40) { await S.keyDown(k); await sleep(hold); await S.keyUp(k); },
    async open(url) { await page.goto(url); },
    async waitIdle(ms = 120000) { await page.waitFor("window.__app && (window.__app.state === 'engine-idle' || window.__app.state === 'error' || window.__app.state === 'exited')", ms, 'idle'); const st = await page.ev('window.__app.state'); if (st !== 'engine-idle') throw new Error('state ' + st); },
    /** Enter closes the Welcome wizard (the engine takes input only after its first idle); retried until the main window is the only visible one */
    async closeWizard() {
      await sleep(3500);
      for (let i = 0; i < 6; i++) {
        await S.tap('Enter'); await sleep(2500);
        const w = await page.ev("JSON.stringify((window.__app.appView?.windows ?? []).filter(w=>w.visible).map(w=>w.className))");
        if (!JSON.parse(w).includes('TWelcomeWizard')) return JSON.parse(w);
      }
      throw new Error('the wizard did not close');
    },
    async settle(ms = 3000, quietMs = 250, bg = null) {
      // quiet = no CHANGING draw outside the background classes `bg` (keys "lid|x|y|h") for quietMs, and nothing pending in the tracker
      const bgJson = JSON.stringify(bg ? [...bg] : []);
      const t0 = Date.now();
      for (;;) {
        const r = JSON.parse(await page.ev(`JSON.stringify((() => { const B = new Set(${bgJson}); return { pend: window.__app.latency.pending, last: (() => { const p = window.__lat1.puts; for (let i = p.length - 1; i >= 0; i--) if (p[i].changed && !B.has(p[i].lid + '|' + p[i].x + '|' + p[i].y + '|' + p[i].h)) return p[i].tEnd; return 0; })(), now: performance.timeOrigin + performance.now() }; })())`));
        if (r.pend === 0 && r.now - r.last > quietMs) return true;
        if (Date.now() - t0 > ms) return false;
        await sleep(40);
      }
    },
    async reset() {
      await page.ev('(() => { const L = window.__lat1; L.dom.length = 0; L.msgs.length = 0; L.puts.length = 0; L.workerDumps.length = 0; window.__app.latency.records.length = 0; return 1; })()');
    },
    async collect() {
      await page.ev("window.__lat1.worker.postMessage({ cmd: 'lat1-dump' })");
      await page.waitFor('window.__lat1.workerDumps.length > 0', 20000, 'worker dump');
      const j = JSON.parse(await page.ev(`JSON.stringify({ dom: window.__lat1.dom, puts: window.__lat1.puts, wd: window.__lat1.workerDumps, recs: window.__app.latency.records.map(r => ({ seq: r.seq, desc: r.desc, kind: r.kind, from: r.from, tPage: r.tPage, tInject: r.tInject, tIdle: r.tIdle, dropped: r.dropped })), now: performance.timeOrigin + performance.now(), hook: { hashMs: window.__lat1.hashMs, nputs: window.__lat1.nputs } })`));
      const wd = { recv: j.wd.flatMap((d) => d.recv), posts: j.wd.flatMap((d) => d.posts) };
      return { dom: j.dom, puts: j.puts, recs: j.recs, recv: wd.recv, posts: wd.posts, now: j.now, hook: j.hook };
    },
  };
  return S;
}

/**
 * Joins the records of one stretch into per-input timelines. All values in epoch ms.
 * `isResponse(put)` says which draws count as the answer (default: every draw that changed a pixel); the draw must be posted by the Worker at or after the input's injection.
 * An input's answer window ends at the next input's injection only when `separate` (isolated inputs); for streams (drags) each input takes the first response after its own injection.
 */
export function join(c, { isResponse = (p) => p.changed > 0, horizonMs = 1500 } = {}) {
  const recvBySeq = new Map(c.recv.map((r) => [r.seq, r]));
  const recs = c.recs.filter((r) => r.tInject !== null && r.dropped === null).sort((a, b) => a.seq - b.seq);
  const resp = c.puts.filter((p) => p.msg && isResponse(p)).sort((a, b) => a.msg.postedAt - b.msg.postedAt);
  const postByPostedAt = new Map(c.posts.filter((p) => p.t === 'frame').map((p) => [p.postedAt, p]));
  const out = [];
  for (const r of recs) {
    const rv = recvBySeq.get(r.seq);
    // the DOM event: the last capture-phase event whose handler ran at or before tPage (within 30 ms)
    let dom = null;
    for (let i = c.dom.length - 1; i >= 0; i--) { const d = c.dom[i]; if (d.t <= r.tPage + 0.05 && r.tPage - d.t < 30) { dom = d; break; } }
    const first = resp.find((p) => p.msg.postedAt >= r.tInject && p.msg.postedAt - r.tInject < horizonMs) ?? null;
    const o = { seq: r.seq, desc: r.desc, tEvent: dom ? dom.ts : null, tHandler: dom ? dom.t : null, tPage: r.tPage, tRecv: rv ? rv.t : null, tInject: r.tInject, tIdle: r.tIdle, first: null };
    if (first) {
      const wp = postByPostedAt.get(first.msg.postedAt);
      o.first = { lid: first.lid, rect: [first.x, first.y, first.w, first.h], changedPx: first.changed, wPostA: wp ? wp.a : first.msg.postedAt, wPostB: wp ? wp.b : null, pRecv: first.msg.tRecv, pPut: first.tEnd, pRaf: first.raf, pRafDone: first.rafDone };
    }
    out.push(o);
  }
  return out;
}

export const ms1 = (x) => (x === null || x === undefined || Number.isNaN(x) ? null : +x.toFixed(2));
/** the stage durations of one joined input */
export function stages(o) {
  const t0 = o.tEvent ?? o.tHandler ?? o.tPage;
  const f = o.first;
  return {
    s0_evt_to_handler: o.tEvent !== null && o.tHandler !== null ? o.tHandler - o.tEvent : null,
    s0b_handler_to_page_send: o.tHandler !== null ? o.tPage - o.tHandler : null,
    s1_page_to_worker_recv: o.tRecv !== null ? o.tRecv - o.tPage : null,
    s1b_worker_recv_to_inject: o.tRecv !== null ? o.tInject - o.tRecv : null,
    s2_inject_to_first_post: f ? f.wPostA - o.tInject : null,
    s2b_inject_to_idle: o.tIdle !== null ? o.tIdle - o.tInject : null,
    s3a_post_to_page_recv: f ? f.pRecv - f.wPostA : null,
    s3b_recv_to_put: f ? f.pPut - f.pRecv : null,
    s4a_put_to_raf: f && f.pRaf !== null ? f.pRaf - f.pPut : null,
    s4b_raf_to_rafdone: f && f.pRaf !== null && f.pRafDone !== null ? f.pRafDone - f.pRaf : null,
    total_event_to_put: f ? f.pPut - t0 : null,
    total_event_to_raf: f && f.pRaf !== null ? f.pRaf - t0 : null,
    total_event_to_rafdone: f && f.pRafDone !== null ? f.pRafDone - t0 : null,
  };
}
export function pct(v, p) { v = v.filter((x) => x !== null && x !== undefined).sort((a, b) => a - b); if (!v.length) return null; return v[Math.min(v.length - 1, Math.max(0, Math.ceil((p / 100) * v.length) - 1))]; }
