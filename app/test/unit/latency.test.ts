import { describe, expect, it } from 'vitest';
import { IN, type RawInput } from '../../src/input/events.ts';
import { engineState, meanCycle, type EngineClockState } from '../../src/ui/engine-status.ts';
import { EVENT_STAMP_MAX_AGE_MS, framesAreChangesFor, LatencyTracker, percentile, RAF_MAX_WAIT_MS, stagesOf, toAnyFrame, toEventRaf, toFrame, toLateRaf, toProcessed, type LatencyRecord } from '../../src/ui/latency.ts';

const mv: RawInput = { k: 'mouse', kind: IN.MOVE, x: 1, y: 2, data: 0 };

describe('LatencyTracker', () => {
  it('page -> injection -> engine idle again -> first frame of each window posted in the processing window (decision 0205: not one posted after the idle)', () => {
    const t = new LatencyTracker();
    t.sent(1, 1000, mv);
    expect(t.pending).toBe(1);
    t.frame(13, 1420, 1380);        // posted by the engine's idle (1400): the answer of window 13
    t.frame(48, 1450, 1399);        // window 48, posted just before the idle
    t.frame(13, 1900, 1890);        // a later frame of window 13, posted long after the idle: not an answer
    t.done([{ seq: 1, tPage: 1000, tInject: 1030, tIdle: 1400 }]);
    expect(t.pending).toBe(0);
    const r = t.records[0]!;
    expect(toProcessed(r)).toBe(400);
    expect(toFrame(48)(r)).toBe(450);
    expect(toFrame(13)(r)).toBe(420);
    expect(toAnyFrame(r)).toBe(420);
    expect(toFrame(99)(r)).toBeNull();
    expect(r.outcome).toBe('window');
  });
  it('the answer that reaches the page BEFORE the event\'s input-done (decision 0170) is credited when input-done arrives: only frames posted at or after the injection count', () => {
    const t = new LatencyTracker();
    t.sent(1, 1000, mv);
    t.frame(13, 1005, 1002);        // posted before the injection (1010), received after the event was sent: not an answer
    t.frame(13, 1040, 1030);        // posted after the injection, received before input-done: the answer
    t.frame(13, 1900, 1890);        // a later, unrelated frame
    t.done([{ seq: 1, tPage: 1000, tInject: 1010, tIdle: 1031 }]);
    const r = t.records[0]!;
    expect(toFrame(13)(r)).toBe(40);
    expect(toAnyFrame(r)).toBe(40);
  });
  it('...also when it reaches the page before input-done: a frame posted before the injection but received after it is not credited, the first one posted after it is', () => {
    const t = new LatencyTracker();
    t.sent(1, 1000, mv);
    t.frame(13, 1020, 1005);        // received at 1020, after the injection (1010), but posted at 1005, before it
    t.frame(13, 1060, 1050);
    t.done([{ seq: 1, tPage: 1000, tInject: 1010, tIdle: 1051 }]);
    expect(toAnyFrame(t.records[0]!)).toBe(60);
  });
  it('a frame received after the injection but posted before it is not the answer (the page saw it late)', () => {
    const t = new LatencyTracker(400, { now: () => 1060 });
    t.sent(1, 1000, mv);
    t.done([{ seq: 1, tPage: 1000, tInject: 1010, tIdle: 1031 }]);
    t.frame(13, 1020, 1005);
    expect(toAnyFrame(t.records[0]!)).toBeNull();
    t.frame(13, 1060, 1030.5);      // posted after the injection and within the margin of the idle (1031 + 1 ms), received after input-done: the answer
    expect(toAnyFrame(t.records[0]!)).toBe(60);
  });
  it('a frame drawn before the injection is not the answer to the event', () => {
    const t = new LatencyTracker();
    t.sent(1, 1000, mv);
    t.done([{ seq: 1, tPage: 1000, tInject: 1100, tIdle: 1500 }]);
    t.frame(13, 1050);
    expect(toAnyFrame(t.records[0]!)).toBeNull();
  });
  it('a frame much later than the event is not counted for it', () => {
    const t = new LatencyTracker();
    t.sent(1, 0, mv);
    t.done([{ seq: 1, tPage: 0, tInject: 10, tIdle: 20 }]);
    t.frame(13, 20_000);
    expect(toAnyFrame(t.records[0]!)).toBeNull();
  });
  it('pending counts what is neither processed nor dropped; dropped events are left out of the summaries', () => {
    const t = new LatencyTracker();
    for (let i = 1; i <= 5; i++) t.sent(i, i * 10, mv);
    t.done([{ seq: 1, tPage: 10, tInject: 11, tIdle: 111 }, { seq: 2, tPage: 20, tInject: 21, tIdle: 121 }]);
    t.dropped(3, 'merged into a later move');
    expect(t.pending).toBe(2);
    expect(t.droppedTotal).toBe(1);
    expect(t.summary(toProcessed).n).toBe(2);
    t.dropped(1, 'late'); // already processed: cannot be dropped any more
    expect(t.droppedTotal).toBe(1);
  });
  it('p50 / p95 / max', () => {
    const t = new LatencyTracker();
    for (let i = 1; i <= 20; i++) { t.sent(i, 0, mv); t.done([{ seq: i, tPage: 0, tInject: 1, tIdle: i * 100 }]); }
    expect(t.summary(toProcessed)).toEqual({ n: 20, p50: 1000, p95: 1900, max: 2000 });
    expect(t.summary(toProcessed, (r) => r.seq > 10).p50).toBe(1500);
  });
  it('percentile of nothing is null', () => { expect(percentile([], 50)).toBeNull(); expect(percentile([5], 95)).toBe(5); });
  it('keeps only the last N records', () => {
    const t = new LatencyTracker(3);
    for (let i = 1; i <= 5; i++) t.sent(i, i, mv);
    expect(t.records.map((r) => r.seq)).toEqual([3, 4, 5]);
  });
});

// Decision 0202: the P2 number starts at the DOM event's own timeStamp and ends at the first animation frame after the answer was drawn
function rig(rafAvailable = true) {
  const queue: (() => void)[] = [];
  let now = 0;
  const t = new LatencyTracker(400, { raf: rafAvailable ? (cb) => { queue.push(cb); } : null, now: () => now });
  const frameStarts = (at: number): void => { now = at; for (const cb of queue.splice(0)) cb(); };
  return { t, queue, frameStarts };
}

describe('LatencyTracker: event.timeStamp -> the next animation frame after the answer was drawn (decision 0202)', () => {
  it('the start is the event\'s timeStamp, not the listener; the end is the rAF callback after the putImageData, not the putImageData; the old number stays beside it, unchanged', () => {
    const { t, frameStarts } = rig();
    t.sent(1, 1010, mv, 'pointermove', { tEvent: 1000, tListener: 1009 });
    t.done([{ seq: 1, tPage: 1010, tInject: 1012, tIdle: 1031 }]);
    t.frame(13, 1030, 1020, 1025);
    const r = t.records[0]!;
    expect(toEventRaf(r)).toBeNull();               // the animation frame has not started yet
    expect(toAnyFrame(r)).toBe(20);                 // version 2's number: tPage -> put
    frameStarts(1046);
    expect(toEventRaf(r)).toBe(46);                 // 1046 - 1000
    expect(toAnyFrame(r)).toBe(20);
    expect(stagesOf(r)).toEqual({ s0: 9, s0b: 1, toWorker: 2, worker: 8, fromWorker: 5, put: 5, raf: 16 });
    const st = stagesOf(r)!;
    expect(st.s0 + st.s0b + st.toWorker + st.worker + st.fromWorker + st.put + st.raf).toBe(46);
  });
  it('the answer that reaches the page BEFORE input-done (decision 0170) closes too: whether the animation frame came before or after input-done', () => {
    for (const rafFirst of [false, true]) {
      const { t, frameStarts } = rig();
      t.sent(1, 1010, mv, 'pointermove', { tEvent: 1000, tListener: 1009 });
      t.frame(13, 1030, 1020, 1025);                // posted after the injection (1012) but the page does not know it yet
      if (rafFirst) frameStarts(1046);
      t.done([{ seq: 1, tPage: 1010, tInject: 1012, tIdle: 1031 }]);
      if (!rafFirst) frameStarts(1046);
      expect(toEventRaf(t.records[0]!)).toBe(46);
    }
  });
  it('the answer is still the first frame POSTED at or after the injection (decision 0170): a frame posted before it, or a later unrelated one, does not take its place', () => {
    const { t, frameStarts } = rig();
    t.sent(1, 1010, mv, 'pointermove', { tEvent: 1000, tListener: 1009 });
    t.done([{ seq: 1, tPage: 1010, tInject: 1012, tIdle: 1031 }]);
    t.frame(13, 1020, 1005, 1018);                  // received after the injection, posted before it
    frameStarts(1030);
    expect(toEventRaf(t.records[0]!)).toBeNull();
    t.frame(13, 1040, 1030, 1035);                  // the answer
    frameStarts(1062);
    t.frame(13, 1900, 1890, 1895);                  // an unrelated frame much later
    frameStarts(1920);
    expect(toEventRaf(t.records[0]!)).toBe(62);
  });
  it('a record without an event timeStamp (blur, a test) has the old number only; so has one whose timeStamp cannot be on the page\'s clock (after its own listener, or long before it)', () => {
    const { t, frameStarts } = rig();
    t.sent(1, 1000, mv, 'test');
    t.sent(2, 1010, mv, 'pointermove', { tEvent: 1011, tListener: 1010 });
    t.sent(3, 1020 + EVENT_STAMP_MAX_AGE_MS + 1, mv, 'pointermove', { tEvent: 1020, tListener: 1020 + EVENT_STAMP_MAX_AGE_MS });
    expect(t.records.map((r) => r.tEvent)).toEqual([null, null, null]);
    t.frame(13, 1030, 1020, 1025);                  // posted in the processing window of both records (injected at 1001 and 1011, idle at 1025)
    t.done([{ seq: 1, tPage: 1000, tInject: 1001, tIdle: 1025 }, { seq: 2, tPage: 1010, tInject: 1011, tIdle: 1025 }]);
    frameStarts(1046);
    expect(t.records.map(toEventRaf)).toEqual([null, null, null]);
    expect(toAnyFrame(t.records[0]!)).toBe(30);
    expect(stagesOf(t.records[0]!)).toBeNull();
  });
  it('a record is never made to look faster than it was: a listener that started before the page record was made keeps its place, tListener never exceeds tPage', () => {
    const t = new LatencyTracker(400, { raf: null });
    t.sent(1, 1000, mv, 'pointermove', { tEvent: 990, tListener: 1004 });
    expect(t.records[0]!.tListener).toBe(1000);
  });
  it('the first window to be drawn decides: of two windows the earlier animation frame counts', () => {
    const { t, frameStarts } = rig();
    t.sent(1, 1010, mv, 'pointerdown', { tEvent: 1000, tListener: 1009 });
    t.done([{ seq: 1, tPage: 1010, tInject: 1012, tIdle: 1040.5 }]);
    t.frame(48, 1030, 1020, 1025);
    frameStarts(1046);
    t.frame(13, 1050, 1040, 1045);                  // another window, posted within the margin of the idle and drawn after the first animation frame
    frameStarts(1062);
    const r = t.records[0]!;
    expect(Object.keys(r.frames).sort()).toEqual(['13', '48']);
    expect(toEventRaf(r)).toBe(46);
    expect(stagesOf(r)!.raf).toBe(16);
  });
  it('one animation frame serves every frame drawn before it; none is asked for while no event is waiting (an idle page costs nothing)', () => {
    const { t, queue, frameStarts } = rig();
    t.frame(13, 100, 90, 95);                       // nothing sent: idle repaints
    t.frame(13, 116, 106, 111);
    expect(queue.length).toBe(0);
    t.sent(1, 1010, mv, 'pointermove', { tEvent: 1000, tListener: 1009 });
    t.frame(13, 1011, 1005, 1008);                  // an event is waiting: this frame may be its answer
    expect(queue.length).toBe(1);
    t.frame(13, 1012, 1006, 1009);
    expect(queue.length).toBe(1);
    t.done([{ seq: 1, tPage: 1010, tInject: 1010, tIdle: 1036 }]);
    expect(queue.length).toBe(1);
    frameStarts(1030);
    expect(queue.length).toBe(0);
    expect(toEventRaf(t.records[0]!)).toBeNull();   // those two frames were posted before the injection (1010): not its answer
    t.frame(13, 1040, 1035, 1038);                  // the answer (posted before the idle at 1036)
    expect(queue.length).toBe(1);
    frameStarts(1050);
    expect(toEventRaf(t.records[0]!)).toBe(50);
    t.frame(13, 1060, 1055, 1058);                  // nothing waits any more
    expect(queue.length).toBe(0);
  });
  it('an animation frame long after the draw (the tab was hidden) is not "the next frame": no eventToRaf', () => {
    const { t, frameStarts } = rig();
    t.sent(1, 1010, mv, 'keydown', { tEvent: 1000, tListener: 1009 });
    t.done([{ seq: 1, tPage: 1010, tInject: 1012, tIdle: 1031 }]);
    t.frame(13, 1030, 1020, 1025);
    frameStarts(1030 + RAF_MAX_WAIT_MS + 1);
    expect(toEventRaf(t.records[0]!)).toBeNull();
    expect(toAnyFrame(t.records[0]!)).toBe(20);
  });
  it('without requestAnimationFrame there is no eventToRaf, and the old number is not disturbed', () => {
    const { t } = rig(false);
    t.sent(1, 1010, mv, 'pointermove', { tEvent: 1000, tListener: 1009 });
    t.done([{ seq: 1, tPage: 1010, tInject: 1012, tIdle: 1031 }]);
    t.frame(13, 1030, 1020, 1025);
    expect(toEventRaf(t.records[0]!)).toBeNull();
    expect(toAnyFrame(t.records[0]!)).toBe(20);
  });
  it('a dropped event has no answer to measure', () => {
    const { t, frameStarts } = rig();
    t.sent(1, 1010, mv, 'pointermove', { tEvent: 1000, tListener: 1009 });
    t.dropped(1, 'merged into a later move');
    t.frame(13, 1030, 1020, 1025);
    frameStarts(1046);
    expect(toEventRaf(t.records[0]!)).toBeNull();
  });
  it('uses requestAnimationFrame and the page clock when the page has them', () => {
    const g = globalThis as Record<string, unknown>;
    const had = g.requestAnimationFrame;
    const calls: (() => void)[] = [];
    g.requestAnimationFrame = (cb: () => void) => { calls.push(cb); return 1; };
    try {
      const t = new LatencyTracker();
      const o = performance.timeOrigin + performance.now();
      t.sent(1, o - 18, mv, 'pointermove', { tEvent: o - 20, tListener: o - 19 });
      t.done([{ seq: 1, tPage: o - 18, tInject: o - 17, tIdle: o - 9 }]);
      t.frame(13, o - 10, o - 12, o - 11);
      expect(calls.length).toBe(1);
      calls[0]!();
      const v = toEventRaf(t.records[0]!);
      expect(v).not.toBeNull();
      expect(v!).toBeGreaterThanOrEqual(20);
      expect(v!).toBeLessThan(20 + RAF_MAX_WAIT_MS);
    } finally { if (had === undefined) delete g.requestAnimationFrame; else g.requestAnimationFrame = had; }
  });
});

// Decision 0205: the answer is the frame posted between the injection and the engine's idle that ends the processing; an input with no such frame has no value and is counted
const down: RawInput = { k: 'mouse', kind: IN.BDOWN, x: 5, y: 6, data: 0 };
const up: RawInput = { k: 'mouse', kind: IN.BUP, x: 5, y: 6, data: 0 };
const key: RawInput = { k: 'key', kind: IN.KDOWN, vk: 0x20, scan: 0x39, ext: 0 };
function world() {
  let now = 0;
  const queue: (() => void)[] = [];
  const t = new LatencyTracker(400, { raf: (cb) => { queue.push(cb); }, now: () => now });
  return {
    t,
    /** the page clock moves to `at`; an animation frame that was asked for starts there */
    tick(at: number) { now = at; for (const cb of queue.splice(0)) cb(); },
    clockTo(at: number) { now = at; },
    /** an input: sent at tPage with the DOM event 10 ms before, taken by the engine at tInject, the engine idle again at tIdle (the Worker posts input-done after the frames of that idle) */
    send(seq: number, ev: RawInput, tPage: number) { t.sent(seq, tPage, ev, 'test', { tEvent: tPage - 10, tListener: tPage - 1 }); },
    done(seq: number, tPage: number, tInject: number, tIdle: number) { t.done([{ seq, tPage, tInject, tIdle }]); },
  };
}

describe('LatencyTracker: the answer is the frame of the processing window; an input with none has no value and is counted (decision 0205)', () => {
  it('a button released (nothing changes on the screen, so with ?dedupe=1 no frame is sent): not answered by the next unrelated frame 0.4 s later, no value, counted as having no answer', () => {
    const w = world();
    w.send(1, up, 1010);
    w.done(1, 1010, 1012, 1015);                    // the engine's idle at 1015: nothing was drawn for the release
    expect(w.t.records[0]!.outcome).toBeNull();     // the wait is not over
    expect(w.t.outcomes).toMatchObject({ answered: 0, late: 0, noAnswer: 0, awaiting: 1 });
    w.t.frame(13, 1395, 1385, 1390);                // an unrelated frame, posted 370 ms after the idle (the blinking caret, say)
    w.tick(1410);
    const r = w.t.records[0]!;
    expect(r.outcome).toBe('none');
    expect(r.frames).toEqual({});
    expect(toEventRaf(r)).toBeNull();
    expect(toAnyFrame(r)).toBeNull();
    expect(w.t.totals()).toEqual({ sent: 1, processed: 1, dropped: 0, pending: 0, answered: 0, late: 0, noAnswer: 1 });
  });
  it('...the same release with a frame in its window (the engine drew something): answered in the window, with a value', () => {
    const w = world();
    w.send(1, up, 1010);
    w.t.frame(13, 1020, 1014, 1018);                // posted by the idle that ends the processing, before input-done
    w.done(1, 1010, 1012, 1015);
    w.tick(1031);
    const r = w.t.records[0]!;
    expect(r.outcome).toBe('window');
    expect(toEventRaf(r)).toBe(1031 - 1000);        // tEvent = tPage - 10 = 1000
    expect(w.t.totals()).toMatchObject({ answered: 1, late: 0, noAnswer: 0 });
  });
  it('the window ends one millisecond after the idle: a frame posted at tIdle + 1 is the answer, one posted at tIdle + 1.5 is not', () => {
    for (const [postedAt, answered] of [[1016, true], [1016.5, false]] as const) {
      const w = world();
      w.send(1, mv, 1010);
      w.done(1, 1010, 1012, 1015);
      w.t.frame(13, 1020, postedAt, 1019);
      w.tick(1031);
      expect(toAnyFrame(w.t.records[0]!) !== null, String(postedAt)).toBe(answered);
    }
  });
  it('the window starts at the injection: a frame posted a moment before it is not the answer, and one input\'s frame is not another\'s', () => {
    const w = world();
    w.send(1, mv, 1010);
    w.send(2, mv, 1030);
    w.t.frame(13, 1025, 1019, 1023);                // the answer of input 1 (window 1012..1020)
    w.done(1, 1010, 1012, 1020);
    w.t.frame(13, 1045, 1040, 1043);                // the answer of input 2 (injected 1032, idle 1041)
    w.done(2, 1030, 1032, 1041);
    w.tick(1060);
    const [a, b] = w.t.records as [LatencyRecord, LatencyRecord];
    expect(a.answers[13]!.postedAt).toBe(1019);
    expect(b.answers[13]!.postedAt).toBe(1040);
    expect(w.t.totals()).toMatchObject({ answered: 2, late: 0, noAnswer: 0 });
  });
  it('an input whose first frame comes soon after the window (not held) is "late": counted apart, no value, and that frame is not its answer', () => {
    const w = world();
    w.send(1, mv, 1010);
    w.done(1, 1010, 1012, 1015);
    w.t.frame(13, 1085, 1070, 1075);                // 54 ms after the idle
    w.tick(1100);
    const r = w.t.records[0]!;
    expect(r.outcome).toBe('late');
    expect(toEventRaf(r)).toBeNull();
    expect(w.t.totals()).toMatchObject({ answered: 0, late: 1, noAnswer: 0 });
    w.t.frame(13, 1200, 1190, 1195);                // later frames change nothing
    expect(w.t.totals()).toMatchObject({ answered: 0, late: 1, noAnswer: 0 });
  });
  it('decision 0205: the page takes every frame for a visible change only with ?dedupe=1 (check mode sends them all, dedupe off sends the redraws); the tracker says what it was told', () => {
    expect(framesAreChangesFor(true)).toBe(true);
    expect(framesAreChangesFor(false)).toBe(false);
    expect(framesAreChangesFor('check')).toBe(false);
    expect(new LatencyTracker(400, { raf: null, framesAreChanges: true }).framesAreChanges).toBe(true);
    expect(new LatencyTracker(400, { raf: null }).framesAreChanges).toBe(false);
  });
  it('a late input keeps its first later frame apart: toLateRaf is that frame\'s animation frame, not the answer (toEventRaf stays null)', () => {
    const w = world();
    w.send(1, mv, 1010);
    w.done(1, 1010, 1012, 1015);
    w.t.frame(13, 1085, 1070, 1075);
    expect(toLateRaf(w.t.records[0]!)).toBeNull();   // its animation frame has not started
    w.tick(1100);
    const r = w.t.records[0]!;
    expect(r.outcome).toBe('late');
    expect(toLateRaf(r)).toBe(1100 - 1000);
    expect(toEventRaf(r)).toBeNull();
    const none = world();
    none.send(1, up, 1010);
    none.done(1, 1010, 1012, 1015);
    none.t.frame(13, 1395, 1385, 1390);
    none.tick(1410);
    expect(toLateRaf(none.t.records[0]!)).toBeNull(); // 'none': the frame is too far from the event to be mentioned
  });
  it('the first press of a knob: the engine is idle again 3 ms after the injection with nothing drawn, the change is drawn 45 ms later (a later idle): a press is held, so that frame is its answer (lat1\'s stream rule), 500 ms at most', () => {
    const w = world();
    w.send(1, down, 1010);
    w.done(1, 1010, 1012, 1015);
    expect(w.t.outcomes.awaiting).toBe(1);
    w.t.frame(13, 1067, 1057, 1062);                // injection + 45 ms
    w.tick(1081);
    const r = w.t.records[0]!;
    expect(r.held).toBe(true);
    expect(r.outcome).toBe('stream');
    expect(toEventRaf(r)).toBe(1081 - 1000);
    expect(stagesOf(r)!.worker).toBe(1057 - 1012);
    expect(w.t.totals()).toMatchObject({ answered: 1, late: 0, noAnswer: 0 });
  });
  it('a held input\'s later frame must be within 500 ms of its injection; a press that changed nothing is not answered by a frame 0.6 s later', () => {
    const w = world();
    w.send(1, down, 1010);
    w.done(1, 1010, 1012, 1015);
    w.t.frame(13, 1630, 1620, 1625);                // 608 ms after the injection
    w.tick(1650);
    expect(w.t.records[0]!.outcome).toBe('none');
    expect(toEventRaf(w.t.records[0]!)).toBeNull();
    expect(w.t.totals()).toMatchObject({ answered: 0, noAnswer: 1 });
  });
  it('held means a press, or a move while a button is down: a move before the press and a move after the release are not held, the moves in between are', () => {
    const w = world();
    w.send(1, mv, 1000);
    w.send(2, down, 1010);
    w.send(3, mv, 1020);
    w.send(4, { k: 'mouse', kind: IN.BDOWN, x: 5, y: 6, data: 1 }, 1030);
    w.send(5, up, 1040);                            // the left button is up, the right one is still down
    w.send(6, mv, 1050);
    w.send(7, { k: 'mouse', kind: IN.BUP, x: 5, y: 6, data: 1 }, 1060);
    w.send(8, mv, 1070);
    w.send(9, key, 1080);
    expect(w.t.records.map((r) => r.held)).toEqual([false, true, true, true, false, true, false, false, false]);
  });
  it('a waiting input is given up when its wait is over, even if no frame ever comes (the counters say so, the record has no value): 150 ms after the window, 500 ms after the injection when held', () => {
    const w = world();
    w.send(1, mv, 1010);
    w.send(2, down, 1010);
    w.done(1, 1010, 1012, 1015);
    w.done(2, 1010, 1012, 1015);
    w.clockTo(1100);
    expect(w.t.outcomes).toMatchObject({ noAnswer: 0, awaiting: 2 });
    w.clockTo(1167);                                // 1015 + 1 + 150 = 1166 passed
    expect(w.t.outcomes).toMatchObject({ noAnswer: 1, awaiting: 1 });
    w.clockTo(1513);                                // 1012 + 500 = 1512 passed
    expect(w.t.outcomes).toMatchObject({ noAnswer: 2, awaiting: 0 });
  });
  it('the counters add up and survive trimming of the records: processed = answered + late + noAnswer + awaiting; a trimmed input that was still waiting is counted at the trimming', () => {
    const t = new LatencyTracker(2, { raf: null, now: () => 1100 });   // no wait is over yet
    t.sent(1, 1000, mv); t.done([{ seq: 1, tPage: 1000, tInject: 1001, tIdle: 1005 }]);
    t.sent(2, 1010, mv); t.frame(13, 1020, 1008, 1019); t.done([{ seq: 2, tPage: 1010, tInject: 1011, tIdle: 1025 }]);
    t.sent(3, 1030, mv);
    expect(t.outcomes).toEqual({ answered: 0, late: 1, noAnswer: 0, awaiting: 1 });   // 1: the frame posted at 1008 came 2 ms after its window (late); 2: that frame was posted before its injection, it still waits
    t.sent(4, 1040, mv); t.done([{ seq: 4, tPage: 1040, tInject: 1041, tIdle: 1045 }]);
    expect(t.records.map((r) => r.seq)).toEqual([3, 4]);                                // 2 was trimmed while it waited
    expect(t.outcomes).toEqual({ answered: 0, late: 1, noAnswer: 1, awaiting: 1 });     // 2 is counted as unanswered, 4 waits, 3 was never processed
    const o = t.outcomes;
    expect(o.answered + o.late + o.noAnswer + o.awaiting).toBe(t.processedTotal);
    expect(t.totals()).toMatchObject({ sent: 4, processed: 3, pending: 1, answered: 0, late: 1, noAnswer: 1 });
  });
  it('a frame that reached the page before input-done but was posted after the window is not credited by done(): for an input that is not held it makes the input late, for a held one it is the answer (stream)', () => {
    for (const [ev, outcome] of [[mv, 'late'], [down, 'stream']] as const) {
      const w = world();
      w.send(1, ev, 1010);
      w.t.frame(13, 1085, 1070, 1075);              // 55 ms after the idle, received before input-done
      w.done(1, 1010, 1012, 1015);
      w.tick(1100);
      const r = w.t.records[0]!;
      expect(r.outcome).toBe(outcome);
      expect(Object.keys(r.answers).length).toBe(outcome === 'stream' ? 1 : 0);
      expect(toEventRaf(r) !== null).toBe(outcome === 'stream');
    }
  });
  it('dedupe off (an unchanged redraw is posted every 16 ms): every input has a frame in its window, a release too - the page cannot tell a redraw from an answer, which is why the counts of ?dedupe=1 are the ones that mean "no visible change"', () => {
    const w = world();
    w.send(1, up, 1010);
    w.t.frame(13, 1020, 1013, 1018);                // the unchanged redraw of the idle
    w.done(1, 1010, 1012, 1014);
    w.tick(1031);
    expect(w.t.records[0]!.outcome).toBe('window');
    expect(w.t.totals()).toMatchObject({ answered: 1, noAnswer: 0 });
  });
  it('inputs injected together share the idle: each one is answered by the first frame after its own injection', () => {
    const w = world();
    w.send(1, mv, 1010);
    w.send(2, mv, 1011);
    w.t.frame(13, 1030, 1024, 1028);
    w.t.done([{ seq: 1, tPage: 1010, tInject: 1012, tIdle: 1025 }, { seq: 2, tPage: 1011, tInject: 1013, tIdle: 1025 }]);
    w.tick(1046);
    expect(w.t.records.map((r) => r.outcome)).toEqual(['window', 'window']);
    expect(w.t.records.map(toEventRaf)).toEqual([1046 - 1000, 1046 - 1001]);
  });
  it('a dropped input is not counted as answered or unanswered', () => {
    const w = world();
    w.send(1, mv, 1010);
    w.t.dropped(1, 'merged into a later move');
    expect(w.t.totals()).toMatchObject({ sent: 1, dropped: 1, processed: 0, answered: 0, late: 0, noAnswer: 0 });
  });
  it('a frame that arrives after input-done but was posted before it asks for no animation frame when no input is waiting (an idle page costs nothing)', () => {
    const w = world();
    const q: (() => void)[] = [];
    const t = new LatencyTracker(400, { raf: (cb) => { q.push(cb); }, now: () => 5000 });
    t.sent(1, 1010, up, 'test', { tEvent: 1000, tListener: 1009 });
    t.done([{ seq: 1, tPage: 1010, tInject: 1012, tIdle: 1015 }]);
    t.frame(13, 5000, 4990, 4995);                  // nothing in its window and the wait is over
    expect(q.length).toBe(0);
    expect(t.records[0]!.outcome).toBe('none');
    void w;
  });
});

const clock = (o: Partial<EngineClockState> = {}): EngineClockState => ({ startedAt: null, firstIdleAt: null, lastIdleAt: null, lastResumeAt: null, ended: null, error: false, cycles: [], ...o });

describe('engineState: waiting / computing / starting / stopped', () => {
  it('not started, then starting: input is not accepted until the first idle', () => {
    expect(engineState(clock(), 0)).toMatchObject({ state: 'not-started', inputAccepted: false });
    const s = engineState(clock({ startedAt: 1000 }), 13_000);
    expect(s).toMatchObject({ state: 'starting', inputAccepted: false });
    expect(s.text).toContain('12 s');
  });
  it('idle: the engine called its idle and has not been resumed since', () => {
    expect(engineState(clock({ startedAt: 0, firstIdleAt: 100, lastIdleAt: 100 }), 200)).toMatchObject({ state: 'idle', inputAccepted: true });
    expect(engineState(clock({ startedAt: 0, firstIdleAt: 100, lastIdleAt: 300, lastResumeAt: 250 }), 400)).toMatchObject({ state: 'idle' });
  });
  it('computing (N s): resumed after its last idle; the time is counted from the resume', () => {
    const s = engineState(clock({ startedAt: 0, firstIdleAt: 100, lastIdleAt: 300, lastResumeAt: 400 }), 1900);
    expect(s).toMatchObject({ state: 'computing', inputAccepted: true });
    expect(s.text).toContain('1.5 s');
  });
  it('stopped: it no longer takes input, and the text says how it ended', () => {
    const s = engineState(clock({ startedAt: 0, firstIdleAt: 1, lastIdleAt: 1, ended: { reason: 'stopped', code: 134 } }), 10);
    expect(s).toMatchObject({ state: 'stopped', inputAccepted: false });
    expect(s.text).toContain('exit code 134');
    expect(engineState(clock({ startedAt: 0, ended: { reason: 'stopped', code: -1 } }), 10).text).not.toContain('exit code');
  });
  it('an error of the page wins over everything', () => {
    expect(engineState(clock({ startedAt: 0, firstIdleAt: 1, error: true }), 10)).toMatchObject({ state: 'error', inputAccepted: false });
  });
  it('meanCycle averages the last cycles', () => {
    expect(meanCycle([])).toBeNull();
    expect(meanCycle([100, 200, 300])).toBe(200);
    expect(meanCycle([1000, 100, 100], 2)).toBe(100);
  });
});

describe('LatencyTracker.answer (decision 0223: display method D makes the picture after the idle)', () => {
  it('the present the Worker made in the turn that processed an input answers it, although it was made after the idle + margin of decision 0205', () => {
    const t = new LatencyTracker();
    t.sent(1, 1000, mv);
    t.done([{ seq: 1, tPage: 1000, tInject: 1010, tIdle: 1030 }]);
    t.answer([1], 0, 1036);                 // the present returned 6 ms after the idle: outside tIdle + IN_WINDOW_MARGIN_MS
    t.frame(0, 1036, 1036, 1036);           // and the generic frame, as the page does for every present
    const r = t.records[0]!;
    expect(r.outcome).toBe('window');
    expect(toFrame(0)(r)).toBe(36);
    expect(toAnyFrame(r)).toBe(36);
    expect(t.outcomes).toMatchObject({ answered: 1, late: 0, noAnswer: 0, awaiting: 0 });
  });
  it('the same present without the answer is a late frame, not the answer (what decision 0205 does for a frame outside the window)', () => {
    const t = new LatencyTracker();
    t.sent(1, 1000, mv);
    t.done([{ seq: 1, tPage: 1000, tInject: 1010, tIdle: 1030 }]);
    t.frame(0, 1036, 1036, 1036);
    expect(t.records[0]!.outcome).toBe('late');
    expect(t.outcomes).toMatchObject({ answered: 0, late: 1 });
  });
  it('an input that was not processed yet, or is unknown, is not answered; an input is answered once; the answer moves an input that was late to answered', () => {
    const t = new LatencyTracker();
    t.sent(1, 1000, mv);
    t.sent(2, 1001, mv);
    t.answer([1, 99], 0, 1040);             // 1 has no input-done yet, 99 does not exist
    expect(t.records[0]!.outcome).toBeNull();
    expect(t.records[0]!.frames[0]).toBeUndefined();
    t.done([{ seq: 1, tPage: 1000, tInject: 1010, tIdle: 1030 }, { seq: 2, tPage: 1001, tInject: 1031, tIdle: 1050 }]);
    t.frame(0, 1060, 1060, 1060);           // late for both
    expect(t.outcomes).toMatchObject({ answered: 0, late: 2 });
    t.answer([2], 0, 1060);
    t.answer([2], 0, 1061);                 // again: no second count
    expect(t.outcomes).toMatchObject({ answered: 1, late: 1 });
    expect(t.records[1]!.frames[0]).toBe(1060);
  });
});
