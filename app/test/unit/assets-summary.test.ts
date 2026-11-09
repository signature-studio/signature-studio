// Decision 0220: scripts/perf-log-summary.mjs reads the beacon's optional `assets` block and tells, per browser, how the people who use the page got their start-up bytes and what the download window cost.
import { describe, expect, it } from 'vitest';
import { format, summarize } from '../../scripts/perf-log-summary.mjs';

const item = (kind: string, origin: string, ms: Record<string, number>, over: Record<string, unknown> = {}): Record<string, unknown> => ({ kind, sha: 'f4a57e84', bytes: 1000, wire: origin === 'storage-cache' ? null : 10, transfer: origin === 'network' ? 10 * 1048576 : origin === 'http-cache' ? 0 : null, origin, store: origin === 'storage-cache' ? 'hit' : 'stored', fail: null, ms: { headers: 5, read: 100, hash: 50, copy: 5, parse: 0, store: 0, total: 200, compile: null, ...ms }, ...over });
const block = (origins: string[], over: Record<string, unknown> = {}, msOver: Record<string, number> = {}): Record<string, unknown> => ({ v: 1, store: 'on', hash: 'subtle', items: origins.map((o, i) => item(['wasm', 'dll', 'seed', 'glue'][i] as string, o, msOver)), storage: { usedMiB: 200, quotaMiB: 10000, persisted: false }, clean: null, ...over });
const rec = (sid: string, ua: string, assets: unknown, idleMs: number, dl: [number, number] = [100, 4600]): string => JSON.stringify({
  t: '2026-11-04T01:00:00.000Z', remote: 'tailnet',
  perf: { v: 3, sid, seq: 0, reason: 'first-idle', bundle: { name: 'fl', version: '1', engineKind: 'emscripten-fruity-v1', standIn: false }, startup: { firstIdleSeconds: idleMs / 1000, targetSeconds: 10 }, timeline: { download_start: dl[0], download_done: dl[1], compile_done: dl[1] + 300, idle: idleMs },
    audio: { underruns: null }, browser: { hardwareConcurrency: 8, deviceMemoryGB: 8, userAgent: ua, screen: { width: 2176, height: 1224 }, devicePixelRatio: 1, webglRenderer: 'GTX 980' }, error: null, ...(assets ? { assets } : {}) },
});
const FF = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:159.0) Gecko/20100101 Firefox/159.0';

describe('the assets block in the log summary', () => {
  const text = [
    rec('aaaaaaaaaaaa', FF, block(['network', 'network', 'network', 'network']), 10400),
    rec('bbbbbbbbbbbb', FF, block(['http-cache', 'http-cache', 'http-cache', 'http-cache']), 10300, [100, 4500]),
    rec('cccccccccccc', FF, block(['storage-cache', 'storage-cache', 'storage-cache', 'storage-cache'], {}, { read: 20, hash: 10 }), 6900, [100, 1200]),
    rec('dddddddddddd', FF, block(['storage-cache', 'network', 'http-cache', 'http-cache']), 8000),
    rec('eeeeeeeeeeee', FF, null, 11000),
  ].join('\n');
  it('classifies each run by where its bytes came from: cold, HTTP cache, own stored copy, mixed, no block; keeps the browsers apart', () => {
    const s = summarize(text);
    expect(s.assets.runs).toBe(5);
    expect(s.assets.withBlock).toBe(4);
    expect(s.assets.rows.map((r) => r.class).sort()).toEqual(['HTTP cache (and own stored copy)', 'cold (network)', 'mixed', 'no assets block', 'own stored copy'].sort());
  });
  it('per class: the medians of first idle, of the download window and of the sums of read / hash / copy', () => {
    const s = summarize(text);
    const own = s.assets.rows.find((r) => r.class === 'own stored copy');
    expect(own).toMatchObject({ runs: 1, firstIdleSecondsMedian: 6.9, downloadMsMedian: 1100, readMsMedian: 80, hashMsMedian: 40, copyMsMedian: 20, hash: 'subtle', storeFailures: 0 });
    const cold = s.assets.rows.find((r) => r.class === 'cold (network)');
    expect(cold?.transferMiBMedian).toBe(40);
    expect(s.assets.rows.find((r) => r.class === 'HTTP cache (and own stored copy)')?.transferMiBMedian).toBe(0);
  });
  it('failure codes and the protection state of the browser\'s storage are counted', () => {
    const t = rec('ffffffffffff', FF, block(['network', 'network', 'network', 'network'], { storage: { usedMiB: 1, quotaMiB: 2, persisted: true } }), 9000).replace('"fail":null', '"fail":"quota"');
    const s = summarize(t);
    expect(s.assets.fails).toEqual({ quota: 1 });
    expect(s.assets.persisted).toBe(1);
  });
  it('the text names the section, the classes and says the log cannot pair loads', () => {
    const out = format(summarize(text));
    expect(out).toContain('Start-up assets (decision 0220; real engine; 4 of 5 runs sent an assets block');
    expect(out).toContain('Firefox 159 | own stored copy | 1 run(s) | first idle median 6.9 s | download window median 1100 ms');
    expect(out).toContain('not paired with the first');
  });
  it('an empty log or a log without the block says so', () => {
    expect(format(summarize(''))).toContain('0 of 0 runs sent an assets block');
    expect(format(summarize(rec('gggggggggggg', FF, null, 9000)))).toContain('no run sent it');
  });
});
