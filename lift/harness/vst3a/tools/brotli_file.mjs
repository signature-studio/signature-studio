import fs from 'node:fs'; import zlib from 'node:zlib';
const b = fs.readFileSync(process.argv[2]); const t = performance.now();
const q = (n) => zlib.brotliCompressSync(b, { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: n, [zlib.constants.BROTLI_PARAM_LGWIN]: 24, [zlib.constants.BROTLI_PARAM_SIZE_HINT]: b.length } }).length;
console.log(JSON.stringify({ file: process.argv[2], raw: b.length, brotli_q11: q(11), gzip9: zlib.gzipSync(b, { level: 9 }).length, ms: Math.round(performance.now() - t) }));
