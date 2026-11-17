// measurement only: the unchanged JS emscripten_return_address (stack walk) wrapped by a counter; after BUDGET_S seconds of run the
// count and the elapsed time are written to FLE_WASM_COUNT_FILE and the process exits. Gives walks/s of the real module.
const fs = require('fs'), path = require('path');
const dir = process.env.FLE_WASM_DIR, base = 'flengine6';
const budget = Number(process.env.BUDGET_S || 120) * 1000;
const create = require(path.join(dir, base + '.js'));
let n = 0, t0 = 0;
create({
  arguments: process.argv.slice(2),
  quit: (code) => { process.exitCode = code; },
  instantiateWasm: (imports, done) => {
    const real = imports.env.emscripten_return_address;
    imports.env.emscripten_return_address = (lvl) => {
      n++;
      if ((n & 1023) === 0 && Date.now() - t0 > budget) {
        fs.writeFileSync(process.env.FLE_WASM_COUNT_FILE, `walks ${n} in ${Date.now() - t0} ms\n`);
        process.exit(0);
      }
      return real(lvl);
    };
    WebAssembly.instantiate(fs.readFileSync(path.join(dir, base + '.wasm')), imports).then((r) => { t0 = Date.now(); done(r.instance, r.module); });
    return {};
  },
}).catch((e) => console.error('exc', e && e.message));
