// The raw input events the page sends to the engine Worker, in the vocabulary of lift/runtime/fle_input.h (decision 0136: the page makes NO Windows message; the engine's own input thread turns a raw event into
// the hit test, WM_NCHITTEST, WM_SETCURSOR, WM_MOUSEACTIVATE and WM_MOUSEMOVE / WM_*BUTTON* / WM_KEY* when the owning thread retrieves it).
export const IN = { MOVE: 1, BDOWN: 2, BUP: 3, WHEEL: 4, KDOWN: 5, KUP: 6, DISPLAY: 7, SYSCMD: 8 } as const;
/** the system commands the page may send (Learn WM_SYSCOMMAND; lift/runtime/fle_input.h fle_input_host_syscommand accepts SC_MINIMIZE 0xF020, SC_MAXIMIZE 0xF030, SC_CLOSE 0xF060 and SC_RESTORE 0xF120) */
export const SC = { RESTORE: 0xf120 } as const;
/** FLE_IN_BTN_L / R / M: note that the DOM numbers its buttons 0 = left, 1 = MIDDLE, 2 = right and its `buttons` mask 1 = left, 2 = RIGHT, 4 = MIDDLE */
export const BTN = { L: 0, R: 1, M: 2 } as const;

/** the virtual screen of the engine (GetSystemMetrics 1920 x 1080, 96 DPI: decision 0098) */
export const SCREEN = { width: 1920, height: 1080 } as const;

export type MouseRaw = { k: 'mouse'; kind: typeof IN.MOVE | typeof IN.BDOWN | typeof IN.BUP | typeof IN.WHEEL; x: number; y: number; data: number };
export type KeyRaw = { k: 'key'; kind: typeof IN.KDOWN | typeof IN.KUP; vk: number; scan: number; ext: number };
/** decision 0212: the display changed to width x height (lift/runtime/fle_input.h FLE_IN_DISPLAY): not a user's input; it travels the same road so that it reaches the engine only when the engine is idle */
export type DisplayRaw = { k: 'display'; kind: typeof IN.DISPLAY; width: number; height: number };
/** decision 0272: the taskbar button of window `lid` was pressed (lift/runtime/fle_input.h FLE_IN_SYSCMD: the engine sends WM_SYSCOMMAND(cmd) to the window when its thread next retrieves messages). `lid` is the number the engine itself reported (fle_g3_taskbar_iconic); the page names no window */
export type SyscmdRaw = { k: 'syscmd'; kind: typeof IN.SYSCMD; lid: number; cmd: number };
export type RawInput = MouseRaw | KeyRaw | DisplayRaw | SyscmdRaw;

export const describeRaw = (r: RawInput): string => {
  if (r.k === 'display') return `display change to ${r.width}x${r.height}`;
  if (r.k === 'syscmd') return `system command 0x${r.cmd.toString(16)} for window ${r.lid}`;
  if (r.k === 'key') return `${r.kind === IN.KDOWN ? 'key down' : 'key up'} vk 0x${r.vk.toString(16)} scan 0x${r.scan.toString(16)}${r.ext ? ' ext' : ''}`;
  const name = r.kind === IN.MOVE ? 'move' : r.kind === IN.BDOWN ? `button ${'LRM'[r.data] ?? r.data} down` : r.kind === IN.BUP ? `button ${'LRM'[r.data] ?? r.data} up` : `wheel ${r.data}`;
  return `${name} at screen (${r.x}, ${r.y})`;
};
