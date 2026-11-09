// Decision 0223 (desk1): the real engine on the desktop, with real browser input, on the new display path (the runtime composes ONE picture, the Worker shows it, the page knows no window).
// This is test/e2e/engine-desktop.mjs (decision 0141) migrated: the same one page load, in the order a person would do it, with the same goldens of decision 0136; what changed is WHERE the pixels are read:
//   old: the canvas of a window (window.__app.readbackAll(), appView.windows found by class, #desktop canvas[data-lid])      new: the picture the compositor holds (window.__app.deskDumpBase64(true), the part of it where
//   the compositor says the window is: the test finds the welcome wizard by its width, 874 (its height was 570 on the runtime of this morning and is 573 on HEAD 40d4f64's; the cause was not investigated), because the page - and the Worker - no longer know classes) and a SCREENSHOT of the page's one canvas.
// What each old check became is in docs/decisions/0223-desktop-compositor.md section 6. The class names below are used only to read the engine's own trace lines (FLE_G3_TRACE=1), never to find a window on the screen.
// Needs a bundle whose engine has the compositor (every engine build has it): FL_ENGINE_BUNDLE=<that bundle>. Window rectangles come from window.__app.deskDumpBase64 (the compositor's own text).
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { decodePng, toRgba } from '../../scripts/lib/png.mjs';
import { desktopRect, moveTo, pressAt, releaseAt, screenToClient, settle, sleep, tap, keyDown, keyUp } from './input-helpers.mjs';

const C = { main: 'TFruityLoopsMainForm', wizard: 'TWelcomeWizard' };

export const G3 = {
  wizardBefore: '1223c73ff0b5451d013cf8e76bf3f4e387c07000b2c1dc152e49031a030b3018',
  wizardPressed: 'f285760a566f94d07568d746083c9f8f04f7a23a34626faa368323794328673c',
  wizardReleased: '7bf902bdd63c0d123d032e97ecaadf3686d075efebbf28c7b164c83ee965a22f',
  settings: '7e34c76505bee14919d4761998d6e8fecaa9f103dadef0ea86d92cbe50879336',
  noThanks: { x: 1036, y: 561 },
};
const sha256 = (b) => crypto.createHash('sha256').update(b).digest('hex');
const pct = (a, p) => { const v = [...a].sort((x, y) => x - y); return v.length ? v[Math.min(v.length - 1, Math.max(0, Math.ceil((p / 100) * v.length) - 1))] : null; };

/** the RGBA bytes of a rectangle of the desktop in a screenshot taken at scale 1 */
function crop(rgba, w, x, y, cw, ch) {
  const out = Buffer.alloc(cw * ch * 4);
  for (let r = 0; r < ch; r++) rgba.copy(out, r * cw * 4, ((y + r) * w + x) * 4, ((y + r) * w + x + cw) * 4);
  return out;
}

/** the picture the compositor holds now (no compose, no redraw) with the text it wrote about the windows it drew, and its pixels as RGBA bytes */
async function peek(page, whole = false) {
  const d = await page.ev(`window.__app.deskDumpBase64(${whole ? 'false' : 'true'})`);
  return { windows: d.windows, width: d.width, height: d.height, rgba: Buffer.from(d.base64, 'base64') };
}
/** "lid 13 at (-5, -6) 1930x1092 layered ..." -> { lid, x, y, w, h, rest } (the compositor's text, fle_desk_describe) */
function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => ({ lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), rest: m[6] }));
}
const isWizard = (w) => w.w === 874 && w.h >= 560 && w.h <= 590;
const regionSha = (pk, r) => sha256(crop(pk.rgba, pk.width, r.x, r.y, r.w, r.h));
const diffPixels = (a, b) => { let n = 0; const A = new Uint32Array(a.buffer, a.byteOffset, a.length >> 2), B = new Uint32Array(b.buffer, b.byteOffset, b.length >> 2); for (let i = 0; i < A.length; i++) if (A[i] !== B[i]) n++; return n; };
/** the differing pixels of two RGBA pictures of the same size: their number and bounding box */
function diffBox(a, b, w = 1920) {
  const A = new Uint32Array(a.buffer, a.byteOffset, a.length >> 2), B = new Uint32Array(b.buffer, b.byteOffset, b.length >> 2);
  let n = 0, x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1;
  for (let i = 0; i < A.length; i++) if (A[i] !== B[i]) { n++; const x = i % w, y = (i - x) / w; if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; }
  return n === 0 ? { n } : { n, box: [x0, y0, x1 + 1, y1 + 1] };
}
/** the engine draws a meter in the toolbar by itself (about 13 to 83 pixels in a box of 84 x 14): two reads a few ms apart may differ there and nowhere else */
// the engine animates the meter of its toolbar column by itself (a 1-pixel-wide strip redrawn every frame, decision 0187: up to 13 rows at (1037..1120, 6..19), and a 1-pixel row at y = 36 of the same column, seen in the in-run identity tests of
// decision 0223 §11.3): the pixels two readings of the same moment can differ in are those, at that place (the box was a size, 120 x 20 anywhere, before desk2; it is a place now, 85 x 32, and at most 200 pixels)
const METER_BOX = [1037, 6, 1122, 38];
const onlyTheMeter = (d) => d.n === 0 || (d.n <= 200 && d.box[0] >= METER_BOX[0] && d.box[1] >= METER_BOX[1] && d.box[2] <= METER_BOX[2] && d.box[3] <= METER_BOX[3]);
/** the picture once it is at rest: two peeks 400 ms apart differ in fewer than 100 pixels */
async function stillPicture(page, ms = 30000) {
  const t0 = Date.now();
  let a = await peek(page);
  for (;;) {
    await sleep(400);
    const b = await peek(page);
    if (diffPixels(a.rgba, b.rgba) < 100) return b;
    a = b;
    if (Date.now() - t0 > ms) return b;
  }
}
async function waitRegion(page, r, want, ms) {
  const t0 = Date.now();
  let last = null;
  for (;;) {
    last = regionSha(await peek(page), r);
    if (last === want) return { ok: true, sha: last, ms: Date.now() - t0 };
    if (Date.now() - t0 > ms) return { ok: false, sha: last, ms: Date.now() - t0 };
    await sleep(50);
  }
}
/** waits until the region holds a picture other than `than` that has stayed the same for two readings 300 ms apart (the button is drawn in its new state, not half-way) */
async function waitRegionChange(page, r, than, ms) {
  const t0 = Date.now();
  let prev = null;
  for (;;) {
    const sha = regionSha(await peek(page), r);
    if (sha !== than && sha === prev) return { ok: true, sha, ms: Date.now() - t0 };
    prev = sha;
    if (Date.now() - t0 > ms) return { ok: false, sha, ms: Date.now() - t0 };
    await sleep(300);
  }
}
/** polls the region's SHA-256 for `ms` and returns the changes: [{ t, sha }] (the first reading included) */
async function traceRegion(page, r, ms, every = 250) {
  const t0 = Date.now(), out = [];
  while (Date.now() - t0 < ms) {
    const sha = regionSha(await peek(page), r);
    if (!out.length || out.at(-1).sha !== sha) out.push({ t: Date.now() - t0, sha });
    await sleep(every);
  }
  return out;
}
/** the engine is idle and the compositor draws the welcome wizard in front of the main window */
async function waitIdleWithWizard(page, ms = 400000) {
  await page.waitFor("window.__app && (window.__app.state === 'engine-idle' || window.__app.state === 'error' || window.__app.state === 'exited')", ms, 'engine idle');
  const t0 = Date.now();
  for (;;) {
    const ws = parseWindows((await peek(page)).windows);
    if (ws.some(isWizard) && ws.length >= 2) return ws;
    if (Date.now() - t0 > 60000) throw new Error('the compositor never drew the welcome wizard and the main window: ' + JSON.stringify(ws));
    await sleep(100);
  }
}
// The screenshot of the desktop (the part of the page where its canvas is), 1:1. Page.captureScreenshot's clip is in DOCUMENT coordinates and getBoundingClientRect is in VIEWPORT coordinates: they are the same only while the page is not scrolled.
// The page of a bundle whose manifest note makes #bundle-info two lines is 1302 px high in this 1300 px window and Chrome sometimes scrolls it by 2 px (decision 0232): the screenshot then showed the picture 2 px lower (290,000 pixels
// different, and none of them a missing update). The desktop's place is therefore read again at each screenshot and the scroll offset added.
const shotOf = async (page, w = 1920, h = 1080) => {
  const at = JSON.parse(await page.ev("JSON.stringify((() => { const b = document.getElementById('desktop').getBoundingClientRect(); return { x: b.x + window.scrollX, y: b.y + window.scrollY }; })())"));
  return Buffer.from((await page.S('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: true, clip: { x: at.x, y: at.y, width: w, height: h, scale: 1 } })).data, 'base64');
};

export async function engineDesktopScenarioD({ browser, base, target, exp0129, check, record, setGroup, logsDir, appExpr }) {
  setGroup(`${target}/engine-desk`);
  const name = 'engine-desk';
  const page = await browser.newPage({ width: 2400, height: 1300 });
  const marks = {};
  try {
    // a listener on the document sees every key and context-menu event AFTER the desktop's own listeners ran: it tells whether they called preventDefault
    await page.S('Page.addScriptToEvaluateOnNewDocument', { source: `window.__seen = { keydown: [], contextmenu: [], pointer: [] };
      document.addEventListener('keydown', (e) => window.__seen.keydown.push({ code: e.code, prevented: e.defaultPrevented, trusted: e.isTrusted }), true);
      document.addEventListener('keydown', (e) => window.__seen.keydown.push({ code: e.code, prevented: e.defaultPrevented, trusted: e.isTrusted, phase: 'after' }));
      document.addEventListener('contextmenu', (e) => window.__seen.contextmenu.push({ prevented: e.defaultPrevented, trusted: e.isTrusted }));
      document.addEventListener('pointerdown', (e) => window.__seen.pointer.push({ type: e.type, trusted: e.isTrusted, buttons: e.buttons, pointerType: e.pointerType }));
      document.addEventListener('pointerup', (e) => window.__seen.pointer.push({ type: e.type, trusted: e.isTrusted, buttons: e.buttons, pointerType: e.pointerType }));` });
    await page.goto(`${base}/`);
    const t0 = Date.now();
    // display method D marks "first pixel" at the engine's FIRST IDLE (the first present of the picture is made there), so the engine's start-up is looked at when its main() has started: the engine runs, has not yet been idle
    await page.waitFor("window.__app && window.__app.timeline && (window.__app.timeline.main_start || window.__app.state === 'error')", 600000, 'main() started');
    marks.mainStartWallS = (Date.now() - t0) / 1000;
    const startingStage = (await page.ev('window.__app.state')) === 'running';
    record('record: the page state when the engine\'s main() had started (the starting-stage checks need "running")', String(await page.ev('window.__app.state')));
    if (startingStage) check('the engine is running while it starts (state "running"), not yet idle', (await page.ev('window.__app.state')) === 'running');
    {
      // the PM's rule: only the stand-in engine's page carries the STAND-IN banner; the real engine's page has none
      const ban = JSON.parse(await page.ev(`JSON.stringify((() => { const e = document.getElementById('standin-banner'); return { text: e.textContent, hidden: e.hidden, display: getComputedStyle(e).display, kind: window.__app.engineKind, inPageText: document.body.innerText.includes('STAND-IN') }; })())`));
      check('the real engine\'s page has no STAND-IN banner (the banner is hidden and empty, engineKind is emscripten-fruity-v1)', ban.hidden && ban.text === '' && ban.display === 'none' && ban.kind === 'emscripten-fruity-v1' && !ban.inPageText, JSON.stringify(ban));
    }
    if (startingStage) {
      const early = await page.ev("JSON.stringify({ st: window.__app.engineStatus(), input: document.getElementById('desktop').dataset.input })");
      const e0 = JSON.parse(early);
      check('while the engine starts the status line says "starting" and the desktop takes no input', e0.st.state === 'starting' && e0.st.inputAccepted === false && e0.input === 'off', early);
      // input before the first idle: a real click on the (still unfinished) picture must NOT be sent
      const before = await page.ev('window.__app.inputEvents');
      await moveTo(page, 600, 400); await pressAt(page, 600, 400); await releaseAt(page, 600, 400);
      check('a click made while the engine is starting is not sent (the page counts 0 input events)', (await page.ev('window.__app.inputEvents')) === before && before === 0);
    } else record('record: NOT CHECKED - the starting-stage checks (status "starting", a click before the first idle is not sent): the first pixel was marked at the first idle, so the page was not in its starting stage when this script looked', fp);

    // (the Worker's three-way check of the first pixel, decision 0150, went with display method B: the picture the compositor holds is compared with the screen below)

    // ---- idle: the desktop ----
    const winsAtIdle = await waitIdleWithWizard(page, 400000);
    marks.idleWallS = (Date.now() - t0) / 1000;
    marks.timeline = await page.ev('window.__app.timeline');
    check('the engine reached its idle', (await page.ev('window.__app.state')) === 'engine-idle', `the compositor draws ${winsAtIdle.length} windows`);
    await page.waitFor('window.__app.idles.length >= 4', 240000, 'four idles');
    const pk0 = await stillPicture(page);
    const wins = parseWindows(pk0.windows);
    record('record: the compositor\'s window list at the idle (the text of fle_desk_describe)', pk0.windows.trim());
    const wWiz = wins.find(isWizard), wMain = wins.find((w) => w.w === 1930 && w.h === 1092);
    check('the compositor draws exactly two windows: the welcome wizard (874 wide, centred on the 1920x1080 screen) and the main window 1930x1092 at (-5, -6)', wins.length === 2 && !!wWiz && !!wMain && Math.abs(wWiz.x + wWiz.w / 2 - 960) <= 1 && Math.abs(wWiz.y + wWiz.h / 2 - 540) <= 2 && wMain.x === -5 && wMain.y === -6, pk0.windows.trim());
    check('the picture is the size of the virtual screen, 1920 x 1080 (scale=native keeps the legacy screen)', pk0.width === 1920 && pk0.height === 1080, `${pk0.width}x${pk0.height}`);
    // decision 0199 (G2) as the page does it now (decision 0223): document.title is the title of the foreground window as the Worker reports the engine's state; before that, the page's own title
    const baseTitle = await page.ev("window.__app.manifest.bundle.name + ' · FL Studio web (private)'");
    await page.waitFor("document.title === window.__app.pageTitle().title && window.__app.pageTitle().source === 'engine'", 30000, 'document.title follows the foreground window');
    const dt = JSON.parse(await page.ev("JSON.stringify({ title: document.title, pt: window.__app.pageTitle() })"));
    check('decision 0199 on the new path: document.title is the title of the engine\'s foreground window (not empty, not the page\'s own title), source "engine"', dt.title.trim() !== '' && dt.title !== baseTitle && dt.pt.source === 'engine', JSON.stringify({ documentTitle: dt.title, baseTitle, pt: dt.pt }));
    check('the status line says the engine is waiting or computing, with its input accepted', ['idle', 'computing'].includes(await page.ev('window.__app.engineStatus().state')) && (await page.ev('window.__app.engineStatus().inputAccepted')) === true);
    check('the page holds ONE canvas for the whole desktop (the page knows no window)', (await page.ev("document.querySelectorAll('#desktop canvas').length")) === 1, String(await page.ev("document.querySelectorAll('#desktop canvas').length")));

    // what the screen shows: the wizard in front of the main window, pixel for pixel, against the picture the compositor holds
    const rect = await desktopRect(page);
    check('the desktop is 1920x1080 CSS px at native scale', Math.round(rect.width) === 1920 && Math.round(rect.height) === 1080, JSON.stringify(rect));
    const full = Buffer.from((await page.S('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64');
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-desktop-idle-page.png`), full);
    const shot = await shotOf(page);
    {
      const pg = JSON.parse(await page.ev('JSON.stringify({ scrollX: window.scrollX, scrollY: window.scrollY, docHeight: document.documentElement.scrollHeight, innerHeight: window.innerHeight, docWidth: document.documentElement.scrollWidth, innerWidth: window.innerWidth })'));
      // decision 0232: the legacy layout of this window (2400 x 1300) holds the page whatever the bundle's manifest note is (the shipping bundle's note used to make the header 122 px: a 1302 px page, a scrollbar, a page that Chrome scrolled by 2 px)
      check('the page fits the window (no scrollbar, not scrolled): the document is not higher than the window and the page is at the top', pg.docHeight <= pg.innerHeight && pg.scrollY === 0 && pg.scrollX === 0, JSON.stringify(pg));
    }
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-desktop-idle.png`), shot);
    const rgba = toRgba(decodePng(shot));
    const cWiz = regionSha(pk0, wWiz);
    marks.wizardBefore = cWiz;
    const d0 = diffBox(rgba, pk0.rgba);
    check('window updates reach the screen: the screenshot of the page\'s canvas and the picture the compositor holds differ in nothing but the engine\'s own animated meter (0 of 2,073,600 pixels, or fewer than 200 in one box of at most 120 x 20)', onlyTheMeter(d0), JSON.stringify(d0));
    check('the screenshot region of the wizard is, pixel for pixel, the picture\'s: the wizard is drawn in front', sha256(crop(rgba, 1920, wWiz.x, wWiz.y, wWiz.w, wWiz.h)) === cWiz, `screenshot crop ${sha256(crop(rgba, 1920, wWiz.x, wWiz.y, wWiz.w, wWiz.h)).slice(0, 12)} picture ${cWiz.slice(0, 12)}`);
    check('main window (-5,-6): the screenshot of screen (100,300) 400x200 is the picture\'s, so it is drawn where the engine has it', sha256(crop(rgba, 1920, 100, 300, 400, 200)) === sha256(crop(pk0.rgba, 1920, 100, 300, 400, 200)));
    const dump0 = await peek(page, true);
    const dd0 = diffBox(dump0.rgba, pk0.rgba);
    check('the damage tracking lost nothing: the picture as held and the whole picture drawn again from the window table differ in nothing but the animated meter (at rest)', onlyTheMeter(dd0), JSON.stringify(dd0));
    // decision 0136's SHA-256 of the wizard (1223c73ff0...) is a fixed-virtual-clock value of that day's engine; the faces of the wizard differ from run to run on the engine of today (3 runs of the old path: 1d8f041c, 9422fdd9, 221e46db), so it is recorded, not required
    record('record: the wizard region before any input equals decision 0136 (1223c73ff0...)? and its SHA-256', `${cWiz === G3.wizardBefore} ${cWiz}`);

    // ---- the click on "No thanks" with real pointer events ----
    const nt = G3.noThanks;
    const tMove = Date.now();
    await moveTo(page, nt.x, nt.y);
    await settle(page);
    await sleep(5000);   // a repaint that the move might cause would come within a frame or two
    const shaMoved = regionSha(await peek(page), wWiz);
    check('a move onto the button does not change the wizard (decision 0136: move alone changes nothing)', shaMoved === cWiz, `got ${shaMoved?.slice(0, 12)} before ${cWiz.slice(0, 12)}`);
    const tDown = Date.now();
    await pressAt(page, nt.x, nt.y);
    const pressed = await waitRegionChange(page, wWiz, cWiz, 180000);
    marks.pressWallMs = Date.now() - tDown;
    check('PRESS with a real pointer event: the wizard\'s region of the picture changes (the button is drawn pressed)', pressed.ok, `got ${pressed.sha?.slice(0, 12)} after ${pressed.ms} ms`);
    record('record: the pressed face equals decision 0136 (f285760a56...)?', `${pressed.sha === G3.wizardPressed} ${pressed.sha}`);
    await settle(page);
    const hold = await traceRegion(page, wWiz, 4000);
    const heldSha = hold.at(-1)?.sha;   // the first reading is the pressed face
    check('held down for 4 s: the wizard stays pressed (no change of its region without the release)', hold.length === 1 && heldSha === pressed.sha, `region SHA changes while held: ${JSON.stringify(hold.map((h) => [h.t, h.sha?.slice(0, 10)]))}; the events so far: ${await page.ev("JSON.stringify(window.__app.latency.records.slice(-3).map((r) => r.desc + ' <' + r.from + '>'))")}`);
    const tUp = Date.now();
    await releaseAt(page, nt.x, nt.y);
    const released = await waitRegionChange(page, wWiz, pressed.sha, 180000);
    marks.releaseWallMs = Date.now() - tUp;
    check('RELEASE with a real pointer event: the wizard\'s region of the picture changes again (the button is released and the guest acts)', released.ok, `got ${released.sha?.slice(0, 12)} after ${released.ms} ms`);
    record('record: the released face equals decision 0136 (7bf902bdd6...)?', `${released.sha === G3.wizardReleased} ${released.sha}`);
    marks.clickTotalMs = Date.now() - tMove;
    const shotAfter = await shotOf(page);
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-desktop-after-no-thanks.png`), shotAfter);
    const pkAfter = await stillPicture(page);
    const wizAfter = sha256(crop(toRgba(decodePng(await shotOf(page))), 1920, wWiz.x, wWiz.y, wWiz.w, wWiz.h));
    check('the screenshot of the wizard after the click is the picture the compositor holds there (the screen shows what the compositor holds)', wizAfter === regionSha(pkAfter, wWiz), `screenshot ${wizAfter.slice(0, 12)} picture ${regionSha(pkAfter, wWiz).slice(0, 12)}`);
    const pointerSeen = JSON.parse(await page.ev('JSON.stringify(window.__seen.pointer)'));
    check('every pointerdown / pointerup the page saw was trusted', pointerSeen.length >= (startingStage ? 4 : 2) && pointerSeen.every((p) => p.trusted), JSON.stringify(pointerSeen.slice(0, 6)));

    // ---- browser defaults and context menu ----
    await moveTo(page, 200, 900);   // over the disabled main window: the engine drops mouse input to it, so this is harmless
    await page.S('Input.dispatchMouseEvent', { type: 'mousePressed', x: (await screenToClient(page, 200, 900)).x, y: (await screenToClient(page, 200, 900)).y, button: 'right', buttons: 2, clickCount: 1, pointerType: 'mouse' });
    await page.S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: (await screenToClient(page, 200, 900)).x, y: (await screenToClient(page, 200, 900)).y, button: 'right', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    await settle(page);
    const seen = await page.ev('JSON.stringify(window.__seen)');
    const sn = JSON.parse(seen);
    check('the right button: the page saw trusted pointerdown / pointerup events (they come from the browser, not from script)', sn.pointer.some((p) => p.type === 'pointerdown' && p.trusted && p.buttons === 2) && sn.pointer.some((p) => p.type === 'pointerup' && p.trusted), seen.slice(0, 300));
    check('the context menu is suppressed on the desktop (contextmenu event default prevented)', sn.contextmenu.length >= 1 && sn.contextmenu.every((c) => c.prevented && c.trusted), JSON.stringify(sn.contextmenu));
    const lastEv = await page.ev('JSON.stringify(window.__app.latency.records.slice(-3).map((r) => r.desc))');
    check('the right button reached the engine path as button R down / up', /button R down/.test(lastEv) && /button R up/.test(lastEv), lastEv);

    // ---- the key policy, with real key events ----
    await keyDown(page, 'KeyA'); await keyUp(page, 'KeyA');
    await keyDown(page, 'Tab'); await keyUp(page, 'Tab');
    await settle(page);
    check('Tab moved the browser focus off the desktop (it was left to the browser)', (await page.ev('document.activeElement && document.activeElement.id')) !== 'desktop');
    await page.ev("document.getElementById('desktop').focus()");
    const ks = JSON.parse(await page.ev('JSON.stringify(window.__seen.keydown.filter((k) => k.phase === "after"))'));
    const kA = ks.find((k) => k.code === 'KeyA'), kT = ks.find((k) => k.code === 'Tab');
    check('a real keydown of A: forwarded and the browser action suppressed (defaultPrevented)', kA && kA.prevented && kA.trusted, JSON.stringify(ks));
    check('a real keydown of Tab: left to the browser (not prevented, not forwarded)', kT && !kT.prevented, JSON.stringify(ks));
    const sentKeys = JSON.parse(await page.ev('JSON.stringify(window.__app.latency.records.filter((r) => /key/.test(r.desc)).map((r) => r.desc))'));
    check('the engine got key A down / up and no Tab', sentKeys.some((d) => /key down vk 0x41/.test(d)) && sentKeys.some((d) => /key up vk 0x41/.test(d)) && !sentKeys.some((d) => /vk 0x9\b/.test(d)), JSON.stringify(sentKeys));
    // a blur releases the keys the engine was told about: hold Shift, blur, expect a key up
    await keyDown(page, 'ShiftLeft');
    await settle(page);
    await page.ev("window.dispatchEvent(new Event('blur'))");
    await settle(page);
    const sk = JSON.parse(await page.ev('JSON.stringify(window.__app.latency.records.filter((r) => /vk 0xa0/.test(r.desc)).map((r) => r.desc))'));
    check('Shift held, then the window blurs: the engine gets a key up for it (no stuck key)', sk.length === 2 && /key down/.test(sk[0]) && /key up/.test(sk[1]), JSON.stringify(sk));


    // ---- pointer moves over the wizard: latency and the cursor ----
    const cursorsSeen = new Map();
    const spots = [[700, 330], [884, 561], [1036, 561], [960, 420], [1100, 700], [800, 600], [1250, 350], [640, 700]];
    const nMoves = Number(process.env.E2E_MOVES || 24);
    for (let i = 0; i < nMoves; i++) {
      const [bx, by] = spots[i % spots.length];
      await moveTo(page, bx + (i % 5), by + (i % 3));
      await settle(page);
      if (i % 6 === 5) {
        const cur = JSON.parse(await page.ev('JSON.stringify({ css: window.__app.cursor.css, why: window.__app.cursor.why, style: document.getElementById("desktop").style.cursor })'));
        cursorsSeen.set(`${cur.css} | ${cur.why}`, [bx, by]);
        check(`the desktop's CSS cursor is the one the engine chose (${cur.css}) after the move to (${bx}, ${by})`, cur.style === cur.css, JSON.stringify(cur));
      }
    }
    record('record: cursors the engine asked for over the wizard (css | why -> first spot)', JSON.stringify([...cursorsSeen.entries()]));
    // ---- a scaled desktop on a HiDPI screen: the same screen point is reached ----
    await page.S('Emulation.setDeviceMetricsOverride', { width: 1400, height: 900, deviceScaleFactor: 2, mobile: false });
    await page.ev("(() => { const s = document.getElementById('scale-mode'); s.value = 'fit'; s.dispatchEvent(new Event('change')); })()");
    const rsc = await desktopRect(page);
    const infoText = await page.ev("document.getElementById('canvas-info').textContent");
    check('device pixel ratio 2, "fit window" in a 1400x900 window: the desktop is smaller than the screen (scaled by CSS) and keeps the 16:9 shape', rsc.width < 1400 && Math.abs(rsc.width / rsc.height - 16 / 9) < 0.02 && /device pixel ratio 2/.test(infoText), JSON.stringify(rsc) + ' ' + infoText);
    const doms2 = JSON.parse(await page.ev("JSON.stringify([...document.querySelectorAll('#desktop canvas')].map((c) => ({ w: c.style.width, bw: c.width, bh: c.height })))"));
    const sc = rsc.width / 1920;
    check('the one canvas is scaled with the desktop (its bitmap is the 1920 x 1080 picture, shown 1920 x scale CSS px wide)', doms2.length === 1 && Math.abs(parseFloat(doms2[0].w) - 1920 * sc) <= 1 && doms2[0].bw === 1920 && doms2[0].bh === 1080, JSON.stringify(doms2));
    for (const [sx, sy] of [[1036, 561], [100, 100], [1800, 1000]]) {
      const before = (await page.ev('window.__app.latency.sentTotal'));
      await moveTo(page, sx, sy);
      await settle(page);
      const recs = JSON.parse(await page.ev(`JSON.stringify(window.__app.latency.records.slice(${before}).map((r) => r.desc))`));
      check(`scaled and HiDPI: a pointer move to the screen point (${sx}, ${sy}) reaches the engine as that screen point`, recs.some((d) => d === `move at screen (${sx}, ${sy})`), JSON.stringify(recs));
    }
    await page.S('Emulation.setDeviceMetricsOverride', { width: 2400, height: 1300, deviceScaleFactor: 1, mobile: false });
    await page.ev("(() => { const s = document.getElementById('scale-mode'); s.value = 'native'; s.dispatchEvent(new Event('change')); })()");

    await settle(page);
    // ---- the engine's idle-to-idle cycle (how long it computes between two idles) ----
    const idles = JSON.parse(await page.ev('JSON.stringify(window.__app.idles.map((i) => ({ at: i.at - performance.timeOrigin, busy: i.busyMs, now: i.now, next: i.next })))'));
    const busy = idles.map((i) => i.busy).filter((b) => b !== null);
    marks.engineCycle = { idles: idles.length, busyMs: { p50: pct(busy, 50), p95: pct(busy, 95), max: Math.max(...busy), min: Math.min(...busy) }, long: busy.filter((b) => b > 500).length };
    record('record: engine idle-to-idle compute time (ms)', JSON.stringify(marks.engineCycle));
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-idles.json`), JSON.stringify(idles));

    // ---- latency of all the input so far ----
    const lat = JSON.parse(await page.ev(`JSON.stringify(window.__app.latency.records.filter((r) => r.dropped === null).map((r) => ({ seq: r.seq, d: r.desc, kind: r.kind, page: r.tPage, inject: r.tInject === null ? null : r.tInject - r.tPage, idle: r.tIdle === null ? null : r.tIdle - r.tPage, frames: Object.fromEntries(Object.entries(r.frames).map(([k, v]) => [k, Math.round(v - r.tPage)])) })))`));
    const processed = lat.filter((r) => r.idle !== null);
    const toIdle = processed.map((r) => r.idle), toInject = processed.map((r) => r.inject);
    const anyFrame = processed.map((r) => Math.min(...Object.values(r.frames).concat([1e9]))).filter((v) => v < 1e9);
    const moves = processed.filter((r) => /move/.test(r.d));
    const clicks = processed.filter((r) => /button L/.test(r.d));
    const fWiz = clicks.map((r) => r.frames[0]).filter((v) => v !== undefined);   // the one picture is the only 'window' the latency tracker is told about (key 0)
    const latency = {
      events: lat.length, processed: processed.length,
      eventToInjectMs: { p50: pct(toInject, 50), p95: pct(toInject, 95), max: Math.max(...toInject) },
      eventToEngineIdleAgainMs: { p50: pct(toIdle, 50), p95: pct(toIdle, 95), max: Math.max(...toIdle) },
      eventToFirstFrameOfAnyWindowMs: { n: anyFrame.length, p50: pct(anyFrame, 50), p95: pct(anyFrame, 95), max: Math.max(...anyFrame) },
      movesToIdleMs: { n: moves.length, p50: pct(moves.map((r) => r.idle), 50), p95: pct(moves.map((r) => r.idle), 95) },
      leftButtonEventsToWizardFrameMs: fWiz,
    };
    marks.latency = latency;
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-latency.json`), JSON.stringify({ latency, events: lat }, null, 1));
    record('record: latency (ms)', JSON.stringify(latency));
    check('input was processed: at least 30 events reached the engine and came back idle', processed.length >= 30, `${processed.length} of ${lat.length}`);
    check('the status line shows the pending count and the last input', /input: \d+ sent, \d+ processed, \d+ pending/.test(await page.ev("document.getElementById('engine-metrics').textContent")));
    check('the engine\'s own counters: events injected, none rejected', (await page.ev('window.__app.driverStats?.injected')) >= 30 && (await page.ev('window.__app.driverStats?.rejected')) === 0, JSON.stringify(await page.ev('window.__app.driverStats')));

    // ---- Enter closes the wizard (decision 0145): the guest's layered window TLightweightLayeredControl (the logo) appears and is shown as a LAYERED window; the engine goes on ----
    const fullBefore = await shotOf(page);
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-before-enter-desktop.png`), fullBefore);
    await tap(page, 'Enter');
    const tEnter = Date.now();
    let gone = false, endState = null;
    for (let i = 0; i < 600; i++) {
      endState = await page.ev('window.__app.state');
      gone = !parseWindows((await peek(page)).windows).some(isWizard);
      if (gone || endState === 'exited' || endState === 'error') break;
      await sleep(500);
    }
    marks.enterToWizardGoneS = (Date.now() - tEnter) / 1000;
    check('after Enter the wizard is no longer among the windows the compositor draws, and the engine did not stop', gone && endState !== 'exited' && endState !== 'error', `gone ${gone} state ${endState} after ${marks.enterToWizardGoneS} s`);
    // decision 0145: the guest's layered logo window (TLightweightLayeredControl, UpdateLayeredWindow) may appear after the wizard; the old test waited for it. The compositor marks layered windows in its text: watch about a minute for one
    let seenLayered = null;
    for (let i = 0; i < 40 && !seenLayered; i++) {
      seenLayered = parseWindows((await peek(page)).windows).find((w) => /layered/.test(w.rest)) ?? null;
      const st = await page.ev('window.__app.state');
      if (seenLayered || st === 'exited' || st === 'error') break;
      await sleep(500);
    }
    record('record: a layered window among the windows the compositor draws after Enter (watched for about 40 looks, about a minute)', seenLayered ? JSON.stringify(seenLayered) : 'none seen');
    // a few more engine frames, then the picture at rest (layered windows are marked in the compositor's text)
    // (the page keeps the last 5000 idle reports: after minutes of idling the array's length no longer grows, the engine's own idle number n does)
    const n0 = await page.ev('window.__app.idles.at(-1)?.n ?? 0');
    await page.waitFor(`(window.__app.idles.at(-1)?.n ?? 0) >= ${n0 + 6}`, 240000, 'six more idles');
    await sleep(1500);
    const pk1 = await stillPicture(page);
    record('record: the compositor\'s window list after Enter (layered windows are marked "layered")', pk1.windows.trim());
    const shownShot = await shotOf(page);
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-after-enter-desktop.png`), shownShot);
    const d1 = diffBox(toRgba(decodePng(shownShot)), pk1.rgba);
    check('window updates reach the screen (after the wizard closed and the main window was drawn): the screenshot and the picture the compositor holds differ in nothing but the animated meter', onlyTheMeter(d1), JSON.stringify(d1));
    const dump1 = await peek(page, true);
    const dd1 = diffBox(dump1.rgba, pk1.rgba);
    check('the damage tracking lost nothing after the wizard closed: the picture as held and the whole picture drawn again from the window table differ in nothing but the animated meter', onlyTheMeter(dd1), JSON.stringify(dd1));
    for (const lw of parseWindows(pk1.windows).filter((w) => /layered/.test(w.rest))) {
      const opaqueBlack = (() => { let n = 0; for (let y = Math.max(0, lw.y); y < Math.min(1080, lw.y + lw.h); y++) for (let x = Math.max(0, lw.x); x < Math.min(1920, lw.x + lw.w); x++) { const o = (y * 1920 + x) * 4; if (pk1.rgba[o] === 0 && pk1.rgba[o + 1] === 0 && pk1.rgba[o + 2] === 0) n++; } return n; })();
      check(`a layered window the engine shows (lid ${lw.lid}, ${lw.w}x${lw.h}) is not a black box: fewer than 1% of its area is pure black in the picture (F2 drew 158,316 black pixels there)`, opaqueBlack < lw.w * lw.h * 0.01, `${opaqueBlack} of ${lw.w * lw.h}`);
    }
    // the engine goes on and takes input after the wizard is gone
    const sentBefore = await page.ev('window.__app.inputEvents');
    await moveTo(page, 300, 300);
    await settle(page);
    check('after the wizard is closed the desktop still takes input (the engine is not stopped): a pointer move is sent and processed', (await page.ev('window.__app.inputEvents')) === sentBefore + 1 && (await page.ev('window.__app.latency.pending')) === 0 && (await page.ev('window.__app.state')) !== 'exited', `state ${await page.ev('window.__app.state')}`);
    check('the status line does not say stopped', (await page.ev('window.__app.engineStatus().state')) !== 'stopped');
    const sum = await page.ev('window.__app.summary');
    record('record: engine stop message (empty: the engine runs)', sum?.runtimeMessage ?? '');
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-engine-log.txt`), (await page.ev('window.__app.engineLog()')).join('\n') + '\n');
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-after-enter-page.png`), Buffer.from((await page.S('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64'));
    const errs = page.consoleLog.filter((l) => l.startsWith('error') || l.startsWith('EXCEPTION'));
    check('no console errors or exceptions', errs.length === 0, errs.join(' | ').slice(0, 400));
    const rs = [...page.responses.keys()];
    check('no response from any origin other than the app\'s own', rs.filter((u) => !u.startsWith(base) && !u.startsWith('data:') && !u.startsWith('blob:')).length === 0);
    record('record: marks', JSON.stringify(marks));
    fs.writeFileSync(path.join(logsDir, `${target}-${name}-marks.json`), JSON.stringify(marks, null, 1));
  } catch (e) {
    check('scenario ran to completion', false, String(e.stack ?? e).slice(0, 800));
    try { fs.writeFileSync(path.join(logsDir, `${target}-${name}-failure-page.png`), Buffer.from((await page.S('Page.captureScreenshot', { format: 'png', fromSurface: true })).data, 'base64')); } catch { /* the page is gone */ }
    try { fs.writeFileSync(path.join(logsDir, `${target}-${name}-failure-console.txt`), page.consoleLog.join('\n')); } catch { /* ignore */ }
  }
  await page.close();
  return marks;
}

/** 'End session' while the engine is idle (suspended in fle_idle_wait): the engine is told to end its wait (fleIdleDeliver(-1)), exits by its own rule, and the page says the session was ended from the page - not that the engine hit a wall */
export async function engineEndSessionScenarioD({ browser, base, target, check, record, setGroup }) {
  setGroup(`${target}/engine-desk-end-session`);
  const page = await browser.newPage({ width: 2400, height: 1300 });
  try {
    await page.goto(`${base}/`);
    await waitIdleWithWizard(page, 400000);
    await page.waitFor('window.__app.idles.length >= 4', 240000, 'four idles');
    // the engine is idle only for a few ms between two frames: call stop() the moment the page's status says so
    const hit = await page.ev(`new Promise((res) => { const t0 = performance.now(); const t = setInterval(() => { if (window.__app.engineStatus().state === 'idle') { clearInterval(t); window.__app.stop(); res(true); } else if (performance.now() - t0 > 90000) { clearInterval(t); res(false); } }, 2); })`);
    check('stop() could be called while the engine was idle', hit === true);
    await page.waitFor("window.__app.state === 'exited'", 60000, 'exited after End session');
    const ex = await page.ev('window.__app.exit');
    const txt = await page.ev("document.getElementById('exit').textContent");
    record('record: End session exit and message', JSON.stringify({ ex, txt: txt.slice(0, 500) }));
    check('the engine exited on its own rule (an exit code, not a terminated Worker)', ex && ex.reason === 'stopped' && ex.code >= 0, JSON.stringify(ex));
    // the words are whatever runtime message the engine left ("fruity: runtime message: ..." in its log): in the engine of HEAD 741d626 they were "idle: the host ended the wait", in the shipping engine of decision 0218 the last message is a
    // later one (the first pixel of a layered window written during the wind-down). The page must quote that line verbatim (desk2: the wording of the engine is not what this checks)
    const said = await page.ev("(window.__app.engineLog().filter((l) => l.includes('fruity: runtime message: ')).pop() ?? '').split('fruity: runtime message: ')[1] ?? ''");
    check('the page says the session was ended from the page and quotes the engine\'s own last words verbatim; it does not call it a wall', /The session was ended from the page \(End session\)/.test(txt) && said.length > 0 && txt.includes(said.slice(0, 120)) && !/at a wall/.test(txt), JSON.stringify({ said: said.slice(0, 160), txt: txt.slice(0, 300) }));
    await sleep(600);   // data-input is refreshed every 250 ms
    check('the status line says stopped and the desktop takes no input', (await page.ev('window.__app.engineStatus().state')) === 'stopped' && (await page.ev("document.getElementById('desktop').dataset.input")) === 'off');
    const errs = page.consoleLog.filter((l) => l.startsWith('error') || l.startsWith('EXCEPTION'));
    check('no console errors or exceptions', errs.length === 0, errs.join(' | ').slice(0, 400));
  } catch (e) {
    check('scenario ran to completion', false, String(e.stack ?? e).slice(0, 800));
  }
  await page.close();
}

/** The screen follows the room (decision 0212 screen1, on the new path): the page tells the engine the size of the room, the compositor makes a picture of the screen's size and the Worker's canvas takes it; the screen shows the picture.
 *  With an engine build that has the display-change entry (fle_g3_display) a change of the viewport while the engine runs is told to the engine (WM_DISPLAYCHANGE) and the picture changes size; a build without
 *  it (the test bundles of decision 0223 are made from objects that predate it) cannot do that: it is recorded as NOT CHECKED, and the screen size is checked the way it works there - a page opened in a smaller window. */
export async function engineResizeScenarioD({ browser, base, target, check, record, setGroup, logsDir }) {
  setGroup(`${target}/engine-desk-resize`);
  const canvasSize = async (page) => JSON.parse(await page.ev("JSON.stringify((() => { const c = document.querySelector('#desktop canvas'); const r = c.getBoundingClientRect(); return { bw: c.width, bh: c.height, cssW: r.width, cssH: r.height }; })())"));
  const identity = async (page, pk, label) => {
    const rect = await desktopRect(page);
    const shot = await shotOf(page, pk.width, pk.height);
    fs.writeFileSync(path.join(logsDir, `${target}-engine-desk-resize-${label}.png`), shot);
    const pk2 = await stillPicture(page);
    const sameSize = Math.abs(Math.round(rect.width) - pk2.width) <= 1;
    const dB = sameSize ? diffBox(toRgba(decodePng(shot)), pk2.rgba, pk2.width) : { n: -1 };
    check(`the screen shows the picture (${label}): the screenshot and the picture differ in nothing but the animated meter (the canvas on the screen is the picture's size, 1 px = 1 px)`, sameSize && onlyTheMeter(dB), `canvas rect ${JSON.stringify(rect)}, picture ${pk2.width}x${pk2.height}, ${JSON.stringify(dB)}`);
  };
  const open = async (width, height) => {
    const page = await browser.newPage({ width, height });
    await page.goto(`${base}/`);
    await waitIdleWithWizard(page, 400000);
    await page.waitFor('window.__app.idles.length >= 4', 240000, 'four idles');
    return page;
  };
  let page = null;
  try {
    page = await open(2400, 1300);
    const pkA = await stillPicture(page);
    const cA = await canvasSize(page);
    record('record: the picture in a 2400x1300 window', JSON.stringify({ w: pkA.width, h: pkA.height, canvas: cA }));
    check('fit mode in a 2400x1300 window: the guest screen is not the legacy 1920x1080 but what fits the room, and the canvas bitmap is the compositor\'s picture', cA.bw === pkA.width && cA.bh === pkA.height && (pkA.width !== 1920 || pkA.height !== 1080), JSON.stringify({ cA, pk: [pkA.width, pkA.height] }));
    await identity(page, pkA, 'first window');
    const follows = await page.ev('window.__app.screen.engineFollows');
    if (follows === true) {
      await page.S('Emulation.setDeviceMetricsOverride', { width: 1500, height: 900, deviceScaleFactor: 1, mobile: false });
      await page.waitFor(`(() => { const c = document.querySelector('#desktop canvas'); return c && (c.width !== ${cA.bw} || c.height !== ${cA.bh}); })()`, 120000, 'the picture changes size after the viewport change');
      await page.waitFor('window.__app.idles.length >= 4', 240000, 'idles after the change').catch(() => {});
      const pkB = await stillPicture(page);
      const cB = await canvasSize(page);
      check('after the viewport shrank the picture is smaller and the canvas bitmap equals it (the screen followed the room while the engine ran)', pkB.width < pkA.width && pkB.height <= pkA.height && cB.bw === pkB.width && cB.bh === pkB.height, JSON.stringify({ pkB: [pkB.width, pkB.height], cB }));
      await identity(page, pkB, 'after the live change');
    } else {
      record('record: NOT CHECKED - a display change while the engine runs', `window.__app.screen.engineFollows is ${follows}: this engine build has no display-change entry (fle_g3_display); the compositor\'s and the presenter\'s part of a size change is checked by the group compositor-synth (1920x1080 -> 1280x800 -> back) and by desk1_compose.rs (two display changes in 1500 random steps)`);
      await page.close();
      page = await open(1500, 900);
      const pkB = await stillPicture(page);
      const cB = await canvasSize(page);
      record('record: the picture in a 1500x900 window', JSON.stringify({ w: pkB.width, h: pkB.height, canvas: cB }));
      check('a page opened in a smaller window (1500x900) gets a smaller guest screen: the picture is smaller than in the 2400x1300 window and the canvas bitmap equals it', pkB.width < pkA.width && pkB.height < pkA.height && cB.bw === pkB.width && cB.bh === pkB.height, JSON.stringify({ pkB: [pkB.width, pkB.height], pkA: [pkA.width, pkA.height], cB }));
      check('the canvas is shown whole in the window (CSS size within the viewport)', cB.cssW <= 1500 + 1 && cB.cssH <= 900 + 1, JSON.stringify(cB));
      await identity(page, pkB, 'second, smaller window');
    }
    const errs = page.consoleLog.filter((l) => l.startsWith('error') || l.startsWith('EXCEPTION'));
    check('no console errors or exceptions', errs.length === 0, errs.join(' | ').slice(0, 400));
  } catch (e) {
    check('scenario ran to completion', false, String(e.stack ?? e).slice(0, 800));
  }
  if (page) await page.close();
}
