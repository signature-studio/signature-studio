// mkbr.mjs DIR: NAME.br next to every asset (brotli quality 6; the local server sends it with Content-Encoding like the Worker does)
import fs from 'node:fs'; import zlib from 'node:zlib'; import path from 'node:path';
const d = process.argv[2];
for (const n of fs.readdirSync(d)) {
  if (n.endsWith('.br') || n === 'manifest.json') continue;
  const b = fs.readFileSync(path.join(d, n));
  fs.writeFileSync(path.join(d, n + '.br'), zlib.brotliCompressSync(b, { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 6, [zlib.constants.BROTLI_PARAM_LGWIN]: 24, [zlib.constants.BROTLI_PARAM_SIZE_HINT]: b.length } }));
}
