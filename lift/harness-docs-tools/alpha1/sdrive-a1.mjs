// ship3 (decision 0218) driver: one script, two browsers. The real engine on the page of a served bundle, driven by the TEST's input (CDP Input.* in headless Chrome 156, WebDriver Actions in headless Firefox 158):
// the page sees trusted pointer / key events, but there is no person and no real display. Derived from lift/out-git/crash1-results/tools/drive.mjs and drive-ff.mjs (decision 0209) with: one op code for both
// browsers, coordinates in GUEST pixels of whatever screen the page has (the desktop element's rect / __app.screen.guest, not a fixed 1920x1080), a wizard close that WAITS until the wizard window is gone
// (no extra Enter: an extra key makes FL paint for 3-4 s), per-scenario checks (a window class is visible / the engine did not stop), menu cycles that check the popup surface every time.
// usage: node sdrive.mjs --browser chrome|firefox --url http://127.0.0.1:PORT/[?query] --out DIR --scenarios FILE.json [--only a,b] [--viewport 2176x1224] [--app APPDIR] [--gd-port N] [--repeat K]
//        node sdrive.mjs ... --probe    (boot, close the wizard, print the window list and the guest size, write a screenshot)
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';


import zlib from 'node:zlib';
function encodePng(rgba, w, h) {
  const crcT = (() => { const t = new Uint32Array(256); for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } return t; })();
  const crc = (b) => { let c = 0xffffffff; for (const x of b) c = crcT[(c ^ x) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
  const chunk = (t, d) => { const l = Buffer.alloc(4); l.writeUInt32BE(d.length); const td = Buffer.concat([Buffer.from(t), d]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([l, td, c]); };
  const ih = Buffer.alloc(13); ih.writeUInt32BE(w, 0); ih.writeUInt32BE(h, 4); ih[8] = 8; ih[9] = 6;
  const raw = Buffer.alloc(h * (w * 4 + 1)); for (let y = 0; y < h; y++) { raw[y * (w * 4 + 1)] = 0; Buffer.from(rgba.buffer, rgba.byteOffset + y * w * 4, w * 4).copy(raw, y * (w * 4 + 1) + 1); }
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ih), chunk('IDAT', zlib.deflateSync(raw, { level: 3 })), chunk('IEND', Buffer.alloc(0))]);
}
const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const has = (n) => argv.includes('--' + n);
const BROWSER = opt('browser', 'chrome');
const URL0 = opt('url');
const OUT = path.resolve(opt('out', '/tmp/opencode/gpu1/drive'));
const [VW, VH] = opt('viewport', '2176x1224').split('x').map(Number);
const APP = path.resolve(opt('app', path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../lift/out-gpu1/src/app')));
const ONLY = opt('only', '') ? opt('only', '').split(',') : null;
const REPEAT = Number(opt('repeat', '1'));
fs.mkdirSync(OUT, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);

// ------------------------------------------------------------------ backends
const CHROME_KEYS = { Enter: [13, '\r'], Space: [32, ' '], Escape: [27], Tab: [9], Backspace: [8], Delete: [46], Home: [36], End: [35], PageUp: [33], PageDown: [34], ArrowLeft: [37], ArrowUp: [38], ArrowRight: [39], ArrowDown: [40], Minus: [189, '-'], Comma: [188, ','], Period: [190, '.'], ...Object.fromEntries([...Array(12)].map((_, i) => ['F' + (i + 1), [112 + i]])) };
const FF_KEYS = { Enter: '\uE007', Escape: '\uE00C', Tab: '\uE004', Space: ' ', Backspace: '\uE003', Delete: '\uE017', ArrowLeft: '\uE012', ArrowRight: '\uE014', ArrowUp: '\uE013', ArrowDown: '\uE015', Home: '\uE011', End: '\uE010', PageUp: '\uE00E', PageDown: '\uE00F', Minus: '-', Comma: ',', Period: '.', ...Object.fromEntries([...Array(12)].map((_, i) => ['F' + (i + 1), String.fromCharCode(0xE031 + i)])) };

async function chromeBackend() {
  const { openBrowser } = await import(pathToFileURL(path.join(APP, 'test/e2e/browser.mjs')).href);
  const browser = await openBrowser(OUT, 'sdrive');
  const page = await browser.newPage({ width: VW, height: VH, dpr: 1 });
  const chooser = [];
  const B = {
    name: 'chrome', version: null, pid: browser.pid,
    goto: (u) => page.goto(u),
    ev: (e, ms) => page.ev(e, ms),
    evAsync: (e) => page.ev(`(async () => (${e}))()`, 300000),
    async mouse(type, x, y, button, buttons, clickCount) { await page.S('Input.dispatchMouseEvent', { type, x, y, button, buttons, clickCount, pointerType: 'mouse' }); },
    async wheel(x, y, dx, dy) { await page.S('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y, deltaX: dx, deltaY: dy, button: 'none', buttons: 0, pointerType: 'mouse' }); },
    async key(code, mods, text) {
      let k = CHROME_KEYS[code];
      if (!k && /^Key[A-Z]$/.test(code)) k = [code.charCodeAt(3), code[3].toLowerCase()];
      if (!k && /^Digit[0-9]$/.test(code)) k = [code.charCodeAt(5), code[5]];
      if (!k) throw new Error('unknown key ' + code);
      const t = mods & 3 ? undefined : (text ?? k[1]);
      const kn = k[1] ?? code;
      await page.S('Input.dispatchKeyEvent', { type: t ? 'keyDown' : 'rawKeyDown', code, key: kn, windowsVirtualKeyCode: k[0], nativeVirtualKeyCode: k[0], modifiers: mods, ...(t ? { text: t } : {}) });
      await sleep(40);
      await page.S('Input.dispatchKeyEvent', { type: 'keyUp', code, key: kn, windowsVirtualKeyCode: k[0], nativeVirtualKeyCode: k[0], modifiers: mods });
    },
    async shot(file, clip) { const s = await page.S('Page.captureScreenshot', { format: 'png', fromSurface: true, clip: clip ? { ...clip, scale: 1 } : undefined }); fs.writeFileSync(file, Buffer.from(s.data, 'base64')); },
    async close() { await browser.close?.(); },
    raw: { page, browser, chooser },
  };
  return B;
}

async function firefoxBackend() {
  const port = Number(opt('gd-port', String(4444 + Math.floor(Math.random() * 400))));
  const gd = spawn('/snap/bin/geckodriver', ['--port', String(port), '--binary', '/snap/firefox/current/usr/lib/firefox/firefox'], { stdio: ['ignore', fs.openSync(path.join(OUT, 'geckodriver.log'), 'a'), fs.openSync(path.join(OUT, 'geckodriver.err'), 'a')] });
  const GD = `http://127.0.0.1:${port}`;
  for (let i = 0; i < 60; i++) { try { await fetch(GD + '/status'); break; } catch { await sleep(200); } }
  const wd = async (method, p, body) => {
    const r = await fetch(GD + p, { method, headers: { 'content-type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
    const j = await r.json().catch(() => ({}));
    if (!r.ok) throw new Error(`${method} ${p}: ${r.status} ${JSON.stringify(j).slice(0, 300)}`);
    return j.value;
  };
  const created = await wd('POST', '/session', { capabilities: { alwaysMatch: { browserName: 'firefox', 'moz:firefoxOptions': { args: ['-headless'], prefs: { ...(process.env.FF_PREFS ? JSON.parse(process.env.FF_PREFS) : {}), 'network.proxy.type': 0, 'datareporting.healthreport.uploadEnabled': false, 'app.update.auto': false } } } } });
  const sid = created.sessionId; const S = (p) => `/session/${sid}${p}`;
  await wd('POST', S('/timeouts'), { script: 600000, pageLoad: 600000 });
  await wd('POST', S('/window/rect'), { x: 0, y: 0, width: VW, height: VH });
  const act = (id, type, acts, params) => wd('POST', S('/actions'), { actions: [{ type, id, ...(params ? { parameters: params } : {}), actions: acts }] });
  const BTN = { none: 0, left: 0, middle: 1, right: 2 };
  const B = {
    name: 'firefox', version: created.capabilities.browserVersion, pid: gd.pid,
    goto: (u) => wd('POST', S('/url'), { url: u }),
    ev: (e) => wd('POST', S('/execute/sync'), { script: `return (${e})`, args: [] }),
    async evAsync(e) { const r = await wd('POST', S('/execute/async'), { script: `const done = arguments[arguments.length - 1]; (async () => (${e}))().then((v) => done(v === undefined ? null : v)).catch((x) => done('ERR:' + x));`, args: [] }); return r; },
    async mouse(type, x, y, button, buttons) {
      x = Math.round(x); y = Math.round(y);
      const mv = { type: 'pointerMove', duration: 0, x, y, origin: 'viewport' };
      if (type === 'mouseMoved') return act('mouse', 'pointer', [mv], { pointerType: 'mouse' });
      if (type === 'mousePressed') return act('mouse', 'pointer', [mv, { type: 'pointerDown', button: BTN[button] }], { pointerType: 'mouse' });
      return act('mouse', 'pointer', [mv, { type: 'pointerUp', button: BTN[button] }], { pointerType: 'mouse' });
    },
    async wheel(x, y, dx, dy) { await act('wheel', 'wheel', [{ type: 'scroll', x: Math.round(x), y: Math.round(y), deltaX: dx, deltaY: dy, duration: 0, origin: 'viewport' }]); },
    async key(code, mods, text) {
      const k = FF_KEYS[code] ?? (/^Key([A-Z])$/.test(code) ? (text ?? code[3].toLowerCase()) : /^Digit([0-9])$/.test(code) ? code[5] : null);
      if (!k) throw new Error('unknown key ' + code);
      const ms = [mods & 2 ? '\uE009' : null, mods & 8 ? '\uE008' : null, mods & 1 ? '\uE00A' : null].filter(Boolean);
      await act('kbd', 'key', [...ms.map((m) => ({ type: 'keyDown', value: m })), { type: 'keyDown', value: k }, { type: 'pause', duration: 40 }, { type: 'keyUp', value: k }, ...[...ms].reverse().map((m) => ({ type: 'keyUp', value: m }))]);
    },
    async shot(file) { const png = await wd('GET', S('/screenshot')); fs.writeFileSync(file, Buffer.from(png, 'base64')); },
    async close() { await wd('DELETE', S('')).catch(() => {}); gd.kill(); },
  };
  return B;
}


// alpha1 (decision 0244): a PNG decoder (8-bit RGB / RGBA, no interlace; all five filters) for the `series` op below
import zlib2 from 'node:zlib';
function decodePng(buf) {
  let pos = 8, w = 0, h = 0, ct = 0; const idat = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos), type = buf.toString('latin1', pos + 4, pos + 8), d = buf.subarray(pos + 8, pos + 8 + len);
    if (type === 'IHDR') { w = d.readUInt32BE(0); h = d.readUInt32BE(4); ct = d[9]; if (d[8] !== 8 || d[12] !== 0) throw new Error('png: only 8-bit, no interlace'); }
    if (type === 'IDAT') idat.push(d);
    pos += 12 + len;
  }
  const bpp = ct === 6 ? 4 : ct === 2 ? 3 : 0; if (!bpp) throw new Error('png: colour type ' + ct);
  const raw = zlib2.inflateSync(Buffer.concat(idat)), stride = w * bpp, out = Buffer.alloc(h * stride);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)], src = y * (stride + 1) + 1, dst = y * stride;
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? out[dst + x - bpp] : 0, b = y > 0 ? out[dst - stride + x] : 0, c = x >= bpp && y > 0 ? out[dst - stride + x - bpp] : 0;
      let v = raw[src + x];
      if (f === 1) v += a; else if (f === 2) v += b; else if (f === 3) v += (a + b) >> 1; else if (f === 4) { const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c); v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      out[dst + x] = v & 255;
    }
  }
  return { w, h, bpp, data: out };
}

const B = BROWSER === 'firefox' ? await firefoxBackend() : await chromeBackend();
// CPU of the browser's threads by group: every descendant of the browser we started (and its threads), ticks from /proc/<pid>/task/<tid>/stat
function procTreeThreads() {
  const kids = new Map();
  for (const d of fs.readdirSync('/proc')) { if (!/^\d+$/.test(d)) continue; try { const st = fs.readFileSync(`/proc/${d}/stat`, 'utf8'); const pp = Number(st.slice(st.lastIndexOf(')') + 2).split(' ')[1]); (kids.get(pp) ?? kids.set(pp, []).get(pp)).push(Number(d)); } catch { /* gone */ } }
  const all = []; const walk = (p) => { for (const k of kids.get(p) ?? []) { all.push(k); walk(k); } }; walk(B.pid); all.push(B.pid);
  return all;
}
function cpuSnap() {
  const m = new Map();
  for (const pid of procTreeThreads()) {
    let tids = []; try { tids = fs.readdirSync(`/proc/${pid}/task`); } catch { continue; }
    for (const tid of tids) { try { const st = fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, 'utf8'); const comm = st.slice(st.indexOf('(') + 1, st.lastIndexOf(')')); const f = st.slice(st.lastIndexOf(')') + 2).split(' '); m.set(`${pid}/${tid}`, { comm, ticks: Number(f[11]) + Number(f[12]) }); } catch { /* gone */ } }
  }
  return m;
}
function cpuDiff(a, b) {
  const g = { worker: 0, main: 0, compositor: 0, gpu: 0, other: 0 }; const by = new Map();
  for (const [k, v] of b) {
    const t = (v.ticks - (a.get(k)?.ticks ?? 0)) / 100; if (t <= 0) continue;
    by.set(v.comm, (by.get(v.comm) ?? 0) + t);
    if (/DedicatedWorker|DOM Worker/i.test(v.comm)) g.worker += t; else if (/CrRendererMain|GeckoMain/i.test(v.comm)) g.main += t; else if (/Compositor|VizCompositor/i.test(v.comm)) g.compositor += t; else if (/Gpu|GPU|WebGL|Render/i.test(v.comm)) g.gpu += t; else g.other += t;
  }
  for (const k of Object.keys(g)) g[k] = +g[k].toFixed(3);
  g.top = Object.fromEntries([...by.entries()].sort((p, q) => q[1] - p[1]).slice(0, 8).map(([k, v]) => [k, +v.toFixed(2)]));
  return g;
}

log('backend', B.name, B.version ?? '');

// ------------------------------------------------------------------ page helpers
const jev = async (expr) => { const r = await B.ev(`JSON.stringify(${expr})`); return r === undefined || r === null ? null : JSON.parse(r); };
async function geometry() {
  return jev(`(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app && window.__app.screen && window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: (g && g.width) || 1920, gh: (g && g.height) || 1080 }; })()`);
}
let geo = null;
const toClient = async (sx, sy) => { if (!geo) geo = await geometry(); return { x: geo.x + (sx + 0.5) * (geo.w / geo.gw), y: geo.y + (sy + 0.5) * (geo.h / geo.gh) }; };
async function state() {
  return jev(`(() => { const a = window.__app; if (!a) return { state: 'no-app' }; return { state: a.state, exit: a.exit, error: a.error, inputEvents: a.inputEvents, idles: a.idles.length, idleAt: a.timeline && a.timeline.idle, lastIdleAgoMs: a.idles.length ? Math.round(performance.timeOrigin + performance.now() - a.idles[a.idles.length - 1].at) : null }; })()`);
}
const ended = async () => { const s = await state(); return s.state === 'exited' || s.state === 'error'; };
async function windows() {   // desk2 form (single canvas): the compositor's text about the windows it drew (needs ?debug=1): lid, position, size, class, title; every window in the text is drawn = visible
  const t = (await jev(`(window.__app && window.__app.deskWindows) || ''`)) ?? '';
  return t.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m, i) => {
    const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
    return { lid: Number(m[1]), cls: names ? names[1] : '', title: names ? names[2] : '', x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), vis: true, z: i };
  });
}
async function surfaces() { return (await windows()).map((w) => ({ lid: w.lid, key: 'w' + w.lid, x: w.x, y: w.y, w: w.w, h: w.h, vis: true })); }
async function exitReport() {
  return jev(`(() => { const a = window.__app; const el = document.getElementById('exit'); const lines = a.engineLog(); const snap = a.perf.snapshot('exit'); return { exitText: el ? el.textContent : null, exit: a.exit, error: a.error, beaconExit: snap.engine && snap.engine.exit, tail: lines.slice(-14) }; })()`);
}
const BTNS = { left: 1, right: 2, middle: 4 };
let held = 0, lastPos = { x: 0, y: 0 };
const sent = [];
async function mouse(type, sx, sy, button, buttons, count = 0) {
  const c = await toClient(sx, sy); lastPos = { x: sx, y: sy };
  await B.mouse(type, c.x, c.y, button, buttons, count);
}
async function waitWindow(cls, vis, ms) {
  const t0 = Date.now();
  for (;;) {
    const ws = await windows();
    const hit = ws.some((w) => w.cls === cls && w.vis);
    if (hit === vis) return true;
    if (await ended()) return false;
    if (Date.now() - t0 > ms) return false;
    await sleep(200);
  }
}
const ctx = { project: opt('project', '') };
async function waitJs(expr, ms) { const t0 = Date.now(); for (;;) { const v = await B.ev(`(() => { try { return !!(${expr}); } catch (e) { return false; } })()`).catch(() => false); if (v === true) return true; if (Date.now() - t0 > ms) throw new Error('timeout waiting for ' + expr.slice(0, 80)); await sleep(200); } }

async function N(v) { if (typeof v !== 'string') return v; if (!geo) geo = await geometry(); return Math.round(new Function('W', 'H', `return (${v});`)(geo.gw, geo.gh)); }
async function runOp(op0) {
  const op = { ...op0 };
  for (const k of ['x', 'y', 'x2', 'y2', 'dx', 'dy']) if (typeof op[k] === 'string' && !(op.op === 'wheel' && (k === 'dx' || k === 'dy'))) op[k] = await N(op[k]);
  sent.push(JSON.stringify(op0));
  switch (op.op) {
    case 'goto': geo = null; await B.goto(op.url ?? URL0); break;
    case 'wait': await sleep(op.ms ?? 500); break;
    case 'waitIdle': {
      const t0 = Date.now();
      for (;;) {
        const s = await state().catch(() => ({ state: 'loading', idles: 0 }));
        if (s.state === 'engine-idle' && s.idles >= (op.minIdles ?? 5)) break;
        if (s.state === 'exited' || s.state === 'error') break;
        if (Date.now() - t0 > (op.ms ?? 240000)) throw new Error('waitIdle timeout, state ' + s.state);
        await sleep(250);
      }
      geo = await geometry();
      break;
    }
    case 'waitWindow': { const ok = await waitWindow(op.cls, op.visible ?? true, op.ms ?? 20000); if (!ok && !op.soft) throw new Error(`window ${op.cls} did not become ${op.visible ?? true ? 'visible' : 'hidden'}`); break; }
    case 'move': await mouse('mouseMoved', op.x, op.y, held ? (held & 1 ? 'left' : held & 2 ? 'right' : 'middle') : 'none', held); break;
    case 'down': held |= BTNS[op.button ?? 'left']; await mouse('mousePressed', op.x ?? lastPos.x, op.y ?? lastPos.y, op.button ?? 'left', held, op.count ?? 1); break;
    case 'up': { const b = op.button ?? 'left'; held &= ~BTNS[b]; await mouse('mouseReleased', op.x ?? lastPos.x, op.y ?? lastPos.y, b, held, op.count ?? 1); break; }
    case 'click': {
      const b = op.button ?? 'left';
      await mouse('mouseMoved', op.x, op.y, 'none', 0); await sleep(op.hover ?? 120);
      for (let i = 1; i <= (op.count ?? 1); i++) {
        held |= BTNS[b]; await mouse('mousePressed', op.x, op.y, b, held, i); await sleep(op.hold ?? 70);
        held &= ~BTNS[b]; await mouse('mouseReleased', op.x, op.y, b, held, i); await sleep(60);
      }
      break;
    }
    case 'drag': {
      const b = op.button ?? 'left';
      await mouse('mouseMoved', op.x, op.y, 'none', 0); await sleep(120);
      held |= BTNS[b]; await mouse('mousePressed', op.x, op.y, b, held, 1); await sleep(80);
      const n = op.steps ?? 10;
      for (let i = 1; i <= n; i++) { await mouse('mouseMoved', Math.round(op.x + ((op.x2 - op.x) * i) / n), Math.round(op.y + ((op.y2 - op.y) * i) / n), b, held); await sleep(op.stepMs ?? 40); }
      await sleep(80);
      held &= ~BTNS[b]; await mouse('mouseReleased', op.x2, op.y2, b, held, 1);
      break;
    }
    case 'dragRate': {
      // ship5: a continuous drag with `button` between (x,y) and (x2,y2) for `secs` seconds (the pointer moves as fast as the driver can send them, `steps` positions per leg); the page's desktop presenter counters (__app.deskStats)
      // before and after give the updates/s that the page drew (presents = putImageData calls of the compositor), rects/s and pixels/s; moves/s = what the driver managed to send
      const b = op.button ?? 'middle'; const secs = op.secs ?? 8; const n = op.steps ?? 20;
      const st0 = await jev('window.__app.deskStats'); const e0 = await jev('window.__app.engineStats ? window.__app.engineStats() : null').catch(() => null);
      await mouse('mouseMoved', op.x, op.y, 'none', 0); await sleep(120);
      held |= BTNS[b]; await mouse('mousePressed', op.x, op.y, b, held, 1); await sleep(80);
      const ld0 = Number(fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]);
      const t0 = Date.now(); let moves = 0, leg = 0;
      while (Date.now() - t0 < secs * 1000) {
        const [ax, ay, bx, by] = leg % 2 === 0 ? [op.x, op.y, op.x2, op.y2] : [op.x2, op.y2, op.x, op.y];
        for (let i = 1; i <= n && Date.now() - t0 < secs * 1000; i++) { await mouse('mouseMoved', Math.round(ax + ((bx - ax) * i) / n), Math.round(ay + ((by - ay) * i) / n), b, held); moves++; if (op.stepMs) await sleep(op.stepMs); }
        leg++;
      }
      const wall = (Date.now() - t0) / 1000;
      const st1 = await jev('window.__app.deskStats');
      const ld1 = Number(fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]);
      await sleep(80); if (!op.idle) { held &= ~BTNS[b]; await mouse('mouseReleased', op.x2, op.y2, b, held, 1); }
      const d = (k) => (st0 && st1 ? st1[k] - st0[k] : null);
      (ctx.dragRates ??= []).push({ name: op.name ?? 'dragRate', wallS: +wall.toFixed(2), moves, movesPerS: +(moves / wall).toFixed(1), presents: d('presents'), updatesPerS: st0 && st1 ? +(d('presents') / wall).toFixed(2) : null, rectsPerS: st0 && st1 ? +(d('rects') / wall).toFixed(2) : null, pixelsPerS: st0 && st1 ? Math.round(d('pixels') / wall) : null, composeMsPerS: st0 && st1 ? +(d('composeMs') / wall).toFixed(1) : null, putMsPerS: st0 && st1 ? +(d('putMs') / wall).toFixed(1) : null, loadStart: ld0, loadEnd: ld1 });
      break;
    }
    case 'mem': {
      // ship5: the memory of the browser's process tree (every descendant of the browser / geckodriver we started): RSS (statm) and PSS (smaps_rollup) per process, by type, in MB
      const kids = new Map();
      for (const d of fs.readdirSync('/proc')) { if (!/^\d+$/.test(d)) continue; try { const st = fs.readFileSync(`/proc/${d}/stat`, 'utf8'); const pp = Number(st.slice(st.lastIndexOf(')') + 2).split(' ')[1]); (kids.get(pp) ?? kids.set(pp, []).get(pp)).push(Number(d)); } catch { /* gone */ } }
      const all = []; const walk = (p) => { for (const k of kids.get(p) ?? []) { all.push(k); walk(k); } }; walk(B.pid); all.push(B.pid);
      const rows = [];
      for (const pid of all) {
        try {
          const cmd = fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').split('\0').join(' ');
          const tm = (/--type=([\w-]+)/.exec(cmd)?.[1] ?? (/-contentproc\b.* ([\w-]+)\s*$/.exec(cmd) ? 'content:' + /-contentproc\b.* ([\w-]+)\s*$/.exec(cmd)[1] : (/geckodriver/.test(cmd) ? 'geckodriver' : 'browser'))) + (/--utility-sub-type=([\w.]+)/.exec(cmd) ? ':' + /--utility-sub-type=[\w.]*\.(\w+)/.exec(cmd)?.[1] : '');
          const rss = Number(fs.readFileSync(`/proc/${pid}/statm`, 'utf8').split(' ')[1]) * 4096 / 1048576;
          let pss = null; try { pss = Number(/Pss:\s+(\d+) kB/.exec(fs.readFileSync(`/proc/${pid}/smaps_rollup`, 'utf8'))[1]) / 1024; } catch { /* none */ }
          rows.push({ pid, type: tm, rssMB: +rss.toFixed(1), pssMB: pss === null ? null : +pss.toFixed(1) });
        } catch { /* gone */ }
      }
      const tot = (k) => +rows.reduce((a, r) => a + (r[k] ?? 0), 0).toFixed(1);
      (ctx.mems ??= []).push({ name: op.name ?? 'mem', rssMB: tot('rssMB'), pssMB: tot('pssMB'), maxRssMB: Math.max(...rows.map((r) => r.rssMB)), procs: rows.filter((r) => r.rssMB > 30) });
      break;
    }
    case 'wheel': { const c = await toClient(op.x, op.y); await B.wheel(c.x, c.y, op.dx ?? 0, op.dy ?? 120); break; }
    case 'key': await B.key(op.code, (op.ctrl ? 2 : 0) | (op.shift ? 8 : 0) | (op.alt ? 1 : 0), op.text); break;
    case 'type': for (const ch of op.text) { await B.key(/[a-z]/i.test(ch) ? 'Key' + ch.toUpperCase() : /[0-9]/.test(ch) ? 'Digit' + ch : ch === ' ' ? 'Space' : ch === '-' ? 'Minus' : ch === ',' ? 'Comma' : 'Period', /[A-Z]/.test(ch) ? 8 : 0, ch); await sleep(op.gap ?? 80); } break;
    case 'shot': { geo = geo ?? (await geometry()); await B.shot(path.join(OUT, `${op.name ?? 'shot'}.png`), BROWSER === 'chrome' && !op.full ? { x: geo.x, y: geo.y, width: geo.w, height: geo.h } : undefined); break; }
    case 'closeWizard': {
      // the usage-data box ("No thanks") if the page has it, then the welcome wizard's X; each click is followed by waiting until that window is gone (no Enter, no blind second try)
      const ws0 = await windows();
      const wiz = ws0.find((w) => w.cls === 'TWelcomeWizard' && w.vis);
      if (!wiz) break;
      await runOp({ op: 'click', x: wiz.x + wiz.w - 18, y: wiz.y + 18 });
      await waitWindow('TWelcomeWizard', false, 15000);
      await sleep(op.settle ?? 1500);
      break;
    }
    case 'menuCycles': {
      // File menu (or op.x/op.y) opened and closed with Escape N times; every time the popup window (TQuickPopupMenuWindow) must be visible in the engine's window table AND have a visible surface entry in the page
      const res = { n: op.n, popupSeen: 0, popupMissingSurface: [], notVisible: [] };
      for (let i = 1; i <= op.n; i++) {
        await runOp({ op: 'click', x: op.x ?? 18, y: op.y ?? 20, hover: 60, hold: 50 });
        const ok = await waitWindow('TQuickPopupMenuWindow', true, 6000);
        if (await ended()) { res.stoppedAt = i; break; }
        if (!ok) { res.notVisible.push(i); } else {
          const w = (await windows()).filter((x) => x.cls === 'TQuickPopupMenuWindow' && x.vis).pop();
          let sf = null;
          for (let k = 0; k < 20 && !sf; k++) { sf = (await surfaces()).find((s) => s.lid === w.lid && s.vis); if (!sf) await sleep(100); }
          if (sf) res.popupSeen++; else res.popupMissingSurface.push(i);
        }
        await B.key('Escape', 0);
        await waitWindow('TQuickPopupMenuWindow', false, 6000);
        await sleep(150);
      }
      res.completed = res.stoppedAt ? res.stoppedAt - 1 : op.n;
      ctx.menuCycles = res;
      break;
    }
    case 'openProject': {
      // both browsers: the project's files are fetched from this server (/assets/proj/) and put into the page's file manager window with its own "Upload files" input (a DataTransfer stands for the person's choice in a file chooser);
      // then File > Open of the engine, the rows of the window (DOM clicks) and the engine's own title change as the proof
      const flp = op.flp ?? 'Sampler.flp'; const names = op.filesFile ? JSON.parse(fs.readFileSync(op.filesFile, 'utf8')) : (op.files ?? [flp, 'SHP_Sawing Synth.wav']);
      await B.ev("(() => { document.querySelector('#files-btn').click(); return true; })()");
      await waitJs("!!window.__files && !!window.__files.explorer && !!document.querySelector('#files-overlay #fx-list')", 20000);
      await sleep(1200);
      const have = await B.ev(`[...document.querySelectorAll('#files-overlay #fx-list .fx-row[data-name]')].some((r) => r.dataset.name === ${JSON.stringify(flp)})`);
      const up = have ? names.length : await B.evAsync(`(async () => { const dt = new DataTransfer(); for (const n of ${JSON.stringify(names)}) { const b = await (await fetch('/assets/proj/' + encodeURIComponent(n))).blob(); dt.items.add(new File([b], n)); } const i = document.querySelector('#files-overlay #fx-input-files'); const n = dt.files.length; i.files = dt.files; i.dispatchEvent(new Event('change', { bubbles: true })); return n; })()`);
      if (up !== names.length) throw new Error('openProject: upload ' + JSON.stringify(up));
      await waitJs(`window.__files.explorer && !window.__files.explorer.busy && [...document.querySelectorAll('#files-overlay #fx-list .fx-row[data-name]')].some((r) => r.dataset.name === ${JSON.stringify(flp)})`, op.uploadMs ?? 120000);
      await B.ev("(() => { document.querySelector('#files-overlay #fx-close').click(); return true; })()");
      await sleep(500);
      for (let k = 0; k < 3; k++) {
        await runOp({ op: 'click', x: 18, y: 20, hover: 60 });
        if (await waitWindow('TQuickPopupMenuWindow', true, 4000)) break;
      }
      await runOp({ op: 'click', x: 48, y: 85 });
      await waitJs('window.__app.fileDlg.open === true', 120000);
      await sleep(800);
      await waitJs(`[...document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]')].some((r) => r.dataset.name === ${JSON.stringify(flp)})`, 30000);
      await B.ev(`(() => { const r = [...document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]')].find((x) => x.dataset.name === ${JSON.stringify(flp)}); r.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, detail: 1 })); return true; })()`);
      await B.ev("(() => { document.querySelector('#filedlg #fx-ok').click(); return true; })()");
      const want = `${flp} - FL Studio 2026`;
      const t1 = Date.now(); let title = null;
      while (Date.now() - t1 < (op.ms ?? 180000)) {
        title = (await windows()).filter((w) => w.cls === 'TFruityLoopsMainForm').map((w) => w.title)[0] ?? null;
        if (title === want || (title && title.startsWith(flp.replace(/\.flp$/i, '')))) break;
        const miss = (await windows()).find((w) => w.cls === 'TMissingPluginsForm' && w.vis);
        if (miss) { await runOp({ op: 'click', x: miss.x + 77, y: miss.y + 259 }); await sleep(2000); }
        if (await ended()) break;
        await sleep(300);
      }
      ctx.openedTitle = title;
      await sleep(op.settle ?? 4000);
      break;
    }
    case 'openFile': {
      if (BROWSER !== 'chrome') throw new Error('openFile is implemented for Chrome only');
      const { page, browser, chooser } = B.raw;
      await page.S('Page.setInterceptFileChooserDialog', { enabled: true });
      chooser.length = 0;
      browser.cdp.on((m) => { if (m.sessionId !== undefined && m.method === 'Page.fileChooserOpened') chooser.push(m.params); });
      await runOp({ op: 'click', x: 18, y: 20 }); await sleep(1500);
      await runOp({ op: 'click', x: op.itemX ?? 48, y: op.itemY ?? 85 });
      const t0 = Date.now();
      while (Date.now() - t0 < 120000 && (await B.ev('window.__app.fileDlg.open')) !== true) await sleep(250);
      if ((await B.ev('window.__app.fileDlg.open')) !== true) throw new Error('openFile: the page was not asked for a file');
      await sleep(500);
      const inputId = op.mode === 'folder' ? 'filedlg-dir' : 'filedlg-input';
      const box = await jev(`document.getElementById('${inputId}').getBoundingClientRect().toJSON()`);
      await page.S('Input.dispatchMouseEvent', { type: 'mousePressed', x: box.x + 10, y: box.y + box.height / 2, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
      await page.S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: box.x + 10, y: box.y + box.height / 2, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
      for (let i = 0; i < 50 && chooser.length === 0; i++) await sleep(100);
      if (chooser.length === 0) throw new Error('openFile: no file chooser opened');
      await page.S('DOM.setFileInputFiles', { files: [op.file ?? ctx.project], backendNodeId: chooser[0].backendNodeId });
      const want = (op.title ?? 'Sampler.flp') + ' - FL Studio 2026';
      const t1 = Date.now(); let title = null;
      while (Date.now() - t1 < (op.ms ?? 180000)) {
        title = (await windows()).filter((w) => w.cls === 'TFruityLoopsMainForm').map((w) => w.title)[0] ?? null;
        if (title === want) break;
        if (await ended()) break;
        await sleep(300);
      }
      ctx.openedTitle = title;
      await sleep(op.settle ?? 4000);
      break;
    }
    case 'clickIn': { const w = (await windows()).find((x) => x.cls === op.cls && x.vis); if (!w) throw new Error('clickIn: no visible ' + op.cls); await runOp({ ...op, op: 'click', x: w.x + op.dx, y: w.y + op.dy }); break; }
    case 'expectAbove': { const ws = (await windows()).filter((x) => x.vis); const ia = ws.findIndex((x) => x.cls === op.a), ib = ws.findIndex((x) => x.cls === op.b); (ctx.expects ??= []).push({ cls: `${op.a} above ${op.b}`, visible: true, ok: ia >= 0 && ib >= 0 && ia < ib, detail: { ia, ib } }); break; }
    case 'listWindows': ctx.listed = (await windows()).filter((x) => x.vis && x.w > 0).map((x) => `${x.lid} ${x.cls} '${x.title}' @${x.x},${x.y} ${x.w}x${x.h}`); break;
    case 'pixel': {
      // the colour of a guest-screen point as the page's canvas of window `lid` (default: the main form) holds it now (getImageData: the page's own picture of that window)
      const ws = await windows(); const w = ws.find((x) => x.cls === (op.cls ?? 'TFruityLoopsMainForm') && x.vis);
      if (!w) throw new Error('pixel: no window');
      const rgb = await jev(`(() => { const c = document.querySelector('canvas[data-lid="${w.lid}"]'); if (!c) return null; const d = c.getContext('2d').getImageData(${op.x - w.x}, ${op.y - w.y}, 1, 1).data; return [d[0], d[1], d[2]]; })()`);
      (ctx.pix ??= {})[op.name] = rgb; break;
    }
    case 'expectPixel': { const a = ctx.pix?.[op.a], b = ctx.pix?.[op.b]; const dist = a && b ? Math.max(...a.map((v, i) => Math.abs(v - b[i]))) : null; const differ = dist !== null && dist > (op.tol ?? 12); (ctx.expects ??= []).push({ cls: `pixel ${op.a} ${op.differ === false ? '==' : '!='} ${op.b}`, visible: true, ok: dist !== null && (op.differ === false ? !differ : differ), detail: { a, b, dist } }); break; }
    case 'expectExit': { const st = await state(); (ctx.expects ??= []).push({ cls: `engine ended with code ${op.code}`, visible: true, ok: st.state === 'exited' && st.exit && st.exit.code === op.code, detail: st.exit }); break; }
    case 'expectRunning': { const st = await state(); (ctx.expects ??= []).push({ cls: 'engine still running', visible: true, ok: st.state === 'engine-idle' || st.state === 'running', detail: st.state }); break; }
    case 'expectWindow': { const ws = await windows(); const hit = ws.some((w) => w.cls === op.cls && w.vis === (op.visible ?? true)); (ctx.expects ??= []).push({ cls: op.cls, visible: op.visible ?? true, ok: hit }); break; }

    case 'burst': {
      // gpu1: `n` pointer moves from (x,y) to (x2,y2), sent through the page's own sendRaw (the road a real pointer takes: tracker -> SAB ring -> the engine's input thread), `per` of them in one task, `gapMs` between tasks.
      // The button must already be held (op down). A guest that is slower than the pointer sees several moves between two paints.
      const n = op.n ?? 100, per = op.per ?? 10, gap = op.gapMs ?? 0;
      const r = await B.evAsync(`(async () => { const a = window.__app; let sent = 0; for (let i = 1; i <= ${n}; i += ${per}) { for (let j = i; j < i + ${per} && j <= ${n}; j++) { const x = Math.round(${op.x} + (${op.x2} - ${op.x}) * j / ${n}), y = Math.round(${op.y} + (${op.y2} - ${op.y}) * j / ${n}); a.sendRaw({ k: 'mouse', kind: 1, x, y, data: 0 }); sent++; } await new Promise((r) => setTimeout(r, ${gap})); } return sent; })()`);
      lastPos = { x: op.x2, y: op.y2 };
      (ctx.bursts ??= []).push({ n, per, gap, sent: r });
      break;
    }
    case 'cmpShot': {
      // gpu1: the browser's own picture of the desktop element (CDP clip / WebDriver screenshot) against the compositor's picture as the presenter last read it (peek). Needs a 1:1 scale (viewport width = guest width + 340).
      // Differences are counted by pixel (any of R, G, B), with the bounding box and a 6x8 grid, so that a stale canvas, a stale browser surface or an animated area can be told apart by where they are.
      const { decodePng, toRgba } = await import(pathToFileURL(path.join(APP, 'scripts/lib/png.mjs')).href);
      geo = await geometry();
      const file = path.join(OUT, `${op.name}.png`);
      await B.shot(file, BROWSER === 'chrome' ? { x: geo.x, y: geo.y, width: geo.w, height: geo.h } : undefined);
      const peek = await B.evAsync(op.gl ? 'window.__app.deskGlPixelsBase64()' : 'window.__app.deskDumpBase64(true)');
      const pk = Buffer.from(peek.base64, 'base64');
      const png = decodePng(fs.readFileSync(file)); const sh = toRgba(png);
      const ox = BROWSER === 'chrome' ? 0 : Math.round(geo.x), oy = BROWSER === 'chrome' ? 0 : Math.round(geo.y);
      const W = peek.width, H = peek.height;
      if (BROWSER === 'chrome' ? (png.width !== W || png.height !== H) : (png.width < ox + W || png.height < oy + H)) { (ctx.cmpShots ??= []).push({ name: op.name, error: `shot ${png.width}x${png.height} vs picture ${W}x${H} at ${ox},${oy}: not 1:1` }); break; }
      let n = 0, x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1; const grid = Array.from({ length: 6 }, () => Array(8).fill(0));
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const i = ((y + oy) * png.width + (x + ox)) * 4, j = (y * W + x) * 4;
        if (sh[i] !== pk[j] || sh[i + 1] !== pk[j + 1] || sh[i + 2] !== pk[j + 2]) { n++; if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; grid[Math.min(5, (y * 6 / H) | 0)][Math.min(7, (x * 8 / W) | 0)]++; }
      }
      (ctx.cmpShots ??= []).push({ name: op.name, differing: n, box: n ? [x0, y0, x1 + 1, y1 + 1] : null, grid });
      break;
    }
    case 'logMark': { ctx.marks ??= {}; ctx.marks[op.name] = await B.ev('window.__app.engineLog().length'); break; }
    case 'saveLog': {
      const from = op.all ? 0 : (ctx.marks?.[op.mark] ?? 0);
      const lines = await jev(`window.__app.engineLog().slice(${from})`);
      fs.writeFileSync(path.join(OUT, `${op.name}.log`), lines.join('\n') + '\n');
      (ctx.logs ??= []).push({ name: op.name, lines: lines.length });
      break;
    }
    case 'dragMeasure': {
      // gpu1: a continuous drag with `button` held, the pointer moving along (x,y) <-> (x2,y2) in `n` steps per leg for `secs` seconds (each step is one CDP/WebDriver pointer event; `stepMs` pause after each; 0 = as fast as the driver can send).
      // Reported: moves/s the driver managed, the presenter's counters of the Worker (presents, rects, pixels, composeMs, putMs: __app.deskStats, reported at most every 100 ms) before and after, and the CPU of the browser's threads by group
      // (the engine Worker thread, the page's main thread, compositor/GPU/other) from /proc/<pid>/task/<tid>/stat over the same interval. The engine is not asked anything else.
      const b = op.button ?? 'left'; const secs = op.secs ?? 6; const n = op.n ?? 20;
      if (!op.idle) { await mouse('mouseMoved', op.x, op.y, 'none', 0); await sleep(150); held |= BTNS[b]; await mouse('mousePressed', op.x, op.y, b, held, 1); await sleep(300); }
      await sleep(op.settle ?? 1200); const st0 = await jev('window.__app.deskStats'); const c0 = cpuSnap(); const ld0 = Number(fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]);
      const t0 = Date.now(); let moves = 0, leg = 0;
      if (op.idle) await sleep(secs * 1000);
      else while (Date.now() - t0 < secs * 1000) {
        const [ax, ay, bx, by] = leg % 2 === 0 ? [op.x, op.y, op.x2, op.y2] : [op.x2, op.y2, op.x, op.y];
        for (let i = 1; i <= n && Date.now() - t0 < secs * 1000; i++) { await mouse('mouseMoved', Math.round(ax + ((bx - ax) * i) / n), Math.round(ay + ((by - ay) * i) / n), b, held); moves++; if (op.stepMs) await sleep(op.stepMs); }
        leg++;
      }
      const wall = (Date.now() - t0) / 1000; const c1 = cpuSnap(); await sleep(250); const st1 = await jev('window.__app.deskStats'); const ld1 = Number(fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]);
      await sleep(80); if (!op.idle) { held &= ~BTNS[b]; await mouse('mouseReleased', op.x2, op.y2, b, held, 1); }
      const d = (k) => (st0 && st1 ? st1[k] - st0[k] : null); const cpu = cpuDiff(c0, c1);
      const pres = d('presents');
      (ctx.measures ??= []).push({ name: op.name ?? 'dragMeasure', wallS: +wall.toFixed(2), moves, movesPerS: +(moves / wall).toFixed(1), presents: pres, presentsPerS: pres === null ? null : +(pres / wall).toFixed(2), rectsPerS: st0 ? +(d('rects') / wall).toFixed(2) : null, mpxPerS: st0 ? +(d('pixels') / wall / 1e6).toFixed(3) : null, composeMsPerPresent: pres ? +(d('composeMs') / pres).toFixed(3) : null, putMsPerPresent: pres ? +(d('putMs') / pres).toFixed(3) : null, wholeFrames: d('whole'), cpuS: cpu, workerMsPerPresent: pres ? +(1000 * cpu.worker / pres).toFixed(2) : null, workerMsPerMove: moves ? +(1000 * cpu.worker / moves).toFixed(2) : null, workerBusyFrac: +(cpu.worker / wall).toFixed(3), ...(st1 && st1.draws !== undefined ? { gl: { drawMsPerPresent: pres ? +(d('drawMs') / pres).toFixed(3) : null, uploadsPerPresent: pres ? +(d('uploads') / pres).toFixed(2) : null, drawsPerPresent: pres ? +(d('draws') / pres).toFixed(2) : null, uploadMBPerS: +(d('uploadBytes') / wall / 1e6).toFixed(2), texAllocs: d('texAllocs') } } : {}), load0: ld0, load1: ld1 });
      break;
    }
    case 'glcmp': {
      // gpu1: the WebGL canvas read back (the scene drawn again, readPixels) against the CPU compositor's whole picture drawn from the window table at the next moment (dump). Counts the pixels that differ in R, G or B, the largest channel difference, the bounding box and a 6x8 grid.
      const gl = await B.evAsync('window.__app.deskGlPixelsBase64()');
      const cpu = await B.evAsync('window.__app.deskDumpBase64(false)');
      const a = Buffer.from(gl.base64, 'base64'), b = Buffer.from(cpu.base64, 'base64');
      const W = cpu.width, H = cpu.height;
      if (gl.width !== W || gl.height !== H) { (ctx.glcmps ??= []).push({ name: op.name, error: `size ${gl.width}x${gl.height} vs ${W}x${H}` }); break; }
      let n = 0, mx = 0, x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1; const grid = Array.from({ length: 6 }, () => Array(8).fill(0)); const hist = {};
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const j = (y * W + x) * 4; const d = Math.max(Math.abs(a[j] - b[j]), Math.abs(a[j + 1] - b[j + 1]), Math.abs(a[j + 2] - b[j + 2]));
        if (d > 0) { n++; if (d > mx) mx = d; hist[d] = (hist[d] ?? 0) + 1; if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; grid[Math.min(5, (y * 6 / H) | 0)][Math.min(7, (x * 8 / W) | 0)]++; }
      }
      (ctx.glcmps ??= []).push({ name: op.name, differing: n, maxChannelDiff: mx, box: n ? [x0, y0, x1 + 1, y1 + 1] : null, grid, hist: Object.fromEntries(Object.entries(hist).sort((p, q) => q[1] - p[1]).slice(0, 6)) });
      if (op.save) { fs.writeFileSync(path.join(OUT, `${op.name}-gl.png`), encodePng(a, W, H)); fs.writeFileSync(path.join(OUT, `${op.name}-cpu.png`), encodePng(b, W, H)); }
      break;
    }
    case 'seqMeasure': {
      // gpu1: run the sub-operations `ops` and report what the Worker and the presenter did meanwhile, per unit (`units`: e.g. the number of clicks): presents, rects, Mpx, compose/put ms per present, Worker CPU ms per unit
      const st0 = await jev('window.__app.deskStats'); const c0 = cpuSnap(); const t0 = Date.now();
      for (const sub of op.ops) await runOp(sub);
      const wall = (Date.now() - t0) / 1000; const c1 = cpuSnap(); await sleep(250); const st1 = await jev('window.__app.deskStats'); const cpu = cpuDiff(c0, c1);
      const d = (k) => (st0 && st1 ? st1[k] - st0[k] : null); const pres = d('presents'); const units = op.units ?? 1;
      (ctx.measures ??= []).push({ name: op.name ?? 'seq', wallS: +wall.toFixed(2), units, presents: pres, presentsPerUnit: pres === null ? null : +(pres / units).toFixed(2), mpxPerUnit: st0 ? +(d('pixels') / units / 1e6).toFixed(3) : null, composeMsPerUnit: st0 ? +(d('composeMs') / units).toFixed(2) : null, putMsPerUnit: st0 ? +(d('putMs') / units).toFixed(2) : null, cpuS: cpu, workerMsPerUnit: +(1000 * cpu.worker / units).toFixed(1), ...(st1 && st1.draws !== undefined ? { gl: { drawMsPerUnit: +(d('drawMs') / units).toFixed(2), uploadsPerUnit: +(d('uploads') / units).toFixed(2) } } : {}) });
      break;
    }
    case 'crashGpu': { if (BROWSER !== 'chrome') throw new Error('crashGpu is Chrome only'); await B.raw.browser.cdp.send('Browser.crashGpuProcess'); break; }
    case 'hashes': {
      // gpu1: three SHA-256 of the desktop picture at the same moment, in an order that does not change what they look at: (canvas) what the Worker's OffscreenCanvas holds (getImageData), (peek) what the compositor believes it
      // gave the canvas (no compose), (dump) the whole picture drawn again from the window table (this resyncs: the next present shows everything, so it is always the LAST of the three and after any screenshot)
      const canvas = await B.evAsync('window.__app.readbackDesk()');
      const peek = await B.evAsync('(async () => { const d = await window.__app.deskDumpBase64(true); const b = Uint8Array.from(atob(d.base64), (c) => c.charCodeAt(0)); const h = await crypto.subtle.digest("SHA-256", b); return { sha256: [...new Uint8Array(h)].map((x) => x.toString(16).padStart(2, "0")).join(""), width: d.width, height: d.height }; })()');
      const dump = op.dump === false ? null : await B.evAsync('window.__app.deskDumpHash()');
      (ctx.hashes ??= []).push({ name: op.name, canvas: canvas?.sha256?.slice(0, 16), peek: peek?.sha256?.slice(0, 16), dump: dump?.sha256?.slice(0, 16), canvasEqPeek: canvas?.sha256 === peek?.sha256, peekEqDump: dump ? peek?.sha256 === dump?.sha256 : null, size: [canvas?.width, canvas?.height] });
      break;
    }
    case 'savePic': {
      const d = await B.evAsync(`window.__app.deskDumpBase64(${op.peek ? 'true' : 'false'})`);
      const rgba = Buffer.from(d.base64, 'base64');
      fs.writeFileSync(path.join(OUT, `${op.name}.png`), encodePng(rgba, d.width, d.height));
      if (op.raw) fs.writeFileSync(path.join(OUT, `${op.name}.rgba`), rgba);
      (ctx.pics ??= []).push({ name: op.name, windows: d.windows.split('\n').filter(Boolean).map((l) => l.replace(/ alpha .*class/, ' class')) });
      break;
    }
    case 'series': {
      // alpha1: `n` screenshots of the page (Chrome: only the box around the points; Firefox: the whole viewport), one after the other as fast as the browser gives them (`gapMs` between), and the colour of each guest-screen point in `points` in every one,
      // with the time (ms since the first). It is the page as the person sees it: engine picture -> canvas -> the browser's compositor -> screenshot. Written to ctx.series[name].
      if (!geo) geo = await geometry();
      const pts = []; for (const [px_, py_] of op.points) pts.push(await toClient(px_, py_));
      const x0 = Math.floor(Math.min(...pts.map((p) => p.x))) - 2, y0 = Math.floor(Math.min(...pts.map((p) => p.y))) - 2, x1 = Math.ceil(Math.max(...pts.map((p) => p.x))) + 3, y1 = Math.ceil(Math.max(...pts.map((p) => p.y))) + 3;
      const rows = []; const t0 = Date.now(); const f = path.join(OUT, `series-${op.name}.png`);
      for (let i = 0; i < (op.n ?? 20); i++) {
        const t = Date.now() - t0;
        await B.shot(f, BROWSER === 'chrome' ? { x: x0, y: y0, width: x1 - x0, height: y1 - y0 } : undefined);
        const png = decodePng(fs.readFileSync(f));
        const ox = BROWSER === 'chrome' ? x0 : 0, oy = BROWSER === 'chrome' ? y0 : 0;
        rows.push({ t, px: pts.map((p) => { const ix = Math.floor(p.x) - ox, iy = Math.floor(p.y) - oy, k = (iy * png.w + ix) * png.bpp; return [png.data[k], png.data[k + 1], png.data[k + 2]]; }) });
        if (op.gapMs) await sleep(op.gapMs);
      }
      (ctx.series ??= {})[op.name] = { points: op.points, rows };
      break;
    }
    default: throw new Error('unknown op ' + op.op);
  }
}

async function handle(ops, keepGoing = false) {
  const res = { results: [] };
  for (const op of ops) {
    try { await runOp(op); } catch (e) { res.results.push({ op, error: String(e.message ?? e).slice(0, 300) }); break; }
    if (!keepGoing && op.op !== 'wait' && op.op !== 'shot' && (await ended().catch(() => false))) break;
  }
  await sleep(ops.length && ops[ops.length - 1].settle !== undefined ? ops[ops.length - 1].settle : 700);
  res.state = await state();
  if (res.state.state === 'exited' || res.state.state === 'error') { res.ended = await exitReport(); res.inputs = sent.slice(-40); }
  return res;
}

try {
  if (has('probe')) {
    await B.goto(URL0);
    await sleep(2000);
    const r = await handle([{ op: 'waitIdle', ms: 600000, minIdles: 20 }, { op: 'wait', ms: 1500 }]);
    const g = await geometry();
    const vis = (ws) => ws.filter((w) => w.vis && w.w > 0).map((w) => `${w.lid} ${w.cls} '${w.title}' @${w.x},${w.y} ${w.w}x${w.h} z${w.z}`);
    console.log(JSON.stringify({ state: r.state, geo: g, windows: vis(await windows()), surfaces: (await surfaces()).length }));
    await runOp({ op: 'shot', name: 'probe-before' });
    await runOp({ op: 'closeWizard' });
    console.log(JSON.stringify({ afterWizard: vis(await windows()) }));
    await runOp({ op: 'shot', name: 'probe-after' });
  } else {
    const scen = JSON.parse(fs.readFileSync(opt('scenarios'), 'utf8')).filter((x) => !ONLY || ONLY.includes(x.name));
    const report = [];
    for (let rep = 1; rep <= REPEAT; rep++) for (const sc of scen) {
      const t0 = Date.now();
      log('scenario', sc.name, REPEAT > 1 ? `(rep ${rep})` : '');
      const loadStart = Number(fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]);
      sent.length = 0; held = 0; ctx.menuCycles = undefined; ctx.expects = []; ctx.openedTitle = undefined; ctx.listed = undefined; ctx.dragRates = undefined; ctx.mems = undefined; ctx.hashes = undefined; ctx.pics = undefined; ctx.series = undefined; ctx.bursts = undefined; ctx.cmpShots = undefined; ctx.glcmps = undefined; ctx.logs = undefined; ctx.marks = undefined; ctx.measures = undefined;
      await B.goto(URL0); geo = null;
      await sleep(2000);
      const ops = [{ op: 'waitIdle', ms: 600000, minIdles: 20 }, { op: 'wait', ms: 1500 }, ...sc.ops];
      let r;
      try { r = await handle(ops, sc.intendedExit === true); } catch (e) { r = { error: String(e.message ?? e), state: await state().catch(() => null), results: [] }; }
      const st = r.state ?? {};
      const row = {
        name: sc.name, what: sc.what, rep, seconds: Math.round((Date.now() - t0) / 1000), firstIdleMs: st.idleAt ?? null, stopped: !!r.ended, state: st.state,
        opErrors: (r.results ?? []).map((x) => `${x.op.op}: ${x.error}`), error: r.error ?? null,
        menuCycles: ctx.menuCycles, expects: ctx.expects, openedTitle: ctx.openedTitle, listed: ctx.listed, dragRates: ctx.dragRates, mems: ctx.mems, hashes: ctx.hashes, pics: ctx.pics, series: ctx.series, bursts: ctx.bursts, cmpShots: ctx.cmpShots, glcmps: ctx.glcmps, logs: ctx.logs, measures: ctx.measures,
      };
      if (r.ended) { row.exitText = (r.ended.exitText ?? '').slice(0, 400); row.beaconExit = r.ended.beaconExit; row.tail = r.ended.tail; row.inputs = r.inputs; }
      try { row.prunedLines = (await jev("window.__app.engineLog().filter((l) => /pruned function/.test(l)).length")); row.screen = await jev('(() => { const a = window.__app; const g = a.screen && a.screen.guest; const r = document.getElementById("desktop").getBoundingClientRect(); return { guest: g, reported: a.screen && a.screen.reported, desktop: [r.width, r.height], viewport: [innerWidth, innerHeight] }; })()'); } catch { /* none */ }
      try { row.timeline = await jev('window.__app.timeline'); row.loadStart = loadStart; row.loadEnd = Number(fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]); } catch { /* none */ }
      try { row.windowsEnd = (await windows()).filter((w) => w.vis).map((w) => `${w.cls}@${w.x},${w.y} ${w.w}x${w.h}`); } catch { /* none */ }
      try { geo = geo ?? (await geometry()); await B.shot(path.join(OUT, `scenario-${sc.name}${REPEAT > 1 ? '-' + rep : ''}.png`), BROWSER === 'chrome' ? { x: geo.x, y: geo.y, width: geo.w, height: geo.h } : undefined); } catch { /* no picture */ }
      row.intendedExit = sc.intendedExit === true;
      row.ok = (!row.stopped || row.intendedExit) && !row.error && row.opErrors.length === 0 && (row.expects ?? []).every((e) => e.ok) && (!row.menuCycles || (row.menuCycles.completed === row.menuCycles.n && row.menuCycles.popupMissingSurface.length === 0 && row.menuCycles.notVisible.length === 0));
      report.push(row);
      log('  ->', row.ok ? (row.stopped ? 'ok (the intended exit)' : 'ok (no stop)') : row.stopped ? 'STOPPED: ' + (row.beaconExit?.message ?? row.exitText ?? '').slice(0, 220) : `NOT OK ${JSON.stringify({ e: row.error, o: row.opErrors, x: row.expects?.filter((e) => !e.ok), m: row.menuCycles })}`.slice(0, 400));
      fs.writeFileSync(path.join(OUT, 'report.json'), JSON.stringify(report, null, 1));
    }
    const md = ['| scenario | what | result | detail |', '|---|---|---|---|', ...report.map((x) => `| ${x.name}${REPEAT > 1 ? ' #' + x.rep : ''} | ${(x.what ?? '').replace(/\|/g, '/')} | ${x.ok ? 'ok' : x.stopped ? 'STOPPED' : 'NOT OK'} | ${x.stopped ? (x.beaconExit?.message ?? x.exitText ?? '').replace(/\|/g, '/').slice(0, 160) : x.menuCycles ? `cycles ${x.menuCycles.completed}/${x.menuCycles.n}, popup surface ${x.menuCycles.popupSeen}` : ''} |`)];
    fs.writeFileSync(path.join(OUT, 'report.md'), md.join('\n') + '\n');
  }
} finally {
  await B.close();
}
process.exit(0);
