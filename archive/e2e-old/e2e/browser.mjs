// An isolated headless Chrome for the e2e tests. Rules of decision 0124 (lift/harness/web/present/lib.mjs isolationArgs, adapted):
// a throw-away profile, a NEW BROWSER CONTEXT per page (an old finding: the default context could not read 127.0.0.1), every host except 127.0.0.1 unresolvable,
// no proxy / IPv6 / QUIC / background networking, a net-log written for the check afterwards.
import fs from 'node:fs';
import path from 'node:path';
import { Cdp, launchChrome, shutdown, sleep } from './cdp.mjs';
import { summarizeNetlog } from './netlog.mjs';

export { sleep };

function isolationArgs(profile, netlog) {
  return [
    '--password-store=basic',
    '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1',
    '--no-proxy-server', '--disable-ipv6', '--disable-quic',
    '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-domain-reliability',
    '--no-pings', '--metrics-recording-only', '--disable-client-side-phishing-detection', '--disable-breakpad', '--disable-extensions', '--safebrowsing-disable-auto-update', '--disable-search-engine-choice-screen', '--no-service-autorun',
    '--disable-features=OptimizationHints,MediaRouter,Translate,CertificateTransparencyComponentUpdater,NetworkTimeServiceQuerying,AutofillServerCommunication,OptimizationGuideModelDownloading,SafeBrowsingEnhancedProtection,PrivacySandboxSettings4,Variations,SafeBrowsing,SafeBrowsingHashPrefixRealTimeLookups,HashPrefixRealTimeLookups,SafeBrowsingOnUIThread,SafeBrowsingNewGmsApiForBrowseUrlDatabaseCheck',
    '--variations-server-url=http://127.0.0.1:9/', '--disable-field-trial-config',
    `--crash-dumps-dir=${path.join(profile, 'crash')}`,
    `--log-net-log=${netlog}`, '--net-log-capture-mode=Default',
    '--force-color-profile=srgb', '--disable-lcd-text',
  ];
}

export async function openBrowser(workDir, name) {
  const profile = path.join(workDir, 'profile-' + name);
  const netlog = path.join(workDir, 'netlog-' + name + '.json');
  fs.rmSync(profile, { recursive: true, force: true });
  fs.mkdirSync(workDir, { recursive: true });
  // FL_CHROME_ARGS (decision 0170): extra command-line flags for the measurement, separated by spaces (for example --js-flags=--liftoff-only); none by default
  const chrome = await launchChrome({ profile, extraArgs: [...isolationArgs(profile, netlog), ...(process.env.FL_CHROME_ARGS ? process.env.FL_CHROME_ARGS.split(' ').filter(Boolean) : [])] });
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
  await cdp.opened;
  const timeout = (p, ms, label) => { let t; return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${label}`)), ms); })]).finally(() => clearTimeout(t)); };

  // one page in a new isolated browser context
  async function newPage({ width = 1930, height = 1092, dpr = 1, initScript = '' } = {}) {
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    // decision 0170: `activateOther()` opens a second blank page in the SAME context and brings it to the front, which makes this page a background tab (document.visibilityState 'hidden'); `activate()` brings this page back
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params) => timeout(cdp.send(method, params, sessionId), 120000, 'CDP ' + method);
    await S('Runtime.enable');
    await S('Page.enable');
    await S('Network.enable');
    await S('Network.setCacheDisabled', { cacheDisabled: true });
    await S('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: dpr, mobile: false });
    if (initScript) await S('Page.addScriptToEvaluateOnNewDocument', { source: initScript });
    const consoleLog = [];
    const responses = new Map();
    cdp.on((m) => {
      if (m.sessionId !== sessionId) return;
      if (m.method === 'Runtime.consoleAPICalled') consoleLog.push(m.params.type + ': ' + m.params.args.map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 300));
      if (m.method === 'Runtime.exceptionThrown') consoleLog.push('EXCEPTION: ' + JSON.stringify(m.params.exceptionDetails).slice(0, 500));
      if (m.method === 'Network.responseReceived') responses.set(m.params.response.url, { status: m.params.response.status, headers: m.params.response.headers });
    });
    const ev = async (expression, ms = 120000) => {
      const r = await timeout(S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }), ms, 'evaluate ' + expression.slice(0, 80));
      if (r.exceptionDetails) throw new Error('evaluate: ' + JSON.stringify(r.exceptionDetails).slice(0, 800));
      return r.result.value;
    };
    const goto = async (url) => { await S('Page.navigate', { url }); };
    const waitFor = async (expr, ms = 60000, label = expr) => {
      const t0 = Date.now();
      for (;;) {
        const v = await ev(expr).catch(() => undefined);
        if (v) return v;
        if (Date.now() - t0 > ms) throw new Error(`waitFor timed out after ${ms} ms: ${label}`);
        await sleep(50);
      }
    };
    // x, y are CLIENT (viewport) coordinates, as getBoundingClientRect gives them; the clip of Page.captureScreenshot is in DOCUMENT coordinates (decision 0232): the two are the same only while the page is not scrolled
    const shot = async (x, y, w, h) => {
      const sc = await ev('({ x: window.scrollX, y: window.scrollY })');
      const r = await S('Page.captureScreenshot', { format: 'png', fromSurface: true, clip: { x: x + sc.x, y: y + sc.y, width: w, height: h, scale: 1 } });
      return Buffer.from(r.data, 'base64');
    };
    const close = async () => { await cdp.send('Target.closeTarget', { targetId }).catch(() => {}); };
    let other = null;
    const activateOther = async () => { if (!other) other = (await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId })).targetId; await cdp.send('Target.activateTarget', { targetId: other }); };
    const activate = async () => { await cdp.send('Target.activateTarget', { targetId }); };
    return { ev, goto, waitFor, shot, consoleLog, responses, close, S, activateOther, activate };
  }

  async function close() {
    await shutdown(chrome, cdp);
    await sleep(400);
    // the browser writes the end of the net-log when it exits; after a run with ~1 GB of wasm memory that can take a while: wait (up to 20 s) until the file is complete JSON
    for (let i = 0; i < 100; i++) { try { JSON.parse(fs.readFileSync(netlog, 'utf8')); break; } catch { await sleep(200); } }
    const summary = summarizeNetlog(netlog);
    fs.rmSync(profile, { recursive: true, force: true });
    return { ...summary, netlogFile: netlog };
  }
  return { newPage, close, cdp, pid: chrome.proc.pid, version: chrome.version.Browser, v8: chrome.version['V8-Version'], userAgent: chrome.version['User-Agent'] };
}
