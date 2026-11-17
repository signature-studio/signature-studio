// measurement only: like run_node_nmod.cjs with FLE_WASM_FAST_RETADDR=1, plus a counter on the emscripten_return_address import
// (= the number of JS stack-trace walks that the unchanged driver makes). The count is written to FLE_WASM_COUNT_FILE.
const fs = require('fs'), path = require('path');
const dir = process.env.FLE_WASM_DIR || __dirname;
const base = process.env.FLE_WASM_BIN || 'flengine6';
const create = require(path.join(dir, base + '.js'));
let n = 0;
create({
  arguments: process.argv.slice(2),
  quit: (code) => { process.exitCode = code; },
  instantiateWasm: (imports, done) => {
    imports.env.emscripten_return_address = () => { n++; return 0; };
    WebAssembly.instantiate(fs.readFileSync(path.join(dir, base + '.wasm')), imports).then((r) => done(r.instance, r.module));
    return {};
  },
}).catch((e) => { console.error('wasm trap/exception:', e && e.message ? e.message : e); process.exitCode = 99; })
  .finally(() => fs.writeFileSync(process.env.FLE_WASM_COUNT_FILE, String(n) + '\n'));
