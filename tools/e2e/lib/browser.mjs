// The browser: Playwright's Chromium (headless shell), the display (3840 x 2160 physical pixels), and the frame clock.
// 120 Hz (method i): the headless shell runs with --enable-begin-frame-control and the frames are driven by CDP HeadlessExperimental.beginFrame,
// sent every 1000/120 ms on the wall clock (no catch-up: a frame that takes longer is followed by the next tick). Each frame calls the page's
// requestAnimationFrame callbacks. achievedFps is what the page's own rAF counter (lib/browser.mjs rafCounter) reads per second: nothing is assumed.
import { chromium } from 'playwright';

export const DISPLAY = { width: 3840, height: 2160, refreshHzTarget: 120 };

// no network, no updates, no phishing or safe-browsing traffic: the same isolation as tools/boot-check/boot-bundle.mjs
const ISOLATION = [
  '--password-store=basic',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1',
  '--disable-ipv6', '--disable-quic',
  '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-domain-reliability',
  '--no-pings', '--metrics-recording-only', '--disable-client-side-phishing-detection', '--disable-breakpad', '--disable-extensions', '--safebrowsing-disable-auto-update', '--disable-search-engine-choice-screen', '--no-service-autorun',
  '--disable-features=OptimizationHints,MediaRouter,Translate,CertificateTransparencyComponentUpdater,NetworkTimeServiceQuerying,AutofillServerCommunication,OptimizationGuideModelDownloading,SafeBrowsingEnhancedProtection,PrivacySandboxSettings4,Variations,SafeBrowsing,SafeBrowsingHashPrefixRealTimeLookups,HashPrefixRealTimeLookups,SafeBrowsingOnUIThread,SafeBrowsingNewGmsApiForBrowseUrlDatabaseCheck',
  '--variations-server-url=http://127.0.0.1:9/', '--disable-field-trial-config',
  '--force-color-profile=srgb', '--disable-lcd-text', '--mute-audio',
];

/** the frame clock: 'begin' (BeginFrame control, lib/browser.mjs startFrameClock) or 'raf' (the page's own rAF, the frame-rate limit off). The loopback is
 *  sent through the proxy (lib/proxy.mjs: the access token goes with every request, as the edge of the access service does in production) */
export function browserArgs(clock, proxyPort) {
  const window = `--window-size=${DISPLAY.width},${DISPLAY.height}`;
  const proxy = `--proxy-server=http://127.0.0.1:${proxyPort}`;
  const via = [proxy, '--proxy-bypass-list=<-loopback>'];
  if (clock === 'begin') return [window, '--enable-begin-frame-control', '--run-all-compositor-stages-before-draw', ...via, ...ISOLATION];
  return [window, '--disable-frame-rate-limit', '--disable-gpu-vsync', ...via, ...ISOLATION];
}

export async function launchBrowser(clock = 'raf', proxyPort) {
  return chromium.launch({ headless: true, args: browserArgs(clock, proxyPort) });
}

/** a page of its own (a new context: its own profile, permissions and headers). The viewport is the CSS size of the display at this DPR. */
export async function newPage(browser, { cssWidth, cssHeight, dpr, origin, token }) {
  const context = await browser.newContext({
    viewport: { width: cssWidth, height: cssHeight },
    deviceScaleFactor: dpr,
    screen: { width: cssWidth, height: cssHeight },
    extraHTTPHeaders: { 'Cf-Access-Jwt-Assertion': token },
  });
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);
  const { targetInfo } = await cdp.send('Target.getTargetInfo');
  return { context, page, cdp, browserContextId: targetInfo.browserContextId, origin };
}

/** the 120 Hz frame clock of one page: BeginFrame every 1000/hz ms on the wall clock; a late tick is not made up */
export async function startFrameClock(cdp, hz = DISPLAY.refreshHzTarget) {
  await cdp.send('HeadlessExperimental.enable');
  const interval = 1000 / hz;
  let running = true;
  let frames = 0, errors = 0, lastError = null;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const loop = (async () => {
    let next = performance.now();
    while (running) {
      next += interval;
      const wait = next - performance.now();
      if (wait > 0) await sleep(wait); else next = performance.now();
      if (!running) break;
      try {
        await cdp.send('HeadlessExperimental.beginFrame', { interval, noDisplayUpdates: false });
        frames++;
      } catch (e) {
        errors++; lastError = String(e?.message ?? e).slice(0, 200);
        await sleep(50);
      }
    }
  })();
  return {
    stop: async () => { running = false; await loop; },
    stats: () => ({ frames, errors, lastError }),
  };
}

/** installs the page's rAF counter (one callback per frame; it changes nothing else) */
export async function installRafCounter(page) {
  await page.evaluate(() => {
    window.__e2eRaf = { n: 0 };
    const tick = () => { window.__e2eRaf.n++; requestAnimationFrame(tick); };
    requestAnimationFrame(tick);
    return true;
  });
}

/** reads the counter once: { n, t } (t: the page's performance.now()) */
export const readRaf = (page) => page.evaluate(() => ({ n: window.__e2eRaf ? window.__e2eRaf.n : 0, t: performance.now() }));

/** samples the rAF counter once a second; stop() gives the frames and the seconds of the samples (the average is frames / seconds) */
export function startRafSampler(page) {
  let last = null;
  let frames = 0, seconds = 0;
  let busy = false;
  const timer = setInterval(async () => {
    if (busy) return;
    busy = true;
    try {
      const now = await readRaf(page);
      if (last && now.t > last.t) { frames += now.n - last.n; seconds += (now.t - last.t) / 1000; }
      last = now;
    } catch { /* the page is navigating or closed: the next sample tries again */ }
    busy = false;
  }, 1000);
  return {
    stop: async () => {
      clearInterval(timer);
      for (let i = 0; i < 50 && busy; i++) await new Promise((r) => setTimeout(r, 50));
      return { frames, seconds };
    },
  };
}
