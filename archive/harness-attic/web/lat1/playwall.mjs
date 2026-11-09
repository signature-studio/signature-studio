// lat1: why does the engine stop (exit 134) when the FILE menu is opened during Space playback? Fresh session per attempt; Space, then every --step-s seconds: click FILE, wait, Escape; stops at the first engine exit.
// Saves the stop message in full, the time since playback started (wall and the engine's virtual clock), the inputs just before, the engine's last log lines.
// usage: node tools/playwall.mjs --config SHIPH|F1xH --attempts 5 --max-play-s 150 --step-s 10 --out DIR [--mode file|none]   (mode none = playback only: does playback alone stop it?)
import fs from 'node:fs';
import path from 'node:path';
import { openServer, openSession, sleep, say, load1, ROOT } from './lib.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const config = opt('config', 'SHIPH');
const attempts = Number(opt('attempts', '5'));
const maxPlayS = Number(opt('max-play-s', '150'));
const stepS = Number(opt('step-s', '10'));
const mode = opt('mode', 'file');
const out = path.resolve(opt('out', `${ROOT}/lift/out-lat1/runs/playwall-${config}`));
const CONFIGS = { SHIPH: '', F1xH: '?bundle=/assets/fl-engine-ship1-F1xH/manifest.json' };
fs.mkdirSync(out, { recursive: true });
const srv = await openServer(out, Number(opt('port', '4450')));
const results = [];
for (let a = 1; a <= attempts; a++) {
  const S = await openSession(out, `pw${a}`);
  const r = { config, attempt: a, mode, loadStart: load1() };
  try {
    await S.open(`http://127.0.0.1:${srv.port}/${CONFIGS[config]}`);
    await S.waitIdle(); await S.refreshRect(); await S.closeWizard(); await sleep(3000);
    const FILE = [18, 19];   // SCREEN coordinates of the FILE title (the first version derived them from CSS positions, which aim wrongly when the page is laid out lower, as with F1xH)
    const virt = async () => JSON.parse(await S.page.ev('JSON.stringify((() => { const a = window.__app.idles; const l = a[a.length - 1]; return l ? { vms: l.now / 10000, n: l.n } : null; })())'));
    const v0 = await virt();
    const t0 = Date.now();
    if (mode !== 'opens') await S.tap('Space', 150);
    r.playStartedVirtualMs = v0 ? v0.vms : null;
    r.clicks = 0;
    let exited = false;
    for (let t = stepS; t <= maxPlayS && !exited; t += stepS) {
      if (mode !== 'opens') await sleep(Math.max(0, t0 + t * 1000 - Date.now()));
      let st = await S.page.ev('window.__app.state');
      if (st !== 'engine-idle') { exited = true; break; }
      if (mode === 'file' || mode === 'opens') {
        await S.refreshRect();
        const F = FILE;
        await S.move(F[0], F[1]); await sleep(150); await S.down(F[0], F[1]); await sleep(200); await S.up(F[0], F[1]);
        r.clicks++;
        await sleep(mode === 'opens' ? 450 : 1200);
        const seen = await S.page.ev("(window.__app.appView?.windows ?? []).some((w) => w.visible && /Popup|Menu/.test(w.className))").catch(() => false);
        if (seen) r.menusSeen = (r.menusSeen ?? 0) + 1;
        st = await S.page.ev('window.__app.state');
        if (st !== 'engine-idle') { exited = true; break; }
        await S.tap('Escape', 150);
        await sleep(mode === 'opens' ? 250 : 400);
      }
      r.lastOkAtPlayS = t;
    }
    const st = await S.page.ev('window.__app.state');
    r.state = st;
    r.wallSincePlayS = +((Date.now() - t0) / 1000).toFixed(1);
    const v1 = await virt();
    r.virtualSincePlayS = v1 && v0 ? +((v1.vms - v0.vms) / 1000).toFixed(1) : null;
    if (st !== 'engine-idle') {
      r.exit = JSON.parse(await S.page.ev('JSON.stringify(window.__app.exit)'));
      r.exitText = await S.page.ev("(document.getElementById('exit')?.textContent ?? '').slice(0, 2500)");
      r.engineLogTail = JSON.parse(await S.page.ev('JSON.stringify(window.__app.engineLog().slice(-14).map((l) => l.slice(0, 400)))'));
      r.lastInputs = JSON.parse(await S.page.ev('JSON.stringify(window.__app.latency.records.slice(-8).map((x) => ({ seq: x.seq, desc: x.desc, tPage: x.tPage, tIdle: x.tIdle })))'));
      try { fs.writeFileSync(path.join(out, `stop-${config}-a${a}.png`), await S.page.shot(0, 0, 2400, 1300)); } catch { /* ignore */ }
    }
    r.mem = await S.page.ev('window.__app.engineMemoryBytes');
  } catch (e) { r.error = String(e.stack ?? e).slice(0, 600); }
  r.loadEnd = load1();
  results.push(r);
  say(`attempt ${a} ${config}: state ${r.state} ${r.error ?? ''} after ${r.wallSincePlayS} s wall (${r.virtualSincePlayS} virtual s), ${r.clicks} FILE clicks; stop: ${(r.exitText ?? '').split('\n')[0]?.slice(0, 160)}; load ${r.loadStart.toFixed(1)}`);
  fs.writeFileSync(path.join(out, 'results.json'), JSON.stringify(results, null, 1));
  try { await S.page.close(); await S.browser.close(); } catch { /* ignore */ }
}
await srv.stop();
