// Local HTTP server for the browser measurements: no dependency, one file.
//   /                      the files of this directory
//   /dist/...              $WEB_DIST (flengine6.wasm, flengine6.web.js, data/); /dist/flengine6.wasm?enc=gzip|br|zstd serves the
//                          pre-compressed file next to it (flengine6.wasm.gz|.br|.zst) with Content-Encoding
//   /coi/...               the same, with Cross-Origin-Opener-Policy: same-origin and Cross-Origin-Embedder-Policy: require-corp
// ?rate=<MB/s> limits the bytes on the wire of that response (loopback is otherwise unlimited).
// ?cache=1 on a request allows the browser to cache it (Cache-Control: public, max-age=3600); the default is no-store.
// Each response of the wasm is logged with the time from the request to the end of the body (server side, the bytes handed to the kernel).
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { Transform } from 'node:stream';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm', '.json': 'application/json', '.css': 'text/css' };
const ENC = { gzip: ['.gz', 'gzip'], br: ['.br', 'br'], zstd: ['.zst', 'zstd'] };

export function startServer({ dist, port = 0, log = [] } = {}) {
  const server = http.createServer((req, res) => {
    const t0 = process.hrtime.bigint();
    const u = new URL(req.url, 'http://x');
    let p = decodeURIComponent(u.pathname);
    const headers = {};
    if (p.startsWith('/coi/')) {
      p = p.slice(4);
      headers['Cross-Origin-Opener-Policy'] = 'same-origin';
      headers['Cross-Origin-Embedder-Policy'] = 'require-corp';
    }
    if (p === '/') p = '/index.html';
    let file;
    if (p.startsWith('/dist/')) file = path.join(dist, p.slice(6)); else file = path.join(HERE, p);
    if (!file.startsWith(p.startsWith('/dist/') ? dist : HERE) || file.includes('..')) { res.writeHead(403).end('forbidden'); return; }
    const enc = u.searchParams.get('enc');
    let encName = null;
    if (enc && ENC[enc] && p.endsWith('.wasm')) { file += ENC[enc][0]; encName = ENC[enc][1]; }
    fs.stat(file, (err, st) => {
      if (err || !st.isFile()) { res.writeHead(404).end('not found: ' + p); return; }
      const base = file.replace(/\.(gz|br|zst)$/, '');
      headers['Content-Type'] = TYPES[path.extname(base)] || 'application/octet-stream';
      headers['Content-Length'] = st.size;
      headers['Cache-Control'] = u.searchParams.get('cache') === '1' ? 'public, max-age=3600' : 'no-store';
      if (encName) headers['Content-Encoding'] = encName;
      res.writeHead(200, headers);
      const s = fs.createReadStream(file);
      const rate = +(u.searchParams.get('rate') || 0);   // MB/s on the wire (the encoded bytes); 0 = unlimited
      if (rate > 0) {
        const start = process.hrtime.bigint();
        let sent = 0;
        const throttle = new Transform({ transform(chunk, _enc, cb) {
          sent += chunk.length;
          const due = sent / (rate * 1048576) * 1000;
          const wait = due - Number(process.hrtime.bigint() - start) / 1e6;
          if (wait > 0) setTimeout(() => cb(null, chunk), wait); else cb(null, chunk);
        } });
        s.pipe(throttle).pipe(res);
      } else s.pipe(res);
      res.on('close', () => {
        if (st.size > 1e6) log.push({ url: req.url, bytes: st.size, enc: encName || 'identity', ms: +(Number(process.hrtime.bigint() - t0) / 1e6).toFixed(1), complete: res.writableFinished });
      });
    });
  });
  return new Promise((resolve) => server.listen(port, '127.0.0.1', () => resolve({ server, port: server.address().port, log })));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const dist = process.env.WEB_DIST || path.join(HERE, 'dist');
  const { port } = await startServer({ dist, port: +(process.env.PORT || 8080) });
  console.log(`serving ${HERE} and ${dist} on http://127.0.0.1:${port}/`);
}
