// The default entrance (no parameter) of the served bundle in an isolated headless Chrome 156 or Firefox 158: first idle, title, console errors, the beacon (POST /__perf) and its credentials mode, the window list.
// usage: node entry-check.mjs --browser chrome|firefox --url http://127.0.0.1:PORT/ --app APPDIR --out DIR [--wait-idle-s 300] [--beacon-wait-s 8]
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const B = opt('browser', 'chrome'), URL0 = opt('url'), APP = path.resolve(opt('app')), OUT = path.resolve(opt('out')), WAIT = Number(opt('wait-idle-s', '300')), BW = Number(opt('beacon-wait-s', '8'));
fs.mkdirSync(OUT, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const imp = (p) => import(pathToFileURL(path.join(APP, p)).href);
const PATCH = `(() => { if (window.__fetchLog) return true; window.__fetchLog = []; const f = window.fetch; window.fetch = function (input, init) { try { const u = typeof input === 'string' ? input : input.url; window.__fetchLog.push({ url: String(u).slice(0, 120), method: (init && init.method) || 'GET', credentials: (init && init.credentials) || null }); } catch (e) {} return f.apply(window, arguments); }; return true; })()`;
const STATE = `JSON.stringify((() => { const a = window.__app; if (!a) return { state: 'no-app' }; const wins = (a.appView && a.appView.windows) || []; const mw = a.mainWindow; const jsonSafe = (v) => { try { return JSON.parse(JSON.stringify(v)); } catch (e) { return String(v); } }; return { keys: Object.keys(a), mainWindow: jsonSafe(typeof mw === 'function' ? mw.call(a) : mw), pageTitle: jsonSafe(typeof a.pageTitle === 'function' ? a.pageTitle() : a.pageTitle), screen: jsonSafe(a.screen), summary: jsonSafe(typeof a.summary === 'function' ? a.summary() : a.summary), bannerEl: !!document.getElementById('standin-banner'), state: a.state, kind: a.engineKind, idle: a.timeline && a.timeline.idle, error: a.error, exit: a.exit, bannerHidden: (document.getElementById('standin-banner') || {}).hidden, standinText: document.body.innerText.includes('STAND-IN'), mainTitle: (wins.filter((w) => w.className === 'TFruityLoopsMainForm').map((w) => w.title)[0]) ?? null, docTitle: document.title, visible: wins.filter((w) => w.visible).map((w) => w.className), canvases: [...document.querySelectorAll('canvas')].map((c) => c.width + 'x' + c.height), perfHistory: (a.perf && a.perf.history || []).map((h) => ({ seq: h.seq, reason: h.reason, bytes: h.bytes, result: jsonSafe(h.result) })), perfSent: jsonSafe(a.perf && a.perf.sent), fetchPerf: (window.__fetchLog || []).filter((x) => x.url.includes('__perf')) }; })())`;
const res = { browser: B, url: URL0 };
let ev, goto, close, consoleErrors = () => [];
if (B === 'chrome') {
  const { openBrowser } = await import('../lib/browser.mjs');
  const br = await openBrowser(OUT, 'entry');
  const page = await br.newPage({ width: 2176, height: 1224, initScript: PATCH });
  res.version = br.version; ev = page.ev; goto = page.goto;
  consoleErrors = () => page.consoleLog.filter((l) => l.startsWith('error') || l.startsWith('EXCEPTION') || l.startsWith('assert'));
  close = async () => { await page.close().catch(() => {}); await br.close(); };
  res.page = page;
} else {
  const { openFirefox } = await imp('test/e2e/firefox-driver.mjs');
  const ff = await openFirefox({ work: OUT, downloadDir: path.join(OUT, 'dl'), dataDir: path.join(OUT, 'data'), extraPrefs: { 'devtools.console.stdout.content': true } });
  res.version = ff.version; ev = ff.ev; goto = async (u) => { await ff.goto(u); await ff.ev(PATCH); };
  consoleErrors = () => fs.existsSync(path.join(OUT, 'geckodriver.log')) ? fs.readFileSync(path.join(OUT, 'geckodriver.log'), 'utf8').split('\n').filter((l) => /console\.(error|warn)|JavaScript error|Uncaught/i.test(l) && !/console\.warn/.test(l)).slice(0, 60) : [];
  close = async () => { await ff.close(); };
}
const t0 = Date.now();
try {
  await goto(URL0);
  let st = {};
  for (;;) {
    await sleep(1000);
    st = JSON.parse(await ev(STATE).catch(() => '{}'));
    if (st.state === 'error' || st.state === 'exited' || typeof st.idle === 'number') break;
    if ((Date.now() - t0) / 1000 > WAIT) break;
  }
  res.waitedS = (Date.now() - t0) / 1000;
  await sleep(BW * 1000);
  res.final = JSON.parse(await ev(STATE));
  res.consoleErrorsAll = consoleErrors(); res.consoleErrors = res.consoleErrorsAll.filter((l) => !/resource:\/\/|chrome:\/\/|Experiment |RSLoader|rust_mozprofile|FaviconLoader|Nimbus|moz-extension|NS_ERROR_FILE_NO_DEVICE_SPACE/.test(l));
  if (B === 'chrome') {
    const png = await res.page.shot(0, 0, 2176, 1224).catch(() => null);
    if (png) fs.writeFileSync(path.join(OUT, 'entry-' + B + '.png'), png);
  }
} catch (e) { res.exception = String(e.stack ?? e).slice(0, 600); }
delete res.page;
fs.writeFileSync(path.join(OUT, 'entry-' + B + '.json'), JSON.stringify(res, null, 1));
console.log(JSON.stringify(res, null, 1).slice(0, 4000));
await close();
process.exit(0);
