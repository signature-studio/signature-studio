// Test-only JWT signing: a throw-away RSA key pair per test run, JWKS in the shape of https://<team>.cloudflareaccess.com/cdn-cgi/access/certs. No network.
import type { Jwk } from '../../worker/access.ts';

export interface TestKey { kid: string; privateKey: CryptoKey; jwk: Jwk }

const b64url = (b: Uint8Array | string): string => {
  const u = typeof b === 'string' ? new TextEncoder().encode(b) : b;
  let s = '';
  for (const x of u) s += String.fromCharCode(x);
  return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
};

export async function makeKey(kid: string): Promise<TestKey> {
  const kp = await crypto.subtle.generateKey({ name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' }, true, ['sign', 'verify']);
  const jwk = (await crypto.subtle.exportKey('jwk', kp.publicKey)) as JsonWebKey;
  return { kid, privateKey: kp.privateKey, jwk: { kid, kty: 'RSA', alg: 'RS256', use: 'sig', n: jwk.n as string, e: jwk.e as string } };
}

export async function signJwt(key: TestKey, claims: Record<string, unknown>, header: Record<string, unknown> = {}): Promise<string> {
  const h = b64url(JSON.stringify({ alg: 'RS256', kid: key.kid, typ: 'JWT', ...header }));
  const p = b64url(JSON.stringify(claims));
  const sig = new Uint8Array(await crypto.subtle.sign('RSASSA-PKCS1-v1_5', key.privateKey, new TextEncoder().encode(`${h}.${p}`)));
  return `${h}.${p}.${b64url(sig)}`;
}

export const claimsFor = (team: string, aud: string, nowS: number, extra: Record<string, unknown> = {}): Record<string, unknown> => ({
  iss: `https://${team}`, aud: [aud], exp: nowS + 3600, iat: nowS, nbf: nowS - 5, email: 'owner@example.invalid', sub: 'test-subject', ...extra,
});
