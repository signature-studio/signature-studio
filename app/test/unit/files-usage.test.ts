// Decision 0216: what the page counts about Files for the measurement beacon (src/files/usage.ts), and that nothing personal can travel in it.
import { describe, expect, it } from 'vitest';
import { FILES_FAIL_CODES, UsageCounter, failCode } from '../../src/files/usage.ts';
import { buildBeacon, type BeaconInput } from '../../src/perf/beacon.ts';
import type { BrowserInfo } from '../../src/perf/browser-info.ts';
import { PERF_FILES_FAIL_CODES, beaconProblems } from '../../shared/perf.ts';

const browser: BrowserInfo = { hardwareConcurrency: 8, deviceMemoryGB: 8, userAgent: 'UA/1', screen: { width: 2560, height: 1440 }, devicePixelRatio: 1.5, crossOriginIsolated: true, isSecureContext: true, webglRenderer: 'ANGLE (X)', jsHeap: { usedBytes: 1, totalBytes: 2, limitBytes: 3 } };
const stat = (n: number): { n: number; intervalMedianMs: number | null; intervalP95Ms: number | null; perSecond: number | null } => ({ n, intervalMedianMs: 8, intervalP95Ms: 9, perSecond: 120 });
const input = {
  sent: 3, processed: 3, dropped: 0, pending: 0, answered: 3, late: 0, noAnswer: 0,
  eventToRaf: { n: 2, p50Ms: 30, p95Ms: 40, maxMs: 40, lastMs: [30, 40] }, listenerToPut: { n: 2, p50Ms: 10, p95Ms: 20, maxMs: 20, lastMs: [10, 20] }, lateVisible: null, eventToRafWithLate: null,
  stages: { n: 2, p50: { s0: 13, s0b: 0.3, toWorker: 0.5, worker: 2.5, fromWorker: 0.4, put: 0.2, raf: 13 }, p95: { s0: 15, s0b: 0.5, toWorker: 0.9, worker: 3, fromWorker: 0.6, put: 0.4, raf: 16 } },
};
const base: BeaconInput = {
  seq: 4, sid: '0123456789ab', reason: 'periodic', pageMs: 123456.5, visibility: 'visible', browser, bundle: { name: 'fl-Oz', version: '2', engineKind: 'emscripten-fruity-v1' },
  state: 'engine-idle', exit: null, error: null, timeline: { download_start: 100, idle: 38750 }, updates: { total: 500, recent: stat(10) }, idles: { total: 40, recent: stat(5) }, input, underruns: null,
};

describe('failCode', () => {
  // the texts are the ones the file manager really produces (opfs.ts describeError, zip-read.ts, paths.ts, ops.ts)
  const cases: [string, string][] = [
    ['CRC-32 mismatch (the zip is damaged)', 'zip-crc'],
    ['compression method 12 is not supported (only stored and deflate)', 'zip-method'],
    ['encrypted entry', 'zip-encrypted'],
    ['unsafe path (.. or a drive letter)', 'unsafe-path'],
    ['This is not a zip (no end-of-central-directory record).', 'zip-format'],
    ['Central directory entry 3 is damaged.', 'zip-format'],
    ['the entry runs past the end of the file', 'zip-format'],
    ['size 5 instead of 9', 'zip-size'],
    ['The file is in use (open in the engine or another tab), so it cannot be changed.', 'locked'],
    ['The browser has no room left for this (storage quota).', 'quota'],
    ['There is not enough room: 5 MiB to add', 'quota'],
    ['Not found.', 'notfound'],
    ['A file and a folder have the same name.', 'kind'],
    ['a file is in the way of a folder', 'kind'],
    ['A name cannot contain ":". (The engine\'s Windows rules apply here.)', 'name'],
    ['"CON" is a reserved device name on Windows.', 'name'],
    ['That name is already used here.', 'conflict'],
    ['Unpacking was cancelled.', 'cancelled'],
    ['this browser has no DecompressionStream', 'unsupported'],
    ['SecurityError: x', 'security'],
    ['NotReadableError: x', 'io'],
    ['something nobody expected', 'other'],
  ];
  it.each(cases)('%s -> %s', (text, code) => { expect(failCode(text)).toBe(code); });
  it('reads DOMException-like errors by name', () => {
    expect(failCode({ name: 'QuotaExceededError', message: 'x' })).toBe('quota');
    expect(failCode({ name: 'NoModificationAllowedError', message: 'x' })).toBe('locked');
    expect(failCode({ name: 'NotFoundError', message: '' })).toBe('notfound');
    expect(failCode(new Error('boom'))).toBe('other');
  });
  it('every code of the list is reachable and nothing else is ever returned', () => {
    const seen = new Set(cases.map(([, c]) => c));
    for (const c of FILES_FAIL_CODES) expect(seen.has(c) || c === 'io' || c === 'other' || c === 'security', c).toBe(true);
    for (const [t] of cases) expect((FILES_FAIL_CODES as readonly string[]).includes(failCode(t))).toBe(true);
    expect(FILES_FAIL_CODES).toBe(PERF_FILES_FAIL_CODES);
  });
});

describe('UsageCounter', () => {
  it('is null until something is counted: a page that never used Files sends no block', () => {
    const u = new UsageCounter();
    expect(u.snapshot()).toBeNull();
    u.c.opens++;
    expect(u.snapshot()?.opens).toBe(1);
    u.reset();
    expect(u.snapshot()).toBeNull();
  });
  it('counts operations, bytes in MiB rounded down, and failure codes', () => {
    const u = new UsageCounter();
    u.result({ written: 3, skipped: 1, failed: [{ reason: 'CRC-32 mismatch (the zip is damaged)' }, { reason: 'x.wav: something' }], bytes: 5 * 1048576 + 7 });
    u.result({ written: 1, skipped: 0, failed: [], bytes: 3 * 1048576 });
    u.storage(2.5 * 1048576, 10 * 1048576 * 1024, false);
    const s = u.snapshot()!;
    expect(s).toMatchObject({ v: 1, written: 4, skipped: 1, failed: 2, writtenMiB: 8, fail: { 'zip-crc': 1, other: 1 }, storage: { usedMiB: 2, quotaMiB: 10240, persisted: false } });
  });
  it('the snapshot is a copy', () => {
    const u = new UsageCounter();
    u.c.opens = 2;
    const s = u.snapshot()!;
    s.opens = 99;
    expect(u.snapshot()!.opens).toBe(2);
  });
  it('PRIVACY: after a busy session the JSON holds only the fixed keys, numbers, booleans/null and codes of the list: no name, no path, no text', () => {
    const u = new UsageCounter();
    const names = ['Song.flp', '日本語 サンプル.wav', 'C:\\Users\\cabbage\\x.wav', '[fadr.com] Vocals - クリスマスセリナ.wav'];
    for (const n of names) { u.result({ written: 1, skipped: 0, failed: [{ reason: `${n}: NoModificationAllowedError` }, { reason: `${n} was not found` }, { reason: `${n} is gone` }], bytes: 10 }); u.fail(new Error(`${n} broke`)); }
    u.c.pick.folder++; u.c.drop.folders++; u.c.engine.open++; u.c.persist.asked++;
    const json = JSON.stringify(u.snapshot());
    for (const n of names) expect(json).not.toContain(n.slice(0, 4));
    const walk = (v: unknown, path: string): void => {
      if (v === null || typeof v === 'number' || typeof v === 'boolean') return;
      if (typeof v === 'string') throw new Error(`a string value at ${path}: ${v}`);
      if (typeof v === 'object') for (const [k, x] of Object.entries(v as Record<string, unknown>)) {
        if (path === 'fail') expect((FILES_FAIL_CODES as readonly string[]).includes(k), `fail code ${k}`).toBe(true);
        else expect(/^[a-zA-Z]{1,16}$/.test(k), `key ${k}`).toBe(true);
        walk(x, path === '' ? k : `${path}.${k}`);
      }
    };
    walk(JSON.parse(json), '');
  });
});

describe('the beacon block', () => {
  const used = (): UsageCounter => { const u = new UsageCounter(); u.c.opens = 2; u.c.pick.zip = 1; u.result({ written: 49, skipped: 0, failed: [{ reason: 'CRC-32 mismatch' }], bytes: 62747002 }); u.storage(60e6, 1e10, true); return u; };
  it('is in the payload only when Files was used; version stays 3', () => {
    const none = buildBeacon({ ...base, files: new UsageCounter().snapshot() });
    expect('files' in none).toBe(false);
    expect(none.v).toBe(3);
    const b = buildBeacon({ ...base, files: used().snapshot() });
    expect(b.files?.opens).toBe(2);
    expect(b.v).toBe(3);
    expect(beaconProblems(b)).toEqual([]);
    expect(beaconProblems(none)).toEqual([]);
  });
  it('the validation rejects what the page cannot have written', () => {
    const ok = buildBeacon({ ...base, files: used().snapshot() });
    const bad = (mut: (f: Record<string, any>) => void): string[] => { const f = JSON.parse(JSON.stringify(ok.files)); mut(f); return beaconProblems({ ...ok, files: f }); };
    expect(bad((f) => { f.fail['Song.flp'] = 1; }).join()).toMatch(/not a failure code/);
    expect(bad((f) => { f.fail.other = -1; }).join()).toMatch(/outside/);
    expect(bad((f) => { f.opens = 1.5; }).join()).toMatch(/whole number/);
    expect(bad((f) => { f.pick.secret = 3; }).join()).toMatch(/not a counter of this block/);
    expect(bad((f) => { f.storage.persisted = 'yes'; }).join()).toMatch(/not a boolean/);
    expect(bad((f) => { f.v = 2; }).join()).toMatch(/files.v/);
    expect(bad((f) => { delete f.engine; }).join()).toMatch(/files.engine/);
    expect(beaconProblems({ ...ok, files: 'x' }).join()).toMatch(/files: not an object/);
  });
});
