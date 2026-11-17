// Decision 0220, job 2: is the COMPILED code of the engine wasm cached by the browser, and does it matter where the Response comes from? Three navigations in one browser (cache enabled, profile kept):
//   mode "fetch":   WebAssembly.compileStreaming(fetch(url))   -- an HTTP-cache Response with a URL (the original path)
//   mode "buffer":  WebAssembly.compile(arrayBuffer)           -- bytes without a URL (what the stored copy gives)
//   mode "stream":  WebAssembly.compileStreaming(new Response(arrayBuffer))
// Each compile is timed from the call to the resolved Module; between navigations the page waits (--gap-s) so that the browser can finish tiering and write whatever it caches.
// Usage: node test/e2e/cache-compile-probe.mjs --browser chrome|firefox --port 8802 --mode fetch|buffer|stream [--navs 3] [--gap-s 8] [--out FILE]
import fs from 'node:fs';
import path from 'node:path';
import { Cdp, launchChrome, shutdown, sleep } from './cdp.mjs';
import { openFirefox } from './firefox-driver.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const which = opt('browser', 'chrome'), port = Number(opt('port', '8802')), mode = opt('mode', 'fetch'), navs = Number(opt('navs', '3')), gap = Number(opt('gap-s', '8'));
const base = `http://127.0.0.1:${port}`;
const WORK = '/home/ubuntu/work/signature-studio-develop/lift/out-cache1/work';
const URL_ = '/a/fl-engine-ship2-F2xH2/engine.f4a57e84a5e1.wasm';
const code = `(async () => {
  const url = ${JSON.stringify(URL_)}, mode = ${JSON.stringify(mode)};
  let buf = null, tFetch = 0;
  if (mode !== 'fetch') { const t = performance.now(); buf = await (await fetch(url)).arrayBuffer(); tFetch = performance.now() - t; }
  const t0 = performance.now();
  let m;
  if (mode === 'fetch') m = await WebAssembly.compileStreaming(fetch(url));
  else if (mode === 'buffer') m = await WebAssembly.compile(buf);
  else m = await WebAssembly.compileStreaming(new Response(buf, { headers: { 'Content-Type': 'application/wasm' } }));
  return { compileMs: Math.round(performance.now() - t0), fetchMs: Math.round(tFetch), exports: WebAssembly.Module.exports(m).length };
})()`;
const rows = [];
let version = '';
if (which === 'chrome') {
  const profile = path.join(WORK, 'compile-profile-chrome');
  fs.rmSync(profile, { recursive: true, force: true });
  const chrome = await launchChrome({ profile, extraArgs: ['--no-proxy-server', '--disable-background-networking'] });
  version = chrome.version.Browser;
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
  await cdp.opened;
  try {
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    await cdp.send('Page.enable', {}, sessionId); await cdp.send('Runtime.enable', {}, sessionId);
    for (let n = 1; n <= navs; n++) {
      await cdp.send('Page.navigate', { url: base + '/p.html?n=' + n }, sessionId);
      await sleep(600);
      const r = await cdp.send('Runtime.evaluate', { expression: code, awaitPromise: true, returnByValue: true }, sessionId);
      if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 300));
      rows.push({ n, ...r.result.value, load: fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0] });
      await sleep(gap * 1000);
    }
  } finally { await shutdown(chrome, cdp); }
} else {
  const work = path.join(WORK, 'ff-compile-work'); fs.mkdirSync(work, { recursive: true });
  const ff = await openFirefox({ work, downloadDir: path.join(work, 'dl'), dataDir: work, scriptTimeoutMs: 300000 });
  version = ff.version;
  try {
    for (let n = 1; n <= navs; n++) {
      await ff.goto(base + '/p.html?n=' + n);
      await sleep(600);
      const r = await ff.ev(code);
      rows.push({ n, ...r, load: fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0] });
      await sleep(gap * 1000);
    }
  } finally { await ff.close(); }
}
const lines = [`## ${which} ${version}, mode ${mode}: time from the compile call to the Module, ${navs} navigations ${gap} s apart (wasm 130 MB, served by the probe server, immutable)`, '', '| navigation | compile ms | fetch of the bytes ms (buffer / stream modes) | load |', '|---:|---:|---:|---:|'];
for (const r of rows) lines.push(`| ${r.n} | ${r.compileMs} | ${r.fetchMs} | ${r.load} |`);
console.log(lines.join('\n'));
if (opt('out', '')) fs.appendFileSync(opt('out'), lines.join('\n') + '\n\n');
