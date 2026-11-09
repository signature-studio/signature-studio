// Reader of lift/runtime/fle_appview.h (decision 0141): what the engine says about the cursor, the pointer, the number of queued input events and the top-level windows in z-order.
// The structure is filled by the exported C function fle_av_poll() and read from the engine's linear memory as little-endian words. The layout below is pinned by test/unit/appview.test.ts to the C header.
export const AV_MAGIC = 0x50414c46; // bytes 'F' 'L' 'A' 'P'
export const AV_VERSION = 1;
export const AV_HEADER_BYTES = 68;   // 17 words: magic, version, entry_size, max_windows, seq, cursor, cursor_idc, cursor_shared, pointer_x, pointer_y, pending_events, n_windows, screen_w, screen_h (decision 0212), reserved[3]
export const AV_WINDOW_BYTES = 144;
export const AV_CLASS_BYTES = 32;
export const AV_TITLE_BYTES = 64;

const LATIN1 = new TextDecoder('latin1');   // one decoder (a new one per string cost about as much as the decoding)
export const W_VISIBLE = 0x1, W_ENABLED = 0x2, W_TOPMOST = 0x4, W_ACTIVE = 0x8, W_FOCUS = 0x10, W_CAPTURE = 0x20;

export interface AvWindow {
  lid: number; hwnd: number; flags: number; ownerLid: number;
  x: number; y: number; width: number; height: number;
  style: number; exstyle: number; thread: number;
  className: string; title: string;
  visible: boolean; enabled: boolean; topmost: boolean; active: boolean; focus: boolean; capture: boolean;
}

export interface AppView {
  seq: number;
  /** low 32 bits of the handle SetCursor set last, 0 = none */
  cursor: number;
  cursorIdc: number;
  cursorShared: boolean;
  pointerX: number; pointerY: number;
  pendingEvents: number;
  /** decision 0212: the size of the virtual screen the guest is told (lift/runtime/fle_screen.h); 0 from a build that does not report it (the page then takes 1920 x 1080) */
  screenW?: number; screenH?: number;
  /** top-level windows, top of the z-order first */
  windows: AvWindow[];
}

/** header: 12 words + screen_w, screen_h + reserved[3] = 68 bytes; then max_windows entries of 144 bytes (the C struct is fle_av_state; fle_appview.c asserts offsetof(windows) == 68) */
export function readAppView(buffer: ArrayBufferLike, ptr: number, sinceSeq?: number): AppView | null {
  const dv = new DataView(buffer, ptr);
  if (dv.getUint32(0, true) !== AV_MAGIC || dv.getUint32(4, true) !== AV_VERSION || dv.getUint32(8, true) !== AV_WINDOW_BYTES) return null;
  // decision 0170: the caller that already holds the view with sequence number `sinceSeq` does not need the window list again (about 30 us for 114 windows, at every one of the engine's idles)
  if (sinceSeq !== undefined && dv.getUint32(16, true) === sinceSeq) return null;
  const max = dv.getUint32(12, true);
  const n = Math.min(dv.getUint32(44, true), max);
  const u8 = new Uint8Array(buffer, ptr);
  const text = (off: number, len: number): string => {
    let e = off;
    while (e < off + len && u8[e] !== 0) e++;
    return LATIN1.decode(typeof SharedArrayBuffer !== 'undefined' && u8.buffer instanceof SharedArrayBuffer ? u8.slice(off, e) : u8.subarray(off, e));   // decision 0234: a shared memory's view is refused by TextDecoder
  };
  const windows: AvWindow[] = [];
  for (let i = 0; i < n; i++) {
    const b = AV_HEADER_BYTES + i * AV_WINDOW_BYTES;
    const flags = dv.getUint32(b + 8, true);
    windows.push({
      lid: dv.getUint32(b, true), hwnd: dv.getUint32(b + 4, true), flags, ownerLid: dv.getUint32(b + 12, true),
      x: dv.getInt32(b + 16, true), y: dv.getInt32(b + 20, true), width: dv.getInt32(b + 24, true), height: dv.getInt32(b + 28, true),
      style: dv.getUint32(b + 32, true), exstyle: dv.getUint32(b + 36, true), thread: dv.getUint32(b + 40, true),
      className: text(b + 48, AV_CLASS_BYTES), title: text(b + 48 + AV_CLASS_BYTES, AV_TITLE_BYTES),
      visible: (flags & W_VISIBLE) !== 0, enabled: (flags & W_ENABLED) !== 0, topmost: (flags & W_TOPMOST) !== 0, active: (flags & W_ACTIVE) !== 0, focus: (flags & W_FOCUS) !== 0, capture: (flags & W_CAPTURE) !== 0,
    });
  }
  return {
    seq: dv.getUint32(16, true), cursor: dv.getUint32(20, true), cursorIdc: dv.getUint32(24, true), cursorShared: dv.getUint32(28, true) !== 0,
    pointerX: dv.getInt32(32, true), pointerY: dv.getInt32(36, true), pendingEvents: dv.getUint32(40, true), screenW: dv.getUint32(48, true), screenH: dv.getUint32(52, true), windows,
  };
}
