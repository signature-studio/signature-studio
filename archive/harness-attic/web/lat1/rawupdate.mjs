// lat1: how long does the browser hold an isolated pointer move before the page's listener runs, for 'pointermove' (aligned to the frame) and 'pointerrawupdate' (not aligned; Chrome)? A blank page, the same CDP injection as run.mjs. Also: an idle page vs a page whose main thread draws a canvas at 60 Hz.
import fs from 'node:fs';
import { ROOT, sleep, say , SRC } from './lib.mjs';
const { openBrowser } = await import(`${SRC}/test/e2e/browser.mjs`);
const out = `${ROOT}/lift/out-lat1/runs/rawupdate`;
fs.mkdirSync(out, { recursive: true });
const browser = await openBrowser(out, 'raw');
const page = await browser.newPage({ width: 1200, height: 800 });
const html = `<!doctype html><body style="margin:0"><canvas id=c width=400 height=200></canvas><script>
const O = performance.timeOrigin; const L = window.L = { pm: [], raw: [], me: [] };
addEventListener('pointermove', (e) => L.pm.push([e.timeStamp, performance.now()]), true);
addEventListener('pointerrawupdate', (e) => L.raw.push([e.timeStamp, performance.now()]), true);
addEventListener('mousemove', (e) => L.me.push([e.timeStamp, performance.now()]), true);
window.busy = false; const ctx = document.getElementById('c').getContext('2d'); let k = 0;
// no rAF loop: an idle page has no frame scheduled; 'busy' draws into the canvas from a 16 ms timer like the app's Worker frames do (decision 0194: the idle repaint of the default entry)\nsetInterval(() => { if (window.busy) { ctx.fillStyle = 'rgb(' + (k++ % 255) + ',0,0)'; ctx.fillRect(0, 0, 400, 200); } }, 16);
</script>`;
try {
  await page.goto('data:text/html,' + encodeURIComponent(html));
  await sleep(800);
  const q = (v, p) => { v = v.slice().sort((a, b) => a - b); return v[Math.min(v.length - 1, Math.ceil(p / 100 * v.length) - 1)]; };
  for (const busy of [false, true]) {
    await page.ev(`window.busy = ${busy}; L.pm.length = L.raw.length = L.me.length = 0; 1`);
    await sleep(300);
    for (let i = 0; i < 60; i++) {
      await page.S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 100 + (i % 2) * 40, y: 100, button: 'none', buttons: 0, pointerType: 'mouse' });
      await sleep(137 + (i % 7) * 11);   // irregular spacing, so that the phase against the 16.7 ms frame is spread
    }
    await sleep(300);
    const r = JSON.parse(await page.ev('JSON.stringify(L)'));
    const d = (a) => a.map((x) => x[1] - x[0]);
    say(`${busy ? 'canvas redrawn every 16 ms from a timer' : 'idle page'}: pointermove n ${r.pm.length} delay (timeStamp -> listener) p50 ${q(d(r.pm), 50).toFixed(1)} p95 ${q(d(r.pm), 95).toFixed(1)} max ${q(d(r.pm), 100).toFixed(1)} ms | pointerrawupdate n ${r.raw.length} p50 ${r.raw.length ? q(d(r.raw), 50).toFixed(1) : '-'} p95 ${r.raw.length ? q(d(r.raw), 95).toFixed(1) : '-'} max ${r.raw.length ? q(d(r.raw), 100).toFixed(1) : '-'} | mousemove n ${r.me.length} p50 ${q(d(r.me), 50).toFixed(1)}`);
    fs.writeFileSync(`${out}/raw-${busy ? 'busy' : 'idle'}.json`, JSON.stringify(r));
  }
} finally { try { await page.close(); await browser.close(); } catch {} }
