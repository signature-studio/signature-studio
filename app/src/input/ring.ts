// Decision 0223 (desk1): the raw input events from the page to the engine Worker through a SharedArrayBuffer ring, instead of a postMessage per event. One producer (the page's main thread), one consumer
// (the Worker). The Worker reads the ring at every idle of the engine and when the page wakes it; the page wakes it (one small message) only if the Worker has said it is parked, i.e. the engine is idle and
// nothing but a message or a timer would run the Worker's code. The order is the usual one for a flag and a queue (Dekker): the producer writes the record, then stores `head`, then reads `parked`; the consumer
// stores `parked`, then reads `head`; with Atomics (sequentially consistent) one of the two sees the other.
import { IN, type DisplayRaw, type KeyRaw, type MouseRaw, type RawInput, type SyscmdRaw } from './events.ts';

export const RING_CAPACITY = 1024;
const HEAD = 0, TAIL = 1, PARKED = 2, FULL = 3;
const HEADER_WORDS = 16;           // 64 bytes: the records start 8-byte aligned
const REC_WORDS = 8;               // seq, type, kind, a, b, c (word 5), tPage as a Float64 (words 6 and 7)
export const RING_BYTES = (HEADER_WORDS + RING_CAPACITY * REC_WORDS) * 4;

const T_MOUSE = 1, T_KEY = 2, T_DISPLAY = 3, T_SYSCMD = 4;

export const newRingBuffer = (): SharedArrayBuffer => new SharedArrayBuffer(RING_BYTES);

export interface RingRecord { seq: number; tPage: number; ev: RawInput }

export class RingWriter {
  private readonly i32: Int32Array;
  private readonly f64: Float64Array;
  constructor(sab: SharedArrayBuffer) { this.i32 = new Int32Array(sab); this.f64 = new Float64Array(sab); }

  /** false: the ring is full (the caller sends the event another way) */
  push(seq: number, tPage: number, ev: RawInput): boolean {
    const head = Atomics.load(this.i32, HEAD), tail = Atomics.load(this.i32, TAIL);
    if (head - tail >= RING_CAPACITY) { Atomics.add(this.i32, FULL, 1); return false; }
    const r = HEADER_WORDS + (head % RING_CAPACITY) * REC_WORDS;
    this.i32[r] = seq;
    if (ev.k === 'mouse') { this.i32[r + 1] = T_MOUSE; this.i32[r + 2] = ev.kind; this.i32[r + 3] = ev.x; this.i32[r + 4] = ev.y; this.i32[r + 5] = ev.data; }
    else if (ev.k === 'display') { this.i32[r + 1] = T_DISPLAY; this.i32[r + 2] = ev.kind; this.i32[r + 3] = ev.width; this.i32[r + 4] = ev.height; this.i32[r + 5] = 0; }
    else if (ev.k === 'syscmd') { this.i32[r + 1] = T_SYSCMD; this.i32[r + 2] = ev.kind; this.i32[r + 3] = ev.lid; this.i32[r + 4] = ev.cmd; this.i32[r + 5] = 0; }
    else { this.i32[r + 1] = T_KEY; this.i32[r + 2] = ev.kind; this.i32[r + 3] = ev.vk; this.i32[r + 4] = ev.scan; this.i32[r + 5] = ev.ext; }
    this.f64[(r + 6) >> 1] = tPage;
    Atomics.store(this.i32, HEAD, head + 1);
    return true;
  }

  /** the Worker has said that only a message or a timer runs its code */
  get parked(): boolean { return Atomics.load(this.i32, PARKED) === 1; }
  get fullCount(): number { return Atomics.load(this.i32, FULL); }
}

export class RingReader {
  private readonly i32: Int32Array;
  private readonly f64: Float64Array;
  constructor(sab: SharedArrayBuffer) { this.i32 = new Int32Array(sab); this.f64 = new Float64Array(sab); }

  get pending(): number { return Atomics.load(this.i32, HEAD) - Atomics.load(this.i32, TAIL); }

  setParked(v: boolean): void { Atomics.store(this.i32, PARKED, v ? 1 : 0); }

  /** hands every waiting record to `fn` in order; the count */
  drain(fn: (r: RingRecord) => void): number { return this.drainN(Infinity, fn); }

  /** decision 0280: hands at most `max` waiting records to `fn` in order; the count. `fn` may return false to refuse a record: it stays in the ring (with every record behind it) for the idle path, and the loop ends */
  drainN(max: number, fn: (r: RingRecord) => unknown): number {
    let n = 0;
    for (;;) {
      if (n >= max) return n;
      const head = Atomics.load(this.i32, HEAD), tail = Atomics.load(this.i32, TAIL);
      if (tail === head) return n;
      const r = HEADER_WORDS + (tail % RING_CAPACITY) * REC_WORDS;
      const t = this.i32[r + 1];
      const ev: RawInput = t === T_MOUSE
        ? ({ k: 'mouse', kind: this.i32[r + 2] as MouseRaw['kind'], x: this.i32[r + 3] as number, y: this.i32[r + 4] as number, data: this.i32[r + 5] as number })
        : t === T_DISPLAY
          ? ({ k: 'display', kind: this.i32[r + 2] as DisplayRaw['kind'], width: this.i32[r + 3] as number, height: this.i32[r + 4] as number })
          : t === T_SYSCMD
            ? ({ k: 'syscmd', kind: this.i32[r + 2] as SyscmdRaw['kind'], lid: this.i32[r + 3] as number, cmd: this.i32[r + 4] as number })
            : ({ k: 'key', kind: this.i32[r + 2] as KeyRaw['kind'], vk: this.i32[r + 3] as number, scan: this.i32[r + 4] as number, ext: this.i32[r + 5] as number });
      const rec: RingRecord = { seq: this.i32[r] as number, tPage: this.f64[(r + 6) >> 1] as number, ev };
      if (fn(rec) === false) return n;
      Atomics.store(this.i32, TAIL, tail + 1);
      n++;
    }
  }
}

/** decision 0280: the kinds of input that the runtime queues at a scheduler point, with the same functions the exports use (fle_g3_mouse / fle_g3_key / fle_g3_display). Any other kind (the task bar's syscmd, decision 0272, or a later one) is not taken there: it stays in the ring, with the records behind it, for the IdleDriver at the engine's next idle. */
export const takenAtSchedulerPoint = (ev: RawInput): ev is MouseRaw | KeyRaw | DisplayRaw => ev.k === 'mouse' || ev.k === 'key' || ev.k === 'display';

export { IN };
