// A raw HTTP client for the header measurements: the body is counted but not decoded, headers keep their order, no cache, no redirect.
import http from 'node:http';
import https from 'node:https';

export function rawRequest(url, { method = 'GET', headers = {}, readBody = true, timeoutMs = 120000 } = {}) {
  return new Promise((resolve, reject) => {
    const u = new URL(url);
    const lib = u.protocol === 'https:' ? https : http;
    const t0 = performance.now();
    const req = lib.request(u, { method, headers, agent: false }, (res) => {
      let bytes = 0;
      let first = 0;
      res.on('data', (d) => { if (!first) first = performance.now() - t0; bytes += d.length; if (!readBody) { res.destroy(); resolve(done()); } });
      res.on('end', () => resolve(done()));
      res.on('error', reject);
      function done() { return { status: res.statusCode, headers: res.headers, rawHeaders: res.rawHeaders, bytes, ms: performance.now() - t0, firstByteMs: first }; }
    });
    req.setTimeout(timeoutMs, () => req.destroy(new Error('timeout')));
    req.on('error', reject);
    req.end();
  });
}
