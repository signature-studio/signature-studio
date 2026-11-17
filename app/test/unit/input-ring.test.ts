import { describe, expect, it } from 'vitest';
import { IN, SC, type RawInput } from '../../src/input/events.ts';
import { newRingBuffer, RING_CAPACITY, RingReader, RingWriter, takenAtSchedulerPoint, type RingRecord } from '../../src/input/ring.ts';

const mouse = (kind: 1 | 2 | 3 | 4, x: number, y: number, data = 0): RawInput => ({ k: 'mouse', kind, x, y, data });

describe('the input ring', () => {
  it('delivers mouse and key events in order, with the page clock as a double', () => {
    const sab = newRingBuffer(), w = new RingWriter(sab), r = new RingReader(sab);
    const evs: RawInput[] = [mouse(IN.MOVE, 100, -5), mouse(IN.BDOWN, 3, 4, 1), { k: 'key', kind: IN.KDOWN, vk: 0x41, scan: 0x1e, ext: 1 }, mouse(IN.WHEEL, 10, 11, -120)];
    evs.forEach((e, i) => expect(w.push(i + 1, 1.7e12 + i + 0.25, e)).toBe(true));
    expect(r.pending).toBe(4);
    const got: { seq: number; tPage: number; ev: RawInput }[] = [];
    expect(r.drain((x) => got.push(x))).toBe(4);
    expect(got.map((g) => g.ev)).toEqual(evs);
    expect(got.map((g) => g.seq)).toEqual([1, 2, 3, 4]);
    expect(got[0]?.tPage).toBe(1.7e12 + 0.25);
    expect(r.pending).toBe(0);
  });
  it('carries a display change (decision 0212) like the other events', () => {
    const sab = newRingBuffer(), w = new RingWriter(sab), r = new RingReader(sab);
    const ev: RawInput = { k: 'display', kind: IN.DISPLAY, width: 2176, height: 1092 };
    expect(w.push(7, 12.5, ev)).toBe(true);
    const got: RingRecord[] = [];
    r.drain((x) => got.push(x));
    expect(got).toEqual([{ seq: 7, tPage: 12.5, ev }]);
  });
  it('carries a system command for a window (decision 0272) like the other events', () => {
    const sab = newRingBuffer(), w = new RingWriter(sab), r = new RingReader(sab);
    const ev: RawInput = { k: 'syscmd', kind: IN.SYSCMD, lid: 77, cmd: SC.RESTORE };
    const key: RawInput = { k: 'key', kind: IN.KDOWN, vk: 0x41, scan: 0x1e, ext: 0 };
    expect(w.push(8, 3.5, ev)).toBe(true);
    expect(w.push(9, 4.5, key)).toBe(true);
    const got: RingRecord[] = [];
    r.drain((x) => got.push(x));
    expect(got).toEqual([{ seq: 8, tPage: 3.5, ev }, { seq: 9, tPage: 4.5, ev: key }]);
  });
  it('wraps around, and says "full" instead of overwriting (the caller then sends the event as a message)', () => {
    const sab = newRingBuffer(), w = new RingWriter(sab), r = new RingReader(sab);
    for (let i = 0; i < RING_CAPACITY; i++) expect(w.push(i, i, mouse(IN.MOVE, i, i))).toBe(true);
    expect(w.push(9999, 0, mouse(IN.MOVE, 0, 0))).toBe(false);
    expect(w.fullCount).toBe(1);
    const seen: number[] = [];
    r.drain((x) => seen.push(x.seq));
    expect(seen.length).toBe(RING_CAPACITY);
    for (let i = 0; i < 10; i++) w.push(RING_CAPACITY + i, 0, mouse(IN.MOVE, 1, 2));
    const seen2: number[] = [];
    r.drain((x) => seen2.push(x.seq));
    expect(seen2).toEqual(Array.from({ length: 10 }, (_, i) => RING_CAPACITY + i));
  });
  it('the parked flag goes from the reader to the writer', () => {
    const sab = newRingBuffer(), w = new RingWriter(sab), r = new RingReader(sab);
    expect(w.parked).toBe(false);
    r.setParked(true);
    expect(w.parked).toBe(true);
    r.setParked(false);
    expect(w.parked).toBe(false);
  });
  it('decision 0280 (L1c, alpha11): the scheduler point takes mouse, key and display only; a syscmd (the task bar\'s restore) is not taken and stays first in the ring, with what follows it, for the IdleDriver', () => {
    const sab = newRingBuffer(), w = new RingWriter(sab), r = new RingReader(sab);
    const kinds: RawInput[] = [
      mouse(IN.MOVE, 5, 6),
      { k: 'key', kind: IN.KDOWN, vk: 0x41, scan: 0x1e, ext: 0 },
      { k: 'display', kind: IN.DISPLAY, width: 1920, height: 1080 },
      { k: 'syscmd', kind: IN.SYSCMD, lid: 14, cmd: 0xf120 },
      mouse(IN.BDOWN, 7, 8),
    ];
    kinds.forEach((ev, i) => w.push(i + 1, i, ev));
    const taken: string[] = [], idle: string[] = [];
    expect(r.drainN(16, (x) => { if (!takenAtSchedulerPoint(x.ev)) return false; taken.push(x.ev.k); return true; })).toBe(3);
    expect(taken).toEqual(['mouse', 'key', 'display']);
    expect(r.pending).toBe(2);
    expect(r.drain((x) => { idle.push(x.ev.k); })).toBe(2);
    expect(idle).toEqual(['syscmd', 'mouse']);
    for (const ev of kinds) expect(takenAtSchedulerPoint(ev)).toBe(ev.k !== 'syscmd');
  });
  it('decision 0280: drainN stops at a record that `fn` refuses (returns false): it and the records behind it stay in the ring, in order, for the idle path (the task bar\'s syscmd is not taken by the scheduler point)', () => {
    const sab = newRingBuffer(), w = new RingWriter(sab), r = new RingReader(sab);
    for (let i = 1; i <= 4; i++) w.push(i, i, mouse(IN.MOVE, i, i));
    const a: number[] = [], b: number[] = [];
    expect(r.drainN(16, (x) => { if (x.seq === 3) return false; a.push(x.seq); return true; })).toBe(2);
    expect(a).toEqual([1, 2]);
    expect(r.pending).toBe(2);
    expect(r.drain((x) => { b.push(x.seq); })).toBe(2);
    expect(b).toEqual([3, 4]);
  });
  it('decision 0280: drainN hands at most `max` records, in order, and leaves the rest for the next look (the runtime takes 16 at a scheduler point)', () => {
    const sab = newRingBuffer(), w = new RingWriter(sab), r = new RingReader(sab);
    for (let i = 0; i < 40; i++) w.push(i + 1, i, mouse(IN.MOVE, i, i));
    const a: number[] = [], b: number[] = [];
    expect(r.drainN(16, (x) => a.push(x.seq))).toBe(16);
    expect(r.pending).toBe(24);
    expect(r.drainN(100, (x) => b.push(x.seq))).toBe(24);
    expect(a).toEqual(Array.from({ length: 16 }, (_, k) => k + 1));
    expect(b).toEqual(Array.from({ length: 24 }, (_, k) => k + 17));
    expect(r.drainN(16, () => { throw new Error('nothing is left'); })).toBe(0);
    expect(r.drainN(0, () => { throw new Error('max 0 takes nothing'); })).toBe(0);
  });
});
