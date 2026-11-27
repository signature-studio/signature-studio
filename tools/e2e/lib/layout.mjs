// The places of a window's parts as ratios of the window (Issue #1507). A part's ratio is its distance from the window's top-left at DPR 1 divided by the window's
// width (x) or height (y) at DPR 1. The window's rectangle is read from the window table now (lib/dump.mjs absoluteRect), so the place follows the window at any DPR:
// the UI is drawn larger at DPR 2 and the picture stays 3840 x 2160. Pure: numbers in, numbers out (the tests use it).

/** the place (guest px) of the ratios (rx of the width, ry of the height) inside the window's rectangle r (x, y, w, h) */
export function place(r, rx, ry) {
  return { x: r.x + rx * r.w, y: r.y + ry * r.h };
}

/** a rectangle inside r: its top-left by the ratios (rx, ry), its size by the ratios of the width (rw) and the height (rh) */
export function box(r, rx, ry, rw, rh) {
  return { x: r.x + rx * r.w, y: r.y + ry * r.h, w: rw * r.w, h: rh * r.h };
}

/** a place by the height only (rx and ry are the ratios of the height): for a window whose width is cut by the screen at another DPR */
export function placeH(r, rx, ry) {
  return { x: r.x + rx * r.h, y: r.y + ry * r.h };
}

/** a rectangle by the height only (all four ratios are of the height): see placeH */
export function boxH(r, rx, ry, rw, rh) {
  return { x: r.x + rx * r.h, y: r.y + ry * r.h, w: rw * r.h, h: rh * r.h };
}
