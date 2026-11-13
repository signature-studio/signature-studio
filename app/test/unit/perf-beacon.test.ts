// Decision 0152: the beacon the page sends (src/perf/beacon.ts, browser-info.ts) and when it sends it.
import { describe, expect, it } from 'vitest';
import { BEACON_MAX_PERIODIC, BEACON_PERIOD_MS, BeaconScheduler, buildBeacon, beaconText, postBeacon, type BeaconInput, type BeaconReason } from '../../src/perf/beacon.ts';
import { collectBrowserInfo, collectClientInfo, hostKind, parseBrowser, webglRenderer, type BrowserInfo } from '../../src/perf/browser-info.ts';
import { DisplayMeter, DisplayMonitor, hzFit, type DisplayHost, type DisplaySnapshot } from '../../src/perf/display.ts';
import { PERF_MAX_BODY, beaconProblems } from '../../shared/perf.ts';

const browser: BrowserInfo = { hardwareConcurrency: 8, deviceMemoryGB: 8, userAgent: 'UA/1', screen: { width: 2560, height: 1440 }, devicePixelRatio: 1.5, crossOriginIsolated: true, isSecureContext: true, webglRenderer: 'ANGLE (X)', jsHeap: { usedBytes: 1, totalBytes: 2, limitBytes: 3 } };
const stat = (n: number): { n: number; intervalMedianMs: number | null; intervalP95Ms: number | null; perSecond: number | null } => ({ n, intervalMedianMs: 8, intervalP95Ms: 9, perSecond: 120 });
const input = {
  sent: 3, processed: 3, dropped: 0, pending: 0, answered: 3, late: 0, noAnswer: 0,
  eventToRaf: { n: 2, p50Ms: 30, p95Ms: 40, maxMs: 40, lastMs: [30, 40] }, listenerToPut: { n: 2, p50Ms: 10, p95Ms: 20, maxMs: 20, lastMs: [10, 20] }, lateVisible: null, eventToRafWithLate: null,
  stages: { n: 2, p50: { s0: 13, s0b: 0.3, toWorker: 0.5, worker: 2.5, fromWorker: 0.4, put: 0.2, raf: 13 }, p95: { s0: 15, s0b: 0.5, toWorker: 0.9, worker: 3, fromWorker: 0.6, put: 0.4, raf: 16 } },
};
const base: BeaconInput = {
  seq: 4, reason: 'periodic', pageMs: 123456.5, visibility: 'visible', browser, bundle: { name: 'fl-Oz', version: '2', engineKind: 'emscripten-fruity-v1' },
  state: 'engine-idle', exit: null, error: null, timeline: { download_start: 100, idle: 38750 }, updates: { total: 500, recent: stat(10) }, idles: { total: 40, recent: stat(5) }, input, underruns: null,
};

describe('buildBeacon', () => {
  it('has every field the PM asked for, and the targets next to them', () => {
    const b = buildBeacon(base);
    expect(b.v).toBe(3);
    expect(b).toMatchObject({ seq: 4, reason: 'periodic', pageMs: 123456.5, visibility: 'visible' });
    expect(b.browser).toEqual(browser);
    expect(b.bundle).toEqual({ name: 'fl-Oz', version: '2', engineKind: 'emscripten-fruity-v1' });
    expect(b.timeline).toEqual({ download_start: 100, idle: 38750 });
    expect(b.startup).toEqual({ firstIdleSeconds: 38.75, targetSeconds: 10 });
    expect(b.ui.updates).toEqual({ total: 500, recent10s: stat(10) });
    expect(b.ui.engineIdles).toEqual({ total: 40, recent10s: stat(5) });
    expect(b.ui.targetUpdatesPerSecond).toBe(120);
    expect(b.input).toEqual({ ...input, targetP95Ms: 17 });
    expect(b.audio).toEqual({ underruns: null });
    expect(buildBeacon({ ...base, underruns: 2, audioDetail: { episodes: 1 } }).audio).toEqual({ underruns: 2, detail: { episodes: 1 } });
    expect(b.engine).toEqual({ present: 'D', updatesObservable: true, state: 'engine-idle', exit: null });
    expect(b.error).toBeNull();
  });
  it('carries the engine\'s wasm memory against the limit of its memory section (decision 0170); null before the first idle and for a stand-in', () => {
    expect(buildBeacon({ ...base, wasmMemoryBytes: 1024 * 1048576 }).memory).toEqual({ wasmBytes: 1024 * 1048576, wasmLimitBytes: 2048 * 1048576, wasmFraction: 0.5 });
    expect(buildBeacon(base).memory).toEqual({ wasmBytes: null, wasmLimitBytes: 2048 * 1048576, wasmFraction: null });
    expect(buildBeacon({ ...base, wasmMemoryBytes: null }).memory.wasmFraction).toBeNull();
  });
  it('before the first idle there is no start-up time (not a guess)', () => {
    expect(buildBeacon({ ...base, timeline: { download_start: 100 } }).startup.firstIdleSeconds).toBeNull();
  });
  it('the page counts the presents the Worker reports (decision 0223), so the updates are observable and reported; the P2 measure (UI updates per second) exists', () => {
    const b = buildBeacon(base);
    expect(b.engine.updatesObservable).toBe(true);
    expect(b.ui.updates).toEqual({ total: 500, recent10s: expect.anything() });
  });
  it('an error and an exit are carried; a long error text is cut', () => {
    const b = buildBeacon({ ...base, reason: 'error', error: { what: 'w'.repeat(1000), where: 'h'.repeat(500) }, exit: { reason: 'trap', code: 1, presents: 7 } });
    expect(b.error?.what).toHaveLength(400);
    expect(b.error?.where).toHaveLength(200);
    expect(b.engine.exit).toEqual({ reason: 'trap', code: 1, presents: 7 });
  });
  it('the engine\'s last words are carried in engine.exit.message, cleaned again here, absent when there is none (decision 0209)', () => {
    const e = { reason: 'stopped', code: 134, presents: 9 };
    expect(buildBeacon({ ...base, reason: 'exit', exit: { ...e, message: 'fle: CreateFileW: not modelled' } }).engine.exit).toEqual({ ...e, message: 'fle: CreateFileW: not modelled' });
    expect(buildBeacon({ ...base, reason: 'exit', exit: { ...e, message: null } }).engine.exit).toEqual(e);
    expect(buildBeacon({ ...base, reason: 'exit', exit: { ...e, message: '' } }).engine.exit).toEqual(e);
    // a caller that passes a raw string cannot send a path or a name
    const raw = buildBeacon({ ...base, reason: 'exit', exit: { ...e, message: 'fle: X: C:\\Users\\Jane Doe\\mix.flp and /home/ubuntu/a.b ' + 'z'.repeat(600) } }).engine.exit?.message as string;
    expect(raw).not.toMatch(/Jane|ubuntu|mix/);
    expect(raw.length).toBeLessThanOrEqual(400);
    // the beacon with a message is still a version 3 beacon that the checks accept, and its text is under the server's limit
    const withMsg = buildBeacon({ ...base, sid: SID, reason: 'exit', exit: { ...e, message: 'fle: A: ' + 'm'.repeat(380) } });
    expect(withMsg.v).toBe(3);
    expect(beaconProblems(JSON.parse(beaconText(withMsg)))).toEqual([]);
    expect(beaconText(withMsg).length).toBeLessThan(PERF_MAX_BODY);
    expect(JSON.parse(beaconText(withMsg)).engine.exit.message).toHaveLength(388);
  });
  it('carries nothing personal: no IP, no URL, no cookie, no field', () => {
    const text = JSON.stringify(buildBeacon(base));
    expect(text).not.toMatch(/\d+\.\d+\.\d+\.\d+/);
    expect(text).not.toMatch(/https?:|cookie|location|href|"origin"|"url"|"ip"/i);
  });
});

describe('beaconText', () => {
  it('a normal beacon is well under the 16 KB limit', () => {
    expect(beaconText(buildBeacon(base)).length).toBeLessThan(3000);
  });
  it('a beacon that would be too big is shrunk below the limit: long strings first', () => {
    const big = buildBeacon({ ...base, browser: { ...browser, userAgent: 'U'.repeat(300), webglRenderer: 'G'.repeat(200) }, timeline: Object.fromEntries(Array.from({ length: 600 }, (_, i) => [`mark_${i}`, i * 1.123456789])) });
    const full = JSON.stringify(big);
    expect(full.length).toBeGreaterThan(PERF_MAX_BODY);
    const text = beaconText(big);
    expect(text.length).toBeLessThanOrEqual(PERF_MAX_BODY - 512);
    expect(JSON.parse(text).seq).toBe(4);
  });
});

describe('postBeacon', () => {
  const answer = (status: number, headers: Record<string, string> = {}): typeof fetch => (async () => new Response(null, { status, headers })) as unknown as typeof fetch;
  it('POSTs to /__perf of this origin as JSON with same-origin credentials (behind Cloudflare Access the CF_Authorization cookie must go with it, or Access answers 302 and the Worker never sees the beacon), and reads X-Perf-Stored', async () => {
    let seen: { url: string; init: RequestInit } | null = null;
    const f = (async (url: string, init: RequestInit) => { seen = { url, init }; return new Response(null, { status: 204, headers: { 'X-Perf-Stored': 'yes' } }); }) as unknown as typeof fetch;
    const r = await postBeacon('{"v":1}', f);
    expect(r).toEqual({ ok: true, status: 204, stored: true, note: 'stored by the server' });
    expect(seen).not.toBeNull();
    const s = seen as unknown as { url: string; init: RequestInit };
    expect(s.url).toBe('/__perf');
    expect(s.init.method).toBe('POST');
    expect(s.init.credentials).toBe('same-origin');
    expect(s.init.body).toBe('{"v":1}');
    expect((s.init.headers as Record<string, string>)['Content-Type']).toBe('application/json');
  });
  it('204 with "no": accepted but not stored (the Worker); no header: stored unknown', async () => {
    expect(await postBeacon('{}', answer(204, { 'X-Perf-Stored': 'no' }))).toEqual({ ok: true, status: 204, stored: false, note: 'accepted, not stored by this server' });
    expect((await postBeacon('{}', answer(204))).stored).toBeNull();
  });
  it('an HTTP error is a result with its status, not a throw', async () => {
    expect(await postBeacon('{}', answer(503))).toEqual({ ok: false, status: 503, stored: null, note: 'HTTP 503' });
    expect(await postBeacon('{}', answer(413, { 'X-Perf-Stored': 'no' }))).toMatchObject({ ok: false, status: 413, stored: false });
  });
  it('a network failure is a result with status null', async () => {
    const f = (async () => { throw new TypeError('Failed to fetch'); }) as unknown as typeof fetch;
    const r = await postBeacon('{}', f);
    expect(r.ok).toBe(false);
    expect(r.status).toBeNull();
    expect(r.note).toContain('Failed to fetch');
  });
});

// a fake clock: timers fire when the test advances time
function fakeScheduler(over: { periodMs?: number; maxPeriodic?: number } = {}) {
  const sent: BeaconReason[] = [];
  let now = 0, id = 0;
  const timers = new Map<number, { at: number; fn: () => void }>();
  const s = new BeaconScheduler({
    send: (r) => sent.push(r),
    setTimer: (fn, ms) => { const h = ++id; timers.set(h, { at: now + ms, fn }); return h; },
    clearTimer: (h) => { timers.delete(h as number); },
    ...over,
  });
  const advance = (ms: number): void => {
    const end = now + ms;
    for (;;) {
      const next = [...timers.entries()].filter(([, t]) => t.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
      if (!next) break;
      timers.delete(next[0]);
      now = next[1].at;
      next[1].fn();
    }
    now = end;
  };
  return { s, sent, advance, timers };
}

describe('BeaconScheduler', () => {
  it('constants: every 30 s, at most 20 periodic', () => { expect(BEACON_PERIOD_MS).toBe(30000); expect(BEACON_MAX_PERIODIC).toBe(20); });
  it('nothing is sent before the first idle', () => {
    const f = fakeScheduler();
    f.advance(10 * 60_000);
    expect(f.sent).toEqual([]);
  });
  it('the first idle sends once, at once; then one every 30 s: at 29.9 s still one, at 30 s two, at 60 s three', () => {
    const f = fakeScheduler();
    f.s.firstIdle();
    expect(f.sent).toEqual(['first-idle']);
    f.advance(29_999);
    expect(f.sent).toEqual(['first-idle']);
    f.advance(1);
    expect(f.sent).toEqual(['first-idle', 'periodic']);
    f.advance(30_000);
    expect(f.sent).toEqual(['first-idle', 'periodic', 'periodic']);
  });
  it('a second first-idle changes nothing', () => {
    const f = fakeScheduler();
    f.s.firstIdle(); f.s.firstIdle();
    expect(f.sent).toEqual(['first-idle']);
    expect(f.timers.size).toBe(1);
  });
  it('at most 20 periodic beacons, then the timer is gone: 21 in all after an hour', () => {
    const f = fakeScheduler();
    f.s.firstIdle();
    f.advance(60 * 60_000);
    expect(f.sent.filter((r) => r === 'periodic')).toHaveLength(20);
    expect(f.sent).toHaveLength(21);
    expect(f.s.sentPeriodic).toBe(20);
    expect(f.timers.size).toBe(0);
    expect(f.s.running).toBe(false);
  });
  it('the end sends one beacon and stops the periodic ones; a second end (error after exit) is dropped', () => {
    const f = fakeScheduler();
    f.s.firstIdle();
    f.advance(30_000);
    f.s.end('exit');
    expect(f.sent).toEqual(['first-idle', 'periodic', 'exit']);
    expect(f.timers.size).toBe(0); // the timer is cancelled at once, not left to fire and find nothing to do
    expect(f.s.running).toBe(false);
    f.advance(10 * 60_000);
    f.s.end('error');
    expect(f.sent).toEqual(['first-idle', 'periodic', 'exit']);
    expect(f.timers.size).toBe(0);
  });
  it('an end before any idle (a stand-in, a failed start) is the only beacon and no timer is started; a late first idle is ignored', () => {
    const f = fakeScheduler();
    f.s.end('error');
    f.s.firstIdle();
    f.advance(120_000);
    expect(f.sent).toEqual(['error']);
    expect(f.timers.size).toBe(0);
  });
  it('the first of exit / error wins', () => {
    const f = fakeScheduler();
    f.s.end('error'); f.s.end('exit');
    expect(f.sent).toEqual(['error']);
  });
  it('stop() cancels the timer', () => {
    const f = fakeScheduler();
    f.s.firstIdle();
    f.s.stop();
    f.advance(120_000);
    expect(f.sent).toEqual(['first-idle']);
  });
  it('the period and the cap can be set (5 s, 2): first-idle at 0, periodic at 5 s and 10 s, no more', () => {
    const f = fakeScheduler({ periodMs: 5000, maxPeriodic: 2 });
    f.s.firstIdle();
    f.advance(60_000);
    expect(f.sent).toEqual(['first-idle', 'periodic', 'periodic']);
  });
});

describe('collectBrowserInfo', () => {
  it('reads the cores, memory, user agent, screen, pixel ratio, isolation, secure context, GPU renderer and the JS heap', () => {
    const gl = { getExtension: (n: string) => (n === 'WEBGL_debug_renderer_info' ? { UNMASKED_RENDERER_WEBGL: 37446 } : n === 'WEBGL_lose_context' ? { loseContext: () => undefined } : null), getParameter: (p: number) => (p === 37446 ? 'ANGLE (NVIDIA, GeForce RTX 4070)' : null) };
    const info = collectBrowserInfo({
      navigator: { hardwareConcurrency: 16, deviceMemory: 8, userAgent: 'Mozilla/5.0 X' }, screen: { width: 3840, height: 2160 }, devicePixelRatio: 2, crossOriginIsolated: true, isSecureContext: true,
      performance: { memory: { usedJSHeapSize: 10, totalJSHeapSize: 20, jsHeapSizeLimit: 30 } }, document: { createElement: () => ({ getContext: () => gl }) },
    });
    expect(info).toEqual({ hardwareConcurrency: 16, deviceMemoryGB: 8, userAgent: 'Mozilla/5.0 X', screen: { width: 3840, height: 2160 }, devicePixelRatio: 2, crossOriginIsolated: true, isSecureContext: true, webglRenderer: 'ANGLE (NVIDIA, GeForce RTX 4070)', jsHeap: { usedBytes: 10, totalBytes: 20, limitBytes: 30 } });
  });
  it('whatever the browser does not give is null, not a guess; nothing throws', () => {
    expect(collectBrowserInfo({})).toEqual({ hardwareConcurrency: null, deviceMemoryGB: null, userAgent: '', screen: null, devicePixelRatio: null, crossOriginIsolated: null, isSecureContext: null, webglRenderer: null, jsHeap: null });
    expect(collectBrowserInfo({ navigator: { hardwareConcurrency: Number.NaN }, performance: { memory: { usedJSHeapSize: 1 } } })).toMatchObject({ hardwareConcurrency: null, jsHeap: null });
  });
  it('the user agent is cut at 300 characters', () => {
    expect(collectBrowserInfo({ navigator: { userAgent: 'u'.repeat(1000) } }).userAgent).toHaveLength(300);
  });
  it('webglRenderer: no WebGL, no debug extension, an empty name and a throwing browser all give null', () => {
    expect(webglRenderer(undefined)).toBeNull();
    expect(webglRenderer({ createElement: () => ({ getContext: () => null }) })).toBeNull();
    expect(webglRenderer({ createElement: () => ({ getContext: () => ({ getExtension: () => null, getParameter: () => 'x' }) }) })).toBeNull();
    expect(webglRenderer({ createElement: () => ({ getContext: () => ({ getExtension: () => ({ UNMASKED_RENDERER_WEBGL: 1 }), getParameter: () => '' }) }) })).toBeNull();
    expect(webglRenderer({ createElement: () => { throw new Error('no canvas'); } })).toBeNull();
  });
});


// ---- decision 0192: the screen measurement in the beacon ----

const T0 = 1_700_000_000_000;
/** a monitor that has run for `seconds` at `hz` with `drawHz` canvas draws a second, one long task, slow events and a hidden stretch: every block of the beacon filled */
function busySnapshot(hz = 120, seconds = 30, drawHz = 125): DisplaySnapshot {
  const fakeHost: DisplayHost = { now: () => T0 + t, timeOrigin: () => T0, requestFrame: (cb) => { cbs.push(cb); }, setTimer: () => 0, clearTimer: () => undefined, isVisible: () => true, onVisibility: () => undefined, observe: () => true };
  let t = 0;
  const cbs: ((x: number) => void)[] = [];
  const m = new DisplayMonitor(fakeHost, { mode: 'continuous' });
  m.start();
  m.markIdle();
  let nextDraw = 0;
  for (let i = 0; i < seconds * hz; i++) {
    t += 1000 / hz;
    while (nextDraw < t) { m.meter.received(); m.meter.draw(T0 + nextDraw, T0 + nextDraw + 0.3, T0 + nextDraw - 1); nextDraw += 1000 / drawHz; }
    const run = cbs.splice(0);
    for (const cb of run) cb(t);
  }
  m.longTasks.add(T0 + 5000, 61);
  m.events.add({ name: 'pointermove', startTime: 100, processingStart: 104, duration: 24 });
  return m.snapshot(true, 40);
}
const client = collectClientInfo({ navigator: { userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36' }, screen: { colorDepth: 24 }, innerWidth: 1900, innerHeight: 960, isSecureContext: true, location: { hostname: 'machine.example.ts.net' } });
const SID = 'a1b2c3d4e5f6';
const v2 = (over: Partial<BeaconInput> = {}) => buildBeacon({ ...base, sid: SID, client, display: busySnapshot(), hz: hzFit({ engineEnvHz: 120, engineHz: 120, autoMeasuredHz: null }, busySnapshot().raf), ...over });

describe('the beacon of version 2 (decision 0192)', () => {
  it('carries a page-load token, the client block, the display block and the hz block; every field of version 1 is still there', () => {
    const b = v2();
    expect(b.v).toBe(3);
    expect(b.sid).toBe(SID);
    for (const k of ['seq', 'reason', 'pageMs', 'visibility', 'browser', 'bundle', 'engine', 'error', 'timeline', 'startup', 'ui', 'input', 'audio', 'memory']) expect(b, k).toHaveProperty(k);
    expect(b.client?.browser).toEqual({ name: 'Chrome', version: '156.0', major: 156, os: 'Windows', headless: false });
    expect(b.display?.rafSupported).toBe(true);
    expect(b.display?.raf.recent10s.hz).toBeGreaterThan(119);
    expect(b.display?.canvas?.received).toBe(b.display?.canvas?.drawn);
    expect(b.hz?.fit).toBe('match');
  });
  it('a filled beacon passes the shape check; so does one from a page that did not run the meter (the blocks are null)', () => {
    expect(beaconProblems(JSON.parse(beaconText(v2())))).toEqual([]);
    expect(beaconProblems(JSON.parse(beaconText(buildBeacon({ ...base, sid: SID }))))).toEqual([]);
    expect(buildBeacon(base).display).toBeNull();
    expect(buildBeacon(base).hz).toBeNull();
    expect(buildBeacon(base).client).toBeNull();
  });
  it('the physical core count is not sent because it cannot be read: the block says so (value null and why), it never makes one up', () => {
    const b = v2();
    expect(b.client?.physicalCores.value).toBeNull();
    expect(b.client?.physicalCores.reason).toMatch(/logical/);
    expect(b.client?.physicalCores.reason).toMatch(/no physical core count/);
    expect(beaconProblems({ ...JSON.parse(JSON.stringify(b)), client: { ...JSON.parse(JSON.stringify(b.client)), physicalCores: { value: 8, reason: 'x' } } })).toEqual(expect.arrayContaining([expect.stringContaining('physicalCores')]));
  });
  it('method A: the display block has no canvas part (null, not zero)', () => {
    const m = new DisplayMeter();
    m.start(T0, true);
    expect(JSON.parse(JSON.stringify({ c: m.snapshot(T0 + 5, false).canvas })).c).toBeNull();
  });
  it('nothing in it is personal: no address, no URL, no host name, no cookie (the host is a class)', () => {
    const text = beaconText(v2());
    expect(text).not.toMatch(/\d+\.\d+\.\d+\.\d+/);
    expect(text).not.toMatch(/https?:|cookie|href|"origin"|"url"|"ip"|\.ts\.net|tailb77b86|agent-workspace/i);
    expect(JSON.parse(text).client.page).toEqual({ secure: true, hostKind: 'ts.net' });
  });
  it('size: the worst case — 300-char user agent, 200-char GPU, 600 timeline marks, long error text, every histogram full — is cut under the limit, and the page-load token survives', () => {
    const worst = v2({ browser: { ...browser, userAgent: 'U'.repeat(300), webglRenderer: 'G'.repeat(200) }, timeline: Object.fromEntries(Array.from({ length: 600 }, (_, i) => [`mark_${i}`, i * 1.123456789])), error: { what: 'w'.repeat(400), where: 'h'.repeat(200) } });
    expect(JSON.stringify(worst).length).toBeGreaterThan(PERF_MAX_BODY - 512);
    const text = beaconText(worst);
    expect(text.length).toBeLessThanOrEqual(PERF_MAX_BODY - 512);
    expect(JSON.parse(text).sid).toBe(SID);
  });
  it('size: a normal busy beacon (the new blocks included) is under 8 KB, half of the limit', () => {
    const n = beaconText(v2({ timeline: { page_script: 1, manifest_loaded: 40, download_start: 50, compile_done: 3000, download_done: 3100, worker_ready: 3200, engine_start: 3210, main_start: 3300, first_pixel: 9000, idle: 9500 } })).length;
    expect(n).toBeLessThan(8192);
    expect(n).toBeGreaterThan(2000);
  });
  it('the thinning keeps the headline of the screen measurement (the 10 s window) when it has to cut', () => {
    const big = v2({ browser: { ...browser, userAgent: 'U'.repeat(300) }, timeline: Object.fromEntries(Array.from({ length: 400 }, (_, i) => [`m${i}`, i * 1.123456789])) });
    const text = beaconText(big, 5000);
    const j = JSON.parse(text);
    expect(j.display.raf.recent10s.hz).toBeGreaterThan(119);
    expect(j.display.raf.sinceIdle).toBeNull();
  });
});

describe('beaconProblems: engine.exit (decision 0209)', () => {
  const rec = (exit: unknown) => ({ v: 3, sid: SID, engine: { present: 'D', updatesObservable: true, state: 'exited', exit } });
  const probs = (exit: unknown): string[] => beaconProblems(rec(exit)).filter((p) => p.startsWith('engine.exit'));
  it('a record of the engine before this field (no message), with a message, and without an exit are all fine', () => {
    expect(probs({ reason: 'stopped', code: 134, presents: 3 })).toEqual([]);
    expect(probs({ reason: 'stopped', code: 134, presents: 3, message: 'fle: A: why' })).toEqual([]);
    expect(probs(null)).toEqual([]);
    expect(beaconProblems({ v: 3, sid: SID, engine: { present: 'B' } }).filter((p) => p.startsWith('engine.exit'))).toEqual([]);
  });
  it('finds a message that is too long, not a string, with a line break, or with something personal in it', () => {
    expect(probs({ reason: 'stopped', code: 134, presents: 3, message: 'x'.repeat(401) })).toEqual([expect.stringContaining('longer than 400')]);
    expect(probs({ reason: 'stopped', code: 134, presents: 3, message: 5 })).toEqual([expect.stringContaining('not a string')]);
    expect(probs({ reason: 'stopped', code: 134, presents: 3, message: 'a\nb' })).toEqual([expect.stringContaining('control character')]);
    expect(probs({ reason: 'stopped', code: 134, presents: 3, message: 'open /home/ubuntu/x' })).toEqual([expect.stringContaining('home directory')]);
    expect(probs({ reason: 'stopped', code: 134, presents: 3, message: 'open C:\\Users\\Jane\\x' })).toEqual([expect.stringContaining('home directory')]);
    expect(probs({ reason: 'stopped', code: 134, presents: 3, message: 'to a@b.cc' })).toEqual([expect.stringContaining('e-mail')]);
    expect(probs({ reason: 'stopped', code: 134, presents: 3, message: 'see https://x/y' })).toEqual([expect.stringContaining('URL')]);
    expect(probs('stopped')).toEqual([expect.stringContaining('not an object')]);
    expect(probs({ reason: 'stopped', code: 1.5, presents: 3 })).toEqual([expect.stringContaining('not a whole number')]);
  });
});

describe('beaconProblems: numbers that cannot be true are found', () => {
  const good = () => JSON.parse(beaconText(v2()));
  it('a good record has none; a version 1 record (no new blocks) has none', () => {
    expect(beaconProblems(good())).toEqual([]);
    expect(beaconProblems({ v: 1, seq: 0 })).toEqual([]);
  });
  it.each([
    ['a negative Hz', (b: any) => { b.display.raf.recent10s.hz = -1; }, 'display.raf.recent10s.hz'],
    ['an infinite rate', (b: any) => { b.display.raf.total.perSecond = 1e9; }, 'perSecond'],
    ['more slow frames than intervals', (b: any) => { b.display.raf.total.slow = b.display.raf.total.n + 1; }, 'more slow frames than intervals'],
    ['bins that do not add up', (b: any) => { b.display.raf.total.bins[0] += 1; }, 'do not add up'],
    ['the wrong number of bins', (b: any) => { b.display.raf.total.bins = [1, 2]; }, 'bins'],
    ['p95 under the median', (b: any) => { b.display.raf.total.p95Ms = 1; }, 'p95 below the median'],
    ['drawn more than received', (b: any) => { b.display.canvas.drawn = b.display.canvas.received + 5; }, 'more frames than it received'],
    ['a fresh ratio above 1', (b: any) => { b.display.canvas.total.freshRatio = 1.5; }, 'freshRatio'],
    ['more fresh ticks than ticks', (b: any) => { b.display.canvas.total.freshTicks = b.display.canvas.total.ticks + 1; }, 'more fresh ticks'],
    ['a string where a number goes', (b: any) => { b.display.visibility.hiddenMs = '5'; }, 'hiddenMs'],
    ['an unknown fit', (b: any) => { b.hz.fit = 'perfect'; }, 'hz.fit'],
    ['a browser name that is 100 characters', (b: any) => { b.client.browser.name = 'x'.repeat(100); }, 'client.browser.name'],
    ['a bad token', (b: any) => { b.sid = 'xyz'; }, 'sid'],
    ['a fractional count', (b: any) => { b.display.longTasks.total = 1.5; }, 'whole number'],
    ['version 4', (b: any) => { b.v = 4; }, 'outside'],
    ['version 1 with the new blocks', (b: any) => { b.v = 1; }, 'outside'],
    ['an input block that is not an object', (b: any) => { b.input = 5; }, 'input: not an object'],
    ['an input block without eventToRaf (a version 3 beacon must say which definition)', (b: any) => { delete b.input.eventToRaf; }, 'input.eventToRaf: not an object'],
    ['the old flat p95 left as if it were the new number', (b: any) => { delete b.input.listenerToPut; }, 'input.listenerToPut: not an object'],
    ['a negative latency', (b: any) => { b.input.eventToRaf.lastMs[0] = -3; }, 'input.eventToRaf.lastMs[0]'],
    ['p95 under the median', (b: any) => { b.input.eventToRaf.p95Ms = 1; }, 'input.eventToRaf: p95 below the median'],
    ['more values than n', (b: any) => { b.input.eventToRaf.n = 1; }, 'more values than n'],
    ['a median with no values', (b: any) => { b.input.listenerToPut.n = 0; b.input.listenerToPut.lastMs = []; }, 'no values but a median'],
    ['a stage that is below 0 where it cannot be', (b: any) => { b.input.stages.p50.raf = -1; }, 'input.stages.p50.raf'],
    ['more staged events than events', (b: any) => { b.input.stages.n = 5; }, 'more events than eventToRaf has'],
    ['a stage missing', (b: any) => { delete b.input.stages.p95.worker; }, 'input.stages.p95.worker'],
    ['the counts of the answered inputs missing (decision 0205: an unanswered input must be visible)', (b: any) => { delete b.input.noAnswer; }, 'input.noAnswer'],
    ['lateVisible missing (it is an object or null, never absent)', (b: any) => { delete b.input.lateVisible; }, 'input.lateVisible: not an object or null'],
    ['lateVisible without eventToRafWithLate', (b: any) => { b.input.lateVisible = { n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] }; }, 'both be null or both be there'],
    ['a count that is not a whole number', (b: any) => { b.input.late = 0.5; }, 'input.late'],
    ['more answered, late and unanswered inputs than processed ones', (b: any) => { b.input.noAnswer = 1; b.input.answered = 3; }, 'answered + late + noAnswer exceeds processed'],
  ])('%s', (_name, mutate, text) => {
    const b = good();
    mutate(b);
    const p = beaconProblems(b);
    expect(p.length).toBeGreaterThan(0);
    expect(p.join('\n')).toContain(text);
  });
  it('not an object', () => { expect(beaconProblems([])).toHaveLength(1); expect(beaconProblems(null)).toHaveLength(1); });
  it('a stage that crosses between the page\'s and the Worker\'s clock may be a little below 0 (the two clocks differ by a fraction of a millisecond); the others may not', () => {
    const b = good();
    b.input.stages.p50.toWorker = -0.2;
    b.input.stages.p50.fromWorker = -0.1;
    expect(beaconProblems(b)).toEqual([]);
    b.input.stages.p50.toWorker = -500;
    expect(beaconProblems(b).join('\n')).toContain('input.stages.p50.toWorker');
  });
  it('a record with no event and no animation frame yet has empty statistics and no stages, and passes', () => {
    const empty = { n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] };
    const b = JSON.parse(beaconText(v2({ input: { sent: 0, processed: 0, dropped: 0, pending: 0, answered: 0, late: 0, noAnswer: 0, eventToRaf: empty, listenerToPut: empty, lateVisible: null, eventToRafWithLate: null, stages: null } })));
    expect(b.input.stages).toBeNull();
    expect(beaconProblems(b)).toEqual([]);
  });
  it('a version 2 record (the flat input of decision 0192) is still accepted: its input block is not checked, because it is the old definition', () => {
    const b = good();
    b.v = 2;
    b.input = { sent: 3, processed: 3, dropped: 0, pending: 0, n: 2, p50Ms: 10, p95Ms: 20, maxMs: 20, lastMs: [10, 20], targetP95Ms: 17 };
    expect(beaconProblems(b)).toEqual([]);
  });
  it('the worst case still fits: with 20 values of each definition and the stages, the beacon with every histogram full is under the limit', () => {
    const lastMs = Array.from({ length: 20 }, (_, i) => 12.345 + i * 3.14159);
    const full = { n: 20, p50Ms: 40.123, p95Ms: 70.456, maxMs: 80.789, lastMs };
    const text = beaconText(v2({ input: { ...input, eventToRaf: full, listenerToPut: full, lateVisible: full, eventToRafWithLate: full } }), PERF_MAX_BODY - 512);
    expect(text.length).toBeLessThanOrEqual(PERF_MAX_BODY - 512);
    expect(JSON.parse(text).input.eventToRaf.p95Ms).toBe(70.456);
    expect(beaconProblems(JSON.parse(text))).toEqual([]);
  });
  it('cut to the last resort, the lists of values are thinned and both definitions stay named', () => {
    const lastMs = Array.from({ length: 20 }, (_, i) => 12 + i);
    const full = { n: 20, p50Ms: 20, p95Ms: 30, maxMs: 31, lastMs };
    const j = JSON.parse(beaconText(v2({ input: { ...input, eventToRaf: full, listenerToPut: full, lateVisible: full, eventToRafWithLate: full } }), 800));
    expect(j.input.eventToRaf.lastMs).toHaveLength(5);
    expect(j.input.listenerToPut.lastMs).toHaveLength(5);
    expect(j.input.lateVisible.lastMs).toHaveLength(5);
    expect(j.input.eventToRafWithLate.lastMs).toHaveLength(5);
    expect(j.input.eventToRaf.p95Ms).toBe(30);
    expect(beaconProblems(j)).toEqual([]);
  });
});

describe('browser and client info (decision 0192)', () => {
  it.each([
    ['Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36', 'Chrome', '156.0', 156, 'Windows', false],
    ['Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/156.0.0.0 Safari/537.36', 'Chrome', '156.0', 156, 'Linux', true],
    ['Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36 Edg/156.0.2800.31', 'Edge', '156.0', 156, 'Windows', false],
    ['Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0', 'Firefox', '128.0', 128, 'Windows', false],
    ['Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15', 'Safari', '17.5', 17, 'macOS', false],
    ['Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36 OPR/110.0.0.0', 'Opera', '110.0', 110, 'Windows', false],
    ['Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Mobile Safari/537.36', 'Chrome', '156.0', 156, 'Android', false],
  ])('%s', (ua, name, version, major, os, headless) => { expect(parseBrowser(ua)).toEqual({ name, version, major, os, headless }); });
  it('an unreadable user agent is "unknown", not a guess; the version never has four parts (it would look like an address)', () => {
    expect(parseBrowser('')).toEqual({ name: 'unknown', version: null, major: null, os: null, headless: false });
    expect(parseBrowser('curl/8.0')).toMatchObject({ name: 'unknown', version: null });
    expect(parseBrowser('Chrome/156.0.0.0').version).toBe('156.0');
  });
  it('the OS comes from userAgentData.platform when there is one', () => {
    expect(parseBrowser('Mozilla/5.0 (X11; Linux x86_64) Chrome/156.0.0.0', 'Chrome OS').os).toBe('Chrome OS');
  });
  it('hostKind is a class: localhost, ts.net, ip, other; never the name', () => {
    expect(hostKind('localhost')).toBe('localhost');
    expect(hostKind('127.0.0.1')).toBe('localhost');
    expect(hostKind('[::1]')).toBe('localhost');
    expect(hostKind('machine.example.ts.net')).toBe('ts.net');
    expect(hostKind('192.168.0.5')).toBe('ip');
    expect(hostKind('[fd7a::1]')).toBe('ip');
    expect(hostKind('example.com')).toBe('other');
    expect(hostKind(undefined)).toBeNull();
  });
  it('collectClientInfo reads the colour depth, the viewport and how the page was reached; missing globals are null', () => {
    expect(client).toMatchObject({ colorDepth: 24, viewport: { width: 1900, height: 960 }, page: { secure: true, hostKind: 'ts.net' } });
    expect(collectClientInfo({})).toMatchObject({ browser: { name: 'unknown' }, colorDepth: null, viewport: null, page: { secure: null, hostKind: null } });
  });
});
