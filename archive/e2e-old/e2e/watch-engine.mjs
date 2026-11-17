// Opens ONE engine bundle in the isolated headless Chrome of the e2e tests (the same browser.mjs: isolated context, network blocked except 127.0.0.1) and WATCHES it: prints the state and the
// start-up marks every few seconds, and when the engine stops (or goes idle, or the page fails) saves a screenshot and the engine's printed lines. No assertions: it is the tool for an engine whose stop is not
// known in advance (decision 0137: a bundle built from another tree, for example with SEH). Only the dev/preview file server of the app is used.
// With --end-session, an engine that goes idle is then ended through the page's "End session" button (fle_idle_wait returns 0) and the exit that follows is printed.
// usage: FL_ASSETS_DIR=<dir whose manifest.json is the bundle's> node test/e2e/watch-engine.mjs [--minutes 30] [--out DIR] [--end-session]
import fs from 'node:fs';
import path from 'node:path';
import { freePort, start } from '../helpers/proc.mjs';
import { APP, workDirOf } from '../helpers/wrangler.mjs';
import { openBrowser, sleep } from './browser.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
if (!process.env.FL_ASSETS_DIR) { console.error('usage: FL_ASSETS_DIR=<dir> node test/e2e/watch-engine.mjs [--minutes 30] [--out DIR]'); process.exit(2); }
const minutes = Number(opt('minutes', '30'));
const out = path.resolve(opt('out', path.join(workDirOf('watch'), 'out')));
fs.mkdirSync(out, { recursive: true });
const work = workDirOf('watch');

const port = await freePort();
const server = start(path.join(APP, 'node_modules/.bin/vite'), ['--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env }, ready: /Local:\s+http/, label: 'vite dev' });
await server.ready;
const browser = await openBrowser(work, 'watch');
const page = await browser.newPage({ width: 2400, height: 1300 });
const t0 = Date.now();
await page.goto(`http://127.0.0.1:${port}/`);
let last = '';
let final = null;
while ((Date.now() - t0) / 60000 < minutes) {
  await sleep(5000);
  const s = await page.ev("JSON.stringify({ state: window.__app.state, tl: window.__app.timeline, err: window.__app.error, exit: window.__app.exit })").catch((e) => JSON.stringify({ state: 'unreadable: ' + e.message }));
  const j = JSON.parse(s);
  const line = `${((Date.now() - t0) / 1000).toFixed(0)} s state=${j.state} marks=${Object.keys(j.tl ?? {}).join(',')}`;
  if (line.replace(/^\S+ s /, '') !== last) console.log(line);
  last = line.replace(/^\S+ s /, '');
  if (['exited', 'error', 'engine-idle'].includes(j.state)) { final = j; break; }
}
console.log('final:', JSON.stringify(final ?? { state: 'still running after ' + minutes + ' min' }).slice(0, 1500));
let ended = null;
if (argv.includes('--end-session') && final?.state === 'engine-idle') {
  const label = await page.ev("document.getElementById('stop-btn').textContent");
  await page.ev("document.getElementById('stop-btn').click()");
  const t1 = Date.now();
  let st = '';
  while (Date.now() - t1 < 120000) { await sleep(500); st = await page.ev('window.__app.state').catch(() => 'unreadable'); if (st === 'exited' || st === 'error') break; }
  ended = { buttonLabel: label, state: st, seconds: (Date.now() - t1) / 1000, exit: await page.ev('window.__app.exit').catch(() => null), exitText: await page.ev("document.getElementById('exit').textContent").catch(() => null), error: await page.ev('window.__app.error').catch(() => null), stopButtonHidden: await page.ev("document.getElementById('stop-btn').hidden").catch(() => null) };
  console.log('after End session:', JSON.stringify(ended).slice(0, 1500));
}
const shot = await page.S('Page.captureScreenshot', { format: 'png', fromSurface: true });
fs.writeFileSync(path.join(out, 'watch-page.png'), Buffer.from(shot.data, 'base64'));
const rect = JSON.parse((await page.ev("JSON.stringify(document.querySelector('#main-slot canvas')?.getBoundingClientRect().toJSON() ?? null)").catch(() => 'null')) ?? 'null');
if (rect) fs.writeFileSync(path.join(out, 'watch-canvas.png'), await page.shot(rect.x, rect.y, Math.round(rect.width), Math.round(rect.height)));
fs.writeFileSync(path.join(out, 'watch-engine-log.txt'), ((await page.ev('window.__app.engineLog()').catch(() => [])) ?? []).join('\n') + '\n');
fs.writeFileSync(path.join(out, 'watch-state.json'), JSON.stringify({ final, ended, summary: await page.ev('window.__app.summary').catch(() => null), checks: await page.ev('window.__app.checks').catch(() => null), details: await page.ev('window.__app.details').catch(() => null) }, null, 1));
const nl = await browser.close();
console.log(nl.line);
await server.stop();
process.exit(0);
