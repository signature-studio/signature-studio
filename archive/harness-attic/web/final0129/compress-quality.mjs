// final0129 (decision 0134, PROVISIONAL): compress one file with brotli at several qualities (and gzip level 9) and record, for every setting, the encoded size, the wall time and the CPU time of the encode (process.cpuUsage()
// user+system: the zlib work runs on libuv pool threads and is part of the process' rusage) and the wall and CPU time of the decode with node's zlib (the same decoder family as the browser's, a single thread).
// usage: node compress-quality.mjs FILE --br 5,9,11 [--gz 9] [--lgwin 24] [--keep <q|gz>[,...]] --out-dir DIR --json OUT.json
//   --keep: the encoded file of those settings is kept as FILE.br / FILE.gz next to FILE (the file the page's server sends with Content-Encoding); the others are written to --out-dir and deleted after the decode.
// Streaming (constant memory). One setting at a time, so the CPU times do not overlap.
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { pipeline } from 'node:stream/promises';
import { Writable } from 'node:stream';

const args = process.argv.slice(2);
const file = args[0];
let brQ = [], gz = [], lgwin = 24, keep = new Set(), outDir = '/tmp/opencode/wasm0129/compress', jsonOut = null;
for (let i = 1; i < args.length; i++) {
  if (args[i] === '--br') brQ = args[++i].split(',').map(Number);
  else if (args[i] === '--gz') gz = args[++i].split(',').map(Number);
  else if (args[i] === '--lgwin') lgwin = +args[++i];
  else if (args[i] === '--keep') keep = new Set(args[++i].split(','));
  else if (args[i] === '--out-dir') outDir = args[++i];
  else if (args[i] === '--json') jsonOut = args[++i];
}
fs.mkdirSync(outDir, { recursive: true });
const size = fs.statSync(file).size;
const cpuMs = (a, b) => ((b.user - a.user) + (b.system - a.system)) / 1000;
const results = [];
async function one(kind, level) {
  const tag = kind === 'br' ? `br-q${level}` : `gz-${level}`;
  const kept = keep.has(kind === 'br' ? String(level) : 'gz');
  const dst = kept ? `${file}.${kind}` : path.join(outDir, path.basename(file) + '.' + tag);
  const enc = kind === 'br'
    ? zlib.createBrotliCompress({ params: { [zlib.constants.BROTLI_PARAM_QUALITY]: level, [zlib.constants.BROTLI_PARAM_LGWIN]: lgwin, [zlib.constants.BROTLI_PARAM_SIZE_HINT]: size } })
    : zlib.createGzip({ level, memLevel: 9 });
  const c0 = process.cpuUsage(), t0 = process.hrtime.bigint();
  await pipeline(fs.createReadStream(file, { highWaterMark: 1 << 20 }), enc, fs.createWriteStream(dst));
  const wall = Number(process.hrtime.bigint() - t0) / 1e9, cpu = cpuMs(c0, process.cpuUsage()) / 1000;
  const encoded = fs.statSync(dst).size;
  const dec = kind === 'br' ? zlib.createBrotliDecompress() : zlib.createGunzip();
  let n = 0;
  const sink = new Writable({ write(chunk, _e, cb) { n += chunk.length; cb(); } });
  const d0 = process.cpuUsage(), u0 = process.hrtime.bigint();
  await pipeline(fs.createReadStream(dst, { highWaterMark: 1 << 20 }), dec, sink);
  const dwall = Number(process.hrtime.bigint() - u0) / 1e9, dcpu = cpuMs(d0, process.cpuUsage()) / 1000;
  if (n !== size) throw new Error(`${tag}: decoded ${n} != ${size}`);
  if (!kept) fs.rmSync(dst);
  const r = { file: path.basename(file), setting: tag, raw: size, encoded, ratio_pct: +(100 * encoded / size).toFixed(2), enc_wall_s: +wall.toFixed(1), enc_cpu_s: +cpu.toFixed(1), dec_wall_s: +dwall.toFixed(2), dec_cpu_s: +dcpu.toFixed(2), kept };
  console.log(JSON.stringify(r));
  results.push(r);
}
for (const q of brQ) await one('br', q);
for (const l of gz) await one('gz', l);
if (jsonOut) fs.writeFileSync(jsonOut, JSON.stringify(results, null, 1));
