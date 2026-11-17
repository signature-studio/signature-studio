// The opener page: it owns the engine worker (as the real design would: the audio and the wasm live here) and relays between the worker and
// the popups. window.D is the test API that run-detach.mjs drives over CDP. Nothing here is FL code.
'use strict';
const out = document.getElementById('o');
const log = (s) => { out.textContent += s + '\n'; (window.__log = window.__log || []).push(s); };
const now = () => performance.timeOrigin + performance.now();
const pct = (a, qs = [50, 95, 99]) => { const s = Float64Array.from(a).sort(); const o = { n: s.length }; if (!s.length) return o; o.min = s[0]; o.max = s[s.length - 1]; o.mean = s.reduce((x, y) => x + y, 0) / s.length; for (const q of qs) o['p' + q] = s[Math.min(s.length - 1, Math.floor(q / 100 * s.length))]; return o; };
const D = window.D = { worker: null, popups: new Map(), reports: new Map(), via: new Map(), relayed: 0, closedIds: [], workerReport: null, mainLat: [], ring: null, sab: null, events: [], gens: [] };
const RING = 256;

D.init = (mode) => new Promise((resolve) => {
  D.mode = mode;
  D.worker = new Worker('engine-worker.js');
  D.worker.onmessage = (e) => onWorker(e.data);
  const msg = { t: 'init', mode };
  if (mode === 'block') {
    if (typeof SharedArrayBuffer === 'undefined') { resolve({ error: 'no SharedArrayBuffer (page is not cross-origin isolated)' }); return; }
    D.sab = new SharedArrayBuffer(16 + RING * 4 * 8);
    D.sabIn = new SharedArrayBuffer(16 + RING * 4 * 8);      // popup -> worker input ring (single producer: the popup)
    msg.sabIn = D.sabIn;
    D.bell = new SharedArrayBuffer(16); D.bellI = new Int32Array(D.bell); msg.bell = D.bell;
    D.ctl = new Int32Array(D.sab, 0, 4); D.slots = new Float64Array(D.sab, 16, RING * 4); D.wr = 0;
    msg.sab = D.sab; msg.ring = RING;
  }
  D.initResolve = resolve;
  D.worker.postMessage(msg);
});

// the ring is the only way in once the worker is inside Atomics.wait (mode 'block')
function ringPost(kind, id, seq, t0) {
  const o = (D.wr % RING) * 4; D.slots[o] = seq; D.slots[o + 1] = id; D.slots[o + 2] = t0; D.slots[o + 3] = kind;
  D.wr++; Atomics.store(D.ctl, 0, D.wr); Atomics.add(D.bellI, 0, 1); Atomics.notify(D.bellI, 0);
}

function toWorker(m, tr) { D.worker.postMessage(m, tr || []); }

D.pips = new Map();
function onWorker(m) {
  const pip = m.id !== undefined && D.pips.get(m.id);
  if (pip) {
    if (m.t === 'frame') { const t = now(); if (pip.last) pip.gaps.push(t - pip.last); pip.last = t; pip.lat.push(t - m.tGen); pip.frames++; pip.ctx.transferFromImageBitmap(m.bmp); return; }
    if (m.t === 'ack') { pip.acks++; pip.rtt.push(now() - m.t0); return; }
  }
  if (m.t === 'inited') { D.coi = m.coi; D.initResolve({ ok: true, sab: m.sab, coi: m.coi }); return; }
  if (m.t === 'frame') {                       // relay path: worker -> opener -> popup
    D.relayed++;
    const p = D.popups.get(m.id);
    if (p && !p.closed) p.postMessage({ t: 'frame', id: m.id, frame: m.frame, tGen: m.tGen, bmp: m.bmp }, location.origin, [m.bmp]); else m.bmp.close();
  } else if (m.t === 'ack') {                  // input reply for the relay path
    const p = D.popups.get(m.id);
    if (p && !p.closed) p.postMessage(m, location.origin);
  } else if (m.t === 'stopped') { D.workerReport = m; D.stopResolve && D.stopResolve(m); }
}

// popup -> opener
window.addEventListener('message', (e) => {
  if (e.origin !== location.origin) return;
  const m = e.data;
  if (!m || !m.t) return;
  D.events.push({ t: m.t, id: m.id, at: now() });
  if (m.t === 'hello') {
    if (D.sab && D.popups.get(m.id)) { try { D.popups.get(m.id).postMessage({ t: 'sab', sab: D.sabIn, ring: RING, bell: D.bell }, location.origin); D.sabSend = 'sent after hello'; } catch (e) { D.sabSend = String(e.name + ': ' + e.message).slice(0, 160); } }
    D.hello = D.hello || {}; D.hello[m.id] = m; D.helloWait && D.helloWait[m.id] && D.helloWait[m.id](m); }
  else if (m.t === 'in') {                     // popup input arrives on the main thread, then goes to the worker
    const via = D.via.get(m.id);
    if (D.mode === 'block') ringPost(1, m.id, m.seq, m.t0);
    else toWorker(m);
  } else if (m.t === 'canvas') {               // OffscreenCanvas from the popup, handed on to the worker
    toWorker({ t: 'addWin', id: m.id, via: 'canvas', canvas: m.canvas }, [m.canvas]);
  } else if (m.t === 'popup-report') { D.reports.set(m.id, m); D.reportWait && D.reportWait[m.id] && D.reportWait[m.id](m); }
  else if (m.t === 'closed') { D.closedIds.push({ id: m.id, at: now() }); }
  else if (m.t === 'log') log('popup ' + m.id + ': ' + m.msg);
});

// open a popup. Needs transient user activation (the caller makes one with a CDP-injected click first).
D.open = (id, via, opts = {}) => {
  const w = opts.w || 512, h = opts.h || 384;
  const feat = opts.features !== undefined ? opts.features : `popup=yes,width=${w},height=${h},left=${opts.left ?? 80},top=${opts.top ?? 80}`;
  const url = `popup.html?id=${id}&via=${via}&w=${w}&h=${h}&inp=${opts.inp || 'msg'}`;
  D.via.set(id, via);
  const t0 = now();
  const p = window.open(url, 'detached-' + id, feat);
  if (!p) return { blocked: true };
  D.popups.set(id, p);
  D.sabSend = null;
  const hello = new Promise((res) => { (D.helloWait = D.helloWait || {})[id] = res; });
  return { blocked: false, hello: hello.then((m) => ({ ms: now() - t0, hello: m })) };
};

// route the frames of window id to the popup by the chosen path; 'main' = relay by the opener, 'port' = MessageChannel worker<->popup, 'canvas' = popup's OffscreenCanvas
D.attach = (id, via) => {
  const p = D.popups.get(id);
  if (via === 'main') { if (D.mode === 'block') ringPost(100, id, 0, 0); else toWorker({ t: 'addWin', id, via: 'main' }); }
  else if (via === 'port') {
    const ch = new MessageChannel();
    toWorker({ t: 'addWin', id, via: 'port', port: ch.port1 }, [ch.port1]);
    p.postMessage({ t: 'port', port: ch.port2 }, location.origin, [ch.port2]);
  }
};
D.addWinInline = (id) => { if (D.mode === 'block') ringPost(100, id, 0, 0); else toWorker({ t: 'addWin', id, via: 'main' }); };
D.start = () => new Promise((res) => { D.worker.onmessage = ((orig) => (e) => { if (e.data.t === 'started') res(true); orig(e); })(D.worker.onmessage); toWorker({ t: 'start' }); });
D.stop = () => new Promise((res) => { D.stopResolve = res; if (D.mode === 'block') { Atomics.store(D.ctl, 2, 1); Atomics.add(D.bellI, 0, 1); Atomics.notify(D.bellI, 0); } else toWorker({ t: 'stop' }); });
D.report = (id) => new Promise((res) => { (D.reportWait = D.reportWait || {})[id] = res; D.popups.get(id).postMessage({ t: 'report' }, location.origin); });
D.say = (id, m) => D.popups.get(id).postMessage(m, location.origin);
D.closePopup = (id) => { const p = D.popups.get(id); const t = now(); p.close(); return { closedFlagRightAfter: p.closed, t }; };
D.probe = () => ({ coi: self.crossOriginIsolated, dpr: devicePixelRatio, screenX, screenY, outerWidth, outerHeight, innerWidth, innerHeight, screenW: screen.width, screenH: screen.height, availW: screen.availWidth, availH: screen.availHeight,
  isExtended: screen.isExtended, hasDocPiP: 'documentPictureInPicture' in window, hasGetScreenDetails: typeof window.getScreenDetails, hasFocus: document.hasFocus(), ua: navigator.userAgent });
D.pct = pct;
// the engine worker asks the main thread to open a popup some ms after a click (the real chain: click -> worker handles WM_COMMAND -> host call)
D.relayOpen = (delayMs) => new Promise((res) => {
  const w = D.worker || (D.worker = new Worker('engine-worker.js'));
  const t0 = performance.now();
  const h = (e) => { if (e.data && e.data.t === 'open-request') { w.removeEventListener('message', h); const active = navigator.userActivation.isActive; const p = window.open('popup.html?id=7&via=main', 'relay-' + Math.random(), 'popup=yes,width=200,height=150'); const r = { delayAskedMs: delayMs, activeAtHandler: active, blocked: !p, workerToMainMs: +(performance.now() - t0 - delayMs).toFixed(2) }; if (p) p.close(); res(r); } };
  w.addEventListener('message', h);
  w.postMessage({ t: 'ask-open', delayMs });
});
D.busy = (ms, every, total) => new Promise((res) => { const t0 = performance.now(); const tick = () => { const s = performance.now(); while (performance.now() - s < ms); if (performance.now() - t0 < total) setTimeout(tick, every); else res(true); }; tick(); });
D.geom = (id) => { const p = D.popups.get(id); return { sx: p.screenX, sy: p.screenY, ow: p.outerWidth, oh: p.outerHeight, iw: p.innerWidth, ih: p.innerHeight, dpr: p.devicePixelRatio }; };
D.moveFromOpener = (id, x, y) => { const p = D.popups.get(id); try { p.moveTo(x, y); return 'ok'; } catch (e) { return e.name + ': ' + e.message; } };
D.resizeFromOpener = (id, w, h) => { const p = D.popups.get(id); try { p.resizeTo(w, h); return 'ok'; } catch (e) { return e.name + ': ' + e.message; } };
// Document Picture-in-Picture window as the detached window: same JS realm cluster as this page, so canvas/bitmap handling and input listeners run here directly (no postMessage relay).
D.openPip = async (id, w = 512, h = 384) => {
  const pw = await documentPictureInPicture.requestWindow({ width: w, height: h });
  pw.document.title = 'pip-win';
  pw.document.body.style.cssText = 'margin:0;background:#000';
  const c = pw.document.createElement('canvas'); c.width = w; c.height = h; c.style.cssText = 'display:block;width:' + w + 'px;height:' + h + 'px';
  pw.document.body.appendChild(c);
  const st = { win: pw, ctx: c.getContext('bitmaprenderer'), frames: 0, gaps: [], lat: [], rtt: [], acks: 0, last: 0, inSent: 0, seq: 1, closedAt: null, moves: 0 };
  pw.addEventListener('pagehide', () => { st.closedAt = now(); });
  const sendIn = (kind) => { const m = { t: 'in', id, seq: st.seq++, t0: now(), kind }; st.inSent++; if (D.mode === 'block') ringPost(1, id, m.seq, m.t0); else toWorker(m); };
  c.addEventListener('mousemove', () => { st.moves++; sendIn('mousemove'); });
  pw.addEventListener('keydown', () => sendIn('keydown'));
  D.pips.set(id, st);
  D.addWinInline(id);
  return { w: pw.innerWidth, h: pw.innerHeight, dpr: pw.devicePixelRatio, outerW: pw.outerWidth, outerH: pw.outerHeight, screenX: pw.screenX, screenY: pw.screenY, opener: !!pw.opener, sameOriginDoc: !!pw.document.body };
};
D.pipReport = (id) => { const p = D.pips.get(id); return { frames: p.frames, gaps: p.gaps, lat: p.lat, rtt: p.rtt, acks: p.acks, inSent: p.inSent, moves: p.moves, closedAt: p.closedAt, closed: p.win.closed }; };
D.pipClose = (id) => { const p = D.pips.get(id); const t = now(); p.win.close(); return { t, closedFlag: p.win.closed }; };
D.popupState = (id) => { const p = D.popups.get(id); return { closed: p.closed }; };
D.audio = async () => { try { const ctx = new AudioContext({ latencyHint: 'interactive' }); const st0 = ctx.state; await Promise.race([ctx.resume(), new Promise((r) => setTimeout(r, 1500))]); D.ac = { ctx, t0: now(), c0: ctx.currentTime }; return { stateBeforeResume: st0, state: ctx.state, rate: ctx.sampleRate }; } catch (e) { return { error: e.name + ': ' + e.message }; } };
D.audioRead = () => D.ac ? ({ state: D.ac.ctx.state, ctxElapsed: D.ac.ctx.currentTime - D.ac.c0, realElapsed: (now() - D.ac.t0) / 1000 }) : { error: 'no audio context' };
// decode a PNG screenshot (base64) of the popup in the opener page: find the moving square (non-background colour) and return its R,G,B and the count of non-background pixels
window.__decodeShot = async (b64) => {
  const bin = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
  const bmp = await createImageBitmap(new Blob([bin], { type: 'image/png' }));
  const cv = new OffscreenCanvas(bmp.width, bmp.height); const cx = cv.getContext('2d'); cx.drawImage(bmp, 0, 0);
  const d = cx.getImageData(0, 0, bmp.width, bmp.height).data;
  const isBg = (r, g, b) => (r === 0x20 && g === 0x30 && b === 0x40) || (r === 0 && g === 0 && b === 0);
  let n = 0, sx = 0, sy = 0, R = -1, G = -1, B = -1, hist = new Map();
  for (let y = 40; y < bmp.height; y++) for (let x = 0; x < bmp.width; x++) { const i = (y * bmp.width + x) * 4; if (d[i + 2] === 128 && !isBg(d[i], d[i + 1], d[i + 2])) { n++; sx += x; sy += y; const k = d[i] + ',' + d[i + 1]; hist.set(k, (hist.get(k) || 0) + 1); } }
  let best = null; for (const [k, c] of hist) if (!best || c > best[1]) best = [k, c];
  if (best) { const [r, g] = best[0].split(',').map(Number); R = r; G = g; B = 128; }
  return { w: bmp.width, h: bmp.height, squarePixels: n, cx: n ? Math.round(sx / n) : null, cy: n ? Math.round(sy / n) : null, R, G, frame_lo: R, frame_hi: G, bg: [d[(60 * bmp.width + 5) * 4], d[(60 * bmp.width + 5) * 4 + 1], d[(60 * bmp.width + 5) * 4 + 2]] };
};
document.getElementById('b').onclick = () => { D.lastClick = now(); };
window.__ready = true;
