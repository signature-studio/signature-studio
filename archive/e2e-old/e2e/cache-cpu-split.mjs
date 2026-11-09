// Decision 0220: where does the page's "download" time go when the bytes come from the browser's HTTP cache? For each of the four assets (read from the probe server with the
// production headers, stored brotli): fetch + read all chunks (network / cache + the browser's brotli decode), then SHA-256 of the chunks in JS (src/bundle/sha256.ts), then one buffer + WebCrypto digest.
// Usage: node test/e2e/cache-cpu-split.mjs --browser chrome|firefox --port 8802 [--rounds 3] [--out FILE]
import fs from 'node:fs';
import path from 'node:path';
import module from 'node:module';
import { Cdp, launchChrome, shutdown, sleep } from './cdp.mjs';
import { openFirefox } from './firefox-driver.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const which = opt('browser', 'chrome');
const port = Number(opt('port', '8802'));
const rounds = Number(opt('rounds', '3'));
const base = `http://127.0.0.1:${port}`;
const WORK = '/home/ubuntu/work/signature-studio-develop/lift/out-cache1/work';
const sha = module.stripTypeScriptTypes(fs.readFileSync(new URL('../../src/bundle/sha256.ts', import.meta.url), 'utf8')).replace(/export /g, '');
const files = ['fl-engine-ship2-F2xH2/engine.f4a57e84a5e1.wasm', 'fl-engine-ship2-F2xH2/dll.50206405ba5f.flpack', 'fl-engine-ship2-F2xH2/seed.0f4b547e6656.flpack', 'fl-engine-ship2-F2xH2/engine-glue.b5952d3e5165.js'];
const script = `${sha}
return (async () => {
  const files = ${JSON.stringify(files)};
  const rounds = ${rounds};
  const out = [];
  for (let r = 0; r < rounds; r++) for (const f of files) {
    const t0 = performance.now();
    const res = await fetch('/a/' + f);
    const hdr = performance.now() - t0;
    const reader = res.body.getReader();
    const chunks = []; let total = 0, n = 0;
    for (;;) { const x = await reader.read(); if (x.done) break; chunks.push(x.value); total += x.value.length; n++; }
    const t1 = performance.now();
    const h = new Sha256(); for (const c of chunks) h.update(c); const d1 = toHex(h.digest());
    const t2 = performance.now();
    const buf = new Uint8Array(total); let at = 0; for (const c of chunks) { buf.set(c, at); at += c.length; }
    const t3 = performance.now();
    const d2 = toHex(new Uint8Array(await crypto.subtle.digest('SHA-256', buf)));
    const t4 = performance.now();
    const e = performance.getEntriesByType('resource').filter((x) => x.name.endsWith(f)).pop();
    out.push({ r, f: f.split('/').pop(), bytes: total, chunks: n, headersMs: hdr, readMs: t1 - t0 - hdr, shaJsMs: t2 - t1, copyMs: t3 - t2, subtleMs: t4 - t3, same: d1 === d2, delivery: e && e.deliveryType, transfer: e && e.transferSize });
  }
  return out;
})()`;

async function run() {
  if (which === 'chrome') {
    const profile = path.join(WORK, 'split-profile-chrome');
    fs.rmSync(profile, { recursive: true, force: true });
    const chrome = await launchChrome({ profile, extraArgs: ['--no-proxy-server', '--disable-background-networking'] });
    const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    try {
      const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
      const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
      await cdp.send('Page.enable', {}, sessionId); await cdp.send('Runtime.enable', {}, sessionId);
      await cdp.send('Page.navigate', { url: base + '/p.html' }, sessionId);
      await sleep(800);
      const r = await cdp.send('Runtime.evaluate', { expression: `(async () => { ${script.replace(/^return /, 'return await ')} })()`, awaitPromise: true, returnByValue: true }, sessionId);
      if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 500));
      return { version: chrome.version.Browser, rows: r.result.value };
    } finally { await shutdown(chrome, cdp); }
  }
  const work = path.join(WORK, 'ff-split-work'); fs.mkdirSync(work, { recursive: true });
  const ff = await openFirefox({ work, downloadDir: path.join(work, 'dl'), dataDir: work, scriptTimeoutMs: 300000 });
  try {
    await ff.goto(base + '/p.html'); await sleep(800);
    const s = await ff.ev(`(async () => { ${script.replace(/^return /, 'return await ')} })()`);
    return { version: ff.version, rows: s };
  } finally { await ff.close(); }
}
const res = await run();
const med = (a) => a.slice().sort((x, y) => x - y)[Math.floor(a.length / 2)];
const lines = [`## ${which} ${res.version}: the cost of reading an asset (probe server on loopback, production headers, brotli stored; ${rounds} rounds; round 0 is a cold cache, later rounds are served from the HTTP cache); load ${fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]}`, '',
  '| asset | round | bytes (decoded) | chunks | fetch→headers ms | read all chunks ms | JS SHA-256 ms | one buffer copy ms | WebCrypto digest ms | digests equal | delivery / transfer |', '|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|'];
for (const r of res.rows) lines.push(`| ${r.f} | ${r.r} | ${r.bytes} | ${r.chunks} | ${r.headersMs.toFixed(0)} | ${r.readMs.toFixed(0)} | ${r.shaJsMs.toFixed(0)} | ${r.copyMs.toFixed(0)} | ${r.subtleMs.toFixed(0)} | ${r.same} | ${r.delivery || '-'} / ${r.transfer} |`);
const warm = res.rows.filter((r) => r.r > 0);
const sum = (k) => files.map((f) => med(warm.filter((r) => f.endsWith(r.f)).map((r) => r[k]))).reduce((a, b) => a + b, 0);
lines.push('', `Sum over the four assets of the warm-round medians: read ${sum('readMs').toFixed(0)} ms, JS SHA-256 ${sum('shaJsMs').toFixed(0)} ms, copy ${sum('copyMs').toFixed(0)} ms, WebCrypto digest ${sum('subtleMs').toFixed(0)} ms.`);
console.log(lines.join('\n'));
const o = opt('out', '');
if (o) fs.appendFileSync(o, lines.join('\n') + '\n\n');
