// Compression of the wasm for the size/time table: gzip -9 (the gzip binary), brotli quality 11 with a 16 MiB window (lgwin 24, the
// largest window a browser accepts for Content-Encoding: br; node's zlib = libbrotli) and, with --zstd, zstd -19 (window 8 MiB, the
// largest window a browser accepts for Content-Encoding: zstd). One file at a time, streams, output next to the input.
// usage: node compress.mjs <file> [--zstd] > result.json
import fs from 'node:fs';
import zlib from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { pipeline } from 'node:stream/promises';

const file = process.argv[2];
const size = fs.statSync(file).size;
const out = [];
const t = (label, f) => { const t0 = process.hrtime.bigint(); const extra = f(); const s = Number(process.hrtime.bigint() - t0) / 1e9; return { label, seconds: +s.toFixed(1), ...extra }; };

{
  const t0 = process.hrtime.bigint();
  const r = spawnSync('sh', ['-c', `gzip -9 -c "${file}" > "${file}.gz"`]);
  const s = Number(process.hrtime.bigint() - t0) / 1e9;
  out.push({ label: 'gzip -9', seconds: +s.toFixed(1), status: r.status, bytes: fs.statSync(file + '.gz').size });
}
{
  const t0 = process.hrtime.bigint();
  await pipeline(fs.createReadStream(file), zlib.createBrotliCompress({ params: {
    [zlib.constants.BROTLI_PARAM_QUALITY]: 11, [zlib.constants.BROTLI_PARAM_LGWIN]: 24, [zlib.constants.BROTLI_PARAM_SIZE_HINT]: size } }),
    fs.createWriteStream(file + '.br'));
  const s = Number(process.hrtime.bigint() - t0) / 1e9;
  out.push({ label: 'brotli q11 lgwin24', seconds: +s.toFixed(1), bytes: fs.statSync(file + '.br').size });
}
if (process.argv.includes('--zstd')) {
  const t0 = process.hrtime.bigint();
  const r = spawnSync('sh', ['-c', `zstd -19 --zstd=wlog=23 -T1 -q -f -c "${file}" > "${file}.zst"`]);
  const s = Number(process.hrtime.bigint() - t0) / 1e9;
  out.push({ label: 'zstd -19 wlog23', seconds: +s.toFixed(1), status: r.status, bytes: fs.statSync(file + '.zst').size });
}
console.log(JSON.stringify({ file, size, results: out }, null, 1));
