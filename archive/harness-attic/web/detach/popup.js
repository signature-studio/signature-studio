// Detached-window stand-in (the popup page). One canvas = one guest window. It never talks to the engine worker by itself unless it was
// handed a MessagePort / an OffscreenCanvas; the opener tells it how (message 'setup'). Input events go back as {t:'in', id, seq, t0}
// where t0 = performance.timeOrigin + performance.now() of this document (comparable across documents of one machine: same clock source).
'use strict';
const c = document.getElementById('c');
const q = new URLSearchParams(location.search);
const id = +q.get('id') || 1, via = q.get('via') || 'main';
const W = +q.get('w') || 512, H = +q.get('h') || 384;
const inp = q.get('inp') || 'msg';
const dpr = window.devicePixelRatio || 1;
c.style.width = W + 'px'; c.style.height = H + 'px';
const now = () => performance.timeOrigin + performance.now();
const st = { id, via, inp, sabState: 'none', frames: 0, lastFrameT: 0, gaps: [], lat: [], inSent: 0, acks: 0, rtt: [], port: null, gen: [], keys: 0, moves: 0 };
let ctx2 = null, bmpCtx = null, seq = 1;
const opener = window.opener;
const toOpener = (m, tr) => { if (opener) opener.postMessage(m, location.origin, tr || []); };
const send = (m, tr) => { if (st.port && (m.t === 'in')) st.port.postMessage(m, tr || []); else toOpener(m, tr); };

if (via === 'canvas') {
  c.width = W; c.height = H;
  const off = c.transferControlToOffscreen();
  window.addEventListener('load', () => send({ t: 'canvas', id, canvas: off, w: W, h: H }, [off]));
} else {
  c.width = W; c.height = H;
  bmpCtx = c.getContext('bitmaprenderer');
}

function onFrame(m) {
  const t = now();
  if (st.lastFrameT) st.gaps.push(t - st.lastFrameT);
  st.lastFrameT = t;
  st.lat.push(t - m.tGen);
  st.frames++;
  if (bmpCtx) bmpCtx.transferFromImageBitmap(m.bmp); else m.bmp.close();
}
function onAck(m) { st.acks++; st.rtt.push(now() - m.t0); }

let ctl = null, slots = null, ringN = 0, wr = 0, bell = null;
function recv(m) {
  if (m.t === 'sab') { bell = new Int32Array(m.bell, 0, 1); ctl = new Int32Array(m.sab, 0, 4); slots = new Float64Array(m.sab, 16, m.ring * 4); ringN = m.ring; wr = Atomics.load(ctl, 0); st.sabState = 'received'; }
  else if (m.t === 'frame') onFrame(m);
  else if (m.t === 'ack') onAck(m);
  else if (m.t === 'port') { st.port = m.port; st.port.onmessage = (e) => recv(e.data); }
  else if (m.t === 'report') {
    send({ t: 'popup-report', id, via, frames: st.frames, gaps: st.gaps, lat: st.lat, inSent: st.inSent, acks: st.acks, rtt: st.rtt,
      screenX: window.screenX, screenY: window.screenY, outerW: window.outerWidth, outerH: window.outerHeight, innerW: window.innerWidth, innerH: window.innerHeight,
      dpr: window.devicePixelRatio, hasFocus: document.hasFocus(), visibility: document.visibilityState, opener: !!window.opener, keys: st.keys, moves: st.moves,
      canvasW: c.width, canvasH: c.height, isTop: window.top === window, sabState: st.sabState, coi: self.crossOriginIsolated });
  } else if (m.t === 'move') { try { window.moveTo(m.x, m.y); } catch (e) { send({ t: 'log', msg: 'moveTo failed ' + e }); } }
  else if (m.t === 'geom') { send({ t: 'geom-reply', id, sx: window.screenX, sy: window.screenY, ow: window.outerWidth, oh: window.outerHeight, iw: window.innerWidth, ih: window.innerHeight, dpr: window.devicePixelRatio }); }
  else if (m.t === 'resize') { try { window.resizeTo(m.w, m.h); } catch (e) { send({ t: 'log', msg: 'resizeTo failed ' + e }); } }
  else if (m.t === 'send-input') sendInput('probe');
}
window.addEventListener('messageerror', (e) => { st.sabState = 'messageerror'; });
window.addEventListener('message', (e) => { if (e.origin === location.origin && e.source === opener) recv(e.data); });

function sendInput(kind) {
  const m = { t: 'in', id, seq: seq++, t0: now(), kind };
  st.inSent++;
  if (inp === 'sab' && ctl) { const o = (wr % ringN) * 4; slots[o] = m.seq; slots[o + 1] = id; slots[o + 2] = m.t0; slots[o + 3] = 1; wr++; Atomics.store(ctl, 0, wr); Atomics.add(bell, 0, 1); Atomics.notify(bell, 0); return; }
  send(m);
}
c.addEventListener('mousemove', () => { st.moves++; sendInput('mousemove'); });
c.addEventListener('mousedown', () => sendInput('mousedown'));
window.addEventListener('keydown', () => { st.keys++; sendInput('keydown'); });
// the popup closed by the user (or by close()): tell the opener (the "attach again" path)
window.addEventListener('pagehide', () => { try { send({ t: 'closed', id }); } catch (e) { /* port gone */ } });
window.addEventListener('beforeunload', () => { try { send({ t: 'closed', id }); } catch (e) { /* port gone */ } });
window.__popup = st;
send({ t: 'hello', id, via, dpr, screenX: window.screenX, screenY: window.screenY, outerW: window.outerWidth, outerH: window.outerHeight });
