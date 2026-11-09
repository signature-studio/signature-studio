import fs from 'node:fs';
import { openServer, openSession, sleep, say, ROOT } from './lib.mjs';
const out = `${ROOT}/lift/out-lat1/runs/probe`;
fs.mkdirSync(out, { recursive: true });
const srv = await openServer(out);
const S = await openSession(out, 'probe');
const fromImg = (ix, iy) => [Math.round((ix / 0.8333 - S.rect.x) * 1920 / S.rect.width), Math.round((iy / 0.8333 - S.rect.y) * 1080 / S.rect.height)];
const st = async (tag) => say(tag + ' ' + await S.page.ev("JSON.stringify({ state: window.__app.state, exit: window.__app.exit, err: window.__app.error, sent: window.__app.latency.sentTotal, proc: window.__app.latency.processedTotal, pend: window.__app.latency.pending, status: window.__app.engineStatus().text })"));
try {
  await S.open(`http://127.0.0.1:${srv.port}/`);
  await S.waitIdle(); await S.refreshRect(); await S.closeWizard(); await sleep(2000);
  const [x, y] = fromImg(1250, 600);
  await S.move(x, y); await sleep(500); await st('after move');
  await S.wheel(x, y, 120); await sleep(1500); await st('after wheel');
  fs.writeFileSync(`${out}/after-wheel.png`, await S.page.shot(0, 0, 2400, 1300));
  const lines = JSON.parse(await S.page.ev('JSON.stringify(window.__app.engineLog().slice(-6))'));
  say(lines.join(' | ').slice(0, 800));
  await S.move(x + 5, y); await sleep(500); await st('after 2nd move');
  say(await S.page.ev("document.getElementById('exit').textContent.slice(0,600)"));
} finally { try { await S.page.close(); await S.browser.close(); } catch {} await srv.stop(); }
