// Decision 0220, jobs 1(c) and 4: the REAL page (default entrance, the real engine) in Chrome 156 / Firefox 158 headless with the browser's cache ENABLED and a profile that is kept between loads.
// One cycle = a cold load (fresh profile) and then `--warm` loads, each in a freshly started browser with the SAME profile (so only what the browser stored on disk is left).
// For each load: the page's timeline (window.__app.timeline), PerformanceResourceTiming of the four assets, the page's `assets` beacon block if the build has it, and the access-log lines the server wrote.
// Usage: node test/e2e/cache-real-run.mjs --browser chrome|firefox --port 4962 --access LOG.jsonl --cycles 5 --warm 1 [--query "cache=0"] [--until idle|worker_ready] [--flags "..."] [--out FILE.jsonl] [--label X]
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { Cdp, descendants, launchChrome, shutdown, sleep } from './cdp.mjs';
import { openFirefox } from './firefox-driver.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const which = opt('browser', 'chrome');
const port = Number(opt('port', '4962'));
const access = opt('access', '');
const cycles = Number(opt('cycles', '5'));
const warm = Number(opt('warm', '1'));
const query = opt('query', '');
const until = opt('until', 'idle');
const flags = opt('flags', '').split(' ').filter(Boolean);
const out = opt('out', '');
const label = opt('label', which);
const waitS = Number(opt('wait-s', '150'));
const settle = argv.includes('--settle');
const WORK = '/home/ubuntu/work/signature-studio-develop/lift/out-cache1/work';
// --variants "label|port|query|accessLog;label|port|query|accessLog": the cycles alternate between the variants (cycle c runs variant 0, 1, 2 ... in turn, each with its own cold + warm loads)
const variants = opt('variants', '') ? opt('variants').split(';').map((v) => { const [l, p, q, a] = v.split('|'); return { label: l, port: Number(p), query: q ?? '', access: a ?? '' }; }) : [{ label, port, query, access }];

const PROBE = `(() => {
  const a = window.__app;
  const res = performance.getEntriesByType('resource').filter((e) => /\\/assets\\//.test(e.name)).map((e) => ({ n: e.name.split('/').pop(), start: Math.round(e.startTime), resStart: Math.round(e.responseStart), resEnd: Math.round(e.responseEnd), transfer: e.transferSize, enc: e.encodedBodySize, dec: e.decodedBodySize, delivery: e.deliveryType || '', proto: e.nextHopProtocol }));
  return JSON.stringify({ state: a && a.state, tl: a && a.timeline, err: a && a.error, res, load: null });
})()`;

function accessLines(since, access) {
  if (!access || !fs.existsSync(access)) return [];
  return fs.readFileSync(access, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l)).filter((r) => r.t >= since && /\/assets\//.test(r.url));
}

async function openChrome(profile) {
  const chrome = await launchChrome({ profile, url: 'about:blank', extraArgs: ['--no-proxy-server', '--disable-background-networking', '--disable-component-update', '--disable-sync', '--no-pings', '--disable-default-apps', '--disable-extensions', ...flags] });
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
  await cdp.opened;
  const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
  const S = (m, p) => cdp.send(m, p, sessionId);
  await S('Page.enable'); await S('Runtime.enable');
  await S('Emulation.setDeviceMetricsOverride', { width: 1930, height: 1092, deviceScaleFactor: 1, mobile: false });
  return {
    version: chrome.version.Browser,
    goto: (u) => S('Page.navigate', { url: u }),
    ev: async (expression) => { const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 300)); return r.result.value; },
    // total resident set of the Chrome process tree, MiB (sampled by the poll loop: the peak of the samples is reported)
    rss: () => { let t = 0; for (const pid of descendants(chrome.proc.pid)) { try { t += Number(fs.readFileSync(`/proc/${pid}/statm`, 'utf8').split(' ')[1]) * 4096; } catch { /* gone */ } } return t / 1048576; },
    close: () => shutdown(chrome, cdp),
  };
}

async function openFox(profile) {
  const work = path.join(WORK, 'ff-real-work');
  fs.mkdirSync(work, { recursive: true });
  const ff = await openFirefox({ work, downloadDir: path.join(work, 'dl'), dataDir: work, extraArgs: ['-profile', profile], scriptTimeoutMs: 120000 });
  return { version: ff.version, goto: (u) => ff.goto(u), ev: (e) => ff.ev(e), close: () => ff.close() };
}

async function oneLoad(profile, kind, v) {
  const url = `http://127.0.0.1:${v.port}/` + (v.query ? '?' + v.query : '');
  const b = which === 'chrome' ? await openChrome(profile) : await openFox(profile);
  const t0 = Date.now();
  const rec = { kind, label: v.label, browser: which, version: b.version, startedAt: new Date(t0).toISOString(), load1: os.loadavg()[0] };
  try {
    await b.goto(url);
    const deadline = Date.now() + waitS * 1000;
    let j = null;
    for (;;) {
      await sleep(1000);
      const s = await b.ev(PROBE).catch(() => null);
      if (b.rss) rec.rssPeakMiB = Math.max(rec.rssPeakMiB ?? 0, Math.round(b.rss()));
      if (s) {
        j = JSON.parse(s);
        const tl = j.tl || {};
        if (j.state === 'error' || j.state === 'exited') break;
        if (until === 'idle' ? typeof tl.idle === 'number' : typeof tl.worker_ready === 'number') break;
      }
      if (Date.now() > deadline) { rec.timeout = true; break; }
    }
    if (j) { rec.state = j.state; rec.err = j.err; rec.tl = j.tl; rec.res = j.res; }
    if (settle) {
      for (let i = 0; i < 90; i++) {
        const w = await b.ev('(() => { try { const r = window.__app.assetsReport(); return r ? r.items.filter((x) => x.store === "writing").length : 0; } catch (e) { return 0; } })()').catch(() => 0);
        if (!w) break;
        await sleep(1000);
      }
      rec.settled = true;
    }
    rec.opfs = await b.ev(`(async () => { try { let d = await navigator.storage.getDirectory(); for (const n of ['fl', 'cache', 'assets']) d = await d.getDirectoryHandle(n); const o = []; for await (const [n, h] of d.entries()) { const f = await h.getFile(); o.push(n.slice(0, 12) + ':' + f.size); } return o.join(' '); } catch (e) { return 'no store: ' + e.name; } })()`).catch(() => null);
    rec.assetsBlock = await b.ev('(() => { try { return JSON.stringify(window.__app && window.__app.assetsReport ? window.__app.assetsReport() : null); } catch (e) { return null; } })()').catch(() => null);
    await sleep(500);
    rec.server = accessLines(t0, v.access).map((r) => ({ url: r.url.split('/').pop(), status: r.status, bytes: r.bytes, inm: !!r.inm, ms: r.ms }));
    rec.load2 = os.loadavg()[0];
  } finally { await b.close().catch(() => {}); }
  return rec;
}

const all = [];
for (let c = 0; c < cycles; c++) {
  for (const v of variants) {
    const profile = path.join(WORK, `real-profile-${which}`);
    fs.rmSync(profile, { recursive: true, force: true });
    fs.mkdirSync(profile, { recursive: true });
    for (let w = 0; w <= warm; w++) {
      const rec = await oneLoad(profile, w === 0 ? 'cold' : 'warm' + w, v);
      rec.cycle = c;
      all.push(rec);
      const dl = rec.tl && rec.tl.download_done !== undefined ? (rec.tl.download_done - rec.tl.download_start).toFixed(0) : '?';
      console.log(`${v.label} cycle ${c} ${rec.kind}: first idle ${rec.tl?.idle?.toFixed(0) ?? '-'} ms, download_start ${rec.tl?.download_start?.toFixed(0)} download_done ${rec.tl?.download_done?.toFixed(0)} (${dl} ms) compile_done ${rec.tl?.compile_done?.toFixed(0)}; server ${rec.server.map((s) => s.url.slice(0, 12) + ':' + s.status).join(' ')}; load ${rec.load1.toFixed(1)}`);
      if (out) fs.appendFileSync(out, JSON.stringify(rec) + '\n');
    }
  }
}
