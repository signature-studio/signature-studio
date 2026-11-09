// perf2 (decision 0138, PROVISIONAL): a copy of ../final0129/run-chrome-final.mjs whose run loop is for LONG runs (duration=, shot=, hold=; the page is lib of this directory: index.html, main.js, fle-perf2-worker.js; the p2: lines of perf2_probe.c go to <name>.p2.log as they arrive).
// Original header of final0129: a copy of ../perf1/run-chrome-perf.mjs (decision 0130; itself a copy of ../fruity/run-chrome.mjs, decision 0128) that serves THIS directory (page, Worker fle-final-worker.js), takes the screenshot
// of the canvas itself (the page's rectangle, beyond the viewport) for every run, and keeps the Worker's full mark records and heap probes in the result. The rules of isolation are those of decisions 0124 / 0128 / 0130: a new browser context, host-resolver rules,
// no proxy, IPv6 off, net-log in `Default` mode; the verdict is the check of ../threads/check_netlog.mjs (TCP non-loopback 0, UDP bytes sent 0; NETLOG_TOLERATE_IPV6_PROBE=1 tolerates the connect() of the IPv6 probe, which sends nothing).
// usage: node run-chrome-final.mjs name= dist= out= [expect=<json {first,stop}>] [timeout=] [stall=] [enc=none|gz|br] [rate=<B/s>] [cache=1] [cacheapi=gz] [par=1] [seedmode=pack|embed|preload] [streaming=1] [runs=<n>] [env=<JSON of environment additions>]
// perf1 (decision 0130, PROVISIONAL): a copy of ../fruity/run-chrome.mjs (decision 0128) with these additions: enc=none|gz|br (the server sends <file>.gz / <file>.br with Content-Encoding: Chrome decodes it), rate=<bytes/s>
// (the local server paces its writes: an EMULATION of a slower line on loopback, no latency, no TCP dynamics), cache=1 (HTTP cache on: Cache-Control max-age + ETag, the page uses compileStreaming),
// runs=<n> (the same page is navigated n times in the SAME browser context: run 2 is a start with whatever Chrome kept), a request log of the server (path, bytes, start, end), the driver's navigate time,
// and a separate RSS sampler per run. The rules of isolation are those of decision 0128.
// One run of the FRUITY-on wasm in headless Chrome (decision 0128, PROVISIONAL): an isolated Chrome (the rules of decision 0124 and of present1's lib.mjs: new browser context, host-resolver rules, no
// proxy, net-log in `Default` mode), a local COOP/COEP server, lift/harness/web/fruity/index.html with the Worker, waits for window.__done, reads canvas A (read back inside the Worker) and canvas B
// (read back in the page) results, compares them with the expected RGBA SHA-256 of decision 0126's PNGs, takes a screenshot, and checks the net-log.
// usage: node run-chrome-final.mjs name=<case> dist=<dir: flengine6.js flengine6.wasm data/> out=<dir> [expect=<expected.json: {stop: sha, first: sha}>] [timeout=3600] [jsflags='--flag ...'] [env=<JSON>]
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
const ENC = A.enc || 'none', RATE = +(A.rate || 0), CACHE = A.cache === '1', STREAMING = (A.streaming || A.cache || '0') === '1' || A.streaming === '1', RUNS = +(A.runs || 1);
const reqLog = [];
const reqSeen = [];
let lineFreeAt = 0;
if (!dist || !outDir) { console.error('usage: node run-chrome-final.mjs name= dist= out= [expect=] [timeout=] [jsflags=] [env=]'); process.exit(2); }
const WORK = process.env.PRESENT_WORK || '/tmp/opencode/wf1';
fs.mkdirSync(outDir, { recursive: true });
fs.mkdirSync(WORK, { recursive: true });

const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm', '.json': 'application/json' };
function startServer() {
  const server = http.createServer((req, res) => {
    const seen = { path: req.url.slice(0, 80), tSeen: Date.now(), method: req.method, range: req.headers.range || null };
    reqSeen.push(seen);
    res.on('close', () => { seen.closed = Date.now(); seen.finished = res.writableFinished; });
    res.on('error', (e) => { seen.error = String(e); });
    req.on('error', (e) => { seen.reqError = String(e); });
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
      let sendFile = file, size = st.size, encHdr = {};
      if (ENC !== 'none' && p.startsWith('/dist/') && fs.existsSync(file + '.' + ENC)) { sendFile = file + '.' + ENC; size = fs.statSync(sendFile).size; encHdr = { 'Content-Encoding': ENC === 'br' ? 'br' : 'gzip' }; }
      const etag = '"' + st.size + '-' + Math.round(st.mtimeMs) + '-' + ENC + '"';
      if (CACHE && req.headers['if-none-match'] === etag) { res.writeHead(304, { ETag: etag }); res.end(); reqLog.push({ path: p, status: 304, tStart: Date.now(), tEnd: Date.now(), bytes: 0 }); return; }
      const h = Object.assign({ 'Content-Type': TYPES[path.extname(file)] || 'application/octet-stream', 'Content-Length': size, 'Cache-Control': CACHE ? 'max-age=3600' : 'no-store' }, encHdr, CACHE ? { ETag: etag } : {});
      res.writeHead(200, h);
      const rec = { path: p, status: 200, enc: ENC !== 'none' && encHdr['Content-Encoding'] ? ENC : 'none', tStart: Date.now(), tEnd: null, bytes: size };
      reqLog.push(rec);
      if (!RATE) { const rs = fs.createReadStream(sendFile, { highWaterMark: 65536 }); rs.pipe(res); res.on('finish', () => { rec.tEnd = Date.now(); }); }
      else {
        // ONE shared line of RATE bytes per second for all requests (decision 0130: an EMULATION on loopback: no latency, no TCP dynamics). Every 64 KiB chunk takes len / RATE seconds of the line after the chunk before it
        // (lineFreeAt is shared, so concurrent requests interleave by arrival); the file is read into memory first, the chunks are written one after the other, res.end() comes after the last write.
        fs.promises.readFile(sendFile).then(async (buf) => {
          for (let o = 0; o < buf.length; o += 65536) {
            const chunk = buf.subarray(o, Math.min(buf.length, o + 65536));
            lineFreeAt = Math.max(Date.now(), lineFreeAt) + (chunk.length / RATE) * 1000;
            const wait = lineFreeAt - Date.now();
            if (wait > 0) await sleep(wait);
            if (res.destroyed) return;
            if (!res.write(chunk)) await new Promise((r) => res.once('drain', r));
          }
          res.end(() => { rec.tEnd = Date.now(); });
        }).catch((e) => { rec.error = String(e); res.destroy(); });
      }
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
const sampler = new RssSampler(chrome.proc.pid, 2000, false);
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
await S('Runtime.enable'); await S('Page.enable'); await S('Network.enable'); await S('Network.setCacheDisabled', { cacheDisabled: !CACHE });
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
if (STREAMING) q.set('streaming', '1');
if (CACHE) q.set('cache', '1');
if (A.cacheapi === '1' || A.cacheapi === 'gz') q.set('cacheapi', A.cacheapi);
if (A.env) q.set('env', A.env);
if (A.par === '1') q.set('par', '1');
if (A.seedmode) q.set('seedmode', A.seedmode);
const url = `http://127.0.0.1:${port}/?${q}`;
out.runs = [];
// perf2 (decision 0138): ONE navigation, a long run. duration=<s>: leave the engine running for that many seconds from the navigation start, then take the last snapshots and stop (the page is closed with the browser); 0 = until the page says done.
// shot=<s>: a snapshot of every lid's latest frame (P.snap) and the renderer's RSS every s seconds. hold=<s>: if the engine reaches the idle hook (stock fruity_idle.c), stay that long, then send `end` (the engine exits through its own idle stop).
const DURATION = +(A.duration || 0), SHOT_EVERY = +(A.shot || 120), HOLD = +(A.hold || 20), TIMEOUT = +(A.timeout || 3600), STALL_S = +(A.stall || 300);
const p2File = path.join(outDir, `${name}.p2.log`);
fs.writeFileSync(p2File, '');
const tStart = Date.now();
await S('Page.navigate', { url });
let firstSnap = false, done = false, lastSeq = -1, lastChange = Date.now(), p2idx = 0, nextShot = SHOT_EVERY, idleSeenAt = 0, ended = false, stalled = false, endedBy = null;
const rssSeries = [], shots = [];
const saveSnaps = async (tag) => {
  try {
    const lids = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.P.lids())', returnByValue: true }, 30000)).result.value);
    for (const lid of lids) {
      const sn = JSON.parse((await S('Runtime.evaluate', { expression: `JSON.stringify(window.P.snap(${lid}))`, returnByValue: true }, 60000)).result.value);
      if (!sn) continue;
      const f = `${name}.lid${lid}-${tag}.png`;
      fs.writeFileSync(path.join(outDir, f), Buffer.from(sn.png.split(',')[1], 'base64'));
      shots.push({ tag, lid, w: sn.w, h: sn.h, kind: sn.kind, tWorker: sn.tWorker, file: f });
    }
  } catch (e) { consoleLog.push('snap-error: ' + String(e).slice(0, 200)); }
};
while (!done) {
  const el = (Date.now() - tStart) / 1000;
  if (el > TIMEOUT) { endedBy = 'timeout'; break; }
  if (DURATION && el > DURATION) { endedBy = 'duration'; break; }
  await sleep(2000);
  try {
    const r = await S('Runtime.evaluate', { expression: `JSON.stringify({done: window.__done === true, live: window.__live, p2: window.__p2.slice(${p2idx})})`, returnByValue: true }, 60000);
    const v = JSON.parse(r.result.value);
    done = v.done;
    if (v.live.seq !== lastSeq) { lastSeq = v.live.seq; lastChange = Date.now(); }
    if (v.p2.length) { fs.appendFileSync(p2File, v.p2.join('\n') + '\n'); p2idx += v.p2.length; }
    const tS = Math.round((Date.now() - tStart) / 1000);
    if (tS >= nextShot) {
      nextShot += SHOT_EVERY;
      sampler.sample();
      const last = sampler.samples[sampler.samples.length - 1];
      rssSeries.push({ tS, rendererMB: Math.round((last.byType.renderer || 0) / 1024), totalMB: Math.round(last.totalKB / 1024) });
      await saveSnaps('t' + String(tS).padStart(5, '0'));
    }
    if (v.live.frames >= 1 && !firstSnap) { firstSnap = true; await saveSnaps('first'); }
    if (v.live.idles.length && !idleSeenAt) { idleSeenAt = Date.now(); out.idleAtS = Math.round((idleSeenAt - tStart) / 1000); await saveSnaps('idle'); }
    if (idleSeenAt && !ended && Date.now() - idleSeenAt > HOLD * 1000) { ended = true; await saveSnaps('idle-end'); await S('Runtime.evaluate', { expression: 'window.P.end()', returnByValue: true }, 30000); }
    if (Date.now() - lastChange > STALL_S * 1000) { stalled = true; endedBy = 'stall'; break; }
  } catch (e) { consoleLog.push('driver-error: ' + String(e).slice(0, 200)); }
}
out.stalled = stalled; out.endedBy = endedBy || (done ? 'done' : null);
try { const rest = JSON.parse((await S('Runtime.evaluate', { expression: `JSON.stringify(window.__p2.slice(${p2idx}))`, returnByValue: true }, 60000)).result.value); if (rest.length) fs.appendFileSync(p2File, rest.join('\n') + '\n'); } catch (e) { consoleLog.push('final-p2-error: ' + String(e).slice(0, 200)); }
if (!done) {
  try { out.pageState = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__live)', returnByValue: true }, 30000)).result.value); } catch (e) { out.pageState = { error: String(e).slice(0, 200) }; }
  try { out.samples = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__samples)', returnByValue: true }, 60000)).result.value); } catch (e) { out.samples = { error: String(e).slice(0, 200) }; }
  try { out.markFull = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__markFull)', returnByValue: true }, 60000)).result.value); } catch (e) { out.markFull = { error: String(e).slice(0, 200) }; }
  try { out.pingsPong = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__live.pongs)', returnByValue: true }, 30000)).result.value); } catch (e) { /* none */ }
  try { out.heartbeat = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.P.hb())', returnByValue: true }, 30000)).result.value); } catch (e) { out.heartbeat = { error: String(e).slice(0, 200) }; }
  out.workerStateAtStop = [];
  for (const [sid, wurl] of workerSessions) {
    try { const r = await withTimeout(cdp.send('Runtime.evaluate', { expression: 'JSON.stringify(self.wf1State ? self.wf1State() : null)', returnByValue: true }, sid), 15000, 'worker evaluate'); out.workerStateAtStop.push({ url: wurl, state: r.result && r.result.value }); }
    catch (e) { out.workerStateAtStop.push({ url: wurl, error: String(e).slice(0, 200) }); }
  }
}
await saveSnaps('final');
out.completed = done;
out.wallMs = Date.now() - tStart;
out.rssSeries = rssSeries;
out.shots = shots;
if (done) {
  const r = await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__result)', returnByValue: true });
  out.result = JSON.parse(r.result.value);
  out.samples = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__samples)', returnByValue: true }, 60000)).result.value);
  const so = await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__stdoutAll)', returnByValue: true }, 120000);
  fs.writeFileSync(path.join(outDir, `${name}.stdout.txt`), JSON.parse(so.result.value).join('\n') + '\n');
}
try {
  const rc = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.P.canvasRect())', returnByValue: true }, 30000)).result.value);
  const shot = await S('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: true, clip: { x: rc.x, y: rc.y + (await S('Runtime.evaluate', { expression: 'window.scrollY', returnByValue: true }, 30000)).result.value, width: 1930, height: 1092, scale: 1 } }, 60000);
  const buf = Buffer.from(shot.data, 'base64');
  fs.writeFileSync(path.join(outDir, `${name}.screenshot.png`), buf);
  out.screenshot = { bytes: buf.length, file: `${name}.screenshot.png`, canvasRect: rc };
} catch (e) { out.screenshot = { error: String(e).slice(0, 200) }; }
out.runs.push({ run: 1, completed: out.completed, wallMs: out.wallMs, result: out.result ? { markNav: out.result.markNav, markFull: out.result.markFull, heap: out.result.heap, stdoutLines: out.result.stdoutLines, events: out.result.events, doneNav: out.result.doneNav, navTiming: out.result.navTiming, memoryBytes: out.result.memoryBytes, exitCode: out.result.exitCode, grows: out.result.grows } : null, requests: reqLog.map((x) => Object.assign({}, x)) });
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
if (process.env.KEEP_NETLOG) fs.copyFileSync(netlog, path.join(outDir, `${name}.netlog.json`));
fs.rmSync(netlog, { force: true });
if (A.expect && fs.existsSync(A.expect) && out.result) {
  const exp = JSON.parse(fs.readFileSync(A.expect, 'utf8'));
  out.compare = {};
  for (const ev of out.result.events || []) {
    if (!(ev.main ?? ev.lid === 13)) continue;   // decision 0150: the Worker flags the main window's events; a result saved before it has no flag and its main form was lid 13 (those builds only)
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
