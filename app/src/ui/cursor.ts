// The cursor the guest chose (SetCursor / WM_SETCURSOR result, read through fle_av_poll: lift/runtime/fle_appview.h) as a CSS cursor value.
// Windows names (Learn "About Cursors", numbers as in lift/runtime/fle_cursor.h) -> MDN CSS `cursor` keywords, one to one, only where both mean the same cursor (no look-alike: #1390).
// A standard cursor with no CSS equivalent (IDC_UPARROW) is shown as the arrow and is reported as `unmapped` (its name and value), so the page can count it.
// A handle of 0 is "no cursor" (Learn SetCursor: NULL removes the cursor from the screen) -> `none`.
// A cursor the guest loaded from one of its own modules (fle_cursor_is_shared_handle) is shown as the picture of its resource (#1392, decision 0260): the engine gives the resource's pixels
// with each osstate that names the handle (cursor-picture.ts), and the page shows them as a PNG of the same pixels (png.ts), not scaled. Until the picture has come, or when the engine has none, it is the arrow and the reason says so.
// A cursor the guest made itself (CreateIconIndirect, cursorMade) is shown as the picture of its bitmaps (the engine copies them when the cursor is made; #1392), and reported as `made`.
// Above a device pixel ratio of 1 the picture is given as image-set of its copies at 1x, 2x and 3x (nearest neighbour; the browser takes the one for its density); at 1 it is the url of the 1x picture.
import type { AppView } from '../engine/appview.ts';

export const IDC_CSS: Readonly<Record<number, string>> = {
  32512: 'default',      // IDC_ARROW
  32513: 'text',         // IDC_IBEAM
  32514: 'wait',         // IDC_WAIT
  32515: 'crosshair',    // IDC_CROSS
  32642: 'nwse-resize',  // IDC_SIZENWSE
  32643: 'nesw-resize',  // IDC_SIZENESW
  32644: 'ew-resize',    // IDC_SIZEWE
  32645: 'ns-resize',    // IDC_SIZENS
  32646: 'move',         // IDC_SIZEALL
  32648: 'not-allowed',  // IDC_NO
  32649: 'pointer',      // IDC_HAND
  32650: 'progress',     // IDC_APPSTARTING
  32651: 'help',         // IDC_HELP
};

/** the standard cursors with no CSS equivalent: CSS has no up-arrow, so none is put in its place (a look-alike is not allowed); the arrow is shown and the cursor is reported */
export const IDC_UNMAPPED: Readonly<Record<number, string>> = {
  32516: 'IDC_UPARROW',
};

/** #1392: the picture of a cursor of a module or made by CreateIconIndirect, as the page shows it: its PNGs (`images`: the picture at scale 1, then the nearest-neighbour copies at 2 and 3 when they fit) and the hot spot; `raw` (base64) is what the picture was made of (a made cursor); or the reason there is none */
export type CursorPictureState =
  | { ok: true; images: { scale: number; url: string }[]; hotX: number; hotY: number; source: string; cursorId: number; width: number; height: number; invert: number; alpha: boolean; raw?: string }
  | { ok: false; why: string };

/** the largest cursor image the page gives (device pixels: a larger cursor is not shown by the browser, so no copy of that size is made) */
export const CURSOR_IMAGE_MAX = 128;

export interface CursorChoice {
  css: string;
  why: string;
  /** a standard cursor with no CSS equivalent (IDC_UNMAPPED): its name and its value */
  unmapped?: { name: string; value: number };
  /** a cursor the guest made itself (CreateIconIndirect): it has no standard identifier and no picture yet */
  made?: boolean;
  /** a cursor of a module of the guest whose picture is not shown (none from the engine yet, or none at all): the arrow is shown */
  unpictured?: boolean;
  /** a cursor shown as its picture (a module's resource, or made by CreateIconIndirect when cursorId is 0): its name or ordinal, the RT_CURSOR id, the size, the hot spot, the inverted pixels, whether the alpha was used, the CSS the picture is shown with (`url` at DPR 1, `image-set` above it), and (made) the pixels it was made of, base64 */
  picture?: { source: string; cursorId: number; width: number; height: number; hotX: number; hotY: number; invert: number; alpha: boolean; css: string; raw?: string };
  /** the handle is not none, not a standard cursor, not a cursor of a module and not one the guest made: the engine has no identity for it */
  unresolved?: boolean;
}

/** `dpr` is the page's device pixel ratio (window.devicePixelRatio): above 1 a picture is given as image-set of its copies (the browser takes the one for the density), at 1 as the picture itself */
export function cursorCss(v: Pick<AppView, 'cursor' | 'cursorIdc' | 'cursorShared' | 'cursorMade'> | null, picture?: CursorPictureState, dpr = 1): CursorChoice {
  if (!v) return { css: 'default', why: 'the engine reports no cursor' };
  if (v.cursor === 0) return { css: 'none', why: 'SetCursor(NULL): the guest removed the cursor' };
  if (v.cursorIdc !== 0) {
    const css = IDC_CSS[v.cursorIdc];
    if (css) return { css, why: `standard cursor IDC ${v.cursorIdc}` };
    const name = IDC_UNMAPPED[v.cursorIdc] ?? `IDC ${v.cursorIdc}`;
    return { css: 'default', why: `standard cursor ${name} (IDC ${v.cursorIdc}) has no CSS equivalent: shown as the arrow`, unmapped: { name, value: v.cursorIdc } };
  }
  if (v.cursorShared) {
    const handle = `handle 0x${v.cursor.toString(16)}`;
    if (picture === undefined) return { css: 'default', why: `a cursor from a module of the guest (${handle}): its picture has not come from the engine yet: shown as the arrow`, unpictured: true };
    if (!picture.ok) return { css: 'default', why: `a cursor from a module of the guest (${handle}): the engine has no picture of it (${picture.why}): shown as the arrow`, unpictured: true };
    return pictured(picture, handle, dpr);
  }
  if (v.cursorMade) {
    const handle = `handle 0x${v.cursor.toString(16)}`;
    if (picture?.ok) return { ...pictured(picture, handle, dpr), made: true };
    const why = picture === undefined ? 'its picture has not come from the engine yet' : `the engine has no picture of it (${picture.why})`;
    return { css: 'default', why: `a cursor the guest made itself (CreateIconIndirect, ${handle}): ${why}: shown as the arrow`, made: true, unpictured: true };
  }
  return { css: 'default', why: `a handle with no identity (0x${v.cursor.toString(16)}): shown as the arrow`, unresolved: true };
}

/** the CSS of a picture at the hot spot: `url` of the picture at DPR 1; above 1, image-set of its copies (1x, 2x, 3x; the browser takes the one for its density); then the arrow as the fallback (no look-alike is put in its place) */
function pictured(picture: Extract<CursorPictureState, { ok: true }>, handle: string, dpr: number): CursorChoice {
  const what = picture.cursorId === 0 ? 'made by CreateIconIndirect' : `RT_CURSOR ${picture.cursorId}`;
  const hot = `${picture.hotX} ${picture.hotY}`;
  const [first] = picture.images;
  const set = picture.images.map((i) => `url("${i.url}") ${i.scale}x`).join(', ');
  const shown = dpr > 1 && picture.images.length > 1 ? `image-set(${set}) ${hot}, default` : `url("${first?.url ?? ''}") ${hot}, default`;
  return {
    css: shown,
    why: `the guest's cursor ${picture.source} (${handle}): ${what}, ${picture.width} x ${picture.height}, hot spot ${picture.hotX}, ${picture.hotY}`,
    picture: { source: picture.source, cursorId: picture.cursorId, width: picture.width, height: picture.height, hotX: picture.hotX, hotY: picture.hotY, invert: picture.invert, alpha: picture.alpha, css: shown, ...(picture.raw !== undefined ? { raw: picture.raw } : {}) },
  };
}
