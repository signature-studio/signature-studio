// node run-input.mjs [name=input-coi] [moves=2000]   inject input over CDP, measure PointerEvent -> ring -> Worker and probe what the browser lets a page do
import fs from 'node:fs';
import path from 'node:path';
import { withBrowser, argv, OUT } from './lib.mjs';
const name = argv.name || 'input-coi';
const MOVES = +(argv.moves || 2000);
const out = await withBrowser({ name, page: '/coi/input.html', timeoutS: 120, gpu: false }, async ({ S, evalJSON, waitDone, cdp, base }) => {
  await waitDone(20);
  const R = {};
  try { await cdp.send('Browser.grantPermissions', { permissions: ['clipboardReadWrite', 'clipboardSanitizedWrite'], origin: base }); R.grant = 'ok'; } catch (e) { R.grant = String(e).slice(0, 150); }
  try { await S('Page.bringToFront'); await S('Emulation.setFocusEmulationEnabled', { enabled: true }); R.bringToFront = 'ok'; } catch (e) { R.bringToFront = String(e).slice(0, 100); }
  R.probe = JSON.parse(await evalJSON('window.probe().then(JSON.stringify)'));
  R.focusPad = await evalJSON('window.focusPad()');
  const mouse = (type, x, y, extra) => S('Input.dispatchMouseEvent', Object.assign({ type, x, y, button: 'none', buttons: 0 }, extra));
  let t = Date.now();
  for (let k = 0; k < MOVES; k++) await mouse('mouseMoved', 50 + (k % 400), 60 + ((k * 3) % 200));
  R.seenAfterMoves = await evalJSON('JSON.stringify(window.__seenNow())');
  R.cdp_move_roundtrip_ms = +((Date.now() - t) / MOVES).toFixed(3);
  for (let k = 0; k < 100; k++) { await mouse('mousePressed', 100, 100, { button: 'left', buttons: 1, clickCount: 1 }); await mouse('mouseReleased', 100, 100, { button: 'left', buttons: 0, clickCount: 1 }); }
  for (let k = 0; k < 3; k++) { await mouse('mousePressed', 120, 100, { button: 'right', buttons: 2, clickCount: 1 }); await mouse('mouseReleased', 120, 100, { button: 'right', buttons: 0, clickCount: 1 }); }
  for (let k = 0; k < 50; k++) await S('Input.dispatchMouseEvent', { type: 'mouseWheel', x: 100, y: 100, deltaX: 0, deltaY: 100 });
  const keyList = [['KeyA', 'a', 65, 'a'], ['Digit1', '1', 49, '1'], ['Enter', 'Enter', 13, '\r'], ['Space', ' ', 32, ' '], ['ArrowLeft', 'ArrowLeft', 37], ['F9', 'F9', 120], ['ShiftLeft', 'Shift', 16], ['ControlLeft', 'Control', 17], ['Numpad5', '5', 101, '5'], ['Backspace', 'Backspace', 8]];
  for (const [code, key, vk, text] of keyList) {
    await S('Input.dispatchKeyEvent', Object.assign({ type: text ? 'keyDown' : 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk }, text ? { text } : {}));
    await S('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
  }
  // drag and drop of a file
  const f = path.join(OUT, 'tmp', 'dropme.bin');
  fs.writeFileSync(f, Buffer.from([0x52, 0x49, 0x46, 0x46, 1, 2, 3, 4, 5, 6, 7, 8]));
  try {
    const data = { items: [], files: [f], dragOperationsMask: 1 };
    await S('Input.dispatchDragEvent', { type: 'dragEnter', x: 100, y: 100, data });
    await S('Input.dispatchDragEvent', { type: 'dragOver', x: 100, y: 100, data });
    await S('Input.dispatchDragEvent', { type: 'drop', x: 100, y: 100, data });
    R.dragDrop = 'dispatched';
  } catch (e) { R.dragDrop = String(e).slice(0, 200); }
  // IME composition into a text area
  try {
    R.focusIme = await evalJSON('window.focusIme()');
    await S('Input.imeSetComposition', { text: 'に', selectionStart: 1, selectionEnd: 1 });
    await S('Input.imeSetComposition', { text: 'にほ', selectionStart: 2, selectionEnd: 2 });
    await S('Input.insertText', { text: '日本' });
    R.ime = JSON.parse(await evalJSON('JSON.stringify(window.imeLog())'));
  } catch (e) { R.ime = String(e).slice(0, 200); }
  // pointer lock, fullscreen and clipboard need transient user activation: one injected click before each
  const click = async () => { await mouse('mousePressed', 100, 100, { button: 'left', buttons: 1, clickCount: 1 }); await mouse('mouseReleased', 100, 100, { button: 'left', buttons: 0, clickCount: 1 }); };
  await evalJSON('window.focusPad()');
  R.gesture = {};
  await click(); R.gesture.pointerLock = await evalJSON('window.tryPointerLock(false)');
  await click(); R.gesture.pointerLockUnadjusted = await evalJSON('window.tryPointerLock(true)');
  await click(); R.gesture.fullscreen = await evalJSON('window.tryFullscreen()');
  await click(); R.gesture.clipboardKeyboard = await evalJSON('window.tryClipboardKeyboard()');
  R.summary = await evalJSON('window.inputFinish()');
  return R;
});
console.log(JSON.stringify({ name, error: out.error, pids_left: out.chrome_pids_left }));
