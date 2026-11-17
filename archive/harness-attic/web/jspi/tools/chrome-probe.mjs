import { launchChrome, Cdp, shutdown, sleep } from '../cdp.mjs';
import path from 'node:path';
const t0 = Date.now();
const chrome = await launchChrome({ profile: path.resolve('chrome-profile/probe') });
console.log('started', Date.now() - t0, 'ms', chrome.version.Browser);
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const T = (p, n) => Promise.race([p, sleep(15000).then(() => { throw new Error('timeout ' + n); })]);
try {
  const { targetId } = await T(cdp.send('Target.createTarget', { url: 'about:blank' }), 'createTarget');
  console.log('target', targetId, Date.now() - t0);
  const { sessionId } = await T(cdp.send('Target.attachToTarget', { targetId, flatten: true }), 'attach');
  console.log('attached', Date.now() - t0);
  const r = await T(cdp.send('Runtime.evaluate', { expression: '1+1', returnByValue: true }, sessionId), 'evaluate');
  console.log('eval', JSON.stringify(r.result), Date.now() - t0);
  const n = await T(cdp.send('Page.navigate', { url: 'data:text/html,<p>hi</p>' }, sessionId), 'navigate-data');
  console.log('navigated data', JSON.stringify(n), Date.now() - t0);
} catch (e) { console.log('FAILED', String(e)); }
console.log('chrome stderr:', chrome.getStderr().slice(0, 1500));
await shutdown(chrome, cdp);
console.log('shut down', Date.now() - t0);
