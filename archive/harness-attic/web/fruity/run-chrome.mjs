// One run of the FRUITY-on wasm in headless Chrome (decision 0128, PROVISIONAL): an isolated Chrome (the rules of decision 0124 and of present1's lib.mjs: new browser context, host-resolver rules, no
// proxy, net-log in `Default` mode), a local COOP/COEP server, lift/harness/web/fruity/index.html with the Worker, waits for window.__done, reads canvas A (read back inside the Worker) and canvas B
// (read back in the page) results, compares them with the expected RGBA SHA-256 of decision 0126's PNGs, takes a screenshot, and checks the net-log.
// usage: node run-chrome.mjs name=<case> dist=<dir: flengine6.js flengine6.wasm data/> out=<dir> [expect=<expected.json: {stop: sha, first: sha}>] [timeout=3600] [jsflags='--flag ...'] [env=<JSON>]
// env:   PRESENT_WORK (the work dir of lib.mjs: profile and net-log; default /tmp/opencode/present1 -> set it to a directory under /tmp/opencode/wf1)   CHROME_BIN
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { launchChrome, Cdp, RssSampler, shutdown, sleep } from '../cdp.mjs';
import { isolationArgs, withTimeout } from '../present/lib.mjs';
import { decodePng, toRgba, sha256 } from '../present/png.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const A = Object.fromEntries(process.argv.slice(2).map((a) => { const i = a.indexOf('='); return [a.slice(0, i), a.slice(i + 1)]; }));
const name = A.name || 'run';
const dist = A.dist, outDir = A.out;
if (!dist || !outDir) { console.error('usage: node run-chrome.mjs name= dist= out= [expect=] [timeout=] [jsflags=] [env=]'); process.exit(2); }
const WORK = process.env.PRESENT_WORK || '/tmp/opencode/wf1';
fs.mkdirSync(outDir, { recursive: true });
fs.mkdirSync(WORK, { recursive: true });

const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm', '.json': 'application/json' };
function startServer() {
  const server = http.createServer((req, res) => {
    const u = new URL(req.url, 'http://x');
    const p = decodeURIComponent(u.pathname);
    let file;
    if (p === '/') file = path.join(HERE, 'index.html');
    else if (p.startsWith('/dist/')) file = path.join(dist, p.slice(6));
    else file = path.join(HERE, p);
    const root = p.startsWith('/dist/') ? dist : HERE;
    if (!file.startsWith(root) || file.includes('..')) { res.writeHead(403).end('forbidden'); return; }
    fs.stat(file, (err, st) => {
      if (err || !st.isFile()) { res.writeHead(404).end('not found: ' + p); return; }
      const h = { 'Content-Type': TYPES[path.extname(file)] || 'application/octet-stream', 'Content-Length': st.size, 'Cache-Control': 'no-store' };
      res.writeHead(200, h);
      fs.createReadStream(file).pipe(res);
    });
  });
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve({ server, port: server.address().port })));
}

const out = { name, args: A, started: new Date().toISOString() };
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
// the Worker is a child target of the page: attach to it (flatten) so that its console and exceptions, and a state query, are visible (decision 0128: the first run saw nothing of the Worker)
const workerSessions = new Map();
cdp.on((m) => {
  if (m.method === 'Target.attachedToTarget' && m.sessionId === sessionId && m.params.targetInfo.type === 'worker') {
    workerSessions.set(m.params.sessionId, m.params.targetInfo.url);
    cdp.send('Runtime.enable', {}, m.params.sessionId).catch(() => {});
  }
  if (m.sessionId && m.sessionId !== sessionId && workerSessions.has(m.sessionId)) {
    if (m.method === 'Runtime.consoleAPICalled') consoleLog.push('worker ' + m.params.type + ': ' + m.params.args.map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 300));
    if (m.method === 'Runtime.exceptionThrown') consoleLog.push('worker EXCEPTION: ' + JSON.stringify(m.params.exceptionDetails).slice(0, 600));
  }
});
await S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
await S('Emulation.setDeviceMetricsOverride', { width: 1930, height: 1092, deviceScaleFactor: 1, mobile: false });
const q = new URLSearchParams({ glue: '/dist/flengine6.js', wasm: '/dist/flengine6.wasm', data: '/dist/data/' });
if (A.env) q.set('env', A.env);
const url = `http://127.0.0.1:${port}/?${q}`;
const tStart = Date.now();
await S('Page.navigate', { url });
const deadline = Date.now() + (+(A.timeout || 3600)) * 1000;
let done = false, last = 0, lastSeq = -1, lastChange = Date.now(), stalled = false, quitSeenAt = 0;
// G3 (decision 0128): g3=<n> sends n real mouse moves, one after each idle of the engine (the first idle is the one at the end of CreateFruityInstance); then ends the session with the `end` command.
const G3 = +(A.g3 || 0);
const G3KIND = A.g3kind || 'move';   // move: a mouseMoved per idle; click: a mousePressed at the first idle, a mouseReleased at the next one
let sentInputs = 0, lastIdleCount = 0;
const g3log = [];
const STALL_S = +(A.stall || 300);
const progFile = path.join(outDir, `${name}.progress.jsonl`);
fs.writeFileSync(progFile, '');
while (!done && Date.now() < deadline) {
  await sleep(2000);
  try {
    const r = await S('Runtime.evaluate', { expression: 'JSON.stringify({done: window.__done === true, live: window.__live})', returnByValue: true }, 30000);
    const v = JSON.parse(r.result.value);
    done = v.done;
    if (v.live.seq !== lastSeq) { lastSeq = v.live.seq; lastChange = Date.now(); }
    if (Date.now() - last > 5000 || done) { last = Date.now(); fs.appendFileSync(progFile, JSON.stringify({ tS: Math.round((Date.now() - tStart) / 1000), done, live: v.live }) + '\n'); }
    if (G3 && v.live.idles.length > lastIdleCount) {
      lastIdleCount = v.live.idles.length;
      if (sentInputs < G3) {
        // the engine is suspended in its idle: a real browser pointer event over canvas A (Input.dispatchMouseEvent goes through the browser's input pipeline to the page's pointermove handler)
        const rect = (await S('Runtime.evaluate', { expression: 'JSON.stringify(window.P.canvasRect())', returnByValue: true }, 30000)).result.value;
        const rc = JSON.parse(rect);
        const px = rc.x + 400 + 37 * sentInputs, py = rc.y + 300 + 21 * sentInputs;
        const tSend = Date.now();
        if (G3KIND === 'click') await S('Input.dispatchMouseEvent', sentInputs === 0 ? { type: 'mousePressed', x: rc.x + 900, y: rc.y + 600, button: 'left', buttons: 1, clickCount: 1 } : { type: 'mouseReleased', x: rc.x + 900, y: rc.y + 600, button: 'left', buttons: 0, clickCount: 1 }, 30000);
        else await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: px, y: py, buttons: 0 }, 30000);
        g3log.push({ idle: lastIdleCount, input: sentInputs, cssX: px, cssY: py, tSendMs: tSend - tStart });
        sentInputs++;
      } else if (!v.live.ended) {
        await S('Runtime.evaluate', { expression: 'window.P.end()', returnByValue: true }, 30000);
        g3log.push({ idle: lastIdleCount, end: true, tMs: Date.now() - tStart });
        await S('Runtime.evaluate', { expression: 'window.__live.ended = true', returnByValue: true }, 30000);
      }
    }
    if (Date.now() - lastChange > STALL_S * 1000) { stalled = true; break; }
    if (v.live.quit && !quitSeenAt) quitSeenAt = Date.now();
    if (quitSeenAt && !done && Date.now() - quitSeenAt > 60000) { stalled = true; break; }   // the engine exited and no `done` came within 60 s
  } catch (e) { consoleLog.push('driver-error: ' + String(e).slice(0, 200)); }
}
out.stalled = stalled;
out.g3 = g3log;
if (stalled || !done) {
  // read the state once more, from the page and from the Worker (a Worker blocked inside wasm does not answer: that is itself the answer)
  try { out.pageStateAtStop = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__live)', returnByValue: true }, 30000)).result.value); } catch (e) { out.pageStateAtStop = { error: String(e).slice(0, 200) }; }
  try { out.pageFramesAtStop = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(frames)', returnByValue: true }, 30000)).result.value); } catch (e) { out.pageFramesAtStop = { error: String(e).slice(0, 200) }; }
  out.workerStateAtStop = [];
  for (const [sid, wurl] of workerSessions) {
    try { const r = await withTimeout(cdp.send('Runtime.evaluate', { expression: 'JSON.stringify(self.wf1State ? self.wf1State() : null)', returnByValue: true }, sid), 15000, 'worker evaluate'); out.workerStateAtStop.push({ url: wurl, state: r.result && r.result.value }); }
    catch (e) { out.workerStateAtStop.push({ url: wurl, error: String(e).slice(0, 200) }); }
  }
}
out.completed = done;
out.wallMs = Date.now() - tStart;
if (done) {
  const r = await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__result)', returnByValue: true });
  out.result = JSON.parse(r.result.value);
  const so = await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__stdout)', returnByValue: true });
  fs.writeFileSync(path.join(outDir, `${name}.stdout.txt`), JSON.parse(so.result.value).join('\n') + '\n');
  const b = await S('Runtime.evaluate', { expression: 'window.P.shaB()', awaitPromise: true, returnByValue: true });
  out.canvasB_final = b.result.value;
  // a screenshot of the page area of canvas A (the Worker's OffscreenCanvas shows on the page) and the SHA-256 of its RGBA
  try {
    const shot = await S('Page.captureScreenshot', { format: 'png', fromSurface: true, clip: { x: 8, y: 0, width: 1930, height: 1092, scale: 1 } }, 60000);
    const buf = Buffer.from(shot.data, 'base64');
    fs.writeFileSync(path.join(outDir, `${name}.screenshot.png`), buf);
    out.screenshot = { bytes: buf.length };
  } catch (e) { out.screenshot = { error: String(e).slice(0, 200) }; }
}
sampler.sample();
out.rss = sampler.summary();
sampler.stop();
out.console = consoleLog.slice(0, 60);
await shutdown(chrome, cdp);
server.closeAllConnections(); server.close();
await sleep(400);
const nl = spawnSync('node', [path.join(HERE, '../threads/check_netlog.mjs'), netlog, '127.0.0.1', ...(process.env.NETLOG_TOLERATE_IPV6_PROBE === '1' ? ['tolerate-ipv6-probe'] : [])], { encoding: 'utf8' });
out.netlog = { summary: (nl.stdout || nl.stderr || '').trim(), status: nl.status };
fs.rmSync(profile, { recursive: true, force: true });
fs.rmSync(netlog, { force: true });
if (A.expect && fs.existsSync(A.expect) && out.result) {
  const exp = JSON.parse(fs.readFileSync(A.expect, 'utf8'));
  out.compare = {};
  for (const ev of out.result.events || []) {
    if (ev.lid !== 13) continue;
    const key = ev.kind === 1 ? 'first' : 'stop';
    out.compare[key] = { expected: exp[key], module_rgba: ev.rgbaSha256, canvasA_readback: ev.canvasA_readback_sha256, equal_module: ev.rgbaSha256 === exp[key], equal_canvasA: ev.canvasA_readback_sha256 === exp[key] };
  }
  for (const f of out.result.frames || []) {
    const key = f.kind === 1 ? 'first' : 'stop';
    out.compare[key] = Object.assign(out.compare[key] || { expected: exp[key] }, { canvasB_readback: f.canvasB_readback_sha256, equal_canvasB: f.canvasB_readback_sha256 === exp[key] });
  }
}
fs.writeFileSync(path.join(outDir, `${name}.json`), JSON.stringify(out, null, 1));
console.log(JSON.stringify({ name, completed: out.completed, chrome: out.chrome.product, v8: out.chrome.v8, wallS: Math.round(out.wallMs / 1000), exitCode: out.result && out.result.exitCode, error: out.result && out.result.error,
  compare: out.compare, memoryBytes: out.result && out.result.memoryBytes, grows: out.result && out.result.grows && out.result.grows.length, maxHeartbeatGapMs: out.result && out.result.maxHeartbeatGapMs,
  renderer_peak_rss_MB: out.rss.busiestRenderer && Math.round(out.rss.busiestRenderer.peakRssKB / 1024), tree_peak_rss_MB: Math.round(out.rss.peakTreeRssKB / 1024), netlog: out.netlog.summary }, null, 1));
