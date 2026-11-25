// Issue #1418: the calls the engine answers without a model (lift/runtime/fle_env.c fle_warn) reach the beacon as `warns` (src/perf/warns.ts): the last counts line of the engine's log, the top 20.
import { describe, expect, it } from 'vitest';
import { buildBeacon, type BeaconInput } from '../../src/perf/beacon.ts';
import { WARNS_TOP, warnsOf } from '../../src/perf/warns.ts';
import { beaconProblems, warnsValid } from '../../shared/perf.ts';
import type { BrowserInfo } from '../../src/perf/browser-info.ts';

const stat = (n: number): { n: number; intervalMedianMs: number | null; intervalP95Ms: number | null; perSecond: number | null } => ({ n, intervalMedianMs: 8, intervalP95Ms: 9, perSecond: 120 });
const input = {
  sent: 3, processed: 3, dropped: 0, pending: 0, answered: 3, late: 0, noAnswer: 0,
  eventToRaf: { n: 2, p50Ms: 30, p95Ms: 40, maxMs: 40, lastMs: [30, 40] }, listenerToPut: { n: 2, p50Ms: 10, p95Ms: 20, maxMs: 20, lastMs: [10, 20] }, lateVisible: null, eventToRafWithLate: null,
  stages: { n: 2, p50: { s0: 13, s0b: 0.3, toWorker: 0.5, worker: 2.5, fromWorker: 0.4, put: 0.2, raf: 13 }, p95: { s0: 15, s0b: 0.5, toWorker: 0.9, worker: 3, fromWorker: 0.6, put: 0.4, raf: 16 } },
};
const browser: BrowserInfo = { hardwareConcurrency: 8, deviceMemoryGB: 8, userAgent: 'UA/1', screen: { width: 2560, height: 1440 }, devicePixelRatio: 1.5, crossOriginIsolated: true, isSecureContext: true, webglRenderer: 'ANGLE (X)', jsHeap: { usedBytes: 1, totalBytes: 2, limitBytes: 3 } };
const base: BeaconInput = {
  seq: 1, sid: '0123456789ab', reason: 'periodic', pageMs: 1000, visibility: 'visible', browser, bundle: { name: 'fl-Oz', version: '2', engineKind: 'emscripten-fruity-v1' },
  state: 'engine-idle', exit: null, error: null, timeline: { idle: 1000 }, updates: { total: 1, recent: stat(1) }, idles: { total: 1, recent: stat(1) }, input, underruns: null,
};

describe('warnsOf (the last counts line of the engine log)', () => {
  it('is empty when the engine has printed no counts', () => {
    expect(warnsOf([])).toEqual({});
    expect(warnsOf(['fle: warn: GetProcAddress: not modelled, returned 0 (caller 0x1)', '[err] fruity: == stop: x'])).toEqual({});
  });

  it('takes the last counts line, with or without the [err] stream prefix, and orders the names by calls', () => {
    const lines = [
      '[err] fle-warns: {"RegSetValueExW":1}',
      'fle: warn: GetProcAddress: not modelled, returned 0 (caller 0x1)',
      '[err] fle-warns: {"GetProcAddress":2,"RegEnumValueW":5,"RegSetValueExW":2}',
    ];
    expect(warnsOf(lines)).toEqual({ RegEnumValueW: 5, GetProcAddress: 2, RegSetValueExW: 2 });
  });

  it('keeps the top 20 and drops what is not a plain name with a whole count', () => {
    const many: Record<string, number> = {};
    for (let i = 0; i < 25; i++) many[`Api${String(i).padStart(2, '0')}`] = i;
    many['bad name'] = 99;
    many['x'.repeat(70)] = 98;
    many.Frac = 1.5;
    const out = warnsOf([`fle-warns: ${JSON.stringify(many)}`]);
    expect(Object.keys(out)).toHaveLength(WARNS_TOP);
    expect(Object.keys(out)[0]).toBe('Api24');
    expect(Object.values(out)).toEqual(Array.from({ length: WARNS_TOP }, (_, i) => 24 - i));
    expect(warnsValid(out)).toBe(true);
  });

  it('a line that is not JSON gives nothing rather than throwing', () => {
    expect(warnsOf(['fle-warns: {"A":'])).toEqual({});
    expect(warnsOf(['fle-warns: [1,2]'])).toEqual({});
  });
});

describe('the beacon carries warns', () => {
  it('puts warns in every record of this change, {} when there were none', () => {
    expect(buildBeacon(base).warns).toEqual({});
    expect(buildBeacon({ ...base, warns: { GetProcAddress: 3 } }).warns).toEqual({ GetProcAddress: 3 });
  });

  it('the server accepts a record with warns and one from before the field', () => {
    const withWarns = JSON.parse(JSON.stringify(buildBeacon({ ...base, warns: { GetProcAddress: 3 } })));
    const before = JSON.parse(JSON.stringify(buildBeacon(base))) as Record<string, unknown>;
    delete before.warns;
    expect(beaconProblems(withWarns)).toEqual([]);
    expect(beaconProblems(before)).toEqual([]);
  });

  it('the server refuses a warns that is not a map of names to whole counts', () => {
    const b = JSON.parse(JSON.stringify(buildBeacon(base))) as Record<string, unknown>;
    b.warns = { 'C:\\Users\\x': 1 };
    expect(beaconProblems(b).some((p) => p.startsWith('warns:'))).toBe(true);
    b.warns = { GetProcAddress: 1.5 };
    expect(beaconProblems(b).some((p) => p.startsWith('warns:'))).toBe(true);
    b.warns = ['GetProcAddress'];
    expect(beaconProblems(b).some((p) => p.startsWith('warns:'))).toBe(true);
  });
});
