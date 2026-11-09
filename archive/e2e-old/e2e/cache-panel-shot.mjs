// Decision 0220: a screenshot of the Download stage and of the Performance panel's start-up asset rows after a WARM load (the stored copy), in Chrome 156 headless. A look at what the page tells, not a test.
// usage: node test/e2e/cache-panel-shot.mjs --port 4964 --out DIR
import fs from 'node:fs';
import path from 'node:path';
import { Cdp, launchChrome, shutdown, sleep } from './cdp.mjs';
const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const port = Number(opt('port', '4964')), out = opt('out', '.');
const profile = '/home/ubuntu/work/signature-studio-develop/lift/out-cache1/work/shot-profile';
fs.rmSync(profile, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });
for (const n of [1, 2]) {
  const chrome = await launchChrome({ profile, extraArgs: ['--no-proxy-server', '--disable-background-networking'] });
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl); await cdp.opened;
  try {
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (m, p) => cdp.send(m, p, sessionId);
    await S('Page.enable'); await S('Runtime.enable');
    await S('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false });
    await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    const ev = async (expression) => { const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result.value; };
    for (let i = 0; i < 90; i++) { await sleep(1000); const s = await ev('(() => { const a = window.__app; return a && a.assetsReport && a.assetsReport() && a.timeline.worker_ready ? JSON.stringify(a.assetsReport().items.map((x) => x.store)) : null; })()'); if (s && !s.includes('writing')) break; }
    await sleep(3000);
    await ev("document.getElementById('perf-panel').open = true; true");
    await sleep(1500);
    const box = JSON.parse(await ev("JSON.stringify((() => { const r = document.getElementById('perf-panel').getBoundingClientRect(); const p = document.getElementById('phases').getBoundingClientRect(); return { panel: { x: r.x, y: r.y + window.scrollY, w: r.width, h: Math.min(r.height, 900) }, phases: { x: p.x, y: p.y + window.scrollY, w: p.width, h: p.height } }; })())"));
    const shot = async (b, name) => { const r = await S('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip: { x: b.x, y: b.y, width: Math.max(1, Math.ceil(b.w)), height: Math.max(1, Math.ceil(b.h)), scale: 1 } }); fs.writeFileSync(path.join(out, name), Buffer.from(r.data, 'base64')); };
    await shot(box.phases, `load${n}-phases.png`);
    await shot(box.panel, `load${n}-perf-panel.png`);
    const rows = await ev("JSON.stringify([...document.querySelectorAll('#perf-body tr, #perf-body li, #perf-body .row')].map((e) => e.textContent.trim().slice(0, 300)).filter((t) => /asset/i.test(t)))");
    fs.writeFileSync(path.join(out, `load${n}-asset-rows.json`), rows);
  } finally { await shutdown(chrome, cdp); }
}
console.log('done');
