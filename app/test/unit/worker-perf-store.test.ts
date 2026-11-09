// Decision 0224: the Worker keeps each measurement beacon as ONE R2 object (worker/perf-store.ts, shared/perf.ts perfObjectKey) and GET /__perf/summary reads them back (worker/perf-summary.ts). Run here against a few lines of
// in-memory R2; the same code runs against wrangler's local R2 in test/worker (npm run test:worker).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PERF_PREFIX, perfLine, perfObjectKey } from '../../shared/perf.ts';
import { storeBeacon } from '../../worker/perf-store.ts';
import { PERF_SUMMARY_MAX_DAYS, PERF_SUMMARY_MAX_OBJECTS, perfSummary, windowDays, type PerfReadBucket } from '../../worker/perf-summary.ts';
import { format, joinStoredObjects, machineOf, summarize } from '../../shared/perf-summary.mjs';
import { readStoredObjects } from '../../scripts/perf-log-summary.mjs';

const NOW = new Date('2026-11-04T08:00:00.000Z');

describe('perfObjectKey: perf/<UTC date>/<sid>-<seq>.json, and nothing the page typed goes into a key', () => {
  it('a beacon with its session token and sequence number', () => {
    expect(perfObjectKey({ sid: '0123456789ab', seq: 7 }, NOW, 'aabbccdd')).toBe('perf/2026-11-04/0123456789ab-7.json');
    expect(perfObjectKey({ sid: 'fedcba987654', seq: 0 }, NOW, 'aabbccdd')).toBe('perf/2026-11-04/fedcba987654-0.json');
  });
  it('the date is the UTC date, whatever the time zone of the machine is', () => {
    expect(perfObjectKey({ sid: '0123456789ab', seq: 1 }, new Date('2026-11-04T23:59:59.999Z'), 'aabbccdd')).toBe('perf/2026-11-04/0123456789ab-1.json');
    expect(perfObjectKey({ sid: '0123456789ab', seq: 2 }, new Date('2026-11-05T00:00:00.000Z'), 'aabbccdd')).toBe('perf/2026-11-05/0123456789ab-2.json');
    expect(perfObjectKey({ sid: '0123456789ab', seq: 3 }, new Date('2026-11-04T15:00:00.000Z'), 'aabbccdd')).toBe('perf/2026-11-04/0123456789ab-3.json');
  });
  it.each([['../../x'], ['0123456789AB'], ['0123456789a'], ['0123456789abc'], [''], ['a/b/c/d/e/f'], [12], [null], [undefined], [{ a: 1 }]])('a sid that is not 12 lower-case hex digits (%j) becomes nosid-<random>', (sid) => {
    expect(perfObjectKey({ sid, seq: 1 }, NOW, 'aabbccdd')).toBe('perf/2026-11-04/nosid-aabbccdd-1.json');
  });
  it.each([[-1], [1.5], ['3'], [null], [undefined], [1e9], [Number.NaN]])('a seq that is not a whole number 0..999999999 (%j) is 0', (seq) => {
    expect(perfObjectKey({ sid: '0123456789ab', seq }, NOW, 'aabbccdd')).toBe('perf/2026-11-04/0123456789ab-0.json');
  });
  it('a spare that is not 8 hex digits is replaced, so a key never carries text from outside', () => {
    expect(perfObjectKey({}, NOW, '../..')).toBe('perf/2026-11-04/nosid-00000000-0.json');
  });
  it('every key is under perf/ and has no more than three path parts', () => {
    for (const sid of ['../../etc', 'x/y', '%2e%2e', '0123456789ab']) {
      const k = perfObjectKey({ sid, seq: 1 }, NOW, 'aabbccdd');
      expect(k.startsWith(PERF_PREFIX)).toBe(true);
      expect(k.split('/').length).toBe(3);
      expect(k).toMatch(/^perf\/\d{4}-\d\d-\d\d\/[0-9a-z-]+\.json$/);
    }
  });
});

describe('storeBeacon: one object per beacon, the body is the JSONL line of the dev server with remote "workers"', () => {
  afterEach(() => { vi.restoreAllMocks(); });
  const beacon = { v: 3, seq: 4, sid: '0123456789ab', reason: 'periodic', startup: { firstIdleSeconds: 6.1 } };

  it('puts the line (time, "workers", the beacon) at the key with the JSON content type', async () => {
    const puts: { key: string; value: string; options: unknown }[] = [];
    const ok = await storeBeacon({ put: async (key, value, options) => { puts.push({ key, value, options }); } }, beacon, NOW);
    expect(ok).toBe(true);
    expect(puts).toHaveLength(1);
    expect(puts[0]?.key).toBe('perf/2026-11-04/0123456789ab-4.json');
    expect(puts[0]?.options).toEqual({ httpMetadata: { contentType: 'application/json' } });
    expect(puts[0]?.value).toBe(perfLine(beacon, NOW, 'workers'));
    expect(puts[0]?.value.endsWith('\n')).toBe(true);
    expect(puts[0]?.value.trim().split('\n')).toHaveLength(1);
    expect(JSON.parse(puts[0]?.value ?? '')).toEqual({ t: '2026-11-04T08:00:00.000Z', remote: 'workers', perf: beacon });
  });
  it('keeps nothing but the time, the word "workers" and the beacon', async () => {
    let value = '';
    await storeBeacon({ put: async (_k, v) => { value = v; } }, beacon, NOW);
    expect(Object.keys(JSON.parse(value))).toEqual(['t', 'remote', 'perf']);
  });
  it('no binding: false, nothing is thrown, and nothing is logged as a failure (not having a bucket is a configuration, not an error)', async () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    expect(await storeBeacon(undefined, beacon, NOW)).toBe(false);
    expect(err).not.toHaveBeenCalled();
  });
  it('a write that fails: false, no exception, the failure is in the log without the beacon\'s content', async () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const ok = await storeBeacon({ put: async () => { throw new Error('R2 is down (10043)'); } }, { ...beacon, secret: 'do-not-log-me' }, NOW);
    expect(ok).toBe(false);
    expect(err).toHaveBeenCalledTimes(1);
    const line = String(err.mock.calls[0]?.[0]);
    expect(line).toContain('0123456789ab-4.json');
    expect(line).toContain('R2 is down');
    expect(line).not.toContain('do-not-log-me');
  });
  it('a write that rejects with something that is not an Error is also false', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
    expect(await storeBeacon({ put: () => Promise.reject('plain text') }, beacon, NOW)).toBe(false);
  });
});

// ---- a few lines of R2 ----
interface Stored { text: string; uploaded: Date }
function fakeR2(objects: Map<string, Stored>, trace: { listPrefixes: string[]; gets: number; maxInFlight: number; inFlight: number; pageSize: number } = { listPrefixes: [], gets: 0, maxInFlight: 0, inFlight: 0, pageSize: 1000 }): { bucket: PerfReadBucket; trace: typeof trace } {
  const bucket: PerfReadBucket = {
    async list({ prefix, cursor }) {
      trace.listPrefixes.push(prefix);
      const keys = [...objects.keys()].filter((k) => k.startsWith(prefix)).sort();
      const from = cursor ? Number(cursor) : 0;
      const page = keys.slice(from, from + trace.pageSize);
      const more = from + trace.pageSize < keys.length;
      return { objects: page.map((key) => ({ key, uploaded: (objects.get(key) as Stored).uploaded })), truncated: more, ...(more ? { cursor: String(from + trace.pageSize) } : {}) };
    },
    async get(key) {
      trace.gets++;
      trace.inFlight++;
      trace.maxInFlight = Math.max(trace.maxInFlight, trace.inFlight);
      await new Promise((r) => setTimeout(r, 1));
      trace.inFlight--;
      const o = objects.get(key);
      return o ? { text: async () => o.text } : null;
    },
  };
  return { bucket, trace };
}
const rec = (t: string, sid: string, seq: number, extra: Record<string, unknown> = {}): { key: string; line: string } => ({
  key: `perf/${t.slice(0, 10)}/${sid}-${seq}.json`,
  line: perfLine({ v: 3, seq, sid, reason: 'periodic', bundle: { name: 'fl', version: '1', engineKind: 'emscripten-fruity-v1', standIn: false }, startup: { firstIdleSeconds: 6.5 }, browser: { userAgent: 'Mozilla/5.0 Chrome/156.0', hardwareConcurrency: 8 }, ...extra }, new Date(t), 'workers'),
});
function put(m: Map<string, Stored>, t: string, sid: string, seq: number, extra: Record<string, unknown> = {}, uploaded = new Date(t)): void {
  const r = rec(t, sid, seq, extra);
  m.set(r.key, { text: r.line, uploaded });
}

describe('GET /__perf/summary: the stored beacons, summarised with the library of the command line', () => {
  it('lists the UTC days of the window, newest first', () => {
    expect(windowDays(NOW, 3)).toEqual(['2026-11-04', '2026-11-03', '2026-11-02']);
    expect(windowDays(new Date('2026-03-01T00:30:00Z'), 2)).toEqual(['2026-03-01', '2026-02-28']);
  });
  it('lists one prefix per day (default 2 days), reads each object once, and summarises: the records are "workers" and the first idle is the page\'s', async () => {
    const m = new Map<string, Stored>();
    put(m, '2026-11-04T07:00:00Z', '0123456789ab', 0);
    put(m, '2026-11-04T07:00:30Z', '0123456789ab', 1);
    put(m, '2026-11-03T23:59:59Z', 'abcdef012345', 0, { startup: { firstIdleSeconds: 9.5 } });
    put(m, '2026-11-02T10:00:00Z', 'ffffffffffff', 0); // outside the 2-day window
    const { bucket, trace } = fakeR2(m);
    const a = await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams(), NOW);
    expect(a.status).toBe(200);
    expect(trace.listPrefixes).toEqual(['perf/2026-11-04/', 'perf/2026-11-03/']);
    expect(trace.gets).toBe(3);
    expect(a.body).toContain('範囲: UTC 2026-11-03 〜 2026-11-04(2 日)。範囲内のオブジェクト 3 件、読んだ 3 件');
    expect(a.body).toContain('3 records (3 real engine, 0 stand-in)');
    expect(a.body).toContain('P1 first idle (target <= 10 s), real engine: 9.5, 6.5, 6.5 s');
    expect(a.body).toContain('workers (through Cloudflare Workers, behind Access)');
    expect(a.body).not.toContain('truncated');
  });
  it('days=N widens the window; days outside 1..180, not a number, or rows / machine that make no sense: 400 and nothing is read', async () => {
    const m = new Map<string, Stored>();
    put(m, '2026-10-30T07:00:00Z', '0123456789ab', 0);
    const { bucket, trace } = fakeR2(m);
    const a = await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams('days=7'), NOW);
    expect(a.status).toBe(200);
    expect(a.body).toContain('範囲内のオブジェクト 1 件、読んだ 1 件');
    expect(trace.listPrefixes).toHaveLength(7);
    for (const q of ['days=0', `days=${PERF_SUMMARY_MAX_DAYS + 1}`, 'days=abc', 'days=-1', 'days=1.5', 'rows=-1', 'rows=x', 'machine=bogus']) {
      const before = trace.listPrefixes.length;
      expect((await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams(q), NOW)).status, q).toBe(400);
      expect(trace.listPrefixes.length, q).toBe(before);
    }
  });
  it('without a PERF_BUCKET binding: 503 that says so', async () => {
    const a = await perfSummary({}, new URLSearchParams(), NOW);
    expect(a.status).toBe(503);
    expect(a.body).toContain('PERF_BUCKET');
  });
  it('a listing of more than 1,000 keys is followed page by page', async () => {
    const m = new Map<string, Stored>();
    for (let i = 0; i < 25; i++) put(m, '2026-11-04T07:00:00Z', `${String(i).padStart(12, '0')}`, 0);
    const f = fakeR2(m);
    f.trace.pageSize = 10;
    const a = await perfSummary({ PERF_BUCKET: f.bucket }, new URLSearchParams('days=1'), NOW);
    expect(a.body).toContain('範囲内のオブジェクト 25 件、読んだ 25 件');
    expect(f.trace.gets).toBe(25);
  });
  it(`reads at most ${PERF_SUMMARY_MAX_OBJECTS} objects, the newest by upload time, never more than 6 at once, and says that the older ones were not counted`, async () => {
    const m = new Map<string, Stored>();
    const N = PERF_SUMMARY_MAX_OBJECTS + 40;
    for (let i = 0; i < N; i++) put(m, '2026-11-04T07:00:00Z', `${String(i).padStart(12, '0')}`, 0, {}, new Date(Date.UTC(2026, 10, 4, 0, 0, 0) + i * 1000));
    const f = fakeR2(m);
    const a = await perfSummary({ PERF_BUCKET: f.bucket }, new URLSearchParams('days=1'), NOW);
    expect(f.trace.gets).toBe(PERF_SUMMARY_MAX_OBJECTS);
    expect(f.trace.maxInFlight).toBeLessThanOrEqual(6);
    expect(f.trace.maxInFlight).toBeGreaterThan(1);
    expect(a.body).toContain(`範囲内のオブジェクト ${N} 件、読んだ ${PERF_SUMMARY_MAX_OBJECTS} 件(多すぎるため、アップロードの新しい ${PERF_SUMMARY_MAX_OBJECTS} 件だけ`);
    expect(a.body).toContain(`${PERF_SUMMARY_MAX_OBJECTS} records`);
    // the 40 oldest were left out
    const json = JSON.parse((await perfSummary({ PERF_BUCKET: f.bucket }, new URLSearchParams('days=1&format=json'), NOW)).body);
    const sids = new Set(json.summary.rows.map((r: { sid: string }) => r.sid));
    expect(sids.has('000000000000')).toBe(false);
    expect(sids.has('000000000039')).toBe(false);
    expect(sids.has('000000000040')).toBe(true);
    expect(sids.has(String(N - 1).padStart(12, '0'))).toBe(true);
  });
  it('an object that vanishes between the listing and the read is counted, not fatal', async () => {
    const m = new Map<string, Stored>();
    put(m, '2026-11-04T07:00:00Z', '0123456789ab', 0);
    put(m, '2026-11-04T07:00:01Z', 'abcdef012345', 0);
    const f = fakeR2(m);
    const get = f.bucket.get;
    f.bucket.get = async (k) => (k.includes('abcdef') ? null : get(k));
    const a = await perfSummary({ PERF_BUCKET: f.bucket }, new URLSearchParams('days=1'), NOW);
    expect(a.status).toBe(200);
    expect(a.body).toContain('読んだ 1 件、読む前に消えていた 1 件');
  });
  it('rows limits the one-line-per-record part to the last N; rows=all shows all; the tables count every record', async () => {
    const m = new Map<string, Stored>();
    for (let i = 0; i < 40; i++) put(m, `2026-11-04T07:00:${String(i).padStart(2, '0')}Z`, '0123456789ab', i);
    const { bucket } = fakeR2(m);
    const dflt = (await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams('days=1'), NOW)).body;
    expect(dflt).toContain('(the last 30 of 40 record lines; the tables below count all records)');
    expect(dflt).toContain('40 records (40 real engine, 0 stand-in)');
    expect((dflt.match(/ workers {2}#\d+ /g) ?? []).length).toBe(30);
    expect(dflt).toContain('#39 periodic');
    expect(dflt).not.toContain('#9 periodic');
    const all = (await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams('days=1&rows=all'), NOW)).body;
    expect((all.match(/ workers {2}#\d+ /g) ?? []).length).toBe(40);
    expect(all).not.toContain('record lines;');
    const none = (await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams('days=1&rows=0'), NOW)).body;
    expect((none.match(/ workers {2}#\d+ /g) ?? []).length).toBe(0);
  });
  it('machine=workers keeps the groups of the Worker; format=json is the library\'s summary plus the window', async () => {
    const m = new Map<string, Stored>();
    put(m, '2026-11-04T07:00:00Z', '0123456789ab', 0);
    const { bucket } = fakeR2(m);
    const t = await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams('days=1&machine=workers'), NOW);
    expect(t.body).toMatch(/By machine \/ browser \/ screen rate .*1 group\(s\)/);
    const none = await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams('days=1&machine=tailnet'), NOW);
    expect(none.body).toContain('no real-engine run from machine "tailnet"');
    const j = await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams('days=1&format=json'), NOW);
    expect(j.json).toBe(true);
    const o = JSON.parse(j.body);
    expect(o.window).toEqual({ dates: ['2026-11-04'], objects: 1, read: 1, missing: 0 });
    expect(o.summary.records).toBe(1);
    expect(o.summary.rows[0].remote).toBe('workers');
  });
  it('a stored object that is not a record is reported as an unreadable line, not hidden', async () => {
    const m = new Map<string, Stored>();
    put(m, '2026-11-04T07:00:00Z', '0123456789ab', 0);
    m.set('perf/2026-11-04/broken-0.json', { text: '{"t": broken\n', uploaded: NOW });
    const { bucket } = fakeR2(m);
    const a = await perfSummary({ PERF_BUCKET: bucket }, new URLSearchParams('days=1'), NOW);
    expect(a.body).toMatch(/1 records .*unreadable lines: 1/);
  });
});

describe('joinStoredObjects / --dir: the objects of R2, put back into one log', () => {
  const line = (t: string, seq: number): string => perfLine({ v: 3, seq, sid: '0123456789ab', reason: 'periodic' }, new Date(t), 'workers');
  it('orders by the time of the record and then seq, whatever the order of the objects', () => {
    const joined = joinStoredObjects([line('2026-11-04T08:00:02Z', 2), line('2026-11-04T08:00:00Z', 0), line('2026-11-04T08:00:01Z', 1)]);
    expect(joined.trim().split('\n').map((l) => JSON.parse(l).perf.seq)).toEqual([0, 1, 2]);
    const same = joinStoredObjects([line('2026-11-04T08:00:00Z', 5), line('2026-11-04T08:00:00Z', 3)]);
    expect(same.trim().split('\n').map((l) => JSON.parse(l).perf.seq)).toEqual([3, 5]);
  });
  it('keeps an unreadable object (it is shown as such) and drops an empty one', () => {
    const joined = joinStoredObjects([line('2026-11-04T08:00:00Z', 0), '{"t": broken', '', '\n']);
    expect(joined.trim().split('\n')).toHaveLength(2);
    expect(summarize(joined).unreadableLines).toHaveLength(1);
  });
  it('the command line reads a directory of *.json (any depth), and the result equals the summary of the same records as one log', () => {
    const dir = fs.mkdtempSync(path.join(process.env.APP_WORK ?? os.tmpdir(), 'perf-dir-'));
    try {
      const recs = [0, 1, 2].map((i) => line(`2026-11-04T08:00:0${i}Z`, i));
      fs.mkdirSync(path.join(dir, '2026-11-04'), { recursive: true });
      recs.forEach((r, i) => fs.writeFileSync(path.join(dir, '2026-11-04', `0123456789ab-${i}.json`), r));
      fs.writeFileSync(path.join(dir, 'note.txt'), 'not a beacon'); // not *.json: ignored
      expect(readStoredObjects(dir)).toHaveLength(3);
      const viaDir = format(summarize(joinStoredObjects(readStoredObjects(dir))));
      const viaLog = format(summarize(recs.join('')));
      expect(viaDir).toBe(viaLog);
      const cli = fileURLToPath(new URL('../../scripts/perf-log-summary.mjs', import.meta.url));
      const r = spawnSync(process.execPath, [cli, '--dir', dir], { encoding: 'utf8' });
      expect(r.status).toBe(0);
      expect(r.stdout).toContain('3 records (0 real engine, 0 stand-in)'); // these lines carry no bundle block, so none is counted as the real engine
      const bad = spawnSync(process.execPath, [cli, '--dir', path.join(dir, 'nope')], { encoding: 'utf8' });
      expect(bad.status).toBe(2);
      expect(spawnSync(process.execPath, [cli, '--dir'], { encoding: 'utf8' }).status).toBe(2);
      expect(spawnSync(process.execPath, [cli], { encoding: 'utf8' }).status).toBe(2);
      const file = path.join(dir, 'log.jsonl');
      fs.writeFileSync(file, recs.join(''));
      expect(spawnSync(process.execPath, [cli, file], { encoding: 'utf8' }).stdout).toContain('3 records');
    } finally { fs.rmSync(dir, { recursive: true, force: true }); }
  });
  it('machineOf names the Worker\'s records; an automated browser through the Worker is marked', () => {
    expect(machineOf('workers', false)).toBe('workers (through Cloudflare Workers, behind Access)');
    expect(machineOf('workers', true)).toContain('automated browser');
    expect(machineOf('tailnet', false)).toBe('tailnet (the owner via the tailnet)');
  });
});
