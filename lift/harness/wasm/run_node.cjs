// node driver for flengine.js (MODULARIZE=1), built by build.sh (decision 0045).
// usage: [FLE_WASM_DIR=<dir with flengine.js>] node run_node.cjs <dll> [entry_va_hex]
// The default directory is the one of this file (build.sh copies this file next to flengine.js).
// The stop report is printed by the wasm module itself (trace.c). A wasm trap or a JS exception exits with 99.
const path = require('path');
const dir = process.env.FLE_WASM_DIR || __dirname;
const create = require(path.join(dir, (process.env.FLE_WASM_BIN || 'flengine') + '.js'));
create({ arguments: process.argv.slice(2), quit: (code) => { process.exitCode = code; } })
  .catch((e) => { console.error('wasm trap/exception:', e && e.message ? e.message : e); process.exitCode = 99; });
