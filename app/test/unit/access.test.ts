import { beforeEach, describe, expect, it } from 'vitest';
import { AccessError, checkAccess, clearJwksCache, verifyAccessJwt, type Jwks } from '../../worker/access.ts';
import { claimsFor, makeKey, signJwt, type TestKey } from '../helpers/jwt.ts';

const TEAM = 'test-team.cloudflareaccess.com';
const AUD = 'aud-tag-0123456789abcdef';
const NOW = 1_800_000_000;
let key: TestKey, other: TestKey;
let fetches: number;
let jwks: Jwks;
const deps = () => ({ now: () => NOW * 1000, fetchJwks: async () => { fetches++; return jwks; } });
const cfg = { teamDomain: TEAM, aud: AUD };

beforeEach(async () => {
  clearJwksCache();
  fetches = 0;
  key = await makeKey('kid-current');
  other = await makeKey('kid-previous');
  jwks = { keys: [key.jwk, other.jwk] };
});

describe('verifyAccessJwt', () => {
  it('accepts a correctly signed token and returns its claims', async () => {
    const c = await verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW)), cfg, deps());
    expect(c.email).toBe('owner@example.invalid');
  });

  it('matches the key by kid (the previous key still verifies), as the Access docs require', async () => {
    const c = await verifyAccessJwt(await signJwt(other, claimsFor(TEAM, AUD, NOW)), cfg, deps());
    expect(c.sub).toBe('test-subject');
  });

  it('accepts aud as a string and as an array', async () => {
    await verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW, { aud: AUD })), cfg, deps());
    await verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW, { aud: ['x', AUD] })), cfg, deps());
  });

  it('accepts a team domain written with https:// and a trailing slash', async () => {
    await verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW)), { teamDomain: `https://${TEAM}/`, aud: AUD }, deps());
  });

  it('rejects a token for another application (aud)', async () => {
    await expect(verifyAccessJwt(await signJwt(key, claimsFor(TEAM, 'some-other-aud', NOW)), cfg, deps())).rejects.toThrow('aud');
    await expect(verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW, { aud: [] })), cfg, deps())).rejects.toThrow('aud');
  });

  it('rejects another issuer', async () => {
    await expect(verifyAccessJwt(await signJwt(key, claimsFor('evil.cloudflareaccess.com', AUD, NOW)), cfg, deps())).rejects.toThrow('issuer');
  });

  it('rejects an expired token, but allows the 60 s clock leeway', async () => {
    await expect(verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW, { exp: NOW - 120 })), cfg, deps())).rejects.toThrow('expired');
    await verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW, { exp: NOW - 30 })), cfg, deps());
    await expect(verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW, { exp: undefined })), cfg, deps())).rejects.toThrow('expired');
  });

  it('rejects a token that is not valid yet (nbf)', async () => {
    await expect(verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW, { nbf: NOW + 600 })), cfg, deps())).rejects.toThrow('nbf');
  });

  it('rejects a payload that was changed after signing', async () => {
    const t = await signJwt(key, claimsFor(TEAM, AUD, NOW));
    const [h, p, s] = t.split('.') as [string, string, string];
    const evil = btoa(JSON.stringify({ ...JSON.parse(atob(p.replace(/-/g, '+').replace(/_/g, '/'))), email: 'attacker@example.invalid' })).replace(/=+$/, '');
    await expect(verifyAccessJwt(`${h}.${evil}.${s}`, cfg, deps())).rejects.toThrow('signature');
  });

  it('rejects a token signed by a key that is not in the JWKS under that kid', async () => {
    const stranger = await makeKey('kid-current'); // same kid, different key material
    await expect(verifyAccessJwt(await signJwt(stranger, claimsFor(TEAM, AUD, NOW)), cfg, deps())).rejects.toThrow('signature');
    const unknown = await makeKey('kid-unknown');
    await expect(verifyAccessJwt(await signJwt(unknown, claimsFor(TEAM, AUD, NOW)), cfg, deps())).rejects.toThrow('kid');
  });

  it('rejects alg none, HS256 and a missing kid', async () => {
    for (const alg of ['none', 'HS256']) await expect(verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW), { alg }), cfg, deps())).rejects.toThrow('alg');
    await expect(verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW), { kid: '' }), cfg, deps())).rejects.toThrow('kid');
  });

  it('rejects garbage', async () => {
    for (const t of ['', 'a.b', 'a.b.c.d', 'not-base64.!!!.???']) await expect(verifyAccessJwt(t, cfg, deps())).rejects.toBeInstanceOf(AccessError);
  });

  it('fetches the signing keys once and caches them; an unknown kid does not make it fetch on every request', async () => {
    const t = await signJwt(key, claimsFor(TEAM, AUD, NOW));
    await verifyAccessJwt(t, cfg, deps());
    await verifyAccessJwt(t, cfg, deps());
    expect(fetches).toBe(1);
    const unknown = await makeKey('kid-unknown');
    const bad = await signJwt(unknown, claimsFor(TEAM, AUD, NOW));
    await expect(verifyAccessJwt(bad, cfg, deps())).rejects.toThrow('kid');
    await expect(verifyAccessJwt(bad, cfg, deps())).rejects.toThrow('kid');
    expect(fetches).toBe(1);
  });

  it('a key that appears after rotation is found by one re-fetch once the minimum interval has passed', async () => {
    await verifyAccessJwt(await signJwt(key, claimsFor(TEAM, AUD, NOW)), cfg, deps());
    const rotated = await makeKey('kid-rotated');
    jwks = { keys: [rotated.jwk, key.jwk] };
    const later = { now: () => (NOW + 120) * 1000, fetchJwks: async () => { fetches++; return jwks; } };
    await verifyAccessJwt(await signJwt(rotated, claimsFor(TEAM, AUD, NOW + 120)), cfg, later);
    expect(fetches).toBe(2);
  });
});

describe('checkAccess (the decision the Worker takes)', () => {
  const req = (token?: string) => new Request('https://app.example.invalid/', token ? { headers: { 'Cf-Access-Jwt-Assertion': token } } : {});
  const env = { ACCESS_TEAM_DOMAIN: TEAM, ACCESS_AUD: AUD };

  it('configured + valid token: verified', async () => {
    const d = await checkAccess(req(await signJwt(key, claimsFor(TEAM, AUD, NOW))), env, deps());
    expect(d).toMatchObject({ ok: true });
  });
  it('configured + no header: 403', async () => {
    expect(await checkAccess(req(), env, deps())).toMatchObject({ ok: false, status: 403 });
  });
  it('configured + wrong audience: 403', async () => {
    const d = await checkAccess(req(await signJwt(key, claimsFor(TEAM, 'wrong', NOW))), env, deps());
    expect(d).toMatchObject({ ok: false, status: 403 });
    expect(d.ok ? '' : d.reason).toContain('aud');
  });
  it('not configured: fails closed with 403', async () => {
    const d = await checkAccess(req(), {}, deps());
    expect(d).toMatchObject({ ok: false, status: 403 });
  });
  it('only one of the two set: still fails closed', async () => {
    expect(await checkAccess(req(), { ACCESS_TEAM_DOMAIN: TEAM }, deps())).toMatchObject({ ok: false, status: 403 });
    expect(await checkAccess(req(), { ACCESS_AUD: AUD }, deps())).toMatchObject({ ok: false, status: 403 });
  });
  it('there is no switch that skips the check: ACCESS_REQUIRED=false in the environment changes nothing', async () => {
    expect(await checkAccess(req(), { ACCESS_REQUIRED: 'false' } as never, deps())).toMatchObject({ ok: false, status: 403 });
  });
});
