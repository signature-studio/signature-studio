#!/usr/bin/env node
// tools/vst3-link/check.mjs <absolute path to the bundle directory raw/<name>> : does the default engine build contain the VST3 link (harness/two/wrp1_vst3.c, linked by lift/harness/two/harness-inputs.txt)?
// Prints one JSON line on stdout: {"linked","hookSymbols","error"}. Exit 0 when the check ran, 1 when it could not read the bundle, 2 when no directory is given.
// The engine wasm is read from the manifest (raw/<name>/manifest.json: engine.wasm), in its brotli form under ../../br/<name>/ (the same layout as tools/boot-check/boot-bundle.mjs), and its sha256 must be the manifest's.
// The build keeps no `name` section (emcc drops it), so a hook is found by a mark: a string that only that hook's own code holds (its printf or fprintf format). A hook counts when its mark is in the wasm's data.
// linked is true when all the marks are there. hookSymbols lists the hooks whose mark is there. The weak hooks of fle_ole.c and the GetFileType/SetFilePointerEx/... wraps are not marked; they are in the link list (lift/harness/web/link.rsp).
// usage: node tools/vst3-link/check.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz"
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const HOOKS = [
  { symbol: 'fle_realmod_pre_attach', mark: 'VST3 TLS callback %#llx(DLL_PROCESS_ATTACH)' },
  { symbol: 'fle_realmod_pre_thread', mark: 'wrp3: TLS block %#llx for slot' },
  { symbol: 'wrp1_after_image_load', mark: "VST3 IAT slots set to the row's pseudo-VA" },
];

function report(obj, code) {
  process.stdout.write(JSON.stringify(obj) + '\n');
  process.exitCode = code;
}

function engineWasm(bundleDir) {
  const manifest = JSON.parse(fs.readFileSync(path.join(bundleDir, 'manifest.json'), 'utf8'));
  const info = manifest.engine?.wasm;
  if (!info || typeof info.url !== 'string' || typeof info.sha256 !== 'string') throw new Error('manifest.json has no engine.wasm');
  const brDir = path.join(bundleDir, '..', '..', 'br', path.basename(bundleDir));
  const stored = fs.readFileSync(path.join(brDir, path.basename(info.url)));
  const wasm = info.compression === 'br' ? zlib.brotliDecompressSync(stored) : stored;
  const sha = crypto.createHash('sha256').update(wasm).digest('hex');
  if (sha !== info.sha256) throw new Error(`the engine wasm's sha256 is ${sha}, the manifest says ${info.sha256}`);
  return wasm;
}

if (!process.argv[2]) {
  process.stderr.write('usage: node tools/vst3-link/check.mjs <absolute path to the bundle directory raw/<name>>\n');
  process.exit(2);
}

try {
  const text = engineWasm(path.resolve(process.argv[2])).toString('latin1');
  const hookSymbols = HOOKS.filter((h) => text.includes(h.mark)).map((h) => h.symbol);
  report({ linked: hookSymbols.length === HOOKS.length, hookSymbols, error: null }, 0);
} catch (e) {
  report({ linked: false, hookSymbols: [], error: String(e?.message ?? e).slice(0, 500) }, 1);
}
