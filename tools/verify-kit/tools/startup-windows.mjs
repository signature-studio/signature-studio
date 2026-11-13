// startup-windows.mjs --url URL --app APPDIR --out DIR [--for-s 25]: Chrome; from the page load on, the compositor's window list (deskDumpBase64 peek, the windows drawn) every ~300 ms until the first idle + 8 s: which windows appear when, with their rects. E9 check item 6 (lid 8, the logo window, may be 326x460 = time driven). Read only.
import fs from 'node:fs'; import path from 'node:path';
const argv = process.argv.slice(2); const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const URL0 = opt('url'), APP = path.resolve(opt('app')), OUT = path.resolve(opt('out')), FOR = Number(opt('for-s', '25'));
fs.mkdirSync(OUT, { recursive: true });
const { openBrowser } = await import('../lib/browser.mjs');
const br = await openBrowser(OUT, 'startup'); const page = await br.newPage({ width: 2260, height: 1116 });
const t0 = Date.now(); await page.goto(URL0);
const seen = new Map(); const log = []; let idleAt = null;
const expr = `(async () => { const a = window.__app; if (!a || !a.deskDumpBase64) return JSON.stringify({ w: '', idle: null }); const d = await a.deskDumpBase64(true); return JSON.stringify({ w: d.windows, idle: a.timeline && a.timeline.idle }); })()`;
while ((Date.now() - t0) / 1000 < FOR) {
  const t = Date.now() - t0; let r = null;
  try { r = JSON.parse(await page.ev(expr)); } catch { /* page not ready */ }
  if (r) {
    if (r.idle != null && idleAt == null) idleAt = t;
    for (const l of r.w.split('\n').filter(Boolean)) {
      const m = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+).*class "([^"]*)" title "([^"]*)"/.exec(l); if (!m) continue;
      const key = `${m[1]} ${m[6]} @${m[2]},${m[3]} ${m[4]}x${m[5]}`; const e = seen.get(key);
      if (!e) { seen.set(key, { first: t, last: t, n: 1 }); log.push({ t, appeared: key, title: m[7] }); } else { e.last = t; e.n++; }
    }
    if (idleAt != null && t - idleAt > 8000) break;
  }
  await new Promise((s) => setTimeout(s, 250));
}
const out = { url: URL0, firstIdleMs: idleAt, states: [...seen.entries()].map(([k, v]) => ({ window: k, firstSeenMs: v.first, lastSeenMs: v.last, samples: v.n })), log };
fs.writeFileSync(path.join(OUT, 'startup-windows.json'), JSON.stringify(out, null, 1)); console.log(JSON.stringify(out.states));
await page.close().catch(() => {}); await br.close(); process.exit(0);
