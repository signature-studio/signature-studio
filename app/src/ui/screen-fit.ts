// The size of the guest's screen as the page's room for it (decision 0212, screen1; one mode since decision 0243 / opt1).
// The page measures the room it has for the screen (everything below the header and left of the monitor panel), tells the engine that size through the engine's environment (FLE_MONITOR_W / FLE_MONITOR_H:
// lift/runtime/fle_screen.h) and draws the desktop to fill the room: the guest's screen is the room's device pixels (the room in CSS pixels times the device pixel ratio, C-1), so 1 guest pixel = 1 device pixel
// and the canvas is shown at 1 / devicePixelRatio CSS pixels per guest pixel (the browser does not stretch it).
// When the room changes while the engine runs, the engine is told (WM_DISPLAYCHANGE and WM_SETTINGCHANGE; the runtime resizes the maximized windows: S-remaximize). Everything here is pure: the numbers come in, the numbers go out (test/unit/screen-fit.test.ts).

export const SCREEN_DEFAULT = { width: 1920, height: 1080 } as const;
/** the limits of lift/runtime/fle_screen.h (FLE_SCREEN_MIN_* .. FLE_SCREEN_MAX_*): outside them the engine ignores the value */
export const SCREEN_MIN = { width: 640, height: 480 } as const;
export const SCREEN_MAX = { width: 7680, height: 4320 } as const;

/** what the beacon says about the display (decision 0212): the guest's screen, whether the engine reported it, the room and the device pixel ratio. `mode`, `source`, `dpi` and `follow` are the fields of beacon version 3 (the
 *  server still accepts the other values of records stored earlier); the page has one mode: fit, from the default, no DPI told, following the room live. */
export interface GuestScreenInfo {
  mode: 'fit';
  source: 'default';
  width: number;
  height: number;
  reported: boolean;
  dpi: null;
  roomWidth: number | null;
  roomHeight: number | null;
  dpr: number;
  follow: 'live';
}

/** the room for the screen, in CSS pixels, and the device pixel ratio */
export interface Room { width: number; height: number; dpr: number }
export interface GuestScreen {
  /** the guest's screen in device pixels */
  width: number;
  height: number;
  /** CSS pixels per guest pixel: the desktop is drawn at this scale (1 / devicePixelRatio when the screen is the room's device pixels) */
  scale: number;
  /** the CSS length that edges are rounded to: 1 (whole CSS pixels) */
  snap: number;
  /** true when the room asked for a size outside the engine's limits and the size was pulled inside them */
  clamped: boolean;
}

const clampDim = (v: number, lo: number, hi: number): number => Math.max(lo, Math.min(hi, Math.floor(v)));
const ratioOf = (room: Room): number => (room.dpr > 0 ? room.dpr : 1);

/**
 * The guest screen for a room: the room's device pixels (the room in CSS pixels times the device pixel ratio, rounded down), pulled inside the engine's limits. The scale is then whatever makes the screen
 * fill the room: 1 / devicePixelRatio when the room is inside the limits.
 */
export function chooseGuestScreen(room: Room): GuestScreen {
  const dpr = ratioOf(room);
  const rw = Math.max(1, room.width * dpr), rh = Math.max(1, room.height * dpr);
  const w = clampDim(rw, SCREEN_MIN.width, SCREEN_MAX.width), h = clampDim(rh, SCREEN_MIN.height, SCREEN_MAX.height);
  const clamped = w !== Math.floor(rw) || h !== Math.floor(rh);
  const d = drawScale({ width: w, height: h }, room);
  return { width: w, height: h, scale: d.scale, snap: d.snap, clamped };
}

/**
 * How the desktop of a guest screen is drawn in a room. The guest has the device pixels the room holds, so a guest pixel is one device pixel: 1 / devicePixelRatio CSS pixels, which the browser shows
 * without stretching the canvas (C-1). When the guest's screen is not that (the room is outside the engine's limits, or the engine did not take the size the page asked for) the screen is fitted into
 * the room in both directions and edges are rounded to whole CSS pixels.
 */
export function drawScale(guest: { width: number; height: number }, room: Room, honoured = true): { scale: number; snap: number } {
  const fit = Math.min(Math.max(1, room.width) / guest.width, Math.max(1, room.height) / guest.height);
  const natural = 1 / ratioOf(room);
  return { scale: honoured && natural <= fit + 1e-9 ? natural : fit, snap: 1 };
}

/** the engine's environment for a guest screen: both variables */
export function screenEnv(width: number, height: number): Record<string, string> {
  return { FLE_MONITOR_W: String(width), FLE_MONITOR_H: String(height) };
}

export function screenDiffers(a: { width: number; height: number }, b: { width: number; height: number }): boolean {
  return a.width !== b.width || a.height !== b.height;
}

/**
 * Collects a stream of room measurements (a window being dragged to a new size) into one: `push` the room whenever it changes; `onSettled` is called with the last room once no new one has come for
 * `quietMs`. The timer is injected (setTimeout/clearTimeout in the page, a fake in the unit test). S-screen1-quiet (a choice without a source): the default quiet time is 300 ms.
 */
export class RoomDebouncer {
  private timer: unknown = null;
  private last: Room | null = null;
  private readonly onSettled: (room: Room) => void;
  private readonly timers: { set(fn: () => void, ms: number): unknown; clear(h: unknown): void };
  private readonly quietMs: number;
  constructor(onSettled: (room: Room) => void, timers: { set(fn: () => void, ms: number): unknown; clear(h: unknown): void }, quietMs = 300) {
    this.onSettled = onSettled;
    this.timers = timers;
    this.quietMs = quietMs;
  }
  push(room: Room): void {
    if (this.last && this.last.width === room.width && this.last.height === room.height && this.last.dpr === room.dpr && this.timer === null) return;
    this.last = room;
    if (this.timer !== null) this.timers.clear(this.timer);
    this.timer = this.timers.set(() => { this.timer = null; if (this.last) this.onSettled(this.last); }, this.quietMs);
  }
  get pending(): boolean { return this.timer !== null; }
  cancel(): void { if (this.timer !== null) { this.timers.clear(this.timer); this.timer = null; } }
}
