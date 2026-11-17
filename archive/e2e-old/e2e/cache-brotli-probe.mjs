// Decision 0220, job 2: store the DECODED bytes or the brotli bytes? Costs of the second choice in this browser: is DecompressionStream('brotli') there, and how long do 4 assets take to decode from a Blob (the stored copy)
// compared with reading the decoded bytes of the same assets back (File.arrayBuffer())? The bytes come from the probe server (the same files the bundle serves).
// usage: node test/e2e/cache-brotli-probe.mjs --browser chrome|firefox --port 8802 [--out FILE]
import fs from 'node:fs';
import path from 'node:path';
import { Cdp, launchChrome, shutdown, sleep } from './cdp.mjs';
import { openFirefox } from './firefox-driver.mjs';
const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const which = opt('browser', 'chrome'), port = Number(opt('port', '8802'));
const base = `http://127.0.0.1:${port}`;
const WORK = '/home/ubuntu/work/signature-studio-develop/lift/out-cache1/work';
const FILES = ['fl-engine-ship2-F2xH2/engine.f4a57e84a5e1.wasm', 'fl-engine-ship2-F2xH2/dll.50206405ba5f.flpack', 'fl-engine-ship2-F2xH2/seed.0f4b547e6656.flpack', 'fl-engine-ship2-F2xH2/engine-glue.b5952d3e5165.js'];
const code = `(async () => {
  const out = { brotliSupported: false, rows: [] };
  try { new DecompressionStream('brotli'); out.brotliSupported = true; } catch (e) { out.brotliError = String(e).slice(0, 120); }
  const root = await navigator.storage.getDirectory();
  for (const f of ${JSON.stringify(FILES)}) {
    // the browser decodes the response (Content-Encoding br): these are the decoded bytes
    const dec = new Uint8Array(await (await fetch('/a/' + f)).arrayBuffer());
    // the stored (still brotli) bytes: ask for them as a plain file (.br is served as is, without Content-Encoding)
    const enc = new Uint8Array(await (await fetch('/a/' + f + '.br')).arrayBuffer());
    const h1 = await root.getFileHandle('d-' + f.split('/').pop(), { create: true }); let w = await h1.createWritable(); await w.write(dec); await w.close();
    const h2 = await root.getFileHandle('e-' + f.split('/').pop(), { create: true }); w = await h2.createWritable(); await w.write(enc); await w.close();
    const row = { f: f.split('/').pop(), decoded: dec.length, brotli: enc.length };
    let t = performance.now(); const b1 = new Uint8Array(await (await h1.getFile()).arrayBuffer()); row.readDecodedMs = Math.round(performance.now() - t);
    t = performance.now(); const b2 = new Uint8Array(await (await h2.getFile()).arrayBuffer()); row.readBrotliMs = Math.round(performance.now() - t);
    if (out.brotliSupported) {
      t = performance.now();
      const ds = (await h2.getFile()).stream().pipeThrough(new DecompressionStream('brotli'));
      const r = ds.getReader(); let n = 0; const parts = []; for (;;) { const x = await r.read(); if (x.done) break; parts.push(x.value); n += x.value.length; }
      row.decodeFromFileMs = Math.round(performance.now() - t); row.decodedOk = n === dec.length;
    }
    out.rows.push(row);
    await root.removeEntry('d-' + f.split('/').pop()); await root.removeEntry('e-' + f.split('/').pop());
  }
  return out;
})()`;
let version, res;
if (which === 'chrome') {
  const profile = path.join(WORK, 'brotli-profile-chrome'); fs.rmSync(profile, { recursive: true, force: true });
  const chrome = await launchChrome({ profile, extraArgs: ['--no-proxy-server'] }); version = chrome.version.Browser;
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl); await cdp.opened;
  try {
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    await cdp.send('Page.enable', {}, sessionId); await cdp.send('Runtime.enable', {}, sessionId);
    await cdp.send('Page.navigate', { url: base + '/p.html' }, sessionId); await sleep(500);
    const r = await cdp.send('Runtime.evaluate', { expression: code, awaitPromise: true, returnByValue: true }, sessionId);
    if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 400));
    res = r.result.value;
  } finally { await shutdown(chrome, cdp); }
} else {
  const work = path.join(WORK, 'ff-brotli-work'); fs.mkdirSync(work, { recursive: true });
  const ff = await openFirefox({ work, downloadDir: path.join(work, 'dl'), dataDir: work, scriptTimeoutMs: 300000 }); version = ff.version;
  try { await ff.goto(base + '/p.html'); await sleep(500); res = await ff.ev(code); } finally { await ff.close(); }
}
const L = [`## ${which} ${version}: DecompressionStream('brotli') ${res.brotliSupported ? 'is available' : 'is NOT available' + (res.brotliError ? ' (' + res.brotliError + ')' : '')}; load ${fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]}`, '', '| asset | decoded bytes | brotli bytes | read decoded copy ms | read brotli copy ms | decode brotli copy (DecompressionStream) ms |', '|---|---:|---:|---:|---:|---:|'];
for (const r of res.rows) L.push(`| ${r.f} | ${r.decoded} | ${r.brotli} | ${r.readDecodedMs} | ${r.readBrotliMs} | ${r.decodeFromFileMs ?? '-'}${r.decodedOk === false ? ' (WRONG LENGTH)' : ''} |`);
console.log(L.join('\n'));
if (opt('out', '')) fs.appendFileSync(opt('out'), L.join('\n') + '\n\n');
