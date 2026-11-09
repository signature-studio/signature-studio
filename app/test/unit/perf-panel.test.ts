// Decision 0152: the Performance panel shows the measured numbers next to the targets and never states a target as reached unless a measured number says so.
import { describe, expect, it } from 'vitest';
import { perfNotice, perfRows, renderPerfPanel, screenNote, type PanelView, type PerfRow } from '../../src/perf/panel.ts';
import type { CanvasSummary, DisplaySnapshot, HzInfo, RafSummary } from '../../src/perf/display.ts';
import { collectClientInfo } from '../../src/perf/browser-info.ts';

const none = { n: 0, intervalMedianMs: null, intervalP95Ms: null, perSecond: null };
const stat0 = { n: 0, p50Ms: null, p95Ms: null, maxMs: null, lastMs: [] };
const input0 = { sent: 0, processed: 0, dropped: 0, pending: 0, answered: 0, late: 0, noAnswer: 0, eventToRaf: stat0, listenerToPut: stat0, lateVisible: null, eventToRafWithLate: null, stages: null };
const stat = (n: number, p50: number, p95: number) => ({ n, p50Ms: p50, p95Ms: p95, maxMs: p95, lastMs: [] as number[] });
const view = (over: Partial<PanelView> = {}): PanelView => ({
  waitingSeconds: null, firstIdleSeconds: null, updates: none, input: input0, underruns: null, server: { sent: 0, stored: 0, accepted: 0, lastNote: null }, ...over,
});
const row = (rows: PerfRow[], id: string): PerfRow => rows.find((r) => r.id === id) as PerfRow;

describe('perfRows: nothing measured yet', () => {
  it('every target row is "na" with a dash, none is met or missed', () => {
    const rows = perfRows(view());
    for (const id of ['p1', 'p2-rate', 'p2-interval', 'p2-input']) expect(row(rows, id).status, id).toBe('na');
    expect(row(rows, 'p1').value).toBe('-');
    expect(row(rows, 'p3')).toMatchObject({ status: 'na', value: 'not measured (the audio output is off or has not run)', target: '0' });
    expect(row(rows, 'beacon').value).toBe('nothing sent yet');
  });
});

describe('engine stop (decision 0209)', () => {
  it('a dash while the engine runs; how it ended and its last words after; never green or red', () => {
    expect(row(perfRows(view()), 'engine-stop')).toMatchObject({ value: '-', status: 'na' });
    expect(row(perfRows(view({ exit: { reason: 'stopped', code: 134, message: 'fle: A: why' } })), 'engine-stop')).toMatchObject({ value: 'stopped, exit code 134: fle: A: why', status: 'na' });
    expect(row(perfRows(view({ exit: { reason: 'stopped', code: 134, message: null } })), 'engine-stop').value).toBe('stopped, exit code 134: (the engine printed no message)');
    expect(row(perfRows(view({ exit: { reason: 'finished', code: 0, message: null } })), 'engine-stop').status).toBe('na');
  });
  it('the notice names the field that is sent and what is taken out of it', () => {
    const t = perfNotice().join('\n');
    for (const w of ['エンジンが止まったときのエンジン自身の最後のメッセージ', '最大 400 文字', 'ホームディレクトリ', 'メールアドレス']) expect(t, w).toContain(w);
  });
});

describe('engine memory (decision 0170)', () => {
  it('a dash before the first idle; the size and its share of the 2,048 MB limit after; never green or red (it is not one of P1-P3)', () => {
    expect(row(perfRows(view()), 'wasm-memory')).toMatchObject({ value: '-', target: '< 2048 MB', status: 'na' });
    expect(row(perfRows(view({ wasmMemoryBytes: 1473.4 * 1048576 })), 'wasm-memory')).toMatchObject({ value: '1473 MB (72% of 2048 MB)', status: 'na' });
    expect(row(perfRows(view({ wasmMemoryBytes: 2000 * 1048576 })), 'wasm-memory').status).toBe('na');
  });
});

describe('P1', () => {
  it('40.0 s against 10 s is a miss; 9.9 and 10.0 are met; 10.04 s is shown as 10.0 but is a miss (the comparison is on the number)', () => {
    expect(row(perfRows(view({ firstIdleSeconds: 40 })), 'p1')).toMatchObject({ value: '40.0 s', target: '<= 10 s', status: 'miss' });
    expect(row(perfRows(view({ firstIdleSeconds: 9.9 })), 'p1').status).toBe('met');
    expect(row(perfRows(view({ firstIdleSeconds: 10 })), 'p1').status).toBe('met');
    expect(row(perfRows(view({ firstIdleSeconds: 10.04 })), 'p1')).toMatchObject({ value: '10.0 s', status: 'miss' });
  });
  it('while waiting: "not yet", grey until 10 s have passed, red after', () => {
    expect(row(perfRows(view({ waitingSeconds: 4.2 })), 'p1')).toMatchObject({ value: 'not yet, 4.2 s so far', status: 'na' });
    expect(row(perfRows(view({ waitingSeconds: 10 })), 'p1').status).toBe('na');
    expect(row(perfRows(view({ waitingSeconds: 10.5 })), 'p1').status).toBe('miss');
  });
});

describe('P2', () => {
  it('updates per second: 0.3 /s against 120 is a miss; 119.9 a miss; 120 and 144 are met', () => {
    const r = (perSecond: number) => row(perfRows(view({ updates: { n: 3, intervalMedianMs: 3000, intervalP95Ms: 3200, perSecond } })), 'p2-rate');
    expect(r(0.3)).toMatchObject({ value: '0.3 /s (3 updates)', target: '>= 120 /s', status: 'miss' });
    expect(r(119.9).status).toBe('miss');
    expect(r(120).status).toBe('met');
    expect(r(144).value).toBe('144 /s (3 updates)');
  });
  it('the interval row judges the p95 against one frame at 120 fps (8.3 ms)', () => {
    const r = (p95: number) => row(perfRows(view({ updates: { n: 9, intervalMedianMs: 8, intervalP95Ms: p95, perSecond: 120 } })), 'p2-interval');
    expect(r(3200)).toMatchObject({ value: '8.0 ms / 3200 ms', target: 'p95 <= 8.3 ms', status: 'miss' });
    expect(r(8.3).status).toBe('met');
    expect(r(8.34).status).toBe('miss');
  });
  it('input -> update (decision 0202): the p95 of event.timeStamp -> next animation frame against 17 ms; shows p50, the counts and, small, the old number', () => {
    const i = (p95: number) => row(perfRows(view({ input: { sent: 12, processed: 10, dropped: 1, pending: 1, answered: 8, late: 1, noAnswer: 1, eventToRaf: stat(8, 5, p95), listenerToPut: stat(8, 2, 3), lateVisible: null, eventToRafWithLate: null, stages: null } })), 'p2-input');
    expect(i(1900)).toMatchObject({ value: '1900 ms (p50 5.0 ms; 12 sent, 1 pending; of 10 processed 8 have a value, 2 (20%) have none: 1 late (no frame in the processing window, a first later frame within 150 ms: with dedupe off the page cannot tell if it changed anything, so these are not in the p95), 1 with no frame soon enough; the old number (page listener -> putImageData, which leaves out the browser\'s waits): p95 3.0 ms, p50 2.0 ms)', target: 'p95 <= 17 ms', status: 'miss' });
    expect(i(17).status).toBe('met');
    expect(i(17.1).status).toBe('miss');
    expect(i(12).label).toBe('P2 input -> next update: event.timeStamp -> next animation frame after the draw, p95 of the last 8');
  });
  it('decision 0205, ?dedupe=1: the p95 judged is over the answered inputs AND the late visible changes, the answered-only number stands beside it; the verdict follows the first one', () => {
    const i = (answeredP95: number, withLateP95: number) => row(perfRows(view({ input: { sent: 40, processed: 40, dropped: 0, pending: 0, answered: 28, late: 10, noAnswer: 2, eventToRaf: stat(20, 6, answeredP95), listenerToPut: stat(20, 2, 3), lateVisible: stat(10, 25, 30), eventToRafWithLate: stat(20, 12, withLateP95), stages: null } })), 'p2-input');
    const r = i(14, 31);
    expect(r.status).toBe('miss');                       // the answered inputs alone would be met (14 ms): the late visible changes are not left out
    expect(r.value).toContain('31 ms (p50 12 ms, over the answered inputs and the late visible changes; answered inputs only: p95 14 ms, p50 6.0 ms (n=20)');
    expect(r.value).toContain('10 late (a visible change after the engine\'s first idle, 25 ms / 30 ms p50 / p95 after the event; in the p95 judged)');
    expect(r.value).toContain('of 40 processed 28 have a value, 12 (30%) have none');
    expect(r.label).toBe('P2 input -> next update: event.timeStamp -> next animation frame after the draw, p95 of the last 20');
    expect(i(30, 12).status).toBe('met');                // the other way round: the late ones can not make it worse than the answered ones alone only if they are fast
  });
  it('the judgement is the new number\'s, never the old one\'s: a small old number with no new number is "na", and a large old one with a small new one is "met"', () => {
    const r = (e: typeof stat0 | ReturnType<typeof stat>, o: typeof stat0 | ReturnType<typeof stat>) => row(perfRows(view({ input: { ...input0, sent: 5, eventToRaf: e, listenerToPut: o } })), 'p2-input');
    expect(r(stat0, stat(5, 2, 3))).toMatchObject({ status: 'na' });
    expect(r(stat0, stat(5, 2, 3)).value).toContain('p95 3.0 ms');
    expect(r(stat(5, 8, 12), stat(5, 40, 90)).status).toBe('met');
  });
  it('the stages row names where the time goes, with no target and no verdict', () => {
    const p = { s0: 13, s0b: 0.3, toWorker: 0.5, worker: 2.7, fromWorker: 0.4, put: 0.2, raf: 13 };
    const stages = { n: 20, p50: p, p95: { ...p, s0: 15 } };
    expect(row(perfRows(view({ input: { ...input0, stages } })), 'p2-input-stages')).toMatchObject({ value: expect.stringContaining('browser wait for the event 13 + page 0.3 + to the Worker 0.5 + engine and Worker 2.7 + back to the page 0.4 + draw 0.2 + wait for the next animation frame 13 ms (20 events'), target: '-', status: 'na' });
    expect(row(perfRows(view({ input: input0 })), 'p2-input-stages')).toMatchObject({ value: '-', status: 'na' });
  });
});

describe('P3 and the beacon row', () => {
  it('under-runs: null is not measured, 0 is met, 3 is a miss', () => {
    expect(row(perfRows(view({ underruns: 0 })), 'p3')).toMatchObject({ value: '0', status: 'met' });
    expect(row(perfRows(view({ underruns: 3 })), 'p3')).toMatchObject({ value: '3', status: 'miss' });
    expect(row(perfRows(view({ underruns: 0, underrunsNote: 'last 60 s: 0' })), 'p3')).toMatchObject({ value: '0 (last 60 s: 0)', status: 'met' });
  });
  it('the beacon row says how many were sent, accepted and stored, and the last answer', () => {
    expect(row(perfRows(view({ server: { sent: 3, accepted: 3, stored: 0, lastNote: '#2 periodic: accepted, not stored by this server' } })), 'beacon').value).toBe('3 sent, 3 accepted, 0 stored by the server; last: #2 periodic: accepted, not stored by this server');
  });
});

// ---- decision 0192: the screen rows, the notice, the machine ----

const rafOf = (hz: number | null, o: Partial<RafSummary> = {}): RafSummary => ({ n: 600, perSecond: hz, medianMs: hz ? 1000 / hz : null, p95Ms: hz ? 1000 / hz : null, p99Ms: hz ? 1000 / hz : null, maxMs: hz ? 1000 / hz : null, hz, peakHz: hz, slow: 0, over12p5: hz && hz < 80 ? 600 : 0, bins: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0], ...o });
const canvasOf = (o: Partial<CanvasSummary> = {}): CanvasSummary => ({ draws: 1200, drawsPerSecond: 120, ticks: 600, freshTicks: 600, freshRatio: 1, freshPerSecond: 120, drawsPerFreshTick: 1, drawMs: { n: 1200, p50: 0.4, p95: 1.1, max: 3 }, transitMs: { n: 1200, p50: 1, p95: 2, max: 5 }, ...o });
const snap = (o: { raf?: RafSummary; canvas?: CanvasSummary | null; lt?: Partial<DisplaySnapshot['longTasks']>; ev?: Partial<DisplaySnapshot['events']> } = {}): DisplaySnapshot => {
  const raf = o.raf ?? rafOf(120);
  return {
    rafSupported: true, runningMs: 60000, visibility: { visibleMs: 60000, hiddenMs: 0, changes: 0, visibleNow: true },
    raf: { recent10s: raf, sinceIdle: raf, total: raf, binEdgesMs: [6, 10.5, 14, 20, 30, 45, 70, 120, 250] },
    canvas: o.canvas === null ? null : { received: 1200, drawn: 1200, recent10s: o.canvas ?? canvasOf(), sinceIdle: null, total: o.canvas ?? canvasOf(), workerSent: null, workerOmitted: null },
    longTasks: { supported: true, total: 0, maxMs: null, sumMs: 0, sinceIdle: 0, maxSinceIdleMs: null, recent30s: 0, ...o.lt },
    events: { supported: true, thresholdMs: 16, slow: 0, inputsSent: 10, delayMs: null, durationMs: null, firstInput: null, ...o.ev },
  };
};
const hzOf = (o: Partial<HzInfo> = {}): HzInfo => ({ param: null, request: 'auto', engineEnvHz: 120, engineHz: 120, autoMeasuredHz: null, rafHz: 120, peakHz: 120, basis: 'since-idle', ratio: 1, fit: 'match', note: 'エンジン 120 Hz と画面の rAF 120 Hz は合っています', ...o });

describe('the screen rows: the display rate against the 120 fps of P2', () => {
  it('a 60 Hz screen: red, and the row says this screen is 60 Hz class and cannot reach 120 fps in principle', () => {
    const rows = perfRows(view({ display: snap({ raf: rafOf(60) }) }));
    const r = row(rows, 'display-hz');
    expect(r.status).toBe('miss');
    expect(r.value).toContain('60.0 Hz');
    expect(r.value).toContain('この画面は 60 Hz 相当');
    expect(r.value).toContain('120 fps は原理的に出せません');
    expect(r.target).toContain('>= 114 Hz');
    expect(row(rows, 'raf-rate')).toMatchObject({ status: 'miss', value: '60 /s(間隔 600 個)' });
    expect(row(rows, 'raf-gaps')).toMatchObject({ status: 'miss', target: 'p95 <= 8.3 ms' });
  });
  it('a 120 Hz screen: met, no warning about the screen; 119.9 Hz is met, 113 Hz is not', () => {
    const rows = perfRows(view({ display: snap({ raf: rafOf(120) }) }));
    expect(row(rows, 'display-hz').status).toBe('met');
    expect(row(rows, 'display-hz').value).toContain('この画面は 120 Hz 相当です');
    expect(row(rows, 'display-hz').value).not.toContain('原理的に');
    expect(row(rows, 'raf-rate').status).toBe('met');
    expect(row(rows, 'raf-gaps').status).toBe('met');
    expect(row(perfRows(view({ display: snap({ raf: rafOf(119.9) }) })), 'display-hz').status).toBe('met');
    expect(row(perfRows(view({ display: snap({ raf: rafOf(114) }) })), 'display-hz').status).toBe('met');
    expect(row(perfRows(view({ display: snap({ raf: rafOf(113.9) }) })), 'display-hz').status).toBe('miss');
  });
  it('a 144 Hz screen is met and named', () => {
    expect(row(perfRows(view({ display: snap({ raf: rafOf(144) }) })), 'display-hz').value).toContain('144 Hz 相当');
  });
  it('the p95 of the gaps is judged against 8.3 ms with 10% for timestamp jitter: 9.1 ms is met, 9.2 ms is not', () => {
    const g = (p95: number) => row(perfRows(view({ display: snap({ raf: rafOf(120, { p95Ms: p95 }) }) })), 'raf-gaps').status;
    expect(g(9.1)).toBe('met');
    expect(g(9.2)).toBe('miss');
  });
  it('dropped frames: more than 1% of the intervals is red', () => {
    const d = (slow: number) => row(perfRows(view({ display: snap({ raf: rafOf(120, { slow }) }) })), 'raf-dropped');
    expect(d(6)).toMatchObject({ status: 'met', value: '6 / 600(1.0%);12.5 ms 超 0' });
    expect(d(7).status).toBe('miss');
  });
  it('the page being behind (the fastest frames are twice as fast as the median) is said as that, not as a 60 Hz screen', () => {
    expect(screenNote(60, 120)).toContain('ページが間に合わず');
    const r = row(perfRows(view({ display: snap({ raf: rafOf(60, { peakHz: 120 }) }) })), 'display-hz');
    expect(r.value).toContain('ページが間に合わず');
    expect(r.value).not.toContain('60 Hz 相当');
  });
  it('a rAF below 50 Hz is a throttled page (power saving, load), not a kind of screen', () => {
    expect(screenNote(30, 30)).toContain('30 回/s しか来ていません');
    expect(screenNote(30, 30)).not.toContain('Hz 相当');
  });
  it('too few intervals: a dash, not a number; nothing is red or green', () => {
    const rows = perfRows(view({ display: snap({ raf: rafOf(60, { n: 12 }) }) }));
    expect(row(rows, 'display-hz')).toMatchObject({ status: 'na' });
    expect(row(rows, 'display-hz').value).toContain('間隔 12 個');
    expect(row(rows, 'raf-gaps').status).toBe('na');
    expect(row(rows, 'raf-dropped').status).toBe('na');
  });
  it('the meter off (?beacon=0 and the panel never opened): rows say "not measured", all grey', () => {
    const rows = perfRows(view({ display: null }));
    expect(row(rows, 'display-hz')).toMatchObject({ status: 'na' });
    expect(row(rows, 'display-hz').value).toContain('測っていません');
    expect(row(rows, 'canvas-fps').status).toBe('na');
    expect(rows.some((r) => r.id === 'longtasks')).toBe(false);
  });
});

describe('the canvas rows', () => {
  it('120 canvas-updating frames a second is met; 59 is a miss; the row also says draws per frame and received / drawn', () => {
    const ok = row(perfRows(view({ display: snap() })), 'canvas-fps');
    expect(ok.status).toBe('met');
    expect(ok.value).toContain('1 フレームに平均 1.0 回描画');
    expect(ok.value).toContain('受信 1200 回・描画 1200 回');
    const slow = row(perfRows(view({ display: snap({ canvas: canvasOf({ freshPerSecond: 59, drawsPerFreshTick: 2.1, drawsPerSecond: 125, freshRatio: 1 }) }) })), 'canvas-fps');
    expect(slow.status).toBe('miss');
    expect(slow.value).toContain('平均 2.1 回描画');
  });
  it('the share of screen frames with new content: 50% while the canvas is moving is red, 95% is green; when nothing moves (a static screen) it is not judged', () => {
    const f = (freshRatio: number, drawsPerSecond: number) => row(perfRows(view({ display: snap({ canvas: canvasOf({ freshRatio, drawsPerSecond, freshTicks: Math.round(600 * freshRatio) }) }) })), 'canvas-fresh');
    expect(f(0.5, 60).status).toBe('miss');
    expect(f(0.95, 60).status).toBe('met');
    expect(f(0.94, 60).status).toBe('miss');
    expect(f(0.007, 0.8)).toMatchObject({ status: 'na' });
    expect(f(0.007, 0.8).value).toContain('動いていないので判定しません');
  });
  it('draw time: p95 against half a frame of 120 fps (4.2 ms)', () => {
    const d = (p95: number) => row(perfRows(view({ display: snap({ canvas: canvasOf({ drawMs: { n: 100, p50: 1, p95, max: p95 } }) }) })), 'canvas-draw');
    expect(d(4.1).status).toBe('met');
    expect(d(4.2).status).toBe('met');
    expect(d(4.3).status).toBe('miss');
    expect(d(4.1).value).toContain('p95 4.1 ms');
    expect(d(4.1).value).toContain('Worker が post してから受け取るまで');
  });
  it('display method A: the page cannot see the canvases — a statement, grey rows', () => {
    const r = row(perfRows(view({ display: snap({ canvas: null }) })), 'canvas-fps');
    expect(r.status).toBe('na');
    expect(r.value).toContain('表示方式 A');
  });
});

describe('long tasks, slow input events, the engine rate, visibility, the machine', () => {
  it('long tasks since the first idle: 0 is met, 2 is red with the largest; before the first idle and unsupported are grey', () => {
    expect(row(perfRows(view({ display: snap() })), 'longtasks').status).toBe('met');
    const bad = row(perfRows(view({ display: snap({ lt: { sinceIdle: 2, maxSinceIdleMs: 140, recent30s: 1, total: 5 } }) })), 'longtasks');
    expect(bad.status).toBe('miss');
    expect(bad.value).toContain('2 回(最大 140 ms');
    expect(row(perfRows(view({ display: snap({ lt: { sinceIdle: null, total: 3, maxMs: 900 } }) })), 'longtasks').status).toBe('na');
    expect(row(perfRows(view({ display: snap({ lt: { supported: false } }) })), 'longtasks')).toMatchObject({ status: 'na', value: 'このブラウザでは測れません' });
  });
  it('slow input events: any is red and the row says the API reports only events of 16 ms or more; unsupported is grey', () => {
    const r = row(perfRows(view({ display: snap({ ev: { slow: 3, inputsSent: 40, delayMs: { n: 3, p50: 6, p95: 20, max: 20 }, firstInput: { name: 'pointerdown', delayMs: 4.4, durationMs: 24 } } }) })), 'slow-events');
    expect(r.status).toBe('miss');
    expect(r.label).toContain('16 ms 以上');
    expect(r.value).toContain('3 件 / 送った入力 40 件');
    expect(r.value).toContain('p95 20 ms');
    expect(row(perfRows(view({ display: snap() })), 'slow-events').status).toBe('met');
    expect(row(perfRows(view({ display: snap({ ev: { supported: false } }) })), 'slow-events').status).toBe('na');
  });
  it('the engine rate against the screen: a match is met, anything else red with the page\'s sentence, unknown grey', () => {
    expect(row(perfRows(view({ hz: hzOf() })), 'hz-fit').status).toBe('met');
    const bad = row(perfRows(view({ hz: hzOf({ fit: 'display-slower', rafHz: 60, ratio: 2, note: 'エンジンは 120 Hz で描画しますが、この画面の rAF は 60 回/s なので、見えるのは最大 60 fps です' }) })), 'hz-fit');
    expect(bad.status).toBe('miss');
    expect(bad.value).toContain('エンジン 120 Hz');
    expect(bad.value).toContain('最大 60 fps');
    expect(row(perfRows(view({ hz: hzOf({ fit: 'unknown', rafHz: null, engineHz: 60, note: 'x' }) })), 'hz-fit').status).toBe('na');
    expect(row(perfRows(view({ hz: null })), 'hz-fit').status).toBe('na');
  });
  it('the visible / hidden time is shown, grey (hidden time is why the fps statistics skip it)', () => {
    const d = snap();
    d.visibility = { visibleMs: 312_000, hiddenMs: 41_000, changes: 2, visibleNow: true };
    const r = row(perfRows(view({ display: d })), 'visibility');
    expect(r.status).toBe('na');
    expect(r.value).toContain('見えていた 312 s / 隠れていた 41 s');
  });
  it('the machine row names the browser, the logical cores (and that physical cores cannot be read), the screen and the GPU', () => {
    const client = collectClientInfo({ navigator: { userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36' }, screen: { colorDepth: 24 }, innerWidth: 1900, innerHeight: 960, isSecureContext: true, location: { hostname: 'x.ts.net' } });
    const browser = { hardwareConcurrency: 16, deviceMemoryGB: 8, userAgent: 'x', screen: { width: 2560, height: 1440 }, devicePixelRatio: 1.5, crossOriginIsolated: true, isSecureContext: true, webglRenderer: 'ANGLE (NVIDIA)', jsHeap: null };
    const r = row(perfRows(view({ client, browser })), 'machine');
    expect(r.status).toBe('na');
    for (const t of ['Chrome 156.0', 'Windows', '論理コア 16', '物理コア数は Web API で取れません', 'メモリの目安 8 GB', '2560x1440', '拡大率 1.5', '24 bit', '1900x960', 'ANGLE (NVIDIA)']) expect(r.value, t).toContain(t);
  });
  it('after a refusal by the server the beacon row says nothing is sent, and why', () => {
    const r = row(perfRows(view({ server: { sent: 3, stored: 0, accepted: 0, lastNote: null, off: 'the server answered HTTP 403 to a beacon; no more are sent' } })), 'beacon');
    expect(r.value).toBe('送っていません: the server answered HTTP 403 to a beacon; no more are sent');
  });
});

describe('the notice (what is sent, what is not, how to stop it)', () => {
  it('says whose numbers they are, that they go to the server\'s log, what is sent, that IP / cookie / typed text / URL are not, and how to stop it', () => {
    const t = perfNotice().join('\n');
    for (const w of ['あなたのブラウザ・あなたの機械で実測', 'サーバーのログ', 'perf-log.jsonl', '送る項目', 'requestAnimationFrame', '論理コア数', 'GPU の名前', '送らないもの', 'IP アドレス', 'Cookie', '入力した文字', 'URL', '物理コア数はブラウザから取れない', '起動に使う 4 つのデータの由来', '保存領域の使用量と上限']) expect(t, w).toContain(w);
  });
  it('the paragraph of what is NOT sent names the IP address first and says what the server records instead; the paragraph of what is sent mentions IP addresses only as taken out of the engine\'s last message', () => {
    const p = perfNotice();
    const not = p.find((x) => x.startsWith('送らないもの'));
    expect(not, 'there is a paragraph that starts with 送らないもの').toBeDefined();
    expect(not).toContain('送らないもの: IP アドレス(サーバーは相手が tailnet か手元かの種類だけを記録します)');
    for (const w of ['Cookie', '入力した文字', 'URL', 'ファイルの中身', '外部への通信はありません']) expect(not, w).toContain(w);
    const sent = p.find((x) => x.startsWith('送る項目'));
    expect(sent).toContain('IP アドレスは送る前に除いています');
    expect(sent).not.toContain('送らないもの');
  });
  it('is rendered above the table, one paragraph each', () => {
    // a minimal DOM: the tests run in node
    interface N { tag: string; className: string; textContent: string; dataset: Record<string, string>; children: N[]; ownerDocument?: D; append(...k: N[]): void; replaceChildren(...k: N[]): void }
    interface D { createElement(tag: string): N }
    const doc: D = { createElement: (tag) => { const n: N = { tag, className: '', textContent: '', dataset: {}, children: [], append(...k) { this.children.push(...k); }, replaceChildren(...k) { this.children = k; } }; n.ownerDocument = doc; return n; } };
    const root = doc.createElement('div');
    renderPerfPanel(root as unknown as HTMLElement, [{ id: 'a', label: 'l', value: 'v', target: 't', status: 'na' }], perfNotice());
    expect(root.children.map((c) => c.className)).toEqual(['perf-notice', 'perf-table']);
    expect(root.children[0]?.children.map((p) => p.tag)).toEqual(['p', 'p', 'p']);
    expect(root.children[0]?.children[0]?.textContent).toContain('あなたのブラウザ');
  });
});


describe('the screen meter\'s mode row (decision 0202)', () => {
  it('burst mode is stated with its numbers and why; continuous says so; no meter has no row', () => {
    const base = snap();
    const burst = { ...base, mode: 'burst' as const, burst: { onMs: 2000, periodMs: 60000, count: 3 } };
    const r = row(perfRows(view({ display: burst })), 'display-mode');
    expect(r.status).toBe('na');
    expect(r.value).toContain('間欠');
    expect(r.value).toContain('60 秒ごとに 2 秒');
    expect(r.value).toContain('いままで 3 回');
    expect(r.value).toContain('決定 0202');
    expect(row(perfRows(view({ display: { ...base, mode: 'continuous' as const, burst: null } })), 'display-mode').value).toContain('連続');
    expect(perfRows(view({ display: null })).some((x) => x.id === 'display-mode')).toBe(false);
  });
});
