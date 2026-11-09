// perf1 (decision 0130, PROVISIONAL): compress files with gzip (zlib level 9) and brotli (quality 11, lgwin 24) and write <file>.gz / <file>.br next to them, one JSON line per file with sizes and times.
// usage: node compress.mjs FILE... [--out-json FILE] [--only gz|br]
// Streaming (constant memory); the files are processed in parallel (one stream pair per file). The sizes are of the encoded bytes that the page's server sends with Content-Encoding.
import fs from 'node:fs';
import zlib from 'node:zlib';
import { pipeline } from 'node:stream/promises';

const args = process.argv.slice(2);
const files = [];
let outJson = null, only = null;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--out-json') outJson = args[++i];
  else if (args[i] === '--only') only = args[++i];
  else files.push(args[i]);
}
async function one(file, kind) {
  const t0 = Date.now();
  const dst = `${file}.${kind}`;
  const enc = kind === 'gz'
    ? zlib.createGzip({ level: 9, memLevel: 9 })
    : zlib.createBrotliCompress({ params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 11, [zlib.constants.BROTLI_PARAM_LGWIN]: 24, [zlib.constants.BROTLI_PARAM_SIZE_HINT]: fs.statSync(file).size } });
  await pipeline(fs.createReadStream(file, { highWaterMark: 1 << 20 }), enc, fs.createWriteStream(dst));
  return { file, kind, raw: fs.statSync(file).size, encoded: fs.statSync(dst).size, seconds: +((Date.now() - t0) / 1000).toFixed(1) };
}
const jobs = [];
for (const f of files) for (const k of ['gz', 'br']) if (!only || only === k) jobs.push(one(f, k));
const res = await Promise.all(jobs);
for (const r of res) console.log(JSON.stringify(r));
if (outJson) fs.writeFileSync(outJson, JSON.stringify(res, null, 1));
