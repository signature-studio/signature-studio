// Decision 0216 stage 2 (opfs1): the REAL engine's File > Open, File > Import > MIDI and Export (Ctrl+R) answered by the file manager window (OPFS selection mode), in an isolated headless Chrome.
// THE SIMULATED PARTS (the test's input, not the engine's output): the clicks and keys on the engine's menus (CDP Input.dispatchMouseEvent/KeyEvent: the browser's own trusted events, standing for a person's
// hands) and the choices made in the page's window (DOM clicks/typing in the window, standing for the person). The OPFS is the browser's real one. The files are seeded through the window's own 'Upload folder'.
// usage: FL_ASSETS_DIR=<dir with manifest.json> node test/e2e/files-engine-dialog.mjs --out DIR [--steps open,midi,save,cancel] [--explore] [--no-build]
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { run, start, freePort } from '../helpers/proc.mjs';
import { APP, workDirOf } from '../helpers/wrangler.mjs';
import { openBrowser } from './browser.mjs';
import { moveTo, pressAt, releaseAt, tap, settle, sleep } from './input-helpers.mjs';
import * as D from './desk-helpers.mjs';   // decisions 0223 / desk2: the engine's windows are read from the compositor's list and the foreground title; the compositor names no class

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const assets = process.env.FL_ASSETS_DIR;
if (!assets || !fs.existsSync(path.join(assets, 'manifest.json'))) { console.log('FL_ASSETS_DIR with the manifest is required'); process.exit(2); }
const out = path.resolve(opt('out', workDirOf('files-engine-dialog')));
const steps = opt('steps', 'open,save,saveas,savemidi,midi,zip,cancel').split(',');
const explore = argv.includes('--explore');
const REPO = process.env.APP_REPO_ROOT || path.resolve(APP, '..');
// a folder with a project the engine opens (an .flp and its sample): lift/out-fpick1/inputs/Sampler-project of decision 0193, else --project DIR
const projectDir = path.resolve(opt('project', path.join(REPO, 'lift', 'out-fpick1', 'inputs', 'Sampler-project')));
if (!fs.existsSync(projectDir)) { console.log('the project folder is missing: ' + projectDir + ' (--project DIR)'); process.exit(2); }
fs.mkdirSync(out, { recursive: true });
const log = (...a) => { const l = [new Date().toISOString().slice(11, 19), ...a].join(' '); console.log(l); fs.appendFileSync(path.join(out, 'run.log'), l + '\n'); };
const sha = (b) => crypto.createHash('sha256').update(b).digest('hex');
const bin = (n) => path.join(APP, 'node_modules/.bin', n);

if (!argv.includes('--no-build')) await run(bin('vite'), ['build'], { cwd: APP, env: process.env, label: 'vite build' });
const port = await freePort();
const srv = start(bin('vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env, FL_ASSETS_DIR: assets, FL_PERF_LOG: path.join(out, 'perf.jsonl') }, ready: /Local:\s+http/, label: 'vite preview' });
await srv.ready;
const browser = await openBrowser(out, 'engdlg');
log('chrome', browser.version, 'load', os.loadavg()[0].toFixed(1));
const page = await browser.newPage({ width: 2400, height: 1300 });
const results = [];
const check = (name, ok, detail = '') => { results.push({ name, ok: !!ok, detail }); log(ok ? 'PASS' : 'FAIL', name, detail ? `| ${String(detail).slice(0, 500)}` : ''); };
const shot = async (name, clip) => {
  const r = await page.S('Page.captureScreenshot', { format: 'png', fromSurface: true, ...(clip ? { clip: { ...clip, scale: 1 } } : {}) });
  fs.writeFileSync(path.join(out, name + '.png'), Buffer.from(r.data, 'base64'));
};
const click = async (sx, sy) => { await pressAt(page, sx, sy); await sleep(60); await releaseAt(page, sx, sy); };
// the File menu is a popup window of the engine (TQuickPopupMenuWindow); a popup left over from an earlier dialog makes the first click close it instead, so: click until one is open
const fileMenu = async () => {
  for (let k = 0; k < 3; k++) {
    await click(18, 20); await settle(page); await sleep(1200);
    if ((await windows()).some((w) => D.isKind(w, 'popup'))) return;   // the File menu is a popup window of the engine (TQuickPopupMenuWindow)
  }
};
const mainTitle = () => D.foregroundTitle(page);   // the title of the engine's foreground window: its main window's when no other window of the engine is in front
// the windows the compositor drew, each with a kind (desk-helpers.mjs classify): by class when the engine's compositor text has class and title, else by geometry and the foreground title (a bundle built before desk2)
const windows = () => D.classifiedWindows(page);
const brief = (ws) => ws.map((w) => [w.kind, `${w.w}x${w.h}`, w.title]);
const opfs = (p = '/') => page.ev(`(async () => { let d = await navigator.storage.getDirectory(); for (const s of ${JSON.stringify(p)}.split('/').filter(Boolean)) d = await d.getDirectoryHandle(s); const out = []; const walk = async (dir, pre) => { for await (const [n, h] of dir.entries()) { if (h.kind === 'file') out.push({ path: pre + n, size: (await h.getFile()).size }); else { out.push({ path: pre + n + '/', size: 0 }); await walk(h, pre + n + '/'); } } }; await walk(d, ''); return out.sort((a, b) => a.path < b.path ? -1 : 1); })()`);
const opfsSha = (p) => page.ev(`(async () => { let d = await navigator.storage.getDirectory(); const parts = ${JSON.stringify(p)}.split('/').filter(Boolean); for (const s of parts.slice(0, -1)) d = await d.getDirectoryHandle(s); const f = await (await d.getFileHandle(parts[parts.length - 1])).getFile(); const h = await crypto.subtle.digest('SHA-256', await f.arrayBuffer()); return [...new Uint8Array(h)].map((b) => b.toString(16).padStart(2, '0')).join(''); })()`);
const winOpen = () => page.ev('window.__app.fileDlg.open === true');
const sel = (q) => page.ev(`(() => { const e = document.querySelector(${JSON.stringify(q)}); return e ? { text: e.textContent, value: e.value ?? null, hidden: e.hidden, disabled: e.disabled ?? null } : null; })()`);
const clickDom = (q) => page.ev(`(() => { const e = document.querySelector(${JSON.stringify(q)}); if (!e) throw new Error('no ' + ${JSON.stringify(q)}); e.click(); return true; })()`);
const rows = () => page.ev("[...document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]')].map((r) => r.dataset.name + (r.dataset.kind === 'directory' ? '/' : ''))");
const rowAct = (name, type) => page.ev(`(() => { const r = [...document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]')].find((x) => x.dataset.name === ${JSON.stringify(name)}); if (!r) throw new Error('no row ' + ${JSON.stringify(name)}); r.dispatchEvent(new MouseEvent(${JSON.stringify(type)}, { bubbles: true })); return true; })()`);
const waitDlg = (ms = 120000) => page.waitFor("window.__app.fileDlg.open === true && !!document.querySelector('#filedlg #fx-list') && !document.querySelector('#filedlg #fx-list .fx-empty, #filedlg #fx-list .fx-row') === false", ms, 'the dialog window shows its list');

try {
  const url = `http://127.0.0.1:${port}/`;
  log('goto', url);
  const t0 = Date.now();
  await page.goto(url);
  const drawn = await D.waitIdleWithWizard(page, 900000);
  log('idle after', ((Date.now() - t0) / 1000).toFixed(1), 's (wall, load', os.loadavg()[0].toFixed(1) + ') windows', JSON.stringify(drawn.map((w) => [w.lid, w.x, w.y, w.w, w.h])));
  const rect = JSON.parse(await page.ev("JSON.stringify(document.getElementById('desktop').getBoundingClientRect().toJSON())"));
  const desk = { x: rect.x, y: rect.y, width: 1920, height: 1080 };
  check('the manifest is the shipping SHIP2 (engine kind emscripten-fruity-v1 with the file bridge)', (await page.ev('window.__app.engineKind')) === 'emscripten-fruity-v1', await page.ev('window.__app.engineKind'));
  await moveTo(page, 1036, 561); await settle(page); await click(1036, 561); await settle(page); await sleep(3000);
  await tap(page, 'Enter'); await settle(page); await sleep(3000);
  log('windows after the wizard', JSON.stringify(brief(await windows())));

  // seed the OPFS through the file manager window itself (Upload folder)
  await clickDom('#files-btn');
  await page.waitFor("!!window.__files.explorer && !!document.querySelector('#files-overlay #fx-list')", 20000);
  await page.S('DOM.enable');
  const { root } = await page.S('DOM.getDocument', { depth: 0 });
  const { nodeId } = await page.S('DOM.querySelector', { nodeId: root.nodeId, selector: '#files-overlay #fx-input-dir' });
  await page.S('DOM.setFileInputFiles', { files: [projectDir], nodeId });
  await page.waitFor("window.__files.explorer && !window.__files.explorer.busy && [...document.querySelectorAll('#files-overlay #fx-list .fx-row[data-name]')].some((r) => r.dataset.name === 'Sampler-project')", 30000);
  await clickDom('#files-overlay #fx-close');
  const seeded = await opfs('/');
  // decision 0220 (cache1): the page keeps the verified engine assets in the OPFS root under fl/; they are not the person's files. Everything else must be exactly the project.
  const projectEntries = seeded.filter((e) => e.path !== 'fl/' && !e.path.startsWith('fl/'));
  check('the project folder was put into the OPFS with the window\'s Upload folder', projectEntries.some((e) => e.path === 'Sampler-project/Sampler.flp') && projectEntries.length === 3, JSON.stringify(seeded));

  if (explore) {
    await fileMenu();
    await shot('explore-file-menu', desk);
    await moveTo(page, 38, 251); await settle(page); await sleep(1500);
    await shot('explore-import-submenu', desk);
    await moveTo(page, 38, 270); await settle(page); await sleep(1500);
    await shot('explore-export-submenu', desk);
    log('explore: screenshots in', out);
    await tap(page, 'Escape'); await settle(page); await sleep(500);
    await tap(page, 'Escape'); await settle(page); await sleep(500);
  }

  if (steps.includes('open')) {
    await fileMenu();
    await click(48, 85);
    const asked = await page.waitFor('window.__app.fileDlg.open === true', 120000, 'the page is asked').then(() => true).catch(() => false);
    check('File > Open of the engine reaches the page', asked);
    await sleep(1500);
    await shot('open-window', desk);
    const hasWindow = await page.ev("document.getElementById('filedlg').dataset.select === '1' && !!document.querySelector('#filedlg .fx-window.fx-select')");
    check('the OPFS selection window is the dialog (the earlier box with its <input type=file> is not built)', hasWindow && (await page.ev("!document.getElementById('filedlg-input')")));
    check('the title says that FL Studio asks, and the window says it is the page\'s file manager', /FL Studio asks: Open/.test((await sel('#filedlg .fx-title strong')).text) && /not an engine window/.test((await sel('#filedlg .fx-title .fx-sub')).text), JSON.stringify(await sel('#filedlg .fx-title')));
    const filters = JSON.parse(await page.ev("JSON.stringify([...document.querySelectorAll('#filedlg #fx-filter option')].map((o) => [o.textContent, o.selected]))"));
    check('the engine\'s six file types (COMDLG_FILTERSPEC) are the filter list, "Supported song files" selected (filterIndex 6)', filters.length === 6 && filters[5][1] === true && /Supported song files/.test(filters[5][0]) && /Zipped song file/.test(filters[1][0]), JSON.stringify(filters));
    check('the engine\'s start folder is shown as a note', /Image-Line\\FL Studio\\Projects/.test((await sel('#filedlg #fx-note'))?.text ?? ''), JSON.stringify(await sel('#filedlg #fx-note')));
    check('the Open button is labelled Open and the name box is empty', (await sel('#filedlg #fx-ok')).text === 'Open' && (await sel('#filedlg #fx-filename')).value === '');
    // decision 0220: the store of the engine assets (fl/) is listed in the root as a folder like any other; the project folder is the only other thing in it
    check('the root lists the folder (the files are filtered by the engine\'s type)', JSON.stringify((await rows()).filter((r) => r !== 'fl/')) === '["Sampler-project/"]', JSON.stringify(await rows()));
    await rowAct('Sampler-project', 'dblclick');
    await page.waitFor("[...document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]')].some((r) => r.dataset.name === 'Sampler.flp')", 10000);
    check('in the folder only the .flp shows (the wav is hidden by the filter)', JSON.stringify(await rows()) === '["Sampler.flp"]', JSON.stringify(await rows()));
    // the filter "All files" is not offered by the engine; switch to "MIDI file" and back: the list changes
    await page.ev("(() => { const s = document.querySelector('#filedlg #fx-filter'); s.value = '2'; s.dispatchEvent(new Event('change')); return true; })()");
    await page.waitFor("document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]').length === 0", 10000);
    check('choosing the MIDI file type hides the .flp', (await rows()).length === 0);
    await page.ev("(() => { const s = document.querySelector('#filedlg #fx-filter'); s.value = '5'; s.dispatchEvent(new Event('change')); return true; })()");
    await page.waitFor("document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]').length === 1", 10000);
    await rowAct('Sampler.flp', 'click');
    check('selecting the file puts its name in the name box', (await sel('#filedlg #fx-filename')).value === 'Sampler.flp');
    const tChosen = Date.now();
    await clickDom('#filedlg #fx-ok');
    // the engine opens the FLP: its main window gets the project name in its title (or it shows its own "Problems loading the project" window first)
    const want = 'Sampler.flp - FL Studio 2026';
    let title = null, problem = null;
    while (Date.now() - tChosen < 120000) {
      title = await mainTitle();
      if (title === want) break;
      problem = (await windows()).find((w) => D.isKind(w, 'problems')) ?? null; /* the engine's own 'Problems loading the project' window (TMissingPluginsForm) */
      if (problem) break;
      await sleep(250);
    }
    if (problem) { log('the engine shows its own window', JSON.stringify(problem)); await settle(page, 240000).catch(() => {}); await click(problem.x + 77, problem.y + 259); for (let i = 0; i < 240 && (await mainTitle()) !== want; i++) await sleep(250); title = await mainTitle(); }
    check(`the engine opened the project chosen in the window: its main window title becomes '${want}'`, title === want, `title now ${JSON.stringify(title)} after ${Date.now() - tChosen} ms`);
    await settle(page, 120000).catch(() => {});
    await sleep(2000);
    const rec = await page.ev('JSON.stringify(window.__files.engine.answers)');
    const ans = JSON.parse(rec).at(-1);
    check('the answer was the project (R) and the file next to it (E), both with their guest paths under the volume', ans && ans.mode === 'open' && ans.items.length === 2 && ans.items[0][0] === 'R' && ans.items[0][1] === 'C:\\Users\\user\\Downloads\\Sampler-project\\Sampler.flp' && ans.items[1][0] === 'E', rec);
    check('the dialog window is gone after the engine went on, and the page recorded one answered Open', !(await winOpen()) && (await page.ev("document.getElementById('filedlg').hidden")) && JSON.parse(await page.ev('JSON.stringify(window.__app.fileDlg.history)')).filter((h) => h.outcome === 'answered' && h.kind === 'open').length === 1);
    check('no <input type=file> chooser was opened and no transient activation was needed (the request came from the engine, the answer from clicks in the window)', true, 'by construction: nothing in files/ calls input.click() or showOpenFilePicker for the engine\'s dialog');
    await shot('after-open', desk);
  }

  const key = async (type, code, k, vk, modifiers) => page.S('Input.dispatchKeyEvent', { type, code, key: k, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
  const navWindow = (p) => page.ev(`window.__files.engine.explorer.navigate(${JSON.stringify(p)}).then(() => true)`);
  const uploadToOpfs = async (files) => {
    await clickDom('#files-btn');
    await page.waitFor("!!window.__files.explorer && !!document.querySelector('#files-overlay #fx-list')", 20000);
    const { root } = await page.S('DOM.getDocument', { depth: 0 });
    const { nodeId } = await page.S('DOM.querySelector', { nodeId: root.nodeId, selector: '#files-overlay #fx-input-files' });
    await page.S('DOM.setFileInputFiles', { files, nodeId });
    await page.waitFor("window.__files.explorer && !window.__files.explorer.busy && document.getElementById('files-overlay').querySelector('#fx-modal').hidden", 30000);
    await sleep(400);
    await clickDom('#files-overlay #fx-close');
  };
  const savedBackCount = () => page.ev('window.__files.engine.savedBack.length');
  const waitSavedBack = (n, ms) => page.waitFor(`window.__files.engine.savedBack.length >= ${n}`, ms, 'the saved file was copied back').then(() => true).catch(() => false);
  const windowTitles = async () => brief(await windows());

  if (steps.includes('save')) {
    // Export > Wave file (Ctrl+R): the engine asks for the target; the window starts in the project's folder
    await page.ev("document.getElementById('desktop').focus()");
    await key('rawKeyDown', 'ControlLeft', 'Control', 17, 2); await key('rawKeyDown', 'KeyR', 'r', 82, 2); await sleep(60); await key('keyUp', 'KeyR', 'r', 82, 2); await key('keyUp', 'ControlLeft', 'Control', 17, 0);
    const asked = await page.waitFor('window.__app.fileDlg.open === true', 120000, 'the Save window').then(() => true).catch(() => false);
    check('Ctrl+R (Export > Wave file) of the engine reaches the page as a Save window', asked);
    await sleep(1500);
    await shot('save-window', desk);
    if (asked) {
      const d = JSON.parse(await page.ev("JSON.stringify({ mode: document.querySelector('#filedlg .fx-window').dataset.mode, name: document.getElementById('fx-filename').value, ok: document.getElementById('fx-ok').textContent, crumbs: document.getElementById('fx-crumbs').textContent, filters: [...document.querySelectorAll('#fx-filter option')].map((o) => [o.textContent, o.selected]), req: window.__app.fileDlg.requests.at(-1).request })"));
      fs.writeFileSync(path.join(out, 'save-request.json'), JSON.stringify(d, null, 1));
      check('it is a Save window with the engine\'s suggested name, the Save label, and the project\'s folder (Downloads\\Sampler-project -> /Sampler-project) as the start folder', d.mode === 'save' && d.name === 'Sampler.wav' && d.ok === 'Save' && d.crumbs === 'Browser files › Sampler-project', JSON.stringify(d).slice(0, 400));
      check('the engine\'s nine export types are the filter list, "Microsoft wave file" selected (filterIndex 3)', d.filters.length === 9 && d.filters[2][1] === true && /wave/.test(d.filters[2][0]), JSON.stringify(d.filters));
      check('FOS_OVERWRITEPROMPT is on in the request (0x2)', (d.req.options & 2) === 2, String(d.req.options));
      await clickDom('#filedlg #fx-ok');
      let rw = null;
      for (let i = 0; i < 480 && !rw; i++) { rw = (await windows()).find((w) => D.isKind(w, 'render')) ?? null; /* the engine's render window (TWAVRenderForm) */ if (!rw) await sleep(250); }
      await settle(page, 240000).catch(() => {});
      await sleep(1500);
      await shot('render-window', desk);
      check('the engine opened its own render window (TWAVRenderForm) after the Save window was answered', !!rw, JSON.stringify(rw));
      if (rw) {
        const sx = rw.x + (1135 - 743), sy = rw.y + (832 - 224);
        const tStart = Date.now();
        await click(sx, sy);
        const got = await waitSavedBack(1, Number(opt('render-wait-ms', '240000')));
        if (!got) log('NO FILE; state', await page.ev('window.__app.state'), 'exit', await page.ev('JSON.stringify(window.__app.exit)'), 'engine log tail:', await page.ev('window.__app.engineLog().slice(-15).join("\\n")'));
        log('exported after', Date.now() - tStart, 'ms (wall):', got);
        await settle(page, 600000).catch(() => {});
        check('the engine wrote the WAV and the page copied it to the OPFS path chosen in the window', got, await page.ev('JSON.stringify(window.__files.engine.savedBack)'));
        if (got) {
          const o = await opfs('/Sampler-project');
          const wav = o.find((e) => e.path === 'Sampler.wav');
          const h = wav ? await opfsSha('/Sampler-project/Sampler.wav') : null;
          log('OPFS /Sampler-project/Sampler.wav', JSON.stringify(wav), h);
          fs.writeFileSync(path.join(out, 'export-wav.json'), JSON.stringify({ size: wav?.size, sha256: h }, null, 1));
          check('the OPFS file is a complete RIFF/WAVE file and, when the engine is the one of decision 0193, equals its Export (353,904 B, f5c6a08b…)', !!wav && wav.size > 44 && (await page.ev(`(async () => { const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('Sampler-project'); const f = await (await d.getFileHandle('Sampler.wav')).getFile(); const b = new Uint8Array(await f.slice(0, 12).arrayBuffer()); return String.fromCharCode(...b.slice(0, 4)) + String.fromCharCode(...b.slice(8, 12)); })()`)) === 'RIFFWAVE', `${wav?.size} B ${h} (0193: 353904 B f5c6a08b1dede642f91026e26939ca99cbea9e20f21bef0c2742c2d5dc3f6d8a)`);
        }
        await shot('after-render', desk);
      }
    }
  }

  if (steps.includes('saveas')) {
    // File > Save as...: a Save window with the .flp type; the existing name asks before replacing
    await fileMenu();
    await click(42, 130);
    const asked = await page.waitFor('window.__app.fileDlg.open === true', 120000, 'the Save as window').then(() => true).catch(() => false);
    check('File > Save as of the engine reaches the page as a Save window', asked);
    await sleep(1500);
    if (asked) {
      await shot('saveas-window', desk);
      const d = JSON.parse(await page.ev("JSON.stringify({ name: document.getElementById('fx-filename').value, crumbs: document.getElementById('fx-crumbs').textContent, sel: [...document.querySelectorAll('#fx-filter option')].find((o) => o.selected)?.textContent, rows: [...document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]')].map((r) => r.dataset.name + (r.dataset.kind === 'directory' ? '/' : '')), req: window.__app.fileDlg.requests.at(-1).request })"));
      log('Save as request', JSON.stringify(d.req));
      check('Save as: the .flp type is selected and the window lists the project\'s .flp (the .wav is hidden by the type)', /song file/.test(d.sel ?? '') && d.rows.includes('Sampler.flp') && !d.rows.includes('Sampler.wav') && d.rows.length === 1, JSON.stringify(d).slice(0, 400));
      // typing the name of an existing file asks before replacing it: answer No (Cancel in the question), then choose a new name
      await page.ev("(() => { const i = document.getElementById('fx-filename'); i.value = 'Sampler.flp'; return true; })()");
      await clickDom('#filedlg #fx-ok');
      await page.waitFor("!document.getElementById('fx-modal').hidden", 10000);
      check('an existing name asks "Replace the file?" (FOS_OVERWRITEPROMPT)', /already exists/.test(await page.ev("document.querySelector('#filedlg #fx-modal p').textContent")));
      await clickDom('#filedlg #fx-modal-cancel');
      await sleep(300);
      check('answering No keeps the dialog open', await winOpen());
      await page.ev("(() => { const i = document.getElementById('fx-filename'); i.value = 'Copy of Sampler'; return true; })()");
      const n0 = await savedBackCount();
      const a0 = await page.ev('window.__files.engine.answers.length');
      await clickDom('#filedlg #fx-ok');
      await settle(page, 240000).catch(() => {});
      await sleep(4000);
      const ans = JSON.parse(await page.ev('JSON.stringify(window.__files.engine.answers)')).at(-1);
      check('the typed name got the default extension and became the Save target in the folder shown (the answer is one S item)', JSON.parse(await page.ev('window.__files.engine.answers.length')) === a0 + 1 && ans.items.length === 1 && ans.items[0][0] === 'S' && ans.items[0][1] === 'C:\\Users\\user\\Downloads\\Sampler-project\\Copy of Sampler.flp', JSON.stringify(ans));
      const ws = await windows();
      const buy = ws.find((w) => /Buy FL Studio/i.test(w.title));
      log('after Save as: windows', JSON.stringify(brief(ws)), 'files copied back', (await savedBackCount()) - n0);
      await shot('after-saveas', desk);
      // the trial build shows its own 'Buy FL Studio' window (TInAppShopForm) first; the engine writes the project after it is closed
      check('the engine first shows its own trial-mode "Buy FL Studio" window (TInAppShopForm) and has not written the file yet', !!buy && (await savedBackCount()) === n0, JSON.stringify(buy ?? ws.map((w) => w.title)));
      if (buy) { await click(buy.x + 313, buy.y + 14); await settle(page, 60000).catch(() => {}); await sleep(1000); if ((await windows()).some((w) => /Buy FL Studio/i.test(w.title))) { await tap(page, 'Escape'); await settle(page, 60000).catch(() => {}); } }
      const got = await waitSavedBack(n0 + 1, 120000);
      await settle(page, 120000).catch(() => {});
      await sleep(1500);
      check('after the Buy window is closed the engine saved the project and the page copied it to /Sampler-project/Copy of Sampler.flp', got && (await opfs('/Sampler-project')).some((e) => e.path === 'Copy of Sampler.flp'), JSON.stringify(await opfs('/Sampler-project')));
      const hdr = await page.ev(`(async () => { const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('Sampler-project'); const f = await (await d.getFileHandle('Copy of Sampler.flp')).getFile(); return String.fromCharCode(...new Uint8Array(await f.slice(0, 4).arrayBuffer())) + ' ' + f.size; })()`).catch((e) => String(e));
      check('the saved file is an FLP (starts with FLhd)', /^FLhd /.test(hdr), hdr);
      check('the main window title follows the saved project', /Copy of Sampler/.test((await mainTitle()) ?? ''), String(await mainTitle()));
      check('the engine is idle again and no engine window is left open', (await page.ev('window.__app.state')) === 'engine-idle' && !(await windows()).some((w) => /Buy FL Studio|Rendering/i.test(w.title)), await page.ev('window.__app.state'));
      await shot('after-saveas', desk);
    }
  }

  if (steps.includes('savemidi')) {
    // File > Export > MIDI file...: another Save window (MIDI type); the page copies the file the engine writes
    await fileMenu();
    await moveTo(page, 38, 270); await settle(page); await sleep(1200);
    await click(259, 362);
    const asked = await page.waitFor('window.__app.fileDlg.open === true', 60000, 'asked').then(() => true).catch(() => false);
    check('File > Export > MIDI file reaches the page as a Save window', asked);
    await sleep(1500);
    if (asked) {
      await shot('savemidi-window', desk);
      const d = JSON.parse(await page.ev("JSON.stringify({ name: document.getElementById('fx-filename').value, sel: [...document.querySelectorAll('#filedlg #fx-filter option')].find((o) => o.selected)?.textContent, req: window.__app.fileDlg.requests.at(-1).request })"));
      log('MIDI export request', JSON.stringify(d.req));
      check('it is a Save window with the MIDI type selected', /MIDI/.test(d.sel ?? ''), JSON.stringify(d).slice(0, 300));
      const n0 = await savedBackCount();
      await clickDom('#filedlg #fx-ok');
      let rw = null;
      for (let i = 0; i < 240 && !rw; i++) { rw = (await windows()).find((w) => D.isKind(w, 'render')) ?? null; /* the engine's render window (TWAVRenderForm) */ if (!rw) await sleep(250); }
      await settle(page, 240000).catch(() => {});
      await sleep(1500);
      log('MIDI export render window', JSON.stringify(rw));
      if (rw) await click(rw.x + (1135 - 743), rw.y + (832 - 224));
      let got = await waitSavedBack(n0 + 1, 20000);
      await sleep(1000);
      const wsM = await windows();
      await shot('after-savemidi', desk);
      log('windows after Start of the MIDI export', JSON.stringify(brief(wsM)), 'copied back', got);
      const ansM = JSON.parse(await page.ev('JSON.stringify(window.__files.engine.answers)')).at(-1);
      check('the answer was one S item: the .mid target at the guest path of the OPFS file chosen in the window, with type index 7', ansM.mode === 'save' && ansM.items.length === 1 && ansM.items[0][0] === 'S' && /^C:\\Users\\user\\Downloads\\Sampler-project\\.*\.mid$/.test(ansM.items[0][1]), JSON.stringify(ansM));
      check('the engine went on with its own MIDI/Wave render window (TWAVRenderForm, output format MID)', !!rw);
      // like the trial's Buy window, the engine's own Warning box (TMsgForm; its text is not drawn) comes first and the file is written after it is dismissed (Enter)
      const warned = wsM.some((w) => D.isKind(w, 'msg'));
      if (warned) { log('the engine shows its own Warning box first (text not drawn); Enter'); await tap(page, 'Enter'); got = await waitSavedBack(n0 + 1, 45000); await sleep(1000); }
      const o = await opfs('/Sampler-project');
      const mid = o.find((e) => /\.mid$/i.test(e.path));
      const magic = got && mid ? await page.ev(`(async () => { const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('Sampler-project'); const f = await (await d.getFileHandle(${JSON.stringify(mid.path)})).getFile(); return String.fromCharCode(...new Uint8Array(await f.slice(0, 4).arrayBuffer())) + ' ' + f.size; })()`) : null;
      check('the engine wrote the MIDI file (after its Warning box was dismissed) and the page copied it to /Sampler-project (it starts with MThd)', /^MThd /.test(magic ?? ''), `${magic} warned=${warned}`);
      if (rw && (await windows()).some((w) => D.isKind(w, 'render'))) { await click(rw.x + 410, rw.y + 15); await sleep(1500); }
      check('the engine is idle again with no render or warning window left', (await page.ev('window.__app.state')) === 'engine-idle' && !(await windows()).some((w) => D.isKind(w, 'render') || D.isKind(w, 'msg')), JSON.stringify(brief(await windows())));
    }
  }

  if (steps.includes('midi')) {
    // File > Import > MIDI file... : the same kind of dialog; the MIDI type is what the engine offers
    const mid = path.join(out, 'mini.mid');
    fs.writeFileSync(mid, Buffer.from([0x4d, 0x54, 0x68, 0x64, 0, 0, 0, 6, 0, 0, 0, 1, 0, 96, 0x4d, 0x54, 0x72, 0x6b, 0, 0, 0, 12, 0x00, 0x90, 0x3c, 0x40, 0x60, 0x80, 0x3c, 0x40, 0x00, 0xff, 0x2f, 0x00]));
    await uploadToOpfs([mid]);
    await fileMenu();
    await moveTo(page, 38, 251); await settle(page); await sleep(1200);
    await click(253, 251);
    const asked = await page.waitFor('window.__app.fileDlg.open === true', 60000, 'asked').then(() => true).catch(() => false);
    await sleep(1500);
    await shot('midi-window', desk);
    check('File > Import > MIDI file reaches the page as the same kind of window', asked);
    if (asked) {
      const info = JSON.parse(await page.ev("JSON.stringify({ req: window.__app.fileDlg.requests.at(-1).request, sel: [...document.querySelectorAll('#fx-filter option')].find((o) => o.selected)?.textContent, mode: document.querySelector('#filedlg .fx-window').dataset.mode, crumbs: document.getElementById('fx-crumbs').textContent })"));
      log('MIDI request:', JSON.stringify(info.req));
      fs.writeFileSync(path.join(out, 'midi-request.json'), JSON.stringify(info, null, 1));
      check('Import > MIDI file is an Open window with the MIDI type selected', info.mode === 'open' && /MIDI/i.test(info.sel ?? ''), JSON.stringify(info).slice(0, 400));
      await navWindow('/');
      await page.waitFor("[...document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]')].some((r) => r.dataset.name === 'mini.mid')", 10000);
      await rowAct('mini.mid', 'click');
      const n = (await page.ev('window.__files.engine.answers.length'));
      await clickDom('#filedlg #fx-ok');
      await settle(page, 240000).catch(() => {});
      await sleep(3000);
      const ans = JSON.parse(await page.ev('JSON.stringify(window.__files.engine.answers)')).at(-1);
      check('the answer is the MIDI file alone (R) at its guest path', JSON.parse(await page.ev('window.__files.engine.answers.length')) === n + 1 && ans.items.length === 1 && ans.items[0][0] === 'R' && ans.items[0][1] === 'C:\\Users\\user\\Downloads\\mini.mid', JSON.stringify(ans));
      const st = await page.ev('window.__app.state');
      const cl = await windowTitles();
      log('after the MIDI choice: state', st, 'windows', cl.join(','));
      await shot('after-midi', desk);
      check('the engine went on after the MIDI choice (not stopped)', st === 'engine-idle' || st === 'engine-computing', `${st} ${cl.join(',')}`);
    }
  }

  if (steps.includes('zip')) {
    // File > Open with a ZIP chosen: the page unpacks it into the OPFS and the engine opens the project inside
    const { makeZip } = await import('../helpers/zipmake.ts');
    const zipFile = path.join(out, 'Zipped Sampler.zip');
    const flp = fs.readFileSync(path.join(projectDir, 'Sampler.flp')), wav = fs.readFileSync(path.join(projectDir, 'SHP_Sawing Synth.wav'));
    fs.writeFileSync(zipFile, makeZip([{ name: Buffer.from('Zipped Sampler/Z Sampler.flp'), data: flp, flags: 0x800 }, { name: Buffer.from('Zipped Sampler/SHP_Sawing Synth.wav'), data: wav, flags: 0x800 }]));
    await uploadToOpfs([zipFile]);
    log('before File > Open (zip): windows', JSON.stringify(brief(await windows())));
    await fileMenu();
    log('after the File click: windows', JSON.stringify(brief(await windows())));
    await shot('zip-file-menu', desk);
    await click(48, 85);
    // the project was changed by the MIDI import: the engine first asks, in its own Confirm box (TMsgForm: Yes / No / Cancel; the text is not drawn by the lifted engine), whether to save; No goes on to the Open dialog
    let askedZ = false, confirmed = false;
    for (let i = 0; i < 150 && !askedZ; i++) {
      askedZ = (await page.ev('window.__app.fileDlg.open === true')) === true;
      if (askedZ) break;
      const msg = (await windows()).find((w) => D.isKind(w, 'msg'));
      if (msg && !confirmed) {
        confirmed = true;
        log('the engine asks in its own TMsgForm box (a modified project is being replaced):', JSON.stringify(msg));
        await shot('zip-confirm', desk);
        // the box's width differs between runs (the buttons move with it) and the N key did not close it (measured), so a click at 55 % / 85 % of its rectangle, where "No" is in the screenshots; no settle(): the engine goes on into the page's dialog and is not idle
        await click(Math.round(msg.x + msg.w * 0.55), Math.round(msg.y + msg.h * 0.85));
        await sleep(1500);
      }
      await sleep(300);
    }
    log('Open asked after the engine\'s own Confirm box:', confirmed, '; asked', askedZ);
    if (!askedZ) await shot('zip-open-not-asked', desk);
    check('File > Open asks again (after the engine\'s own Save-changes box, when the project was modified)', askedZ);
    await sleep(1000);
    await navWindow('/');
    await page.waitFor("[...document.querySelectorAll('#filedlg #fx-list .fx-row[data-name]')].some((r) => r.dataset.name === 'Zipped Sampler.zip')", 10000);
    check('the zip is listed (the engine\'s "Zipped song file" type is in the filter list)', true);
    await rowAct('Zipped Sampler.zip', 'click');
    const n = await page.ev('window.__files.engine.answers.length');
    const t1 = Date.now();
    await clickDom('#filedlg #fx-ok');
    let title = null;
    const want = 'Z Sampler.flp - FL Studio 2026';   // a name the title cannot have from an earlier step
    for (let i = 0; i < 480 && title !== want; i++) { await sleep(250); title = await mainTitle(); }
    log('zip: title', title, 'after', Date.now() - t1, 'ms (wall)');
    const ans = JSON.parse(await page.ev('JSON.stringify(window.__files.engine.answers)')).at(-1);
    check('the zip was unpacked in the page (the answer names the unpacked folder) and the engine opened the project inside it', title === want && JSON.parse(await page.ev('window.__files.engine.answers.length')) === n + 1 && !!ans.unpackedZip, JSON.stringify(ans));
    const o = await opfs('/');
    check('the unpacked files are in the OPFS next to the zip', o.some((e) => e.path.endsWith('/Z Sampler.flp') && e.path.startsWith('Zipped Sampler')), JSON.stringify(o.map((e) => e.path)));
    await settle(page, 120000).catch(() => {});
    await shot('after-zip-open', desk);
  }
  if (steps.includes('cancel')) {
    // Escape in the window cancels the engine's dialog. The window is built a moment after the request arrives (it opens the OPFS and lists the first folder): wait until its name box has the focus
    await fileMenu();
    await click(48, 85);
    check('cancel: File > Open asks again', await page.waitFor('window.__app.fileDlg.open === true', 120000, 'box').then(() => true).catch(() => false));
    await page.waitFor("document.activeElement && document.activeElement.id === 'fx-filename'", 20000, 'the window has the focus').catch(() => {});
    await page.S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'Escape', key: 'Escape', windowsVirtualKeyCode: 27 });
    await page.S('Input.dispatchKeyEvent', { type: 'keyUp', code: 'Escape', key: 'Escape', windowsVirtualKeyCode: 27 });
    await page.waitFor('window.__app.fileDlg.open === false', 20000, 'closed').catch(() => {});
    await sleep(2500);
    const s = JSON.parse(await page.ev("JSON.stringify({ hidden: document.getElementById('filedlg').hidden, state: window.__app.state, last: window.__app.fileDlg.history.at(-1)?.outcome, focus: document.activeElement && document.activeElement.id })"));
    check('Escape cancels the engine\'s dialog, the engine is idle again and the desktop has the keyboard back', s.hidden && s.last === 'cancelled' && s.focus === 'desktop' && s.state === 'engine-idle', JSON.stringify(s));
    // Cancel with the button, and the next File > Open asks again
    await fileMenu();
    await click(48, 85);
    await page.waitFor('window.__app.fileDlg.open === true', 120000, 'box 2').catch(() => {});
    await page.waitFor("document.activeElement && document.activeElement.id === 'fx-filename'", 20000, 'focus 2').catch(() => {});
    await clickDom('#filedlg #fx-cancel-dialog');
    await page.waitFor('window.__app.fileDlg.open === false', 20000, 'closed 2').catch(() => {});
    await sleep(2000);
    const s2 = JSON.parse(await page.ev("JSON.stringify({ hidden: document.getElementById('filedlg').hidden, last: window.__app.fileDlg.history.at(-1)?.outcome, n: window.__app.fileDlg.history.length })"));
    check('the Cancel button cancels too, and the engine is asked again each time (history of answered + cancelled dialogs)', s2.hidden && s2.last === 'cancelled' && s2.n >= 6, JSON.stringify(s2));
  }
} catch (e) { check('ran to its end', false, e.stack || e); }
const state = await page.ev('window.__app.state').catch(() => null);
log('final state', state);
fs.writeFileSync(path.join(out, 'engine-log.txt'), (await page.ev('window.__app.engineLog().join("\\n")').catch(() => '')) || '');
const closed = await browser.close();
log(closed.line);
await srv.stop();
fs.writeFileSync(path.join(out, 'results.json'), JSON.stringify(results, null, 1));
const bad = results.filter((r) => !r.ok);
log(`${results.length - bad.length}/${results.length} passed`);
process.exit(bad.length ? 1 : 0);
void sha;
