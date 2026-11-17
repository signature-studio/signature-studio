// The cursor the guest chose (SetCursor / WM_SETCURSOR result, read through fle_av_poll: lift/runtime/fle_appview.h) as a CSS cursor value.
// Windows names (Learn "About Cursors", numbers as in lift/runtime/fle_cursor.h) -> MDN CSS `cursor` keywords. A handle of 0 is "no cursor" (Learn SetCursor: NULL removes the cursor from the screen) -> `none`.
// A cursor the guest loaded from one of its own modules (fle_cursor_is_shared_handle) has no standard identifier: it is shown as `default` until a picture of it is supplied (S-module-cursor-default).
import type { AppView } from '../engine/appview.ts';

export const IDC_CSS: Readonly<Record<number, string>> = {
  32512: 'default',      // IDC_ARROW
  32513: 'text',         // IDC_IBEAM
  32514: 'wait',         // IDC_WAIT
  32515: 'crosshair',    // IDC_CROSS
  32516: 'default',      // IDC_UPARROW (CSS has no up-arrow: S-uparrow-default)
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

export interface CursorChoice { css: string; why: string }

export function cursorCss(v: Pick<AppView, 'cursor' | 'cursorIdc' | 'cursorShared'> | null): CursorChoice {
  if (!v) return { css: 'default', why: 'the engine reports no cursor' };
  if (v.cursor === 0) return { css: 'none', why: 'SetCursor(NULL): the guest removed the cursor' };
  if (v.cursorIdc !== 0) {
    const css = IDC_CSS[v.cursorIdc];
    return css ? { css, why: `standard cursor IDC ${v.cursorIdc}` } : { css: 'default', why: `standard cursor IDC ${v.cursorIdc} has no CSS equivalent` };
  }
  if (v.cursorShared) return { css: 'default', why: `a cursor from a module of the guest (handle 0x${v.cursor.toString(16)}): shown as the arrow` };
  return { css: 'default', why: `a cursor the guest made itself (handle 0x${v.cursor.toString(16)}): shown as the arrow` };
}
