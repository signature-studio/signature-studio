// node driver for the FRUITY-on wasm (decision 0128, PROVISIONAL; lift/harness/two/build_nmod_wasm.sh FRUITY=1 THREADS=1 JSPI=1 SJLJ=wasm): module flengine6.{js,wasm}, arguments as main_two.c
// (<FLEngine dll> <real module file>[:<real module file>...]); FLE_CALL_FRUITY, FLE_VFS_SEED, FLE_VFS_SEED_ROOT, FLE_REG_SEED, FLE_AUTH_CLAIMS, FLE_FRUITY_* are read by the module through getenv (NODERAWFS,
// process.env). Run with `node --experimental-wasm-jspi` (node 24). Differences from run_node_jspi.cjs: no JSPI_CFG, no records; the wasm is instantiated here only to get the memory (and the phase times).
//   WF1_OUT=<dir>   where the surfaces go: every surface event of the module (Module.fleSurfaceEvent: kind 1 = the first non-background pixel, 2 = the stop) writes <dir>/surface-<kind>-lid<lid>.bgrx (BGRX, as held)
//                   and <dir>/surface-<kind>-lid<lid>.rgba.sha256 (SHA-256 of the RGBA bytes with A = 255, computed here with node's crypto: an independent second computation of the SHA the module prints).
//   WF1_JSON=<file> a JSON summary: exit code, error, phase times, memory (initial / final pages, the number of memory.grow calls seen through a Proxy of nothing: the final size only), rss, node/v8 versions.
// A wasm trap or a JS exception exits with 99.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const dir = process.env.FLE_WASM_DIR || __dirname;
const base = process.env.FLE_WASM_BIN || 'flengine6';
const create = require(path.join(dir, base + '.js'));
const t0 = process.hrtime.bigint();
const stamps = {};
const ms = () => Number(process.hrtime.bigint() - t0) / 1e6;
let memory = null, error = null;
const events = [];
const outDir = process.env.WF1_OUT || null;
if (outDir) fs.mkdirSync(outDir, { recursive: true });
let memPeakPages = 0;
const opts = { arguments: process.argv.slice(2), quit: (code) => { process.exitCode = code; stamps.quit = ms(); } };
opts.fleSurfaceEvent = (kind, lid, pixels, w, h, stride) => {
  const bytes = new Uint8Array(memory.buffer, pixels, stride * h);
  const rgba = Buffer.alloc(w * h * 4);
  for (let y = 0; y < h; y++) {
    const s = y * stride, d = y * w * 4;
    for (let x = 0; x < w; x++) {
      rgba[d + 4 * x] = bytes[s + 4 * x + 2]; rgba[d + 4 * x + 1] = bytes[s + 4 * x + 1]; rgba[d + 4 * x + 2] = bytes[s + 4 * x]; rgba[d + 4 * x + 3] = 255;
    }
  }
  const sha = crypto.createHash('sha256').update(rgba).digest('hex');
  events.push({ kind, lid, w, h, stride, pixels, rgbaSha256: sha, t_ms: ms(), memPages: memory.buffer.byteLength / 65536 });
  if (outDir) {
    fs.writeFileSync(path.join(outDir, `surface-${kind}-lid${lid}.bgrx`), Buffer.from(bytes));
    fs.writeFileSync(path.join(outDir, `surface-${kind}-lid${lid}.rgba.sha256`), sha + '\n');
  }
};
opts.instantiateWasm = (imports, done) => {
  const bytes = fs.readFileSync(path.join(dir, base + '.wasm'));
  stamps.read = ms();
  WebAssembly.compile(bytes).then((m) => {
    stamps.compiled = ms();
    return WebAssembly.instantiate(m, imports).then((inst) => {
      stamps.instantiated = ms();
      memory = inst.exports.memory || imports.env.memory;
      done(inst, m);
    });
  }).catch((e) => { console.error('wasm compile/instantiate failed:', e && e.message ? e.message : e); process.exitCode = 99; });
  return {};
};
function summary() {
  const mem = memory ? memory.buffer.byteLength : 0;
  return { exitCode: process.exitCode === undefined ? 0 : process.exitCode, error, stamps, events, finalMemoryBytes: mem, rssBytes: process.memoryUsage().rss, node: process.version, v8: process.versions.v8, execArgv: process.execArgv };
}
let written = false;
function writeSummary() {
  if (written) return;
  written = true;
  stamps.end = ms();
  if (process.env.WF1_JSON) fs.writeFileSync(process.env.WF1_JSON, JSON.stringify(summary(), null, 1));
}
// the summary is written on 'exit': an exit() inside a guest thread other than the first ends that thread's JSPI stack with an ExitStatus (fle_thread_lib.js) while main's promise may never settle
process.on('exit', writeSummary);
create(opts)
  .catch((e) => { error = e && e.message ? e.message : String(e); if (!(e && e.name === 'ExitStatus')) { console.error('wasm trap/exception:', error); process.exitCode = 99; } })
  .finally(writeSummary);
