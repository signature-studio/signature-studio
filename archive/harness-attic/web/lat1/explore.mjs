// Finds, on the real engine's main window, targets for the six input kinds. Prints per action the joined timeline of the first input and the draws that changed pixels.
import path from 'node:path';
import fs from 'node:fs';
import { openServer, openSession, join, stages, ms1, sleep, say, load1, ROOT } from './lib.mjs';

const out = `${ROOT}/lift/out-lat1/runs/explore`;
fs.mkdirSync(out, { recursive: true });
const srv = await openServer(out);
const S = await openSession(out, 'explore');
const q = process.argv[2] ?? '';
const fromImg = (ix, iy) => { const r = { x: (ix / 0.8333 - S.rect.x) * 1920 / S.rect.width, y: (iy / 0.8333 - S.rect.y) * 1080 / S.rect.height }; return [Math.round(r.x), Math.round(r.y)]; };
try {
  await S.open(`http://127.0.0.1:${srv.port}/${q}`);
  await S.waitIdle();
  await S.refreshRect();
  S.rect = await S.refreshRect();
  say('wizard closed: ' + JSON.stringify(await S.closeWizard()));
  await sleep(2000);
  await S.settle(10000, 500);
  const run = async (name, fn) => {
    await S.reset();
    await fn();
    await S.settle(4000, 400);
    const c = await S.collect();
    const j = join(c);
    const chg = c.puts.filter((p) => p.changed > 0).map((p) => `lid${p.lid}[${p.x},${p.y},${p.w}x${p.h}]chg${p.changed}`);
    say(`== ${name}: ${j.length} inputs, ${c.puts.length} draws, ${chg.length} changing`);
    for (const o of j.slice(0, 4)) say('   ' + o.desc + ' -> ' + (o.first ? `lid${o.first.lid} ${o.first.rect.join(',')} chg ${o.first.changedPx}  ` + JSON.stringify(Object.fromEntries(Object.entries(stages(o)).map(([k, v]) => [k.split('_')[0], ms1(v)]))) : 'NO CHANGING DRAW'));
    say('   changing draws: ' + chg.slice(0, 8).join(' ') + (chg.length > 8 ? ` ... (${chg.length})` : ''));
  };
  const hover = async (name, ix, iy) => { const [x, y] = fromImg(ix, iy); await run('hover ' + name, async () => { await S.move(x - 40, y + 30); await sleep(300); await S.reset(); await S.move(x, y); }); };
  await hover('play', 305, 190);
  await hover('stop', 330, 190);
  await hover('file', 28, 190);
  await hover('step1', 505, 293);
  await hover('channel button', 443, 293);
  await hover('mixer fader1', 446, 833);
  await hover('toolbar icon', 275, 225);
  await hover('playlist cell', 1250, 600);
  const click = async (name, ix, iy, btn = 'left') => { const [x, y] = fromImg(ix, iy); await S.move(x, y); await sleep(300); await run('click ' + name, async () => { await S.down(x, y, btn); await sleep(80); await S.up(x, y, btn); }); };
  await click('step1', 505, 293);
  await click('step1 again', 505, 293);
  await click('file', 28, 190);
  fs.writeFileSync(path.join(out, 'file-menu.png'), await S.page.shot(0, 0, 2400, 1300));
  await run('Escape', async () => { await S.tap('Escape'); });
  await click('right-click channel button', 443, 293, 'right');
  fs.writeFileSync(path.join(out, 'rclick.png'), await S.page.shot(0, 0, 2400, 1300));
  await run('Escape', async () => { await S.tap('Escape'); });
  await run('key F8', async () => { await S.tap('F8'); });
  await sleep(1500);
  fs.writeFileSync(path.join(out, 'f8.png'), await S.page.shot(0, 0, 2400, 1300));
  await run('Escape after F8', async () => { await S.tap('Escape'); });
  say('windows: ' + await S.page.ev("JSON.stringify((window.__app.appView?.windows ?? []).filter(w=>w.visible).map(w=>[w.lid,w.className,w.enabled]))"));
  await run('key Right', async () => { await S.tap('Right'); });
  await run('wheel playlist', async () => { const [x, y] = fromImg(1250, 600); await S.move(x, y); await sleep(300); await S.reset(); await S.wheel(x, y, 120); });
  await run('wheel playlist -', async () => { const [x, y] = fromImg(1250, 600); await S.wheel(x, y, -120); });
  await run('drag mixer fader1', async () => { const [x, y] = fromImg(446, 833); await S.move(x, y); await sleep(200); await S.reset(); await S.down(x, y); for (let i = 1; i <= 12; i++) { await S.dragMove(x, y - i * 6); await sleep(16); } await S.up(x, y - 72); });
  fs.writeFileSync(path.join(out, 'fader.png'), await S.page.shot(0, 0, 2400, 1300));
  await run('drag fader back', async () => { const [x, y] = fromImg(446, 833 - 72 * 0.8333); await S.move(x, y); await sleep(200); await S.reset(); await S.down(x, y); for (let i = 1; i <= 12; i++) { await S.dragMove(x, y + i * 6); await sleep(16); } await S.up(x, y + 72); });
  await run('drag knob (channel rack)', async () => { const [x, y] = fromImg(349, 293); await S.move(x, y); await sleep(200); await S.reset(); await S.down(x, y); for (let i = 1; i <= 12; i++) { await S.dragMove(x, y - i * 5); await sleep(16); } await S.up(x, y - 60); });
  await run('Space start', async () => { await S.tap('Space'); });
  await sleep(2500);
  await run('Space stop', async () => { await S.tap('Space'); });
} finally {
  await S.page.close().catch(() => {});
  try { await S.browser.close(); } catch (e) { say('close: ' + String(e).slice(0, 100)); }
  await srv.stop();
}
