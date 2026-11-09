#!/usr/bin/env node
// Builds an engine BUNDLE directory: manifest.json, the engine wasm, the emscripten glue, the seed pack (the guest's virtual files, FLSEEDP1) and the DLL pack, with SHA-256s, brotli-compressed variants (.br next to
// each file), and the commands that upload it to R2. It never uploads anything and never needs credentials: the commands are only printed (and written to upload-commands.txt) for the operator to run after `wrangler login`.
//
//   node scripts/pack-assets.mjs --out DIR --name NAME --version VER --engine-wasm FILE --glue FILE --seed-pack FILE --dll-dir DIR --host-files emscripten-X.Y.Z --args "/data/FLEngine_x64.dll /data/mod1.dll:/data/mod2.dll"
//        [--env K=V]... [--root /data] [--fruity-hints first=49,stop=55,variant=Oz] [--stop-note "TEXT CONTAINED IN THE ENGINE'S STOP MESSAGE=>NOTE"]... [--brotli-quality 11] [--bucket NAME] [--prefix KEY_PREFIX] [--description TEXT]
//   scripts/build-engine-bundle.sh calls it (decision 0137). --glue is the emscripten ES module flengine6.js (-sMODULARIZE=1 -sEXPORT_ES6=1) linked with -sFILESYSTEM=0 by the Emscripten named in --host-files (decision 0221: the page
//   answers the engine's file reads from the pack bytes). --seed-pack is an FLSEEDP1 pack (lift/harness/web/final0129/pack_seed.py), --dll-dir the engine and module DLLs (packed as flpack1: pack path = file name).
//   --stop-note texts are shown by the UI when the engine's stop message contains the text before `=>` (repeatable). The engine wasm must export the desktop compositor (fle_desk_compose, fle_desk_resync): the page shows
//   the engine through it and has no other way (decision 0223).
//
// Files in the output directory:  engine.<12 hex of sha256>.wasm[.br]   seed.<12 hex>.flpack[.br]   dll.<12 hex>.flpack[.br]   manifest.json   SHA256SUMS   upload-commands.txt
// FL_ASSETS_DIR=<DIR>/.. (the parent of PREFIX, see README) serves them at /assets/ for `npm run dev` and `npm run preview`.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { buildPack } from '../src/bundle/pack.ts';
import { validateManifest, formatErrors } from '../src/bundle/manifest.ts';
import { wasmHasDesk } from './lib/wasm-exports.mjs';

function parseArgs(argv) {
  const a = { env: {}, flags: new Set() };
  for (let i = 0; i < argv.length; i++) {
    const t = argv[i];
    if (!t.startsWith('--')) throw new Error(`unexpected argument ${t}`);
    const k = t.slice(2);
    if (k === 'help') { a.flags.add(k); continue; }
    const v = argv[++i];
    if (v === undefined) throw new Error(`--${k} needs a value`);
    if (k === 'env') { const j = v.indexOf('='); if (j < 1) throw new Error('--env K=V'); a.env[v.slice(0, j)] = v.slice(j + 1); }
    else if (k === 'stop-note') (a[k] ??= []).push(v);
    else a[k] = v;
  }
  return a;
}

const sha256File = (f) => new Promise((res, rej) => { const h = crypto.createHash('sha256'); fs.createReadStream(f).on('data', (d) => h.update(d)).on('end', () => res(h.digest('hex'))).on('error', rej); });

function walk(dir, base = dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true }).sort((x, y) => (x.name < y.name ? -1 : 1))) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) walk(full, base, out);
    else if (e.isFile()) out.push({ path: path.relative(base, full).split(path.sep).join('/'), data: new Uint8Array(fs.readFileSync(full)) });
  }
  return out;
}

async function compressFile(src, dst, quality) {
  const { pipeline } = await import('node:stream/promises');
  const t = zlib.createBrotliCompress({ params: { [zlib.constants.BROTLI_PARAM_QUALITY]: quality, [zlib.constants.BROTLI_PARAM_SIZE_HINT]: fs.statSync(src).size, [zlib.constants.BROTLI_PARAM_LGWIN]: 24 } });
  await pipeline(fs.createReadStream(src), t, fs.createWriteStream(dst));
  return fs.statSync(dst).size;
}

function parseKv(text) { const o = {}; for (const part of String(text ?? '').split(',').filter(Boolean)) { const j = part.indexOf('='); if (j < 1) throw new Error(`bad key=value list: ${text}`); o[part.slice(0, j)] = part.slice(j + 1); } return o; }

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.flags.has('help') || !args.out) { console.log(fs.readFileSync(fileURLToPath(import.meta.url), 'utf8').split('\n').slice(1).filter((l, i, all) => all.slice(0, i + 1).every((x) => x.startsWith('//'))).map((l) => l.slice(3)).join('\n')); process.exit(args.flags.has('help') ? 0 : 2); }
  const out = path.resolve(args.out);
  fs.mkdirSync(out, { recursive: true });
  const prefix = (args.prefix ?? '').replace(/^\/+|\/+$/g, '');
  const urlBase = '/assets/' + (prefix ? prefix + '/' : '');
  const quality = Number(args['brotli-quality'] ?? 11);
  const bucket = args.bucket ?? 'BUCKET-NAME';

  for (const k of ['name', 'version', 'engine-wasm', 'glue', 'seed-pack', 'dll-dir', 'host-files', 'args']) if (!args[k]) throw new Error(`--${k} is required`);
  const { name, version, description } = args;
  const wasmSrc = path.resolve(args['engine-wasm']);
  const dllFiles = walk(path.resolve(args['dll-dir']));

  const written = []; // {file, key, contentType, contentEncoding?, cacheControl}
  async function emit(srcFile, base, ext, contentType) {
    const sha = await sha256File(srcFile);
    const fname = `${base}.${sha.slice(0, 12)}.${ext}`;
    const dst = path.join(out, fname);
    if (path.resolve(srcFile) !== dst) fs.copyFileSync(srcFile, dst);
    const size = fs.statSync(dst).size;
    const vf = dst + '.br';
    const encSize = await compressFile(dst, vf, quality);
    written.push({ file: dst, key: prefix ? `${prefix}/${fname}` : fname, contentType, cacheControl: 'public, max-age=31536000, immutable' });
    written.push({ file: vf, key: prefix ? `${prefix}/${fname}` : fname, contentType, contentEncoding: 'br', cacheControl: 'public, max-age=31536000, immutable', alternative: true });
    return { url: urlBase + fname, size, sha256: sha, compression: 'br', encodedSize: encSize };
  }

  const wasm = await emit(wasmSrc, 'engine', 'wasm', 'application/wasm');
  // decision 0223 (desk2): the page shows the engine through the runtime's desktop compositor and has no other way, so a wasm without it is not a bundle
  if (!wasmHasDesk(wasmSrc)) throw new Error('this engine wasm does not export the desktop compositor (fle_desk_compose, fle_desk_resync): build the runtime with fle_desk.c');
  const hints = parseKv(args['fruity-hints']);
  const stopNotes = [];
  for (const t of args['stop-note'] ?? []) { const j = t.indexOf('=>'); if (j < 1) throw new Error('--stop-note "CONTAINS=>NOTE"'); stopNotes.push({ contains: t.slice(0, j), note: t.slice(j + 2) }); }
  const options = {
    glue: await emit(path.resolve(args.glue), 'engine-glue', 'js', 'text/javascript'),
    root: args.root ?? '/data',
    hostFilesAbi: args['host-files'],
    ...(hints.first !== undefined ? { hints: { firstPixelSeconds: Number(hints.first), ...(hints.stop !== undefined ? { stopSeconds: Number(hints.stop) } : {}), ...(hints.variant ? { variant: hints.variant } : {}) } } : {}),
    ...(stopNotes.length ? { stopNotes } : {}),
  };
  const tmp = path.join(out, '.tmp'); fs.mkdirSync(tmp, { recursive: true });
  async function packRef(files, base) {
    const f = path.join(tmp, base + '.flpack');
    fs.writeFileSync(f, buildPack(files));
    return { ...(await emit(f, base, 'flpack', 'application/octet-stream')), format: 'flpack1' };
  }
  // a seed pack in the FLSEEDP1 format (lift/harness/web/final0129/pack_seed.py, decision 0134): named in the manifest as it is
  const seedFile = path.resolve(args['seed-pack']);
  const magic = fs.readFileSync(seedFile).subarray(0, 8).toString('latin1');
  if (magic !== 'FLSEEDP1') throw new Error(`${seedFile} is not an FLSEEDP1 pack (magic ${JSON.stringify(magic)})`);
  const seedPack = { ...(await emit(seedFile, 'seed', 'flpack', 'application/octet-stream')), format: 'flseedp1' };
  const dllPack = await packRef(dllFiles, 'dll');
  fs.rmSync(tmp, { recursive: true, force: true });

  const manifest = {
    schema: 1,
    bundle: { name, version, created: new Date().toISOString(), ...(description ? { description } : {}) },
    engine: { kind: 'emscripten-fruity-v1', wasm, args: args.args.split(/\s+/).filter(Boolean), env: args.env, options },
    seedPack, dllPack,
  };
  const v = validateManifest(manifest);
  if (!v.ok) throw new Error('the manifest this script built is invalid:\n' + formatErrors(v.errors));
  fs.writeFileSync(path.join(out, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');

  // checksums of everything in the directory, and the upload commands (printed, written, NOT run)
  const sums = [];
  for (const f of fs.readdirSync(out).sort()) if (f !== 'SHA256SUMS' && f !== 'upload-commands.txt') sums.push(`${await sha256File(path.join(out, f))}  ${f}`);
  fs.writeFileSync(path.join(out, 'SHA256SUMS'), sums.join('\n') + '\n');

  const cmds = [
    '# Upload commands. NOT executed by pack-assets.mjs. Run them yourself after `wrangler login` and after creating the bucket:',
    `#   wrangler r2 bucket create ${bucket}`,
    '# Objects whose key ends in .wasm / .flpack are stored ENCODED (brotli) with --content-encoding br: the Worker sends Content-Encoding: br and the browser decodes it.',
  ];
  const q = (s) => `'${s.replace(/'/g, `'\\''`)}'`;
  const keysWithBr = new Set(written.filter((w) => w.contentEncoding === 'br').map((w) => w.key));
  for (const w of written) {
    if (!w.alternative && keysWithBr.has(w.key)) continue;      // one upload per key: the brotli variant when there is one
    cmds.push(`wrangler r2 object put ${q(`${bucket}/${w.key}`)} --remote --file ${q(w.file)} --content-type ${q(w.contentType)}${w.contentEncoding ? ` --content-encoding ${w.contentEncoding}` : ''} --cache-control ${q(w.cacheControl)}`);
  }
  const mkey = prefix ? `${prefix}/manifest.json` : 'manifest.json';
  cmds.push(`wrangler r2 object put ${q(`${bucket}/${mkey}`)} --remote --file ${q(path.join(out, 'manifest.json'))} --content-type application/json --cache-control no-cache`);
  fs.writeFileSync(path.join(out, 'upload-commands.txt'), cmds.join('\n') + '\n');

  console.log(`bundle ${name} ${version} -> ${out}`);
  console.log(`  engine wasm  ${wasm.size} B  sha256 ${wasm.sha256}  stored as ${wasm.compression}${wasm.encodedSize ? ` (${wasm.encodedSize} B)` : ''}`);
  console.log(`  seed pack    ${seedPack.size} B  sha256 ${seedPack.sha256}  ${seedPack.format}`);
  console.log(`  dll pack     ${dllPack.size} B  sha256 ${dllPack.sha256}  ${dllFiles.length} files`);
  console.log(`  manifest     ${path.join(out, 'manifest.json')}`);
  console.log(`  upload       ${path.join(out, 'upload-commands.txt')} (commands only; nothing was uploaded)`);
}

main().catch((e) => { console.error(String(e.message ?? e)); process.exit(1); });
