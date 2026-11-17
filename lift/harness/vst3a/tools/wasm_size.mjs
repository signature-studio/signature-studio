// Decision 0177 (srm1): size of a wasm file (raw, brotli q11 [slow, the best], brotli q5, gzip 9) and the node WebAssembly.compile / instantiate time.
// usage: node wasm_size.mjs <vst.wasm> [--no-compress]
import fs from 'node:fs';
import zlib from 'node:zlib';
const f = process.argv[2];
const b = fs.readFileSync(f);
const out = { file: f, raw: b.length };
if (!process.argv.includes('--no-compress')) {
  let t = performance.now();
  out.brotli_q5 = zlib.brotliCompressSync(b, { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 5, [zlib.constants.BROTLI_PARAM_LGWIN]: 24, [zlib.constants.BROTLI_PARAM_SIZE_HINT]: b.length } }).length;
  out.brotli_q5_ms = Math.round(performance.now() - t); t = performance.now();
  out.brotli_q11 = zlib.brotliCompressSync(b, { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 11, [zlib.constants.BROTLI_PARAM_LGWIN]: 24, [zlib.constants.BROTLI_PARAM_SIZE_HINT]: b.length } }).length;
  out.brotli_q11_ms = Math.round(performance.now() - t);
  out.gzip9 = zlib.gzipSync(b, { level: 9 }).length;
}
const t0 = performance.now();
const m = await WebAssembly.compile(b);
out.compile_ms_node = Math.round(performance.now() - t0);
out.imports = WebAssembly.Module.imports(m).length;
console.log(JSON.stringify(out));
