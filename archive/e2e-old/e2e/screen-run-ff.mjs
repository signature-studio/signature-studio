// Decision 0212 (screen1): the measurement of the guest's screen against the room the page has (screen-run.mjs for Chrome was deleted in desk2: engine-desk-resize checks it) in headless FIREFOX through geckodriver (WebDriver classic): opens the page at a window size and a device pixel ratio
// (pref layout.css.devPixelsPerPx), waits for the engine's first idle, records the engine's report of its screen, the stage / desktop rectangles and the canvases, and takes a screenshot of the page.
// usage: FL_ASSETS_DIR=<dir> FL_DIST_DIR=<built app> node test/e2e/screen-run-ff.mjs --gd http://127.0.0.1:4981 --window 2176x1224 --dpx 1.7647 --out DIR
// The window size is the OUTER size (WebDriver window rect): the inner size is printed. Not a person's Firefox on Windows: Firefox 158 on this Linux machine.
import fs from 'node:fs';
import path from 'node:path';
import { freePort, start } from '../helpers/proc.mjs';
import { APP } from '../helpers/wrangler.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const GD = opt('gd', 'http://127.0.0.1:4981');
const [ww, wh] = opt('window', '2176x1224').split('x').map(Number);
const dpx = opt('dpx', '1');
const query = opt('query', '');
const out = path.resolve(opt('out', '/tmp/opencode/screen1/ff'));
const waitIdleS = Number(opt('wait-idle-s', '300'));
fs.mkdirSync(out, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);
async function wd(method, p, body) {
  const r = await fetch(GD + p, { method, headers: { 'content-type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
  const j = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(`${method} ${p}: ${r.status} ${JSON.stringify(j).slice(0, 400)}`);
  return j.value;
}
const port = await freePort();
const server = start(path.join(APP, 'node_modules/.bin/vite'), [...(process.env.FL_DIST_DIR ? ['preview', '--outDir', process.env.FL_DIST_DIR] : []), '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env, FL_PERF_LOG: path.join(out, 'perf-log.jsonl') }, ready: /Local:\s+http/, label: 'vite' });
await server.ready;
const result = { window: { ww, wh }, dpx, query, steps: [] };
let sid = null;
try {
  const created = await wd('POST', '/session', { capabilities: { alwaysMatch: { browserName: 'firefox', 'moz:firefoxOptions': { args: ['-headless'], prefs: { 'network.proxy.type': 0, 'datareporting.healthreport.uploadEnabled': false, 'toolkit.telemetry.enabled': false, 'layout.css.devPixelsPerPx': dpx } } } } });
  sid = created.sessionId;
  result.browser = { version: created.capabilities.browserVersion, userAgent: created.capabilities.userAgent };
  log('session', sid, result.browser.version);
  const S = (p) => `/session/${sid}${p}`;
  await wd('POST', S('/window/rect'), { x: 0, y: 0, width: ww, height: wh });
  const ev = async (expr) => wd('POST', S('/execute/sync'), { script: `return (${expr})`, args: [] });
  await wd('POST', S('/url'), { url: `http://127.0.0.1:${port}/${query ? '?' + query : ''}` });
  const t0 = Date.now();
  let idle = null;
  while ((Date.now() - t0) / 1000 < waitIdleS) {
    await sleep(2000);
    const s = await ev("(() => { const a = window.__app; return a ? JSON.stringify({ state: a.state, idle: a.timeline.idle ?? null, err: a.error }) : null; })()").catch(() => null);
    if (!s) continue;
    const j = JSON.parse(s);
    if (j.state === 'error' || j.state === 'exited') { log('ended early', s); result.endedEarly = j; break; }
    if (typeof j.idle === 'number') { idle = j.idle; break; }
  }
  result.firstIdleSeconds = idle === null ? null : idle / 1000;
  log('first idle', result.firstIdleSeconds);
  await sleep(5000);
  result.snapshot = JSON.parse(await ev(`JSON.stringify((() => {
    const a = window.__app;
    const R = (sel) => { const e = document.querySelector(sel); if (!e) return null; const r = e.getBoundingClientRect(); return { left: +r.left.toFixed(2), top: +r.top.toFixed(2), width: +r.width.toFixed(2), height: +r.height.toFixed(2) }; };
    return { innerWidth: innerWidth, innerHeight: innerHeight, devicePixelRatio: devicePixelRatio, crossOriginIsolated: crossOriginIsolated,
      doc: { scrollWidth: document.documentElement.scrollWidth, scrollHeight: document.documentElement.scrollHeight },
      header: R('header'), stage: R('#stage'), desktop: R('#desktop'), aside: R('aside'), screen: a.screen, state: a.state, engineKind: a.engineKind,
      title: document.title,
      windows: null,   // (the window list of the compositor is not held by the page any more: read it with window.__app.deskDumpBase64)
      canvases: [...document.querySelectorAll('#desktop canvas')].filter((c) => c.style.display !== 'none').map((c) => { const r = c.getBoundingClientRect(); return { bitmap: [c.width, c.height], css: [+r.left.toFixed(2), +r.top.toFixed(2), +r.width.toFixed(2), +r.height.toFixed(2)] }; }) };
  })())`));
  log('snapshot', JSON.stringify({ inner: [result.snapshot.innerWidth, result.snapshot.innerHeight], dpr: result.snapshot.devicePixelRatio, stage: result.snapshot.stage, guest: result.snapshot.screen.guest, reported: result.snapshot.screen.reported }));
  const png = await wd('GET', S('/screenshot'));
  fs.writeFileSync(path.join(out, 'shot-1.png'), Buffer.from(png, 'base64'));
} catch (e) {
  result.error = String(e?.stack ?? e);
  log('ERROR', result.error.slice(0, 500));
} finally {
  fs.writeFileSync(path.join(out, 'result.json'), JSON.stringify(result, null, 1));
  if (sid) await wd('DELETE', `/session/${sid}`).catch(() => {});
  await server.stop();
}
log('done');
