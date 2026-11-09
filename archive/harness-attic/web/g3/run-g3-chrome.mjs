// One run of the G3 round trip in headless Chrome (decision 0136, PROVISIONAL): an isolated Chrome (new browser context, host-resolver rules, no proxy, net-log; the rules of decisions 0124 and 0128), a local
// server, lift/harness/web/g3/index.html with the Worker; the REAL input is made with CDP Input.dispatchMouseEvent / dispatchKeyEvent (browser-level events: they reach the page as trusted pointer and key events),
// one command at a time, each after the engine has been idle again.
// usage: node run-g3-chrome.mjs name=<case> dist=<dir: flengine6.js flengine6.wasm data/> out=<dir> script=<file> [expect=<native stdout with g3-snap blocks>] [timeout=<s>] [wall=<s> rate=<n>] [pings=<per second>]
// The script has the commands of harness/fruity_g3.c: snap NAME | move X Y | down [L|R|M] | up [L|R|M] | click X Y [B] (move, down, up as three commands) | key down|up VK | tap VK | advance MS | timers | end.
// wall=<s>: after the script's commands (or instead of them) the clock mode is `wall` (rate = virtual ms per wall ms) for that many seconds while the runner pings the Worker; then `end`.
// env: PRESENT_WORK (profile and net-log directory), CHROME_BIN, NETLOG_TOLERATE_IPV6_PROBE=1
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { launchChrome, Cdp, RssSampler, shutdown, sleep } from '../cdp.mjs';
import { isolationArgs, withTimeout } from '../present/lib.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const A = Object.fromEntries(process.argv.slice(2).map((a) => { const i = a.indexOf('='); return [a.slice(0, i), a.slice(i + 1)]; }));
const name = A.name || 'run';
const dist = A.dist, outDir = A.out;
if (!dist || !outDir) { console.error('usage: node run-g3-chrome.mjs name= dist= out= [script=] [expect=] [timeout=] [wall=] [rate=]'); process.exit(2); }
const WORK = process.env.PRESENT_WORK || '/tmp/opencode/g3in';
fs.mkdirSync(outDir, { recursive: true });
fs.mkdirSync(WORK, { recursive: true });
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm', '.json': 'application/json' };
function startServer() {
  const server = http.createServer((req, res) => {
    const p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    let file;
    if (p === '/') file = path.join(HERE, 'index.html');
    else if (p.startsWith('/dist/')) file = path.join(dist, p.slice(6));
    else file = path.join(HERE, p);
    const root = p.startsWith('/dist/') ? dist : HERE;
    if (!file.startsWith(root) || file.includes('..')) { res.writeHead(403).end('forbidden'); return; }
    fs.stat(file, (err, st) => {
      if (err || !st.isFile()) { res.writeHead(404).end('not found: ' + p); return; }
      res.writeHead(200, { 'Content-Type': TYPES[path.extname(file)] || 'application/octet-stream', 'Content-Length': st.size, 'Cache-Control': 'no-store', 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp' });
      fs.createReadStream(file).pipe(res);
    });
  });
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve({ server, port: server.address().port })));
}

// virtual-key code -> KeyboardEvent.code (the inverse of the page's table, for the keys a script uses)
const CODE = { 8: 'Backspace', 9: 'Tab', 13: 'Enter', 27: 'Escape', 32: 'Space', 37: 'ArrowLeft', 38: 'ArrowUp', 39: 'ArrowRight', 40: 'ArrowDown', 46: 'Delete', 0xA0: 'ShiftLeft', 0xA2: 'ControlLeft', 0xA4: 'AltLeft' };
for (let i = 0; i < 26; i++) CODE[65 + i] = 'Key' + String.fromCharCode(65 + i);
for (let i = 0; i < 10; i++) CODE[48 + i] = 'Digit' + i;
for (let i = 1; i <= 12; i++) CODE[0x6F + i] = 'F' + i;
const KEYTEXT = (vk) => (vk >= 65 && vk <= 90 ? String.fromCharCode(vk + 32) : vk >= 48 && vk <= 57 ? String.fromCharCode(vk) : vk === 13 ? 'Enter' : vk === 27 ? 'Escape' : vk === 32 ? ' ' : vk === 9 ? 'Tab' : vk === 8 ? 'Backspace' : 'Unidentified');
const WINKEY = (vk) => (vk === 0xA0 ? 16 : vk === 0xA2 ? 17 : vk === 0xA4 ? 18 : vk);

const lines = A.script ? fs.readFileSync(A.script, 'utf8').split('\n').map((l) => l.trim()).filter((l) => l && !l.startsWith('#')) : [];
const out = { name, args: A, started: new Date().toISOString(), steps: [] };
const { server, port } = await startServer();
const profile = path.join(WORK, 'profile-' + name);
const netlog = path.join(WORK, 'netlog-' + name + '.json');
fs.rmSync(profile, { recursive: true, force: true });
const chrome = await launchChrome({ profile, extraArgs: [...isolationArgs(profile, netlog), ...(A.jsflags ? [`--js-flags=${A.jsflags}`] : [])] });
out.chrome = { product: chrome.version.Browser, v8: chrome.version['V8-Version'], userAgent: chrome.version['User-Agent'], args: chrome.args.filter((a) => !a.startsWith('--user-data-dir') && !a.startsWith('--crash')) };
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const sampler = new RssSampler(chrome.proc.pid, 500, false);
sampler.start();
const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
const S = (method, params, ms = 180000) => withTimeout(cdp.send(method, params, sessionId), ms, 'CDP ' + method);
const consoleLog = [];
cdp.on((m) => {
  if (m.sessionId === sessionId && m.method === 'Runtime.consoleAPICalled') consoleLog.push(m.params.type + ': ' + m.params.args.map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 300));
  if (m.sessionId === sessionId && m.method === 'Runtime.exceptionThrown') consoleLog.push('EXCEPTION: ' + JSON.stringify(m.params.exceptionDetails).slice(0, 500));
});
await S('Runtime.enable'); await S('Page.enable'); await S('Network.enable'); await S('Network.setCacheDisabled', { cacheDisabled: true });
await S('Emulation.setDeviceMetricsOverride', { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false });
const q = new URLSearchParams({ glue: '/dist/flengine6.js', wasm: '/dist/flengine6.wasm', data: '/dist/data/', mode: 'fixed' });
if (A.env) q.set('env', A.env);
const tStart = Date.now();
await S('Page.navigate', { url: `http://127.0.0.1:${port}/?${q}` });
const ev = async (expr, ms = 60000) => { const r = await S('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true }, ms); if (r.exceptionDetails) throw new Error('page eval: ' + JSON.stringify(r.exceptionDetails).slice(0, 300)); return r.result.value; };
const live = async () => { for (let i = 0; i < 100; i++) { const v = await ev('JSON.stringify(window.__live || null)'); if (v && v !== 'null') return JSON.parse(v); await sleep(200); } throw new Error('the page did not load (window.__live is not set)'); };
const deadline = Date.now() + (+(A.timeout || 1800)) * 1000;
async function until(pred, label, ms) {
  const lim = Math.min(deadline, Date.now() + (ms || 600000));
  for (;;) {
    const L = await live();
    if (pred(L)) return L;
    if (await ev('window.__done === true')) throw new Error('the Worker finished while waiting for ' + label);
    if (Date.now() > lim) throw new Error('timeout waiting for ' + label);
    await sleep(100);
  }
}
let failure = null;
try {
  let L = await until((l) => l.idles >= 1, 'the first idle', 1500000);
  out.firstIdleWallS = +((Date.now() - tStart) / 1000).toFixed(1);
  out.firstIdle = L.lastIdle;
  let mouse = { x: 0, y: 0 };
  let stdLen = 0;
  const stepDone = async (idleBefore, ackWanted) => { const l = await until((x) => x.idles > idleBefore, 'the next idle', 600000); return l; };
  for (const ln of lines) {
    const [cmd, a1, a2, a3] = ln.split(/\s+/);
    const t = Date.now();
    const idle0 = (await live()).idles;
    const acks0 = (await live()).acks.length;
    const rec = { cmd: ln, tWallMs: t - tStart };
    if (cmd === 'snap') {
      const s0 = (await live()).snaps;
      await ev('P.snapshot()');
      await until((x) => x.snaps > s0, 'the snapshot');
      await sleep(300);          // the page hashes each surface after the Worker's message: wait for them to settle
      rec.snapIdle = idle0;
    } else if (cmd === 'probetimer') {
      await ev(`P.probeTimer(${+a1}, ${+a2}, ${+a3})`); await sleep(200);
    } else if (cmd === 'timers') {
      await ev('P.timers()'); await sleep(200);
    } else if (cmd === 'move') {
      mouse = { x: +a1, y: +a2 };
      await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: mouse.x, y: mouse.y, buttons: 0 });
      await stepDone(idle0);
    } else if (cmd === 'down' || cmd === 'up') {
      const button = a1 === 'R' ? 'right' : a1 === 'M' ? 'middle' : 'left';
      await S('Input.dispatchMouseEvent', cmd === 'down' ? { type: 'mousePressed', x: mouse.x, y: mouse.y, button, buttons: button === 'left' ? 1 : button === 'right' ? 2 : 4, clickCount: 1 } : { type: 'mouseReleased', x: mouse.x, y: mouse.y, button, buttons: 0, clickCount: 1 });
      await stepDone(idle0);
    } else if (cmd === 'click') {
      for (const sub of [`move ${a1} ${a2}`, `down ${a3 || 'L'}`, `up ${a3 || 'L'}`]) {
        const i0 = (await live()).idles;
        const [c2, b1, b2] = sub.split(' ');
        if (c2 === 'move') { mouse = { x: +b1, y: +b2 }; await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: mouse.x, y: mouse.y, buttons: 0 }); }
        else await S('Input.dispatchMouseEvent', c2 === 'down' ? { type: 'mousePressed', x: mouse.x, y: mouse.y, button: 'left', buttons: 1, clickCount: 1 } : { type: 'mouseReleased', x: mouse.x, y: mouse.y, button: 'left', buttons: 0, clickCount: 1 });
        await stepDone(i0);
      }
    } else if (cmd === 'key' || cmd === 'tap') {
      const vk = +(cmd === 'key' ? a2 : a1);
      for (const kind of cmd === 'tap' ? ['keyDown', 'keyUp'] : [a1 === 'down' ? 'keyDown' : 'keyUp']) {
        const i0 = (await live()).idles;
        await S('Input.dispatchKeyEvent', { type: kind, key: KEYTEXT(vk), code: CODE[vk], windowsVirtualKeyCode: WINKEY(vk), nativeVirtualKeyCode: WINKEY(vk) });
        await stepDone(i0);
      }
    } else if (cmd === 'advance') {
      const d0 = (await live()).advanceDone;
      await ev(`P.advance(${+a1})`);
      await until((x) => x.advanceDone > d0, 'the advance of ' + a1 + ' ms', 1500000);
    } else if (cmd === 'end') {
      break;
    } else throw new Error('unknown script command ' + ln);
    rec.idlesAfter = (await live()).idles;
    rec.ms = Date.now() - t;
    out.steps.push(rec);
  }
  if (A.wall) {
    // the wall-clock mode: the Worker waits with setTimeout for the engine's earliest deadline; the runner pings the Worker and (optionally) clicks while the engine is idle
    L = await live();
    out.wall = { rate: +(A.rate || 1), seconds: +A.wall, idle0: L.idles, wakes0: L.wakes, pings: [] };
    await ev(`P.mode('wall', ${+(A.rate || 1)})`);
    const tw = Date.now();
    let n = 0;
    while (Date.now() - tw < +A.wall * 1000) {
      await ev(`P.ping(${n})`);
      n++;
      await sleep(+(A.pingms || 500));
    }
    L = await live();
    out.wall.idle1 = L.idles; out.wall.wakes1 = L.wakes; out.wall.pingsSent = n; out.wall.pongs = L.pongs;
    await ev("P.mode('fixed', 1)");
    await sleep(500);
  }
  await ev('P.timers()');
  await sleep(300);
  await ev('P.end()');
  await until(() => ev('window.__done === true').then(() => true), 'the end', 5000).catch(() => {});
  for (let i = 0; i < 50 && !(await ev('window.__done === true')); i++) await sleep(200);
} catch (e) { failure = String((e && e.stack) || e).slice(0, 800); }
out.failure = failure;
out.wallMs = Date.now() - tStart;
try { out.live = await live(); } catch { /* page gone */ }
try { out.inputs = out.live && out.live.inputs; } catch { /* */ }
try { out.surfLog = JSON.parse(await ev('P.surfLog()')); } catch (e) { out.surfLogError = String(e).slice(0, 100); }
try { out.stdout = JSON.parse(await ev('P.stdoutFrom(0)')); fs.writeFileSync(path.join(outDir, `${name}.stdout.txt`), out.stdout.join('\n') + '\n'); delete out.stdout; } catch (e) { out.stdoutError = String(e).slice(0, 100); }
try { out.result = JSON.parse(await ev('JSON.stringify(window.__result || null)')); if (out.result) { delete out.result.stdout; } } catch (e) { out.resultError = String(e).slice(0, 100); }
try {
  const shot = await S('Page.captureScreenshot', { format: 'png', fromSurface: true, clip: { x: 0, y: 0, width: 1920, height: 1080, scale: 1 } }, 60000);
  fs.writeFileSync(path.join(outDir, `${name}.screenshot.png`), Buffer.from(shot.data, 'base64'));
} catch (e) { out.screenshotError = String(e).slice(0, 100); }
sampler.sample();
out.rss = sampler.summary();
sampler.stop();
out.console = consoleLog.slice(0, 40);
await shutdown(chrome, cdp);
server.closeAllConnections(); server.close();
await sleep(400);
const nl = spawnSync('node', [path.join(HERE, '../threads/check_netlog.mjs'), netlog, '127.0.0.1', ...(process.env.NETLOG_TOLERATE_IPV6_PROBE === '1' ? ['tolerate-ipv6-probe'] : [])], { encoding: 'utf8' });
out.netlog = { summary: (nl.stdout || nl.stderr || '').trim(), status: nl.status };
fs.rmSync(profile, { recursive: true, force: true });
fs.rmSync(netlog, { force: true });
fs.writeFileSync(path.join(outDir, `${name}.json`), JSON.stringify(out, null, 1));
console.log(JSON.stringify({ name, failure, firstIdleWallS: out.firstIdleWallS, wallS: Math.round(out.wallMs / 1000), steps: out.steps.length, idles: out.live && out.live.idles, netlog: out.netlog.summary, exit: out.result && out.result.exitCode }, null, 1));
