'use strict';
// M-B Worker: loads one build of mb.c (build/mb_<mode>_<opt>.js) and runs either the echo loop (a C GetMessage loop that suspends in a
// Promise import until the page posts an event) or the suspend/resume micro benchmark. First message = {variant, op, ...}.
let mod = null;
const queue = [];
let waiter = null;

function fill(ptr, d) {
  const H32 = mod.HEAP32;
  H32[ptr >> 2] = d.type; H32[(ptr >> 2) + 1] = d.a; H32[(ptr >> 2) + 2] = d.b;
  mod.HEAPF64[(ptr + 16) >> 3] = d.t || 0;
}

onmessage = async (e) => {
  const c = e.data;
  if (c.op === undefined) { // an event for the loop
    if (waiter) { const w = waiter; waiter = null; w(c); } else queue.push(c);
    return;
  }
  try {
    importScripts('build/' + c.variant + '.js');
    mod = await MB({
      locateFile: (f) => 'build/' + f,
      mc: new MessageChannel(),
      asyncGet: (ptr) => new Promise((res) => {
        if (queue.length) { fill(ptr, queue.shift()); res(1); } else waiter = (d) => { fill(ptr, d); res(1); };
      }),
      syncGet: (ptr) => { fill(ptr, { type: 1, a: 0, b: 0, t: 0 }); return 1; },
    });
    if (c.op === 'echo') {
      onmessage = (e2) => { const d = e2.data; if (waiter) { const w = waiter; waiter = null; w(d); } else queue.push(d); };
      postMessage({ type: 'ready' });
      const n = await mod.ccall('loop_echo', 'number', ['number'], [c.depth], { async: true });
      postMessage({ type: 'result', result: { handled: n } });
    } else if (c.op === 'depth') {
      // largest n for which rec_depth(n) does not throw (binary search); the call is made from the Worker's event loop turn
      const call = async (n) => { try { const r = mod._rec_depth(n); await r; return true; } catch (err) { return /call stack|Maximum/i.test(String(err)) ? false : String(err); } };
      let lo = 1000, hi = 8000000;
      if ((await call(lo)) !== true) { postMessage({ type: 'result', result: { max: 0, note: 'fails at 1000' } }); return; }
      while (hi - lo > 100) { const mid = (lo + hi) >> 1; const ok = await call(mid); if (ok === true) lo = mid; else hi = mid; }
      postMessage({ type: 'result', result: { max_depth: lo, bytes_per_frame_hint: null } });
    } else if (c.op === 'bench') {
      const rows = [];
      for (const wide of c.wides) for (const depth of c.depths) for (const kind of c.kinds) {
        const n = kind === 2 ? c.nTask : kind === 1 ? c.nMicro : c.nNop;
        // the same configuration three times; the first is the warm-up (V8 tier-up), the minimum of the other two is reported
        const ms3 = [];
        for (let rep = 0; rep < 3; rep++) {
          const t0 = performance.now();
          await mod.ccall('bench_yield', 'number', ['number', 'number', 'number', 'number'], [depth, kind, n, wide], { async: true });
          ms3.push(performance.now() - t0);
        }
        const ms = Math.min(ms3[1], ms3[2]);
        rows.push({ wide, depth, kind, n, ns_per_iter: +((ms * 1e6) / n).toFixed(1), ns_first: +((ms3[0] * 1e6) / n).toFixed(1), ns_reps: ms3.slice(1).map((x) => +((x * 1e6) / n).toFixed(1)) });
      }
      postMessage({ type: 'result', result: { rows } });
    }
  } catch (err) {
    postMessage({ type: 'result', result: { error: String(err && err.stack || err) } });
  }
};
