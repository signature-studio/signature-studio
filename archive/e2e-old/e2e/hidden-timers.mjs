// Decision 0170: does Chrome throttle the timers of a dedicated Worker when its page is in a background tab? (The idle driver waits for future deadlines with setTimeout and chains turns with a MessageChannel.)
// Two pages in the SAME browser context of a throw-away headless Chrome (about:blank only, no network): page A starts a Worker that runs a chain of setTimeout(10 ms) and a chain of MessageChannel turns and counts both, and
// the page itself runs a setTimeout(10 ms) chain. Then page B is brought to the front (Target.activateTarget), A's document.visibilityState is read, and the counts are read again after `--hidden-s` seconds.
// Prints the rates in each phase. If A never becomes hidden in this headless Chrome, it says so: then nothing is measured about background tabs.
// usage: node test/e2e/hidden-timers.mjs [--visible-s 5] [--hidden-s 15] [--frozen-s 0] [--out FILE]
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { Cdp, launchChrome, shutdown, sleep } from './cdp.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
const visibleS = Number(opt('visible-s', '5')), hiddenS = Number(opt('hidden-s', '15')), frozenS = Number(opt('frozen-s', '0'));
const outFile = opt('out', '');
const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'opencode', 'clk1-hidden-')) ;
const chrome = await launchChrome({ profile, extraArgs: ['--disable-background-networking', '--no-proxy-server'] });
const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
await cdp.opened;
const result = { chrome: chrome.version.Browser, phases: [] };
const say = (s) => console.log(`${new Date().toISOString()} ${s}`);
try {
  const mk = async () => {
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (m, p) => cdp.send(m, p, sessionId);
    const ev = async (expression) => { const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 500)); return r.result.value; };
    return { targetId, S, ev };
  };
  const A = await mk();
  const B = await mk();
  await A.ev(`(() => {
    const src = \`
      let timers = 0, turns = 0, last = performance.now(), maxGap = 0, gaps = 0;
      const tick = () => { timers++; const n = performance.now(); const g = n - last; last = n; if (g > maxGap) maxGap = g; setTimeout(tick, 10); };
      setTimeout(tick, 10);
      const mc = new MessageChannel();
      mc.port1.onmessage = () => { turns++; setTimeout(() => mc.port2.postMessage(0), 0); };
      mc.port2.postMessage(0);
      onmessage = () => postMessage({ timers, turns, maxGap, now: performance.now() });
    \`;
    window.__w = new Worker(URL.createObjectURL(new Blob([src], { type: 'text/javascript' })));
    window.__wr = null;
    window.__w.onmessage = (e) => { window.__wr = e.data; };
    window.__page = { timers: 0 };
    const pt = () => { window.__page.timers++; setTimeout(pt, 10); };
    setTimeout(pt, 10);
    return true;
  })()`);
  const snap = async () => {
    await A.ev('window.__wr = null; window.__w.postMessage(0); true');
    for (let i = 0; i < 100; i++) { const w = await A.ev('window.__wr'); if (w) return { t: Date.now(), vis: await A.ev('document.visibilityState'), worker: w, pageTimers: await A.ev('window.__page.timers') }; await sleep(100); }
    return { t: Date.now(), vis: await A.ev('document.visibilityState'), worker: null, pageTimers: null };
  };
  const phase = async (name, seconds, before) => {
    if (before) await before();
    const s0 = await snap();
    await sleep(seconds * 1000);
    const s1 = await snap();
    const dt = (s1.t - s0.t) / 1000;
    const r = { name, visibility: s1.vis, seconds: +dt.toFixed(1), workerTimersPerS: s1.worker && s0.worker ? +((s1.worker.timers - s0.worker.timers) / dt).toFixed(1) : null, workerTurnsPerS: s1.worker && s0.worker ? +((s1.worker.turns - s0.worker.turns) / dt).toFixed(0) : null, pageTimersPerS: s1.pageTimers !== null && s0.pageTimers !== null ? +((s1.pageTimers - s0.pageTimers) / dt).toFixed(1) : null, workerMaxTimerGapMs: s1.worker ? +s1.worker.maxGap.toFixed(0) : null };
    result.phases.push(r);
    say(JSON.stringify(r));
  };
  await phase('A in front', visibleS);
  await phase('B in front (A in the background)', hiddenS, async () => { await cdp.send('Target.activateTarget', { targetId: B.targetId }); await sleep(500); });
  if (frozenS > 0) await phase(`A frozen ${frozenS}s then active (counts after thaw)`, frozenS, async () => { await A.S('Page.setWebLifecycleState', { state: 'frozen' }); setTimeout(() => A.S('Page.setWebLifecycleState', { state: 'active' }).catch(() => {}), frozenS * 1000 - 500); });
  await phase('A in front again', visibleS, async () => { await cdp.send('Target.activateTarget', { targetId: A.targetId }); await sleep(500); });
  const hiddenPhase = result.phases[1];
  result.aBecameHidden = hiddenPhase?.visibility === 'hidden';
  say(result.aBecameHidden ? 'page A was hidden: the rates above are those of a background tab' : 'page A stayed "visible" while B was activated: this headless Chrome does not model a background tab; nothing is measured about background throttling');
} catch (e) {
  result.error = String(e.stack ?? e);
  say('FAILED: ' + result.error);
}
if (outFile) fs.writeFileSync(outFile, JSON.stringify(result, null, 1));
await shutdown(chrome, cdp);
fs.rmSync(profile, { recursive: true, force: true });
