// node driver for the N-module wasm (lift/harness/two/build_nmod_wasm.sh): module flengine6.{js,wasm}, arguments as main_two.c
// (<FLEngine dll> <real module file>[:<real module file>...] [entry_va_hex]); FLE_VFS_SEED / FLE_VFS_SEED_ROOT are read by the module
// through getenv (NODERAWFS, process.env). A wasm trap or a JS exception exits with 99.
// FLE_WASM_FAST_RETADDR=1: the wasm is instantiated by this file and the JS import emscripten_return_address (a JS stack-trace walk
// that clang's -finstrument-functions makes at every hook call via __builtin_return_address, 5.5 us each) is replaced by a function that
// returns 0. The hooks of harness/wasm/trace.c ignore the call_site argument ((void)call_site), so the module's output is not changed by
// it; without the variable the module is started exactly as lift/harness/two/run_node_two.cjs does.
// FLE_WASM_TIMING=<file>: write the wall times of the phases there (not to stderr, which is compared).
const fs = require('fs');
const path = require('path');
const dir = process.env.FLE_WASM_DIR || __dirname;
const base = process.env.FLE_WASM_BIN || 'flengine6';
const create = require(path.join(dir, base + '.js'));
const t0 = process.hrtime.bigint();
const stamps = [];
const ms = () => Number(process.hrtime.bigint() - t0) / 1e6;
const opts = { arguments: process.argv.slice(2), quit: (code) => { process.exitCode = code; stamps.push(['quit', ms()]); } };
if (process.env.FLE_WASM_FAST_RETADDR === '1' || process.env.FLE_WASM_TIMING) {
  opts.instantiateWasm = (imports, done) => {
    if (process.env.FLE_WASM_FAST_RETADDR === '1') imports.env.emscripten_return_address = () => 0;
    const bytes = fs.readFileSync(path.join(dir, base + '.wasm'));
    stamps.push(['read', ms()]);
    WebAssembly.instantiate(bytes, imports).then((r) => {
      stamps.push(['instantiated', ms()]);
      done(r.instance, r.module);
    }, (e) => { console.error('wasm instantiate failed:', e && e.message ? e.message : e); process.exitCode = 99; });
    return {};
  };
}
create(opts)
  .catch((e) => { console.error('wasm trap/exception:', e && e.message ? e.message : e); process.exitCode = 99; })
  .finally(() => {
    stamps.push(['end', ms()]);
    if (process.env.FLE_WASM_TIMING) fs.writeFileSync(process.env.FLE_WASM_TIMING, stamps.map(([k, v]) => `${k} ${v.toFixed(0)} ms`).join('\n') + '\n');
  });
