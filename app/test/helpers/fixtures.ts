// Fixture objects shared by the asset-suite (run against the file server and against the Worker + local R2). No vitest import: scripts use this too.
import zlib from 'node:zlib';

// deterministic, incompressible-ish bytes
export function blobBytes(n: number): Buffer {
  const out = Buffer.alloc(n);
  let x = 0x12345678;
  for (let i = 0; i < n; i++) { x = (Math.imul(x, 1664525) + 1013904223) >>> 0; out[i] = x >>> 24; }
  return out;
}

export const FIXTURE = {
  blob: blobBytes(100_000),
  wasm: Buffer.from('0061736d01000000', 'hex'), // an empty but valid module
  pre: Buffer.from(Array.from({ length: 5000 }, (_, i) => (i * 7) & 0xff)),
};
export const PRE_BR = zlib.brotliCompressSync(FIXTURE.pre);
export const PRE_GZ = zlib.gzipSync(FIXTURE.pre);


// decision 0152: what the default manifest path may serve. Only engine.kind matters to the server; the rest is a plausible manifest.
const manifestWith = (kind: string): string => JSON.stringify({ schema: 1, bundle: { name: kind === 'unknown-engine-kind-v0' ? 'refused-kind-test' : 'fl-test', version: '1' }, engine: { kind, wasm: { url: '/assets/x/engine.wasm', size: 8, sha256: '0'.repeat(64), compression: 'identity' }, args: [], env: {} }, seedPack: null, dllPack: null }, null, 2) + '\n';
export const MANIFEST_REAL = manifestWith('emscripten-fruity-v1');
export const MANIFEST_UNKNOWN_KIND = manifestWith('unknown-engine-kind-v0');
/** the words every refusal must carry (the PM's text, decision 0152) */
export const REFUSAL_WORDS = ['このパスは実エンジンのバンドル専用です', 'engine.kind "unknown-engine-kind-v0" は実エンジンの種類(emscripten-fruity-v1)ではありません', 'docs/policies/progress-integrity.md'];
export const NOT_DEPLOYED_WORDS = ['実エンジンのバンドルが未配置です', '/assets/manifest.json', 'docs/policies/progress-integrity.md'];
