'use strict';
// Single-producer / single-consumer event ring in a SharedArrayBuffer (used by the page and by Workers: <script src> / importScripts).
// Header (Int32 index): WAKE futex word, HEAD (consumer), TAIL (producer), DROPPED, CAP, REPLY (consumer -> producer sequence), STOP,
// MOVE_GEN (seqlock of the "latest mouse position" slot; the coalescing path of WM_MOUSEMOVE), MOVE_SEEN (consumer side).
// Slot = 8 doubles: [type, seq, a, b, tSend, ...]. Time = performance.timeOrigin + performance.now() (ms, the clock of that thread).
const RG = { WAKE: 0, HEAD: 1, TAIL: 2, DROPPED: 3, CAP: 4, REPLY: 5, STOP: 6, MOVE_GEN: 7, MOVE_SEEN: 8, HDR_BYTES: 64, SLOT: 8 };
const EV = { PING: 1, MOVE: 2, DOWN: 3, UP: 4, KEY: 5, TIMER: 6, QUIT: 9 };
const wallNow = () => performance.timeOrigin + performance.now();

function ringAlloc(cap) {
  const sab = new SharedArrayBuffer(RG.HDR_BYTES + 64 + cap * 64);
  new Int32Array(sab)[RG.CAP] = cap;
  return sab;
}

class Ring {
  constructor(sab) {
    this.sab = sab;
    this.i = new Int32Array(sab);
    this.f = new Float64Array(sab, RG.HDR_BYTES);
    this.cap = this.i[RG.CAP];
  }
  // producer. returns false (and counts a drop) if the ring is full; the producer never blocks (the page's main thread must not).
  post(type, seq, a, b, t) {
    const i = this.i;
    const tail = Atomics.load(i, RG.TAIL);
    if (tail - Atomics.load(i, RG.HEAD) >= this.cap) { Atomics.add(i, RG.DROPPED, 1); return false; }
    const o = 8 + (tail % this.cap) * 8;
    const f = this.f;
    f[o] = type; f[o + 1] = seq; f[o + 2] = a; f[o + 3] = b; f[o + 4] = t;
    Atomics.store(i, RG.TAIL, tail + 1);
    Atomics.add(i, RG.WAKE, 1);
    Atomics.notify(i, RG.WAKE, 1);
    return true;
  }
  // coalesced mouse position: only the latest value is kept, the consumer synthesises one MOVE per generation it has not seen
  postMove(x, y, t) {
    const i = this.i;
    const g = Atomics.load(i, RG.MOVE_GEN);
    Atomics.store(i, RG.MOVE_GEN, g + 1);
    const f = this.f;
    f[0] = x; f[1] = y; f[2] = t;
    Atomics.store(i, RG.MOVE_GEN, g + 2);
    Atomics.add(i, RG.WAKE, 1);
    Atomics.notify(i, RG.WAKE, 1);
  }
  depth() { return Atomics.load(this.i, RG.TAIL) - Atomics.load(this.i, RG.HEAD); }
  // consumer. GetMessage: blocks in Atomics.wait up to timeoutMs (Infinity = forever). out = Float64Array(8). returns true if out was filled, false on timeout.
  get(out, timeoutMs) {
    const i = this.i;
    const f = this.f;
    for (;;) {
      const w = Atomics.load(i, RG.WAKE);
      const head = Atomics.load(i, RG.HEAD);
      if (head !== Atomics.load(i, RG.TAIL)) {
        const o = 8 + (head % this.cap) * 8;
        for (let k = 0; k < 8; k++) out[k] = f[o + k];
        Atomics.store(i, RG.HEAD, head + 1);
        return true;
      }
      const g = Atomics.load(i, RG.MOVE_GEN);
      if (!(g & 1) && g !== Atomics.load(i, RG.MOVE_SEEN)) {
        out[0] = EV.MOVE; out[2] = f[0]; out[3] = f[1]; out[4] = f[2];
        if (Atomics.load(i, RG.MOVE_GEN) === g) { Atomics.store(i, RG.MOVE_SEEN, g); return true; }
        continue;
      }
      if (Atomics.wait(i, RG.WAKE, w, timeoutMs) === 'timed-out') return false;
    }
  }
}

function spin(ms) { const e = performance.now() + ms; let x = 0; while (performance.now() < e) x++; return x; }
function summarize(arr, qs) {
  qs = qs || [50, 95, 99];
  const a = Float64Array.from(arr).sort();
  const n = a.length;
  const o = { n };
  if (!n) return o;
  o.min = a[0]; o.max = a[n - 1]; o.mean = a.reduce((s, x) => s + x, 0) / n;
  for (const q of qs) o['p' + q] = a[Math.min(n - 1, Math.floor((q / 100) * n))];
  return o;
}
