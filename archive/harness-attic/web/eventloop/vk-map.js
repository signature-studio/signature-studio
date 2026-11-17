'use strict';
// KeyboardEvent.code (physical key) -> [Windows virtual-key code, extended-key flag (lParam bit 24)]. Written from the public Windows VK list; NOT checked
// against a Windows machine or a real keyboard (docs/investigations/browser-event-loop.md, input section: 読み). Layout-dependent keys (VK_OEM_*) are the
// US-layout meaning: the engine's WM_KEYDOWN wParam for those depends on the user's layout, which `code` does not carry (`key` does).
const VK_MAP = (() => {
  const m = {};
  for (let i = 0; i < 26; i++) m['Key' + String.fromCharCode(65 + i)] = [0x41 + i, 0];
  for (let i = 0; i < 10; i++) { m['Digit' + i] = [0x30 + i, 0]; m['Numpad' + i] = [0x60 + i, 0]; }
  for (let i = 1; i <= 24; i++) m['F' + i] = [0x6f + i, 0];
  Object.assign(m, {
    Backspace: [0x08, 0], Tab: [0x09, 0], Enter: [0x0d, 0], NumpadEnter: [0x0d, 1], ShiftLeft: [0xa0, 0], ShiftRight: [0xa1, 0], ControlLeft: [0xa2, 0], ControlRight: [0xa3, 1],
    AltLeft: [0xa4, 0], AltRight: [0xa5, 1], Pause: [0x13, 0], CapsLock: [0x14, 0], Escape: [0x1b, 0], Space: [0x20, 0], PageUp: [0x21, 1], PageDown: [0x22, 1], End: [0x23, 1], Home: [0x24, 1],
    ArrowLeft: [0x25, 1], ArrowUp: [0x26, 1], ArrowRight: [0x27, 1], ArrowDown: [0x28, 1], PrintScreen: [0x2c, 1], Insert: [0x2d, 1], Delete: [0x2e, 1],
    MetaLeft: [0x5b, 1], MetaRight: [0x5c, 1], ContextMenu: [0x5d, 1], NumpadMultiply: [0x6a, 0], NumpadAdd: [0x6b, 0], NumpadSubtract: [0x6d, 0], NumpadDecimal: [0x6e, 0], NumpadDivide: [0x6f, 1],
    NumLock: [0x90, 1], ScrollLock: [0x91, 0], Semicolon: [0xba, 0], Equal: [0xbb, 0], Comma: [0xbc, 0], Minus: [0xbd, 0], Period: [0xbe, 0], Slash: [0xbf, 0], Backquote: [0xc0, 0],
    BracketLeft: [0xdb, 0], Backslash: [0xdc, 0], BracketRight: [0xdd, 0], Quote: [0xde, 0], IntlBackslash: [0xe2, 0],
    AudioVolumeMute: [0xad, 1], AudioVolumeDown: [0xae, 1], AudioVolumeUp: [0xaf, 1], MediaTrackNext: [0xb0, 1], MediaTrackPrevious: [0xb1, 1], MediaStop: [0xb2, 1], MediaPlayPause: [0xb3, 1],
    BrowserBack: [0xa6, 1], BrowserForward: [0xa7, 1], BrowserRefresh: [0xa8, 1], BrowserHome: [0xac, 1],
    KanaMode: [0x15, 0], Convert: [0x1c, 0], NonConvert: [0x1d, 0], Lang1: [0x15, 0], Lang2: [0x19, 0],
  });
  return m;
})();
// Windows has one VK for Shift/Control/Alt in WM_KEYDOWN wParam (0x10/0x11/0x12) and the left/right VK only through GetKeyState / MapVirtualKey; this returns both.
function vkOf(code) { const e = VK_MAP[code]; return e ? { vk: e[0], extended: e[1] === 1, generic: e[0] === 0xa0 || e[0] === 0xa1 ? 0x10 : e[0] === 0xa2 || e[0] === 0xa3 ? 0x11 : e[0] === 0xa4 || e[0] === 0xa5 ? 0x12 : e[0] } : null; }
if (typeof module !== 'undefined') module.exports = { VK_MAP, vkOf };
