'use strict';
// perf1 (decision 0130, PROVISIONAL): a copy of ../fruity/main.js (decision 0128) with times since the page's navigation start (performance.now() of the page is that), the Worker's marks converted by
// absolute time, and the paint of canvas B timed (requestAnimationFrame after the putImageData). Query additions: ?streaming=1 ?cache=1 ?worker=<file>.
// Page of the FRUITY-on wasm in headless Chrome (decision 0128, PROVISIONAL). It owns the Worker (fle-fruity-worker.js), hands it canvas A as an OffscreenCanvas, and draws the frames that the Worker posts (method B)
// onto canvas B. window.__done / window.__result are read by run-chrome.mjs through CDP; window.P.sha('b') is the SHA-256 of canvas B's getImageData after the run.
// Query: ?wasm=...&glue=...&data=... (URLs relative to this page; the runner serves them), ?env=<JSON of extra environment>.
const q = new URLSearchParams(location.search);
const P = window.P = {};
const logEl = document.getElementById('log');
const canvasB = document.getElementById('b'), ctxB = canvasB.getContext('2d');
const frames = [];
const stdout = [];
let tick = 0;
window.__done = false;
// what a runner can read at any time (decision 0128): counters and the last events of the Worker; `seq` changes with every message from the Worker
window.__live = { seq: 0, stdout: 0, stderr: 0, last: [], lastMark: null, progress: null, quit: null, jserrors: [], frames: 0, workerErrors: [], idles: [], acks: [], pongs: [], inputs: [] };
const L = window.__live;
window.__ready = true;
const w = new Worker(q.get('worker') || 'fle-perf-worker.js');
const scriptStartNav = performance.now();
const workerCreatedNav = performance.now();
const offscreen = document.getElementById('a').transferControlToOffscreen();
const sha256hex = async (u8) => [...new Uint8Array(await crypto.subtle.digest('SHA-256', u8))].map((b) => b.toString(16).padStart(2, '0')).join('');
const t0 = performance.now();
const marks = [];
const markNav = [];
let lastBeat = performance.now(), maxGap = 0, beats = 0;
// the page's own event loop must turn while the engine runs (G3): a 50 ms heartbeat, the largest gap between two beats is recorded
setInterval(() => { const n = performance.now(); maxGap = Math.max(maxGap, n - lastBeat); lastBeat = n; beats++; }, 50);

w.onmessage = async (ev) => {
  const m = ev.data;
  L.seq++;
  if (m.type === 'stdout') L.stdout++;
  if (m.type === 'stderr') L.stderr++;
  if (m.type === 'stdout' || m.type === 'stderr') { L.last.push(m.line.slice(0, 200)); if (L.last.length > 8) L.last.shift(); }
  if (m.type === 'mark') { L.lastMark = [m.name, m.t]; markNav.push([m.name, +(m.abs - performance.timeOrigin).toFixed(1), m.bytes || null]); }
  if (m.type === 'progress') L.progress = { entries: m.entries, t: m.t };
  if (m.type === 'quit') L.quit = { code: m.code, err: m.err, t: m.t };
  if (m.type === 'jserror' && L.jserrors.length < 20) L.jserrors.push(m.message);
  if (m.type === 'frame') L.frames++;
  if (m.type === 'idle') L.idles.push({ n: m.n, entries: m.entries, tWorker: m.t, tPage: +(performance.now() - t0).toFixed(2) });
  if (m.type === 'inputAck') L.acks.push({ seq: m.seq, r: m.r, why: m.why || null, tWorker: m.t, tPage: +(performance.now() - t0).toFixed(2) });
  if (m.type === 'pong') L.pongs.push({ seq: m.seq, rttMs: +(performance.now() - m.sentPageMs - t0).toFixed(2), stage: m.stage });
  if (m.type === 'ready') {
    w.postMessage({ cmd: 'run', glue: new URL(q.get('glue') || '../dist/flengine6.js', location.href).href, wasm: new URL(q.get('wasm') || '../dist/flengine6.wasm', location.href).href,
      dataBase: new URL(q.get('data') || '../dist/data/', location.href).href, streaming: q.get('streaming') === '1', cacheapi: q.get('cacheapi') === 'gz' ? 'gz' : q.get('cacheapi') === '1', cache: q.get('cache') === '1', root: '/data', env: q.get('env') ? JSON.parse(q.get('env')) : {} });
    // G3: messages to the Worker while the engine runs; the Worker records when its event loop delivers them (see fle-fruity-worker.js)
    let seq = 0;
    for (const at of [5000, 15000, 60000]) setTimeout(() => { if (!window.__done) w.postMessage({ cmd: 'ping', seq: seq++, sentPageMs: +(performance.now() - t0).toFixed(1) }); }, at);
  } else if (m.type === 'stdout' || m.type === 'stderr') {
    stdout.push((m.type === 'stderr' ? '[err] ' : '') + m.line);
    if (stdout.length < 400) logEl.textContent += (m.type === 'stderr' ? '[err] ' : '') + m.line + '\n';
  } else if (m.type === 'mark') {
    marks.push([m.name, m.t]);
  } else if (m.type === 'frame') {
    // method B: the RGBA bytes of the window's surface at an engine event; the page draws them and reads canvas B back at once
    const rgba = new Uint8ClampedArray(m.data);
    if (canvasB.width !== m.w || canvasB.height !== m.h) { canvasB.width = m.w; canvasB.height = m.h; }
    const navRecv = performance.now();
    ctxB.putImageData(new ImageData(rgba, m.w, m.h), 0, 0);
    const navDrawn = performance.now();
    const rb = ctxB.getImageData(0, 0, m.w, m.h);
    const fr = { navWorkerEvent: +(m.abs - performance.timeOrigin).toFixed(1), navRecv: +navRecv.toFixed(1), navDrawn: +navDrawn.toFixed(1), navRaf: null };
    requestAnimationFrame(() => { fr.navRaf = +performance.now().toFixed(1); });
    frames.push({ nav: fr, kind: m.kind, lid: m.lid, main: m.main, w: m.w, h: m.h, tWorker: m.t, tPage: +(performance.now() - t0).toFixed(1),
      sentSha256: await sha256hex(new Uint8Array(m.data)), canvasB_readback_sha256: await sha256hex(new Uint8Array(rb.data.buffer)) });
  } else if (m.type === 'done' || m.type === 'error') {
    const nav = performance.getEntriesByType('navigation')[0];
    window.__result = Object.assign({}, m, { frames, beats, maxHeartbeatGapMs: +maxGap.toFixed(1), pageMs: +(performance.now() - t0).toFixed(1), marksPage: marks, markNav, doneNav: +performance.now().toFixed(1),
      pageTimeOrigin: performance.timeOrigin, scriptStartNav: +scriptStartNav.toFixed(1), workerCreatedNav: +workerCreatedNav.toFixed(1),
      navTiming: nav ? { domContentLoaded: +nav.domContentLoadedEventEnd.toFixed(1), load: +nav.loadEventEnd.toFixed(1), responseEnd: +nav.responseEnd.toFixed(1), transferSize: nav.transferSize } : null });
    delete window.__result.stdout;
    window.__stdout = stdout.concat(m.stdout || []);
    window.__done = true;
  }
};
w.onerror = (ev) => { L.workerErrors.push(String(ev.message)); window.__result = { type: 'error', phase: 'worker.onerror', message: String(ev.message) }; window.__done = true; };
w.onmessageerror = (ev) => { L.workerErrors.push('messageerror'); };
w.postMessage({ cmd: 'init', canvasA: offscreen }, [offscreen]);

// G3 (decision 0128): real browser input on canvas A is forwarded to the engine's Worker (pointer events of the page; the runner makes them with CDP Input.dispatchMouseEvent, which is a browser-level input event, not a call into the page).
// 'main' is the main form's surface (the Worker looks the window up by its class, decision 0150: mainLidOf); the position is in surface (= client) pixels. WM_MOUSEMOVE = 0x200, lParam = (y << 16) | x.
let inputSeq = 0;
const canvasA = document.getElementById('a');
canvasA.addEventListener('pointermove', (e) => {
  const x = Math.max(0, Math.min(1929, Math.floor(e.offsetX))), y = Math.max(0, Math.min(1091, Math.floor(e.offsetY)));
  const rec = { seq: inputSeq++, x, y, tPage: +(performance.now() - t0).toFixed(2), eventTimeStamp: +e.timeStamp.toFixed(2) };
  L.inputs.push(rec);
  w.postMessage({ cmd: 'input', seq: rec.seq, lid: 'main', msg: 0x200, wp: 0, lp: ((y << 16) | x) >>> 0, sentPageMs: rec.tPage });
});
// buttons: WM_LBUTTONDOWN 0x201 / WM_LBUTTONUP 0x202, wParam MK_LBUTTON = 1 on down, 0 on up (real pointer events of the page; the runner uses CDP Input.dispatchMouseEvent)
canvasA.addEventListener('pointerdown', (e) => {
  if (e.button !== 0) return;
  const x = Math.max(0, Math.min(1929, Math.floor(e.offsetX))), y = Math.max(0, Math.min(1091, Math.floor(e.offsetY)));
  const rec = { seq: inputSeq++, kind: 'down', x, y, tPage: +(performance.now() - t0).toFixed(2) };
  L.inputs.push(rec);
  w.postMessage({ cmd: 'input', seq: rec.seq, lid: 'main', msg: 0x201, wp: 1, lp: ((y << 16) | x) >>> 0, sentPageMs: rec.tPage });
});
canvasA.addEventListener('pointerup', (e) => {
  if (e.button !== 0) return;
  const x = Math.max(0, Math.min(1929, Math.floor(e.offsetX))), y = Math.max(0, Math.min(1091, Math.floor(e.offsetY)));
  const rec = { seq: inputSeq++, kind: 'up', x, y, tPage: +(performance.now() - t0).toFixed(2) };
  L.inputs.push(rec);
  w.postMessage({ cmd: 'input', seq: rec.seq, lid: 'main', msg: 0x202, wp: 0, lp: ((y << 16) | x) >>> 0, sentPageMs: rec.tPage });
});
P.canvasRect = () => { const r = canvasA.getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height }; };
P.ping = (seq) => { const s = performance.now() - t0; w.postMessage({ cmd: 'ping', seq: 1000 + seq, sentPageMs: s }); return new Promise((res) => { const h = setInterval(() => { const p = L.pongs.find((q) => q.seq === 1000 + seq); if (p) { clearInterval(h); res(p); } }, 5); }); };
P.end = () => { w.postMessage({ cmd: 'end' }); return true; };
// SHA-256 of what canvas B holds now (the page can read it: it never transferred control of canvas B)
P.shaB = async () => { const d = ctxB.getImageData(0, 0, canvasB.width, canvasB.height); return { w: canvasB.width, h: canvasB.height, sha256: await sha256hex(new Uint8Array(d.data.buffer)) }; };
