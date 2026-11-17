'use strict';
onmessage = async (e) => {
  const c = e.data;
  try {
    importScripts('build/' + c.variant + '.js');
    const mc = new MessageChannel();
    const tick = c.wait === 'task' ? () => new Promise((r) => { mc.port2.onmessage = () => r(); mc.port1.postMessage(0); })
      : c.wait === 'micro' ? () => Promise.resolve() : null;
    const ms = []; const checksums = []; const waits = []; const suspensions = [];
    for (let rep = 0; rep < c.reps; rep++) {
      const m = await SY({ locateFile: (f) => 'build/' + f });
      let n = 0;
      const t0 = performance.now();
      for (;;) {
        m._entry(c.budget);
        if (m._cps_mode() === 1) {         // the wait unwound the chain; "wait for the event", then rewind
          n++;
          if (tick) await tick();
          m._cps_set(2);
          continue;
        }
        break;
      }
      ms.push(performance.now() - t0);
      checksums.push(m._checksum() | 0); waits.push(m._waits()); suspensions.push(n);
    }
    postMessage({ variant: c.variant, ms, checksums, waits, suspensions });
  } catch (err) {
    postMessage({ variant: c.variant, error: String(err && err.stack || err) });
  }
};
