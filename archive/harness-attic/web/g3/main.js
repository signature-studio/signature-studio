'use strict';
// Page of the G3 round trip (decision 0136, PROVISIONAL). It owns the Worker (fle-g3-worker.js), shows each of the engine's top-level window surfaces on a canvas placed at the window's position on a 1920 x 1080
// "desktop" (the virtual screen: 1 CSS pixel = 1 screen pixel at device pixel ratio 1), and forwards the browser's REAL pointer and keyboard events to the Worker as raw events in screen coordinates (the Worker
// calls the engine's exports; the engine's runtime makes the Windows messages). window.__live is read by the runner through CDP; window.P.* are the runner's commands.
// Query: ?wasm=...&glue=...&data=... (URLs relative to this page), ?env=<JSON>, ?mode=fixed|wall, ?rate=<virtual ms per wall ms>.
const q = new URLSearchParams(location.search);
const P = window.P = {};
const desktop = document.getElementById('desktop');
const logEl = document.getElementById('log');
const L = window.__live = { seq: 0, stdout: 0, stderr: 0, last: [], lastMark: null, quit: null, jserrors: [], idles: 0, lastIdle: null, acks: [], pongs: [], inputs: [], snaps: 0, surfaces: 0, wakes: 0, advanceDone: 0 };
const stdout = [];
const canvases = new Map();     // lid -> {canvas, ctx, w, h, x, y, kind, sha}
const surfLog = [];
const t0 = performance.now();
window.__done = false;
let lastBeat = performance.now(), maxGap = 0, beats = 0;
setInterval(() => { const n = performance.now(); maxGap = Math.max(maxGap, n - lastBeat); lastBeat = n; beats++; }, 50);
const sha256hex = async (u8) => [...new Uint8Array(await crypto.subtle.digest('SHA-256', u8))].map((b) => b.toString(16).padStart(2, '0')).join('');
const w = new Worker('fle-g3-worker.js');
let seq = 0;
const waiters = [];
function waitFor(pred, ms) { return new Promise((res, rej) => { const rec = { pred, res, rej, t: setTimeout(() => { const i = waiters.indexOf(rec); if (i >= 0) waiters.splice(i, 1); rej(new Error('timeout waiting')); }, ms || 600000) }; waiters.push(rec); }); }
function offer(m) { for (let i = waiters.length - 1; i >= 0; i--) { const r = waiters[i]; if (r.pred(m)) { clearTimeout(r.t); waiters.splice(i, 1); r.res(m); } } }

w.onmessage = async (ev) => {
  const m = ev.data;
  L.seq++;
  if (m.type === 'stdout') { L.stdout++; stdout.push(m.line); }
  if (m.type === 'stderr') { L.stderr++; stdout.push('[err] ' + m.line); }
  if (m.type === 'stdout' || m.type === 'stderr') { L.last.push(m.line.slice(0, 200)); if (L.last.length > 6) L.last.shift(); logEl.textContent = L.last.join('\n'); }
  if (m.type === 'mark') L.lastMark = [m.name, m.t];
  if (m.type === 'quit') L.quit = { code: m.code, t: m.t };
  if (m.type === 'jserror' && L.jserrors.length < 20) L.jserrors.push(m.message);
  if (m.type === 'idle') { L.idles = m.n; L.lastIdle = { n: m.n, now: m.now, next: m.next, tWorker: m.t, tPage: +(performance.now() - t0).toFixed(2) }; }
  if (m.type === 'idleWake') L.wakes++;
  if (m.type === 'advanceDone') L.advanceDone++;
  if (m.type === 'ack') L.acks.push(m);
  if (m.type === 'pong') L.pongs.push({ seq: m.seq, rttMs: +(performance.now() - t0 - m.sentPageMs).toFixed(2), stage: m.stage });
  if (m.type === 'ready') {
    w.postMessage({ cmd: 'run', glue: new URL(q.get('glue') || '../dist/flengine6.js', location.href).href, wasm: new URL(q.get('wasm') || '../dist/flengine6.wasm', location.href).href,
      dataBase: new URL(q.get('data') || '../dist/data/', location.href).href, root: '/data', env: q.get('env') ? JSON.parse(q.get('env')) : {}, mode: q.get('mode') || 'fixed', rate: +(q.get('rate') || 1) });
  } else if (m.type === 'surface') {
    // one window's surface at a snapshot: the page draws it on the window's canvas and reads it back
    L.surfaces++;
    let c = canvases.get(m.lid);
    if (!c) {
      const canvas = document.createElement('canvas');
      canvas.dataset.lid = m.lid;
      desktop.appendChild(canvas);
      c = { canvas, ctx: canvas.getContext('2d', { willReadFrequently: true }) };
      canvases.set(m.lid, c);
    }
    if (c.canvas.width !== m.w || c.canvas.height !== m.h) { c.canvas.width = m.w; c.canvas.height = m.h; }
    const info = m.info || { x: 0, y: 0, flags: 0, style: 0 };
    c.canvas.style.left = info.x + 'px';
    c.canvas.style.top = info.y + 'px';
    c.canvas.style.zIndex = (info.style & 0x80000000) ? 2 : 1;       // an owned pop-up (style WS_POPUP) is above the main form
    c.canvas.style.display = (info.flags & 2) ? 'block' : 'none';
    c.info = info;
    c.ctx.putImageData(new ImageData(new Uint8ClampedArray(m.data), m.w, m.h), 0, 0);
    const rb = c.ctx.getImageData(0, 0, m.w, m.h);
    c.sha = await sha256hex(new Uint8Array(rb.data.buffer));
    c.sent = await sha256hex(new Uint8Array(m.data));
    c.w = m.w; c.h = m.h;
    surfLog.push({ lid: m.lid, w: m.w, h: m.h, x: info.x, y: info.y, visible: !!(info.flags & 2), sentSha256: c.sent, canvasReadbackSha256: c.sha, snap: L.snaps, tWorker: m.t });
  } else if (m.type === 'snapDone') {
    L.snaps++;
  } else if (m.type === 'done' || m.type === 'error') {
    window.__result = Object.assign({}, m, { beats, maxHeartbeatGapMs: +maxGap.toFixed(1), pageMs: +(performance.now() - t0).toFixed(1) });
    window.__stdout = stdout.concat(m.stdout || []);
    window.__done = true;
  }
  offer(m);
};
w.onerror = (ev) => { window.__result = { type: 'error', phase: 'worker.onerror', message: String(ev.message) }; window.__done = true; };
w.postMessage({ cmd: 'init' });

// ---- the real input of the page -> raw events in screen coordinates ----
// Mouse: the position in CSS pixels of the page = the screen pixel (the desktop is at (0, 0), DPR 1). Buttons 0/1/2 of PointerEvent = left/middle/right = FLE_IN_BTN 0/2/1.
const KIND = { move: 1, down: 2, up: 3, wheel: 4, kdown: 5, kup: 6 };
const BTN = (b) => (b === 2 ? 1 : b === 1 ? 2 : 0);
function sendMouse(kind, e, data) {
  const rec = { seq: seq++, kind, x: Math.floor(e.clientX), y: Math.floor(e.clientY), data, tPage: +(performance.now() - t0).toFixed(2), eventTimeStamp: +e.timeStamp.toFixed(2), isTrusted: e.isTrusted };
  L.inputs.push(rec);
  w.postMessage({ cmd: 'mouse', seq: rec.seq, kind, x: rec.x, y: rec.y, data, sentPageMs: rec.tPage });
}
desktop.addEventListener('pointermove', (e) => sendMouse(KIND.move, e, 0));
desktop.addEventListener('pointerdown', (e) => sendMouse(KIND.down, e, BTN(e.button)));
desktop.addEventListener('pointerup', (e) => sendMouse(KIND.up, e, BTN(e.button)));
desktop.addEventListener('wheel', (e) => { sendMouse(KIND.wheel, e, Math.round(-e.deltaY / Math.abs(e.deltaY || 1)) * 120); e.preventDefault(); }, { passive: false });
desktop.addEventListener('contextmenu', (e) => e.preventDefault());
// Keyboard: KeyboardEvent.code -> the Windows virtual-key code, the extended-key flag (winuser.h and the US keyboard: not on the Learn pages read, S-us-layout)
const VK = { Backspace: 8, Tab: 9, Enter: 13, NumpadEnter: 13, ShiftLeft: 0xA0, ShiftRight: 0xA1, ControlLeft: 0xA2, ControlRight: 0xA3, AltLeft: 0xA4, AltRight: 0xA5, Pause: 0x13, CapsLock: 0x14, Escape: 27, Space: 32,
  PageUp: 33, PageDown: 34, End: 35, Home: 36, ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, ArrowDown: 40, Insert: 45, Delete: 46, Semicolon: 0xBA, Equal: 0xBB, Comma: 0xBC, Minus: 0xBD, Period: 0xBE, Slash: 0xBF,
  Backquote: 0xC0, BracketLeft: 0xDB, Backslash: 0xDC, BracketRight: 0xDD, Quote: 0xDE, NumLock: 0x90, ScrollLock: 0x91 };
for (let i = 0; i < 26; i++) VK['Key' + String.fromCharCode(65 + i)] = 65 + i;
for (let i = 0; i < 10; i++) VK['Digit' + i] = 48 + i;
for (let i = 1; i <= 12; i++) VK['F' + i] = 0x6F + i;
const EXT = new Set(['ControlRight', 'AltRight', 'PageUp', 'PageDown', 'End', 'Home', 'ArrowLeft', 'ArrowUp', 'ArrowRight', 'ArrowDown', 'Insert', 'Delete', 'NumpadEnter']);
function sendKey(kind, e) {
  const vk = VK[e.code];
  if (!vk) return;
  const rec = { seq: seq++, kind, code: e.code, vk, repeat: e.repeat, tPage: +(performance.now() - t0).toFixed(2), isTrusted: e.isTrusted };
  L.inputs.push(rec);
  w.postMessage({ cmd: 'key', seq: rec.seq, kind, vk, scan: 0, ext: EXT.has(e.code) ? 1 : 0, sentPageMs: rec.tPage });
  e.preventDefault();
}
addEventListener('keydown', (e) => sendKey(KIND.kdown, e));
addEventListener('keyup', (e) => sendKey(KIND.kup, e));

// ---- commands of the runner ----
P.canvasRect = () => { const r = desktop.getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height }; };
P.mode = (mode, rate) => { w.postMessage({ cmd: 'mode', mode, rate: rate || 1 }); return true; };
P.advance = (ms) => { w.postMessage({ cmd: 'advance', seq: seq++, ms }); return true; };
P.snapshot = () => { w.postMessage({ cmd: 'snapshot', seq: seq++ }); return true; };
P.probeTimer = (lid, id, ms) => { w.postMessage({ cmd: 'probetimer', seq: seq++, lid, id, ms }); return true; };
P.timers = () => { w.postMessage({ cmd: 'timers', seq: seq++ }); return true; };
P.end = () => { w.postMessage({ cmd: 'end', seq: seq++ }); return true; };
P.ping = (n) => { w.postMessage({ cmd: 'ping', seq: 1000 + n, sentPageMs: performance.now() - t0 }); return true; };
P.surfaces = () => JSON.stringify([...canvases.entries()].map(([lid, c]) => ({ lid, w: c.w, h: c.h, x: c.info && c.info.x, y: c.info && c.info.y, visible: !!(c.info && (c.info.flags & 2)), sha: c.sha, sent: c.sent })));
P.surfLog = () => JSON.stringify(surfLog);
P.stdoutFrom = (i) => JSON.stringify(stdout.slice(i));
P.stdoutLen = () => stdout.length;
P.sha = async (lid) => { const c = canvases.get(lid); if (!c) return null; const rb = c.ctx.getImageData(0, 0, c.w, c.h); return sha256hex(new Uint8Array(rb.data.buffer)); };
window.__ready = true;
