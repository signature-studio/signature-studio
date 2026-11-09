// node driver for the JSPI measurement (docs/investigations/jspi-real-wasm.md). Same arguments, environment and exit codes as
// lift/harness/two/run_node_nmod.cjs (<FLEngine dll> <real module files joined with ':'>; FLE_WASM_DIR, FLE_WASM_BIN,
// FLE_WASM_FAST_RETADDR=1 replaces emscripten_return_address by () => 0), and it also runs a wasm built without JSPI. Differences:
//   - node needs --experimental-wasm-jspi (node 24, V8 13.6) for a wasm linked with -sJSPI; other V8 flags can be added on the node
//     command line (--wasm-stack-switching-stack-size=<kB> is the size of a JSPI stack, --stack-size=<kB> that of the main stack);
//   - the wasm is always compiled and instantiated by this file (WebAssembly.compile, then WebAssembly.instantiate) so that the two
//     times can be taken;
//   - FLE_JSPI_CFG='{"mode":1,"every_0":1}' becomes globalThis.JSPI_CFG (read by lift/harness/wasm/jspi_hooks.c);
//   - FLE_JSPI_COUNT_RA=1 (with FLE_WASM_FAST_RETADDR=1) counts the calls of the emscripten_return_address stub (JSON key raCalls);
//   - FLE_JSPI_TICK=<ms> runs a setInterval of that period during the run and reports how often it fired (key ticks): proof that other JS tasks run while the wasm is suspended;
//   - FLE_JSPI_OUT=<file> gets a JSON summary (exit code, error, phase times, per-kind statistics of the records of jspi_hooks.c in
//     microseconds, calls per kind); FLE_JSPI_RAW=<file> gets every record as a JSON array of rows
//     [kind, ordinal, guest return address, guest rsp, live landings, fle_call_guest depth, dt_total_us, dt_js_us, frames, js frames, mode, seq].
const fs = require('fs');
const path = require('path');
const dir = process.env.FLE_WASM_DIR || __dirname;
const base = process.env.FLE_WASM_BIN || 'flengine6';
if (process.env.FLE_JSPI_CFG) globalThis.JSPI_CFG = JSON.parse(process.env.FLE_JSPI_CFG);
const create = require(path.join(dir, base + '.js'));
const t0 = process.hrtime.bigint();
const stamps = {};
const ms = () => Number(process.hrtime.bigint() - t0) / 1e6;
let error = null;
const opts = { arguments: process.argv.slice(2), quit: (code) => { process.exitCode = code; stamps.quit = ms(); } };
opts.instantiateWasm = (imports, done) => {
  if (process.env.FLE_WASM_FAST_RETADDR === '1') imports.env.emscripten_return_address = process.env.FLE_JSPI_COUNT_RA === '1' ? () => { globalThis.RA_CALLS = (globalThis.RA_CALLS || 0) + 1; return 0; } : () => 0;
  const bytes = fs.readFileSync(path.join(dir, base + '.wasm'));
  stamps.read = ms();
  WebAssembly.compile(bytes).then((m) => {
    stamps.compiled = ms();
    return WebAssembly.instantiate(m, imports).then((inst) => {
      stamps.instantiated = ms();
      done(inst, m);
    });
  }).catch((e) => { console.error('wasm compile/instantiate failed:', e && e.message ? e.message : e); process.exitCode = 99; });
  return {};
};

let tickTimer = null;
if (process.env.FLE_JSPI_TICK) { globalThis.TICKS = 0; tickTimer = setInterval(() => { globalThis.TICKS++; }, +process.env.FLE_JSPI_TICK); }
const pct = (a, p) => a.length ? a[Math.min(a.length - 1, Math.floor(a.length * p))] : null;
function stats(v) {
  const a = Array.from(v).sort((x, y) => x - y);
  const sum = a.reduce((s, x) => s + x, 0);
  return { n: a.length, min: a[0], p50: pct(a, 0.5), p95: pct(a, 0.95), p99: pct(a, 0.99), max: a[a.length - 1], mean: a.length ? sum / a.length : null, sum };
}
function summary() {
  const out = { exitCode: process.exitCode === undefined ? 0 : process.exitCode, error, stamps, cfg: globalThis.JSPI_CFG || null,
    counts: globalThis.JSPI_COUNTS || null, raCalls: globalThis.RA_CALLS || null, ticks: tickTimer ? globalThis.TICKS : null, deepest: globalThis.JSPI_DEEPEST ? globalThis.JSPI_DEEPEST.split('\n') : null, rssBytes: process.memoryUsage().rss, node: process.version, v8: process.versions.v8, execArgv: process.execArgv };
  const r = globalThis.JSPI_REC;
  if (r) {
    out.records = r.n;
    out.lastRecords = [];
    for (let i = Math.max(0, r.n - 3); i < r.n; i++) out.lastRecords.push(Array.from(r.f.subarray(i * 12, i * 12 + 12)));
    const by = {};
    for (let i = 0; i < r.n; i++) {
      const o = i * 12;
      const k = r.f[o];
      const b = by[k] || (by[k] = { total: [], js: [], frames: [], jsFrames: [], nland: {}, cb: {}, sp: [] });
      b.total.push(r.f[o + 6] * 1000);
      b.js.push(r.f[o + 7] * 1000);
      if (r.f[o + 8] >= 0) { b.frames.push(r.f[o + 8]); b.jsFrames.push(r.f[o + 9]); }
      b.nland[r.f[o + 4]] = (b.nland[r.f[o + 4]] || 0) + 1;
      b.cb[r.f[o + 5]] = (b.cb[r.f[o + 5]] || 0) + 1;
      b.sp.push(r.f[o + 3]);
    }
    out.byKind = {};
    for (const k of Object.keys(by)) {
      const b = by[k];
      out.byKind[k] = { n: b.total.length, total_us: stats(b.total), js_us: stats(b.js), frames: b.frames.length ? stats(b.frames) : null,
        jsFrames: b.jsFrames.length ? stats(b.jsFrames) : null, nland: b.nland, cbdepth: b.cb, spMin: b.sp.reduce((x, y) => Math.min(x, y), Infinity), spMax: b.sp.reduce((x, y) => Math.max(x, y), 0) };
    }
    if (process.env.FLE_JSPI_RAW) {
      const rows = [];
      for (let i = 0; i < r.n; i++) {
        const o = i * 12;
        rows.push([r.f[o], r.f[o + 1], r.f[o + 2], r.f[o + 3], r.f[o + 4], r.f[o + 5], r.f[o + 6] * 1000, r.f[o + 7] * 1000, r.f[o + 8], r.f[o + 9], r.f[o + 10], r.f[o + 11]]);
      }
      fs.writeFileSync(process.env.FLE_JSPI_RAW, JSON.stringify(rows));
    }
  }
  return out;
}
create(opts)
  .catch((e) => {
    error = { name: e && e.name, message: e && e.message ? e.message : String(e), stack: e && e.stack ? String(e.stack).split('\n').slice(0, 120) : null };
    console.error('wasm trap/exception:', e && e.message ? e.message : e);
    process.exitCode = 99;
  })
  .finally(() => {
    stamps.end = ms();
    if (tickTimer) clearInterval(tickTimer);
    if (process.env.FLE_JSPI_OUT) fs.writeFileSync(process.env.FLE_JSPI_OUT, JSON.stringify(summary(), null, 1) + '\n');
  });
