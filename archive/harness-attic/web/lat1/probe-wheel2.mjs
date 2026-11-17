import fs from 'node:fs';
import { openServer, openSession, sleep, say, join, stages, ms1, ROOT } from './lib.mjs';
const out = `${ROOT}/lift/out-lat1/runs/probe`;
fs.mkdirSync(out, { recursive: true });
const srv = await openServer(out);
const cands = [['tempo', 395, 190], ['mixer fader1', 446, 833], ['channel knob', 349, 293], ['mixer track name', 437, 720], ['channel rack area', 520, 450]];
for (const [name, ix, iy] of cands) {
  const S = await openSession(out, 'probe');
  try {
    await S.open(`http://127.0.0.1:${srv.port}/`);
    await S.waitIdle(); await S.refreshRect(); await S.closeWizard(); await sleep(2000);
    const [x, y] = [Math.round((ix / 0.8333 - S.rect.x) * 1920 / S.rect.width), Math.round((iy / 0.8333 - S.rect.y) * 1080 / S.rect.height)];
    await S.move(x, y); await sleep(600); await S.settle(4000, 400); await S.reset();
    await S.wheel(x, y, 120); await sleep(1500); await S.settle(3000, 400);
    const c = await S.collect();
    const st = JSON.parse(await S.page.ev("JSON.stringify({ state: window.__app.state, exit: window.__app.exit?.code })"));
    const j = join(c);
    say(`wheel @${name}: state ${st.state} exit ${st.exit}; inputs ${j.length}; changing draws ${c.puts.filter(p=>p.changed>0).map(p=>`lid${p.lid}[${p.x},${p.y},${p.w}x${p.h}]${p.changed}`).slice(0,5).join(' ')}`);
    for (const o of j) say('    ' + o.desc + ' ' + (o.first ? JSON.stringify(Object.fromEntries(Object.entries(stages(o)).map(([k, v]) => [k.split('_')[0], ms1(v)]))) : 'no draw'));
  } finally { try { await S.page.close(); await S.browser.close(); } catch {} }
}
await srv.stop();
