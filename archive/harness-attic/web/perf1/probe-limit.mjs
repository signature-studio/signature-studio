// perf1 (decision 0130, PROVISIONAL): the size limit of ONE wasm module in Chrome (the V8 constant kV8MaxWasmModuleSize = 1 GiB, wasm-limits.h): a synthetic module made of the 8-byte header and ONE custom section (id 0, name "pad", zeros),
// total size N, compiled with WebAssembly.compile in a Worker-less page of an isolated Chrome (the isolation of run-chrome-perf.mjs). usage: node probe-limit.mjs OUT.json [sizes...]  (default: 1073741823 1073741824 1073741825)
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { launchChrome, Cdp, shutdown, sleep } from '../cdp.mjs';
import { isolationArgs, withTimeout } from '../present/lib.mjs';
const HERE = path.dirname(fileURLToPath(import.meta.url));
const outFile = process.argv[2];
const sizes = process.argv.length > 3 ? process.argv.slice(3).map(Number) : [1073741823, 1073741824, 1073741825];
const WORK = process.env.PRESENT_WORK || '/tmp/opencode/perf1/chrome';
fs.mkdirSync(WORK, { recursive: true });
const server = http.createServer((req, res) => { res.writeHead(200, { 'Content-Type': 'text/html' }); res.end('<!doctype html><title>limit</title>'); });
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const port = server.address().port;
const profile = path.join(WORK, 'profile-limit'), netlog = path.join(WORK, 'netlog-limit.json');
fs.rmSync(profile, { recursive: true, force: true });
const chrome = await launchChrome({ profile, extraArgs: isolationArgs(profile, netlog) });
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
const S = (method, params, ms = 600000) => withTimeout(cdp.send(method, params, sessionId), ms, 'CDP ' + method);
await S('Runtime.enable'); await S('Page.enable');
await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
await sleep(800);
const out = { chrome: chrome.version.Browser, v8: chrome.version['V8-Version'], results: [] };
for (const n of sizes) {
  const expr = `(async () => {
    const N = ${n};
    const name = new TextEncoder().encode('pad');
    // section payload = name length (1 byte LEB) + name + zeros; the section size LEB is 5 bytes (a padded LEB128 is valid); total = 8 + 1 (id) + 5 (size LEB) + payload
    const payload = N - 8 - 1 - 5;
    const b = new Uint8Array(N);
    b.set([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0], 0);
    b[8] = 0;
    let v = payload; for (let i = 0; i < 5; i++) { b[9 + i] = (v & 0x7f) | (i < 4 ? 0x80 : 0); v = Math.floor(v / 128); }
    b[14] = 3; b.set(name, 15);
    const t = performance.now();
    try { const m = await WebAssembly.compile(b); return { n: N, ok: true, ms: +(performance.now() - t).toFixed(1), sections: WebAssembly.Module.customSections(m, 'pad').length }; }
    catch (e) { return { n: N, ok: false, ms: +(performance.now() - t).toFixed(1), error: String(e).slice(0, 240) }; }
  })()`;
  try { const r = await S('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true }); out.results.push(r.exceptionDetails ? { n, error: JSON.stringify(r.exceptionDetails).slice(0, 300) } : r.result.value); }
  catch (e) { out.results.push({ n, driverError: String(e).slice(0, 200) }); }
}
await shutdown(chrome, cdp);
server.closeAllConnections(); server.close();
await sleep(400);
const nl = spawnSync('node', [path.join(HERE, '../threads/check_netlog.mjs'), netlog, '127.0.0.1', 'tolerate-ipv6-probe'], { encoding: 'utf8' });
out.netlog = { summary: (nl.stdout || nl.stderr || '').trim(), status: nl.status };
fs.rmSync(profile, { recursive: true, force: true }); fs.rmSync(netlog, { force: true });
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));
console.log(JSON.stringify(out, null, 1));
