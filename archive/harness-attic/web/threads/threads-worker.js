// Web Worker of the thread scenarios in a browser (decision 0124, PROVISIONAL). One run = one Worker = one fresh instance of scen.wasm (the program has global state).
// Protocol: the page posts {args: ['<scenario>', ...], dist: '/dist/'}; the Worker answers ONCE with
//   {out: [stdout lines], err: [stderr lines], code, stats: Module.fleThreadStats(), ms}
// `code` is the exit code of main (Module.quit), 99 for a trap or a thread failure. A thread other than the first can end the run (an abort in a thread leaves the first thread suspended
// forever, so the promise of the module never settles): fleThreadFailed answers at once, as the node runner (run_node_scen.cjs) sets the exit code at once.
let answered = false;
onmessage = async (ev) => {
  const { args, dist } = ev.data;
  const out = [], err = [];
  let code = 0, M = null;
  const t0 = performance.now();
  const answer = () => {
    if (answered) return;
    answered = true;
    let stats = null;
    try { stats = M && M.fleThreadStats ? M.fleThreadStats() : null; } catch (e) { stats = { error: String(e) }; }
    postMessage({ out, err, code, stats, ms: performance.now() - t0 });
  };
  try {
    importScripts(dist + 'scen.js');
    M = await createScen({
      arguments: args,
      print: (s) => out.push(s),
      printErr: (s) => err.push(s),
      locateFile: (f) => dist + f,
      quit: (c) => { code = c; },
      fleThreadFailed: (idx, e, msg) => { err.push('[js] ' + msg); code = 99; answer(); },
    });
  } catch (e) {
    err.push('[js] wasm trap/exception: ' + (e && e.message ? e.message : String(e)));
    code = 99;
  }
  answer();
};
