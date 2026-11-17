// node driver for the two-module wasm (an old build; the product bundle is built by app/scripts/build-engine-bundle.sh). Like lift/harness/wasm/run_node.cjs, with two files.
// usage: [FLE_WASM_DIR=<dir with two.js>] node run_node_two.cjs <FLEngine dll> <real module file> [entry_va_hex]
// The stop report is printed by the wasm module itself (harness/wasm/trace.c). A wasm trap or a JS exception exits with 99.
const path = require('path');
const dir = process.env.FLE_WASM_DIR || __dirname;
const create = require(path.join(dir, (process.env.FLE_WASM_BIN || 'two') + '.js'));
create({ arguments: process.argv.slice(2), quit: (code) => { process.exitCode = code; } })
  .catch((e) => { console.error('wasm trap/exception:', e && e.message ? e.message : e); process.exitCode = 99; });
