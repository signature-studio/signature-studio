// Smoke test of the file manager on the page with the REAL engine running (decision 0216, opfs1): the default entrance (no parameters),
// the engine is waited for until idle, then the Files window is opened over it, used (a folder, an upload), closed; the engine must still be idle and the page's own state unchanged.
// usage: FL_ASSETS_DIR=<dir with manifest.json> node test/e2e/files-with-engine.mjs [--no-build]
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { run, start, freePort } from '../helpers/proc.mjs';
import { APP, workDirOf } from '../helpers/wrangler.mjs';
import { openBrowser } from './browser.mjs';

const assets = process.env.FL_ASSETS_DIR;
if (!assets || !fs.existsSync(path.join(assets, 'manifest.json'))) { console.log('FL_ASSETS_DIR with manifest.json is required'); process.exit(2); }
const work = workDirOf('files-engine');
const bin = (n) => path.join(APP, 'node_modules/.bin', n);
if (!process.argv.includes('--no-build')) await run(bin('vite'), ['build'], { cwd: APP, env: process.env, label: 'vite build' });
const port = await freePort();
const server = start(bin('vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env, FL_PERF_LOG: path.join(work, 'perf.jsonl') }, ready: /Local:\s+http/, label: 'vite preview' });
await server.ready;
const results = [];
const check = (n, ok, d = '') => { results.push({ name: n, ok: !!ok, detail: d }); console.log(`${ok ? 'PASS' : 'FAIL'}  ${n}${d ? '  -- ' + String(d).slice(0, 300) : ''}`); };
const b = await openBrowser(work, 'engine');
const p = await b.newPage({ width: 1930, height: 1092 });
const t0 = Date.now();
await p.goto(`http://127.0.0.1:${port}/`);
try {
  await p.waitFor(`!!window.__app && !!window.__files && window.__app.engineStatus().state === 'idle'`, 240000, 'engine idle');
  const tIdle = Date.now() - t0;
  const before = await p.ev(`JSON.stringify({ title: document.title, kind: window.__app.engineKind || null, mem: window.__app.engineMemoryBytes, status: window.__app.engineStatus().state })`);
  console.log(`INFO  engine idle after ${tIdle} ms wall (load ${os.loadavg()[0].toFixed(1)}; not a P1 value); ${before}`);
  check('the real engine has reached its first idle and is running (idle or computing, not stopped)', ['idle', 'computing'].includes(JSON.parse(before).status), before);
  const idles0 = await p.ev('window.__app.idles.length');
  await p.ev(`document.getElementById('files-btn').click(); true`);
  await p.waitFor(`!!window.__files.explorer && document.querySelector('#fx-list') && !document.getElementById('files-overlay').hidden`, 20000);
  check('the Files window opens over the running engine', await p.ev(`document.getElementById('files-overlay').getBoundingClientRect().width === document.documentElement.clientWidth`));
  await p.ev(`(async () => { const ex = window.__files.explorer; await ex.fs.mkdir('/FromEngineTest'); await ex.fs.write('/FromEngineTest/a.txt', new Blob(['x'])); await ex.navigate('/FromEngineTest'); return true; })()`);
  check('it lists what is in the OPFS while the engine runs', JSON.stringify(await p.ev(`[...document.querySelectorAll('#fx-list .fx-row[data-name]')].map((r) => r.dataset.name)`)) === '["a.txt"]');
  // keys typed in the window must not reach the engine
  const kb = await p.ev(`window.__app.inputEvents`);
  await p.S('Input.dispatchKeyEvent', { type: 'keyDown', key: 'a', text: 'a', code: 'KeyA' }); await p.S('Input.dispatchKeyEvent', { type: 'keyUp', key: 'a', code: 'KeyA' });
  await p.ev(`document.getElementById('fx-list').focus(); true`);
  await p.S('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Backspace', code: 'Backspace' }); await p.S('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Backspace', code: 'Backspace' });
  check('keys typed while the window is open do not become engine input', (await p.ev(`window.__app.inputEvents`)) === kb, `${kb} -> ${await p.ev(`window.__app.inputEvents`)}`);
  await p.ev(`document.getElementById('fx-close').click(); true`);
  check('closing returns the keyboard to the desktop', await p.ev(`document.activeElement === document.getElementById('desktop')`));
  await p.waitFor(`window.__app.idles.length > ${idles0} + 3`, 60000, 'engine keeps idling');
  const after = JSON.parse(await p.ev(`JSON.stringify({ title: document.title, status: window.__app.engineStatus().state, exit: !!document.getElementById('exit') && !document.getElementById('exit').hidden })`));
  check('the engine is still running (idle or computing, not stopped) after the window was used, idled 3+ more times, and the page title is the engine\'s', ['idle', 'computing'].includes(after.status) && !after.exit && /FL Studio/.test(after.title), JSON.stringify(after));
  await p.ev(`(async () => { const d = await navigator.storage.getDirectory(); await d.removeEntry('FromEngineTest', { recursive: true }); return true; })()`);
} catch (e) { check('ran to its end', false, e.stack || e); }
await b.close();
await server.stop();
const bad = results.filter((r) => !r.ok);
fs.writeFileSync(path.join(work, 'results.json'), JSON.stringify(results, null, 1));
console.log(`${results.length - bad.length}/${results.length} passed`);
process.exit(bad.length ? 1 : 0);
