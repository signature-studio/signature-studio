// Finds, on the real engine's main window, targets for the six input kinds. Prints per action the joined timeline of the first input and the draws that changed pixels.
import path from 'node:path';
import fs from 'node:fs';
import { openServer, openSession, join, stages, ms1, sleep, say, load1, ROOT } from './lib.mjs';

const out = `${ROOT}/lift/out-lat1/runs/explore`;
fs.mkdirSync(out, { recursive: true });
const srv = await openServer(out);
const S = await openSession(out, 'explore2');
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
  await run('drag mixer fader1', async () => { const [x, y] = fromImg(446, 833); await S.move(x, y); await sleep(200); await S.reset(); await S.down(x, y); for (let i = 1; i <= 12; i++) { await S.dragMove(x, y - i * 6); await sleep(16); } await S.up(x, y - 72); });
  fs.writeFileSync(path.join(out, 'fader.png'), await S.page.shot(0, 0, 2400, 1300));
  await run('drag fader back', async () => { const [x, y] = fromImg(446, 833 - 72 * 0.8333); await S.move(x, y); await sleep(200); await S.reset(); await S.down(x, y); for (let i = 1; i <= 12; i++) { await S.dragMove(x, y + i * 6); await sleep(16); } await S.up(x, y + 72); });
  await run('drag knob (channel rack)', async () => { const [x, y] = fromImg(349, 293); await S.move(x, y); await sleep(200); await S.reset(); await S.down(x, y); for (let i = 1; i <= 12; i++) { await S.dragMove(x, y - i * 5); await sleep(16); } await S.up(x, y - 60); });
  await run('Space start', async () => { await S.tap('Space'); });
  await sleep(2500);
  await run('Space stop', async () => { await S.tap('Space'); });
  await run('Right', async () => { await S.tap('Right'); });
  await S.tap('Space'); await sleep(3000);
  await run('PLAY: hover file', async () => { const [x, y] = fromImg(28, 190); await S.move(x - 40, y + 30); await sleep(300); await S.reset(); await S.move(x, y); });
  await run('PLAY: click step1', async () => { const [x, y] = fromImg(505, 293); await S.move(x, y); await sleep(300); await S.reset(); await S.down(x, y); await sleep(80); await S.up(x, y); });
  await run('PLAY: Right', async () => { await S.tap('Right'); });
  fs.writeFileSync(path.join(out, 'play.png'), await S.page.shot(0, 0, 2400, 1300));
  await S.tap('Space'); await sleep(2000);
} finally {
  await S.page.close().catch(() => {});
  try { await S.browser.close(); } catch (e) { say('close: ' + String(e).slice(0, 100)); }
  await srv.stop();
}
