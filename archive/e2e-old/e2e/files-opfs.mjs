// End-to-end test of the file manager (decision 0216, opfs1) in an isolated headless Chrome with the browser's REAL OPFS (no mock), against `vite preview` of the production build (so the page's
// Content-Security-Policy is in force: the new code must not need an inline style or a blocked origin). The same scenario (files-scenario.mjs) runs in Firefox through geckodriver: --browser firefox.
// usage: node test/e2e/files-opfs.mjs [--browser chrome|firefox] [--target preview|dev] [--no-build] [--data DIR] [--only select]
import fs from 'node:fs';
import path from 'node:path';
import { Buffer } from 'node:buffer';
import { spawn } from 'node:child_process';
import { run, start, sleep, freePort } from '../helpers/proc.mjs';
import { APP, workDirOf } from '../helpers/wrangler.mjs';
import { openBrowser } from './browser.mjs';
import { makeTestData } from './files-testdata.mjs';
import { scenario, selectScenario, usageScenario } from './files-scenario.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const browserName = opt('browser', 'chrome');
const target = opt('target', 'preview');
const work = workDirOf('files-opfs');
// the test data and the downloads go under the work dir (APP_WORK; never into the repository). A snap Firefox can read files under $HOME but not /tmp: set APP_WORK to a directory under $HOME for --browser firefox.
const dataDir = path.resolve(opt('data', path.join(work, 'testdata')));
fs.mkdirSync(work, { recursive: true });
const bin = (n) => path.join(APP, 'node_modules/.bin', n);

const results = [];
const check = (name, ok, detail = '') => { results.push({ name, ok: !!ok, detail }); console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail && !ok ? '  -- ' + String(detail).slice(0, 400) : detail && process.env.VERBOSE ? '  -- ' + String(detail).slice(0, 300) : ''}`); };
const log = (s) => console.log('INFO  ' + s);

if (target === 'preview' && !argv.includes('--no-build')) await run(bin('vite'), ['build'], { cwd: APP, env: process.env, label: 'vite build' });
const data = makeTestData(dataDir);
const port = await freePort();
const assets = path.join(work, 'assets'); fs.mkdirSync(assets, { recursive: true });
const env = { ...process.env, FL_ASSETS_DIR: assets, FL_PERF_LOG: path.join(work, 'perf.jsonl') };
const server = target === 'preview'
  ? start(bin('vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env, ready: /Local:\s+http/, label: 'vite preview' })
  : start(bin('vite'), ['--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env, ready: /Local:\s+http/, label: 'vite dev' });
await server.ready;
const url = `http://127.0.0.1:${port}/`;

const downloadDir = path.join(path.dirname(dataDir), 'downloads-' + browserName);
let closeAll = async () => {};
let page;

if (browserName === 'chrome') {
  const b = await openBrowser(work, 'files');
  const p = await b.newPage({ width: 1400, height: 900 });
  await p.S('DOM.enable');
  await p.S('Page.setDownloadBehavior', { behavior: 'allow', downloadPath: downloadDir }).catch(() => {});
  await p.goto(url);
  await p.waitFor('document.readyState === "complete" && !!window.__files', 30000);
  const setFiles = async (selector, paths) => {
    const { root } = await p.S('DOM.getDocument', { depth: 0 });
    const { nodeId } = await p.S('DOM.querySelector', { nodeId: root.nodeId, selector });
    await p.S('DOM.setFileInputFiles', { files: paths, nodeId });
  };
  page = {
    ev: p.ev, setFiles, downloadDir,
    // a directory chosen with <input webkitdirectory>: CDP is given the directory (Chrome expands it) - the result is read back from the input's webkitRelativePath values
    async dropPaths(paths) {
      const box = JSON.parse(await p.ev(`JSON.stringify((() => { const r = document.querySelector('.fx-window').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })())`));
      const data = { items: [], files: paths, dragOperationsMask: 1 };
      for (const type of ['dragEnter', 'dragOver', 'drop']) await p.S('Input.dispatchDragEvent', { type, x: box.x, y: box.y, data });
    },
    async setDirectory(selector, dir) {
      await setFiles(selector, [dir]);
      return 'CDP DOM.setFileInputFiles with the directory path';
    },
  };
  closeAll = async () => { const r = await b.close(); log(`browser: ${b.version}; console lines: ${p.consoleLog.length}; ${r.line}`); if (p.consoleLog.length) fs.writeFileSync(path.join(work, 'console.txt'), p.consoleLog.join('\n')); };
  log(`Chrome ${b.version}, ${url}`);
} else {
  const { openFirefox } = await import('./firefox-driver.mjs');
  const ff = await openFirefox({ work, downloadDir, dataDir });
  await ff.goto(url);
  await ff.waitFor('document.readyState === "complete" && !!window.__files', 30000);
  page = { ev: ff.ev, setFiles: ff.setFiles, setDirectory: ff.setDirectory, downloadDir };
  closeAll = async () => { await ff.close(); };
  log(`Firefox ${ff.version}, ${url}`);
}

let failed = false;
try {
  if (opt('only', '') !== 'select') await scenario({ page, check, data, log, browserName, target });
  await selectScenario({ page, check, data, log, browserName });
  await usageScenario({ page, check, browserName, full: opt('only', '') !== 'select' });
} catch (e) {
  failed = true;
  check('the scenario ran to its end', false, e.stack || String(e));
}
await closeAll();
await server.stop();
const bad = results.filter((r) => !r.ok);
fs.writeFileSync(path.join(work, `results-${browserName}-${target}.json`), JSON.stringify({ browserName, target, date: new Date().toISOString(), results }, null, 1));
console.log(`\n${results.length - bad.length}/${results.length} passed (${browserName}, ${target})`);
process.exit(bad.length || failed ? 1 : 0);
void Buffer; void spawn; void sleep;
