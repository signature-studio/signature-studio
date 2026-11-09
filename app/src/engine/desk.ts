// Decision 0223 (desk1): the desktop picture. The runtime's compositor (lift/runtime/fle_desk.c) keeps ONE picture of the virtual screen in the engine's linear memory (R, G, B, A bytes, 1920 x 1080) and
// reports the rectangles that changed. This file reads that report (the layout is pinned to lift/runtime/fle_desk.h by test/unit/desk.test.ts) and shows the rectangles on the OffscreenCanvas the page handed to the Worker:
// putImageData of an ImageData that is a VIEW of the picture in the engine's memory, with a dirty rectangle: no JS copy and no conversion of a pixel, no message to the page. The Worker knows nothing about windows here.
export const DESK_MAGIC = 0x4b534544;     // bytes 'D' 'E' 'S' 'K'
export const DESK_VERSION = 1;
export const DESK_MAX_RECTS = 32;
export const DESK_HEADER_BYTES = 64;      // 16 words: magic, version, width, height, stride, pixels, gen, n_rects, n_windows, flags, reserved[6]
export const DESK_RECT_BYTES = 16;
export const DESK_FLAG_WHOLE = 1;

export interface DeskRect { x0: number; y0: number; x1: number; y1: number }
export interface DeskState { width: number; height: number; stride: number; pixels: number; gen: number; nWindows: number; whole: boolean; rects: DeskRect[] }

export function readDeskState(buffer: ArrayBufferLike, ptr: number): DeskState | null {
  if (ptr === 0) return null;
  const dv = new DataView(buffer, ptr, DESK_HEADER_BYTES + DESK_MAX_RECTS * DESK_RECT_BYTES);
  if (dv.getUint32(0, true) !== DESK_MAGIC || dv.getUint32(4, true) !== DESK_VERSION) return null;
  const n = Math.min(dv.getUint32(28, true), DESK_MAX_RECTS);
  const rects: DeskRect[] = [];
  for (let i = 0; i < n; i++) {
    const b = DESK_HEADER_BYTES + i * DESK_RECT_BYTES;
    rects.push({ x0: dv.getInt32(b, true), y0: dv.getInt32(b + 4, true), x1: dv.getInt32(b + 8, true), y1: dv.getInt32(b + 12, true) });
  }
  return {
    width: dv.getUint32(8, true), height: dv.getUint32(12, true), stride: dv.getUint32(16, true), pixels: dv.getUint32(20, true), gen: dv.getUint32(24, true),
    nWindows: dv.getUint32(32, true), whole: (dv.getUint32(36, true) & DESK_FLAG_WHOLE) !== 0, rects,
  };
}

/** the part of the engine's exports the presenter uses */
export interface DeskApi {
  /** fle_desk_compose(): the address of the state */
  compose(): number;
  /** fle_desk_resync(): the next compose reports the whole picture */
  resync(): void;
  /** fle_desk_describe(): a line per window drawn at the last compose (diagnostics) */
  describe(): string;
  /** decision 0240: fle_desk_scene() and fle_desk_scene_mode(), or absent when the wasm was built before them */
  scene?(): number;
  sceneMode?(on: number): void;
}

export interface DeskContext { putImageData(data: ImageData, dx: number, dy: number, sx: number, sy: number, sw: number, sh: number): void }
export interface DeskSurface { width: number; height: number; getContext(kind: '2d'): DeskContext | null }

export interface DeskPresentStats { presents: number; rects: number; pixels: number; composeMs: number; putMs: number; whole: number }

export class DeskPresenter {
  readonly stats: DeskPresentStats = { presents: 0, rects: 0, pixels: 0, composeMs: 0, putMs: 0, whole: 0 };
  private readonly ctx: DeskContext;
  private view: { buffer: ArrayBufferLike; pixels: number; image: ImageData } | null = null;
  private lastGen = -1;
  private copy: ImageData | null = null;   // decision 0234: the picture of a shared memory is copied (a rectangle at a time) into this ordinary ImageData: ImageData refuses a view of a SharedArrayBuffer
  /** the state the last present read (the address and size of the picture): a test aid can read the picture as it is, without a compose */
  lastState: DeskState | null = null;
  private readonly surface: DeskSurface;
  private readonly api: DeskApi;
  private readonly memory: () => ArrayBufferLike | null;
  private readonly makeImage: (buffer: ArrayBufferLike, pixels: number, w: number, h: number) => ImageData;
  private readonly now: () => number;

  constructor(surface: DeskSurface, api: DeskApi, memory: () => ArrayBufferLike | null, makeImage?: (buffer: ArrayBufferLike, pixels: number, w: number, h: number) => ImageData, now: () => number = () => performance.now()) {
    const ctx = surface.getContext('2d');
    if (!ctx) throw new Error('no 2d context for the desktop canvas');
    this.ctx = ctx;
    this.surface = surface;
    this.api = api;
    this.memory = memory;
    this.makeImage = makeImage ?? ((b, p, w, h) => new ImageData(new Uint8ClampedArray(b as ArrayBuffer, p, w * h * 4), w, h));
    this.now = now;
  }

  /** brings the compositor's picture up to date and shows what changed; the number of rectangles shown (0: nothing changed) */
  present(): number {
    if (!this.memory()) return 0;
    const t0 = this.now();
    // compose() allocates in the engine's memory (the picture and the scratch rectangle are malloc'd), which can grow it: the old ArrayBuffer is detached then. The buffer is read AFTER compose; a buffer taken before it
    // made readDeskState throw, and the compose had already changed the picture, so the rectangles it reported were lost and the canvas never got those pixels (found as black window borders at the start in an
    // in-run identity test, and as a page that went to 'error' in a state dump). If reading still fails, the next compose reports the whole picture.
    const ptr = this.api.compose();
    let st: DeskState | null;
    try { st = readDeskState(this.memory() as ArrayBufferLike, ptr); } catch (e) { this.api.resync(); throw e; }
    if (st) this.lastState = st;
    const t1 = this.now();
    this.stats.composeMs += t1 - t0;
    if (!st) return 0;
    if (this.surface.width !== st.width || this.surface.height !== st.height) { this.surface.width = st.width; this.surface.height = st.height; this.api.resync(); return this.present(); }
    if (st.rects.length === 0) return 0;
    // the engine's memory buffer is replaced when the memory grows: the view is made again then
    const buf = this.memory() as ArrayBufferLike;
    const shared = typeof SharedArrayBuffer !== 'undefined' && buf instanceof SharedArrayBuffer;
    if (!shared && (!this.view || this.view.buffer !== buf || this.view.pixels !== st.pixels)) this.view = { buffer: buf, pixels: st.pixels, image: this.makeImage(buf, st.pixels, st.width, st.height) };
    if (shared && (!this.copy || this.copy.width !== st.width || this.copy.height !== st.height)) this.copy = new ImageData(st.width, st.height);
    let px = 0;
    for (const r of st.rects) {
      const w = r.x1 - r.x0, h = r.y1 - r.y0;
      if (w <= 0 || h <= 0) continue;
      if (shared && this.copy) {
        const src = new Uint8Array(buf as SharedArrayBuffer, st.pixels, st.width * st.height * 4);
        for (let y = r.y0; y < r.y1; y++) this.copy.data.set(src.subarray((y * st.width + r.x0) * 4, (y * st.width + r.x1) * 4), (y * st.width + r.x0) * 4);
        this.ctx.putImageData(this.copy, 0, 0, r.x0, r.y0, w, h);
      } else if (this.view) this.ctx.putImageData(this.view.image, 0, 0, r.x0, r.y0, w, h);
      px += w * h;
    }
    this.stats.presents++;
    this.stats.rects += st.rects.length;
    this.stats.pixels += px;
    if (st.whole) this.stats.whole++;
    this.stats.putMs += this.now() - t1;
    this.lastGen = st.gen;
    return st.rects.length;
  }

  get generation(): number { return this.lastGen; }

  /** the canvas lost its context or got it back empty: the whole picture is shown at the next present */
  resync(): void { this.api.resync(); }
}
