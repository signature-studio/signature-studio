'use strict';
// M-A engine stand-in: a Worker that runs a C-like blocking loop (GetMessage = Atomics.wait on the ring) and never returns to its event loop
// while it runs. The page sends one start message {mode, ...}; the loop runs inside that handler and posts a result at the end.
importScripts('ring.js');

onmessage = (e) => {
  const c = e.data;
  const r = MODES[c.mode](c);
  postMessage({ type: 'result', mode: c.mode, result: r });
};

const MODES = {
  // GetMessage loop that answers PING events: via the REPLY word (c.reply = 'sab') or via postMessage (c.reply = 'pm')
  echo(c) {
    const ring = new Ring(c.sab);
    const i = ring.i;
    const out = new Float64Array(8);
    let n = 0;
    postMessage({ type: 'ready' });
    for (;;) {
      ring.get(out, Infinity);
      if (out[0] === EV.QUIT) break;
      if (out[0] === EV.PING) {
        n++;
        if (c.reply === 'pm') postMessage(out[1]);
        else { Atomics.store(i, RG.REPLY, out[1]); Atomics.notify(i, RG.REPLY); }
      }
    }
    return { handled: n };
  },

  // two Workers ping-pong on one word; 'a' measures (both sides are in Atomics.wait, no page thread involved)
  pp(c) {
    const i = new Int32Array(c.sab);
    const lat = [];
    if (c.side === 'a') {
      for (let k = 1; k <= c.n; k++) {
        const t0 = performance.now();
        Atomics.store(i, 0, k);          // request
        Atomics.notify(i, 0);
        while (Atomics.load(i, 1) !== k) Atomics.wait(i, 1, k - 1, 1000);
        lat.push((performance.now() - t0) * 1000);
      }
      Atomics.store(i, 2, 1); Atomics.notify(i, 0);
      return { unit: 'us', rtt: summarize(lat) };
    }
    let seen = 0;
    for (;;) {
      while (Atomics.load(i, 0) === seen && !Atomics.load(i, 2)) Atomics.wait(i, 0, seen, 1000);
      if (Atomics.load(i, 2)) break;
      seen = Atomics.load(i, 0);
      Atomics.store(i, 1, seen); Atomics.notify(i, 1);
    }
    return {};
  },

  // Atomics.wait with a timeout: how late does it return (the SetTimer / WM_TIMER stand-in)
  timeouts(c) {
    const i = new Int32Array(new SharedArrayBuffer(16));
    const rows = {};
    for (const T of c.list) {
      const reps = c.reps || Math.max(20, Math.min(400, Math.round(3000 / T)));
      const late = [];
      for (let k = 0; k < reps; k++) {
        const t0 = performance.now();
        Atomics.wait(i, 0, 0, T);
        late.push(performance.now() - t0 - T);
      }
      rows[T] = summarize(late);
      rows[T].reps = reps;
    }
    return { unit: 'ms late (elapsed - requested)', rows };
  },

  // WM_TIMER stand-in: absolute deadlines every c.period ms for c.ticks ticks; the loop is also a GetMessage loop that drains ring events
  // (each costing c.work ms of CPU) until the next deadline. Records lateness of every tick, and post->dequeue latency of events.
  timer(c) {
    const ring = new Ring(c.sab);
    const out = new Float64Array(8);
    const late = [];
    const evLat = [];
    let events = 0;
    let maxDepth = 0;
    postMessage({ type: 'ready' });
    const t0 = performance.now();
    let next = t0 + c.period;
    let ticks = 0;
    while (ticks < c.ticks) {
      const now = performance.now();
      const wait = next - now;
      if (wait <= 0) {
        late.push(now - next);
        ticks++;
        next += c.period;
        if (next < now) next = now + c.period;   // missed more than one period: Windows also coalesces WM_TIMER
        continue;
      }
      const d = ring.depth();
      if (d > maxDepth) maxDepth = d;
      if (ring.get(out, wait)) {
        events++;
        evLat.push(wallNow() - out[4]);
        if (c.work) spin(c.work);
      }
    }
    const dropped = Atomics.load(ring.i, RG.DROPPED);
    return { unit: 'ms', tickLate: summarize(late), tickLateMax: Math.max.apply(null, late), eventLatency: summarize(evLat), events, dropped, maxDepth, period: c.period, ticks };
  },

  // event consumer that spends c.work ms of CPU per event (the engine is busy: rendering, mixing, loading): queue depth, drops, latency
  busy(c) {
    const ring = new Ring(c.sab);
    const out = new Float64Array(8);
    const evLat = [];
    let events = 0;
    let moves = 0;
    let maxDepth = 0;
    postMessage({ type: 'ready' });
    for (;;) {
      const d = ring.depth();
      if (d > maxDepth) maxDepth = d;
      ring.get(out, Infinity);
      if (out[0] === EV.QUIT || Atomics.load(ring.i, RG.STOP)) break;
      events++;
      if (out[0] === EV.MOVE) moves++;
      evLat.push(wallNow() - out[4]);
      if (c.work) spin(c.work);
    }
    return { unit: 'ms', latency: summarize(evLat), consumed: events, moves, dropped: Atomics.load(ring.i, RG.DROPPED), maxDepth, work: c.work };
  },

  // like 'timer' but the worker posts a tick to the page every period (to see what a busy page thread does to messages coming back)
  ticker(c) {
    const i = new Int32Array(new SharedArrayBuffer(16));
    const late = [];
    postMessage({ type: 'ready' });
    const t0 = performance.now();
    let next = t0 + c.period;
    for (let k = 0; k < c.ticks; k++) {
      const wait = next - performance.now();
      if (wait > 0) Atomics.wait(i, 0, 0, wait);
      const now = performance.now();
      late.push(now - next);
      postMessage({ type: 'tick', k, t: wallNow() });
      next += c.period;
    }
    return { tickLate: summarize(late), tickLateMax: Math.max.apply(null, late) };
  },
};
