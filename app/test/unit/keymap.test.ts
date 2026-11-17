import { describe, expect, it } from 'vitest';
import { keyInfo, keyPolicy, KEY_TABLE } from '../../src/input/keymap.ts';

const ev = (code: string, o: Partial<{ ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean; isComposing: boolean; keyCode: number }> = {}) => ({ code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...o });

describe('KeyboardEvent.code -> virtual key, scan code, extended flag', () => {
  it('letters: the virtual key is the capital letter, the scan codes are the US set-1 make codes', () => {
    for (const ch of 'ABCDEFGHIJKLMNOPQRSTUVWXYZ') expect(keyInfo(`Key${ch}`)?.vk).toBe(ch.charCodeAt(0));
    expect(keyInfo('KeyQ')).toEqual({ vk: 0x51, scan: 0x10, ext: false });
    expect(keyInfo('KeyA')).toEqual({ vk: 0x41, scan: 0x1e, ext: false });
    expect(keyInfo('KeyZ')).toEqual({ vk: 0x5a, scan: 0x2c, ext: false });
    expect(keyInfo('KeyM')).toEqual({ vk: 0x4d, scan: 0x32, ext: false });
  });
  it('digits and function keys', () => {
    expect(keyInfo('Digit1')).toEqual({ vk: 0x31, scan: 0x02, ext: false });
    expect(keyInfo('Digit9')).toEqual({ vk: 0x39, scan: 0x0a, ext: false });
    expect(keyInfo('Digit0')).toEqual({ vk: 0x30, scan: 0x0b, ext: false });
    expect(keyInfo('F1')).toEqual({ vk: 0x70, scan: 0x3b, ext: false });
    expect(keyInfo('F10')).toEqual({ vk: 0x79, scan: 0x44, ext: false });
    expect(keyInfo('F11')).toEqual({ vk: 0x7a, scan: 0x57, ext: false });
    expect(keyInfo('F12')).toEqual({ vk: 0x7b, scan: 0x58, ext: false });
  });
  it('Escape, Enter, Space (the keys of the G3 measurement: decision 0136 sent Space 0x20 / 0x390001 and Enter 0xd / 0x1c0001)', () => {
    expect(keyInfo('Escape')).toEqual({ vk: 0x1b, scan: 0x01, ext: false });
    expect(keyInfo('Enter')).toEqual({ vk: 0x0d, scan: 0x1c, ext: false });
    expect(keyInfo('Space')).toEqual({ vk: 0x20, scan: 0x39, ext: false });
  });
  it('modifiers are sent as the left / right virtual keys; the right Ctrl and Alt are extended', () => {
    expect(keyInfo('ShiftLeft')).toEqual({ vk: 0xa0, scan: 0x2a, ext: false });
    expect(keyInfo('ShiftRight')).toEqual({ vk: 0xa1, scan: 0x36, ext: false });
    expect(keyInfo('ControlLeft')).toEqual({ vk: 0xa2, scan: 0x1d, ext: false });
    expect(keyInfo('ControlRight')).toEqual({ vk: 0xa3, scan: 0x1d, ext: true });
    expect(keyInfo('AltLeft')).toEqual({ vk: 0xa4, scan: 0x38, ext: false });
    expect(keyInfo('AltRight')).toEqual({ vk: 0xa5, scan: 0x38, ext: true });
  });
  it('the extended keys of Learn WM_KEYDOWN: Ins, Del, Home, End, PgUp, PgDn, the arrows, keypad / and Enter', () => {
    const ext = ['Insert', 'Delete', 'Home', 'End', 'PageUp', 'PageDown', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'NumpadDivide', 'NumpadEnter', 'ControlRight', 'AltRight'];
    for (const c of ext) expect(keyInfo(c)?.ext, c).toBe(true);
    for (const c of ['Enter', 'Numpad0', 'Numpad7', 'NumpadAdd', 'KeyA', 'Space', 'ControlLeft']) expect(keyInfo(c)?.ext, c).toBe(false);
    expect(keyInfo('ArrowLeft')).toEqual({ vk: 0x25, scan: 0x4b, ext: true });
    expect(keyInfo('NumpadEnter')).toEqual({ vk: 0x0d, scan: 0x1c, ext: true });
  });
  it('punctuation (US layout) and the keypad with NumLock on', () => {
    expect(keyInfo('Semicolon')?.vk).toBe(0xba); expect(keyInfo('Equal')?.vk).toBe(0xbb); expect(keyInfo('Comma')?.vk).toBe(0xbc); expect(keyInfo('Minus')?.vk).toBe(0xbd);
    expect(keyInfo('Period')?.vk).toBe(0xbe); expect(keyInfo('Slash')?.vk).toBe(0xbf); expect(keyInfo('Backquote')?.vk).toBe(0xc0); expect(keyInfo('BracketLeft')?.vk).toBe(0xdb);
    expect(keyInfo('Backslash')?.vk).toBe(0xdc); expect(keyInfo('BracketRight')?.vk).toBe(0xdd); expect(keyInfo('Quote')?.vk).toBe(0xde);
    expect(keyInfo('Numpad0')?.vk).toBe(0x60); expect(keyInfo('Numpad9')?.vk).toBe(0x69); expect(keyInfo('NumpadMultiply')?.vk).toBe(0x6a); expect(keyInfo('NumpadDecimal')?.vk).toBe(0x6e);
  });
  it('no two codes share a virtual key and a scan code unless one is the extended / keypad twin', () => {
    const seen = new Map<string, string>();
    for (const [code, k] of Object.entries(KEY_TABLE)) {
      const id = `${k.vk}/${k.scan}/${k.ext}`;
      expect(seen.get(id), `${code} and ${seen.get(id)}`).toBeUndefined();
      seen.set(id, code);
      expect(k.vk).toBeGreaterThan(0); expect(k.vk).toBeLessThan(256); expect(k.scan).toBeGreaterThan(0); expect(k.scan).toBeLessThan(0x80);
    }
  });
  it('an unknown or inherited name has no entry (prototype properties are not keys)', () => {
    expect(keyInfo('Nope')).toBeNull();
    expect(keyInfo('constructor')).toBeNull();
    expect(keyInfo('__proto__')).toBeNull();
  });
});

describe('what the desktop does with a key', () => {
  it('forwards a key and suppresses the browser action (Space would scroll, Backspace would navigate)', () => {
    for (const c of ['Space', 'Enter', 'Escape', 'Backspace', 'ArrowDown', 'KeyS', 'F1', 'F3', 'F5']) expect(keyPolicy(ev(c)), c).toMatchObject({ forward: true, prevent: true });
    expect(keyPolicy(ev('KeyS', { ctrlKey: true }))).toMatchObject({ forward: true, prevent: true });   // Ctrl+S: no save dialog
    expect(keyPolicy(ev('KeyP', { ctrlKey: true }))).toMatchObject({ forward: true, prevent: true });
  });
  it('sends F5 to the engine and cancels the browser reload (a reload loses the project); Ctrl+F5 / Shift+F5 are the same key', () => {
    expect(keyPolicy(ev('F5'))).toMatchObject({ forward: true, prevent: true });
    expect(keyPolicy(ev('F5', { ctrlKey: true }))).toMatchObject({ forward: true, prevent: true });
    expect(keyPolicy(ev('F5', { shiftKey: true }))).toMatchObject({ forward: true, prevent: true });
  });
  it('leaves Tab and Ctrl+W to the browser: not forwarded, not suppressed', () => {
    expect(keyPolicy(ev('Tab'))).toMatchObject({ forward: false, prevent: false });
    expect(keyPolicy(ev('KeyW', { ctrlKey: true }))).toMatchObject({ forward: false, prevent: false });
    expect(keyPolicy(ev('KeyW', { metaKey: true }))).toMatchObject({ forward: false, prevent: false });
    expect(keyPolicy(ev('KeyW'))).toMatchObject({ forward: true, prevent: true });          // W alone is a key of the engine
  });
  it('Tab goes to the engine when the user asks for it', () => {
    expect(keyPolicy(ev('Tab'), { sendTab: true })).toMatchObject({ forward: true, prevent: true });
  });
  it('does not forward the Meta keys, an IME composition, or a code it does not know; and suppresses nothing for them', () => {
    expect(keyPolicy(ev('MetaLeft'))).toMatchObject({ forward: false, prevent: false });
    expect(keyPolicy(ev('KeyA', { isComposing: true }))).toMatchObject({ forward: false, prevent: false });
    expect(keyPolicy(ev('KeyA', { keyCode: 229 }))).toMatchObject({ forward: false, prevent: false });
    expect(keyPolicy(ev('MediaPlayPause'))).toMatchObject({ forward: false, prevent: false });
  });
});
