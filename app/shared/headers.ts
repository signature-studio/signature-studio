// Response headers every response of this app carries (dev server, preview, Cloudflare Worker).
// COOP same-origin + COEP require-corp make the page cross-origin isolated (SharedArrayBuffer; the engine's threads need it).
// Same values as lift/harness/web/present/lib.mjs (startServer) plus CORP, X-Robots-Tag and nosniff.

export const ISOLATION_HEADERS: Readonly<Record<string, string>> = {
  'Cross-Origin-Opener-Policy': 'same-origin',
  'Cross-Origin-Embedder-Policy': 'require-corp',
  'Cross-Origin-Resource-Policy': 'same-origin',
};

export const PRIVACY_HEADERS: Readonly<Record<string, string>> = {
  'X-Robots-Tag': 'noindex, nofollow',
  'X-Content-Type-Options': 'nosniff',
  'Referrer-Policy': 'no-referrer',
};

// No external origin may be referenced from the app: everything is 'self'. 'wasm-unsafe-eval' is what WebAssembly.compile needs under a CSP.
// script-src also allows blob: for ONE use: the engine Worker import()s the emscripten glue from a Blob made of the bytes whose SHA-256 the manifest names (src/engine/fruity.ts), so that the
// code that runs is exactly the code that was verified. Nothing else creates blob scripts.
export const CSP =
  "default-src 'self'; script-src 'self' 'wasm-unsafe-eval' blob:; style-src 'self'; img-src 'self' data: blob:; connect-src 'self'; worker-src 'self' blob:; " +
  "font-src 'self'; media-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

export const MIME: Readonly<Record<string, string>> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json',
  '.wasm': 'application/wasm',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.txt': 'text/plain; charset=utf-8',
  '.flpack': 'application/octet-stream',
  '.map': 'application/json',
};

// A request for NAME.gz / NAME.br is a request for the STORED bytes: they get a non-compressible type, so that nothing between the origin and the browser
// compresses them a second time (observed with `wrangler dev --local`: an application/wasm response without Content-Encoding was gzip-encoded on the way out).
export function mimeFor(path: string): string {
  if (/\.gz$/i.test(path)) return 'application/gzip';
  if (/\.br$/i.test(path)) return 'application/octet-stream';
  const i = path.lastIndexOf('.');
  const ext = i >= 0 ? path.slice(i).toLowerCase() : '';
  return MIME[ext] ?? 'application/octet-stream';
}

// A path under /assets/ whose name carries a content hash or a version segment is immutable; the manifests (mutable names) are not.
export function cacheControlFor(path: string): string {
  if (/\/(manifest[^/]*\.json)$/i.test(path)) return 'no-cache';
  if (/(^|[/.-])(v\d+|[0-9a-f]{8,})([/.-]|$)/i.test(path)) return 'public, max-age=31536000, immutable';
  return 'no-cache';
}
