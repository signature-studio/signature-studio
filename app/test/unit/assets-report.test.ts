// Decision 0220: the start-up assets report (src/bundle/assets-report.ts), its beacon block (shared/perf.ts assetsBlock), the panel rows and the Download stage's note.
import { describe, expect, it } from 'vitest';
import { AssetsRecorder, assetLines, classifyNetwork, resourceLookupFromPerformance, sourceSummary, type AssetsReport } from '../../src/bundle/assets-report.ts';
import type { AssetTrace } from '../../src/bundle/loader.ts';
import { buildBeacon, type BeaconInput } from '../../src/perf/beacon.ts';
import { assetRows, perfRows, type PanelView } from '../../src/perf/panel.ts';
import { Timeline, stageRows } from '../../src/ui/phases.ts';
import { beaconProblems } from '../../shared/perf.ts';

const SHA = { wasm: 'f4a57e84a5e16a89820b9b939b17ffe4c65896b69cff50367c13e6f6fb5efab4', dll: '50206405ba5f9d88c070c97ab73366f1564ae1b9215b6f3d073aca3971cd8e77', seed: '0f4b547e66566d18ee240e233066dc2db77c0ff305cc51f8ab8d3a8fa212b938', glue: 'b5952d3e51650c9b3c511263d9f32f9858b042fb17d0ab0a24ac05b31d454df8' };
const URLS = { 'engine wasm': 'http://x/assets/b/engine.wasm', 'DLL pack': 'http://x/assets/b/dll.flpack', 'seed pack': 'http://x/assets/b/seed.flpack', 'engine glue': 'http://x/assets/b/glue.js' };
const SHAS: Record<string, string> = { 'engine wasm': SHA.wasm, 'DLL pack': SHA.dll, 'seed pack': SHA.seed, 'engine glue': SHA.glue };
const trace = (label: string, over: Partial<AssetTrace> = {}): AssetTrace => ({ label, url: URLS[label as keyof typeof URLS], bytes: 1000, origin: 'network', hash: 'subtle', store: 'stored', fail: null, headersMs: 12.4, readMs: 300.6, hashMs: 50, copyMs: 7, storeMs: 800, totalMs: 900, compileMs: null, parseMs: 0, ...over });
const recorder = (store: 'on' | 'off' | 'none' = 'on', legacy = false): AssetsRecorder => {
  const r = new AssetsRecorder(store, legacy ? 'js-stream' : 'subtle', legacy);
  for (const [l, u] of Object.entries(URLS)) r.expect(l, { url: u, size: 1000, sha256: SHAS[l] as string });
  return r;
};

describe('classifyNetwork', () => {
  it('the browser\'s own word wins: deliveryType cache is the HTTP cache (also after a header-only revalidation), anything else is the network', () => {
    expect(classifyNetwork({ transfer: 0, encoded: 11339214, delivery: 'cache' })).toEqual({ origin: 'http-cache', wire: 11339214, transfer: 0 });
    expect(classifyNetwork({ transfer: 300, encoded: 11339214, delivery: 'cache' }).origin).toBe('http-cache');
    expect(classifyNetwork({ transfer: 11339514, encoded: 11339214, delivery: '' }).origin).toBe('network');
  });
  it('without deliveryType, no transferred byte for a body that exists means the cache; no entry at all is the network with unknown sizes', () => {
    expect(classifyNetwork({ transfer: 0, encoded: 100, delivery: '' }).origin).toBe('http-cache');
    expect(classifyNetwork({ transfer: 0, encoded: 0, delivery: '' }).origin).toBe('network');
    expect(classifyNetwork(null)).toEqual({ origin: 'network', wire: null, transfer: null });
  });
  it('reads the entry of the URL from Performance.getEntriesByName', () => {
    const perf = { getEntriesByName: (u: string) => (u === 'http://x/a' ? [{ transferSize: 5, encodedBodySize: 9, deliveryType: 'cache' }, { transferSize: 7, encodedBodySize: 9, deliveryType: '' }] : []) } as unknown as Pick<Performance, 'getEntriesByName'>;
    expect(resourceLookupFromPerformance(perf)('http://x/a')).toEqual({ transfer: 7, encoded: 9, delivery: '' });
    expect(resourceLookupFromPerformance(perf)('http://x/none')).toBeNull();
  });
});

describe('AssetsRecorder.report', () => {
  it('lists the four assets in the fixed order with an 8-digit hash prefix and whole milliseconds, and takes the last trace of an asset (the background write ends later)', () => {
    const r = recorder();
    r.trace(trace('engine glue')); r.trace(trace('seed pack')); r.trace(trace('DLL pack')); r.trace(trace('engine wasm', { compileMs: 1500.4, store: 'writing', storeMs: 0 }));
    r.trace(trace('engine wasm', { compileMs: 1500.4, store: 'stored', storeMs: 2100.5 }));
    const rep = r.report(() => ({ transfer: 0, encoded: 77, delivery: 'cache' }));
    expect(rep.items.map((i) => i.kind)).toEqual(['wasm', 'dll', 'seed', 'glue']);
    expect(rep.items[0]).toMatchObject({ sha: 'f4a57e84', origin: 'http-cache', wire: 77, transfer: 0, store: 'stored', ms: { headers: 12, read: 301, hash: 50, copy: 7, parse: 0, store: 2101, total: 900, compile: 1500 } });
    expect(rep.store).toBe('on'); expect(rep.hash).toBe('subtle');
  });
  it('a stored-copy asset has no wire or transfer numbers (nothing was requested)', () => {
    const r = recorder();
    r.trace(trace('engine wasm', { origin: 'storage-cache', store: 'hit' }));
    expect(r.report(() => null).items[0]).toMatchObject({ origin: 'storage-cache', wire: null, transfer: null, store: 'hit' });
  });
  it('an unknown failure code becomes "other": no text of an error can travel', () => {
    const r = recorder();
    r.trace(trace('engine wasm', { store: 'failed', fail: 'C:\\Users\\someone\\x' }));
    expect(r.report(() => null).items[0]?.fail).toBe('other');
  });
  it('the original path makes no trace: the report lists the manifest\'s assets with the browser\'s resource timing and zero times', () => {
    const r = recorder('off', true);
    const rep = r.report(() => ({ transfer: 0, encoded: 5, delivery: 'cache' }));
    expect(rep.items).toHaveLength(4);
    expect(rep.items.every((i) => i.origin === 'http-cache' && i.store === 'off' && i.ms.read === 0)).toBe(true);
    expect(rep.hash).toBe('js-stream');
  });
  it('writesSettled waits for the last asset that is being written, and stored() says which assets have a stored copy', async () => {
    const r = recorder();
    r.trace(trace('engine wasm', { store: 'writing' }));
    r.trace(trace('DLL pack', { store: 'hit', origin: 'storage-cache' }));
    let done = false;
    const p = r.writesSettled(5000).then(() => { done = true; });
    await Promise.resolve();
    expect(done).toBe(false);
    expect(r.stored({ sha256: SHA.dll })).toBe(true);
    expect(r.stored({ sha256: SHA.wasm })).toBe(false);
    r.trace(trace('engine wasm', { store: 'stored' }));
    await p;
    expect(done).toBe(true);
    expect(r.stored({ sha256: SHA.wasm })).toBe(true);
  });
});

describe('the lines and the summary', () => {
  const rep = (): AssetsReport => { const r = recorder(); r.trace(trace('engine wasm', { origin: 'storage-cache', store: 'hit', compileMs: 700, bytes: 130217196 })); r.trace(trace('DLL pack', { parseMs: 20.2 })); return r.report(() => ({ transfer: 0, encoded: 3, delivery: 'cache' })); };
  it('says where the bytes came from, the hash used, and the times', () => {
    const lines = assetLines(rep());
    expect(lines[0]).toContain('wasm 124.2 MiB from the page\'s own stored copy (OPFS)');
    expect(lines[0]).toContain('WebCrypto');
    expect(lines[0]).toContain('compiled at 700 ms');
    expect(lines[1]).toContain("from the browser's HTTP cache, no byte transferred");
    expect(lines[1]).toContain('pack index 20 ms');
  });
  it('the summary counts the sources', () => { expect(sourceSummary(rep())).toBe('own stored copy ×1, HTTP cache ×1'); });
});

describe('the beacon block', () => {
  const input: BeaconInput = {
    seq: 0, sid: '0123456789ab', reason: 'first-idle', pageMs: 1, visibility: 'visible', browser: { hardwareConcurrency: 8, deviceMemoryGB: 8, userAgent: 'UA', screen: { width: 1, height: 1 }, devicePixelRatio: 1, crossOriginIsolated: true, isSecureContext: true, webglRenderer: null, jsHeap: null },
    bundle: { name: 'b', version: '1', engineKind: 'emscripten-fruity-v1' }, state: 'engine-idle', exit: null, error: null, timeline: {},
    updates: { total: 0, recent: { n: 0, intervalMedianMs: null, intervalP95Ms: null, perSecond: null } }, idles: { total: 0, recent: { n: 0, intervalMedianMs: null, intervalP95Ms: null, perSecond: null } },
    input: { sent: 0, processed: 0, dropped: 0, pending: 0, answered: 0, late: 0, noAnswer: 0, eventToRaf: { n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] }, listenerToPut: { n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] }, lateVisible: null, eventToRafWithLate: null, stages: null },
    underruns: null,
  };
  const full = (): AssetsReport => { const r = recorder(); for (const l of Object.keys(URLS)) r.trace(trace(l)); r.setStorage({ usedMiB: 200, quotaMiB: 10000, persisted: false }); r.setClean({ removed: 0, kept: 4, busy: 0, failed: 0 }); return r.report(() => ({ transfer: 0, encoded: 3, delivery: 'cache' })); };
  it('is carried when given, left out when not, and passes the shape check', () => {
    expect('assets' in buildBeacon(input)).toBe(false);
    const b = buildBeacon({ ...input, assets: full() });
    expect(b.v).toBe(3);
    expect(b.assets?.items).toHaveLength(4);
    expect(beaconProblems(b)).toEqual([]);
    expect(beaconProblems(buildBeacon(input))).toEqual([]);
  });
  const problems = (mut: (a: Record<string, unknown>) => void): string[] => { const b = JSON.parse(JSON.stringify(buildBeacon({ ...input, assets: full() }))); mut(b.assets); return beaconProblems(b); };
  it('finds what cannot be true: a word outside the lists, a name instead of a code, a long hash, origin and store that contradict, a negative time', () => {
    expect(problems((a) => { (a.items as Record<string, unknown>[])[0]!.kind = 'engine.wasm'; }).join('\n')).toContain('assets.items[0].kind');
    expect(problems((a) => { (a.items as Record<string, unknown>[])[0]!.fail = 'C:\\Users\\x'; }).join('\n')).toContain('assets.items[0].fail');
    expect(problems((a) => { (a.items as Record<string, unknown>[])[0]!.sha = 'f4a57e84a5e16a89'; }).join('\n')).toContain('first 8 hex digits');
    expect(problems((a) => { const i = (a.items as Record<string, unknown>[])[0]!; i.origin = 'storage-cache'; i.store = 'stored'; }).join('\n')).toContain('must say store hit');
    expect(problems((a) => { (((a.items as Record<string, unknown>[])[0]!.ms) as Record<string, unknown>).read = -1; }).join('\n')).toContain('assets.items[0].ms.read');
    expect(problems((a) => { a.hash = 'md5'; }).join('\n')).toContain('assets.hash');
    expect(problems((a) => { a.items = [...(a.items as unknown[]), ...(a.items as unknown[])]; }).join('\n')).toContain('at most 4');
    expect(problems((a) => { a.v = 2; }).join('\n')).toContain('assets.v');
  });
});

describe('the panel and the stage', () => {
  const rep = (): AssetsReport => { const r = recorder(); for (const l of Object.keys(URLS)) r.trace(trace(l, { origin: 'storage-cache', store: 'hit' })); r.setStorage({ usedMiB: 200, quotaMiB: 10000, persisted: false }); r.setClean({ removed: 2, kept: 4, busy: 1, failed: 0 }); return r.report(() => null); };
  it('one summary row and one row per asset; never green or red', () => {
    const rows = assetRows(rep());
    expect(rows.map((r) => r.id)).toEqual(['assets', 'assets-wasm', 'assets-dll', 'assets-seed', 'assets-glue']);
    expect(rows[0]?.value).toContain('own stored copy ×4');
    expect(rows[0]?.value).toContain('may be evicted when the disk runs low');
    expect(rows[0]?.value).toContain('old files removed 2, kept 4, in use 1');
    expect(rows.every((r) => r.status === 'na')).toBe(true);
  });
  it('a dash before the downloads begin; "nothing recorded" with an empty report', () => {
    expect(assetRows(null)[0]?.value).toBe('-');
    expect(assetRows({ v: 1, store: 'off', hash: 'js-stream', items: [], storage: null, clean: null })[0]?.value).toContain('nothing recorded');
  });
  it('perfRows puts them after P1', () => {
    const stat0 = { n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] };
    const input0 = { sent: 0, processed: 0, dropped: 0, pending: 0, answered: 0, late: 0, noAnswer: 0, eventToRaf: stat0, listenerToPut: stat0, lateVisible: null, eventToRafWithLate: null, stages: null };
    const v: PanelView = { waitingSeconds: null, firstIdleSeconds: 5, updates: { n: 0, intervalMedianMs: null, intervalP95Ms: null, perSecond: null }, input: input0, underruns: null, server: { sent: 0, stored: 0, accepted: 0, lastNote: null }, assets: rep() };
    const ids = perfRows(v).map((r) => r.id);
    expect(ids.indexOf('assets')).toBe(ids.indexOf('p1') + 1);
  });
  it('the Download row says where the assets came from', () => {
    const tl = new Timeline();
    tl.mark('download_start', 100); tl.mark('download_done', 900);
    expect(stageRows(tl, { now: 1000, assetSources: 'own stored copy ×4' })[0]?.detail).toContain('; from: own stored copy ×4');
    expect(stageRows(tl, { now: 1000 })[0]?.detail).not.toContain('from:');
  });
});
