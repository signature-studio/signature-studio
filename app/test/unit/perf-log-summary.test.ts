// Decision 0152: scripts/perf-log-summary.mjs reads the beacon log; a stand-in's records are marked and do not count towards the real engine's verdict lines.
import { describe, expect, it } from 'vitest';
import { NO_STOP_MESSAGE, format, hzClass, machineOf, summarize } from '../../scripts/perf-log-summary.mjs';

const rec = (over: Record<string, unknown>, standIn = false, kind = 'emscripten-fruity-v1'): string => JSON.stringify({
  t: '2026-11-03T01:00:00.000Z', remote: 'tailnet',
  perf: { v: 1, seq: 0, reason: 'first-idle', bundle: { name: standIn ? 'standin-x' : 'fl', version: '1', engineKind: kind, standIn }, startup: { firstIdleSeconds: 38.5, targetSeconds: 10 },
    ui: { updates: { total: 9, recent10s: { n: 9, intervalMedianMs: 3, intervalP95Ms: 3000, perSecond: 0.9 } } }, input: { n: 4, p95Ms: 1900 }, audio: { underruns: null },
    browser: { hardwareConcurrency: 12, deviceMemoryGB: 8, userAgent: 'UA', screen: { width: 2560, height: 1440 }, devicePixelRatio: 1.25, webglRenderer: 'GPU X' }, error: null, ...over },
});

describe('perf-log-summary', () => {
  it('reads the records, keeps the real engine\'s numbers apart from the stand-in\'s, and tolerates a damaged line', () => {
    const text = [rec({}), rec({ seq: 1, reason: 'periodic', startup: { firstIdleSeconds: 38.5 }, ui: { updates: { recent10s: { perSecond: 0.7, intervalP95Ms: 3300 } } }, input: { n: 5, p95Ms: 1500 } }), rec({ startup: { firstIdleSeconds: 0.2 } }, true, 'unknown-engine-kind-v0'), '{"t": broken', ''].join('\n');
    const s = summarize(text);
    expect(s.records).toBe(3);
    expect(s.real).toBe(2);
    expect(s.standIn).toBe(1);
    expect(s.unreadableLines).toEqual([4]);
    expect(s.p1.seconds).toEqual([38.5, 38.5]);
    expect(s.p1.bestSeconds).toBe(38.5);
    expect(s.p2.updatesPerSecond).toEqual([0.9, 0.7]);
    expect(s.p2.bestUpdatesPerSecond).toBe(0.9);
    // these are version 1 records: their input number is the old listener -> put one, in its own column, and is not the P2 verdict
    expect(s.p2.inputP95Ms).toEqual([]);
    expect(s.p2.bestInputP95Ms).toBeNull();
    expect(s.p2.listenerToPutP95Ms).toEqual([1900, 1500]);
    expect(s.p3.underruns).toEqual([]);
    expect(s.rows[0]).toMatchObject({ remote: 'tailnet', cores: 12, memGB: 8, gpu: 'GPU X', screen: '2560x1440@1.25' });
  });
  it('prints the verdict lines with the targets, marks the stand-in and says "not measured" for the audio', () => {
    const out = format(summarize([rec({}), rec({ startup: { firstIdleSeconds: 0.2 } }, true, 'unknown-engine-kind-v0')].join('\n')));
    expect(out).toContain('STAND-IN unknown-engine-kind-v0 first idle 0.2 s');
    expect(out).toContain('P1 first idle (target <= 10 s), real engine: 38.5 s');
    expect(out).toContain('P2 updates per second (target >= 120), real engine: 0.9');
    expect(out).toContain('input event->rAF p95 (version 3 records; target <= 17 ms): no version 3 record');
    expect(out).toContain('input listener->put p95 (the old definition of versions 1 and 2, and version 3\'s second number; 10-25 ms below what a person feels, decision 0194; NOT compared with the target): 1900 ms');
    expect(out).toContain('input listener->put p95 1900 ms (n=4; the old definition, v1: not event->rAF)');
    expect(out).toContain('P3 audio under-runs: not measured');
    expect(out).toContain('2 records (1 real engine, 1 stand-in)');
  });
  it('an empty log: no records, no number', () => {
    const s = summarize('');
    expect(s.records).toBe(0);
    expect(s.p1.bestSeconds).toBeNull();
    expect(format(s)).toContain('real engine: no record');
  });
});


// ---- decision 0192: by machine / browser / screen rate ----

const raf = (hz: number, o: Record<string, unknown> = {}) => ({ n: 600, perSecond: hz, medianMs: 1000 / hz, p95Ms: 1000 / hz, p99Ms: 1000 / hz, maxMs: 1000 / hz, hz, peakHz: hz, slow: 6, over12p5: hz < 80 ? 600 : 0, bins: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0], ...o });
const disp = (hz: number, fresh = hz, o: Record<string, unknown> = {}) => ({
  rafSupported: true, runningMs: 60000, visibility: { visibleMs: 60000, hiddenMs: 0, changes: 0, visibleNow: true },
  raf: { recent10s: raf(hz), sinceIdle: raf(hz), total: raf(hz), binEdgesMs: [] },
  canvas: { received: 100, drawn: 100, recent10s: {}, sinceIdle: { ticks: 600, freshRatio: fresh / hz, freshPerSecond: fresh, drawsPerSecond: fresh, drawMs: { p95: 1.2 } }, total: {}, workerSent: null, workerOmitted: null },
  longTasks: { supported: true, sinceIdle: 0 }, events: { supported: true, slow: 0 }, ...o,
});
const UA_WIN = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36';
const UA_HL = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/156.0.0.0 Safari/537.36';
const rec2 = (remote: string, sid: string, seq: number, reason: string, ua: string, hz: number, idle: number, extra: Record<string, unknown> = {}, gpu = 'ANGLE (NVIDIA)'): string => JSON.stringify({
  t: `2026-11-03T0${seq}:00:00.000Z`, remote,
  perf: { v: 2, sid, seq, reason, bundle: { name: 'fl', version: '1', engineKind: 'emscripten-fruity-v1', standIn: false }, startup: { firstIdleSeconds: idle, targetSeconds: 10 },
    ui: { updates: { total: 9, recent10s: { n: 9, intervalMedianMs: 8, intervalP95Ms: 9, perSecond: hz } } }, input: { n: 4, p95Ms: 30 }, audio: { underruns: null },
    browser: { hardwareConcurrency: 12, deviceMemoryGB: 8, userAgent: ua, screen: { width: 2560, height: 1440 }, devicePixelRatio: 1, webglRenderer: gpu }, error: null,
    client: { browser: ua.includes('HeadlessChrome') ? { name: 'Chrome', major: 156, os: 'Linux', headless: true } : { name: 'Chrome', major: 156, os: 'Windows', headless: false }, page: { hostKind: 'ts.net' } },
    display: disp(hz), hz: { engineHz: 60, param: null, fit: 'engine-slower' }, ...extra },
});

describe('perf-log-summary: by machine, browser and screen rate (decision 0192)', () => {
  const text = [
    rec2('tailnet', 'aaaaaaaaaaaa', 0, 'first-idle', UA_WIN, 120, 7.5),
    rec2('tailnet', 'aaaaaaaaaaaa', 1, 'periodic', UA_WIN, 120, 7.5),
    rec2('tailnet', 'bbbbbbbbbbbb', 0, 'first-idle', UA_WIN, 60, 9.1),
    rec2('loopback', 'cccccccccccc', 0, 'first-idle', UA_HL, 60, 6.7, {}, 'ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device))'),
    rec2('loopback', 'cccccccccccc', 1, 'periodic', UA_HL, 60, 6.7, {}, 'ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device))'),
    rec2('loopback', 'dddddddddddd', 0, 'first-idle', UA_HL, 60, 6.9, {}, 'ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device))'),
  ].join('\n');
  const s = summarize(text);
  it('one run per page load (the session token), groups by machine, browser and the screen\'s rate class', () => {
    expect(s.runs).toHaveLength(4);
    expect(s.groups.map((g) => [g.machine, g.browser, g.screen, g.runs])).toEqual([
      ['loopback, headless (this machine, automated)', 'Chrome 156', '60 Hz class', 2],
      ['tailnet (the owner via the tailnet)', 'Chrome 156', '120 Hz class', 1],
      ['tailnet (the owner via the tailnet)', 'Chrome 156', '60 Hz class', 1],
    ]);
  });
  it('per group: first idle min / median / max and how many runs met 10 s; the P2 numbers are those of the run\'s LAST record', () => {
    const owner120 = s.groups.find((g) => g.screen === '120 Hz class')!;
    expect(owner120.p1).toMatchObject({ minSeconds: 7.5, medianSeconds: 7.5, maxSeconds: 7.5, metRuns: 1 });
    expect(owner120.p2).toMatchObject({ rafHzMedian: 120, canvasFreshPerSecondMedian: 120, metRuns: 1 });
    const owner60 = s.groups.find((g) => g.machine.startsWith('tailnet') && g.screen === '60 Hz class')!;
    expect(owner60.p1.metRuns).toBe(1);
    expect(owner60.p2.metRuns).toBe(0);
    const here = s.groups.find((g) => g.machine.startsWith('loopback'))!;
    expect(here.p1).toMatchObject({ minSeconds: 6.7, maxSeconds: 6.9 });
  });
  it('the owner\'s machine (tailnet) and this machine (loopback, headless) are never in one group; slow frames are a share of the intervals', () => {
    expect(new Set(s.groups.map((g) => g.machine)).size).toBe(2);
    expect(s.groups[0]!.p2.slowFrameRatioMedian).toBeCloseTo(6 / 600, 6);
  });
  it('a loopback record with a real GPU and no "Headless" is NOT called automated: an older server behind tailscale serve would look like this', () => {
    const r = summarize(rec2('loopback', 'eeeeeeeeeeee', 0, 'first-idle', UA_WIN, 120, 8));
    expect(r.groups[0]!.machine).toBe('loopback, not headless (this machine, or an older server behind tailscale serve)');
    expect(machineOf('lan', false)).toBe('lan');
    expect(machineOf('other', true)).toBe('other');
  });
  it('a software GPU marks the run as automated even when the user agent says nothing (a Chrome started with another UA); a real GPU does not', () => {
    const sw = summarize(rec2('loopback', 'ffffffffffff', 0, 'first-idle', UA_WIN, 60, 7, {}, 'ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device))'));
    expect(sw.groups[0]!.machine).toBe('loopback, headless (this machine, automated)');
    for (const gpu of ['llvmpipe (LLVM 15.0.7, 256 bits)', 'Software Rasterizer']) {
      expect(summarize(rec2('loopback', 'ffffffffffff', 0, 'first-idle', UA_WIN, 60, 7, {}, gpu)).groups[0]!.machine, gpu).toBe('loopback, headless (this machine, automated)');
    }
    expect(summarize(rec2('loopback', 'ffffffffffff', 0, 'first-idle', UA_WIN, 60, 7, {}, 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4070)')).groups[0]!.machine).toContain('not headless');
  });
  it('the text output has the table, the machines, the screen class and the targets; --machine filters', () => {
    const out = format(s);
    expect(out).toContain('By machine / browser / screen rate');
    expect(out).toContain('tailnet (the owner via the tailnet) | Chrome 156 | screen: 120 Hz class | 1 run(s)');
    expect(out).toContain('loopback, headless (this machine, automated) | Chrome 156 | screen: 60 Hz class | 2 run(s)');
    expect(out).toContain('(target <= 10 s met in 1/1 runs)');
    expect(out).toContain('canvas-updating screen frames/s median 120 (target >= 120, met in 1/1 runs)');
    expect(out).toContain('rAF 120 Hz (120 class');
    expect(out).toContain('?hz=- engine-slower');
    const only = format(s, { machine: 'tailnet' });
    expect(only).toContain('2 group(s)');
    expect(only).not.toContain('loopback, headless (this machine, automated) | Chrome');
    expect(format(s, { machine: 'lan' })).toContain('no real-engine run from machine "lan"');
  });
  it('a record of beacon version 1 has no screen measurement: its group says so and its columns are dashes; a stand-in is in no group', () => {
    const old = summarize([rec({}), rec({}, true, 'unknown-engine-kind-v0')].join('\n'));
    expect(old.groups).toHaveLength(1);
    expect(old.groups[0]!.screen).toBe('no screen measurement');
    expect(old.groups[0]!.p2.rafHzMedian).toBeNull();
    expect(format(old)).toContain('rAF Hz median -');
  });
  it('version 1 records without a token are cut into runs at seq 0 (the same browser twice)', () => {
    const o = (seq: number) => rec({ seq, reason: seq === 0 ? 'first-idle' : 'periodic' });
    expect(summarize([o(0), o(1), o(0), o(1), o(2)].join('\n')).runs.map((r) => r.records)).toEqual([2, 3]);
  });
  it('two page loads of the same browser at the same time are told apart by the session token (their records interleave in the log)', () => {
    const t = [
      rec2('tailnet', '111111111111', 0, 'first-idle', UA_WIN, 120, 7.5), rec2('tailnet', '222222222222', 0, 'first-idle', UA_WIN, 60, 9.5),
      rec2('tailnet', '111111111111', 1, 'periodic', UA_WIN, 120, 7.5), rec2('tailnet', '222222222222', 1, 'periodic', UA_WIN, 60, 9.5),
    ].join('\n');
    const r = summarize(t);
    expect(r.runs.map((x) => [x.records, x.firstIdleSeconds])).toEqual([[2, 7.5], [2, 9.5]]);
  });
  it('a run of the stand-in is in no group (it is a picture replay: its screen numbers are not FL Studio\'s), but it is in the runs and in the record count', () => {
    const standIn = JSON.parse(rec2('tailnet', '333333333333', 0, 'exit', UA_WIN, 60, 0.2));
    standIn.perf.bundle = { name: 'standin-animate', version: '1', engineKind: 'unknown-engine-kind-v0', standIn: true };
    const r = summarize([rec2('tailnet', '111111111111', 0, 'first-idle', UA_WIN, 120, 7.5), JSON.stringify(standIn)].join('\n'));
    expect(r.runs).toHaveLength(2);
    expect(r.runs.find((x) => x.standIn)).toBeTruthy();
    expect(r.groups).toHaveLength(1);
    expect(r.groups[0]!.runs).toBe(1);
    expect(r.standIn).toBe(1);
  });
  it('hzClass: the same bands as the page', () => {
    expect(hzClass(59.9)).toBe(60);
    expect(hzClass(119.6)).toBe(120);
    expect(hzClass(143)).toBe(144);
    expect(hzClass(30)).toBeNull();
    expect(hzClass(110)).toBeNull();
  });
});


// ---- decision 0202: input -> update from the event's timeStamp to the next animation frame ----

const stat = (n: number, p50: number, p95: number) => ({ n, p50Ms: p50, p95Ms: p95, maxMs: p95, lastMs: [] as number[] });
const stages = { s0: 13, s0b: 0.3, toWorker: 0.5, worker: 2.7, fromWorker: 0.4, put: 0.2, raf: 13 };
const rec3 = (sid: string, seq: number, e2r: { p50: number; p95: number } | null, lp: { p50: number; p95: number }, ua = UA_HL): string => {
  const r = JSON.parse(rec2('loopback', sid, seq, seq === 0 ? 'first-idle' : 'periodic', ua, 60, 6.5, {}, 'ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device))'));
  r.perf.v = 3;
  r.perf.input = { sent: 30, processed: 30, dropped: 0, pending: 0, answered: 25, late: 1, noAnswer: 4, eventToRaf: e2r ? stat(20, e2r.p50, e2r.p95) : stat(0, null as never, null as never), listenerToPut: stat(20, lp.p50, lp.p95), stages: e2r ? { n: 20, p50: stages, p95: stages } : null, targetP95Ms: 17 };
  return JSON.stringify(r);
};

describe('perf-log-summary: input -> update has two definitions and the log says which (decision 0202)', () => {
  it('a version 3 record\'s P2 verdict is its event->rAF number; its listener->put number is kept apart', () => {
    const s = summarize(rec3('aaaaaaaaaaaa', 0, { p50: 22, p95: 34 }, { p50: 4, p95: 9 }));
    expect(s.p2.inputP95Ms).toEqual([34]);
    expect(s.p2.bestInputP95Ms).toBe(34);
    expect(s.p2.listenerToPutP95Ms).toEqual([9]);
    expect(s.rows[0]).toMatchObject({ inputP95Ms: 34, inputP50Ms: 22, listenerToPutP95Ms: 9, inputDefinition: 'event->rAF', inputN: 20 });
    expect(s.rows[0]!.inputStages).toEqual(stages);
  });
  it('decision 0205: the counts of the inputs that had no answer frame are in the row and in the printed line, next to the p95 they are not part of', () => {
    const s = summarize(rec3('aaaaaaaaaaaa', 0, { p50: 22, p95: 34 }, { p50: 4, p95: 9 }));
    expect(s.rows[0]!.inputCounts).toEqual({ processed: 30, answered: 25, late: 1, noAnswer: 4 });
    expect(format(s)).toContain('of 30 processed 25 have a value, 5 (17%) have none (1 late; 4 with no frame soon enough), not in the p95');
  });
  it('decision 0205: with ?dedupe=1 the verdict is the p95 over the answered inputs and the late visible changes; the answered-only p95 stands beside it and the print says the late ones are in it', () => {
    const r = JSON.parse(rec3('aaaaaaaaaaaa', 0, { p50: 22, p95: 14 }, { p50: 4, p95: 9 }));
    r.perf.input.lateVisible = stat(5, 60, 70);
    r.perf.input.eventToRafWithLate = stat(20, 30, 62);
    const s = summarize(JSON.stringify(r));
    expect(s.rows[0]).toMatchObject({ inputP95Ms: 62, inputP50Ms: 30, inputBasis: 'answered+late', inputAnsweredP95Ms: 14, inputAnsweredP50Ms: 22, inputLate: { n: 5, p50Ms: 60, p95Ms: 70 } });
    expect(s.p2.inputP95Ms).toEqual([62]);
    expect(format(s)).toContain('p95 over the answered inputs and the late visible changes, answered only p95 14 ms p50 22 ms (n=20)');
    expect(format(s)).toContain('(1 late, 60 / 70 ms p50 / p95 after the event; 4 with no frame soon enough), the late ones are in the p95');
  });
  it('without dedupe (lateVisible and eventToRafWithLate null) the verdict is the answered inputs\' p95 and the late ones are said not to be in it', () => {
    const r = JSON.parse(rec3('aaaaaaaaaaaa', 0, { p50: 22, p95: 14 }, { p50: 4, p95: 9 }));
    r.perf.input.lateVisible = null;
    r.perf.input.eventToRafWithLate = null;
    const s = summarize(JSON.stringify(r));
    expect(s.rows[0]).toMatchObject({ inputP95Ms: 14, inputBasis: 'answered', inputLate: null });
    expect(format(s)).toContain('have none (1 late; 4 with no frame soon enough), not in the p95');
  });
  it('a version 2 record\'s input number stays the old definition: it is NOT in the P2 verdict list and not compared with 17 ms, even when it is small', () => {
    const s = summarize(rec2('tailnet', 'bbbbbbbbbbbb', 0, 'first-idle', UA_WIN, 120, 7.5, { input: { n: 20, p50Ms: 3, p95Ms: 6 } }));
    expect(s.rows[0]).toMatchObject({ v: 2, inputP95Ms: null, inputDefinition: 'listener->put', listenerToPutP95Ms: 6 });
    expect(s.p2.inputP95Ms).toEqual([]);
    expect(s.groups[0]!.p2.inputP95MsMin).toBeNull();
    expect(s.groups[0]!.p2.listenerToPutP95MsMin).toBe(6);
    expect(s.groups[0]!.p2).toMatchObject({ inputRuns: 0, inputMetRuns: 0 });
  });
  it('version 2 and version 3 records in one log: each run is judged by its own definition, in separate columns', () => {
    const s = summarize([
      rec2('tailnet', 'bbbbbbbbbbbb', 0, 'first-idle', UA_WIN, 120, 7.5, { input: { n: 20, p50Ms: 3, p95Ms: 6 } }),
      rec3('cccccccccccc', 0, { p50: 20, p95: 40 }, { p50: 3, p95: 8 }), rec3('dddddddddddd', 0, { p50: 9, p95: 15 }, { p50: 2, p95: 5 }),
    ].join('\n'));
    expect(s.p2.inputP95Ms).toEqual([40, 15]);
    expect(s.p2.listenerToPutP95Ms).toEqual([6, 8, 5]);
    const g = s.groups.find((x) => x.machine.startsWith('loopback'))!;
    expect(g.p2).toMatchObject({ inputP95MsMin: 15, inputMetRuns: 1, inputRuns: 2, listenerToPutP95MsMin: 5 });
    expect(g.p2.inputStages).toEqual(stages);
  });
  it('a version 3 record with no animation frame yet (empty event->rAF) has no P2 number, not 0', () => {
    const s = summarize(rec3('eeeeeeeeeeee', 0, null, { p50: 4, p95: 9 }));
    expect(s.p2.inputP95Ms).toEqual([]);
    expect(s.rows[0]).toMatchObject({ inputP95Ms: null, inputN: 0, inputDefinition: 'event->rAF' });
  });
  it('the text names the definition on every line: record line, verdict line, group line', () => {
    const out = format(summarize([rec3('aaaaaaaaaaaa', 0, { p50: 22, p95: 34 }, { p50: 4, p95: 9 }), rec2('tailnet', 'bbbbbbbbbbbb', 0, 'first-idle', UA_WIN, 120, 7.5, { input: { n: 20, p50Ms: 3, p95Ms: 6 } })].join('\n')));
    expect(out).toContain('input event->rAF p95 34 ms p50 22 ms (n=20; old listener->put p95 9.0 ms; of 30 processed 25 have a value, 5 (17%) have none (1 late; 4 with no frame soon enough), not in the p95) stages p50 S0 13 + page 0.3 + toWorker 0.5 + worker 2.7 + fromWorker 0.4 + put 0.2 + rAF 13 ms');
    expect(out).toContain('input listener->put p95 6.0 ms (n=20; the old definition, v2: not event->rAF)');
    expect(out).toContain('input event->rAF p95 (version 3 records; target <= 17 ms): 34 ms');
    expect(out).toContain('input event->rAF p95 best 34 ms (target <= 17, met in 0/1 version 3 runs) [old listener->put p95 best 9.0 ms, not judged]');
  });
});


// ---- decision 0209: the table of how the engine ended (engine.exit.message) ----
describe('perf-log-summary: engine stops (decision 0209)', () => {
  const stop = (sid: string, seq: number, reason: string, exit: Record<string, unknown> | null, ua = UA_WIN): string => rec2('tailnet', sid, seq, reason, ua, 60, 11, { engine: { present: 'B', updatesObservable: true, state: exit ? 'exited' : 'engine-idle', exit } });
  const ex = (code: number, message?: string, reason = 'stopped') => ({ reason, code, presents: 5, ...(message === undefined ? {} : { message }) });
  const text = [
    stop('aaaaaaaaaaaa', 0, 'first-idle', null), stop('aaaaaaaaaaaa', 1, 'exit', ex(134, 'fle: SetWindowLongPtrW: GWLP_USERDATA of a foreign window')),
    stop('bbbbbbbbbbbb', 0, 'first-idle', null), stop('bbbbbbbbbbbb', 1, 'exit', ex(134, 'fle: SetWindowLongPtrW: GWLP_USERDATA of a foreign window')),
    stop('cccccccccccc', 0, 'first-idle', null), stop('cccccccccccc', 1, 'exit', ex(134, 'fle: GetKeyState: not modelled'), UA_HL),
    stop('dddddddddddd', 0, 'first-idle', null), stop('dddddddddddd', 1, 'exit', ex(134)),
    stop('eeeeeeeeeeee', 0, 'first-idle', null), stop('eeeeeeeeeeee', 1, 'periodic', null),
  ].join('\n');
  const s = summarize(text);
  it('counts RUNS (page loads) by reason, code and message, the most frequent first; a run that did not end is not in the table', () => {
    expect(s.stops.map((e) => [e.runs, e.reason, e.code, e.message])).toEqual([
      [2, 'stopped', 134, 'fle: SetWindowLongPtrW: GWLP_USERDATA of a foreign window'],
      [1, 'stopped', 134, NO_STOP_MESSAGE],
      [1, 'stopped', 134, 'fle: GetKeyState: not modelled'],
    ]);
    expect(s.stops[0]!.browsers).toEqual(['Chrome 156']);
    expect(s.runs.filter((r) => r.ended)).toHaveLength(4);
  });
  it('a record has its exit reason, code and message; a record without an exit has none, and an older record without a message says so', () => {
    expect(s.rows[0]).toMatchObject({ exitReason: null, exitCode: null, exitMessage: null });
    expect(s.rows[1]).toMatchObject({ exitReason: 'stopped', exitCode: 134, exitMessage: 'fle: SetWindowLongPtrW: GWLP_USERDATA of a foreign window' });
    expect(s.rows[7]).toMatchObject({ exitReason: 'stopped', exitCode: 134, exitMessage: null });
  });
  it('the text has the table and the message on the record line', () => {
    const out = format(s);
    expect(out).toContain('Engine stops (real engine; one run = one page load; 4 of 5 runs ended with an exit record');
    expect(out).toContain('  2 x stopped code 134 | fle: SetWindowLongPtrW: GWLP_USERDATA of a foreign window | Chrome 156');
    expect(out).toContain(`  1 x stopped code 134 | ${NO_STOP_MESSAGE} |`);
    expect(out).toContain('| EXIT stopped code 134: fle: GetKeyState: not modelled');
    expect(out).toContain('| EXIT stopped code 134 (no message)');
  });
  it('a log with no run that ended says so; a stand-in\'s exit is not counted', () => {
    expect(format(summarize(stop('ffffffffffff', 0, 'first-idle', null)))).toContain('no run ended');
    const standIn = JSON.stringify({ t: '2026-11-03T01:00:00.000Z', remote: 'tailnet', perf: { v: 3, sid: 'abababababab', seq: 1, reason: 'exit', bundle: { name: 'standin-x', engineKind: 'unknown-engine-kind-v0', standIn: true }, engine: { exit: { reason: 'finished', code: 0, presents: 1 } } } });
    expect(summarize(standIn).stops).toEqual([]);
  });
});
