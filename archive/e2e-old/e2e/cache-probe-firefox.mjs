// Decision 0220, job 1(c): does Firefox's HTTP cache keep a response of N MiB? Same probe as cache-probe-chrome.mjs, through geckodriver (Firefox 158 headless), the cache ENABLED.
// Usage: node test/e2e/cache-probe-firefox.mjs --port 8801 --sizes 1,5,6,7,8,12,16,24,32,48 [--pref name=value ...] [--private] [--out FILE]
//   --pref browser.cache.disk.max_entry_size=102400   (repeatable) sets a pref in the throw-away profile
//   --private                                          runs in a private window (-private): Firefox keeps private browsing entries in memory only
import fs from 'node:fs';
import path from 'node:path';
import { openFirefox } from './firefox-driver.mjs';
import { sleep } from '../helpers/proc.mjs';

const args = process.argv.slice(2);
const arg = (n, d) => { const i = args.indexOf('--' + n); return i >= 0 ? args[i + 1] : d; };
const port = Number(arg('port', '8801'));
const urls = arg('urls', '') ? arg('urls').split(',') : arg('sizes', '1,5,6,7,8,12,16,24,32,48').split(',').map((s) => `/f/${s}MB`);
const prefs = {};
for (let i = 0; i < args.length; i++) if (args[i] === '--pref') { const [k, v] = args[i + 1].split('='); prefs[k] = v === 'true' ? true : v === 'false' ? false : /^-?\d+$/.test(v) ? Number(v) : v; }
const priv = args.includes('--private');
if (priv) prefs['browser.privatebrowsing.autostart'] = true;
const base = `http://127.0.0.1:${port}`;
const work = '/home/ubuntu/work/signature-studio-develop/lift/out-cache1/work/ff-probe';
fs.rmSync(work, { recursive: true, force: true });
fs.mkdirSync(work, { recursive: true });
await fetch(base + '/__clear');
const ff = await openFirefox({ work, downloadDir: path.join(work, 'dl'), dataDir: work, extraPrefs: prefs, extraArgs: [] });
try {
  const result = [];
  for (let run = 1; run <= 3; run++) {
    await ff.goto(base + '/p.html?run=' + run);
    await sleep(500);
    const entries = await ff.ev(`(async () => {
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
      return out.map((o) => { const e = rt.find((x) => x.name.endsWith(o.u)); return { ...o, transfer: e && e.transferSize, enc: e && e.encodedBodySize, dec: e && e.decodedBodySize, delivery: e && e.deliveryType }; });
    })()`);
    result.push(entries);
    await sleep(1500);
  }
  const log = await (await fetch(base + '/__log')).json();
  const lines = [`## Firefox ${ff.version}${priv ? ' PRIVATE browsing (browser.privatebrowsing.autostart)' : ''}, prefs: ${JSON.stringify(prefs)}`, '', '| URL | run 1 transferSize/deliveryType | run 2 | run 3 | server responses (in order) |', '|---|---|---|---|---|'];
  for (let i = 0; i < urls.length; i++) {
    const cell = (run) => { const e = result[run][i]; return `${e.transfer}B/${e.delivery || '-'}`; };
    const mine = log.filter((l) => l.path === urls[i]);
    lines.push(`| ${urls[i]} | ${cell(0)} | ${cell(1)} | ${cell(2)} | ${mine.map((l) => l.status + (l.inm ? '(inm)' : '')).join(',')} |`);
  }
  console.log(lines.join('\n'));
  const out = arg('out', '');
  if (out) fs.appendFileSync(out, lines.join('\n') + '\n\n');
} finally { await ff.close(); }
