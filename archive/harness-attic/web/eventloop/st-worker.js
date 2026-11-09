'use strict';
// M-B stand-in for SetTimer: how late does setTimeout(T) fire inside a Worker that returns to its event loop (what a JSPI/Asyncify engine does)
importScripts('ring.js');
onmessage = async (e) => {
  const rows = {};
  for (const T of e.data.list) {
    const reps = Math.max(30, Math.min(400, Math.round(3000 / Math.max(T, 1))));
    const late = [];
    for (let k = 0; k < reps; k++) {
      const t0 = performance.now();
      await new Promise((r) => setTimeout(r, T));
      late.push(performance.now() - t0 - T);
    }
    rows[T] = Object.assign(summarize(late), { reps });
  }
  // the same with Atomics.waitAsync (needs a SharedArrayBuffer, i.e. cross-origin isolation): a timeout that is not subject to the nested-timer clamp?
  rows.waitAsync = {};
  if (typeof SharedArrayBuffer !== 'undefined' && Atomics.waitAsync) {
    const i32 = new Int32Array(new SharedArrayBuffer(16));
    for (const T of [0.5, 1, 2, 5]) {
      const late = [];
      for (let k = 0; k < 300; k++) {
        const t0 = performance.now();
        await Atomics.waitAsync(i32, 0, 0, T).value;
        late.push(performance.now() - t0 - T);
      }
      rows.waitAsync[T] = Object.assign(summarize(late), { reps: 300 });
    }
  }
  // absolute-deadline ticker, like the timer test of M-A: next = prev + period
  const late = [];
  let next = performance.now() + 10;
  for (let k = 0; k < 500; k++) {
    await new Promise((r) => setTimeout(r, Math.max(0, next - performance.now())));
    late.push(performance.now() - next);
    next += 10;
  }
  postMessage({ rows, tick10: summarize(late) });
};
