// Decision 0216: scripts/perf-log-summary.mjs reads the beacon's optional `files` block and tells what the people who use the page do with the file manager (counts only).
import { describe, expect, it } from 'vitest';
import { format, summarize } from '../../scripts/perf-log-summary.mjs';

const files = (over: Record<string, unknown> = {}): Record<string, unknown> => ({
  v: 1, opens: 2, pick: { files: 1, folder: 1, zip: 0 }, drop: { files: 0, folders: 1, zips: 1 }, written: 60, skipped: 0, failed: 2, writtenMiB: 70,
  extract: { zips: 1, files: 49, cancelled: 0 }, download: { file: 0, folderZip: 0 }, ops: { mkdir: 1, rename: 0, delete: 0, copy: 0, move: 0 },
  persist: { asked: 1, granted: 0, denied: 0, pending: 1 }, storage: { usedMiB: 70, quotaMiB: 20000, persisted: false },
  engine: { open: 3, save: 1, folder: 0, answered: 3, cancelled: 1, zipUnpacked: 1, savedBack: 1, savedBackFailed: 0 }, fail: { 'zip-crc': 1, name: 1 }, ...over,
});
const rec = (sid: string, seq: number, f: unknown, ua = 'Firefox/159.0'): string => JSON.stringify({
  t: `2026-11-04T01:00:0${seq}.000Z`, remote: 'tailnet',
  perf: { v: 3, sid, seq, reason: seq ? 'periodic' : 'first-idle', bundle: { name: 'fl', version: '1', engineKind: 'emscripten-fruity-v1', standIn: false }, startup: { firstIdleSeconds: 11 }, ui: { updates: { total: 9, recent10s: { n: 9, perSecond: 60, intervalP95Ms: 20 } } },
    audio: { underruns: null }, browser: { hardwareConcurrency: 8, deviceMemoryGB: 8, userAgent: ua, screen: { width: 2176, height: 1224 }, devicePixelRatio: 1.7647, webglRenderer: 'GTX 980' }, error: null, ...(f ? { files: f } : {}) },
});

describe('the files block in the log summary', () => {
  it('a run\'s block is the one in its last record; runs without one are counted as not having used Files', () => {
    const text = [rec('aaaaaaaaaaaa', 0, files({ opens: 1 })), rec('aaaaaaaaaaaa', 1, files({ opens: 2 })), rec('bbbbbbbbbbbb', 0, null), rec('cccccccccccc', 0, files({ opens: 5, fail: { quota: 3 } }), 'Chrome/156.0')].join('\n');
    const s = summarize(text);
    expect(s.files.runs).toBe(3);
    expect(s.files.runsWithFiles).toBe(2);
    expect(s.files.totals.opens).toBe(7);
    expect(s.files.totals.pickFolder).toBe(2);
    expect(s.files.fail).toEqual({ 'zip-crc': 1, name: 1, quota: 3 });
    expect(s.files.byBrowser.map((b) => [b.browser, b.runs, b.opens])).toEqual([['Chrome 156', 1, 5], ['Firefox 159', 1, 2]].sort((a, b) => (b[1] as number) - (a[1] as number) || String(a[0]).localeCompare(String(b[0]))));
  });
  it('the text says what a browser could fail at: the real folder chooser, real folder drops, persist()', () => {
    const out = format(summarize([rec('aaaaaaaaaaaa', 0, files())].join('\n')));
    expect(out).toContain('File manager (decision 0216; real engine; 1 of 1 runs sent a files block');
    expect(out).toContain('Firefox 159: 1 run(s); opened 2; real folder chooser 1; real folder drops 1; persist() asked 1 -> granted 0, denied 0, no answer in 3 s 1');
    expect(out).toContain('failure codes: name 1, zip-crc 1');
  });
  it('without any block the summary says so', () => {
    expect(format(summarize([rec('aaaaaaaaaaaa', 0, null)].join('\n')))).toContain('no run used Files');
  });
});
