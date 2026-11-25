// Test-only Access check of the Worker: an RSA key pair made for this run, a JWKS served on 127.0.0.1 (the Worker reads it through ACCESS_CERTS_URL),
// and RS256 tokens signed with the key. The team domain and the AUD are made here too (a .invalid name and a random value), so no product
// setting and no real Access value is read or written. The Worker's check (app/worker/access.ts) runs unchanged.
import crypto from 'node:crypto';
import http from 'node:http';

export function makeAccess() {
  const { privateKey, publicKey } = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
  const kid = crypto.randomBytes(8).toString('hex');
  const jwk = { ...publicKey.export({ format: 'jwk' }), kid, alg: 'RS256', use: 'sig' };
  const team = `e2e-${crypto.randomBytes(4).toString('hex')}.invalid`;
  const aud = crypto.randomBytes(32).toString('hex');
  const b64u = (buf) => Buffer.from(buf).toString('base64url');
  const token = (now = Date.now()) => {
    const s = Math.floor(now / 1000);
    const head = b64u(JSON.stringify({ alg: 'RS256', kid, typ: 'JWT' }));
    const body = b64u(JSON.stringify({ iss: `https://${team}`, aud: [aud], sub: 'e2e', email: 'e2e@example.invalid', iat: s, nbf: s - 10, exp: s + 6 * 3600 }));
    const sig = crypto.sign('RSA-SHA256', Buffer.from(`${head}.${body}`), privateKey);
    return `${head}.${body}.${b64u(sig)}`;
  };
  return { team, aud, kid, jwk, token };
}

/** serves { keys: [jwk] } at /certs on 127.0.0.1; resolves with the URL and close() */
export function startJwks(jwk) {
  const server = http.createServer((req, res) => {
    if (req.url !== '/certs') { res.writeHead(404); res.end(); return; }
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(JSON.stringify({ keys: [jwk] }));
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      resolve({ url: `http://127.0.0.1:${port}/certs`, close: () => new Promise((r) => server.close(r)) });
    });
  });
}
