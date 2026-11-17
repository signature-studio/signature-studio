'use strict';
// Input path: DOM event handler on the page -> ring (Atomics.notify) -> Worker blocked in Atomics.wait. The driver (run-input.mjs) injects the events over CDP
// (Input.dispatchMouseEvent / dispatchKeyEvent / dispatchDragEvent / imeSetComposition); this page records what the browser delivered.
const pad = document.getElementById('pad');
const sab = ringAlloc(4096);
const ring = new Ring(sab);
let seq = 0;
const qd = [];       // handler time - event.timeStamp (ms): time the event waited inside the browser before the handler ran
const hcost = [];    // time spent in the handler incl. ring.post (us)
const coal = [];
const seen = { pointermove: 0, pointerdown: 0, pointerup: 0, mousemove: 0, wheel: 0, keydown: 0, keyup: 0, contextmenu: 0, dropFiles: 0 };
const keys = [];
const wheels = [];
const imeLog = [];
const misc = {};

function rec(type, e, a, b) {
  const t = performance.now();
  qd.push(t - e.timeStamp);
  ring.post(type, ++seq, a, b, wallNow());
  hcost.push((performance.now() - t) * 1000);
}
pad.addEventListener('pointermove', (e) => { seen.pointermove++; coal.push(e.getCoalescedEvents ? e.getCoalescedEvents().length : -1); rec(EV.MOVE, e, e.clientX, e.clientY); });
pad.addEventListener('pointerdown', (e) => { seen.pointerdown++; misc.lastPointer = { type: e.pointerType, button: e.button, buttons: e.buttons, pressure: e.pressure, pointerId: e.pointerId, isPrimary: e.isPrimary }; rec(EV.DOWN, e, e.clientX, e.clientY); });
pad.addEventListener('pointerup', (e) => { seen.pointerup++; rec(EV.UP, e, e.clientX, e.clientY); });
pad.addEventListener('mousemove', () => { seen.mousemove++; });
pad.addEventListener('contextmenu', (e) => { seen.contextmenu++; });
pad.addEventListener('wheel', (e) => { seen.wheel++; wheels.push({ dy: e.deltaY, dx: e.deltaX, mode: e.deltaMode, ctrl: e.ctrlKey }); rec(EV.KEY, e, e.deltaX, e.deltaY); }, { passive: false });
pad.addEventListener('keydown', (e) => { seen.keydown++; keys.push({ t: 'd', key: e.key, code: e.code, keyCode: e.keyCode, loc: e.location, rep: e.repeat }); rec(EV.KEY, e, e.keyCode, 1); });
pad.addEventListener('keyup', (e) => { seen.keyup++; keys.push({ t: 'u', key: e.key, code: e.code, keyCode: e.keyCode, loc: e.location }); rec(EV.KEY, e, e.keyCode, 0); });
pad.addEventListener('dragover', (e) => { e.preventDefault(); });
pad.addEventListener('drop', async (e) => {
  e.preventDefault();
  const fs = [...e.dataTransfer.files];
  seen.dropFiles = fs.length;
  misc.drop = { types: [...e.dataTransfer.types], files: [] };
  for (const f of fs) {
    const t0 = performance.now();
    const buf = await f.arrayBuffer();
    misc.drop.files.push({ name: f.name, size: f.size, type: f.type, readBytes: buf.byteLength, read_ms: +(performance.now() - t0).toFixed(2), first: [...new Uint8Array(buf.slice(0, 4))] });
  }
});
const ime = document.getElementById('ime');
for (const n of ['compositionstart', 'compositionupdate', 'compositionend', 'beforeinput', 'input']) ime.addEventListener(n, (e) => imeLog.push({ type: n, data: e.data, inputType: e.inputType, composing: e.isComposing }));

const wk = new Worker('ma-worker.js');
let wres;
const wdone = new Promise((r) => { wres = r; });
wk.onmessage = (e) => { if (e.data.type === 'result') wres(e.data.result); };
wk.postMessage({ mode: 'busy', sab, work: 0 });

window.inputFinish = async () => {
  Atomics.store(ring.i, RG.STOP, 1);
  ring.post(EV.QUIT, 0, 0, 0, 0);
  const w = await wdone;
  return { seen, queueDelay_ms: summarize(qd), handlerCost_us: summarize(hcost), coalescedPerMove: summarize(coal, [50, 95, 99]), worker: w, posted: seq, wheels: wheels.slice(0, 3), keys: keys.slice(0, 40), lastPointer: misc.lastPointer, drop: misc.drop };
};
window.probe = async () => {
  const R = {};
  R.apis = {
    PointerEvent: typeof PointerEvent, pointerrawupdate: 'onpointerrawupdate' in window, getCoalescedEvents: typeof PointerEvent.prototype.getCoalescedEvents,
    getPredictedEvents: typeof PointerEvent.prototype.getPredictedEvents, EditContext: typeof self.EditContext, clipboard: !!navigator.clipboard,
    keyboardLock: !!(navigator.keyboard && navigator.keyboard.lock), requestPointerLock: typeof Element.prototype.requestPointerLock, requestFullscreen: typeof Element.prototype.requestFullscreen,
    showOpenFilePicker: typeof self.showOpenFilePicker, showDirectoryPicker: typeof self.showDirectoryPicker, getGamepads: typeof navigator.getGamepads, requestMIDIAccess: typeof navigator.requestMIDIAccess,
    webkitGetAsEntry: typeof DataTransferItem.prototype.webkitGetAsEntry, getAsFileSystemHandle: typeof DataTransferItem.prototype.getAsFileSystemHandle, setPointerCapture: typeof Element.prototype.setPointerCapture,
    visualViewport: !!window.visualViewport, ResizeObserver: typeof ResizeObserver, matchMediaHover: matchMedia('(hover: hover)').matches, devicePixelRatio, screen: [screen.width, screen.height],
    isSecureContext, focus: document.hasFocus(),
  };
  // cursor: set and read back (the rendering of the cursor itself cannot be seen in headless Chrome)
  R.cursor = {};
  for (const c of ['crosshair', 'ew-resize', 'not-allowed', 'none', 'url("data:image/svg+xml;utf8,<svg xmlns=\'http://www.w3.org/2000/svg\' width=\'32\' height=\'32\'><rect width=\'32\' height=\'32\' fill=\'red\'/></svg>") 16 16, auto']) {
    pad.style.cursor = c;
    R.cursor[c.slice(0, 20)] = getComputedStyle(pad).cursor.slice(0, 40);
  }
  pad.style.cursor = 'default';
  return R;
};
window.tryPointerLock = async (unadjusted) => {
  const R = {};
  try {
    const ev = new Promise((res) => { document.addEventListener('pointerlockchange', () => res('change'), { once: true }); document.addEventListener('pointerlockerror', () => res('error'), { once: true }); setTimeout(() => res('none-in-1s'), 1000); });
    const r0 = pad.requestPointerLock(unadjusted ? { unadjustedMovement: true } : undefined);
    if (r0 && r0.then) await r0.catch((e) => { R.promiseError = String(e).slice(0, 120); });
    R.event = await ev; R.locked = document.pointerLockElement === pad; document.exitPointerLock();
  } catch (e) { R.error = String(e).slice(0, 150); }
  return R;
};
window.tryFullscreen = async () => {
  const R = {};
  try { await pad.requestFullscreen(); R.ok = true; R.el = document.fullscreenElement === pad; R.size = [innerWidth, innerHeight]; await document.exitFullscreen(); } catch (e) { R.ok = false; R.error = String(e).slice(0, 150); }
  return R;
};
window.tryClipboardKeyboard = async () => {
  const R = {};
  try { await navigator.clipboard.writeText('FLclip'); R.clipboardWrite = true; R.clipboardRead = await navigator.clipboard.readText(); } catch (e) { R.clipboard = { error: String(e).slice(0, 150) }; }
  try { if (navigator.keyboard && navigator.keyboard.lock) { await navigator.keyboard.lock(['Escape']); R.keyboardLock = 'ok (outside fullscreen)'; navigator.keyboard.unlock(); } } catch (e) { R.keyboardLock = String(e).slice(0, 120); }
  return R;
};
window.__seenNow = () => Object.assign({}, seen);
window.imeLog = () => imeLog;
window.focusIme = () => { ime.focus(); return document.activeElement === ime; };
window.focusPad = () => { pad.focus(); return document.activeElement === pad; };
window.__done = true;
window.__result = { ready: true };
