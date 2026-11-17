// Decision 0220, job 1(c): does Chrome's HTTP cache keep a response of N MiB? Loads the probe server's /f/<n>MB twice (two navigations, the cache ENABLED) and
// compares PerformanceResourceTiming.transferSize and the server's request log. A throw-away profile on disk, flags from --flags.
// Usage: node test/e2e/cache-probe-chrome.mjs --port 8801 --sizes 1,5,6,7,8,12,16,24,32,48 [--flags "--disk-cache-size=1073741824"] [--profile DIR] [--label X] [--prefix /f/ --suffix MB] [--urls a.js,b.wasm]
import fs from 'node:fs';
import path from 'node:path';
import { Cdp, launchChrome, shutdown, sleep } from './cdp.mjs';

const arg = (n, d) => { const i = process.argv.indexOf('--' + n); return i >= 0 ? process.argv[i + 1] : d; };
const port = Number(arg('port', '8801'));
const urls = arg('urls', '') ? arg('urls').split(',') : arg('sizes', '1,5,6,7,8,12,16,24,32,48').split(',').map((s) => `/f/${s}MB`);
const flags = (arg('flags', '') || '').split(' ').filter(Boolean);
const profile = arg('profile', '/home/ubuntu/work/signature-studio-develop/lift/out-cache1/work/chrome-probe-profile');
const label = arg('label', flags.join(' ') || 'default');
const keep = process.argv.includes('--keep-profile');
if (!keep) fs.rmSync(profile, { recursive: true, force: true });
const base = `http://127.0.0.1:${port}`;
await fetch(base + '/__clear');

const chrome = await launchChrome({ profile, url: 'about:blank', extraArgs: ['--no-proxy-server', '--disable-background-networking', '--disable-component-update', '--disable-sync', '--no-pings', ...flags] });
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
try {
  const incognito = process.argv.includes('--incognito-context');
  const ctx = incognito ? (await cdp.send('Target.createBrowserContext', { disposeOnDetach: false })).browserContextId : undefined;
  const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', ...(ctx ? { browserContextId: ctx } : {}) });
  const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
  const S = (m, p) => cdp.send(m, p, sessionId);
  await S('Page.enable'); await S('Runtime.enable');
  const ev = async (expression) => { const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 500)); return r.result.value; };
  const result = [];
  for (let run = 1; run <= 3; run++) {
    await S('Page.navigate', { url: base + '/p.html?run=' + run });
    await sleep(500);
    const entries = await ev(`(async () => {
      const urls = ${JSON.stringify(urls)};
      const out = [];
      for (const u of urls) {
        const t0 = performance.now();
        const r = await fetch(u);
        const b = await r.arrayBuffer();
        out.push({ u, status: r.status, len: b.byteLength, ms: performance.now() - t0 });
      }
      await new Promise((r) => setTimeout(r, 300));
      const rt = performance.getEntriesByType('resource').filter((e) => urls.some((u) => e.name.endsWith(u)));
      return out.map((o) => { const e = rt.find((x) => x.name.endsWith(o.u)); return { ...o, transfer: e && e.transferSize, enc: e && e.encodedBodySize, dec: e && e.decodedBodySize, delivery: e && e.deliveryType, proto: e && e.nextHopProtocol }; });
    })()`);
    result.push(entries);
    await sleep(1500); // let the cache backend write
  }
  const log = await (await fetch(base + '/__log')).json();
  const rows = [];
  for (let i = 0; i < urls.length; i++) {
    const row = { url: urls[i] };
    for (let run = 1; run <= 3; run++) {
      const e = result[run - 1][i];
      row['r' + run] = `${e.transfer}B/${e.delivery || '-'}`;
    }
    const mine = log.filter((l) => l.path === urls[i]);
    row.server = mine.map((l) => l.status + (l.inm ? '(inm)' : '')).join(',');
    rows.push(row);
  }
  const lines = [`## Chrome ${chrome.version.Browser}, ${process.argv.includes('--incognito-context') ? 'CDP Target.createBrowserContext (an off-the-record context), ' : 'default browser context, '}flags: ${flags.join(' ') || '(none)'}, profile ${profile}`, '', '| URL | run 1 transferSize/deliveryType | run 2 | run 3 | server responses (in order) |', '|---|---|---|---|---|'];
  for (const r of rows) lines.push(`| ${r.url} | ${r.r1} | ${r.r2} | ${r.r3} | ${r.server} |`);
  const out = arg('out', '');
  console.log(lines.join('\n'));
  if (out) fs.appendFileSync(out, lines.join('\n') + '\n\n');
} finally {
  await shutdown(chrome, cdp).catch(() => {});
}
