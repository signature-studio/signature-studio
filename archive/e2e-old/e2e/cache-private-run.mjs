// Decision 0220, job 1(c): the REAL page twice in a row inside ONE private session -- Chrome's off-the-record context (Target.createBrowserContext) and Firefox with permanent private browsing --
// to see what the HTTP cache and the page's own stored copy do there. Per load: the page's assets report (origin of each asset, bytes on the wire, store state and its failure code) and the timeline.
// usage: node test/e2e/cache-private-run.mjs --browser chrome|firefox --port 4963 [--query "cache=0"] [--out FILE]
import fs from 'node:fs';
import path from 'node:path';
import { Cdp, launchChrome, shutdown, sleep } from './cdp.mjs';
import { openFirefox } from './firefox-driver.mjs';
const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const which = opt('browser', 'chrome'), port = Number(opt('port', '4963')), query = opt('query', '');
const WORK = '/home/ubuntu/work/signature-studio-develop/lift/out-cache1/work';
const url = `http://127.0.0.1:${port}/` + (query ? '?' + query : '');
const PROBE = `(() => { const a = window.__app; return JSON.stringify({ state: a && a.state, tl: a && a.timeline, rep: a && a.assetsReport && a.assetsReport() }); })()`;
async function waitLoaded(ev) {
  for (let i = 0; i < 150; i++) {
    await sleep(1000);
    const j = JSON.parse(await ev(PROBE).catch(() => '{}') || '{}');
    if (j.tl && typeof j.tl.worker_ready === 'number') { await sleep(1500); return JSON.parse(await ev(PROBE)); }
    if (j.state === 'error') return j;
  }
  return null;
}
const loads = [];
let version = '';
if (which === 'chrome') {
  const profile = path.join(WORK, 'private-profile-chrome'); fs.rmSync(profile, { recursive: true, force: true });
  const chrome = await launchChrome({ profile, extraArgs: ['--no-proxy-server', '--disable-background-networking'] }); version = chrome.version.Browser;
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl); await cdp.opened;
  try {
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: false });
    for (let n = 1; n <= 2; n++) {
      const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
      const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
      await cdp.send('Page.enable', {}, sessionId); await cdp.send('Runtime.enable', {}, sessionId);
      await cdp.send('Page.navigate', { url }, sessionId);
      const ev = async (expression) => { const r = await cdp.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sessionId); if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 200)); return r.result.value; };
      loads.push(await waitLoaded(ev));
      await sleep(8000); // let a stored copy finish writing
      loads[loads.length - 1].rep = JSON.parse(await ev(PROBE)).rep;
      await cdp.send('Target.closeTarget', { targetId }).catch(() => {});
    }
  } finally { await shutdown(chrome, cdp); }
} else {
  const work = path.join(WORK, 'ff-private-work'); fs.mkdirSync(work, { recursive: true });
  const ff = await openFirefox({ work, downloadDir: path.join(work, 'dl'), dataDir: work, extraPrefs: { 'browser.privatebrowsing.autostart': true }, scriptTimeoutMs: 120000 }); version = ff.version;
  try {
    for (let n = 1; n <= 2; n++) {
      await ff.goto(url);
      loads.push(await waitLoaded((e) => ff.ev(e)));
      await sleep(8000);
      loads[loads.length - 1].rep = JSON.parse(await ff.ev(PROBE)).rep;
    }
  } finally { await ff.close(); }
}
const L = [`## ${which} ${version}, ${which === 'chrome' ? 'an off-the-record context (Target.createBrowserContext)' : 'permanent private browsing (browser.privatebrowsing.autostart)'}, ${query ? '?' + query : 'default URL (stored copy on)'}: the same page loaded twice in one session; load ${fs.readFileSync('/proc/loadavg', 'utf8').split(' ')[0]}`, '', '| load | asset | origin | bytes on the wire (transfer) | stored copy | failure code | read ms | hash ms |', '|---:|---|---|---:|---|---|---:|---:|'];
loads.forEach((l, i) => { for (const it of (l?.rep?.items ?? [])) L.push(`| ${i + 1} | ${it.kind} | ${it.origin} | ${it.transfer ?? '-'} | ${it.store} | ${it.fail ?? '-'} | ${it.ms.read} | ${it.ms.hash} |`); if (!l?.rep) L.push(`| ${i + 1} | (no report: ${l?.state ?? 'timeout'}) | | | | | | |`); });
console.log(L.join('\n'));
if (opt('out', '')) fs.appendFileSync(opt('out'), L.join('\n') + '\n\n');
