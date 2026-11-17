// mt2 (decision 0234): lift/harness/parity/node-bundle-run.mjs with one more replacement, for the glue of a -pthread link (its minimum-runtime check is written with double quotes). Nothing else differs.
// prune2 (decision 0184): run the SHIPPED engine bundle (manifest.json + engine glue + wasm + DLL pack; e.g. lift/out-websrv/assets/fl-engine-fin1-F2xn) in node with a G3 script (FLE_G3_SCRIPT, the commands of lift/harness/fruity_g3.c).
// The glue is the browser glue (MEMFS); this driver does what the browser Worker does before main(): the DLL pack (flpack1) and a seed pack (flseedp1) are written to MEMFS under /data, the manifest's env plus the caller's env go to Module.ENV.
// No input comes from a browser: the script is the only input (virtual clock). 'fle: pruned function 0x... was executed' (abort) and every stop line is counted.
// usage: node --experimental-wasm-jspi node-bundle-run.mjs BUNDLE_DIR SEED_PACK SCRIPT OUT_DIR [KEY=VALUE ...]      (KEY=VALUE: more guest environment variables)
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const [bundle, seedPackFile, scriptFile, outDir, ...kv] = process.argv.slice(2);
if (!outDir) { console.error('usage: node --experimental-wasm-jspi node-bundle-run.mjs BUNDLE_DIR SEED_PACK SCRIPT OUT_DIR [K=V ...]'); process.exit(2); }
fs.mkdirSync(outDir, { recursive: true });
const man = JSON.parse(fs.readFileSync(path.join(bundle, 'manifest.json'), 'utf8'));
const asset = (ref) => path.join(bundle, path.basename(ref.url));
const glueFile = asset(man.engine.options.glue);
const wasmFile = asset(man.engine.wasm);
const dllPackFile = asset(man.dllPack);
const glueCopy = path.join(outDir, 'engine-glue.mjs');
// the glue is built for web+worker only: its minimum-runtime check refuses node; the copy has that check switched off and the node detection is off so that it takes the worker path (nothing else is changed)
fs.writeFileSync(glueCopy, fs.readFileSync(glueFile, 'utf8').replace("if (typeof process !== 'undefined' && !process.versions?.bun && typeof Deno == \"undefined\") {", 'if (false) {').replace('if (typeof process !== "undefined" && !process.versions?.bun && typeof Deno == "undefined") {', 'if (false) {').replace('globalThis.process?.versions?.node && globalThis.process?.type != "renderer"', 'false').replace("globalThis.process?.versions?.node && globalThis.process?.type != 'renderer'", 'false'));
const root = man.engine.options.root || '/data';

function parseFlpack1(b) {
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  if (String.fromCharCode(...b.subarray(0, 4)) !== 'FLPK') throw new Error('dll pack: bad magic');
  const il = dv.getUint32(8, true);
  const idx = JSON.parse(new TextDecoder().decode(b.subarray(12, 12 + il)));
  return idx.files.map((f) => ({ path: f.path, data: b.subarray(12 + il + f.offset, 12 + il + f.offset + f.size) }));
}
function parseSeed(b) {
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  if (String.fromCharCode(...b.subarray(0, 8)) !== 'FLSEEDP1') throw new Error('seed pack: bad magic');
  const hl = dv.getUint32(8, true);
  const h = JSON.parse(new TextDecoder().decode(b.subarray(12, 12 + hl)));
  return h.files.map((f) => ({ path: f.path, data: b.subarray(12 + hl + f.offset, 12 + hl + f.offset + f.size) }));
}
const dllFiles = parseFlpack1(new Uint8Array(fs.readFileSync(dllPackFile)));
const seedFiles = parseSeed(new Uint8Array(fs.readFileSync(seedPackFile)));
const script = fs.readFileSync(scriptFile);
const wasmBytes = fs.readFileSync(wasmFile);

const env = Object.assign({}, man.engine.env);
env.FLE_VFS_SEED = root + '/seed/vfs-seed-base.tsv';
env.FLE_G3_SCRIPT = root + '/script.g3';
env.FLE_G3_PROGRESS = '1';
env.FLE_FRUITY_SECONDS = '36000';
env.FLE_FRUITY_MAX_CALLS = '400000000000';
for (const a of kv) { const i = a.indexOf('='); env[a.slice(0, i)] = a.slice(i + 1); }

const out = fs.createWriteStream(path.join(outDir, 'stdout.txt'));
const err = fs.createWriteStream(path.join(outDir, 'stderr.txt'));
let memory = null, exitCode = null, prunedLines = 0, stopLines = [];
const t0 = Date.now();
const opts = {
  arguments: man.engine.args,
  print: (l) => { out.write(l + '\n'); if (l.startsWith('g3-snap:')) { const u = process.threadCpuUsage(); out.write('parity1-maincpu: ' + ((u.user + u.system) / 1e6).toFixed(3) + ' s procuser ' + (process.cpuUsage().user / 1e6).toFixed(3) + ' procsys ' + (process.cpuUsage().system / 1e6).toFixed(3) + '\n'); } if (/pruned function/.test(l)) prunedLines++; if (/^fruity: == stop/.test(l)) stopLines.push(l); },
  printErr: (l) => { err.write(l + '\n'); if (/pruned function/.test(l)) prunedLines++; },
  onExit: (c) => { exitCode = c; },
  onAbort: (w) => { err.write('onAbort: ' + w + '\n'); },
  fleProgress: () => {},
  fleIdle: () => {},
  fleSurfaceEvent: () => {},
  fleThreadFailed: (i, e, m) => err.write('[js] ' + m + '\n'),
  preRun: [() => {
    const FS = opts.FS;
    const put = (g, d) => { FS.mkdirTree(g.slice(0, g.lastIndexOf('/'))); FS.writeFile(g, d, { canOwn: false }); };
    for (const f of dllFiles) put(root + '/' + f.path, f.data);
    for (const f of seedFiles) put(root + '/' + f.path, f.data);
    put(root + '/script.g3', script);
    // parity1: PARITY_SEED_TSV = a VFS seed list (e.g. lift/harness/seeds/vfs-seed-g8.tsv: Basic.flp, Sampler.flp ...); its host files (relative to PARITY_REPO) are written under /data/seed/<host path> unless the seed pack already has them
    if (process.env.PARITY_SEED_TSV) {
      const repo = process.env.PARITY_REPO;
      put(root + '/seed/parity-seed.tsv', fs.readFileSync(process.env.PARITY_SEED_TSV));
      for (const ln of fs.readFileSync(process.env.PARITY_SEED_TSV, 'utf8').split('\n')) {
        if (!ln || ln.startsWith('#')) continue;
        const f = ln.split('\t'); if (f.length < 2 || f[1] === '-') continue;
        const g = root + '/seed/' + f[1];
        try { FS.stat(g); } catch { try { put(g, fs.readFileSync(path.join(repo, f[1]))); } catch (e) { err.write('parity1: cannot add ' + f[1] + ': ' + e.message + '\n'); } }
      }
      env.FLE_VFS_SEED = root + '/seed/parity-seed.tsv';
    }
    for (const k of Object.keys(env)) opts.ENV[k] = env[k];
  }],
  instantiateWasm(imports, done) {
    WebAssembly.compile(wasmBytes).then((m) => WebAssembly.instantiate(m, imports).then((inst) => { memory = inst.exports.memory || imports.env.memory; done(inst, m); }))
      .catch((e) => { err.write('wasm compile/instantiate failed: ' + (e && e.stack || e) + '\n'); process.exitCode = 99; });
    return {};
  },
};
let error = null;
const mkSummary = () => ({ bundle: path.basename(bundle), wasm: path.basename(wasmFile), script: path.basename(scriptFile), exitCode, processExitCode: process.exitCode ?? null, error, prunedLines, stopLines, wallSeconds: (Date.now() - t0) / 1000, memBytes: memory ? memory.buffer.byteLength : 0, extraEnv: kv });
const finish = () => { try { fs.writeFileSync(path.join(outDir, 'summary.json'), JSON.stringify(mkSummary(), null, 1)); } catch {} };
process.on('exit', finish);   // the engine's end can leave the module promise unsettled (node then exits by itself): the summary is written on exit
globalThis.WorkerGlobalScope = globalThis.WorkerGlobalScope || class WorkerGlobalScope {};   // the glue is built for web+worker: node plays a worker (no worker API is used: the wasm, the files and the script are handed over here)
try {
  const { default: Module } = await import(pathToFileURL(glueCopy).href);
  await Module(opts);
} catch (e) { error = String((e && e.stack) || e); err.write('driver caught: ' + error + '\n'); }
