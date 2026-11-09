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
  const { openBrowser } = await imp('test/e2e/browser.mjs');
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
const N = Number(opt('loads', '7'));
res.loads = [];
const pressure = () => { try { return fs.readFileSync('/proc/pressure/io', 'utf8').split('\n')[0].replace('some ', ''); } catch { return null; } };
try {
  for (let k = 1; k <= N; k++) {
    const t0 = Date.now(); const io0 = pressure(); const load0 = fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0];
    await goto(URL0);
    let st = {};
    for (;;) {
      await sleep(500);
      st = JSON.parse(await ev(STATE).catch(() => '{}'));
      if (st.state === 'error' || st.state === 'exited' || typeof st.idle === 'number') break;
      if ((Date.now() - t0) / 1000 > WAIT) break;
    }
    const tl = await ev(`JSON.stringify((() => { const a = window.__app; return a ? { timeline: a.timeline, assets: (a.perf && a.perf.snapshot ? a.perf.snapshot('probe').assets : null) } : null; })())`).catch(() => null);
    let tlj = null; try { tlj = JSON.parse(tl); } catch { /* none */ }
    res.loads.push({ k, firstIdleMs: st.idle ?? null, state: st.state, load0, io0, io1: pressure(), load1: fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0], timeline: tlj && tlj.timeline, assets: tlj && tlj.assets && tlj.assets.items && tlj.assets.items.map((i) => ({ kind: i.kind, origin: i.origin, store: i.store, ms: i.ms })) });
    await sleep(Number(opt('between-s', '8')) * 1000);
  }
} catch (e) { res.exception = String(e.stack ?? e).slice(0, 600); }
delete res.page;
fs.writeFileSync(path.join(OUT, 'warm-' + B + '.json'), JSON.stringify(res, null, 1));
console.log(JSON.stringify(res, null, 1).slice(0, 4000));
await close();
process.exit(0);
