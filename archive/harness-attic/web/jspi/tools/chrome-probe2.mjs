import { launchChrome, Cdp, shutdown, sleep, descendants } from '../cdp.mjs';
import { startServer } from '../serve.mjs';
import path from 'node:path';
const { server, port } = await startServer({ dist: process.env.WEB_DIST });
server.on('request', (req) => console.log('REQUEST', req.url));
const t0 = Date.now();
const chrome = await launchChrome({ profile: path.resolve(process.env.PROFILE || 'chrome-profile/probe2'), extraArgs: (process.env.XARGS || '').split(' ').filter(Boolean) });
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const T = (p, n) => Promise.race([p, sleep(15000).then(() => { throw new Error('timeout ' + n); })]);
try {
  const { targetId } = await T(cdp.send('Target.createTarget', { url: 'about:blank' }), 'createTarget');
  const { sessionId } = await T(cdp.send('Target.attachToTarget', { targetId, flatten: true }), 'attach');
  cdp.on((m) => { if (m.method && /^(Network|Page)\./.test(m.method)) console.log('EV', m.method, JSON.stringify(m.params).slice(0, 220)); });
  await cdp.send('Network.enable', {}, sessionId); await cdp.send('Page.enable', {}, sessionId);
  for (const u of [process.env.XURL ? process.env.XURL.replace('PORT', port) : `http://127.0.0.1:${port}/index.html`]) {
    const t1 = Date.now();
    const n = await T(cdp.send('Page.navigate', { url: u }, sessionId), 'navigate ' + u);
    console.log('navigated', u, JSON.stringify(n).slice(0, 100), Date.now() - t1, 'ms');
  }
} catch (e) { console.log('FAILED', String(e)); const { execSync } = await import('node:child_process'); console.log(execSync(`ss -tanp 2>/dev/null | grep -E "${server.address().port}" | head`).toString()); }
import('node:fs').then((fs) => { for (const pid of descendants(chrome.proc.pid)) { try { console.log(pid, fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').replace(/\0/g, ' ').slice(0, 160)); } catch {} } });
await sleep(300);
console.log('chrome stderr:', chrome.getStderr().slice(0, 1500));
await shutdown(chrome, cdp);
server.close();
