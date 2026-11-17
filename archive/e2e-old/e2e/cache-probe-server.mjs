// Decision 0220: a small HTTP server for the browser-cache probes. It serves incompressible bytes of a chosen size at /f/<n>MB (n MiB; ?v=<x> makes another URL) with the headers the Worker
// sends for a hashed asset (ETag, Cache-Control public max-age=31536000 immutable), and any file under --root at /a/<path> with the same headers (Content-Encoding by suffix .br).
// Every request is logged (JSON lines: time, method, path, If-None-Match, Range, status, bytes). GET /__log returns the log, GET /__clear empties it.
// Usage: node test/e2e/cache-probe-server.mjs --port 8801 [--root DIR] [--cc "public, max-age=31536000, immutable"] [--vary]
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import crypto from 'node:crypto';

const arg = (n, d) => { const i = process.argv.indexOf('--' + n); return i >= 0 ? process.argv[i + 1] : d; };
const port = Number(arg('port', '8801'));
const root = arg('root', '');
const cc = arg('cc', 'public, max-age=31536000, immutable');
const vary = process.argv.includes('--vary');
const bodies = new Map();
const log = [];
const iso = { 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp', 'Cross-Origin-Resource-Policy': 'same-origin' };

function randomBody(mib, v) {
  const key = mib + ':' + v;
  if (!bodies.has(key)) {
    const b = Buffer.alloc(Math.round(mib * 1024 * 1024));
    const seed = crypto.createHash('sha256').update(key).digest();
    for (let o = 0; o < b.length; o += 32) { seed.copy(b, o, 0, Math.min(32, b.length - o)); seed[0] = (seed[0] + 1) & 255; seed[1] = (seed[1] ^ o) & 255; }
    // make the bytes look random enough for a compressor: xor with a running LCG
    let x = (seed[2] << 8 | seed[3]) | 1;
    for (let o = 0; o < b.length; o++) { x = (Math.imul(x, 1103515245) + 12345) >>> 0; b[o] ^= x >>> 16; }
    bodies.set(key, b);
  }
  return bodies.get(key);
}

function send(req, res, body, headers, rec) {
  const etag = '"' + crypto.createHash('sha1').update(body.subarray(0, 4096)).update(String(body.length)).digest('hex').slice(0, 16) + '"';
  const h = { ...iso, ...headers, ETag: etag, 'Cache-Control': cc, 'Accept-Ranges': 'bytes', ...(vary ? { Vary: 'Accept-Encoding' } : {}) };
  rec.inm = req.headers['if-none-match'] ?? '';
  rec.range = req.headers.range ?? '';
  if (rec.inm === etag) { rec.status = 304; rec.bytes = 0; res.writeHead(304, h); res.end(); return; }
  rec.status = 200; rec.bytes = body.length;
  res.writeHead(200, { ...h, 'Content-Length': body.length });
  res.end(body);
}

http.createServer((req, res) => {
  const u = new URL(req.url, 'http://x');
  const rec = { t: Date.now(), method: req.method, path: u.pathname + u.search };
  if (u.pathname === '/__log') { res.writeHead(200, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store', ...iso }); res.end(JSON.stringify(log)); return; }
  if (u.pathname === '/__clear') { log.length = 0; res.writeHead(200, { 'Cache-Control': 'no-store', ...iso }); res.end('ok'); return; }
  log.push(rec);
  res.on('finish', () => { rec.done = Date.now(); });
  let m;
  if ((m = /^\/f\/([\d.]+)MB$/.exec(u.pathname))) { send(req, res, randomBody(Number(m[1]), u.searchParams.get('v') ?? ''), { 'Content-Type': 'application/octet-stream' }, rec); return; }
  if (u.pathname.startsWith('/a/') && root) {
    const f0 = path.resolve(root, '.' + path.posix.normalize('/' + u.pathname.slice(3)));
    const f = fs.existsSync(f0 + '.br') && !/\.json$/.test(f0) ? f0 + '.br' : f0;
    if (!f0.startsWith(path.resolve(root)) || !fs.existsSync(f)) { rec.status = 404; res.writeHead(404, iso); res.end('nf'); return; }
    const body = fs.readFileSync(f);
    const br = f !== f0;
    const type = /\.wasm$/.test(f0) ? 'application/wasm' : /\.js$/.test(f0) ? 'text/javascript' : /\.json$/.test(f0) ? 'application/json' : 'application/octet-stream';
    send(req, res, body, { 'Content-Type': type, ...(br ? { 'Content-Encoding': 'br' } : {}) }, rec);
    return;
  }
  if (u.pathname === '/p.html') { res.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store', ...iso }); res.end('<!doctype html><title>p</title>'); rec.status = 200; return; }
  rec.status = 404; res.writeHead(404, iso); res.end('nf');
}).listen(port, '127.0.0.1', () => console.log('probe server on', port));
