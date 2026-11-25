// Boot check for an arbitrary bundle directory (copy of boot.mjs with the bundle named by argv[2]). Runs it four times in headless Chrome (a new profile each time): once cold, then three warm starts. Prints one JSON object on stdout: the fields of the warm start whose elapsedMs is the median of the three (elapsedMs is that median; samples are the three warm elapsedMs in the order measured), with coldMs (the cold start's elapsedMs; the first start after a build is the cold one, the OS file cache not yet warm). The median keeps one slow runner's start from deciding the check. Exit 1 when a start failed ("error" set; a cold start that is not engine-idle counts as failed, and so does a warm start that is not engine-idle or shows STAND-IN), 2 when no bundle directory is given.
// Usage: node tools/boot-check/boot-bundle.mjs <absolute path to the bundle directory raw/<name>> > boot.json   (needs app/node_modules and app/dist). The page is opened at "/" with no parameters.
// The brotli files of the bundle are in br/<name>/ next to raw/ (stage-assets.mjs links them for the app's /assets/ route). The temporary directory is inside the repository (.tmp-boot-*) and is removed.
// standIn is only whether the text "STAND-IN" is in the page body; it is a reading, not a proof either way.
// bootBundle() is the same start-up for the other tools (tools/missing-paths/measure.mjs); it also returns the whole engine log, not only its first 30 lines.
// bootBundle(dir, { afterIdle }): when the engine is idle (the state engine-idle, no error), afterIdle({ send, evaluate }) runs once on the same page (send(method, params) is a CDP call, evaluate(expression) a Runtime.evaluate by value);
// then the state and the log are read again. A failure in afterIdle is the start-up's error. Without afterIdle nothing changes.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from './cdp.mjs';
import { stageAssets } from './stage-assets.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const CDP_TIMEOUT_MS = 20000;
const COLD_LIMIT_MS = 60000;

// the isolation flags of archive/e2e-old/e2e/browser.mjs, without the net-log and the crash folder
const ISOLATION = [
  '--password-store=basic',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1',
  '--no-proxy-server', '--disable-ipv6', '--disable-quic',
  '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-domain-reliability',
  '--no-pings', '--metrics-recording-only', '--disable-client-side-phishing-detection', '--disable-breakpad', '--disable-extensions', '--safebrowsing-disable-auto-update', '--disable-search-engine-choice-screen', '--no-service-autorun',
  '--disable-features=OptimizationHints,MediaRouter,Translate,CertificateTransparencyComponentUpdater,NetworkTimeServiceQuerying,AutofillServerCommunication,OptimizationGuideModelDownloading,SafeBrowsingEnhancedProtection,PrivacySandboxSettings4,Variations,SafeBrowsing,SafeBrowsingHashPrefixRealTimeLookups,HashPrefixRealTimeLookups,SafeBrowsingOnUIThread,SafeBrowsingNewGmsApiForBrowseUrlDatabaseCheck',
  '--variations-server-url=http://127.0.0.1:9/', '--disable-field-trial-config',
  '--force-color-profile=srgb', '--disable-lcd-text',
];

// the state, not timeline.idle: the Worker sends the 'idle' mark before the 'idle' message that sets engine-idle, so timeline.idle is set for a moment while the state is still 'running'
const POLL = `(() => { const a = window.__app; if (!a) return null; return { state: a.state }; })()`;
const FINAL = `(() => { const a = window.__app ?? null; return {
  state: a ? a.state : null,
  engineKind: a ? a.engineKind : null,
  timelineIdle: a && typeof a.timeline.idle === 'number' ? a.timeline.idle : null,
  engineLog: a ? a.engineLog() : null,
  documentTitle: document.title,
  standIn: document.body.innerText.includes('STAND-IN'),
}; })()`;

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

/** One start-up of the bundle in its own Chrome: `out` is the check's JSON object (engineLog30: the first 30 lines of the engine's log); `engineLog` is the whole log (null when the page has no app). `afterIdle` see the header. */
export async function bootBundle(bundleDir, { afterIdle } = {}) {
  const t0 = Date.now();
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const out = { startedAt: new Date(t0).toISOString(), state: null, engineKind: null, timelineIdle: null, engineLog30: null, standIn: null, documentTitle: null, elapsedMs: null, loadavg: null, chromeVersion: null, error: null };
  let engineLog = null;
  const root = fs.mkdtempSync(path.join(repo, '.tmp-boot-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null, navAt = null;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;

    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });

    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    out.chromeVersion = chrome.version.Browser;
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params) => timed(cdp.send(method, params, sessionId), CDP_TIMEOUT_MS, method);
    const ev = async (expression) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
      return r.result.value;
    };
    await S('Page.enable');

    navAt = Date.now();
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    let stopped = false;
    // the Worker's 'idle' mark sets timeline.idle, but the state becomes 'engine-idle' only with its 'idle' message, which comes after hostFiles.release(): wait for the state
    for (;;) {
      const p = await ev(POLL).catch(() => null);
      if (p && (p.state === 'engine-idle' || p.state === 'error' || p.state === 'exited')) { stopped = true; break; }
      if (Date.now() - navAt >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    const { engineLog: all, ...fin } = await ev(FINAL);
    engineLog = all;
    Object.assign(out, fin, { engineLog30: all ? all.slice(0, 30) : null });
    if (out.timelineIdle === null && !stopped) out.error = `no first idle, error or exit within ${WAIT_LIMIT_MS} ms (state ${out.state ?? 'window.__app not set'})`;
    else if (out.timelineIdle === null) out.error = `state ${out.state} before the first idle`;
    else if (!stopped) out.error = `state ${out.state} after the first idle, no engine-idle within ${WAIT_LIMIT_MS} ms`;
    if (afterIdle && out.error === null && out.state === 'engine-idle') {
      await afterIdle({ send: S, evaluate: ev });
      const { engineLog: after, ...fin2 } = await ev(FINAL);
      engineLog = after;
      Object.assign(out, fin2, { engineLog30: after ? after.slice(0, 30) : null });
    }
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (navAt !== null) out.elapsedMs = Date.now() - navAt;
    out.loadavg = os.loadavg();
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  return { out, engineLog };
}

/** The middle of the numbers (the lower of the two middle ones for an even count); null when there is no number. */
const median = (xs) => {
  const s = xs.filter((x) => typeof x === 'number').sort((a, b) => a - b);
  return s.length === 0 ? null : s[Math.floor((s.length - 1) / 2)];
};

/** Why a warm start is not a good start (null when it is): its error, else a state other than engine-idle, else the STAND-IN text. */
const warmFailure = (w) => w.error ?? (w.state !== 'engine-idle' ? `warm start: state ${w.state}` : null) ?? (w.standIn === true ? 'warm start: STAND-IN' : null);

/** The check's JSON from its starts: cold (the first start) and warms (the later starts). elapsedMs is the median of the warms' elapsedMs, with the other fields of that start; samples are the warms' elapsedMs in the order measured; coldMs is the cold start's elapsedMs. error is the cold start's (its stop is not hidden), else the first warm start that failed, else the cold start's being too slow. */
export function composeBoot(cold, warms) {
  const coldError = cold.error ?? (cold.state !== 'engine-idle' ? `cold start: state ${cold.state}` : null);
  const slow = cold.elapsedMs > COLD_LIMIT_MS ? `cold boot too slow: ${cold.elapsedMs} ms (limit ${COLD_LIMIT_MS} ms)` : null;
  const warmError = warms.map(warmFailure).find((e) => e !== null) ?? null;
  const samples = warms.map((w) => w.elapsedMs);
  const mid = median(samples);
  const pick = warms.find((w) => w.elapsedMs === mid) ?? warms[warms.length - 1];
  return { ...pick, elapsedMs: mid, samples, coldMs: cold.elapsedMs, error: coldError ?? warmError ?? slow };
}

// the warm starts after the cold one: their median is the check's elapsedMs
export const WARM_STARTS = 3;

/** The bundle started once cold, then WARM_STARTS times warm, each start in its own Chrome: `boot` is bootBundle (a spec passes a fake). */
export async function bootSeries(bundleDir, boot = bootBundle) {
  const { out: cold } = await boot(bundleDir);
  const warms = [];
  for (let i = 0; i < WARM_STARTS; i++) warms.push((await boot(bundleDir)).out);
  return composeBoot(cold, warms);
}

async function main() {
  if (!process.argv[2]) {
    process.stderr.write('usage: node tools/boot-check/boot-bundle.mjs <absolute path to the bundle directory>\n');
    process.exit(2);
  }
  const out = await bootSeries(path.resolve(process.argv[2]));
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
  process.exitCode = out.error ? 1 : 0;
}

// the check runs when this file is the program; a tool that imports it gets bootBundle only
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
