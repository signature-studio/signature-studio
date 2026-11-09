// Real browser input for the e2e tests (decision 0141): Chrome DevTools Protocol Input.dispatchMouseEvent / dispatchKeyEvent. The events they make are the browser's own (the page sees isTrusted === true pointer
// and keyboard events, which `Event.isTrusted` tells from script-made ones), and they go through the same listeners as a person's mouse and keyboard.
export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** the desktop view's rectangle in the page's CSS pixels */
export async function desktopRect(page) {
  return JSON.parse(await page.ev("JSON.stringify(document.getElementById('desktop').getBoundingClientRect().toJSON())"));
}

/** the CSS position (client coordinates) of a point of the engine's virtual screen */
export async function screenToClient(page, sx, sy) {
  const r = await desktopRect(page);
  // decision 0223: with display method D the desktop is ONE canvas whose bitmap is the picture of the virtual screen (its size follows the guest's display); the other methods show 1920 x 1080
  const [w, h] = await page.ev("(() => { const c = document.querySelector('#desktop canvas'); return window.__app.present === 'D' && c ? [c.width, c.height] : [1920, 1080]; })()");
  return { x: r.x + (sx + 0.5) * (r.width / w), y: r.y + (sy + 0.5) * (r.height / h) };
}

export async function mouse(page, type, sx, sy, { button = 'none', buttons = 0, clickCount = 0 } = {}) {
  const c = await screenToClient(page, sx, sy);
  await page.S('Input.dispatchMouseEvent', { type, x: c.x, y: c.y, button, buttons, clickCount, pointerType: 'mouse' });
}
export const moveTo = (page, sx, sy) => mouse(page, 'mouseMoved', sx, sy);
export const pressAt = (page, sx, sy, button = 'left') => mouse(page, 'mousePressed', sx, sy, { button, buttons: button === 'left' ? 1 : button === 'right' ? 2 : 4, clickCount: 1 });
export const releaseAt = (page, sx, sy, button = 'left') => mouse(page, 'mouseReleased', sx, sy, { button, buttons: 0, clickCount: 1 });

const KEYS = { Enter: { key: 'Enter', vk: 13, text: '\r' }, Space: { key: ' ', vk: 32, text: ' ' }, Escape: { key: 'Escape', vk: 27 }, Tab: { key: 'Tab', vk: 9 }, F5: { key: 'F5', vk: 116 }, ShiftLeft: { key: 'Shift', vk: 16 }, KeyA: { key: 'a', vk: 65, text: 'a' } };
export async function keyDown(page, code, modifiers = 0) {
  const k = KEYS[code];
  if (!k) throw new Error('key ' + code + ' is not in the e2e table');
  await page.S('Input.dispatchKeyEvent', { type: k.text ? 'keyDown' : 'rawKeyDown', code, key: k.key, windowsVirtualKeyCode: k.vk, nativeVirtualKeyCode: k.vk, modifiers, ...(k.text ? { text: k.text } : {}) });
}
export async function keyUp(page, code, modifiers = 0) {
  const k = KEYS[code];
  await page.S('Input.dispatchKeyEvent', { type: 'keyUp', code, key: k.key, windowsVirtualKeyCode: k.vk, nativeVirtualKeyCode: k.vk, modifiers });
}
export async function tap(page, code) { await keyDown(page, code); await sleep(40); await keyUp(page, code); }

/** waits until every input event the page sent has been processed by the engine (it was idle again after getting it) */
export async function settle(page, ms = 240000) {
  await page.waitFor('window.__app.latency.pending === 0', ms, 'all input processed');
}
