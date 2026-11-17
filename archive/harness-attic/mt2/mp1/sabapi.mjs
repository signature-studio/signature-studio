// mt2: which browser APIs that the page applies to views of the engine's memory refuse a view of a SharedArrayBuffer? (chrome|firefox)
import http from 'node:http';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
const APP = '/home/ubuntu/work/signature-studio-develop/lift/out-mt2/src/app';
const which = process.argv[2] || 'chrome';
const srv = http.createServer((req, res) => { res.writeHead(200, { 'Content-Type': 'text/html', 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp' }); res.end('<!doctype html><title>x</title>'); });
await new Promise((r) => srv.listen(0, '127.0.0.1', r));
const url = `http://127.0.0.1:${srv.address().port}/`;
const probe = `(() => { const out = {}; const t = (n, f) => { try { const v = f(); out[n] = 'ok' + (v === undefined ? '' : ' ' + v); } catch (e) { out[n] = 'THROWS ' + e.name + ': ' + String(e.message).slice(0, 70); } };
 const sab = new SharedArrayBuffer(64), u8 = new Uint8Array(sab), c8 = new Uint8ClampedArray(sab), ab = new ArrayBuffer(64);
 t('isolated', () => self.crossOriginIsolated);
 t('new ImageData(Uint8ClampedArray over SAB)', () => new ImageData(c8, 4, 4).width);
 t('new ImageData(Uint8ClampedArray over ArrayBuffer)', () => new ImageData(new Uint8ClampedArray(ab), 4, 4).width);
 t('TextDecoder.decode(view over SAB)', () => new TextDecoder().decode(u8.subarray(0, 4)).length);
 t('TextDecoder.decode(view over SAB).slice copy', () => new TextDecoder().decode(u8.slice(0, 4)).length);
 t('canvas.putImageData(ImageData made from a copy)', () => { const c = document.createElement('canvas'); c.width = c.height = 4; c.getContext('2d').putImageData(new ImageData(c8.slice(), 4, 4), 0, 0); });
 t('createImageBitmap(ImageData copy) typeof', () => typeof createImageBitmap);
 t('crypto.subtle.digest(view over SAB)', () => { crypto.subtle.digest('SHA-256', u8).catch(() => 0); return 'async'; });
 t('postMessage(Uint8Array over SAB) (shares)', () => { const mc = new MessageChannel(); mc.port1.postMessage(u8); mc.port1.close(); });
 t('Blob([view over SAB])', () => new Blob([u8]).size);
 t('Atomics.waitAsync', () => typeof Atomics.waitAsync);
 t('WebAssembly.Memory shared', () => new WebAssembly.Memory({ initial: 1, maximum: 2, shared: true }).buffer instanceof SharedArrayBuffer);
 return out; })()`;
let res;
if (which === 'chrome') {
  const { openBrowser } = await import(pathToFileURL(path.join(APP, 'test/e2e/browser.mjs')).href);
  const b = await openBrowser('/home/ubuntu/work/signature-studio-develop/lift/out-mt2/runs/sabapi-chrome', 'sab');
  const p = await b.newPage({ width: 400, height: 300 });
  await p.goto(url);
  res = await p.ev(probe);
  await b.close?.();
} else {
  const { openFirefox } = await import(pathToFileURL(path.join(APP, 'test/e2e/firefox-driver.mjs')).href);
  const w = '/home/ubuntu/work/signature-studio-develop/lift/out-mt2/runs/sabapi-firefox';
  const ff = await openFirefox({ work: w, downloadDir: w + '/dl', dataDir: w, scriptTimeoutMs: 60000 });
  await ff.goto(url);
  res = await ff.ev(probe);
  res.__version = ff.version;
  await ff.close();
}
console.log(JSON.stringify({ browser: which, ...res }, null, 1));
srv.close(); process.exit(0);
