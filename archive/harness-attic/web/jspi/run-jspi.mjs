// One case of the JSPI measurement in a headless Chrome (docs/investigations/jspi-real-wasm.md). A trimmed measure.mjs: starts the local
// server (../serve.mjs), a Chrome with a dedicated throw-away profile (../cdp.mjs), opens jspi/index-jspi.html, waits for window.__done,
// compares the stdout of the engine with a node run and writes <runs>/<name>.json. Everything in one foreground node process.
// usage: node run-jspi.mjs name=<case> [cfg='{"mode":1,"every_0":1}'] [coi=1 (COOP same-origin + COEP require-corp: performance.now() steps of 5 us instead of 100 us)] [jsflags='--wasm-stack-switching-stack-size=200'] [ref=<node stdout file>] [timeout=300] [profile=<dir>]
// env:   WEB_DIST (dist/: flengine6.wasm, flengine6.web.js, data/)  WEB_RUNS  WEB_PROFILES  CHROME_BIN
import fs from 'node:fs';
import path from 'node:path';
import { startServer } from '../serve.mjs';
import { launchChrome, Cdp, RssSampler, shutdown, sleep } from '../cdp.mjs';

const A = Object.fromEntries(process.argv.slice(2).map((a) => { const i = a.indexOf('='); return [a.slice(0, i), a.slice(i + 1)]; }));
const name = A.name || 'jspi';
const dist = process.env.WEB_DIST;
const runs = process.env.WEB_RUNS;
const profiles = process.env.WEB_PROFILES;
if (!dist || !runs || !profiles) { console.error('set WEB_DIST, WEB_RUNS and WEB_PROFILES'); process.exit(2); }
fs.mkdirSync(runs, { recursive: true });
const out = { name, args: A, started: new Date().toISOString() };
const { server, port } = await startServer({ dist });
const profile = path.join(profiles, A.profile || name);
fs.rmSync(profile, { recursive: true, force: true });
// --password-store=basic: in this environment, without it Page.navigate to the local server never completed and the server saw no request (the cause is NOT confirmed;
// a wait on the desktop keyring is a hypothesis); with it navigation works. See docs/investigations/jspi-real-wasm.md.
const chrome = await launchChrome({ profile, extraArgs: ['--password-store=basic', ...(A.jsflags ? [`--js-flags=${A.jsflags}`] : [])] });
out.chrome = { product: chrome.version.Browser, v8: chrome.version['V8-Version'], userAgent: chrome.version['User-Agent'] };
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const sampler = new RssSampler(chrome.proc.pid, 100, false);
sampler.start();
const t0 = Date.now();
const consoleLog = [];
cdp.on((m) => {
  if (m.method === 'Runtime.consoleAPICalled') consoleLog.push({ t: Date.now() - t0, type: m.params.type, text: m.params.args.map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 500) });
  else if (m.method === 'Runtime.exceptionThrown') consoleLog.push({ t: Date.now() - t0, type: 'exception', text: JSON.stringify(m.params.exceptionDetails).slice(0, 800) });
});
const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
const S = (method, params) => Promise.race([cdp.send(method, params, sessionId), sleep(60000).then(() => { throw new Error('CDP call timed out: ' + method); })]);
await S('Runtime.enable');
await S('Page.enable');
await S('Network.enable');
await S('Network.setCacheDisabled', { cacheDisabled: true });
const q = new URLSearchParams({ auto: '1' });
if (A.cfg) q.set('cfg', A.cfg);
out.url = `http://127.0.0.1:${port}/${A.coi === '1' ? 'coi/' : ''}jspi/index-jspi.html?${q}`;
console.error('navigate', out.url.slice(0, 120));
await S('Page.navigate', { url: out.url });
let done = false;
const deadline = Date.now() + (+(A.timeout || 300)) * 1000;
while (!done && Date.now() < deadline) {
  await sleep(250);
  try {
    const r = await S('Runtime.evaluate', { expression: 'window.__done === true', returnByValue: true });
    done = !!(r.result && r.result.value);
    if (process.env.JSPI_DEBUG) console.error('poll', Date.now() - t0, done);
  } catch (e) { consoleLog.push({ t: Date.now() - t0, type: 'driver-error', text: String(e).slice(0, 300) }); }
}
out.completed = done;
out.finished_at_ms = Date.now() - t0;
if (done) {
  const r = await S('Runtime.evaluate', { expression: 'JSON.stringify(window.__result)', returnByValue: true });
  out.result = JSON.parse(r.result.value);
  if (out.result.stdout) {
    fs.writeFileSync(path.join(runs, `${name}.stdout`), out.result.stdout.join('\n') + '\n');
    if (A.ref && fs.existsSync(A.ref)) {
      const ref = fs.readFileSync(A.ref, 'utf8').replace(/\n$/, '').split('\n');
      const got = out.result.stdout;
      const diffs = [];
      for (let i = 0; i < Math.max(ref.length, got.length); i++) if (ref[i] !== got[i]) diffs.push({ line: i + 1, ref: (ref[i] || '').slice(0, 120), browser: (got[i] || '').slice(0, 120) });
      out.compare = { ref: A.ref, ref_lines: ref.length, browser_lines: got.length, differing_lines: diffs.length, diffs: diffs.slice(0, 10) };
    }
    out.a4_all_units_done = out.result.stdout.filter((l) => l.startsWith('a4: all_units_done=')).length;
    delete out.result.stdout;
  }
}
sampler.sample();
out.rss = sampler.summary();
out.console = consoleLog.slice(0, 40);
sampler.stop();
await shutdown(chrome, cdp);
server.close();
fs.rmSync(profile, { recursive: true, force: true });
fs.writeFileSync(path.join(runs, `${name}.json`), JSON.stringify(out, null, 1));
console.log(JSON.stringify({ name, completed: out.completed, chrome: out.chrome.product, v8: out.chrome.v8, exitCode: out.result && out.result.exitCode, error: out.result && out.result.error,
  marks: out.result && out.result.marks, compare: out.compare && { lines: out.compare.browser_lines, differing: out.compare.differing_lines, diffs: out.compare.diffs },
  records: out.result && out.result.jspi && out.result.jspi.records, renderer_peak_rss_MB: out.rss.busiestRenderer && Math.round(out.rss.busiestRenderer.peakRssKB / 1024),
  tree_peak_rss_MB: Math.round(out.rss.peakTreeRssKB / 1024) }, null, 1));
