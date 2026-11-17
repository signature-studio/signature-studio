// Decision 0311 (gpu1, PROVISIONAL, measurement tree): the link between the engine Worker and the present Worker `P` (desk-worker.ts). P owns the desktop canvas and the WebGL2 context; the engine Worker stays the only
// place that calls the engine's exports (fle_desk_scene needs the runtime's tables). Everything that crosses is one SharedArrayBuffer:
//   - a header of Int32 words (below) and one Float64, the single scene slot (the 6.5 KB description fle_desk_scene writes: windows, surfaces' addresses, written rectangles),
//   - no pixel: P reads the surfaces from the engine's own (shared) linear memory, as the engine Worker did.
// The protocol has one scene in flight: the engine publishes only when P has finished the previous one (PUB === DONE), because the rectangles `fle_desk_scene` hands out are consumed by the call (a scene that P never drew
// would lose them); while P is busy the runtime keeps collecting what is written and the next call returns it all. P waits for PUB with Atomics.waitAsync (its event loop stays free for animation frames); the engine polls DONE,
// WANT and the requests at the points where it already looks at the page's input ring (scheduler points and idles). Nothing here knows about windows.
import { SCENE_HEADER_BYTES, SCENE_MAX_UPDATES, SCENE_MAX_WINDOWS, SCENE_UPD_BYTES, SCENE_WIN_BYTES } from './desk-gl.ts';

export const SCENE_BYTES = SCENE_HEADER_BYTES + SCENE_MAX_WINDOWS * SCENE_WIN_BYTES + SCENE_MAX_UPDATES * SCENE_UPD_BYTES;

/** Int32 word indices of the header */
export const W = {
  PUB: 0,            // engine: sequence number of the last published scene (the slot holds it)
  DONE: 1,           // P: sequence number of the last scene P has finished with (drawn or found to show nothing)
  WANT: 2,           // P: 1 = an animation frame asks for a scene; the engine clears it when it publishes (or finds nothing to publish)
  ASLEEP: 3,         // P: 1 = the frame loop sleeps (nothing changed for a while): the engine's idle looks for the next change
  READY: 4,          // P: 0 = starting (has not looked yet), 1 = the GL presenter exists, 2 = this platform cannot (no WebGL2 or no Atomics.waitAsync: the engine Worker keeps the desktop), 3 = it can (checked before the canvas is handed over)
  MODE_REQ: 5,       // P: 1 = switch the runtime to the scene description (after the GL context exists)
  MODE_ON: 6,        // engine: 1 = done
  RESYNC_REQ: 7,     // P: 1 = send the whole picture again (a lost context, a new size)
  RESULT: 8,         // P: rectangles shown by the scene DONE names (0: nothing changed)
  PRESENTS: 9,       // P: presents made
  RING_READS: 10, WAKES: 11, IDLE_TICKS: 12,   // engine counters P puts into its report
} as const;
export const HEADER_BYTES = 128;
const AT_INDEX = 8;                       // Float64 index (byte 64): the instant of the last present of P (performance.timeOrigin + performance.now())
export const LINK_BYTES = HEADER_BYTES + SCENE_BYTES;

export function newDeskLink(): SharedArrayBuffer { return new SharedArrayBuffer(LINK_BYTES); }

abstract class Base {
  protected readonly i32: Int32Array;
  protected readonly f64: Float64Array;
  readonly buffer: SharedArrayBuffer;
  constructor(buffer: SharedArrayBuffer) {
    this.buffer = buffer;
    this.i32 = new Int32Array(buffer, 0, HEADER_BYTES / 4);
    this.f64 = new Float64Array(buffer, 0, HEADER_BYTES / 8);
  }
  protected get(w: number): number { return Atomics.load(this.i32, w); }
  protected set(w: number, v: number): void { Atomics.store(this.i32, w, v); }
  /** byte offset of the scene slot in `buffer` */
  get slotOffset(): number { return HEADER_BYTES; }
  get pub(): number { return this.get(W.PUB); }
  get done(): number { return this.get(W.DONE); }
  get ready(): number { return this.get(W.READY); }
  get asleep(): boolean { return this.get(W.ASLEEP) !== 0; }
  get result(): number { return this.get(W.RESULT); }
  /** written before DONE (sequentially consistent), so a reader that has seen DONE reads the instant of that present */
  get presentAt(): number { return this.f64[AT_INDEX] as number; }
  get presents(): number { return this.get(W.PRESENTS); }
  get modeOn(): boolean { return this.get(W.MODE_ON) !== 0; }
  get wantSet(): boolean { return this.get(W.WANT) !== 0; }
  counter(w: number): number { return this.get(w); }
}

/** the engine Worker's end */
export class DeskLinkEngine extends Base {
  /** P has finished the scene before: a new one may be published */
  canPublish(): boolean { return this.get(W.PUB) === this.get(W.DONE); }
  /** copies the description at `ptr` of the engine's memory into the slot and tells P; returns the sequence number */
  publish(mem: ArrayBufferLike, ptr: number): number {
    new Uint8Array(this.buffer, HEADER_BYTES, SCENE_BYTES).set(new Uint8Array(mem, ptr, SCENE_BYTES));
    const seq = this.get(W.PUB) + 1;
    this.set(W.PUB, seq);
    Atomics.notify(this.i32, W.PUB);
    return seq;
  }
    clearWant(): void { this.set(W.WANT, 0); }
  modeRequested(): boolean { return this.get(W.MODE_REQ) !== 0 && this.get(W.MODE_ON) === 0; }
  setModeOn(): void { this.set(W.MODE_ON, 1); }
  takeResync(): boolean { return Atomics.compareExchange(this.i32, W.RESYNC_REQ, 1, 0) === 1; }
  setCounters(ringReads: number, wakes: number, idleTicks: number): void { this.set(W.RING_READS, ringReads); this.set(W.WAKES, wakes); this.set(W.IDLE_TICKS, idleTicks); }
}

/** P's end */
export class DeskLinkP extends Base {
  /** resolves with the sequence number once PUB differs from `seen` */
  async waitPublished(seen: number): Promise<number> {
    for (;;) {
      const p = this.get(W.PUB);
      if (p !== seen) return p;
      const r = (Atomics as unknown as { waitAsync(a: Int32Array, i: number, v: number): { async: boolean; value: Promise<string> | string } }).waitAsync(this.i32, W.PUB, seen);
      if (r.async) await r.value;
    }
  }
  markDone(seq: number, shown: number, at: number): void {
    this.f64[AT_INDEX] = at;
    this.set(W.RESULT, shown);
    this.set(W.PRESENTS, this.get(W.PRESENTS) + (shown > 0 ? 1 : 0));
    this.set(W.DONE, seq);
  }
  requestScene(): void { this.set(W.WANT, 1); }
  setAsleep(on: boolean): void { this.set(W.ASLEEP, on ? 1 : 0); }
  setReady(v: number): void { this.set(W.READY, v); }
  requestMode(on: number): void { this.set(W.MODE_REQ, on); }
  requestResync(): void { this.set(W.RESYNC_REQ, 1); }
}
