// node runner of the thread scenarios (decision 0124, PROVISIONAL): node --experimental-wasm-jspi run_node_scen.cjs <dir with scen.js/scen.wasm> <scenario>
// stdout of the program -> stdout, stderr (the scheduler trace and the named stops) -> stderr, byte for byte (emscripten's NODERAWFS writes fd 1 and fd 2 directly).
// Exit code: the program's; 99 for a JS exception or a thread failure (a trap, RangeError ...), with the message on stderr after the program's own output.
// env FLE_SCEN_STATS=<file>: write {switches, starts, ends, failed} of the JS scheduler (Module.fleThreadStats) plus maxRssKB and wallMs of this process as JSON.
const fs = require('fs');
const path = require('path');
const T0 = process.hrtime.bigint();
const dir = path.resolve(process.argv[2]);
const create = require(path.join(dir, 'scen.js'));
let M = null;
const opts = {
  arguments: process.argv.slice(3),
  quit: (code) => { process.exitCode = code; },
  fleThreadFailed: (idx, e, msg) => { process.stderr.write('[js] ' + msg + '\n'); process.exitCode = 99; },
};
create(opts).then((m) => { M = m; }).catch((e) => {
  process.stderr.write('[js] wasm trap/exception: ' + (e && e.message ? e.message : e) + '\n');
  process.exitCode = 99;
}).finally(() => {
  if (process.env.FLE_SCEN_STATS) {
    const st = (M && M.fleThreadStats) ? M.fleThreadStats() : {};
    st.maxRssKB = process.resourceUsage().maxRSS;      // peak RSS of the node process (KB)
    st.wallMs = Number(process.hrtime.bigint() - T0) / 1e6;
    fs.writeFileSync(process.env.FLE_SCEN_STATS, JSON.stringify(st));
  }
});
