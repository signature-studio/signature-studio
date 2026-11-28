// A forward proxy for the browser (Issue #1408). Production's access service adds the application token to every request of the page, the audio
// worklet's module included (a worklet's fetch carries none of the page's extra headers, so a local delivery answers it with 403). The browser sends its
// requests to the delivery's origin through this proxy, and the proxy adds the token to a request that has none. Only the delivery's origin is served;
// any other host gets 502. The token is the run's (lib/access.mjs), so the Worker verifies it as it verifies the page's.
import http from 'node:http';

const DROP = ['connection', 'proxy-connection', 'keep-alive', 'transfer-encoding', 'upgrade'];

export async function startProxy({ origin, token }) {
  const target = new URL(origin);
  const server = http.createServer((req, res) => {
    let url;
    try { url = new URL(req.url, origin); } catch { res.writeHead(400).end(); return; }
    if (url.host !== target.host) { res.writeHead(502).end(); return; }
    const headers = {};
    for (const [k, v] of Object.entries(req.headers)) if (!DROP.includes(k)) headers[k] = v;
    headers.host = target.host;
    if (!headers['cf-access-jwt-assertion']) headers['cf-access-jwt-assertion'] = token;
    const up = http.request({ host: target.hostname, port: target.port, path: url.pathname + url.search, method: req.method, headers }, (r) => {
      res.writeHead(r.statusCode ?? 502, r.headers);
      r.pipe(res);
    });
    up.on('error', () => { if (!res.headersSent) res.writeHead(502); res.end(); });
    req.pipe(up);
  });
  server.on('connect', (_req, sock) => { sock.end('HTTP/1.1 502 Bad Gateway\r\n\r\n'); });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  return { port: server.address().port, close: () => new Promise((r) => server.close(() => r())) };
}
