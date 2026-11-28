// What a person does on the page, and what the page says: real mouse and key events (CDP Input, as the existing measurement tools send them),
// the guest's pixels mapped to the page's CSS points (the picture's size and the #desktop box), the engine's window table, and the engine's state.
// No product function is called to act: the engine is driven by its input only. The reads (state, windows, picture size) are the page's own test aids.
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
// a CDP call that does not answer within 20 s is an error (not a hang of the whole run)
const CDP_LIMIT_MS = 20000;
const timed = (p, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${CDP_LIMIT_MS} ms: ${what}`)), CDP_LIMIT_MS); })]).finally(() => clearTimeout(t));
};

const PARSE = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/;
/** the window table: one window per line, `lid N at (x, y) WxH ... class "..." title "..."` */
export function parseWindows(text) {
  return String(text).split('\n').map((l) => PARSE.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
    return { lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: names ? names[2] : '' };
  });
}

const GEOMETRY = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, title: document.title, standIn: document.body.innerText.includes('STAND-IN'), error: a.error ? String(a.error.what ?? a.error.message ?? a.error).slice(0, 600) : null, exit: a.exit ? { reason: a.exit.reason, code: a.exit.code, message: String(a.exit.message ?? '').slice(0, 400) } : null, logLength: a.engineLog().length }; })()`;

export class Guest {
  constructor(page, cdp, origin) {
    this.page = page;
    this.cdp = cdp;
    this.origin = origin;
    this.dumpRetries = 0;
  }
  send(method, params = {}) { return timed(this.cdp.send(method, params), method); }
  async ev(expression) {
    const r = await timed(this.page.evaluate(expression), 'evaluate').catch((e) => { throw new Error(`evaluate: ${String(e?.message ?? e).slice(0, 300)}`); });
    return r;
  }
  state() { return this.ev(STATE); }
  /** the window table. The page answers "ask again" while the picture is being shown (the render Worker has not read the last one): it is asked again every 100 ms, for up to 60 s; the number of asks is in dumpRetries (the scenario's JSON) */
  async windows() {
    const t0 = Date.now();
    for (;;) {
      try {
        return parseWindows(await timed(this.page.evaluate(async () => (await window.__app.deskDumpBase64(false)).windows), 'windows'));
      } catch (e) {
        if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 60000) throw e;
        this.dumpRetries++;
        await sleep(100);
      }
    }
  }
  async findWindow(pred) { return (await this.windows()).find(pred) ?? null; }
  /** the guest pixel (gx, gy) as a CSS point: the centre of the pixel on the #desktop box */
  async pointAt(gx, gy) {
    const g = await this.page.evaluate(GEOMETRY);
    return { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
  }
  /** the #desktop box in CSS px, and the guest's size in guest px */
  async desk() {
    const g = await this.page.evaluate(GEOMETRY);
    return { x: g.x, y: g.y, w: g.w, h: g.h, gw: g.gw, gh: g.gh };
  }
  /** guest pixels per CSS px of the #desktop box (1 at DPR 1 in the room of the 4K picture; 2 at DPR 2): the factor of offsets measured in CSS px */
  async guestPerCss() {
    const d = await this.desk();
    return d.gw / d.w;
  }
  /** a click at a CSS point (for what the page draws in CSS px, not in guest pixels) */
  async clickCss(x, y) {
    const c = { x, y };
    await this.move(c);
    await this.press(c, 'left', 1);
  }
  async move(c) {
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
  }
  /** a press and release; `modifiers` as CDP's (2 = Ctrl), 0 by default */
  async press(c, button, clickCount, modifiers = 0) {
    const buttons = button === 'right' ? 2 : 1;
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button, buttons, clickCount, modifiers, pointerType: 'mouse' });
    await sleep(50);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button, buttons: 0, clickCount, modifiers, pointerType: 'mouse' });
    await sleep(100);
  }
  /** a click (or `clicks` clicks in a row, the second with clickCount 2) with the left button, or the right one */
  async click(gx, gy, { button = 'left', clicks = 1 } = {}) {
    const c = await this.pointAt(gx, gy);
    await this.move(c);
    for (let k = 1; k <= clicks; k++) await this.press(c, button, k);
  }
  /** a right button press held for `holdMs`: the menu of FL opens on the press (measured by tools/feat-c6-popup-menus) */
  async rightClick(gx, gy, holdMs = 1000) {
    const c = await this.pointAt(gx, gy);
    await this.move(c);
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'right', buttons: 2, clickCount: 1, pointerType: 'mouse' });
    await sleep(holdMs);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'right', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    await sleep(300);
  }
  async dblclick(gx, gy) { await this.click(gx, gy, { clicks: 2 }); }
  /** wheel notches over a guest pixel; deltaY 120 is one notch down (tools/smoke/welcome-flow.mjs) */
  async wheel(gx, gy, notches, deltaY) {
    const c = await this.pointAt(gx, gy);
    for (let i = 0; i < notches; i++) {
      await this.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY, pointerType: 'mouse' });
      await sleep(300);
    }
  }
  /** the left button from one guest pixel to another, in `steps` moves */
  async drag(from, to, steps = 10) {
    const a = await this.pointAt(from.x, from.y), b = await this.pointAt(to.x, to.y);
    await this.move(a);
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: a.x, y: a.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
    for (let i = 1; i <= steps; i++) {
      await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: a.x + ((b.x - a.x) * i) / steps, y: a.y + ((b.y - a.y) * i) / steps, button: 'left', buttons: 1, pointerType: 'mouse' });
      await sleep(30);
    }
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: b.x, y: b.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    await sleep(100);
  }
  async key(code, key, vk, modifiers = 0) {
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
    await sleep(40);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
    await sleep(60);
  }
  /** Ctrl + a letter: Control goes down first (the engine reads the Control state from its own key messages) */
  async ctrl(code, key, vk) {
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 0xa2, nativeVirtualKeyCode: 0xa2, modifiers: 2 });
    await sleep(40);
    await this.key(code, key, vk, 2);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 0xa2, nativeVirtualKeyCode: 0xa2, modifiers: 0 });
    await sleep(40);
  }
  escape() { return this.key('Escape', 'Escape', 0x1b); }
  async focusDesktop() { await this.ev("document.getElementById('desktop').focus({ preventScroll: true }); true"); await sleep(100); }
  async waitFor(pred, ms, what) {
    const t0 = Date.now();
    let last = null;
    for (;;) {
      last = await pred().catch(() => null);
      if (last) return last;
      if (Date.now() - t0 >= ms) return null;
      await sleep(300);
    }
  }
  async shot(file) {
    const s = await this.send('Page.captureScreenshot', { format: 'png' });
    const fs = await import('node:fs');
    fs.writeFileSync(file, Buffer.from(s.data, 'base64'));
  }
}
