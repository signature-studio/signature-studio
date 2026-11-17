// Decision 0216 stage 2 (opfs1): does the REAL engine run in Firefox (headless, geckodriver) and does its File > Open reach the OPFS selection window there? The client's browser is Firefox 159.
// THE SIMULATED PARTS: the clicks and keys on the engine's menus are WebDriver pointer/key actions (trusted events made by the browser, standing for a person's hands); the choice in the window is DOM clicks.
// usage: APP_WORK=<dir under $HOME> FL_ASSETS_DIR=<dir with manifest.json> node test/e2e/files-engine-firefox.mjs --project DIR [--no-build]
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { run, start, freePort, sleep } from '../helpers/proc.mjs';
import { APP, workDirOf } from '../helpers/wrangler.mjs';
import { openFirefox } from './firefox-driver.mjs';
import { parseDeskWindows, isWizard } from './desk-helpers.mjs';   // decisions 0223 / desk2: the compositor's window list and the foreground title, no window list with classes

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const assets = process.env.FL_ASSETS_DIR;
if (!assets || !fs.existsSync(path.join(assets, 'manifest.json'))) { console.log('FL_ASSETS_DIR with manifest.json is required'); process.exit(2); }
const work = workDirOf('files-engine-firefox');
const projectDir = path.resolve(opt('project', ''));
if (!fs.existsSync(path.join(projectDir, 'Sampler.flp'))) { console.log('--project DIR with Sampler.flp is required'); process.exit(2); }
const bin = (n) => path.join(APP, 'node_modules/.bin', n);
if (!argv.includes('--no-build')) await run(bin('vite'), ['build'], { cwd: APP, env: process.env, label: 'vite build' });
const port = await freePort();
const srv = start(bin('vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env, FL_ASSETS_DIR: assets, FL_PERF_LOG: path.join(work, 'perf.jsonl') }, ready: /Local:\s+http/, label: 'vite preview' });
await srv.ready;
const results = [];
const check = (n, ok, d = '') => { results.push({ name: n, ok: !!ok, detail: d }); console.log(`${ok ? 'PASS' : 'FAIL'}  ${n}${d ? '  -- ' + String(d).slice(0, 400) : ''}`); };
const ff = await openFirefox({ work, downloadDir: path.join(work, 'downloads'), dataDir: work });
console.log(`INFO  Firefox ${ff.version} load ${os.loadavg()[0].toFixed(1)}`);
const S = (m, p, b) => fetch(`${ff.base}/session/${ff.sid}${p}`, { method: m, headers: { 'content-type': 'application/json' }, body: b === undefined ? undefined : JSON.stringify(b) }).then((r) => r.json());
const click = async (cx, cy) => S('POST', '/actions', { actions: [{ type: 'pointer', id: 'mouse', parameters: { pointerType: 'mouse' }, actions: [{ type: 'pointerMove', duration: 0, origin: 'viewport', x: Math.round(cx), y: Math.round(cy) }, { type: 'pointerDown', button: 0 }, { type: 'pause', duration: 60 }, { type: 'pointerUp', button: 0 }] }] });
const key = async (k) => S('POST', '/actions', { actions: [{ type: 'key', id: 'kb', actions: [{ type: 'keyDown', value: k }, { type: 'keyUp', value: k }] }] });
const windows = async () => parseDeskWindows(await ff.ev('(async () => (await window.__app.deskDumpBase64(true)).windows)()'));
const title = () => ff.ev('document.title');   // the title of the engine's foreground window: its main window's when no other window of the engine is in front
const wizardUp = async () => (await windows()).some(isWizard) || /^Welcome to FL Studio/.test(await title());
let t0 = Date.now();
try {
  await S('POST', '/window/rect', { width: 2100, height: 1400, x: 0, y: 0 });
  await ff.goto(`http://127.0.0.1:${port}/`);
  await ff.waitFor("!!window.__app && !!window.__files", 60000);
  let idle = false;
  for (let i = 0; i < 1800 && !idle; i++) { const st = await ff.ev("window.__app.state + '|' + (window.__app.error ? JSON.stringify(window.__app.error).slice(0, 300) : '')").catch(() => ''); idle = /^engine-idle/.test(st) || /^engine-computing/.test(st); if (/^(error|exited|stopped)/.test(st) || /\|\{/.test(st)) { console.log('INFO  state', st); break; } if (!idle) await sleep(200); }
  console.log(`INFO  after ${((Date.now() - t0) / 1000).toFixed(1)} s (wall): state ${await ff.ev('window.__app.state')}; engineKind ${await ff.ev('window.__app.engineKind')}; features ${await ff.ev("JSON.stringify(window.__app.features ?? null)").catch(() => 'n/a')}`);
  check('the real engine runs in Firefox (first idle reached; emscripten-fruity-v1)', idle && (await ff.ev('window.__app.engineKind')) === 'emscripten-fruity-v1', await ff.ev('window.__app.state'));
  if (idle) {
    // wait for the first window and dismiss the wizard like the Chrome run does
    for (let i = 0; i < 400 && !(await windows()).length; i++) await sleep(250);
    const rect = JSON.parse(await ff.ev("JSON.stringify(document.getElementById('desktop').getBoundingClientRect().toJSON())"));
    console.log('INFO  desktop rect', JSON.stringify(rect), 'windows', JSON.stringify((await windows()).map((w) => [w.lid, w.x, w.y, w.w, w.h])));
    const P = (x, y) => [rect.x + x, rect.y + y];
    if (await wizardUp()) { await click(...P(1036, 561)); await sleep(3000); await key('\uE007'); await sleep(3000); }
    // seed the OPFS with the project through the page (File objects built from the bytes)
    const flp = fs.readFileSync(path.join(projectDir, 'Sampler.flp')).toString('base64');
    const wav = fs.readFileSync(path.join(projectDir, 'SHP_Sawing Synth.wav')).toString('base64');
    await ff.ev(`(async () => { const root = await navigator.storage.getDirectory(); for await (const [n] of root.entries()) await root.removeEntry(n, { recursive: true }); const d = await root.getDirectoryHandle('Sampler-project', { create: true }); const put = async (name, b64) => { const w = await (await d.getFileHandle(name, { create: true })).createWritable(); await w.write(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))); await w.close(); }; await put('Sampler.flp', ${JSON.stringify(flp)}); await put('SHP_Sawing Synth.wav', ${JSON.stringify(wav)}); return true; })()`);
    await click(...P(18, 20)); await sleep(1500);
    await click(...P(48, 85));
    const asked = await ff.waitFor("window.__app.fileDlg.open === true && !!document.querySelector('#filedlg #fx-list')", 60000).then(() => true).catch(() => false);
    check('File > Open of the engine reaches the page in Firefox and the OPFS selection window is the dialog', asked && (await ff.ev("document.getElementById('filedlg').dataset.select === '1'")));
    if (asked) {
      await ff.ev("(async () => { for (let i = 0; i < 200 && !window.__files.engine.explorer; i++) await new Promise((r) => setTimeout(r, 50)); return true; })()");
      const filters = JSON.parse(await ff.ev("JSON.stringify([...document.querySelectorAll('#filedlg #fx-filter option')].map((o) => [o.textContent, o.selected]))"));
      check('the six file types of the engine, "Supported song files" selected', filters.length === 6 && filters[5][1] === true, JSON.stringify(filters).slice(0, 200));
      await ff.ev("(async () => { await window.__files.engine.explorer.navigate('/Sampler-project'); const r = [...document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]')].find((x) => x.dataset.name === 'Sampler.flp'); r.dispatchEvent(new MouseEvent('click', { bubbles: true })); document.querySelector('#filedlg #fx-ok').click(); return true; })()");
      let tt = null;
      const tw = Date.now();
      while (Date.now() - tw < 120000) { tt = await title(); if (tt === 'Sampler.flp - FL Studio 2026') break; await sleep(300); }
      check("the engine opened the project chosen in the window: its main window title becomes 'Sampler.flp - FL Studio 2026'", tt === 'Sampler.flp - FL Studio 2026', `title ${JSON.stringify(tt)} after ${Date.now() - tw} ms`);
      const ans = await ff.ev('JSON.stringify(window.__files.engine.answers)');
      check('the answer: the project (R) and the file next to it (E)', /"R".*Sampler-project\\\\Sampler.flp/.test(ans) && /"E"/.test(ans), ans.slice(0, 300));
    }
  }
} catch (e) { check('ran to its end', false, e.stack || e); }
console.log('INFO  final state', await ff.ev('window.__app.state').catch(() => null));
await ff.close();
await srv.stop();
fs.writeFileSync(path.join(work, 'results.json'), JSON.stringify(results, null, 1));
const bad = results.filter((r) => !r.ok);
console.log(`${results.length - bad.length}/${results.length} passed`);
process.exit(bad.length ? 1 : 0);
