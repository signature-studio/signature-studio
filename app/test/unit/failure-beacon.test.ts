// Decision 0257 (beacon2): the small report of a failure (src/perf/failure-beacon.ts): when it is sent, that it is sent once per kind, that a failure of the report is silent, and what it carries (and does not).
import { describe, expect, it } from 'vitest';
import {
  FAILURE_MESSAGE_MAX, FAILURE_STALL_MS, FailureReporter, buildFailureBeacon, detectCapabilities, installFailureWatch, lastStep,
  type Capabilities, type CapabilityEnv, type FailureFacts, type FailureHost, type FailureKind,
} from '../../src/perf/failure-beacon.ts';
import { PERF_MAX_BODY, checkPerfRequest, parsePerfBody } from '../../shared/perf.ts';
import { format, summarize } from '../../shared/perf-summary.mjs';
import type { SendResult } from '../../src/perf/beacon.ts';

const browser = { hardwareConcurrency: 8, deviceMemoryGB: 8, userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:159.0) Gecko/20100101 Firefox/159.0', screen: { width: 1920, height: 1080 }, devicePixelRatio: 1, crossOriginIsolated: true, isSecureContext: true, webglRenderer: 'NVIDIA GeForce GTX 980', jsHeap: null };
const client = { browser: { name: 'Firefox', version: '159.0', major: 159, os: 'Windows', headless: false }, colorDepth: 24, viewport: { width: 1900, height: 900 }, page: { secure: true, hostKind: 'other' as const }, physicalCores: { value: null, reason: 'no web API gives it' } };
const caps: Capabilities = { crossOriginIsolated: true, secureContext: true, sharedArrayBuffer: true, webgl2: true, offscreenWebgl2: true, opfs: true, jspi: true, wasmStreaming: true, wasmExceptions: true, worker: true, offscreenCanvas: true };
const ok: SendResult = { ok: true, status: 204, stored: true, note: 'stored by the server' };

const mkFacts = (o: Partial<FailureFacts> = {}): FailureFacts => ({ seq: 0, sid: 'a1b2c3d4e5f6', pageMs: 10, visibility: 'visible', browser, client, bundle: { name: 'fl-Oz', version: '1', engineKind: 'emscripten-fruity-v1' }, timeline: {}, state: 'loading', ...o });

function setup(over: { post?: (t: string) => Promise<SendResult>; timeline?: Record<string, number>; pageMs?: number } = {}) {
  const posts: string[] = [];
  let seq = 0;
  const facts = (): FailureFacts => ({ seq: seq++, sid: 'a1b2c3d4e5f6', pageMs: over.pageMs ?? 12345.6, visibility: 'visible', browser, client, bundle: { name: 'fl-Oz', version: '9b3a91d2d8ee', engineKind: 'emscripten-fruity-v1' }, timeline: over.timeline ?? { page_script: 90.2, manifest_loaded: 120.7, download_start: 150 }, state: 'loading' });
  const reporter = new FailureReporter({ facts, capabilities: () => caps, post: over.post ?? (async (t) => { posts.push(t); return ok; }) });
  return { posts, reporter };
}

describe('buildFailureBeacon', () => {
  it('is a small version 3 record with the kind, the stage, a cut message, the elapsed time, the timeline and the capability booleans', () => {
    const b = buildFailureBeacon('feature-missing', { stage: 'download', where: 'feature check (before any download)', message: 'JSPI is missing' }, mkFacts({ seq: 3, pageMs: 4321.4, timeline: { page_script: 90.2, download_start: 150.9 } }), { ...caps, jspi: false });
    expect(b).toMatchObject({ v: 3, seq: 3, sid: 'a1b2c3d4e5f6', reason: 'failure', pageMs: 4321.4 });
    expect(b.failure).toEqual({ kind: 'feature-missing', stage: 'download', where: 'feature check (before any download)', message: 'JSPI is missing', elapsedMs: 4321, state: 'loading', lastStep: { name: 'download_start', ms: 151 }, capabilities: { ...caps, jspi: false } });
    expect(b.timeline).toEqual({ page_script: 90, download_start: 151 });
    expect(b.browser).toEqual(browser);
    expect(b.client).toEqual(client);
  });
  it('is one small JSON object that the Worker accepts: under the limit by a wide margin, the log summary reads it without a failure, and it passes the request check and the body parser', () => {
    const b = buildFailureBeacon('no-first-idle', { message: 'x'.repeat(5000), where: 'w'.repeat(5000) }, mkFacts({ pageMs: 61000, bundle: { name: null, version: null, engineKind: null } }), caps);
    const text = JSON.stringify(b);
    expect(text.length).toBeLessThan(4096);
    expect(text.length).toBeLessThan(PERF_MAX_BODY / 4);
    expect(b.failure.message.length).toBeLessThanOrEqual(FAILURE_MESSAGE_MAX);
    const line = JSON.stringify({ t: '2026-11-05T12:00:00.000Z', remote: 'workers', perf: b });
    expect(() => format(summarize(line + '\n'))).not.toThrow();
    expect(format(summarize(line + '\n'))).toContain('failure');
    expect(checkPerfRequest({ method: 'POST', contentType: 'application/json', contentLength: String(text.length), origin: 'https://x.example', host: 'x.example', secFetchSite: 'same-origin' })).toEqual({ ok: true });
    const parsed = parsePerfBody(new TextEncoder().encode(text));
    expect(parsed.ok).toBe(true);
  });
  it('has no personal part: an e-mail address, a URL, an IP address, a user path and a file name in the message, the place or the stage are removed or replaced before it leaves the page', () => {
    const dirty = 'failed for jane.doe@example.com at https://fl.example.net/x?token=abc from 192.168.1.20 reading C:\\Users\\Jane Doe\\Music\\song.flp and /home/jane/projects/a.wav';
    const b = buildFailureBeacon('boot-exception', { stage: dirty, where: dirty, message: dirty }, mkFacts({ visibility: null, bundle: { name: null, version: null, engineKind: null }, state: dirty }), caps);
    const text = JSON.stringify(b);
    for (const secret of ['jane.doe', 'example.com', 'fl.example.net', 'token=abc', '192.168.1.20', 'Jane Doe', 'song.flp', '/home/jane', 'projects']) expect(text).not.toContain(secret);
    expect(b.failure.message).toContain('<email>');
    expect(b.failure.message).toContain('<url>');
    expect(b.failure.message).toContain('<ip>');
  });
  it('has no field that names a person or a connection: no remote, ip, email, cookie, url, user or token key anywhere in the record', () => {
    const b = buildFailureBeacon('engine-error', { message: 'm' }, mkFacts({ bundle: { name: 'n', version: 'v', engineKind: 'k' }, timeline: { a: 1 }, state: 's' }), caps);
    const keys: string[] = [];
    const walk = (o: unknown): void => { if (o && typeof o === 'object') for (const [k, v] of Object.entries(o)) { keys.push(k.toLowerCase()); walk(v); } };
    walk(b);
    for (const k of keys) expect(k).not.toMatch(/remote|^ip$|address|mail|cookie|^url$|href|user$|username|token|identity|^host$/);
  });
  it('takes the last step of the timeline as the largest stamp', () => {
    expect(lastStep({})).toBeNull();
    expect(lastStep({ a: 5.4, c: 90.6, b: 20 })).toEqual({ name: 'c', ms: 91 });
  });
});

describe('FailureReporter', () => {
  it('sends one POST for a failure and none for the same kind again', async () => {
    const { posts, reporter } = setup();
    expect(await reporter.report('engine-error', { message: 'first' })).toBe(true);
    expect(await reporter.report('engine-error', { message: 'second' })).toBe(false);
    expect(posts).toHaveLength(1);
    expect(JSON.parse(posts[0] as string).failure).toMatchObject({ kind: 'engine-error', message: 'first' });
    expect(reporter.has('engine-error')).toBe(true);
  });
  it('sends a different kind separately (a stall and then an error are two reports), each with its own sequence number', async () => {
    const { posts, reporter } = setup();
    await reporter.report('no-first-idle');
    await reporter.report('engine-trap');
    await reporter.report('engine-trap');
    expect(posts.map((p) => JSON.parse(p).failure.kind)).toEqual(['no-first-idle', 'engine-trap']);
    expect(posts.map((p) => JSON.parse(p).seq)).toEqual([0, 1]);
  });
  it('drops a failure of the POST silently: a rejection, a throw, and a refusal never reach the caller, and the kind is not tried again', async () => {
    for (const post of [async () => { throw new Error('network down'); }, () => { throw new Error('sync throw'); }, async () => ({ ok: false, status: 403, stored: null, note: 'HTTP 403' }) as SendResult]) {
      let calls = 0;
      const { reporter } = setup({ post: (t) => { calls++; return (post as (t: string) => Promise<SendResult>)(t); } });
      await expect(reporter.report('page-error', { message: 'm' })).resolves.toBeTypeOf('boolean');
      await reporter.report('page-error', { message: 'again' });
      expect(calls).toBe(1);
    }
  });
  it('drops a failure to collect the facts silently too (the big beacon\'s collectors may throw; this report must not depend on them)', async () => {
    const reporter = new FailureReporter({ facts: () => { throw new Error('collector broke'); }, capabilities: () => caps, post: async () => ok });
    await expect(reporter.report('boot-exception')).resolves.toBe(false);
    const r2 = new FailureReporter({ facts: () => mkFacts(), capabilities: () => { throw new Error('probe broke'); }, post: async () => ok });
    await expect(r2.report('boot-exception')).resolves.toBe(false);
  });
});

function fakeHost() {
  const timers: { fn: () => void; ms: number; live: boolean }[] = [];
  const handlers = new Map<string, (ev: unknown) => void>();
  const host: FailureHost = {
    setTimer: (fn, ms) => { const t = { fn, ms, live: true }; timers.push(t); return t; },
    clearTimer: (h) => { (h as { live: boolean }).live = false; },
    listen: (type, fn) => { handlers.set(type, fn); },
  };
  const fire = (): void => { for (const t of timers) if (t.live) { t.live = false; t.fn(); } };
  return { host, timers, handlers, fire };
}
const settle = async (): Promise<void> => { for (let i = 0; i < 5; i++) await Promise.resolve(); };

describe('installFailureWatch', () => {
  const kinds = (posts: string[]): FailureKind[] => posts.map((p) => JSON.parse(p).failure.kind);
  it('arms a stall timer of FAILURE_STALL_MS (60 s) and reports no-first-idle when nothing happened by then', async () => {
    const { posts, reporter } = setup();
    const h = fakeHost();
    installFailureWatch(h.host, reporter, { reachedIdle: () => false, failed: () => false, ended: () => false });
    expect(h.timers).toHaveLength(1);
    expect(h.timers[0]?.ms).toBe(FAILURE_STALL_MS);
    expect(FAILURE_STALL_MS).toBe(60_000);
    h.fire();
    await settle();
    expect(kinds(posts)).toEqual(['no-first-idle']);
    expect(JSON.parse(posts[0] as string).failure.message).toContain('60 s');
  });
  it('says nothing when the first idle came, when the page already failed, or when the engine already ended', async () => {
    for (const st of [{ reachedIdle: () => true, failed: () => false, ended: () => false }, { reachedIdle: () => false, failed: () => true, ended: () => false }, { reachedIdle: () => false, failed: () => false, ended: () => true }]) {
      const { posts, reporter } = setup();
      const h = fakeHost();
      installFailureWatch(h.host, reporter, st);
      h.fire();
      h.handlers.get('pagehide')?.({});
      await settle();
      expect(posts).toEqual([]);
    }
  });
  it('reports a pagehide before the first idle once, however many times the event comes', async () => {
    const { posts, reporter } = setup();
    const h = fakeHost();
    installFailureWatch(h.host, reporter, { reachedIdle: () => false, failed: () => false, ended: () => false });
    h.handlers.get('pagehide')?.({});
    h.handlers.get('pagehide')?.({});
    await settle();
    expect(kinds(posts)).toEqual(['left-before-first-idle']);
  });
  it('disarms the stall timer when asked', async () => {
    const { posts, reporter } = setup();
    const h = fakeHost();
    const stop = installFailureWatch(h.host, reporter, { reachedIdle: () => false, failed: () => false, ended: () => false });
    stop();
    h.fire();
    await settle();
    expect(posts).toEqual([]);
  });
  it('reports the first uncaught error and the first unhandled rejection of the page, with the message only (no file name, line or stack), once each', async () => {
    const { posts, reporter } = setup();
    const h = fakeHost();
    installFailureWatch(h.host, reporter, { reachedIdle: () => true, failed: () => false, ended: () => false });
    h.handlers.get('error')?.({ message: 'ReferenceError: foo is not defined', filename: 'https://fl.example.net/static/index-abc.js', lineno: 10, error: { stack: 'at https://fl.example.net/static/index-abc.js:10:5' } });
    h.handlers.get('error')?.({ message: 'another' });
    h.handlers.get('error')?.({ message: '' });
    h.handlers.get('unhandledrejection')?.({ reason: new Error('boom at C:\\Users\\Jane\\x.js') });
    h.handlers.get('unhandledrejection')?.({ reason: 'second' });
    await settle();
    expect(kinds(posts)).toEqual(['page-error', 'unhandled-rejection']);
    const text = posts.join('\n');
    expect(text).not.toContain('index-abc.js');
    expect(text).not.toContain('Jane');
    expect(JSON.parse(posts[0] as string).failure.message).toBe('ReferenceError: foo is not defined');
  });
  it('never lets a failure of the report out of an event handler', async () => {
    const reporter = new FailureReporter({ facts: () => { throw new Error('x'); }, capabilities: () => caps, post: async () => ok });
    const h = fakeHost();
    installFailureWatch(h.host, reporter, { reachedIdle: () => false, failed: () => false, ended: () => false });
    expect(() => { h.fire(); h.handlers.get('pagehide')?.({}); h.handlers.get('error')?.({ message: 'm' }); h.handlers.get('unhandledrejection')?.({ reason: 'r' }); }).not.toThrow();
    await settle();
  });
});

describe('detectCapabilities', () => {
  const gl = () => ({ getExtension: () => ({ loseContext: () => {} }) });
  const full = (): CapabilityEnv => ({
    crossOriginIsolated: true, isSecureContext: true, SharedArrayBuffer: class {},
    navigator: { storage: { getDirectory: () => {} } },
    document: { createElement: () => ({ getContext: (id: string) => (id === 'webgl2' ? gl() : null) }) },
    OffscreenCanvas: class { getContext(id: string) { return id === 'webgl2' ? gl() : null; } },
    WebAssembly: { Suspending: () => {}, promising: () => {}, compileStreaming: () => {}, validate: () => true },
    Worker: class {},
  });
  it('is true for everything in a browser that has it all', () => {
    expect(detectCapabilities(full())).toEqual({ crossOriginIsolated: true, secureContext: true, sharedArrayBuffer: true, webgl2: true, offscreenWebgl2: true, opfs: true, jspi: true, wasmStreaming: true, wasmExceptions: true, worker: true, offscreenCanvas: true });
  });
  it('tells each missing capability apart: no JSPI, no OPFS, no WebGL2, not isolated, no SharedArrayBuffer', () => {
    const e = full();
    const noJspi = detectCapabilities({ ...e, WebAssembly: { compileStreaming: () => {}, validate: () => true } });
    expect(noJspi).toMatchObject({ jspi: false, wasmStreaming: true, wasmExceptions: true });
    expect(detectCapabilities({ ...e, navigator: { storage: {} } }).opfs).toBe(false);
    expect(detectCapabilities({ ...e, navigator: {} }).opfs).toBe(false);
    const noGl = detectCapabilities({ ...e, document: { createElement: () => ({ getContext: () => null }) }, OffscreenCanvas: class { getContext() { return null; } } });
    expect(noGl).toMatchObject({ webgl2: false, offscreenWebgl2: false });
    const notIsolated = detectCapabilities({ ...e, crossOriginIsolated: false, SharedArrayBuffer: undefined });
    expect(notIsolated).toMatchObject({ crossOriginIsolated: false, sharedArrayBuffer: false });
    expect(detectCapabilities({ ...e, WebAssembly: { Suspending: () => {}, promising: () => {}, compileStreaming: () => {}, validate: () => false } }).wasmExceptions).toBe(false);
  });
  it('says null where it cannot know, and never throws: no document, no OffscreenCanvas, a probe that throws, an empty environment', () => {
    const e = full();
    expect(detectCapabilities({ ...e, document: undefined })).toMatchObject({ webgl2: null });
    expect(detectCapabilities({ ...e, OffscreenCanvas: undefined })).toMatchObject({ offscreenWebgl2: null, offscreenCanvas: false });
    expect(detectCapabilities({ ...e, document: { createElement: () => { throw new Error('no canvas'); } } }).webgl2).toBeNull();
    expect(detectCapabilities({ ...e, OffscreenCanvas: class { constructor() { throw new Error('no'); } } }).offscreenWebgl2).toBeNull();
    expect(detectCapabilities({})).toEqual({ crossOriginIsolated: null, secureContext: null, sharedArrayBuffer: false, webgl2: null, offscreenWebgl2: null, opfs: false, jspi: false, wasmStreaming: false, wasmExceptions: false, worker: false, offscreenCanvas: false });
  });
  it('releases the contexts it made for the probe', () => {
    let lost = 0;
    const lose = { getExtension: () => ({ loseContext: () => { lost++; } }) };
    detectCapabilities({ ...full(), document: { createElement: () => ({ getContext: () => lose }) }, OffscreenCanvas: class { getContext() { return lose; } } });
    expect(lost).toBe(2);
  });
});
