// Plugin Worker <-> host protocol over ONE SharedArrayBuffer per plugin instance (decision 0173, S1). Pure data layout and Atomics handshakes, no DOM, no Worker: it runs the same in a browser
// Worker, in the page's main thread (async waits only: the main thread cannot Atomics.wait), and in node.
//
//   bytes 0..127   : 32 Int32 control words (W.*)
//   then           : inL, inR, outL, outR - four planar float32 blocks of maxFrames each (the VST3 ProcessData layout: one float* per channel)
//
// One block in flight per slot. Host: beginBlock() writes the input and publishes the request; the plugin Worker answers with respond(); the host reads the output. A request is a new value of
// W.REQ; the answer is W.RESP == that value. QUIT is a request with CMD = QUIT. Words are written before the Atomics.store of REQ/RESP that publishes them (sequentially consistent).

export const SLOT_MAGIC = 0x3150544d; // "MTP1"
export const SLOT_VERSION = 1;
export const HEADER_WORDS = 32;
export const HEADER_BYTES = HEADER_WORDS * 4;

export const W = { MAGIC: 0, VERSION: 1, MAX_FRAMES: 2, REQ: 3, RESP: 4, CMD: 5, FRAMES: 6, STATE: 7, BLOCK: 8, PROC_US: 9, WAKE_US: 10, ERR: 11 } as const;
export const CMD = { NONE: 0, PROCESS: 1, QUIT: 2 } as const;
export const STATE = { CREATED: 0, LOADING: 1, PREROLL: 2, READY: 3, RUNNING: 4, STOPPED: 5, FAILED: 6 } as const;

export function slotBytes(maxFrames: number): number {
  return HEADER_BYTES + 4 * maxFrames * 4;
}

export class Slot {
  readonly sab: SharedArrayBuffer;
  readonly maxFrames: number;
  readonly ctrl: Int32Array;
  readonly inL: Float32Array;
  readonly inR: Float32Array;
  readonly outL: Float32Array;
  readonly outR: Float32Array;

  private constructor(sab: SharedArrayBuffer, maxFrames: number) {
    this.sab = sab;
    this.maxFrames = maxFrames;
    this.ctrl = new Int32Array(sab, 0, HEADER_WORDS);
    this.inL = new Float32Array(sab, HEADER_BYTES, maxFrames);
    this.inR = new Float32Array(sab, HEADER_BYTES + 4 * maxFrames, maxFrames);
    this.outL = new Float32Array(sab, HEADER_BYTES + 8 * maxFrames, maxFrames);
    this.outR = new Float32Array(sab, HEADER_BYTES + 12 * maxFrames, maxFrames);
  }

  static create(maxFrames = 4096): Slot {
    const s = new Slot(new SharedArrayBuffer(slotBytes(maxFrames)), maxFrames);
    Atomics.store(s.ctrl, W.MAGIC, SLOT_MAGIC);
    Atomics.store(s.ctrl, W.VERSION, SLOT_VERSION);
    Atomics.store(s.ctrl, W.MAX_FRAMES, maxFrames);
    return s;
  }

  static attach(sab: SharedArrayBuffer): Slot {
    const h = new Int32Array(sab, 0, HEADER_WORDS);
    if (Atomics.load(h, W.MAGIC) !== SLOT_MAGIC || Atomics.load(h, W.VERSION) !== SLOT_VERSION) throw new Error('not a plugin slot (magic/version)');
    const maxFrames = Atomics.load(h, W.MAX_FRAMES);
    if (sab.byteLength !== slotBytes(maxFrames)) throw new Error(`plugin slot size ${sab.byteLength} does not match maxFrames ${maxFrames}`);
    return new Slot(sab, maxFrames);
  }

  get state(): number {
    return Atomics.load(this.ctrl, W.STATE);
  }
  setState(v: number): void {
    Atomics.store(this.ctrl, W.STATE, v);
  }
}

type WaitAsyncResult = { async: false; value: 'not-equal' | 'timed-out' } | { async: true; value: Promise<'ok' | 'timed-out'> };
const waitAsyncFn = (): ((ia: Int32Array, i: number, v: number, t?: number) => WaitAsyncResult) =>
  (Atomics as unknown as { waitAsync: (ia: Int32Array, i: number, v: number, t?: number) => WaitAsyncResult }).waitAsync;

// ---- host side ----

/** Copy one input block into the slot and publish the request. Returns the request number to wait for. */
export function beginBlock(slot: Slot, inL: Float32Array, inR: Float32Array, frames: number): number {
  if (frames < 1 || frames > slot.maxFrames) throw new RangeError(`frames ${frames} outside 1..${slot.maxFrames}`);
  slot.inL.set(inL.subarray(0, frames));
  slot.inR.set(inR.subarray(0, frames));
  return publish(slot, CMD.PROCESS, frames);
}

/** Ask the plugin Worker to leave its process loop (it then tears the plugin down and reports). */
export function requestQuit(slot: Slot): number {
  return publish(slot, CMD.QUIT, 0);
}

function publish(slot: Slot, cmd: number, frames: number): number {
  const c = slot.ctrl;
  Atomics.store(c, W.CMD, cmd);
  Atomics.store(c, W.FRAMES, frames);
  const seq = (Atomics.load(c, W.REQ) + 1) | 0;
  Atomics.store(c, W.REQ, seq);
  Atomics.notify(c, W.REQ);
  return seq;
}

/** Blocking wait for the answer to `seq` (Workers and node; the browser's main thread throws). Returns false on timeout. */
export function waitResponse(slot: Slot, seq: number, timeoutMs = Infinity): boolean {
  const c = slot.ctrl;
  const t0 = performance.now();
  for (;;) {
    const v = Atomics.load(c, W.RESP);
    if (v === seq) return true;
    const left = timeoutMs - (performance.now() - t0);
    if (left <= 0) return false;
    Atomics.wait(c, W.RESP, v, left);
  }
}

/** Non-blocking wait (any thread, including the browser's main thread and a JSPI-suspended import). */
export async function waitResponseAsync(slot: Slot, seq: number, timeoutMs = Infinity): Promise<boolean> {
  const c = slot.ctrl;
  const t0 = performance.now();
  for (;;) {
    const v = Atomics.load(c, W.RESP);
    if (v === seq) return true;
    const left = timeoutMs - (performance.now() - t0);
    if (left <= 0) return false;
    const r = waitAsyncFn()(c, W.RESP, v, left);
    if (r.async) await r.value;
  }
}

/** Copy the output block of the last answer. */
export function readOutput(slot: Slot, outL: Float32Array, outR: Float32Array, frames: number): void {
  outL.set(slot.outL.subarray(0, frames));
  outR.set(slot.outR.subarray(0, frames));
}

// ---- plugin side ----

/** Block until a request newer than `last` arrives; returns its number. */
export function waitRequest(slot: Slot, last: number): number {
  const c = slot.ctrl;
  for (;;) {
    const v = Atomics.load(c, W.REQ);
    if (v !== last) return v;
    Atomics.wait(c, W.REQ, last);
  }
}

export function respond(slot: Slot, seq: number, procUs: number): void {
  const c = slot.ctrl;
  Atomics.store(c, W.PROC_US, Math.min(0x7fffffff, Math.round(procUs)));
  Atomics.store(c, W.RESP, seq);
  Atomics.notify(c, W.RESP);
}
