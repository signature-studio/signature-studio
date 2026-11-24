// The cursor the guest chose (SetCursor / WM_SETCURSOR result, read through fle_av_poll: lift/runtime/fle_appview.h) as a CSS cursor value.
// Windows names (Learn "About Cursors", numbers as in lift/runtime/fle_cursor.h) -> MDN CSS `cursor` keywords, one to one, only where both mean the same cursor (no look-alike: #1390).
// A standard cursor with no CSS equivalent (IDC_UPARROW) is shown as the arrow and is reported as `unmapped` (its name and value), so the page can count it.
// A handle of 0 is "no cursor" (Learn SetCursor: NULL removes the cursor from the screen) -> `none`.
// A cursor the guest loaded from one of its own modules (fle_cursor_is_shared_handle) has no standard identifier: it is shown as `default` until a picture of it is supplied (S-module-cursor-default; #1392).
// A cursor the guest made itself (CreateIconIndirect, cursorMade) is shown the same way until its picture is supplied (#1392): it is reported as `made`.
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

export interface CursorChoice {
  css: string;
  why: string;
  /** a standard cursor with no CSS equivalent (IDC_UNMAPPED): its name and its value */
  unmapped?: { name: string; value: number };
  /** a cursor the guest made itself (CreateIconIndirect): it has no standard identifier and no picture yet */
  made?: boolean;
  /** the handle is not none, not a standard cursor, not a cursor of a module and not one the guest made: the engine has no identity for it */
  unresolved?: boolean;
}

export function cursorCss(v: Pick<AppView, 'cursor' | 'cursorIdc' | 'cursorShared' | 'cursorMade'> | null): CursorChoice {
  if (!v) return { css: 'default', why: 'the engine reports no cursor' };
  if (v.cursor === 0) return { css: 'none', why: 'SetCursor(NULL): the guest removed the cursor' };
  if (v.cursorIdc !== 0) {
    const css = IDC_CSS[v.cursorIdc];
    if (css) return { css, why: `standard cursor IDC ${v.cursorIdc}` };
    const name = IDC_UNMAPPED[v.cursorIdc] ?? `IDC ${v.cursorIdc}`;
    return { css: 'default', why: `standard cursor ${name} (IDC ${v.cursorIdc}) has no CSS equivalent: shown as the arrow`, unmapped: { name, value: v.cursorIdc } };
  }
  if (v.cursorShared) return { css: 'default', why: `a cursor from a module of the guest (handle 0x${v.cursor.toString(16)}): shown as the arrow` };
  if (v.cursorMade) return { css: 'default', why: `a cursor the guest made itself (CreateIconIndirect, handle 0x${v.cursor.toString(16)}): shown as the arrow until its picture is supplied (#1392)`, made: true };
  return { css: 'default', why: `a handle with no identity (0x${v.cursor.toString(16)}): shown as the arrow`, unresolved: true };
}
