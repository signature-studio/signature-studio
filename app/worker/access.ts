// Cloudflare Access: validation of the application token (Cf-Access-Jwt-Assertion) inside the Worker, as a second layer behind Access itself.
// Access (the edge) is what actually keeps people out; it is configured in the dashboard / API, never here (docs/decisions/0135-frontend-app.md, section on the operator's steps).
// Checks, following https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/validating-json/ :
//   RS256 signature with the key whose `kid` matches the token, from https://<team>.cloudflareaccess.com/cdn-cgi/access/certs ("keys", not "public_cert");
//   iss = https://<team>.cloudflareaccess.com ; aud contains the application's AUD tag ; exp (and nbf) in time.
// WebCrypto only. The JWKS fetch is injectable so the tests need no network.

export interface Jwk { kid: string; kty: string; n: string; e: string; alg?: string; use?: string }
export interface Jwks { keys: Jwk[] }
export interface AccessConfig { teamDomain: string; aud: string; certsUrl?: string }
export interface AccessClaims { iss: string; aud: string | string[]; exp: number; nbf?: number; iat?: number; email?: string; sub?: string; [k: string]: unknown }
export interface AccessDeps { fetchJwks?: (url: string) => Promise<Jwks>; now?: () => number }

export class AccessError extends Error {}

const LEEWAY_S = 60;
const JWKS_TTL_MS = 10 * 60 * 1000;
const REFETCH_MIN_INTERVAL_MS = 60 * 1000;

const cache = new Map<string, { keys: Jwk[]; fetchedAt: number }>();
export function clearJwksCache(): void { cache.clear(); }

export const normalizeTeamDomain = (d: string): string => d.trim().replace(/^https?:\/\//i, '').replace(/\/+$/, '');

function b64urlToBytes(s: string): Uint8Array<ArrayBuffer> {
  const pad = s.length % 4 === 0 ? '' : '='.repeat(4 - (s.length % 4));
  const bin = atob(s.replace(/-/g, '+').replace(/_/g, '/') + pad);
  const out = new Uint8Array(new ArrayBuffer(bin.length));
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
const b64urlToJson = (s: string): unknown => JSON.parse(new TextDecoder().decode(b64urlToBytes(s)));

async function defaultFetchJwks(url: string): Promise<Jwks> {
  const r = await fetch(url, { headers: { Accept: 'application/json' } });
  if (!r.ok) throw new AccessError(`could not fetch the Access signing keys (HTTP ${r.status})`);
  const j = (await r.json()) as Partial<Jwks>;
  if (!Array.isArray(j.keys)) throw new AccessError('the Access certs document has no "keys" array');
  return { keys: j.keys };
}

async function findKey(kid: string, url: string, deps: AccessDeps): Promise<Jwk | undefined> {
  const now = (deps.now ?? Date.now)();
  const fetchJwks = deps.fetchJwks ?? defaultFetchJwks;
  const hit = cache.get(url);
  if (hit && now - hit.fetchedAt < JWKS_TTL_MS) {
    const k = hit.keys.find((x) => x.kid === kid);
    if (k) return k;
    if (now - hit.fetchedAt < REFETCH_MIN_INTERVAL_MS) return undefined; // an unknown kid does not make us hammer the endpoint
  }
  const fresh = await fetchJwks(url);
  cache.set(url, { keys: fresh.keys, fetchedAt: now });
  return fresh.keys.find((x) => x.kid === kid);
}

export async function verifyAccessJwt(token: string, cfg: AccessConfig, deps: AccessDeps = {}): Promise<AccessClaims> {
  const parts = token.split('.');
  if (parts.length !== 3) throw new AccessError('the token is not a JWT (three dot-separated parts expected)');
  const [h64, p64, s64] = parts as [string, string, string];
  let header: { alg?: unknown; kid?: unknown };
  let payload: AccessClaims;
  try { header = b64urlToJson(h64) as typeof header; payload = b64urlToJson(p64) as AccessClaims; } catch { throw new AccessError('the token header or payload is not base64url JSON'); }
  if (header.alg !== 'RS256') throw new AccessError(`unsupported alg ${String(header.alg)} (RS256 only)`);
  if (typeof header.kid !== 'string' || header.kid === '') throw new AccessError('the token has no kid');
  const team = normalizeTeamDomain(cfg.teamDomain);
  const certsUrl = cfg.certsUrl ?? `https://${team}/cdn-cgi/access/certs`;
  const jwk = await findKey(header.kid, certsUrl, deps);
  if (!jwk) throw new AccessError(`no Access signing key with kid ${header.kid}`);
  if (jwk.kty !== 'RSA') throw new AccessError('the signing key is not an RSA key');
  const key = await crypto.subtle.importKey('jwk', { kty: 'RSA', n: jwk.n, e: jwk.e, alg: 'RS256', ext: true }, { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }, false, ['verify']);
  const okSig = await crypto.subtle.verify('RSASSA-PKCS1-v1_5', key, b64urlToBytes(s64), new TextEncoder().encode(`${h64}.${p64}`));
  if (!okSig) throw new AccessError('bad signature');
  const nowS = Math.floor((deps.now ?? Date.now)() / 1000);
  if (payload.iss !== `https://${team}`) throw new AccessError(`wrong issuer ${String(payload.iss)}`);
  const auds = Array.isArray(payload.aud) ? payload.aud : [payload.aud];
  if (!auds.includes(cfg.aud)) throw new AccessError('the token is for another application (aud)');
  if (typeof payload.exp !== 'number' || payload.exp + LEEWAY_S < nowS) throw new AccessError('the token has expired');
  if (typeof payload.nbf === 'number' && payload.nbf - LEEWAY_S > nowS) throw new AccessError('the token is not valid yet (nbf)');
  return payload;
}

export interface AccessEnv { ACCESS_TEAM_DOMAIN?: string; ACCESS_AUD?: string; ACCESS_CERTS_URL?: string }
export type AccessDecision = { ok: true; claims: AccessClaims } | { ok: false; status: 403; reason: string };

/**
 * Configured (team domain and AUD both set): the token must verify. Not configured: the Worker answers 403 to everything (fail closed). There is no way to skip the check (decision 0243): a test configures a test team, AUD and
 * signing keys (ACCESS_CERTS_URL names where the Worker fetches the keys; it is the team's certs URL when it is not given).
 */
export async function checkAccess(request: Request, env: AccessEnv, deps: AccessDeps = {}): Promise<AccessDecision> {
  const team = (env.ACCESS_TEAM_DOMAIN ?? '').trim();
  const aud = (env.ACCESS_AUD ?? '').trim();
  if (team === '' || aud === '') return { ok: false, status: 403, reason: 'Access validation is required but ACCESS_TEAM_DOMAIN and ACCESS_AUD are not configured on this Worker.' };
  const token = request.headers.get('Cf-Access-Jwt-Assertion');
  if (!token) return { ok: false, status: 403, reason: 'Missing Cf-Access-Jwt-Assertion: this Worker must be reached through Cloudflare Access.' };
  try {
    const claims = await verifyAccessJwt(token, { teamDomain: team, aud, ...(env.ACCESS_CERTS_URL ? { certsUrl: env.ACCESS_CERTS_URL } : {}) }, deps);
    return { ok: true, claims };
  } catch (e) {
    return { ok: false, status: 403, reason: `Invalid Access token: ${e instanceof Error ? e.message : String(e)}` };
  }
}
