// mt2 mp2: the wake-up latency of Atomics.waitAsync (what an idle Worker whose guest threads are JSPI coroutines has to use: it returns to its event loop) against the blocking Atomics.wait.
// Worker A (blocking) posts a value into word 0 and notifies; Worker B (waitAsync loop, in the event loop) answers in word 1; A waits blocking for the answer. Round trips, microseconds.
self.onmessage = async (ev) => {
  const { mode, sab, n } = ev.data;
  const ia = new Int32Array(sab);
  if (ev.data.role === 'B') {
    let last = 0;
    const loop = async () => {
      for (;;) {
        let cur = Atomics.load(ia, 0);
        while (cur === last) {
          if (mode === 'async') { const r = Atomics.waitAsync(ia, 0, last); if (r.async) await r.value; }
          else { await new Promise((res) => setTimeout(res, 0)); }
          cur = Atomics.load(ia, 0);
        }
        last = cur;
        Atomics.store(ia, 1, cur); Atomics.notify(ia, 1);
        if (cur === n) return;
      }
    };
    self.postMessage('ready'); await loop(); self.postMessage('done');
  }
};
