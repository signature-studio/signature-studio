// `vite preview` (the production build + FL_ASSETS_DIR at /assets/ + the production headers) with an ACCESS LOG: one JSON line per request (time, method, url, status, bytes sent, If-None-Match, Range,
// Cache-Control / ETag / Content-Encoding of the response, duration). For the cache measurements of decision 0220. Usage:
//   FL_ASSETS_DIR=<dir> FL_PERF_LOG=<file> node test/e2e/preview-logged.mjs --port 4962 --log access.jsonl [--outDir dist-other]
import fs from 'node:fs';
import { preview } from 'vite';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const port = Number(opt('port', '4962'));
const logFile = opt('log', 'access.jsonl');
const outDir = opt('outDir', '');
fs.writeFileSync(logFile, '');

const accessLog = {
  name: 'access-log',
  enforce: 'pre',
  configurePreviewServer(server) {
    server.middlewares.use((req, res, next) => {
      const t0 = Date.now();
      let bytes = 0;
      const w = res.write.bind(res), e = res.end.bind(res);
      res.write = (c, ...a) => { bytes += c ? c.length : 0; return w(c, ...a); };
      res.end = (c, ...a) => { if (c && typeof c !== 'function') bytes += c.length; return e(c, ...a); };
      res.on('close', () => {
        fs.appendFileSync(logFile, JSON.stringify({ t: t0, ms: Date.now() - t0, method: req.method, url: req.url, status: res.statusCode, bytes, complete: res.writableFinished, inm: req.headers['if-none-match'] ?? null, range: req.headers.range ?? null, cc: res.getHeader('cache-control') ?? null, etag: res.getHeader('etag') ?? null, ce: res.getHeader('content-encoding') ?? null, ua: String(req.headers['user-agent'] ?? '').slice(0, 40) }) + '\n');
      });
      next();
    });
  },
};
const server = await preview({ ...(outDir ? { build: { outDir } } : {}), preview: { host: '127.0.0.1', port, strictPort: true }, plugins: [accessLog] });
server.printUrls();
