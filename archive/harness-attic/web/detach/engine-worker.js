// Stand-in for the engine worker of the detached-window experiment (docs/investigations/detach-window.md). It is NOT FL code: it renders
// 512x384 frames for every registered window and answers input events, in the two loop shapes of browser-event-loop.md:
//   mode 'loop'  : returns to its event loop between frames (what a JSPI-suspended engine looks like from outside); input arrives by onmessage.
//   mode 'block' : after 'start' it never returns (Atomics.wait, the M-A shape); input can only arrive through the SharedArrayBuffer ring.
// A window is registered with one of three ways to get pixels out: via 'main' (frames posted to the opener page, which relays them),
// via 'port' (a MessagePort that goes straight to the popup), via 'canvas' (an OffscreenCanvas the popup transferred; drawn into directly).
'use strict';
const W = 512, H = 384, TICK = 16;
const now = () => performance.timeOrigin + performance.now();
const S = { mode: 'loop', wins: new Map(), running: false, ctl: null, slots: null, ring: 0, inputs: 0, ticks: 0 };
const oc = new OffscreenCanvas(W, H);
const cx = oc.getContext('2d');

function draw(ctx, w, h, f) {
  ctx.fillStyle = '#203040'; ctx.fillRect(0, 0, w, h);
  const x = (f * 4) % (w - 64), y = 40 + ((f * 3) % (h - 104));
  ctx.fillStyle = 'rgb(' + (f & 255) + ',' + ((f >> 8) & 255) + ',128)'; ctx.fillRect(x, y, 64, 64);
  ctx.fillStyle = '#fff'; ctx.font = '20px sans-serif'; ctx.fillText('frame ' + f, 8, 24);
}

function ack(win, id, seq, t0, tW) {
  const m = { t: 'ack', id, seq, t0, tW };
  if (win.via === 'port') win.port.postMessage(m); else self.postMessage(m);
}

function renderAll() {
  S.ticks++;
  for (const [id, win] of S.wins) {
    win.frame++;
    if (win.via === 'canvas') {
      draw(win.ctx, win.canvas.width, win.canvas.height, win.frame);
      win.sent++;
      continue;
    }
    draw(cx, W, H, win.frame);
    const bmp = oc.transferToImageBitmap();
    const m = { t: 'frame', id, frame: win.frame, tGen: now(), bmp };
    if (win.via === 'port') win.port.postMessage(m, [bmp]); else self.postMessage(m, [bmp]);
    win.sent++;
  }
}

function addWin(m) {
  const win = { via: m.via, frame: 0, sent: 0, port: m.port || null, canvas: m.canvas || null, ctx: null };
  if (win.via === 'canvas') win.ctx = win.canvas.getContext('2d');
  if (win.via === 'port') win.port.onmessage = (e) => handleInput(e.data);
  S.wins.set(m.id, win);
}

function handleInput(m) {
  if (m.t !== 'in') return;
  const win = S.wins.get(m.id);
  S.inputs++;
  if (win) ack(win, m.id, m.seq, m.t0, now());
}

function drainIn() {                         // the popup's own ring: only input events, no window commands
  while (S.rd2 < Atomics.load(S.ctl2, 0)) {
    const o = (S.rd2 % S.ring) * 4;
    const seq = S.slots2[o], id = S.slots2[o + 1], t0 = S.slots2[o + 2];
    S.rd2++;
    handleInput({ t: 'in', id, seq, t0 });
  }
}
function drainRing() {
  drainIn();
  const ctl = S.ctl, slots = S.slots;
  while (S.rd < Atomics.load(ctl, 0)) {
    const o = (S.rd % S.ring) * 4;
    const seq = slots[o], id = slots[o + 1], t0 = slots[o + 2], kind = slots[o + 3];
    S.rd++;
    if (kind === 100) { addWin({ id, via: 'main' }); continue; }
    if (kind === 101) { S.wins.delete(id); continue; }
    handleInput({ t: 'in', id, seq, t0 });
  }
}

function blockLoop() {
  let next = performance.now() + TICK;
  while (!Atomics.load(S.ctl, 2)) {
    const left = next - performance.now();
    if (left > 0) Atomics.wait(S.bell, 0, S.bellSeen, left);   // one futex word (the doorbell) that every producer (opener ring, popup ring) rings after writing
    S.bellSeen = Atomics.load(S.bell, 0);
    drainRing();
    if (performance.now() >= next) { renderAll(); next += TICK; if (next < performance.now()) next = performance.now() + TICK; }
  }
  report('stopped');
}

function report(t) {
  const sent = {};
  for (const [id, w] of S.wins) sent[id] = w.sent;
  self.postMessage({ t, sent, inputs: S.inputs, ticks: S.ticks, mode: S.mode });
}

function loopTick() {
  if (!S.running) { report('stopped'); return; }
  renderAll();
  setTimeout(loopTick, TICK);
}

onmessage = (e) => {
  const m = e.data;
  if (m.t === 'ask-open') { setTimeout(() => self.postMessage({ t: 'open-request' }), m.delayMs); return; }
  if (m.t === 'init') {
    S.mode = m.mode;
    if (m.sab) { S.ctl = new Int32Array(m.sab, 0, 4); S.slots = new Float64Array(m.sab, 16, m.ring * 4); S.ring = m.ring; S.rd = 0;
      S.ctl2 = new Int32Array(m.sabIn, 0, 4); S.slots2 = new Float64Array(m.sabIn, 16, m.ring * 4); S.rd2 = 0;
      S.bell = new Int32Array(m.bell, 0, 1); S.bellSeen = 0; }
    self.postMessage({ t: 'inited', sab: !!m.sab, coi: self.crossOriginIsolated });
  } else if (m.t === 'addWin') addWin(m);
  else if (m.t === 'removeWin') S.wins.delete(m.id);
  else if (m.t === 'in') handleInput(m);
  else if (m.t === 'start') {
    S.running = true;
    self.postMessage({ t: 'started' });
    if (S.mode === 'block') blockLoop(); else loopTick();
  } else if (m.t === 'stop') S.running = false;
};
