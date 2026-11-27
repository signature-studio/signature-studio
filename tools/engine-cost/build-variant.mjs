// The engine of one optimization level (O2 or O3) for the engine-cost measurement (Issue #1388).
// It runs the engine and bundle steps of fe-build (lift/crates/fe-build/src/main.rs: plan, engine, bundle) with the level changed
// in two places only: the generated C (cflags-u, cflags-ai) and wasm-opt (wasm-opt-c.cmd, wasm-opt.args). The flag files are
// tools/engine-cost/flags/<name>-*: copies of lift/harness/web/ and lift/harness/hyb/ with the changed lines. The runtime, the harness,
// Blend2D, exports.rsp, the seed and the DLLs are the baseline's (lift/out-app/), reused as they are. Their flags do not change, and
// hybrid-n0 (which writes exports.rsp) is not run again.
// Usage: node tools/engine-cost/build-variant.mjs <O2|O3>
// Output: lift/out-app/variants/<name>/ (obj/, web/, hyb/, xmod/, gen/wasm-func-names.tsv, bundle/, variant.json). A stage whose output is
// there is skipped. To run a stage again, delete its output and the outputs of the later stages. A failed stage removes its outputs
// (the gen objects that are done are kept), prints its name and the exit status, and exits 1. The stage times go to stderr and to
// lift/out-app/engine-cost/build-<name>.log.
import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..'));

const USAGE = 'usage: node tools/engine-cost/build-variant.mjs <O2|O3>\n';
const name = process.argv[2];
if (process.argv.length !== 3 || !['O2', 'O3'].includes(name)) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const OPT = `-${name}`;

const OUT_APP = 'lift/out-app';
const HARNESS = 'lift/harness';
const FLAGS = 'tools/engine-cost/flags';
const VARIANT = `${OUT_APP}/variants/${name}`;
const OBJ = `${VARIANT}/obj`;
const WEB = `${VARIANT}/web`;
const HYB = `${VARIANT}/hyb`;
const GEN = `${VARIANT}/gen`;
const BUNDLE = `${VARIANT}/bundle`;
const VARIANT_JSON = `${VARIANT}/variant.json`;
const LOG = `${OUT_APP}/engine-cost/build-${name}.log`;
const FE_WASMGEN = 'lift/target/release/fe-wasmgen';
const GLUE_SED = `${HARNESS}/web/glue.sed`;
const EMSDK_ROOT = '/home/runner/work/_temp';
const FLAG_FILES = ['cflags-u.args', 'cflags-ai.args', 'wasm-opt-c.cmd', 'wasm-opt.args', 'link.rsp', 'hybrid.cmd', 'pack.args'];
const STAGES = ['gen-objects', 'link', 'wasm-opt', 'hybrid', 'func-names', 'glue', 'bundle'];
// fe-build's pack_step_for: the offline page is a lazy file of the bundle when the file is there
const OFFLINE_PAGE_SRC = `${OUT_APP}/host-extra/resources/System/Internet/error-offline.html`;
const OFFLINE_PAGE_PATH = 'resources/System/Internet/error-offline.html';
const BASELINE = {
  b2dObj: `${OUT_APP}/obj/b2d/b2d_swap.o`,
  blend2d: `${OUT_APP}/b2d/build/libblend2d.a`,
  exports: `${OUT_APP}/exports.rsp`,
  seed: `${OUT_APP}/seed/seed.bin`,
  dll: `${OUT_APP}/dll`,
  host: `${OUT_APP}/host`,
};
const timings = {};

const say = (message) => {
  const line = `${new Date().toISOString()} ${message}\n`;
  process.stderr.write(line);
  fs.appendFileSync(LOG, line);
};
const exists = (file) => fs.existsSync(file);
const isFile = (file) => fs.statSync(file, { throwIfNoEntry: false })?.isFile() ?? false;
const sorted = (items) => [...items].sort();
// Rust's str::lines and split_whitespace (fe-build reads the argument files and the lists this way)
const lines = (file) => {
  const text = fs.readFileSync(file, 'utf8');
  if (text === '') return [];
  return (text.endsWith('\n') ? text.slice(0, -1) : text).split('\n').map((line) => line.replace(/\r$/, ''));
};
const words = (file) => fs.readFileSync(file, 'utf8').split(/\s+/).filter(Boolean);
// fe-build's stem: the file name without its last extension
const stem = (file) => {
  const base = file.slice(file.lastIndexOf('/') + 1);
  const dot = base.lastIndexOf('.');
  return dot < 0 ? base : base.slice(0, dot);
};
const objOf = (src) => {
  const s = stem(src);
  return `${OBJ}/${s.startsWith('fle-') ? 'fle' : 'mod'}/${s}.o`;
};
const valueOf = (args, flag) => {
  const i = args.indexOf(flag);
  if (i < 0 || i + 1 >= args.length) throw new Error(`${flag} has no value in ${FLAGS}/${name}-pack.args`);
  return args[i + 1];
};
const sha256 = (file) => new Promise((resolve, reject) => {
  const hash = crypto.createHash('sha256');
  fs.createReadStream(file).on('data', (chunk) => hash.update(chunk)).on('end', () => resolve(hash.digest('hex'))).on('error', reject);
});

// One command, with no shell. Its output goes to the log; with stdoutFile, its stdout goes to that file. A failure rejects with
// the exit status, and the output is on the error (the stage prints its end).
function run(argv, stdoutFile) {
  const fd = stdoutFile === undefined ? null : fs.openSync(stdoutFile, 'w');
  return new Promise((resolve, reject) => {
    const chunks = [];
    let settled = false;
    const child = spawn(argv[0], argv.slice(1), { stdio: ['ignore', fd ?? 'pipe', 'pipe'] });
    if (fd === null) child.stdout.on('data', (chunk) => chunks.push(chunk));
    child.stderr.on('data', (chunk) => chunks.push(chunk));
    const settle = (error) => {
      if (settled) return;
      settled = true;
      if (fd !== null) fs.closeSync(fd);
      const output = Buffer.concat(chunks).toString('utf8');
      fs.appendFileSync(LOG, `$ ${argv.join(' ')}\n${output}${output === '' || output.endsWith('\n') ? '' : '\n'}`);
      if (error) reject(Object.assign(error, { output }));
      else resolve(output);
    };
    child.on('error', (e) => settle(new Error(`${argv[0]}: ${e.message}`)));
    child.on('close', (code, signal) => settle(code === 0 ? null : new Error(`${argv[0]} ${code === null ? `killed by ${signal}` : `exited with ${code}`}`)));
  });
}

async function pool(items, limit, work) {
  let next = 0;
  let failure = null;
  const lane = async () => {
    while (failure === null && next < items.length) {
      const item = items[next++];
      try {
        await work(item);
      } catch (e) {
        failure ??= e;
      }
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, lane));
  if (failure !== null) throw failure;
}

// A gen unit is one emcc -c; the object is written under a .tmp- name and renamed, so an object that exists is a finished one.
async function compileUnit({ src, flags, obj }) {
  if (exists(obj)) return;
  const tmp = path.join(path.dirname(obj), `.tmp-${path.basename(obj)}`);
  try {
    await run(['emcc', '-c', `@${flags}`, src, '-o', tmp]);
    fs.renameSync(tmp, obj);
  } finally {
    fs.rmSync(tmp, { force: true });
  }
}

// The level of the two generators' flags and of wasm-opt, and the paths the outputs go to (the variant's own directory).
function checkFlags() {
  const read = (file) => fs.readFileSync(`${FLAGS}/${name}-${file}`, 'utf8');
  for (const file of ['cflags-u.args', 'cflags-ai.args', 'wasm-opt-c.cmd', 'wasm-opt.args']) {
    if (/-O[sz]\b/.test(read(file)) || !new RegExp(`${OPT}\\b`).test(read(file))) throw new Error(`${file} must have ${OPT} and no -Os or -Oz`);
  }
  for (const file of ['link.rsp', 'hybrid.cmd', 'pack.args', 'wasm-opt-c.cmd']) {
    const text = read(file).split(`${OUT_APP}/variants/`).join('');
    if (new RegExp(`${OUT_APP}/(web|bundle|hyb|xmod)\\b`).test(text)) throw new Error(`${file} writes to a baseline output under ${OUT_APP}/`);
  }
  if (!read('hybrid.cmd').includes(`${FLAGS}/${name}-wasm-opt.args`)) throw new Error(`hybrid.cmd must name ${FLAGS}/${name}-wasm-opt.args`);
}

// fe-build's check_units: a unit that gen wrote and the lists do not name is a plugin's unit (m<K>-<NN>.c), compiled with the plain units (Issue #1525)
function pluginUnits(listed) {
  const named = new Set(listed.map((src) => src.slice(src.lastIndexOf('/') + 1)));
  const extra = fs.readdirSync(`${OUT_APP}/gen/src`).filter((file) => file.endsWith('.c') && file !== 'common.c' && !named.has(file)).sort();
  const other = extra.find((file) => !/^m\d+-\d+\.c$/.test(file));
  if (other !== undefined) throw new Error(`gen wrote ${other}, which the lists do not name and which is not a plugin unit (m<K>-<NN>.c)`);
  return extra.map((file) => `${OUT_APP}/gen/src/${file}`);
}

function makePlan() {
  const unitFlags = `${FLAGS}/${name}-cflags-u.args`;
  const aiFlags = `${FLAGS}/${name}-cflags-ai.args`;
  const plainUnits = words(`${HARNESS}/two/plain-units.txt`);
  const aiUnits = words(`${HARNESS}/two/ai-units.txt`);
  const genUnits = [
    ...plainUnits.map((src) => ({ src, flags: unitFlags })),
    ...aiUnits.map((src) => ({ src, flags: aiFlags })),
    ...pluginUnits([...plainUnits, ...aiUnits]).map((src) => ({ src, flags: unitFlags })),
    { src: `${OUT_APP}/gen/common.c`, flags: unitFlags },
  ].map((unit) => ({ ...unit, obj: objOf(unit.src) }));
  const rtObjs = fs.readdirSync('lift/runtime').filter((file) => file.startsWith('fle_') && file.endsWith('.c')).sort().map((file) => `${OUT_APP}/obj/rt/${stem(file)}.o`);
  const hObjs = [`${OUT_APP}/obj/h/main_two.o`, ...words(`${HARNESS}/two/harness-inputs.txt`).map((src) => `${OUT_APP}/obj/h/${stem(src)}.o`)];
  const packArgs = lines(`${FLAGS}/${name}-pack.args`);
  const required = [
    ...genUnits.map((unit) => unit.src),
    ...rtObjs,
    ...hObjs,
    BASELINE.b2dObj, BASELINE.blend2d, BASELINE.exports, BASELINE.seed, BASELINE.dll, BASELINE.host,
    FE_WASMGEN, GLUE_SED,
    ...FLAG_FILES.map((file) => `${FLAGS}/${name}-${file}`),
  ];
  const missing = required.filter((file) => !exists(file));
  if (missing.length > 0) throw new Error(`missing inputs: ${missing.join(' ')}`);
  checkFlags();
  return { genUnits, hObjs, rtObjs, packArgs, prefix: valueOf(packArgs, '--prefix') };
}

// fe-build's toolchain_bin: <emcc dir>/../bin first (the emsdk's wasm-opt and wasm-merge), then the emcc directory.
function findEmccDir() {
  if (!exists(EMSDK_ROOT)) return undefined;
  for (const entry of fs.readdirSync(EMSDK_ROOT).sort()) {
    if (entry.startsWith('.')) continue;
    const dir = path.join(EMSDK_ROOT, entry, 'emsdk-main', 'upstream', 'emscripten');
    if (exists(path.join(dir, 'emcc'))) return dir;
  }
  return undefined;
}

function useToolchain() {
  const found = process.env.EMCC_DIR || findEmccDir();
  if (!found || !exists(path.join(found, 'emcc'))) throw new Error('emcc not found: set EMCC_DIR to the emscripten directory');
  const emcc = path.resolve(found);
  process.env.PATH = [path.join(emcc, '..', 'bin'), emcc, process.env.PATH ?? ''].join(path.delimiter);
}

async function stage(stageName, outputs, body, { keep = false } = {}) {
  if (outputs.length > 0 && outputs.every(exists)) {
    say(`${stageName}: exists, skipped`);
    return;
  }
  const start = Date.now();
  say(`${stageName}: start`);
  try {
    await body();
  } catch (e) {
    if (!keep) for (const file of outputs) fs.rmSync(file, { force: true });
    say(`${stageName}: failed: ${e.message}`);
    if (e.output) process.stderr.write(`${e.output.split('\n').slice(-40).join('\n')}\n`);
    process.exit(1);
  }
  const seconds = Math.round((Date.now() - start) / 100) / 10;
  timings[stageName] = seconds;
  say(`${stageName}: done in ${seconds} s`);
}

// fe-build's Glue step: the two rules of glue.sed, run by sed (lift/harness/web/glue.sed; tools/engine-cost/check-glue.mjs checks it)
async function glue() {
  const tmp = `${WEB}/.tmp-engine-glue.js`;
  try {
    await run(['sed', '-E', '-f', GLUE_SED, `${WEB}/flengine6.js`], tmp);
    fs.renameSync(tmp, `${WEB}/engine-glue.js`);
  } finally {
    fs.rmSync(tmp, { force: true });
  }
}

// fe-build's pack_step_for. The seed step is not run: the baseline's seed pack is used (seedPack in the flags' pack.args).
async function bundle(packArgs) {
  const version = (await sha256(`${WEB}/engine.wasm`)).slice(0, 12);
  const argv = [process.execPath, 'app/scripts/pack-assets.mjs', '--version', version, ...packArgs];
  if (isFile(OFFLINE_PAGE_SRC)) argv.push('--lazy-file', `${OFFLINE_PAGE_SRC}=${OFFLINE_PAGE_PATH}`);
  await run(argv);
}

function summarize(prefix) {
  const manifestPath = `${BUNDLE}/raw/${prefix}/manifest.json`;
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const brFile = (url) => `${BUNDLE}/br/${url.replace(/^\/assets\//, '')}`;
  const engineBr = brFile(manifest.engine.wasm.url);
  // the files the page takes before the engine starts: the manifest, the engine, its glue and the two packs (the lazy files come after)
  const preBoot = [manifestPath, engineBr, brFile(manifest.engine.options.glue.url), brFile(manifest.seedPack.url), brFile(manifest.dllPack.url)];
  const previous = exists(VARIANT_JSON) ? JSON.parse(fs.readFileSync(VARIANT_JSON, 'utf8')).stageSeconds ?? {} : {};
  return {
    name,
    flags: Object.fromEntries(FLAG_FILES.map((file) => [file, `${FLAGS}/${name}-${file}`])),
    stageSeconds: Object.fromEntries(STAGES.map((stageName) => [stageName, timings[stageName] ?? previous[stageName] ?? null])),
    engineWasmBytes: fs.statSync(`${WEB}/engine.wasm`).size,
    engineGlueBytes: fs.statSync(`${WEB}/engine-glue.js`).size,
    engineBrBytes: fs.statSync(engineBr).size,
    // the engine as the page downloads it: the bundle's brotli file
    wasmBytes: fs.statSync(engineBr).size,
    preBootBytes: preBoot.reduce((total, file) => total + fs.statSync(file).size, 0),
    preBootFiles: preBoot,
    wasmFuncNames: `${GEN}/wasm-func-names.tsv`,
    bundleVersion: manifest.bundle.version,
    bundleDir: `${BUNDLE}/raw/${prefix}`,
    reusedFromBaseline: Object.values(BASELINE),
    completedAt: new Date().toISOString(),
  };
}

async function main() {
  fs.mkdirSync(path.dirname(LOG), { recursive: true });
  say(`build-variant ${name}: start (level ${OPT}; output ${VARIANT}/)`);
  let plan;
  await stage('preflight', [], () => {
    plan = makePlan();
    useToolchain();
  });
  for (const dir of [`${OBJ}/fle`, `${OBJ}/mod`, WEB, HYB, GEN, BUNDLE]) fs.mkdirSync(dir, { recursive: true });
  const { genUnits, hObjs, rtObjs, packArgs, prefix } = plan;
  const genObjs = genUnits.map((unit) => unit.obj);
  await stage('gen-objects', genObjs, () => pool(genUnits, os.availableParallelism(), compileUnit), { keep: true });
  // fe-build's link order: the groups h, mod, fle and rt, each sorted
  const linkArgv = [
    'emcc',
    `@${FLAGS}/${name}-link.rsp`,
    `@${BASELINE.exports}`,
    ...sorted(hObjs),
    ...sorted(genObjs.filter((obj) => obj.includes('/mod/'))),
    ...sorted(genObjs.filter((obj) => obj.includes('/fle/'))),
    ...sorted(rtObjs),
  ];
  await stage('link', [`${WEB}/flengine6.js`, `${WEB}/flengine6.wasm`], () => run(linkArgv));
  await stage('wasm-opt', [`${WEB}/flengine6.c.opt.wasm`], () => run(['wasm-opt', ...lines(`${FLAGS}/${name}-wasm-opt-c.cmd`)]));
  await stage('hybrid', [`${WEB}/engine.wasm`], () => run([FE_WASMGEN, ...lines(`${FLAGS}/${name}-hybrid.cmd`)]));
  await stage('func-names', [`${GEN}/wasm-func-names.tsv`], () => fs.copyFileSync(`${HYB}/wasm-func-names.tsv`, `${GEN}/wasm-func-names.tsv`));
  await stage('glue', [`${WEB}/engine-glue.js`], glue);
  await stage('bundle', [`${BUNDLE}/raw/${prefix}/manifest.json`], () => bundle(packArgs));
  const summary = summarize(prefix);
  fs.writeFileSync(VARIANT_JSON, `${JSON.stringify(summary, null, 2)}\n`);
  say(`build-variant ${name}: done, ${VARIANT_JSON}`);
  process.stdout.write(`${JSON.stringify(summary)}\n`);
}

main().catch((e) => {
  say(`build-variant ${name}: ${e.message}`);
  process.exit(1);
});
