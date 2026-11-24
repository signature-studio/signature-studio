// Engine cost, stage 0 contrast: what one call from wasm into a JS import costs through the emscripten glue (EM_JS) and through a direct import with no glue.
// The import body is the same expression (x + 1) in both, and the loop is the same C: run(n) adds hot(i) for i < n inside the wasm.
// The two wasm files are built here with the emsdk (emcc, and the clang of the same emsdk), kept in build/, and timed in node.
// Timing: an untimed warm-up run(1000000) per variant, then 3 timed runs per variant (the order of the two alternates). ns per call = ms x 1e6 / 1000000, the median of the 3.
// Usage: node tools/engine-cost/contrast/contrast.mjs   (no arguments). Writes lift/out-app/engine-cost/contrast.json and prints the same JSON as one line.
// Exit 0 when error is null, 1 when the run failed (the JSON is still written), 2 when arguments are given.
// Environment (this tool only): EMCC_DIR overrides the emsdk's upstream/emscripten. Without it, the first /home/runner/work/_temp/*/emsdk-main/upstream/emscripten that has emcc is used.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..', '..');
const BUILD = path.join(here, 'build');
const TMP = path.join(repo, `.tmp-contrast-${process.pid}`);
const OUT = path.join(repo, 'lift', 'out-app', 'engine-cost', 'contrast.json');
const SRC_EMSCRIPTEN = path.join(here, 'hot-emscripten.c');
const SRC_DIRECT = path.join(here, 'hot-direct.c');
const FILE = { glue: 'hot-emscripten.mjs', emscriptenWasm: 'hot-emscripten.wasm', directWasm: 'hot-direct.wasm' };
const CALLS = 1000000;
const RUNS = 3;
const TOOL_TIMEOUT_MS = 1800000;
const EMSDK_TEMP = '/home/runner/work/_temp';
const EMCC_FLAGS = ['-O2', '-sMODULARIZE=1', '-sEXPORT_ES6=1', '-sENVIRONMENT=node', '-sEXPORTED_FUNCTIONS=_run'];
const CLANG_FLAGS = ['--target=wasm32', '-O2', '-nostdlib', '-Wl,--no-entry', '-Wl,--export=run', '-Wl,--allow-undefined'];
const USAGE = 'usage: node tools/engine-cost/contrast/contrast.mjs\n';

const r3 = (x) => Math.round(x * 1000) / 1000;
const errText = (e) => String(e?.message ?? e).slice(0, 1500);
const median = (xs) => [...xs].sort((a, b) => a - b)[(xs.length - 1) >> 1];
const nsPerCall = (ms) => (ms * 1e6) / CALLS;
const spreadNs = (xs) => ((Math.max(...xs) - Math.min(...xs)) * 1e6) / CALLS;
const spreadPct = (xs) => ((Math.max(...xs) - Math.min(...xs)) / median(xs)) * 100;

function findEmccDir() {
  if (process.env.EMCC_DIR) return process.env.EMCC_DIR;
  const found = fs.existsSync(EMSDK_TEMP)
    ? fs.readdirSync(EMSDK_TEMP).sort()
      .map((d) => path.join(EMSDK_TEMP, d, 'emsdk-main', 'upstream', 'emscripten'))
      .find((d) => fs.existsSync(path.join(d, 'emcc')))
    : undefined;
  if (!found) throw new Error(`emcc not found: no ${EMSDK_TEMP}/*/emsdk-main/upstream/emscripten/emcc (set EMCC_DIR)`);
  return found;
}

function tool(cmd, args) {
  try {
    return execFileSync(cmd, args, { cwd: TMP, env: { ...process.env, TMPDIR: TMP }, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], timeout: TOOL_TIMEOUT_MS, maxBuffer: 64 * 1024 * 1024 });
  } catch (e) {
    throw new Error(`${path.basename(cmd)} failed: ${String(e.stderr || e.message).slice(-1200)}`);
  }
}

function build(emccDir) {
  const emcc = path.join(emccDir, 'emcc');
  const clang = path.join(emccDir, '..', 'bin', 'clang');
  const emccVersion = tool(emcc, ['--version']).split('\n')[0].trim();
  tool(emcc, [...EMCC_FLAGS, SRC_EMSCRIPTEN, '-o', path.join(TMP, FILE.glue)]);
  tool(clang, [...CLANG_FLAGS, SRC_DIRECT, '-o', path.join(TMP, FILE.directWasm)]);
  return emccVersion;
}

function keepBuild() {
  fs.mkdirSync(BUILD, { recursive: true });
  fs.writeFileSync(path.join(BUILD, '.gitignore'), '*\n');
  for (const f of Object.values(FILE)) fs.copyFileSync(path.join(TMP, f), path.join(BUILD, f));
}

function checkWasm(bytes, label) {
  const mod = new WebAssembly.Module(bytes);
  const hasImport = WebAssembly.Module.imports(mod).some((i) => i.module === 'env' && i.name === 'hot' && i.kind === 'function');
  const hasExport = WebAssembly.Module.exports(mod).some((e) => e.name === 'run' && e.kind === 'function');
  if (!hasImport || !hasExport) throw new Error(`${label}: needs the import env.hot (function) and the export run (function)`);
}

async function load() {
  const emscriptenBytes = fs.readFileSync(path.join(BUILD, FILE.emscriptenWasm));
  const directBytes = fs.readFileSync(path.join(BUILD, FILE.directWasm));
  checkWasm(emscriptenBytes, FILE.emscriptenWasm);
  checkWasm(directBytes, FILE.directWasm);
  const glueSource = fs.readFileSync(path.join(BUILD, FILE.glue), 'utf8');
  if (!/(?:function hot\(x\)|hot:\s*function\(x\))\s*\{\s*return x\s*\+\s*1;?\s*\}/.test(glueSource)) throw new Error(`${FILE.glue}: hot is not "function hot(x) { return x + 1; }" (the glue may minify it as hot:function(x){return x+1})`);
  const glue = await import(pathToFileURL(path.join(BUILD, FILE.glue)).href);
  const emscriptenModule = await glue.default();
  const direct = await WebAssembly.instantiate(directBytes, { env: { hot: (x) => x + 1 } });
  const fns = { emscripten: emscriptenModule._run, direct: direct.instance.exports.run };
  for (const [name, fn] of Object.entries(fns)) if (typeof fn !== 'function') throw new Error(`${name}: run is not a function`);
  return fns;
}

function reference(n) {
  let s = 0;
  for (let i = 0; i < n; i++) s = (s + i + 1) | 0;
  return s;
}

function measure(fns, expected) {
  const check = (name, v) => {
    if (v !== expected) throw new Error(`checksum mismatch: ${name} returned ${v}, expected ${expected} (the JS reference of the loop)`);
  };
  const names = Object.keys(fns);
  for (const name of names) check(name, fns[name](CALLS));
  const ms = { emscripten: [], direct: [] };
  for (let r = 0; r < RUNS; r++) {
    const order = r % 2 === 0 ? names : [...names].reverse();
    for (const name of order) {
      const t0 = performance.now();
      const v = fns[name](CALLS);
      ms[name].push(performance.now() - t0);
      check(name, v);
    }
  }
  return ms;
}

function notesFor({ ms, nsEm, nsDir, expected, la0, la1 }) {
  const cores = os.availableParallelism();
  const diff = nsEm - nsDir;
  const noise = Math.max(spreadNs(ms.emscripten), spreadNs(ms.direct));
  const pct = Math.max(spreadPct(ms.emscripten), spreadPct(ms.direct));
  const lines = [
    `Method: an untimed warm-up run(${CALLS}) per variant, then ${RUNS} timed run(${CALLS}) calls per variant (the order of the two alternates each round). performance.now() brackets each call only. ns per call = ms x 1e6 / ${CALLS}, the median of the ${RUNS} runs.`,
    `Same work in both: the C loop (unsigned sum of hot(i) for i < n) and the import body (x + 1). emscripten: emcc ${EMCC_FLAGS.join(' ')}; the glue defines hot as "function hot(x) { return x + 1; }" and passes it as env.hot (checked). direct: clang ${CLANG_FLAGS.join(' ')} with extern hot; the importObject has env.hot = (x) => x + 1, no glue.`,
    `Checksum ${expected} from both wasm and the JS reference of the loop.`,
    `Difference (emscripten minus direct): ${r3(diff)} ns per call. Run-to-run spread: ${r3(noise)} ns per call, ${r3(pct)}% of the median (the larger of the two).`,
    `Node ${process.version}, nproc ${cores}, loadavg before ${la0.map(r3).join(' ')}, after ${la1.map(r3).join(' ')}.`,
  ];
  if (Math.abs(diff) <= noise) lines.push('The difference is within the run-to-run spread: this measurement does not resolve it.');
  else if (Math.abs(diff) > 20) lines.push('The difference is over 20 ns per call, which is more than a glue that passes the same JS function should cost: check the wasm imports and the glue before reading it as glue cost.');
  if (pct > 10) lines.push('The spread is over 10% of the median: the machine was not quiet. Rerun with no other job on it before using the numbers.');
  if (Math.min(nsEm, nsDir) < 1) lines.push('Under 1 ns per call for one variant: the engine may inline the JS import into the wasm loop, so this is a best case, not the cost of the boundary.');
  if (Math.max(nsEm, nsDir) > 100) lines.push('Over 100 ns per call: slower than a direct call should be. Check for an interpreter tier or other load.');
  if (Math.max(la0[0], la1[0]) > cores / 2) lines.push('The 1-minute loadavg was above half the cores: other work ran during the measurement.');
  return lines.join(' ');
}

async function main() {
  const result = { calls: CALLS, nsPerCallEmscripten: null, nsPerCallDirect: null, diffNsPerCall: null, runs: { emscripten: [], direct: [] }, checksum: null, emccVersion: null, error: null, notes: '' };
  try {
    fs.mkdirSync(TMP, { recursive: true });
    result.emccVersion = build(findEmccDir());
    keepBuild();
    const fns = await load();
    const expected = reference(CALLS);
    const la0 = os.loadavg();
    const ms = measure(fns, expected);
    const la1 = os.loadavg();
    const nsEm = nsPerCall(median(ms.emscripten));
    const nsDir = nsPerCall(median(ms.direct));
    result.nsPerCallEmscripten = r3(nsEm);
    result.nsPerCallDirect = r3(nsDir);
    result.diffNsPerCall = r3(nsEm - nsDir);
    result.runs = { emscripten: ms.emscripten.map(r3), direct: ms.direct.map(r3) };
    result.checksum = expected;
    result.notes = notesFor({ ms, nsEm, nsDir, expected, la0, la1 });
  } catch (e) {
    result.error = errText(e);
    result.notes = 'Not measured: the build or the run failed (see error).';
  } finally {
    fs.rmSync(TMP, { recursive: true, force: true });
  }
  return result;
}

if (process.argv.length > 2) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const result = await main();
const line = JSON.stringify(result);
fs.mkdirSync(path.dirname(OUT), { recursive: true });
fs.writeFileSync(OUT, `${line}\n`);
process.stdout.write(`${line}\n`);
process.exitCode = result.error === null ? 0 : 1;
