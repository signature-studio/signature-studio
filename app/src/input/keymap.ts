// KeyboardEvent.code -> the virtual-key code, scan code and extended flag the engine's fle_g3_key(kind, vk, scan, extended) takes (lift/runtime/fle_input.h; decision 0141).
// `code` is the PHYSICAL key (MDN KeyboardEvent.code: "isn't altered by keyboard layout or the state of the modifier keys"), so the table is the US layout of S-us-layout (decision 0136): the
// engine's input locale is 0x0409 and its TranslateMessage makes WM_CHAR from the virtual key and the shift state itself - the page sends no character.
// Scan codes are set 1 make codes (the same table as scan_of_vk in fle_input.c, which fills the scan in when the page sends 0). Extended keys (lParam bit 24) are those of Learn WM_KEYDOWN:
// "the right-hand ALT and CTRL keys ...; the INS, DEL, HOME, END, PAGE UP, PAGE DOWN, and arrow keys in the clusters to the left of the numeric keypad; and the divide (/) and ENTER keys in the numeric keypad".
// Left / right modifiers are sent as VK_LSHIFT ... VK_RMENU: the engine keeps GetKeyState of the generic code in step and puts the generic code in the message (fle_input.c translate_key).

export interface KeyInfo { vk: number; scan: number; ext: boolean }

const t: Record<string, KeyInfo> = {};
const add = (code: string, vk: number, scan: number, ext = false): void => { t[code] = { vk, scan, ext }; };

for (const [i, ch] of [...'ABCDEFGHIJKLMNOPQRSTUVWXYZ'].entries()) add(`Key${ch}`, 0x41 + i, 0);
const LETTER_SCAN: Record<string, number> = { Q: 0x10, W: 0x11, E: 0x12, R: 0x13, T: 0x14, Y: 0x15, U: 0x16, I: 0x17, O: 0x18, P: 0x19, A: 0x1e, S: 0x1f, D: 0x20, F: 0x21, G: 0x22, H: 0x23, J: 0x24, K: 0x25, L: 0x26, Z: 0x2c, X: 0x2d, C: 0x2e, V: 0x2f, B: 0x30, N: 0x31, M: 0x32 };
for (const [ch, sc] of Object.entries(LETTER_SCAN)) (t[`Key${ch}`] as KeyInfo).scan = sc;
for (let d = 1; d <= 9; d++) add(`Digit${d}`, 0x30 + d, 0x01 + d);
add('Digit0', 0x30, 0x0b);
for (let f = 1; f <= 10; f++) add(`F${f}`, 0x6f + f, 0x3a + f);
add('F11', 0x7a, 0x57); add('F12', 0x7b, 0x58);
add('Escape', 0x1b, 0x01); add('Minus', 0xbd, 0x0c); add('Equal', 0xbb, 0x0d); add('Backspace', 0x08, 0x0e); add('Tab', 0x09, 0x0f);
add('BracketLeft', 0xdb, 0x1a); add('BracketRight', 0xdd, 0x1b); add('Enter', 0x0d, 0x1c); add('ControlLeft', 0xa2, 0x1d);
add('Semicolon', 0xba, 0x27); add('Quote', 0xde, 0x28); add('Backquote', 0xc0, 0x29); add('ShiftLeft', 0xa0, 0x2a); add('Backslash', 0xdc, 0x2b);
add('Comma', 0xbc, 0x33); add('Period', 0xbe, 0x34); add('Slash', 0xbf, 0x35); add('ShiftRight', 0xa1, 0x36);
add('AltLeft', 0xa4, 0x38); add('Space', 0x20, 0x39); add('CapsLock', 0x14, 0x3a);
add('NumLock', 0x90, 0x45); add('ScrollLock', 0x91, 0x46); add('IntlBackslash', 0xe2, 0x56);
// the numeric keypad with NumLock on (S-numlock-on)
add('Numpad7', 0x67, 0x47); add('Numpad8', 0x68, 0x48); add('Numpad9', 0x69, 0x49); add('NumpadSubtract', 0x6d, 0x4a);
add('Numpad4', 0x64, 0x4b); add('Numpad5', 0x65, 0x4c); add('Numpad6', 0x66, 0x4d); add('NumpadAdd', 0x6b, 0x4e);
add('Numpad1', 0x61, 0x4f); add('Numpad2', 0x62, 0x50); add('Numpad3', 0x63, 0x51); add('Numpad0', 0x60, 0x52); add('NumpadDecimal', 0x6e, 0x53);
add('NumpadMultiply', 0x6a, 0x37);
// extended keys
add('NumpadEnter', 0x0d, 0x1c, true); add('ControlRight', 0xa3, 0x1d, true); add('NumpadDivide', 0x6f, 0x35, true); add('AltRight', 0xa5, 0x38, true);
add('PrintScreen', 0x2c, 0x37, true); add('Home', 0x24, 0x47, true); add('ArrowUp', 0x26, 0x48, true); add('PageUp', 0x21, 0x49, true);
add('ArrowLeft', 0x25, 0x4b, true); add('ArrowRight', 0x27, 0x4d, true); add('End', 0x23, 0x4f, true); add('ArrowDown', 0x28, 0x50, true);
add('PageDown', 0x22, 0x51, true); add('Insert', 0x2d, 0x52, true); add('Delete', 0x2e, 0x53, true); add('ContextMenu', 0x5d, 0x5d, true);
add('Pause', 0x13, 0x45);

export const KEY_TABLE: Readonly<Record<string, KeyInfo>> = t;

export function keyInfo(code: string): KeyInfo | null { return Object.prototype.hasOwnProperty.call(KEY_TABLE, code) ? (KEY_TABLE[code] as KeyInfo) : null; }

export interface KeyEventLike { code: string; ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean; isComposing?: boolean; keyCode?: number }
export interface KeyDecision {
  /** the key goes to the engine */
  forward: boolean;
  /** the browser's own action for it is suppressed (preventDefault) */
  prevent: boolean;
  why: string;
}

/**
 * What the desktop does with a key while it has the focus.
 *  - not forwarded and not suppressed (the browser keeps them): Tab (unless `sendTab`), Ctrl+W (and Cmd+W). Tab is how the keyboard leaves the desktop; Ctrl+W closes the tab (a page cannot stop it anyway).
 *  - F5 is the engine's (FL: show / hide the Playlist; Learn WM_KEYDOWN: a key message goes to the window with the focus): it is forwarded and the browser's reload is suppressed, like every other key of the table. Neither Chromium's reserved list (kReservedCommandIds) nor Firefox's `reserved="true"` keys contain F5, so a page may cancel it.
 *  - not forwarded: the Meta keys (the OS takes them), an IME composition (keyCode 229 / isComposing), a code the table does not know.
 *  - every other key is forwarded and its browser action suppressed (space and arrows scroll, Backspace, Ctrl+S, Ctrl+P, F1, ...), only while the desktop has the focus (the caller attaches this to the desktop element).
 */
export function keyPolicy(e: KeyEventLike, opts: { sendTab?: boolean } = {}): KeyDecision {
  if (e.isComposing || e.keyCode === 229) return { forward: false, prevent: false, why: 'ime' };
  if (e.code === 'Tab' && !opts.sendTab) return { forward: false, prevent: false, why: 'tab-leaves-the-desktop' };
  if (e.code === 'KeyW' && (e.ctrlKey || e.metaKey)) return { forward: false, prevent: false, why: 'close-tab' };
  if (e.code === 'MetaLeft' || e.code === 'MetaRight') return { forward: false, prevent: false, why: 'meta' };
  if (!keyInfo(e.code)) return { forward: false, prevent: false, why: 'unknown-code' };
  return { forward: true, prevent: true, why: 'forward' };
}
