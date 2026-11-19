import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AUDIO, audioOffReason, GESTURE_EVENTS, installAudio } from '../../src/audio/install.ts';
import { KEEP_PULSES, KEEP_REPORTS, KEEP_UNLOCKS, PageAudio, RESUME_WAIT_MS } from '../../src/audio/page-audio.ts';
import { GRACE_MS, SoftSink } from '../../src/audio/soft-sink.ts';
import { audioLine, bindAudioStatus } from '../../src/audio/status.ts';
import { W } from '../../src/audio/ring-layout.ts';
import { beaconProblems } from '../../shared/perf.ts';
import { buildBeacon } from '../../src/perf/beacon.ts';

// decision 0211: the audio output is on by default, a suspended context must not stop the song, and the page says why it is silent.

const header = (): Int32Array => { const h = new Int32Array(new SharedArrayBuffer(128)); return h; };

describe('audio on by default (decision 0211)', () => {
  it('the audio output has no on/off parameter and no URL parameter: the ring and the rate are constants', () => {
    expect(AUDIO).toEqual({ sampleRate: 44100, capFrames: 32768 });
  });

  it('the reason there is no audio output: no SharedArrayBuffer (not cross-origin isolated), no AudioContext, no AudioWorklet; null when the browser has all of them', () => {
    const full = { SharedArrayBuffer, AudioContext: class {}, AudioWorkletNode: class {} } as never;
    expect(audioOffReason(full)).toBeNull();
    expect(audioOffReason({ ...(full as object), SharedArrayBuffer: undefined } as never)).toMatch(/SharedArrayBuffer/);
    expect(audioOffReason({ ...(full as object), AudioContext: undefined } as never)).toMatch(/AudioContext/);
    expect(audioOffReason({ ...(full as object), AudioWorkletNode: undefined } as never)).toMatch(/AudioWorklet/);
  });

  it('installAudio makes nothing (and leaves Worker alone) when the browser cannot', () => {
    class W0 {}
    const win = { SharedArrayBuffer: undefined, AudioContext: class {}, AudioWorkletNode: class {}, Worker: W0 } as never as Window & typeof globalThis;
    expect(installAudio(win)).toBeNull();
    expect((win as unknown as { Worker: unknown }).Worker).toBe(W0);
  });

  it('every event that a browser may count as a user activation resumes the context: pointer, mouse, click, key and touch', () => {
    for (const e of ['pointerdown', 'mousedown', 'click', 'keydown', 'touchend']) expect(GESTURE_EVENTS as readonly string[]).toContain(e);
  });
});

describe('SoftSink: the engine\'s play cursor moves while the AudioWorklet does not run', () => {
  it('moves RD by the elapsed time at the ring rate once the worklet\'s quantum counter has stood still for GRACE_MS while the device is PLAYING', () => {
    const h = header();
    const s = new SoftSink(h, () => 44100);
    Atomics.store(h, W.STATE, 2);
    expect(s.tick(1000)).toBe(0);                                   // the first tick only sets the baseline
    expect(s.tick(1000 + GRACE_MS - 10)).toBe(0);                   // the worklet may only be late
    expect(s.tick(1000 + GRACE_MS)).toBe(4410);                     // 100 ms since the baseline, all of it at once
    expect(Atomics.load(h, W.RD)).toBe(4410);
    expect(s.tick(1000 + GRACE_MS + 10)).toBe(441);
    expect(Atomics.load(h, W.RD)).toBe(4851);
    expect(s.frames).toBe(4851);
    expect(s.ticksMoved).toBe(2);
  });

  it('keeps the fraction of a frame: 10 s of ticks at 16,000.5 Hz move 160,005 frames (to within the float rounding of one frame), not a drifting count', () => {
    const h = header();
    const t = new SoftSink(h, () => 16000.5);
    Atomics.store(h, W.STATE, 2);
    t.tick(0);
    let total = 0;
    for (let i = 1; i <= 1000; i++) total += t.tick(i * 10);
    expect(total).toBeGreaterThanOrEqual(160004);
    expect(total).toBeLessThanOrEqual(160005);
    expect(Atomics.load(h, W.RD)).toBe(total);
  });

  it('does nothing while the device is not PLAYING, and does not count that time afterwards', () => {
    const h = header();
    const s = new SoftSink(h, () => 44100);
    Atomics.store(h, W.STATE, 1);
    s.tick(0);
    expect(s.tick(500)).toBe(0);
    expect(s.tick(10000)).toBe(0);
    Atomics.store(h, W.STATE, 2);
    expect(s.tick(10010)).toBe(0);                                   // the baseline of the new state
    expect(s.tick(10010 + GRACE_MS)).toBe(4851);                     // 110 ms: the time since the last tick that saw the device not playing (10,000), not the 10 s before
    expect(Atomics.load(h, W.RD)).toBe(4851);
  });

  it('does nothing while the worklet processes quanta (Q_TOTAL moves): the worklet owns RD', () => {
    const h = header();
    const s = new SoftSink(h, () => 44100);
    Atomics.store(h, W.STATE, 2);
    s.tick(0);
    for (let t = 10; t <= 1000; t += 10) { Atomics.add(h, W.Q_TOTAL, 3); expect(s.tick(t)).toBe(0); }
    expect(Atomics.load(h, W.RD)).toBe(0);
    expect(s.frames).toBe(0);
  });

  it('takes over again when the worklet stops (the context was suspended) and gives RD back when it returns', () => {
    const h = header();
    const s = new SoftSink(h, () => 44100);
    Atomics.store(h, W.STATE, 2);
    s.tick(0);
    Atomics.add(h, W.Q_TOTAL, 3);
    s.tick(10);                                                      // the last tick with a moving counter
    expect(s.tick(10 + GRACE_MS - 1)).toBe(0);
    expect(s.tick(10 + GRACE_MS)).toBe(4410);
    Atomics.add(h, W.Q_TOTAL, 3);                                    // the worklet is back
    expect(s.tick(10 + GRACE_MS + 10)).toBe(0);
    expect(s.tick(10 + GRACE_MS + 20)).toBe(0);
    expect(Atomics.load(h, W.RD)).toBe(4410);
  });

  it('a stalled page (a late tick) catches the cursor up in one step, never backwards', () => {
    const h = header();
    const s = new SoftSink(h, () => 44100);
    Atomics.store(h, W.STATE, 2);
    s.tick(0);
    expect(s.tick(2000)).toBe(88200);
    expect(s.tick(1999)).toBe(0);                                    // a clock that went back moves nothing
    expect(Atomics.load(h, W.RD)).toBe(88200);
  });
});

describe('the status the page shows', () => {
  const base = { status: 'running', ctxState: 'running', sampleRate: 44100, outputLatencyMs: 24.3, underrunQuanta: 5, underrunQuantaLast60s: 0, gestures: 3, lastResume: null, autoplayPolicy: null, error: null } as never;
  it('says why it is silent: unusable, waiting for a gesture, starting, refused by the browser, failed', () => {
    expect(audioLine('no SharedArrayBuffer (the page is not cross-origin isolated)', null)).toMatchObject({ kind: 'unavailable' });
    expect(audioLine(null, null).kind).toBe('starting');
    expect(audioLine(null, { ...(base as object), status: 'idle', ctxState: null } as never)).toMatchObject({ kind: 'idle', text: '音: エンジンの起動待ち' });
    expect(audioLine(null, { ...(base as object), status: 'needs-gesture', ctxState: 'suspended' } as never)).toMatchObject({ kind: 'needs-gesture', text: '音: 停止中 (クリックまたはキー入力で有効になります)' });
    expect(audioLine(null, { ...(base as object), status: 'resuming', ctxState: 'suspended' } as never).kind).toBe('resuming');
    expect(audioLine(null, { ...(base as object), status: 'blocked', ctxState: 'suspended', lastResume: { result: 'pending' } } as never).text).toMatch(/許可していません/);
    expect(audioLine(null, { ...(base as object), status: 'error', error: 'NotSupportedError: x' } as never)).toMatchObject({ kind: 'error', title: 'NotSupportedError: x' });
  });
  it('running shows the sample rate, the output latency and the underruns of the last minute (only when there are some)', () => {
    expect(audioLine(null, base).text).toBe('音: 動作中 44.1 kHz · 出力遅延 24 ms');
    expect(audioLine(null, { ...(base as object), underrunQuantaLast60s: 12, sampleRate: 48000, outputLatencyMs: 0 } as never).text).toBe('音: 動作中 48 kHz · 直近 60 秒の途切れ 12');
  });
  it('bindAudioStatus writes the text, the state and the title into the element and again on every change and every second', () => {
    const el = { textContent: '', dataset: {} as Record<string, string>, title: '' } as unknown as HTMLElement;
    let cb: () => void = () => {};
    let tick: () => void = () => {};
    let d = { ...(base as object), status: 'needs-gesture', ctxState: 'suspended' } as never;
    const audio = { detail: () => d, onChange: (fn: () => void) => { cb = fn; return () => {}; } } as unknown as PageAudio;
    bindAudioStatus(el, null, audio, (fn) => { tick = fn; });
    expect(el.dataset.state).toBe('needs-gesture');
    d = base;
    cb();
    expect(el.dataset.state).toBe('running');
    expect(el.textContent).toMatch(/動作中/);
    d = { ...(base as object), underrunQuantaLast60s: 7 } as never;
    tick();
    expect(el.textContent).toMatch(/途切れ 7/);
    const off = { textContent: '', dataset: {} as Record<string, string>, title: '' } as unknown as HTMLElement;
    bindAudioStatus(off, 'no AudioWorklet', null);
    expect(off.dataset.state).toBe('unavailable');
  });
});

// a fake of the parts of the Web Audio API that PageAudio.start() and resume() use
class FakeCtx {
  static last: FakeCtx | null = null;
  state: string = 'suspended';
  sampleRate: number;
  baseLatency = 0.0107;
  outputLatency = 0.024;
  currentTime = 0;
  destination = {};
  audioWorklet = { addModule: vi.fn(async () => {}) };
  onstatechange: (() => void) | null = null;
  static defaultResume: (c: FakeCtx) => Promise<void> = () => new Promise(() => {});   // a browser that keeps the context suspended without a gesture
  resumeImpl: () => Promise<void>;
  constructor(o?: { sampleRate?: number }) { this.sampleRate = o?.sampleRate ?? 44100; FakeCtx.last = this; this.resumeImpl = () => FakeCtx.defaultResume(this); }
  resume(): Promise<void> { return this.resumeImpl(); }
  close(): Promise<void> { this.set('closed'); return Promise.resolve(); }
  set(s: string): void { this.state = s; this.onstatechange?.(); }
}
class FakeNode { port = { onmessage: null as unknown, postMessage: vi.fn() }; connect = vi.fn(); ctx: unknown; name: string; opts: unknown; constructor(ctx: unknown, name: string, opts: unknown) { this.ctx = ctx; this.name = name; this.opts = opts; } }

describe('PageAudio with a context that the browser keeps suspended until a gesture', () => {
  beforeEach(() => { vi.useFakeTimers(); vi.stubGlobal('AudioContext', FakeCtx); vi.stubGlobal('AudioWorkletNode', FakeNode); FakeCtx.last = null; FakeCtx.defaultResume = () => new Promise(() => {}); });
  afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });

  const startPending = async (a: PageAudio): Promise<FakeCtx> => {
    const p = a.start();
    await vi.advanceTimersByTimeAsync(0);
    await p;
    const ctx = FakeCtx.last as FakeCtx;
    return ctx;
  };

  it('a page that has not started its audio (no engine Worker yet, or none at all) says idle and has no context', () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    expect(a.statusKind()).toBe('idle');
    expect(a.detail()).toMatchObject({ status: 'idle', ctxState: null, nodeReady: false, sampleRate: null });
    expect(FakeCtx.last).toBeNull();
  });

  it('before the first gesture the status is needs-gesture; the first gesture resumes the context and the status becomes running', async () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    const ctx = await startPending(a);
    ctx.resumeImpl = () => new Promise(() => {});                   // the browser does not start it without a gesture
    expect(ctx.state).toBe('suspended');
    ctx.resumeImpl = () => new Promise(() => {});
    expect(a.statusKind()).toBe('needs-gesture');
    expect(a.detail().status).toBe('needs-gesture');
    expect(a.detail().ctxState).toBe('suspended');
    expect(a.detail().nodeReady).toBe(true);                         // addModule does not wait for the gesture in this fake (it does not in Chrome or Firefox 158)
    ctx.resumeImpl = async () => { ctx.set('running'); };            // after a gesture it does
    const r = await a.resume();
    expect(r).toMatchObject({ result: 'running', stateAfter: 'running', trigger: 'gesture' });
    expect(a.statusKind()).toBe('running');
    expect(a.detail()).toMatchObject({ status: 'running', nodeReady: true, ctxState: 'running', sampleRate: 44100, baseLatencyMs: 10.7, outputLatencyMs: 24, gestures: 1 });
    expect(a.state).toBe('running');
  });

  it('a resume() that does not settle is recorded as pending after RESUME_WAIT_MS and the status becomes blocked (not before)', async () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    const ctx = await startPending(a);
    ctx.resumeImpl = () => new Promise(() => {});
    const p = a.resume();
    await vi.advanceTimersByTimeAsync(RESUME_WAIT_MS - 100);
    expect(a.statusKind()).toBe('resuming');
    await vi.advanceTimersByTimeAsync(200);
    const r = await p;
    expect(r).toMatchObject({ result: 'pending', stateAfter: 'suspended' });
    expect(a.statusKind()).toBe('blocked');
    expect(a.detail()).toMatchObject({ status: 'blocked', gestures: 1, lastResume: { result: 'pending' } });
  });

  it('a resume() that throws is recorded as rejected with the error name; one that settles with the context still suspended is recorded as suspended', async () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    const ctx = await startPending(a);
    ctx.resumeImpl = () => Promise.reject(Object.assign(new Error('x'), { name: 'NotAllowedError' }));
    expect(await a.resume()).toMatchObject({ result: 'rejected', error: 'NotAllowedError' });
    ctx.resumeImpl = async () => {};
    expect(await a.resume()).toMatchObject({ result: 'suspended' });
    expect(a.detail().resumeAttempts).toBe(a.resumeAttempts);
  });

  it('a gesture while the context already runs does nothing (no resume() call, no record)', async () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    const ctx = await startPending(a);
    ctx.set('running');
    const spy = vi.spyOn(ctx, 'resume');
    const before = a.resumeAttempts;
    expect(await a.resume()).toBeNull();
    expect(spy).not.toHaveBeenCalled();
    expect(a.resumeAttempts).toBe(before);
  });

  it('start() asks the context to run once without a gesture and that call is not a gesture; a browser that allows autoplay runs it at once', async () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    const ctx = await startPending(a);
    expect(a.gestures).toBe(0);
    expect(a.resumeAttempts).toBe(1);
    expect(ctx.audioWorklet.addModule).toHaveBeenCalledWith('/w.js');
    FakeCtx.defaultResume = async (c) => { c.set('running'); };
    const b = new PageAudio({ workletUrl: '/w.js' });
    await startPending(b);
    expect(b.statusKind()).toBe('running');
    expect(b.gestures).toBe(0);
  });

  it('while the worklet does not run, the page\'s timer moves the engine\'s play cursor (the song advances), and stops doing so when the worklet processes quanta', async () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    await startPending(a);
    Atomics.store(a.header, W.STATE, 2);                             // the engine started its DirectSound buffer
    await vi.advanceTimersByTimeAsync(1000);
    const rd = Atomics.load(a.header, W.RD) >>> 0;
    expect(rd).toBeGreaterThan(39000);                               // about 1 s of frames at 44,100 Hz (less the ticks before the state was seen)
    expect(rd).toBeLessThan(45000);
    expect(a.detail().softSinkFrames).toBe(rd);
    Atomics.add(a.header, W.Q_TOTAL, 1);
    await vi.advanceTimersByTimeAsync(10);
    Atomics.add(a.header, W.Q_TOTAL, 1);
    await vi.advanceTimersByTimeAsync(10);
    const rd2 = Atomics.load(a.header, W.RD) >>> 0;
    Atomics.add(a.header, W.Q_TOTAL, 1);
    await vi.advanceTimersByTimeAsync(10);
    expect(Atomics.load(a.header, W.RD) >>> 0).toBe(rd2);
    a.close();
  });

  it('an error while starting is a status of its own and is announced to the listeners', async () => {
    vi.stubGlobal('AudioContext', class { constructor() { throw new Error('no device'); } });
    const a = new PageAudio({ workletUrl: '/w.js' });
    let n = 0;
    a.onChange(() => { n++; });
    await a.start();
    expect(a.statusKind()).toBe('error');
    expect(a.detail().error).toMatch(/no device/);
    expect(n).toBeGreaterThan(0);
  });
});

describe('what the page keeps for diagnosis is bounded (the audio output is on in every session)', () => {
  const watched = (): { a: PageAudio; send: (d: unknown) => void } => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    let h: (ev: MessageEvent) => void = () => {};
    a.watch({ addEventListener: (_t: string, f: (ev: MessageEvent) => void) => { h = f; } } as unknown as Worker);
    return { a, send: (d) => { h({ data: d } as MessageEvent); } };
  };
  it('pulses, reports and unlock times keep their newest entries only', () => {
    const { a, send } = watched();
    for (let i = 0; i < KEEP_PULSES + 50; i++) send({ type: 'audio-pulse', real: i, epoch: 0, virt: 0, consumed: 0, written: 0, threads: [], mixer: -1, unlockCompute: { n: 0, sumMs: 0, maxMs: 0, over78: 0 } });
    for (let i = 0; i < KEEP_REPORTS + 50; i++) send({ type: 'aud2-worker', at: i, unlocks: [{ first: i, n: 4096, t: i }, { first: i, n: 4096, t: i }], waits: 0, publishes: 0, skipped: 0, gapFrames: i, maxWaitDrift: 0, pace: null });
    expect(a.pulses.length).toBe(KEEP_PULSES);
    expect(a.pulses[a.pulses.length - 1]?.real).toBe(KEEP_PULSES + 49);
    expect(a.pulses[0]?.real).toBe(50);
    expect(a.reports.length).toBe(KEEP_REPORTS);
    expect(a.workerUnlocks.length).toBe(KEEP_UNLOCKS);
    expect(a.workerUnlocks[a.workerUnlocks.length - 1]?.first).toBe(KEEP_REPORTS + 49);
  });
  it('the engine\'s own loss (frames of silence it wrote for a hole, frames it dropped) is in the detail: the worklet does not count a hole that the engine has filled before the output reached it', () => {
    const { a, send } = watched();
    expect(a.detail().engineGapFrames).toBeNull();
    send({ type: 'aud2-worker', at: 1, unlocks: [], waits: 0, publishes: 10, skipped: 3, gapFrames: 1000, maxWaitDrift: 0, pace: null });
    send({ type: 'aud2-worker', at: 2, unlocks: [], waits: 0, publishes: 20, skipped: 7, gapFrames: 4410, maxWaitDrift: 0, pace: null });
    expect(a.detail()).toMatchObject({ engineGapFrames: 4410, engineSkippedFrames: 7, engineGapFramesLast60s: 3410, underrunQuanta: 0, engineLateWrites: null, engineLateWriteApplied: null, engineLateWriteLost: null });
  });
  it('decision 0241: the late writes of the guest (the engine\'s counters fle_audio_rewrite_stat, in the report) are in the detail: events, frames handed over again in time, frames too near to the output to be changed', () => {
    const { a, send } = watched();
    send({ type: 'aud2-worker', at: 1, unlocks: [], waits: 0, publishes: 10, skipped: 0, gapFrames: 0, maxWaitDrift: 0, pace: null, rewrite: { events: 1, applied: 640, lost: 0 } });
    send({ type: 'aud2-worker', at: 2, unlocks: [], waits: 0, publishes: 20, skipped: 0, gapFrames: 0, maxWaitDrift: 0, pace: null, rewrite: { events: 3, applied: 1700, lost: 256 } });
    expect(a.detail()).toMatchObject({ engineLateWrites: 3, engineLateWriteApplied: 1700, engineLateWriteLost: 256 });
  });
});

describe('the beacon carries the audio status and the schema check knows what cannot be true', () => {
  const detail = { status: 'running', nodeReady: true, ctxState: 'running', sampleRate: 44100, baseLatencyMs: 10.7, outputLatencyMs: 24, autoplayPolicy: null, gestures: 2, resumeAttempts: 2, lastResume: { at: 1000, trigger: 'gesture', result: 'running', ms: 11.2, stateAfter: 'running' }, softSinkFrames: 0, capFrames: 32768, error: null, engineGapFrames: 0, engineSkippedFrames: 0, engineGapFramesLast60s: 0, engineLateWrites: 0, engineLateWriteApplied: 0, engineLateWriteLost: 0,
    deviceState: 'playing', quantaWhileOpen: 10, quantaWhilePlaying: 1000, playingSeconds: 2.9, underrunQuanta: 4, nonSilentQuanta: 100, fillFrames: 2646, fillMinFrames: 200 };
  const problems = (d: object | undefined, u: number | null = 4): string[] => beaconProblems({ v: 3, sid: 'a1b2c3d4e5f6', audio: { underruns: u, ...(d ? { detail: d } : {}) }, input: undefined }).filter((p) => p.startsWith('audio'));

  it('a consistent detail has no problem; a v3 beacon without a detail, or with the detail of decision 0175 (no status), has none either', () => {
    expect(problems(detail)).toEqual([]);
    expect(problems(undefined, null)).toEqual([]);
    expect(problems({ episodes: 1, underrunQuanta: 3 })).toEqual([]);
  });

  it('finds a status that the other fields contradict', () => {
    expect(problems({ ...detail, status: 'sleeping' }).join()).toMatch(/status/);
    expect(problems({ ...detail, ctxState: 'suspended' }).join()).toMatch(/running but the context is not running/);
    expect(problems({ ...detail, engineGapFrames: 10, engineGapFramesLast60s: 20 }).join()).toMatch(/more gap frames in the last minute/);
    expect(problems({ ...detail, engineGapFrames: -1 }).join()).toMatch(/engineGapFrames/);
    expect(problems({ ...detail, engineLateWriteLost: -1 }).join()).toMatch(/engineLateWriteLost/);
    expect(problems({ ...detail, engineGapFrames: null, engineSkippedFrames: null, engineGapFramesLast60s: null })).toEqual([]);
    expect(problems({ ...detail, status: 'idle' }).join()).toMatch(/idle but a context or a node exists/);
    expect(problems({ ...detail, status: 'idle', ctxState: null, nodeReady: false, sampleRate: null, baseLatencyMs: null, outputLatencyMs: null })).toEqual([]);
    expect(problems({ ...detail, nodeReady: false }).join()).toMatch(/worklet node does not exist/);
    expect(problems({ ...detail, nodeReady: undefined }).join()).toMatch(/nodeReady/);
    expect(problems({ ...detail, status: 'needs-gesture', ctxState: 'suspended' }).join()).toMatch(/needs-gesture/);
    expect(problems({ ...detail, status: 'blocked', ctxState: 'suspended', gestures: 0 }).join()).toMatch(/blocked without a gesture/);
    expect(problems({ ...detail, ctxState: 'playing' }).join()).toMatch(/ctxState/);
    expect(problems({ ...detail, sampleRate: 12 }).join()).toMatch(/sampleRate/);
    expect(problems({ ...detail, lastResume: { ...detail.lastResume, result: 'maybe' } }).join()).toMatch(/lastResume.result/);
    expect(problems({ ...detail, resumeAttempts: 0 }).join()).toMatch(/without any attempt/);
    expect(problems({ ...detail, underrunQuanta: 5000 }).join()).toMatch(/more underrun quanta/);
    expect(problems({ ...detail, nonSilentQuanta: 5000 }).join()).toMatch(/more non-silent quanta/);
  });

  it('buildBeacon passes the detail through unchanged', () => {
    const b = buildBeacon({ seq: 1, sid: 'a1b2c3d4e5f6', reason: 'periodic', pageMs: 5000, visibility: 'visible', browser: { userAgent: 'x' } as never, bundle: { name: null, version: null, engineKind: null }, present: 'A', state: 'idle', exit: null, error: null, timeline: {} as never, updates: { total: 0, recent10s: { n: 0, intervalMedianMs: null, intervalP95Ms: null, perSecond: null } } as never, idles: {} as never, input: null as never, underruns: 4, audioDetail: detail, wasmMemoryBytes: null, client: null as never, display: null, hz: null } as never);
    expect(b.audio).toEqual({ underruns: 4, detail });
  });
});
