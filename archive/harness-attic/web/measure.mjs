// Measurement driver, one case per invocation, everything in one foreground node process:
//   starts the local HTTP server (serve.mjs), starts a headless Chrome with a throw-away profile, drives it over CDP, samples the RSS
//   of the Chrome process tree, writes runs/<name>.json (+ runs/<name>.stdout), stops Chrome and the server.
// usage: node measure.mjs run name=<case> [fast=1] [mode=instantiate|compile|buffer] [enc=identity|gzip|br|zstd] [coi=0|1] [cache=0|1]
//                        [jsflags="--liftoff-only"] [timeout=900] [profile=<dir name>] [reuseprofile=0|1] [keepprofile=0|1] [ref=<node stdout file>] [pss=1 (read smaps_rollup; slows the renderer)] [sample=<ms>] [chromestdout=1]
//        node measure.mjs features name=<case> [coi=0|1]
// env:   WEB_DIST (default /tmp/opencode/web/dist)  WEB_RUNS (default /tmp/opencode/web/runs)  WEB_PROFILES (default /tmp/opencode/web/profiles)
// fast=0 (the unmodified emscripten_return_address, about 30 minutes under node) is refused unless allowstock=1.
import fs from 'node:fs';
import path from 'node:path';
import { startServer } from './serve.mjs';
import { launchChrome, Cdp, RssSampler, shutdown, sleep } from './cdp.mjs';

const [mode0, ...rest] = process.argv.slice(2);
const A = Object.fromEntries(rest.map((a) => { const i = a.indexOf('='); return [a.slice(0, i), a.slice(i + 1)]; }));
const name = A.name || mode0;
const dist = process.env.WEB_DIST || '/tmp/opencode/web/dist';
const runs = process.env.WEB_RUNS || '/tmp/opencode/web/runs';
const profiles = process.env.WEB_PROFILES || '/tmp/opencode/web/profiles';
const timeoutS = +(A.timeout || 900);
fs.mkdirSync(runs, { recursive: true });
if (mode0 === 'run' && A.fast === '0' && A.allowstock !== '1') { console.error('fast=0 is the unmodified driver (about 30 minutes under node); refused (allowstock=1 to override)'); process.exit(2); }

const out = { name, args: A, started: new Date().toISOString() };
const serverLog = [];
const { server, port } = await startServer({ dist, log: serverLog });
const profile = path.join(profiles, A.profile || name);
if (A.reuseprofile !== '1') fs.rmSync(profile, { recursive: true, force: true });
const extra = A.jsflags ? [`--js-flags=${A.jsflags}`] : [];
const stdoutFile = A.chromestdout === '1' ? path.join(runs, `${name}.chrome-stdout`) : null;
const chrome = await launchChrome({ profile, extraArgs: extra, stdoutFile, devShm: A.devshm === '1' });
out.chrome = { product: chrome.version.Browser, protocol: chrome.version['Protocol-Version'], v8: chrome.version['V8-Version'], userAgent: chrome.version['User-Agent'], args: chrome.args.filter((a) => !a.startsWith('--remote-debugging-port')) };
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const sampler = new RssSampler(chrome.proc.pid, +(A.sample || 100), A.pss === '1');
sampler.start();
const t0 = Date.now();

const consoleLog = [];
const crashes = [];
const workers = [];
cdp.on((m) => {
  if (m.method === 'Runtime.consoleAPICalled') consoleLog.push({ t: Date.now() - t0, type: m.params.type, text: m.params.args.map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 500) });
  else if (m.method === 'Runtime.exceptionThrown') consoleLog.push({ t: Date.now() - t0, type: 'exception', text: JSON.stringify(m.params.exceptionDetails).slice(0, 800) });
  else if (m.method === 'Inspector.targetCrashed' || m.method === 'Target.targetCrashed') crashes.push({ t: Date.now() - t0, method: m.method, params: m.params });
  else if (m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') workers.push({ sessionId: m.params.sessionId, url: m.params.targetInfo.url });
  else if (m.method === 'Target.detachedFromTarget') { /* worker ended */ }
});

const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
const S = (method, params) => cdp.send(method, params, sessionId);
await S('Runtime.enable');
await S('Page.enable');
await S('Network.enable');
await S('Network.setCacheDisabled', { cacheDisabled: A.cache !== '1' });
await S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });

const q = new URLSearchParams();
let page;
if (mode0 === 'run') {
  q.set('auto', '1'); q.set('keep', '1');
  for (const k of ['fast', 'mode', 'enc', 'cache', 'root', 'rate']) if (A[k] !== undefined) q.set(k, A[k]);
  if (A.fast === undefined) q.set('fast', '1');
  page = `${A.coi === '1' ? 'coi/' : ''}index.html?${q}`;
} else {
  page = `${A.coi === '1' ? 'coi/' : ''}${A.page || 'features.html'}`;
}
out.url = `http://127.0.0.1:${port}/${page}`;
out.navigate_at_ms = Date.now() - t0;
const traceEvents = [];
if (A.trace === '1') {
  cdp.on((m) => { if (m.method === 'Tracing.dataCollected') for (const e of m.params.value) traceEvents.push(e); });
  await cdp.send('Tracing.start', { categories: A.tracecats || 'v8.wasm,disabled-by-default-v8.wasm.detailed', transferMode: 'ReportEvents' });
}
await S('Page.navigate', { url: out.url });

const events = [];
let seen = 0;
let done = false;
const deadline = Date.now() + timeoutS * 1000;
while (!done && Date.now() < deadline && crashes.length === 0) {
  await sleep(250);
  try {
    const r = await S('Runtime.evaluate', { expression: `JSON.stringify({done: window.__done === true, ev: (window.__events||[]).slice(${seen})})`, returnByValue: true });
    if (r.result && r.result.value) {
      const v = JSON.parse(r.result.value);
      for (const e of v.ev) events.push({ driver_ms: Date.now() - t0, ...e });
      seen += v.ev.length;
      done = v.done;
    }
  } catch (e) { consoleLog.push({ t: Date.now() - t0, type: 'driver-error', text: String(e).slice(0, 300) }); }
}
out.finished_at_ms = Date.now() - t0;
out.completed = done;
out.crashed = crashes;
out.timed_out = !done && crashes.length === 0;
out.events = events;

if (done) {
  const r = await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__result)', returnByValue: true });
  out.result = JSON.parse(r.result.value);
  if (mode0 === 'run' && out.result.stdout) {
    fs.writeFileSync(path.join(runs, `${name}.stdout`), out.result.stdout.join('\n') + '\n');
    const refFile = A.ref || path.join(runs, 'node-fast.out');
    if (fs.existsSync(refFile)) {
      const ref = fs.readFileSync(refFile, 'utf8').replace(/\n$/, '').split('\n');
      const got = out.result.stdout;
      const diffs = [];
      for (let i = 0; i < Math.max(ref.length, got.length); i++) if (ref[i] !== got[i]) diffs.push({ line: i + 1, node: ref[i], browser: got[i] });
      out.compare_with_node = { ref: refFile, node_lines: ref.length, browser_lines: got.length, differing_lines: diffs.length, diffs: diffs.slice(0, 10) };
    }
    delete out.result.stdout;
  }
}
if (A.trace === '1') {
  const ended = new Promise((res) => cdp.on((m) => { if (m.method === 'Tracing.tracingComplete') res(); }));
  await cdp.send('Tracing.end');
  await Promise.race([ended, sleep(60000)]);
  const byName = {};
  for (const e of traceEvents) { const k = `${e.cat}|${e.name}|${e.ph}`; byName[k] = (byName[k] || 0) + 1; }
  out.trace_summary = Object.entries(byName).sort((a, b) => b[1] - a[1]).slice(0, 60);
  out.trace_events = traceEvents.length;
  fs.writeFileSync(path.join(runs, `${name}.trace.json`), JSON.stringify(traceEvents.filter((e) => /wasm/.test(e.cat)).slice(0, 200000)));
}
for (const w of workers) {
  try { const h = await cdp.send('Runtime.getHeapUsage', {}, w.sessionId); out.worker_heap_usage = h; } catch (e) { out.worker_heap_error = String(e).slice(0, 200); }
}
try { out.page_heap_usage = await S('Runtime.getHeapUsage'); } catch { /* ignore */ }
try { out.performance_metrics = (await S('Performance.getMetrics')).metrics; } catch { /* Performance domain not enabled */ }
sampler.sample();
out.rss = sampler.summary();
out.server_log = serverLog;
out.console = consoleLog.slice(0, 60);
out.workers = workers;
out.profile_MB = (() => { try { let n = 0; const walk = (d) => { for (const e of fs.readdirSync(d, { withFileTypes: true })) { const p = path.join(d, e.name); if (e.isDirectory()) walk(p); else if (e.isFile()) n += fs.statSync(p).size; } }; walk(profile); return Math.round(n / 1048576); } catch { return null; } })();
sampler.stop();
await shutdown(chrome, cdp);
server.close();
if (A.keepprofile !== '1') fs.rmSync(profile, { recursive: true, force: true });
fs.writeFileSync(path.join(runs, `${name}.json`), JSON.stringify(out, null, 1));
console.log(JSON.stringify({ name, completed: out.completed, timed_out: out.timed_out, crashed: out.crashed.length, exitCode: out.result && out.result.exitCode, error: out.result && out.result.error, marks: out.result && out.result.marks, compare: out.compare_with_node && { lines: out.compare_with_node.browser_lines, differing: out.compare_with_node.differing_lines, diffs: out.compare_with_node.diffs.map((d) => d.line) }, rss_busiest_renderer: out.rss.busiestRenderer, tree_peak_rss_MB: Math.round(out.rss.peakTreeRssKB / 1024), profile_MB: out.profile_MB }, null, 1));
