import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { DATA_OFFSET, makeRingBuffer, readCounters, W, ringBytes, STATE_OPEN, STATE_PLAYING } from '../../src/audio/ring-layout.ts';
import { UnderrunStats } from '../../src/audio/underrun.ts';

// The real AudioWorkletProcessor source (src/audio/fle-out.worklet.js) is evaluated here with a minimal AudioWorkletGlobalScope, and process() is driven with a scripted ring. Nothing is re-implemented in the test:
// what it checks is what the worklet counts.
const SRC = fs.readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), '../../src/audio/fle-out.worklet.js'), 'utf8');
const CAP = 4096, RATE = 44100;

interface Proc { process(inputs: unknown[], outputs: Float32Array[][]): boolean; port: { posted: Record<string, number>[] } }
function makeProc(): { proc: Proc; sab: SharedArrayBuffer; h: Int32Array; d: Float32Array; scope: { currentFrame: number; currentTime: number } } {
  const sab = makeRingBuffer(CAP, RATE);
  const scope = { currentFrame: 0, currentTime: 0 };
  class Base { port = { posted: [] as Record<string, number>[], postMessage(m: Record<string, number>) { this.posted.push(m); } }; }
  let Registered: new (o: unknown) => Proc = undefined as never;
  // currentFrame / currentTime are globals of the worklet scope: provide them as getters on a function scope object through `with`-free code by wrapping the source
  const wrapped = new Function('AudioWorkletProcessor', 'registerProcessor', 'sampleRate', 'scope', `
    Object.defineProperty(globalThis, 'currentFrame', { get: () => scope.currentFrame, configurable: true });
    Object.defineProperty(globalThis, 'currentTime', { get: () => scope.currentTime, configurable: true });
    ${SRC}`);
  wrapped(Base, (_name: string, c: new (o: unknown) => Proc) => { Registered = c; }, RATE, scope);
  const proc = new Registered({ processorOptions: { sab, dataOffset: DATA_OFFSET, cap: CAP } });
  return { proc, sab, h: new Int32Array(sab, 0, 32), d: new Float32Array(sab, DATA_OFFSET, CAP * 2), scope };
}

/** one quantum of 128 frames; the clock advances like the audio thread's */
function quantum(t: ReturnType<typeof makeProc>): { L: Float32Array; R: Float32Array } {
  const L = new Float32Array(128), R = new Float32Array(128);
  t.proc.process([], [[L, R]]);
  t.scope.currentFrame += 128;
  t.scope.currentTime = t.scope.currentFrame / RATE;
  return { L, R };
}
/** the engine publishes `n` frames starting at the current WR, with value v (L) and -v (R) */
function publish(t: ReturnType<typeof makeProc>, n: number, v: number): void {
  const wr = Atomics.load(t.h, W.WR) >>> 0;
  for (let i = 0; i < n; i++) { const p = ((wr + i) & (CAP - 1)) * 2; t.d[p] = v; t.d[p + 1] = -v; }
  Atomics.store(t.h, W.WR, (wr + n) | 0);
}
const set = (t: ReturnType<typeof makeProc>, state: number): void => { Atomics.store(t.h, W.STATE, state); };
const stats = (t: ReturnType<typeof makeProc>): UnderrunStats => new UnderrunStats(readCounters(t.h), { peak: 0, rms: 0 });

describe('the AudioWorklet counts what the output lacked (decision 0175)', () => {
  it('ring layout: the size of the ring and the header words the engine\'s own JavaScript uses (RD 0, WR 1, STATE 2) are where decision 0139 put them', () => {
    expect(ringBytes(CAP)).toBe(256 + CAP * 8);
    expect([W.RD, W.WR, W.STATE, W.RATE, W.CAP, W.MAGIC, W.VERSION]).toEqual([0, 1, 2, 3, 4, 5, 6]);
    expect([W.Q_TOTAL, W.Q_PLAYING, W.UR_QUANTA, W.UR_FRAMES, W.FILL_MIN, W.FILL_MAX, W.PUBLISHES, W.PUBLISHED_FRAMES, W.SKIPPED_FRAMES]).toEqual([8, 9, 10, 11, 12, 13, 14, 15, 16]);
    expect(() => makeRingBuffer(1000, RATE)).toThrow();
  });

  it('while the device is not playing the output is silence, RD does not move and nothing is counted as an underrun', () => {
    const t = makeProc();
    set(t, STATE_OPEN);
    for (let i = 0; i < 10; i++) quantum(t);
    const c = readCounters(t.h);
    expect(c.quanta).toBe(10);
    expect(c.playing).toBe(0);
    expect(c.urQuanta).toBe(0);
    expect(c.rd).toBe(0);
  });

  it('a fully fed playing stream has 0 underrun quanta, plays the published frames as they are and counts the non-silent quanta', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    publish(t, 1024, 0.25);
    for (let i = 0; i < 8; i++) {
      const o = quantum(t);
      expect(o.L[0]).toBe(0.25);
      expect(o.R[127]).toBe(-0.25);
    }
    const s = stats(t);
    expect(s.c.playing).toBe(8);
    expect(s.total).toBe(0);
    expect(s.p3).toBe(0);
    expect(s.c.nonSilent).toBe(8);
    expect(s.c.rd).toBe(1024);
    expect(s.c.warm).toBe(true);
  });

  it('a published block of exactly one quantum is complete; one frame less is an underrun quantum with 1 missing frame (the rest of the quantum is silence)', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    publish(t, 128, 0.5);
    quantum(t);
    expect(readCounters(t.h).urQuanta).toBe(0);
    publish(t, 127, 0.5);
    const o = quantum(t);
    const c = readCounters(t.h);
    expect(c.urQuanta).toBe(1);
    expect(c.urFrames).toBe(1);
    expect(o.L[126]).toBe(0.5);
    expect(o.L[127]).toBe(0);
    expect(c.urSteady).toBe(1);   // warm after the first complete quantum
  });

  it('the output is a timeline: RD advances by 128 on an underrun too, and frames published late for a passed position are not played', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    for (let i = 0; i < 5; i++) quantum(t);   // nothing published: 5 underruns, all warm-up
    expect(readCounters(t.h).rd).toBe(640);
    const c = readCounters(t.h);
    expect(c.urQuanta).toBe(5);
    expect(c.urStartup).toBe(5);
    expect(c.urSteady).toBe(0);
    expect(c.warm).toBe(false);
    expect(c.urRuns).toBe(1);
    expect(c.urMaxRun).toBe(5);
    expect(c.urCurRun).toBe(5);
  });

  it('warm-up: the underruns before the first complete quantum are counted apart; after it every underrun is a steady one', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    quantum(t); quantum(t);                    // 2 startup underruns
    publish(t, 256, 0.1);
    quantum(t);                                // here WR = 256 and RD = 256: the engine published for frames the output has passed -> underrun too
    const c1 = readCounters(t.h);
    expect(c1.urStartup).toBe(3);
    // the engine catches up: it publishes ahead of the output timeline
    publish(t, 1024, 0.1);
    quantum(t);
    expect(readCounters(t.h).warm).toBe(true);
    expect(readCounters(t.h).urSteady).toBe(0);
    expect(readCounters(t.h).urFirstAfterWarm).toBe(0);
  });

  it('episodes: a maximal run of consecutive underrun quanta is one episode, the longest run is kept, and the run that is going on is visible', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    publish(t, 4000, 0.1);
    for (let i = 0; i < 6; i++) quantum(t);   // 6 complete
    // drain: no more frames; then a run of 3 underruns
    const rd = Atomics.load(t.h, W.RD) >>> 0;
    Atomics.store(t.h, W.WR, rd | 0);
    for (let i = 0; i < 3; i++) quantum(t);
    let c = readCounters(t.h);
    expect([c.urQuanta, c.urRuns, c.urMaxRun, c.urCurRun, c.urSteady, c.urSteadyRuns, c.urSteadyMaxRun]).toEqual([3, 1, 3, 3, 3, 1, 3]);
    expect(c.urFirstAfterWarm).toBe(7);        // the 7th quantum of the playing state
    publish(t, 4000, 0.1);
    for (let i = 0; i < 4; i++) quantum(t);
    c = readCounters(t.h);
    expect(c.urCurRun).toBe(0);
    // a second, shorter run
    Atomics.store(t.h, W.WR, (Atomics.load(t.h, W.RD) >>> 0) | 0);
    quantum(t); quantum(t);
    publish(t, 4000, 0.1);
    quantum(t);
    c = readCounters(t.h);
    expect([c.urQuanta, c.urRuns, c.urMaxRun, c.urSteadyRuns, c.urSteadyMaxRun]).toEqual([5, 2, 3, 2, 3]);
  });

  it('silent and non-silent quanta are told apart, and the level posted to the page covers the output since the last post', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    publish(t, 128, 0);
    quantum(t);
    publish(t, 128, 0.5);
    quantum(t);
    const c = readCounters(t.h);
    expect(c.playing).toBe(2);
    expect(c.nonSilent).toBe(1);
    expect(stats(t).silentQuanta).toBe(1);
    const posted = t.proc.port.posted;
    expect(posted.length).toBeGreaterThanOrEqual(1);
    const last = posted[posted.length - 1] as Record<string, number>;
    expect(last.state).toBe(STATE_PLAYING);
    // the first post happens at frame 0 (before any output of that quantum is counted in the next post): the level is in the later posts
    t.scope.currentFrame += RATE / 20;
    quantum(t);
    const p2 = posted[posted.length - 1] as Record<string, number>;
    expect(p2.peak).toBe(0.5);
    expect(p2.n).toBeGreaterThanOrEqual(128);
    expect(p2.sumSq).toBeCloseTo(2 * 0.25 * 128, 6);   // L = 0.5 and R = -0.5 over 128 frames
  });

  it('FILL_MIN is negative when the engine is behind the output timeline', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    for (let i = 0; i < 3; i++) quantum(t);
    expect(readCounters(t.h).fillMin).toBe(-256);   // the signed fill at the three quanta: 0, -128, -256
    expect(readCounters(t.h).fillMax).toBe(0);
  });

  it('every post carries the smallest fill (the engine\'s margin) since the previous post, and the next post starts again', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    publish(t, 1000, 0.1);
    quantum(t);                       // post at frame 0: fill 1000
    publish(t, 400, 0.1);
    quantum(t);                       // fill 1272
    t.scope.currentFrame += RATE / 20;
    quantum(t);                       // fill 1144, and the post of the window
    const posted = t.proc.port.posted;
    const p2 = posted[posted.length - 1] as Record<string, number>;
    expect(p2.fmin).toBe(1144);
    t.scope.currentFrame += RATE / 20;
    quantum(t);
    const p3 = posted[posted.length - 1] as Record<string, number>;
    expect(p3.fmin).toBe(1016);
  });

  it('capture: on request the worklet copies N frames from the first non-zero sample on, once, and says whether every frame of them was there', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    const port = t.proc.port as unknown as { onmessage: (ev: { data: unknown }) => void; posted: Record<string, unknown>[] };
    port.onmessage({ data: { cmd: 'capture', frames: 200 } });
    publish(t, 128, 0);               // silence first
    quantum(t);
    publish(t, 64, 0);
    publish(t, 64, 0.5);              // the first 64 frames of this quantum are silent, then 0.5
    quantum(t);
    expect(port.posted.some((m) => m.capture)).toBe(false);   // 64 of 200 frames so far
    publish(t, 256, 0.25);
    quantum(t); quantum(t);
    const cap = port.posted.filter((m) => m.capture);
    expect(cap.length).toBe(1);
    const buf = cap[0]!.capture as Float32Array;
    expect(buf.length).toBe(400);
    expect(buf[0]).toBe(0.5);
    expect(buf[1]).toBe(-0.5);
    expect(buf[2 * 63]).toBe(0.5);
    expect(buf[2 * 64]).toBe(0.25);
    expect(cap[0]!.valid).toBe(true);
    expect(cap[0]!.validFrames).toBe(200);
    expect(cap[0]!.firstFrame).toBe(128 + 64);
    for (let i = 0; i < 4; i++) quantum(t);
    expect(port.posted.filter((m) => m.capture).length).toBe(1);   // once
  });

  it('capture: a missing frame inside the stretch makes it invalid', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    const port = t.proc.port as unknown as { onmessage: (ev: { data: unknown }) => void; posted: Record<string, unknown>[] };
    port.onmessage({ data: { cmd: 'capture', frames: 300 } });
    publish(t, 128, 0.5);
    quantum(t);
    quantum(t);                        // nothing published: an underrun inside the stretch
    publish(t, 1000, 0.5);
    quantum(t); quantum(t);
    const cap = port.posted.filter((m) => m.capture);
    expect(cap.length).toBe(1);
    expect(cap[0]!.valid).toBe(false);
    expect(cap[0]!.validFrames).toBe(128);   // the first quantum was complete; the frames after the missing ones are not aligned
  });

  it('capture with timeline (decision 0211): the frames the engine had not delivered are copied as silence, so the copy keeps the output\'s time axis; it is still marked invalid', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    const port = t.proc.port as unknown as { onmessage: (ev: { data: unknown }) => void; posted: Record<string, unknown>[] };
    port.onmessage({ data: { cmd: 'capture', frames: 512, timeline: true } });
    publish(t, 128, 0.5);
    quantum(t);                        // frames 0..127: sound
    quantum(t);                        // frames 128..255: missing (silence on the time axis)
    publish(t, 384, 0.25);             // the engine is late: positions 128..511, of which 128..255 have been played already
    quantum(t); quantum(t);            // frames 256..511: sound again
    const cap = port.posted.filter((m) => m.capture);
    expect(cap.length).toBe(1);
    const buf = cap[0]!.capture as Float32Array;
    expect(buf.length).toBe(1024);
    expect(buf[0]).toBe(0.5);
    expect(buf[2 * 127]).toBe(0.5);
    expect(buf[2 * 128]).toBe(0);      // the gap
    expect(buf[2 * 255]).toBe(0);
    expect(buf[2 * 256]).toBe(0.25);   // aligned with the time axis, not shifted into the gap
    expect(buf[2 * 511]).toBe(0.25);
    expect(cap[0]!.valid).toBe(false);
    expect(cap[0]!.validFrames).toBe(128);
  });

  it('UnderrunStats: seconds, share and the number P3 shows (the steady count)', () => {
    const t = makeProc();
    set(t, STATE_PLAYING);
    publish(t, 128 * 10, 0.1);
    for (let i = 0; i < 10; i++) quantum(t);
    for (let i = 0; i < 5; i++) quantum(t);
    const s = stats(t);
    expect(s.total).toBe(5);
    expect(s.p3).toBe(5);
    expect(s.share).toBeCloseTo(5 / 15, 9);
    expect(s.playingSeconds).toBeCloseTo((15 * 128) / RATE, 9);
    expect(s.steadyMissingSeconds).toBeCloseTo((5 * 128) / RATE, 9);
  });
});
