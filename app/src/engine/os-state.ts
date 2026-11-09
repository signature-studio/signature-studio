// Decision 0223 (desk1): the operating-system state that is not a picture, as the Worker reports it to the page: the cursor the guest chose (SetCursor), the title of the foreground window and the size of the virtual screen.
// This is what is left of the page's old window knowledge (main-window.ts chose "the main window" by its class, TFruityLoopsMainForm, to give the page its title; appview-poll.ts read the whole window list). Every item here is a
// general fact of the operating system and names no FL window: the title is the one a window shows in its title bar, and the foreground window is the one Windows calls active (Learn, GetForegroundWindow / WM_ACTIVATE).
import type { AppView } from './appview.ts';

export interface OsState { cursor: number; cursorIdc: number; cursorShared: boolean; title: string; screenW: number; screenH: number }

/**
 * `lastTitle` is the title shown so far. The new title is the title of the active, visible window whose title is not empty (a menu or a tooltip is a window with an empty title; it is active for a moment and must not blank the
 * page's title); when there is none, the last title stays. The top of the z-order is listed first, so the first active window wins.
 */
export function osStateOf(v: AppView, lastTitle: string): OsState {
  const act = v.windows.find((w) => w.active && w.visible && w.title.trim() !== '');
  return { cursor: v.cursor, cursorIdc: v.cursorIdc, cursorShared: v.cursorShared, title: act ? act.title.trim() : lastTitle, screenW: v.screenW ?? 0, screenH: v.screenH ?? 0 };
}

/** a change of the key is a change of the state: the Worker posts `osstate` only then */
export const osStateKey = (s: OsState): string => `${s.cursor}/${s.cursorIdc}/${s.cursorShared ? 1 : 0}/${s.screenW}x${s.screenH}/${s.title}`;
