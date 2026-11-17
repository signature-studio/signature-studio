// lat1: after the pointer rests on a toolbar button, the explore runs showed a column [x0..378] x 971 px changing again and again. How long does it go on and how many changing draws per second is it? SHIPH, default entry.
import fs from 'node:fs';
import { openServer, openSession, sleep, say, ROOT, load1 } from './lib.mjs';
const out = `${ROOT}/lift/out-lat1/runs/strip`;
fs.mkdirSync(out, { recursive: true });
const srv = await openServer(out, 4470);
const S = await openSession(out, 'strip');
const q = process.argv[2] ?? '';
try {
  await S.open(`http://127.0.0.1:${srv.port}/${q}`);
  await S.waitIdle(); await S.refreshRect(); await S.closeWizard(); await sleep(3000);
  await S.refreshRect();
  const pos = (ix, iy) => [Math.round((ix / 0.8333 - S.rect.x) * 1920 / S.rect.width), Math.round((iy / 0.8333 - S.rect.y) * 1080 / S.rect.height)];
  const stretch = async (label, ms, act) => {
    await S.reset();
    const t0 = Date.now();
    if (act) await act();
    await sleep(ms);
    const c = await S.collect();
    const ch = c.puts.filter((p) => p.changed > 0 && p.msg);
    const by = {};
    for (const p of ch) { const k = `lid${p.lid}[${p.x},${p.y},${p.w}x${p.h}]`; by[k] = (by[k] ?? 0) + 1; }
    const top = Object.entries(by).sort((a, b) => b[1] - a[1]).slice(0, 4).map(([k, v]) => `${k}:${v}`).join(' ');
    const sec = (Date.now() - t0) / 1000;
    say(`${label}: ${sec.toFixed(1)} s, ${c.puts.length} draws, ${ch.length} changing (${(ch.length / sec).toFixed(1)}/s), top ${top}, load ${load1().toFixed(1)}`);
    // per second counts of changing draws
    const t1 = c.puts.length ? c.puts[0].tEnd : 0; const per = {};
    for (const p of ch) { const s = Math.floor((p.tEnd - t1) / 1000); per[s] = (per[s] ?? 0) + 1; }
    say('   changing per second: ' + Object.keys(per).map((k) => per[k]).join(' '));
  };
  await stretch('rest (pointer away, no hover)', 6000);
  const play = pos(305, 190), file = pos(28, 190), away = pos(1250, 600);
  await stretch('hover play button, stay 12 s', 12000, async () => { await S.move(play[0], play[1]); });
  await stretch('leave to the playlist, 12 s', 12000, async () => { await S.move(away[0], away[1]); });
  await stretch('hover FILE title, stay 8 s', 8000, async () => { await S.move(file[0], file[1]); });
  await stretch('leave again, 6 s', 6000, async () => { await S.move(away[0], away[1]); });
} finally { try { await S.page.close(); await S.browser.close(); } catch {} await srv.stop(); }
