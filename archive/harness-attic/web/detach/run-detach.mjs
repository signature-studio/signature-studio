// node run-detach.mjs [probe|popup|pip|all] [name=...]   Drives the detached-window experiment in a headless Chrome (own temporary profile,
// localhost-only server). Headless Chrome has no window manager: positions / always-on-top / focus between windows are NOT observable here.
// Uses ../cdp.mjs read-only. Writes results to ../../../../docs/investigations/detach-window/ (small JSON). No network except 127.0.0.1.
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { fileURLToPath } from 'node:url';
import { launchChrome, Cdp, sleep, shutdown, descendants } from '../cdp.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const RES = path.resolve(HERE, '../../../../docs/investigations/detach-window');
const PROFILE = process.env.DETACH_PROFILE || '/tmp/opencode/detach/chrome-profile';
const argv = Object.fromEntries(process.argv.slice(2).map((s) => { const i = s.indexOf('='); return i < 0 ? [s, '1'] : [s.slice(0, i), s.slice(i + 1)]; }));
const which = process.argv[2] && !process.argv[2].includes('=') ? process.argv[2] : 'all';
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8' };

function serve(coi) {
  const server = http.createServer((req, res) => {
    const u = new URL(req.url, 'http://x');
    const p = decodeURIComponent(u.pathname).replace(/^\/(coi|coos|coap|plain)\//, '/');
    if (p.includes('..')) { res.writeHead(403).end(); return; }
    const f = path.join(HERE, p === '/' ? 'index.html' : p);
    fs.stat(f, (err, st) => {
      if (err || !st.isFile()) { res.writeHead(404).end('nf'); return; }
      const h = { 'Cache-Control': 'no-store', 'Content-Type': TYPES[path.extname(f)] || 'application/octet-stream' };
      // /coi/: COOP same-origin + COEP require-corp (cross-origin isolated; the popup page gets the same headers, so it is in the same browsing context group)
      // /coos/: the opener has COOP+COEP, but popup.html is served WITHOUT COOP (unsafe-none) -> a different browsing context group (window.opener is severed)
      // /coap/: COOP same-origin-allow-popups + COEP (NOT cross-origin isolated per MDN, but keeps window.opener for unsafe-none popups)
      const isPopupFile = /popup\.html$/.test(f);
      if (/^\/coi\//.test(u.pathname)) { h['Cross-Origin-Opener-Policy'] = 'same-origin'; h['Cross-Origin-Embedder-Policy'] = 'require-corp'; h['Cross-Origin-Resource-Policy'] = 'same-origin'; }
      else if (/^\/coos\//.test(u.pathname)) { if (!isPopupFile) { h['Cross-Origin-Opener-Policy'] = 'same-origin'; h['Cross-Origin-Embedder-Policy'] = 'require-corp'; } h['Cross-Origin-Resource-Policy'] = 'same-origin'; }
      else if (/^\/coap\//.test(u.pathname)) { h['Cross-Origin-Opener-Policy'] = 'same-origin-allow-popups'; h['Cross-Origin-Embedder-Policy'] = 'require-corp'; h['Cross-Origin-Resource-Policy'] = 'same-origin'; }
      res.writeHead(200, h); fs.createReadStream(f).pipe(res);
    });
  });
  return new Promise((r) => server.listen(0, '127.0.0.1', () => r({ server, port: server.address().port })));
}

const T0 = Date.now();
const say = (...a) => console.error('[' + ((Date.now() - T0) / 1000).toFixed(1) + 's]', ...a);
async function session(fn, { prefix = '/plain', args = [] } = {}) {
  const { server, port } = await serve();
  say('server on', port);
  fs.rmSync(PROFILE, { recursive: true, force: true });
  // No traffic leaves the machine: Chrome's own background services (variations, component update, safe browsing ...) are switched off and every host name
  // except 127.0.0.1 is made unresolvable (an earlier net-log of this machine's Chrome showed requests to Google hosts without these flags).
  const NET = ['--disable-background-networking', '--disable-component-update', '--disable-sync', '--no-pings', '--disable-default-apps', '--disable-domain-reliability', '--disable-features=OptimizationHints,MediaRouter,Translate', '--host-resolver-rules=MAP * ~NOTFOUND , EXCLUDE 127.0.0.1'];
  const chrome = await launchChrome({ profile: PROFILE, extraArgs: [...NET, ...(process.env.DETACH_NETLOG ? ['--log-net-log=' + process.env.DETACH_NETLOG, '--net-log-capture-mode=Everything'] : []), ...args] });
  say('chrome up', chrome.version.Browser);
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
  await cdp.opened;
  say('cdp open');
  const events = [];
  cdp.on((m) => { if (m.method && /^Target\.(targetCreated|targetDestroyed|attachedToTarget)/.test(m.method)) events.push({ m: m.method, type: m.params.targetInfo?.type, url: (m.params.targetInfo?.url || '').slice(0, 80), at: Date.now() }); });
  // The default browser context of this machine's Chrome does not complete main-frame loads from http://127.0.0.1 (cause not found, see the report);
  // an isolated browser context (Target.createBrowserContext, incognito-like, still our temporary profile) does. All results come from that context.
  const ctxId = (await cdp.send('Target.createBrowserContext', {})).browserContextId;
  const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId: ctxId });
  say('target created');
  cdp.send('Target.setDiscoverTargets', { discover: true }).catch(() => {});
  say('attaching');
  const { sessionId } = await Promise.race([cdp.send('Target.attachToTarget', { targetId, flatten: true }), sleep(10000).then(() => { throw new Error('attachToTarget timeout'); })]);
  say('attached', sessionId.slice(0, 6));
  const S = (method, params, sid) => cdp.send(method, params, sid || sessionId);
  await S('Runtime.enable'); say('Runtime.enable ok'); await S('Page.enable'); say('Page.enable ok');
  const ev = async (expr, sid, ms = 15000) => { const r = await Promise.race([S('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true }, sid), sleep(ms).then(() => { throw new Error('evaluate timeout ' + ms + ' ms: ' + expr.slice(0, 80)); })]); if (r.exceptionDetails) throw new Error('evaluate: ' + JSON.stringify(r.exceptionDetails).slice(0, 600)); return r.result.value; };
  const pids = descendants(chrome.proc.pid);
  let result, error = null;
  try {
    const nav = await Promise.race([S('Page.navigate', { url: `http://127.0.0.1:${port}${prefix}/index.html` }), sleep(10000).then(() => ({ timeout: true }))]);
    say('navigate', JSON.stringify(nav).slice(0, 120));
    let ready = false;
    for (let i = 0; i < 60 && !ready; i++) { await sleep(100); try { ready = await ev('window.__ready === true', undefined, 3000); } catch (e) { if (i % 10 === 0) say('ready poll:', String(e.message).slice(0, 100)); } }
    say('ready', ready);
    say('page loaded');
    cdp0 = cdp;
    result = await fn({ S, ev, cdp, events, port, sessionId, targetId, chrome, ctxId });
  } catch (e) { error = String(e && e.stack || e); }
  await shutdown(chrome, cdp);
  server.close();
  const left = pids.filter((pid) => { try { process.kill(pid, 0); return true; } catch { return false; } }).length;
  fs.rmSync(PROFILE, { recursive: true, force: true });
  return { result, error, chrome: { product: chrome.version.Browser, ua: chrome.version['User-Agent'], args: chrome.args.filter((x) => !x.startsWith('--remote-debugging-port')) }, chrome_pids_left: left, events };
}

const click = async (S, x = 100, y = 30) => {
  await S('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
  await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1 });
};

async function probe() {
  return session(async ({ S, ev, events }) => {
    const R = {};
    R.page = await ev('JSON.stringify(D.probe())').then(JSON.parse);
    // 1) window.open without user activation
    R.openNoGesture = await ev('(() => { const r = D.open(1, "main"); return JSON.stringify({blocked: r.blocked}); })()').then(JSON.parse);
    // 2) with a CDP-injected click (counts as user activation in Chrome; stated in the report as an assumption)
    await click(S);
    R.openWithGesture = await ev('(async () => { const r = D.open(2, "main"); if (r.blocked) return JSON.stringify({blocked:true}); const h = await Promise.race([r.hello, new Promise(r=>setTimeout(()=>r(null),4000))]); return JSON.stringify({blocked:false, hello: h && {ms:h.ms, dpr:h.hello.dpr, outerW:h.hello.outerW, outerH:h.hello.outerH, screenX:h.hello.screenX, screenY:h.hello.screenY}}); })()').then(JSON.parse);
    // 3) second popup right after, without a new click (one activation consumed?)
    R.secondOpenSameActivation = await ev('(() => { const r = D.open(3, "main"); return JSON.stringify({blocked: r.blocked}); })()').then(JSON.parse);
    R.targetEvents = events.filter((e) => e.m === 'Target.targetCreated').map((e) => e.type + ' ' + e.url);
    // 4) Document PiP
    R.docPiP = await ev(`(async () => { const o = {present: 'documentPictureInPicture' in window}; if (!o.present) return JSON.stringify(o); try { const w = await documentPictureInPicture.requestWindow({width: 400, height: 300}); o.ok = true; o.w = w.innerWidth; o.h = w.innerHeight; w.close(); } catch (e) { o.err = String(e.name + ': ' + e.message).slice(0, 200); } return JSON.stringify(o); })()`).then(JSON.parse);
    await click(S);
    R.docPiPWithGesture = await ev(`(async () => { const o = {}; try { const w = await documentPictureInPicture.requestWindow({width: 400, height: 300}); o.ok = true; o.w = w.innerWidth; o.h = w.innerHeight; o.sameOrigin = (() => { try { return !!w.document.body; } catch { return false; } })(); w.close(); } catch (e) { o.err = String(e.name + ': ' + e.message).slice(0, 200); } return JSON.stringify(o); })()`).then(JSON.parse);
    // 5) getScreenDetails
    await click(S);
    R.getScreenDetails = await ev(`(async () => { const o = {type: typeof window.getScreenDetails}; if (o.type !== 'function') return JSON.stringify(o); try { const d = await Promise.race([window.getScreenDetails(), new Promise((_, rj) => setTimeout(() => rj(new Error('timeout 3s')), 3000))]); o.screens = d.screens.length; o.cur = d.currentScreen && {l: d.currentScreen.left, t: d.currentScreen.top, w: d.currentScreen.width, h: d.currentScreen.height, dpr: d.currentScreen.devicePixelRatio}; } catch (e) { o.err = String(e.name + ': ' + e.message).slice(0, 200); } return JSON.stringify(o); })()`).then(JSON.parse);
    return R;
  });
}


const sleepMs = sleep;
const J = (x) => JSON.parse(x);

// one measurement: open a popup, route the frames by `via`, let the engine worker run in `mode` for `ms`, send `nIn` input events from the popup, collect numbers
async function measure({ via, mode, ms = 4000, nIn = 200, prefix = '/plain', inp = 'msg', late = false }) {
  return session(async ({ S, ev }) => {
    const R = { via, mode, prefix, inp, late };
    const init = J(await ev(`D.init(${JSON.stringify(mode)}).then(JSON.stringify)`));
    R.init = init;
    if (init.error) return R;
    await click(S);
    const o = J(await ev(`(async () => { const r = D.open(1, ${JSON.stringify(via)}, {inp: ${JSON.stringify(inp)}}); if (r.blocked) return JSON.stringify({blocked: true}); const h = await Promise.race([r.hello, new Promise((res) => setTimeout(() => res(null), 4000))]); return JSON.stringify({blocked: false, helloMs: h && h.ms}); })()`));
    R.open = o;
    if (o.blocked) return R;
    R.sabSend = await ev('D.sabSend');
    if (late) {
      await ev('D.start().then(() => true)');       // the worker is already in its loop when the window is registered (a window detached at run time)
      await sleepMs(300);
      if (via !== 'canvas') await ev(`D.attach(1, ${JSON.stringify(via)}); true`);
    } else {
      if (via === 'canvas') { await sleepMs(300); }       // the popup transfers its OffscreenCanvas on load; the opener hands it to the worker
      else await ev(`D.attach(1, ${JSON.stringify(via)}); true`);
      await ev('D.start().then(() => true)');
    }
    const t0 = Date.now();
    R.t_start = t0;
    await sleepMs(ms / 2);
    // input from the popup: CDP mouse events to the popup page (target id of the popup page), one every ~8 ms
    const targets = (await cdp0.send('Target.getTargets')).targetInfos.filter((t) => t.type === 'page' && /popup\.html/.test(t.url));
    R.popupTargets = targets.length;
    if (targets.length) {
      const { sessionId: psid } = await cdp0.send('Target.attachToTarget', { targetId: targets[0].targetId, flatten: true });
      const P = (m, p) => cdp0.send(m, p, psid);
      await P('Runtime.enable'); R.popupFocusBefore = await P('Runtime.evaluate', { expression: 'document.hasFocus()', returnByValue: true }).then((r) => r.result.value);
      const tIn0 = Date.now();
      for (let k = 0; k < nIn; k++) { await P('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 20 + (k % 400), y: 30 + ((k * 3) % 300), button: 'none', buttons: 0 }); await sleepMs(4); }
      R.inputSendMs = Date.now() - tIn0;
      await P('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'F9', key: 'F9', windowsVirtualKeyCode: 120 }); await P('Input.dispatchKeyEvent', { type: 'keyUp', code: 'F9', key: 'F9', windowsVirtualKeyCode: 120 });
    }
    await sleepMs(Math.max(300, ms / 2 - (Date.now() - t0 - ms / 2)));
    // what is really displayed: screenshots of the popup page; the square's R,G encode the worker's frame number (f & 255, f >> 8)
    R.shots = [];
    if (targets.length) {
      const { sessionId: ssid } = await cdp0.send('Target.attachToTarget', { targetId: targets[0].targetId, flatten: true });
      for (let k = 0; k < 6; k++) {
        try {
          const shot = await Promise.race([cdp0.send('Page.captureScreenshot', { format: 'png', clip: { x: 0, y: 0, width: 512, height: 384, scale: 1 } }, ssid), sleepMs(3000).then(() => null)]);
          if (shot) { const dd = J(await ev(`window.__decodeShot(${JSON.stringify(shot.data)}).then(JSON.stringify)`)); dd.t = Date.now(); R.shots.push(dd); }
          else R.shots.push({ error: 'screenshot timeout' });
        } catch (e) { R.shots.push({ error: String(e).slice(0, 100) }); }
        await sleepMs(300);
      }
    }
    await sleepMs(500);
    R.popup = J(await ev('D.report(1).then(JSON.stringify)'));
    R.workerStop = J(await ev('D.stop().then(JSON.stringify)'));
    R.run_ms = Date.now() - t0;
    const p = R.popup;
    R.summary = {
      frames_displayed_in_popup: p.frames, worker_sent: R.workerStop.sent && R.workerStop.sent[1], run_ms: R.run_ms, fps_popup: +(p.frames / (R.run_ms / 1000)).toFixed(1),
      frame_gap_ms: p.gaps && p.gaps.length ? J(await ev(`JSON.stringify(D.pct(${JSON.stringify(p.gaps)}))`)) : null,
      worker_to_popup_ms: p.lat && p.lat.length ? J(await ev(`JSON.stringify(D.pct(${JSON.stringify(p.lat)}))`)) : null,
      fps_from_median_gap: null,
      input_sent: p.inSent, input_acked: p.acks, input_roundtrip_ms: p.rtt && p.rtt.length ? J(await ev(`JSON.stringify(D.pct(${JSON.stringify(p.rtt)}))`)) : null,
      worker_inputs_seen: R.workerStop.inputs, popup_mouse_events: p.moves, popup_key_events: p.keys, popup_has_focus_after: p.hasFocus, visibility: p.visibility,
    };
    if (R.summary.frame_gap_ms && R.summary.frame_gap_ms.p50) R.summary.fps_from_median_gap = +(1000 / R.summary.frame_gap_ms.p50).toFixed(1); else delete R.summary.fps_from_median_gap;
    { const sh = R.shots.filter((x) => x.R >= 0); const fr = sh.map((x) => x.R + 256 * x.G);
      if (sh.length >= 2) { const df = fr[fr.length - 1] - fr[0], dt = (sh[sh.length - 1].t - sh[0].t) / 1000; R.summary.fps_from_screenshots = { frames_advanced: df, seconds: +dt.toFixed(2), fps: +(df / dt).toFixed(1), samples: sh.length }; } }
    delete R.popup.gaps; delete R.popup.lat; delete R.popup.rtt;
    return R;
  }, { prefix, exposeCdp: true });
}
let cdp0 = null;


// popup closed: by the page (window.close()) and by the "user" (CDP Page.close of the popup target); what the opener sees, and what the worker keeps doing
async function closeTest() {
  return session(async ({ S, ev, cdp }) => {
    const R = {};
    J(await ev(`D.init('loop').then(JSON.stringify)`));
    await click(S);
    R.open = J(await ev(`(async () => { const r = D.open(1, 'port'); const h = await r.hello; return JSON.stringify({helloMs: h.ms}); })()`));
    await ev(`D.attach(1, 'port'); D.start().then(() => true)`);
    await sleep(800);
    R.before = J(await ev(`JSON.stringify({closedFlag: D.popups.get(1).closed, relayed: D.relayed})`));
    // (a) close from the opener: popup.close()
    const tc = J(await ev(`JSON.stringify(D.closePopup(1))`));
    await sleep(600);
    R.afterWindowClose = J(await ev(`JSON.stringify({closedFlag: D.popups.get(1).closed, closedEvents: D.closedIds, nowDelta: performance.timeOrigin + performance.now() - ${tc.t}})`));
    // the worker keeps rendering into a port nobody reads: does it notice? (messages to a closed port are dropped silently)
    R.workerAfterClose = J(await ev(`D.stop().then(JSON.stringify)`));
    // (b) a second popup closed "by the user": CDP Page.close on the popup target (closest headless equivalent of the title-bar X)
    await ev('D.closedIds.length = 0; true');
    J(await ev(`D.init('loop').then(JSON.stringify)`).catch(() => '{}'));
    return R;
  });
}
async function closeByUser() {
  return session(async ({ S, ev, cdp }) => {
    const R = {};
    J(await ev(`D.init('loop').then(JSON.stringify)`));
    await click(S);
    R.open = J(await ev(`(async () => { const r = D.open(1, 'main'); const h = await r.hello; return JSON.stringify({helloMs: h.ms}); })()`));
    await ev(`D.attach(1, 'main'); D.start().then(() => true)`);
    await sleep(800);
    const targets = (await cdp.send('Target.getTargets')).targetInfos.filter((t) => t.type === 'page' && /popup\.html/.test(t.url));
    const tClose = Date.now();
    await cdp.send('Target.closeTarget', { targetId: targets[0].targetId });
    await sleep(800);
    R.afterUserClose = J(await ev(`JSON.stringify({openerSeesClosed: D.popups.get(1).closed, closedMsgs: D.closedIds.map((x) => x.id), relayedAfter: D.relayed})`));
    R.msFromCloseToSeen = null;
    const rep = J(await ev(`JSON.stringify({events: D.events.filter((e) => e.t === 'closed').map((e) => e.at)})`));
    R.closedMessageReceived = rep.events.length > 0;
    R.workerStop = J(await ev(`D.stop().then(JSON.stringify)`));
    return R;
  });
}

// Document PiP as the detached window
async function pipMeasure({ mode = 'loop', ms = 5000, nIn = 300, prefix = '/plain' }) {
  return session(async ({ S, ev, cdp }) => {
    const R = { mode, prefix };
    const init = J(await ev(`D.init(${JSON.stringify(mode)}).then(JSON.stringify)`)); R.init = init;
    await click(S);
    R.open = J(await ev(`D.openPip(1).then(JSON.stringify)`));
    await ev('D.start().then(() => true)');
    const t0 = Date.now();
    await sleep(ms / 2);
    const targets = (await cdp.send('Target.getTargets')).targetInfos;
    R.targetTypes = targets.map((t) => t.type + ' ' + (t.url || '').slice(0, 50)).filter((x) => /pip|about:blank|page/.test(x)).slice(0, 8);
    const pipT = targets.find((t) => t.type === 'page' && t.title === 'pip-win');
    R.pipTargetFound = !!pipT;
    if (pipT) {
      const { sessionId: psid } = await cdp.send('Target.attachToTarget', { targetId: pipT.targetId, flatten: true });
      const P = (m, p) => cdp.send(m, p, psid);
      for (let k = 0; k < nIn; k++) { await P('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 20 + (k % 400), y: 30 + ((k * 3) % 300), button: 'none', buttons: 0 }); await sleep(4); }
      R.shots = [];
      for (let k = 0; k < 4; k++) { try { const shot = await Promise.race([P('Page.captureScreenshot', { format: 'png', clip: { x: 0, y: 0, width: 512, height: 384, scale: 1 } }), sleep(3000).then(() => null)]); if (shot) { const dd = J(await ev(`window.__decodeShot(${JSON.stringify(shot.data)}).then(JSON.stringify)`)); dd.t = Date.now(); R.shots.push(dd); } else R.shots.push({ error: 'timeout' }); } catch (e) { R.shots.push({ error: String(e).slice(0, 100) }); } await sleep(300); }
    }
    await sleep(500);
    const rep = J(await ev('JSON.stringify(D.pipReport(1))'));
    R.run_ms = Date.now() - t0;
    R.workerStop = J(await ev('D.stop().then(JSON.stringify)'));
    R.summary = { frames_displayed: rep.frames, fps_from_median_gap: rep.gaps.length ? +(1000 / J(await ev(`JSON.stringify(D.pct(${JSON.stringify(rep.gaps)}))`)).p50).toFixed(1) : null,
      worker_to_pip_ms: rep.lat.length ? J(await ev(`JSON.stringify(D.pct(${JSON.stringify(rep.lat)}))`)) : null,
      input_roundtrip_ms: rep.rtt.length ? J(await ev(`JSON.stringify(D.pct(${JSON.stringify(rep.rtt)}))`)) : null, inSent: rep.inSent, acks: rep.acks, mouseEvents: rep.moves };
    const sh = (R.shots || []).filter((x) => x.R >= 0);
    if (sh.length >= 2) { const df = sh[sh.length - 1].R + 256 * sh[sh.length - 1].G - (sh[0].R + 256 * sh[0].G), dt = (sh[sh.length - 1].t - sh[0].t) / 1000; R.summary.fps_from_screenshots = { fps: +(df / dt).toFixed(1), frames_advanced: df, seconds: +dt.toFixed(2) }; }
    // close the PiP: pagehide, and what the opener sees
    const c = J(await ev('JSON.stringify(D.pipClose(1))')); await sleep(300);
    R.afterClose = J(await ev('JSON.stringify({closed: D.pips.get(1).win.closed, closedAtSet: D.pips.get(1).closedAt !== null})'));
    // a second PiP window without a new gesture / while one is open (limit: one per tab)
    R.secondPipWithoutGesture = J(await ev(`(async () => { try { await documentPictureInPicture.requestWindow({width: 200, height: 150}); return JSON.stringify({ok: true}); } catch (e) { return JSON.stringify({err: e.name + ': ' + e.message.slice(0, 120)}); } })()`));
    // two PiP windows with a fresh activation for EACH call (the earlier version of this test spent its only activation on the first call)
    await click(S);
    await ev(`(async () => { window.__pipA = await documentPictureInPicture.requestWindow({width: 200, height: 150}); return true; })()`);
    await click(S);
    R.twoPipWindows = J(await ev(`(async () => { const o = {firstClosedBefore: window.__pipA.closed}; try { const b = await documentPictureInPicture.requestWindow({width: 220, height: 160}); o.second = 'resolved'; o.sameObject = (window.__pipA === b); o.firstClosedAfter = window.__pipA.closed; o.currentWindowIsSecond = (documentPictureInPicture.window === b); } catch (e) { o.second = e.name + ': ' + e.message.slice(0, 120); o.firstClosedAfter = window.__pipA.closed; } return JSON.stringify(o); })()`));
    return R;
  }, { prefix });
}

// popup render path vs a busy opener main thread: via=main depends on the opener's main thread for every frame, via=port does not
async function busyTest(via) {
  return session(async ({ S, ev, cdp }) => {
    const R = { via };
    J(await ev(`D.init('loop').then(JSON.stringify)`));
    await click(S);
    J(await ev(`(async () => { const r = D.open(1, ${JSON.stringify(via)}); const h = await r.hello; return JSON.stringify({ms: h.ms}); })()`));
    await ev(`D.attach(1, ${JSON.stringify(via)}); D.start().then(() => true)`);
    await sleep(1000);
    // reset popup counters by taking a baseline report, then block the opener main thread for 300 ms every 500 ms for 4 s
    const base = J(await ev(`D.report(1).then((r) => JSON.stringify({frames: r.frames}))`));
    await ev(`D.busy(300, 200, 4000).then(() => true)`);
    const rep = J(await ev(`D.report(1).then(JSON.stringify)`));
    R.frames_before = base.frames; R.frames_after = rep.frames;
    const gaps = rep.gaps.slice(base.frames);
    R.gap_during_busy_ms = gaps.length ? J(await ev(`JSON.stringify(D.pct(${JSON.stringify(gaps)}))`)) : null;
    R.worker = J(await ev(`D.stop().then(JSON.stringify)`));
    return R;
  });
}
// geometry / dpr: where the popup is, moveTo / resizeTo from the popup and from the opener (headless: emulated window geometry, no window manager)
async function geomTest() {
  return session(async ({ S, ev, cdp }) => {
    const R = {};
    await click(S);
    R.open = J(await ev(`(async () => { const r = D.open(1, 'main', {left: 123, top: 77, w: 512, h: 384}); const h = await r.hello; return JSON.stringify({ms: h.ms, sx: h.hello.screenX, sy: h.hello.screenY, ow: h.hello.outerW, oh: h.hello.outerH}); })()`));
    R.opener = J(await ev('JSON.stringify(D.probe())'));
    R.geom0 = J(await ev('JSON.stringify(D.geom(1))'));
    R.moveFromOpener = await ev('D.moveFromOpener(1, 300, 200)'); await sleep(300);
    R.geomAfterMoveFromOpener = J(await ev('JSON.stringify(D.geom(1))'));
    await ev('D.say(1, {t: "move", x: 40, y: 30}); true'); await sleep(300);
    R.geomAfterMoveFromPopup = J(await ev('JSON.stringify(D.geom(1))'));
    R.resizeFromOpener = await ev('D.resizeFromOpener(1, 640, 480)'); await sleep(300);
    R.geomAfterResize = J(await ev('JSON.stringify(D.geom(1))'));
    // per-window devicePixelRatio: emulate dsf 2 on the popup only (CDP Emulation; a real second monitor with another scale is NOT tested)
    const targets = (await cdp.send('Target.getTargets')).targetInfos.filter((t) => t.type === 'page' && /popup\.html/.test(t.url));
    if (targets.length) {
      const { sessionId: psid } = await cdp.send('Target.attachToTarget', { targetId: targets[0].targetId, flatten: true });
      await cdp.send('Emulation.setDeviceMetricsOverride', { width: 512, height: 384, deviceScaleFactor: 2, mobile: false }, psid);
      await sleep(300);
      R.dprEmulatedPopupOnly = { opener: J(await ev('JSON.stringify(D.probe().dpr)')), popup: J(await ev('JSON.stringify(D.geom(1).dpr)')) };
    }
    return R;
  });
}

// COOP: what the opener and the popup see when the popup is NOT in the opener's browsing context group
async function coopTest(prefix) {
  return session(async ({ S, ev, cdp }) => {
    const R = { prefix };
    R.opener = J(await ev('JSON.stringify({coi: self.crossOriginIsolated, sab: typeof SharedArrayBuffer})'));
    await click(S);
    R.open = J(await ev(`(() => { const r = D.open(1, 'main', {}); if (r.blocked) return JSON.stringify({blocked: true}); const p = D.popups.get(1); return JSON.stringify({blocked: false, closedRightAfter: p.closed}); })()`));
    await sleep(1500);
    R.afterLoad = J(await ev(`(() => { const p = D.popups.get(1); let access = 'ok'; try { void p.document.title; } catch (e) { access = e.name; } return JSON.stringify({closedFlag: p.closed, helloReceived: !!(D.hello && D.hello[1]), docAccess: access, eventsFromPopup: D.events.map((e) => e.t)}); })()`));
    const targets = (await cdp.send('Target.getTargets')).targetInfos.filter((t) => t.type === 'page' && /popup\.html/.test(t.url));
    R.popupTargets = targets.length;
    if (targets.length) {
      const { sessionId: psid } = await cdp.send('Target.attachToTarget', { targetId: targets[0].targetId, flatten: true });
      await cdp.send('Runtime.enable', {}, psid);
      const r = await cdp.send('Runtime.evaluate', { expression: 'JSON.stringify({hasOpener: !!window.opener, coi: self.crossOriginIsolated, sab: typeof SharedArrayBuffer, href: location.pathname})', returnByValue: true }, psid);
      R.popupSees = J(r.result.value);
    }
    return R;
  }, { prefix });
}

// does the popup keep presenting frames while the opener's main thread is blocked? count CDP screencast frames of the popup target in an idle window and in a busy window
async function busyPresent(via) {
  return session(async ({ S, ev, cdp }) => {
    const R = { via };
    J(await ev(`D.init('loop').then(JSON.stringify)`));
    await click(S);
    J(await ev(`(async () => { const r = D.open(1, ${JSON.stringify(via)}); const h = await r.hello; return JSON.stringify({ms: h.ms}); })()`));
    if (via !== 'canvas') await ev(`D.attach(1, ${JSON.stringify(via)}); true`); else await sleep(300);
    await ev('D.start().then(() => true)');
    await sleep(800);
    const t = (await cdp.send('Target.getTargets')).targetInfos.find((x) => x.type === 'page' && /popup\.html/.test(x.url));
    const { sessionId: psid } = await cdp.send('Target.attachToTarget', { targetId: t.targetId, flatten: true });
    let frames = 0;
    cdp.on((m) => { if (m.method === 'Page.screencastFrame' && m.sessionId === psid) { frames++; cdp.send('Page.screencastFrameAck', { sessionId: m.params.sessionId }, psid).catch(() => {}); } });
    await cdp.send('Page.enable', {}, psid);
    await cdp.send('Page.startScreencast', { format: 'jpeg', quality: 30, everyNthFrame: 1 }, psid);
    const a0 = frames, ta = Date.now(); await sleep(3000); const idle = frames - a0, idleMs = Date.now() - ta;
    // busy: block the opener main thread 300 ms out of every 500 ms for 3 s (do not await: our evaluate on the opener would be blocked too)
    const b0 = frames, tb = Date.now();
    S('Runtime.evaluate', { expression: 'D.busy(300, 200, 3000)', awaitPromise: true, returnByValue: true }).catch(() => {});
    await sleep(3200);
    const busy = frames - b0, busyMs = Date.now() - tb;
    R.screencast = { idle_frames: idle, idle_ms: idleMs, idle_fps: +(idle * 1000 / idleMs).toFixed(1), busy_frames: busy, busy_ms: busyMs, busy_fps: +(busy * 1000 / busyMs).toFixed(1) };
    await cdp.send('Page.stopScreencast', {}, psid).catch(() => {});
    R.worker = J(await ev(`D.stop().then(JSON.stringify)`));
    return R;
  });
}

// several popups at once (one user activation each), all fed by one engine worker; plus: an AudioContext in the opener while popups open / close
async function multiTest(via, n) {
  return session(async ({ S, ev, cdp }) => {
    const R = { via, n };
    J(await ev(`D.init('loop').then(JSON.stringify)`));
    await click(S);
    R.audio0 = J(await ev('D.audio().then(JSON.stringify)'));
    R.audioBefore = J(await ev('JSON.stringify(D.audioRead())'));
    R.opens = [];
    for (let i = 1; i <= n; i++) {
      await click(S);
      R.opens.push(J(await ev(`(async () => { const r = D.open(${i}, ${JSON.stringify(via)}, {left: ${60 * i}, top: ${40 * i}}); if (r.blocked) return JSON.stringify({id: ${i}, blocked: true}); const h = await r.hello; return JSON.stringify({id: ${i}, blocked: false, ms: h.ms}); })()`)));
    }
    for (let i = 1; i <= n; i++) if (!R.opens[i - 1].blocked) await ev(`${via === 'canvas' ? 'true' : `D.attach(${i}, ${JSON.stringify(via)})`}; true`);
    await sleep(500);
    await ev('D.start().then(() => true)');
    await sleep(4000);
    R.perWindow = [];
    for (let i = 1; i <= n; i++) { if (R.opens[i - 1].blocked) continue; const rep = J(await ev(`D.report(${i}).then(JSON.stringify)`)); R.perWindow.push({ id: i, frames: rep.frames }); }
    R.worker = J(await ev('D.stop().then(JSON.stringify)'));
    R.audioDuring = J(await ev('JSON.stringify(D.audioRead())'));
    for (let i = 1; i <= n; i++) if (!R.opens[i - 1].blocked) await ev(`D.closePopup(${i}); true`);
    await sleep(1500);
    R.audioAfterClose = J(await ev('JSON.stringify(D.audioRead())'));
    return R;
  });
}

// Document PiP: is the first window closed when a second one is requested (with a fresh activation)? read the flag right away and 600 ms later
async function pipTwo() {
  return session(async ({ S, ev }) => {
    const R = {};
    await click(S);
    await ev(`(async () => { window.__pipA = await documentPictureInPicture.requestWindow({width: 200, height: 150}); window.__pipA.addEventListener('pagehide', () => { window.__pipAHide = true; }); return true; })()`);
    await click(S);
    R.second = J(await ev(`(async () => { const o = {}; try { const b = await documentPictureInPicture.requestWindow({width: 220, height: 160}); window.__pipB = b; o.resolved = true; o.sameObject = (window.__pipA === b); o.firstClosedRightAfter = window.__pipA.closed; o.currentIsSecond = (documentPictureInPicture.window === b); } catch (e) { o.err = e.name + ': ' + e.message.slice(0, 120); } return JSON.stringify(o); })()`));
    await sleep(600);
    R.later = J(await ev(`JSON.stringify({firstClosedAfter600ms: window.__pipA.closed, firstPagehideSeen: !!window.__pipAHide, secondClosed: window.__pipB ? window.__pipB.closed : null})`));
    return R;
  });
}

// Window Management API with the permission granted by CDP (Browser.grantPermissions, only for our own throw-away context)
async function wmTest() {
  return session(async ({ S, ev, cdp, port, ctxId }) => {
    const R = {};
    const origin = `http://127.0.0.1:${port}`;
    try { await cdp.send('Browser.grantPermissions', { permissions: ['windowManagement'], origin, browserContextId: ctxId }); R.grant = 'ok'; } catch (e) { R.grant = String(e).slice(0, 200); }
    await click(S);
    R.screenDetails = J(await ev(`(async () => { const o = {}; try { const d = await Promise.race([window.getScreenDetails(), new Promise((_, rj) => setTimeout(() => rj(new Error('timeout 4s')), 4000))]); o.screens = d.screens.length; o.current = {left: d.currentScreen.left, top: d.currentScreen.top, width: d.currentScreen.width, height: d.currentScreen.height, availLeft: d.currentScreen.availLeft, availTop: d.currentScreen.availTop, dpr: d.currentScreen.devicePixelRatio, isPrimary: d.currentScreen.isPrimary, isInternal: d.currentScreen.isInternal, label: d.currentScreen.label}; o.isExtended = screen.isExtended; } catch (e) { o.err = e.name + ': ' + e.message.slice(0, 160); } return JSON.stringify(o); })()`));
    await click(S);
    R.popupAtScreenPos = J(await ev(`(async () => { const d = await window.getScreenDetails(); const sc = d.currentScreen; const r = D.open(1, 'main', {features: 'popup=yes,left=' + (sc.availLeft + 200) + ',top=' + (sc.availTop + 100) + ',width=512,height=384'}); if (r.blocked) return JSON.stringify({blocked: true}); const h = await r.hello; return JSON.stringify({blocked: false, sx: h.hello.screenX, sy: h.hello.screenY, requestedLeft: sc.availLeft + 200, requestedTop: sc.availTop + 100}); })()`));
    return R;
  });
}

// how long does one user activation allow window.open()? (the guest's menu handler reaches the host some time after the click)
async function activationTest() {
  return session(async ({ S, ev }) => {
    const R = { tries: [] };
    for (const waitMs of [0, 1000, 3000, 4500, 5500, 7000]) {
      await click(S);
      const t0 = Date.now();
      await sleep(waitMs);
      const r = J(await ev(`JSON.stringify({isActive: navigator.userActivation.isActive, hasBeenActive: navigator.userActivation.hasBeenActive, blocked: (() => { const w = window.open('popup.html?id=9&via=main', 'act-' + Math.random(), 'popup=yes,width=200,height=150'); if (w) { w.close(); return false; } return true; })()})`));
      R.tries.push({ waitedMs: waitMs, elapsedSinceClickMs: Date.now() - t0, ...r });
    }
    return R;
  });
}

// does the popup run in the same renderer process as the opener? (count --type=renderer processes of our own Chrome tree before / after window.open, and with the popup closed)
// and: can the client origin of a popup be recovered from a mouse event (event.screenX - event.clientX)?
async function procTest() {
  return session(async ({ S, ev, cdp, chrome }) => {
    const R = {};
    const fsm = await import('node:fs');
    const renderers = () => descendants(chrome.proc.pid).filter((pid) => { try { return /--type=renderer/.test(fsm.readFileSync(`/proc/${pid}/cmdline`, 'utf8')); } catch { return false; } });
    await sleep(500);
    R.renderersBeforeOpen = renderers().length;
    await click(S);
    J(await ev(`(async () => { const r = D.open(1, 'main', {left: 150, top: 90}); const h = await r.hello; return JSON.stringify({ms: h.ms}); })()`));
    await sleep(800);
    R.renderersWithPopup = renderers().length;
    await ev('D.say(1, {t: "geom"}); true'); await sleep(300);
    const targets = (await cdp.send('Target.getTargets')).targetInfos.filter((t) => t.type === 'page' && /popup\.html/.test(t.url));
    const { sessionId: psid } = await cdp.send('Target.attachToTarget', { targetId: targets[0].targetId, flatten: true });
    await cdp.send('Runtime.enable', {}, psid);
    await cdp.send('Runtime.evaluate', { expression: `window.__me = null; window.addEventListener('mousemove', (e) => { window.__me = {sx: e.screenX, sy: e.screenY, cx: e.clientX, cy: e.clientY, wsx: window.screenX, wsy: window.screenY, ow: window.outerWidth, oh: window.outerHeight, iw: window.innerWidth, ih: window.innerHeight}; }, {once: true}); true`, returnByValue: true }, psid);
    await cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 100, y: 50, button: 'none', buttons: 0 }, psid);
    await sleep(200);
    const me = await cdp.send('Runtime.evaluate', { expression: 'JSON.stringify(window.__me)', returnByValue: true }, psid);
    R.mouseEvent = J(me.result.value);
    R.derivedClientOrigin = R.mouseEvent && { x: R.mouseEvent.sx - R.mouseEvent.cx, y: R.mouseEvent.sy - R.mouseEvent.cy };
    R.fromWindowGeometry = R.mouseEvent && { x: R.mouseEvent.wsx + (R.mouseEvent.ow - R.mouseEvent.iw) / 2, y: R.mouseEvent.wsy + (R.mouseEvent.oh - R.mouseEvent.ih) };
    return R;
  });
}

async function relayTest() {
  return session(async ({ S, ev }) => {
    const R = { tries: [] };
    for (const d of [10, 1000, 4000, 5500]) {
      await click(S);
      R.tries.push(J(await ev(`D.relayOpen(${d}).then(JSON.stringify)`, undefined, 20000)));
    }
    return R;
  });
}

const out = {};
if (which === 'probe' || which === 'all') out.probe = await probe();
if (which === 'coop') { for (const pf of ['/coi', '/coap', '/coos', '/plain']) out['coop' + pf] = await coopTest(pf); }
if (which === 'relay') { out.relay = await relayTest(); }
if (which === 'proc') { out.proc = await procTest(); }
if (which === 'act') { out.activation = await activationTest(); }
if (which === 'wm') { out.wm = await wmTest(); }
if (which === 'piptwo') { out.piptwo = await pipTwo(); }
if (which === 'multi') { out['multi-main-3'] = await multiTest('main', 3); out['multi-port-3'] = await multiTest('port', 3); out['multi-canvas-3'] = await multiTest('canvas', 3); }
if (which === 'busy2') { for (const v of ['main', 'canvas']) out['busy2-' + v] = await busyPresent(v); }
if (which === 'busy') { out['busy-main'] = await busyTest('main'); out['busy-port'] = await busyTest('port'); }
if (which === 'geom') { out.geom = await geomTest(); }
if (which === 'close') { out.closeByWindowClose = await closeTest(); out.closeByUser = await closeByUser(); }
if (which === 'pip') { out['pip-loop'] = await pipMeasure({ mode: 'loop', prefix: '/plain' }); out['pip-block'] = await pipMeasure({ mode: 'block', prefix: '/coi' }); }
if (which === 'measure') {
  const via = argv.via || 'main', mode = argv.mode || 'loop';
  const prefix = argv.prefix || (mode === 'block' ? '/coi' : '/plain');
  out['measure-' + via + '-' + mode] = await measure({ via, mode, ms: +(argv.ms || 4000), nIn: +(argv.nIn || 200), prefix, inp: argv.inp || 'msg', late: argv.late === '1' });
}
fs.mkdirSync(RES, { recursive: true });
const name = argv.name || which;
fs.writeFileSync(path.join(RES, name + '.json'), JSON.stringify(out, null, 1));
console.log(JSON.stringify(out, null, 1).slice(0, 6000));
export { session, click, serve, RES };
