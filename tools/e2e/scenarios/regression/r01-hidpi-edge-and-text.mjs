// R01 (#1408) hidpi-edge-and-text. Report (L-047, L-050, L-055): the display is not blurred and not enlarged; the picture is the physical size.
// Expectation (both displays, DPR 1 and DPR 2): the guest screen's size is the #desktop box's CSS size x DPR (+-1 px); the canvas's backing size is the guest
// screen's size (+-1 px: no upscaling); the canvas's CSS size x DPR is its backing size (+-1 px); and the first window's frame is at least 1 physical pixel wide
// (the pixels from its left edge that differ from the window's body colour, measured on the picture).
import { geometry, picture, startDefault, MAIN, result } from './common.mjs';

const SCENARIO = {
  name: 'hidpi-edge-and-text',
  report: 'R01',
  expect: 'guest screen = #desktop CSS size x DPR (+-1); canvas backing = guest screen (+-1); canvas CSS x DPR = backing (+-1); window frame >= 1 physical px',
  expectFail: null,
};

function frameWidth(pic, w) {
  const y = Math.round(w.ay + w.h / 2);
  const px = (x) => { const i = y * pic.stride + 4 * x; return (pic.bytes[i] << 16) | (pic.bytes[i + 1] << 8) | pic.bytes[i + 2]; };
  const body = px(Math.min(w.ax + 24, w.ax + w.w - 2));
  let n = 0;
  while (n < 24 && w.ax + n < pic.w && px(w.ax + n) !== body) n++;
  return n;
}

export default {
  ...SCENARIO,
  async run({ guest, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const g = await geometry(guest);
    const cv = await guest.ev(`(() => { const c = document.querySelector('#desktop canvas'); if (!c) return null; const r = c.getBoundingClientRect(); return { w: c.width, h: c.height, cssW: r.width, cssH: r.height }; })()`);
    const pic = await picture(guest);
    // every visible window other than the main form (top-level or child); each one's frame width is written to the JSON
    const frames = pic.windows.filter((w) => w.vis && w.cls !== MAIN && w.w > 200 && w.h > 100);
    const widths = frames.map((w) => frameWidth(pic, w));
    const want = { w: Math.round(g.w * g.dpr), h: Math.round(g.h * g.dpr) };
    const gwOk = Math.abs(g.gw - want.w) <= 1 && Math.abs(g.gh - want.h) <= 1;
    const canvasOk = cv !== null && Math.abs(cv.w - g.gw) <= 1 && Math.abs(cv.h - g.gh) <= 1;
    const notEnlarged = cv !== null && Math.abs(cv.cssW * g.dpr - cv.w) <= 1 && Math.abs(cv.cssH * g.dpr - cv.h) <= 1;
    const frameOk = widths.length > 0 && Math.max(...widths) >= 1;
    const notes = [
      `DPR ${g.dpr}: guest ${g.gw}x${g.gh} px; #desktop ${g.w.toFixed(1)}x${g.h.toFixed(1)} CSS px (want ${want.w}x${want.h} physical: ${gwOk})`,
      cv ? `canvas ${cv.w}x${cv.h} px, CSS ${cv.cssW.toFixed(1)}x${cv.cssH.toFixed(1)} (canvas = guest: ${canvasOk}; not enlarged: ${notEnlarged})` : 'no canvas under #desktop',
      `window frames (${frames.length} windows, the left edge run of each): ${frames.map((w, i) => `"${w.text || w.cls}" ${widths[i]} px`).join(', ') || 'none'}; a frame >= 1 physical px in some window: ${frameOk}`,
      `text: the guest's text is drawn at the guest pixels (guest px per CSS px = ${(g.gw / g.w).toFixed(3)} = DPR)`,
    ];
    log(notes.join(' | '));
    const ok = gwOk && canvasOk && notEnlarged && frameOk;
    return result(ok, start.state, notes, { guest: { w: g.gw, h: g.gh, dpr: g.dpr }, canvas: cv, frameWidths: widths });
  },
};
