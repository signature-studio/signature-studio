// Runner of the audio prototype (decision 0139, PROVISIONAL): one isolated headless Chrome (the rules of decisions 0124 / 0128 / 0134: a new browser context, host-resolver rules, no proxy, IPv6 off, net-log; the verdict is
// ../threads/check_netlog.mjs), a local COOP/COEP server for this directory, ../final0129 (the engine Worker, read only) and the engine's dist directory, the page index.html. Waits for window.__audioDone, writes <out>/<name>.json.
// usage: node run-audio.mjs name= dist= out= [timeout=<s>] [stall=<s>] [q=<query string for the page>] [flags="--a --b"]
// env: PRESENT_WORK (profile and net-log directory), NETLOG_TOLERATE_IPV6_PROBE=1
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { launchChrome, Cdp, RssSampler, shutdown, sleep } from '../cdp.mjs';
import { isolationArgs, withTimeout } from '../present/lib.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const A = Object.fromEntries(process.argv.slice(2).map((a) => { const i = a.indexOf('='); return [a.slice(0, i), a.slice(i + 1)]; }));
const name = A.name || 'audio', dist = A.dist, outDir = A.out;
if (!dist || !outDir) { console.error('usage: node run-audio.mjs name= dist= out= [timeout=] [stall=] [q=] [flags=]'); process.exit(2); }
const WORK = process.env.PRESENT_WORK || '/tmp/opencode/audio1/chrome';
fs.mkdirSync(outDir, { recursive: true }); fs.mkdirSync(WORK, { recursive: true });
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm', '.json': 'application/json' };
const FINAL = path.join(HERE, '../final0129');
const server = http.createServer((req, res) => {
  const p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  let file, root;
  if (p.startsWith('/dist/')) { root = dist; file = path.join(dist, p.slice(6)); }
  else if (p.startsWith('/final/')) { root = FINAL; file = path.join(FINAL, p.slice(7)); }
  else { root = HERE; file = path.join(HERE, p === '/' ? 'index.html' : p); }
  if (!file.startsWith(root) || file.includes('..')) { res.writeHead(403).end('forbidden'); return; }
  fs.stat(file, (err, st) => {
    if (err || !st.isFile()) { res.writeHead(404).end('not found: ' + p); return; }
    res.writeHead(200, { 'Content-Type': TYPES[path.extname(file)] || 'application/octet-stream', 'Content-Length': st.size, 'Cache-Control': 'no-store', 'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp', 'Cross-Origin-Resource-Policy': 'same-origin' });
    fs.createReadStream(file).pipe(res);
  });
});
const port = await new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(server.address().port)));
const profile = path.join(WORK, 'profile-' + name), netlog = path.join(WORK, 'netlog-' + name + '.json');
fs.rmSync(profile, { recursive: true, force: true });
const out = { name, args: A, started: new Date().toISOString() };
const chrome = await launchChrome({ profile, extraArgs: [...isolationArgs(profile, netlog), '--autoplay-policy=no-user-gesture-required', ...(A.flags ? A.flags.split(' ') : [])] });
out.chrome = { product: chrome.version.Browser, v8: chrome.version['V8-Version'], args: chrome.args.filter((a) => !a.startsWith('--user-data-dir') && !a.startsWith('--crash')) };
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const sampler = new RssSampler(chrome.proc.pid, 500, false);
sampler.start();
const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
const S = (method, params, ms = 120000) => withTimeout(cdp.send(method, params, sessionId), ms, 'CDP ' + method);
const consoleLog = [];
const workerSessions = new Map();
cdp.on((m) => {
  if (m.sessionId === sessionId && m.method === 'Runtime.consoleAPICalled') consoleLog.push(m.params.type + ': ' + m.params.args.map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 300));
  if (m.sessionId === sessionId && m.method === 'Runtime.exceptionThrown') consoleLog.push('EXCEPTION: ' + JSON.stringify(m.params.exceptionDetails).slice(0, 500));
  if (m.method === 'Target.attachedToTarget' && m.sessionId === sessionId && m.params.targetInfo.type === 'worker') { workerSessions.set(m.params.sessionId, m.params.targetInfo.url); cdp.send('Runtime.enable', {}, m.params.sessionId).catch(() => {}); }
  if (m.sessionId && m.sessionId !== sessionId && workerSessions.has(m.sessionId)) {
    if (m.method === 'Runtime.consoleAPICalled') consoleLog.push('worker ' + m.params.type + ': ' + m.params.args.map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 300));
    if (m.method === 'Runtime.exceptionThrown') consoleLog.push('worker EXCEPTION: ' + JSON.stringify(m.params.exceptionDetails).slice(0, 600));
  }
});
await S('Runtime.enable'); await S('Page.enable'); await S('Network.enable'); await S('Network.setCacheDisabled', { cacheDisabled: true });
await S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
const url = `http://127.0.0.1:${port}/${A.q ? '?' + A.q : ''}`;
const tStart = Date.now();
await S('Page.navigate', { url });
const deadline = Date.now() + (+(A.timeout || 600)) * 1000;
const STALL = +(A.stall || 300);
const progFile = path.join(outDir, `${name}.progress.jsonl`);
fs.writeFileSync(progFile, '');
let done = false, last = 0, lastSeq = -1, lastChange = Date.now(), stalled = false;
while (!done && Date.now() < deadline) {
  await sleep(2000);
  try {
    const r = await S('Runtime.evaluate', { expression: 'JSON.stringify({done: window.__audioDone === true, live: window.__live, c: window.P && window.P.counters ? window.P.counters() : null})', returnByValue: true }, 30000);
    const v = JSON.parse(r.result.value);
    done = v.done;
    const sig = v.live.seq + ':' + v.live.engineMessages;
    if (sig !== String(lastSeq)) { lastSeq = sig; lastChange = Date.now(); }
    if (Date.now() - last > 5000 || done) { last = Date.now(); fs.appendFileSync(progFile, JSON.stringify({ tS: Math.round((Date.now() - tStart) / 1000), done, state: v.live.state, c: v.c, marks: v.live.marks.slice(-2) }) + '\n'); }
    if (Date.now() - lastChange > STALL * 1000) { stalled = true; break; }
  } catch (e) { consoleLog.push('driver-error: ' + String(e).slice(0, 200)); }
}
out.completed = done; out.stalled = stalled; out.wallMs = Date.now() - tStart;
if (done) { out.result = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__audio)', returnByValue: true })).result.value); }
else { try { out.live = JSON.parse((await S('Runtime.evaluate', { expression: 'JSON.stringify({live: window.__live, c: window.P && window.P.counters && window.P.counters()})', returnByValue: true }, 30000)).result.value); } catch (e) { out.live = { error: String(e) }; } }
sampler.sample(); out.rss = sampler.summary(); sampler.stop();
out.console = consoleLog.slice(0, 80);
await shutdown(chrome, cdp);
server.closeAllConnections(); server.close();
await sleep(400);
const nl = spawnSync('node', [path.join(HERE, '../threads/check_netlog.mjs'), netlog, '127.0.0.1', ...(process.env.NETLOG_TOLERATE_IPV6_PROBE === '1' ? ['tolerate-ipv6-probe'] : [])], { encoding: 'utf8' });
out.netlog = { summary: (nl.stdout || nl.stderr || '').trim(), status: nl.status };
fs.rmSync(profile, { recursive: true, force: true });
fs.rmSync(netlog, { force: true });
fs.writeFileSync(path.join(outDir, `${name}.json`), JSON.stringify(out, null, 1));
console.log(JSON.stringify({ name, completed: out.completed, stalled: out.stalled, wallS: Math.round(out.wallMs / 1000), chrome: out.chrome.product, tree_peak_rss_MB: Math.round(out.rss.peakTreeRssKB / 1024), netlog: out.netlog.summary }, null, 1));
