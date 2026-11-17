import { describe, expect, it } from 'vitest';
import { onAudioMessage } from '../../src/audio/worker-hook.ts';
import { AUDIO } from '../../src/audio/install.ts';
import { PageAudio } from '../../src/audio/page-audio.ts';
import { DATA_OFFSET, W } from '../../src/audio/ring-layout.ts';

// install.ts imports the worklet with ?url, which vitest's node environment resolves like vite does; the module is loaded for AUDIO only.
describe('audio wiring (decision 0175): the constants and the messages between the page, the Worker and the engine', () => {
  it('the ring and the rate are constants (decision 0243: no URL parameter): 44,100 Hz (the engine\'s device rate: any other rate would change the speed) and the engine\'s own 32,768-frame ring', () => {
    expect(AUDIO).toEqual({ sampleRate: 44100, capFrames: 32768 });
  });

  it('the page gives the Worker the ring (the init message carries the SharedArrayBuffer, its capacity and the data offset)', () => {
    const a = new PageAudio({ capFrames: 8192, workletUrl: '/w.js', workerReportMs: 500 });
    const m = a.initMessage();
    expect(m.cmd).toBe('audio-init');
    expect(m.sab).toBe(a.sab);
    expect(m.cap).toBe(8192);
    expect(m.dataOffset).toBe(DATA_OFFSET);
    expect(m.reportMs).toBe(500);
    expect('leadMs' in m).toBe(false);   // the lead is the engine's own constant (60 ms), not a message
  });

  it('the Worker hook sets globalThis.fleAudioSab for the engine\'s EM_JS and ignores every other message', () => {
    const scope: { fleAudioSab?: unknown; postMessage(m: unknown): void } = { postMessage() {} };
    const sab = new SharedArrayBuffer(DATA_OFFSET + 1024 * 8);
    expect(onAudioMessage({ cmd: 'init' }, scope as never)).toBe(false);
    expect(onAudioMessage({ cmd: 'start', sab, cap: 1024, dataOffset: DATA_OFFSET }, scope as never)).toBe(false);   // a SharedArrayBuffer in another command is not the ring
    expect(scope.fleAudioSab).toBeUndefined();
    expect(onAudioMessage(null, scope as never)).toBe(false);
    expect(onAudioMessage({ cmd: 'audio-init', sab: new ArrayBuffer(8) }, scope as never)).toBe(false);   // not shared
    expect(scope.fleAudioSab).toBeUndefined();
    expect(onAudioMessage({ cmd: 'audio-init', sab, cap: 1024, dataOffset: DATA_OFFSET }, scope as never)).toBe(true);
    expect(scope.fleAudioSab).toEqual({ sab, cap: 1024, dataOffset: DATA_OFFSET });
    const noLead: { fleAudioSab?: unknown; postMessage(m: unknown): void } = { postMessage() {} };
    onAudioMessage({ cmd: 'audio-init', sab, cap: 1024, dataOffset: DATA_OFFSET }, noLead as never);
    expect(noLead.fleAudioSab).toEqual({ sab, cap: 1024, dataOffset: DATA_OFFSET });
  });

  it('the page records the engine\'s pulses and the Worker\'s reports from the Worker\'s messages and nothing else', () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    let handler: (ev: MessageEvent) => void = () => {};
    const worker = { addEventListener: (_t: string, f: (ev: MessageEvent) => void) => { handler = f; } } as unknown as Worker;
    a.watch(worker);
    handler({ data: { t: 'idle', n: 1 } } as MessageEvent);
    handler({ data: 'text' } as MessageEvent);
    handler({ data: { type: 'audio-pulse', real: 1, epoch: 2, virt: 3, consumed: 4, written: 5, threads: [], mixer: -1, unlockCompute: { n: 0, sumMs: 0, maxMs: 0, over78: 0 } } } as MessageEvent);
    handler({ data: { type: 'aud2-worker', at: 1, unlocks: [{ first: 0, n: 4096, t: 7 }], waits: 0, publishes: 1, skipped: 0, gapFrames: 0, maxWaitDrift: 0, pace: null } } as MessageEvent);
    expect(a.pulses.length).toBe(1);
    expect(a.reports.length).toBe(1);
    expect(a.workerUnlocks).toEqual([{ first: 0, n: 4096, t: 7 }]);
  });

  it('the counters read from the ring are the worklet\'s: a fresh ring has no underruns and 0 playing quanta, P3 is 0 and the share is 0', () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    const u = a.underruns();
    expect(u.p3).toBe(0);
    expect(u.c.playing).toBe(0);
    expect(u.share).toBe(0);
    expect(u.level).toEqual({ peak: 0, rms: 0 });
    expect(a.fill()).toBe(0);
  });

  it('detail(): the counts per state of the device, since the first idle and over the last minute; the guest\'s transport is not visible to the page', () => {
    const a = new PageAudio({ workletUrl: '/w.js' });
    let handler: (ev: MessageEvent) => void = () => {};
    a.watch({ addEventListener: (_t: string, f: (ev: MessageEvent) => void) => { handler = f; } } as unknown as Worker);
    const h = a.header;
    const put = (i: number, v: number): void => { Atomics.store(h, i, v); };
    put(W.STATE, 2); put(W.Q_TOTAL, 1000); put(W.Q_PLAYING, 800); put(W.UR_QUANTA, 700); put(W.UR_RUNS, 30); put(W.UR_MAXRUN, 90); put(W.NONSILENT_Q, 0);
    handler({ data: { t: 'idle', n: 1 } } as MessageEvent);          // the first idle: the mark
    expect(a.marks['first-idle']?.c.urQuanta).toBe(700);
    put(W.UR_QUANTA, 700 + 5); put(W.Q_PLAYING, 900); put(W.Q_TOTAL, 1100); put(W.NONSILENT_Q, 40);
    const d = a.detail(performance.now());
    expect(d.deviceState).toBe('playing');
    expect(d.underrunQuanta).toBe(705);
    expect(d.underrunQuantaSinceFirstIdle).toBe(5);
    expect(d.underrunQuantaSinceFirstSound).toBeNull();   // no non-silent quantum yet
    expect(d.quantaWhileOpen).toBe(200);
    expect(d.quantaWhilePlaying).toBe(900);
    expect(d.episodes).toBe(30);
    expect(d.longestRunQuanta).toBe(90);
    expect(d.nonSilentQuanta).toBe(40);
    expect(d.playingSeconds).toBeCloseTo((900 * 128) / 44100, 6);
    (a as unknown as { onSample(x: object): void }).onSample({ frame: 0, time: 0, rd: 0, wr: 0, state: 2, peak: 0.5, sumSq: 1, n: 128, nonSilent: 1, fmin: 100 });
    put(W.UR_QUANTA, 700 + 9);
    expect(a.detail().underrunQuantaSinceFirstSound).toBe(4);   // the mark was made when the first non-silent post arrived (705 underruns by then)
    expect(a.marks['first-sound']?.c.urQuanta).toBe(705);
    handler({ data: { t: 'idle', n: 2 } } as MessageEvent);          // only the first idle makes the mark
    expect(a.marks['first-idle']?.c.urQuanta).toBe(700);
  });
});
