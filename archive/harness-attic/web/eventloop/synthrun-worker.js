'use strict';
// Run-time cost of the Asyncify instrumentation on lifted-shaped synthetic code when nothing suspends: entry(n) = n function entries of
// the synthetic call graph (mb/synth.mjs, budget variant). Message: {variant, budget, reps}.
onmessage = async (e) => {
  const c = e.data;
  try {
    importScripts('build/' + c.variant + '.js');
    const m = await SY({ locateFile: (f) => 'build/' + f.replace(/^l_/, 'sy_'), asyncGet: () => Promise.resolve(0) });
    const ms = [];
    let ret = null;
    for (let r = 0; r < c.reps; r++) {
      const t0 = performance.now();
      ret = await m._entry(c.budget);   // JSPI: the export returns a promise; Asyncify / plain: the value (nothing suspends here)
      ms.push(performance.now() - t0);
    }
    postMessage({ variant: c.variant, budget: c.budget, ms, ret });
  } catch (err) {
    postMessage({ variant: c.variant, error: String(err && err.stack || err) });
  }
};
