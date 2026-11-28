// The detections of one scenario, all counted over the whole run of the scenario:
//   pageErrors / consoleErrors: the page's uncaught errors and console.error messages (the page's own console; the engine's Worker logs into the engine log).
//   failedRequests: requests that failed (requestfailed) and responses of 4xx or 5xx.
//   stuckToasts: a live region (role=status, role=alert, aria-live) that is visible with text for more than 10 s without a change. The three persistent
//     status regions of the page are not counted: #status (the engine's status line), #audio-status and #taskbar (both are on the page at all times).
//   engineStops: the engine reported exited or error. engineStuckRunning: the engine stayed in `running` for more than 90 s.
//   permissionChanges: a clipboard permission whose state changed during an action that must not change it (the scenario says which).
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const MAX_DETAILS = 40;

/** page events: attached when the page is made, read at the end */
export function attachDetectors(page) {
  const d = { pageErrors: 0, consoleErrors: 0, failedRequests: 0, details: [] };
  const note = (s) => { if (d.details.length < MAX_DETAILS) d.details.push(s.slice(0, 300)); };
  page.on('pageerror', (e) => { d.pageErrors++; note(`pageerror: ${String(e?.message ?? e)}`); });
  page.on('console', (m) => { if (m.type() === 'error') { d.consoleErrors++; note(`console.error: ${m.text()} (at ${m.location().url || '-'})`); } });
  page.on('requestfailed', (r) => { d.failedRequests++; note(`requestfailed: ${r.method()} ${r.url()} ${r.failure()?.errorText ?? ''}`); });
  page.on('response', (r) => { if (r.status() >= 400) { d.failedRequests++; note(`HTTP ${r.status()}: ${r.url()}`); } });
  return d;
}

const TOAST_WATCH = `(() => {
  const EXEMPT = ['status', 'audio-status', 'taskbar'];
  const LIMIT = 10000;
  const since = new Map();
  const stuck = new Set();
  const ids = [];
  const info = [];
  const visible = (el) => {
    if (el.hidden) return false;
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden' || Number(cs.opacity) === 0) return false;
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  };
  window.__e2eToast = { stuck: 0, ids: [], info: [], visible: 0, ids_visible: [] };
  setInterval(() => {
    const now = performance.now();
    const alive = new Set();
    let shown = 0;
    const shownIds = [];
    for (const el of document.querySelectorAll('[role="status"],[role="alert"],[aria-live]')) {
      if (EXEMPT.includes(el.id)) continue;
      const text = el.textContent.trim();
      if (!text || !visible(el)) continue;
      shown++;
      shownIds.push(el.id || el.getAttribute('role'));
      alive.add(el);
      const s = since.get(el);
      if (!s || s.text !== text) since.set(el, { text, t0: now });
      else if (now - s.t0 > LIMIT && !stuck.has(el)) {
        stuck.add(el);
        ids.push(el.id || el.getAttribute('role') || el.tagName);
        info.push({ id: el.id || null, role: el.getAttribute('role'), tag: el.tagName, dataError: el.dataset.error ?? null, text: text.slice(0, 160), textFirstSeenAtMs: Math.round(s.t0), markedAtMs: Math.round(now) });
      }
    }
    for (const el of [...since.keys()]) if (!alive.has(el)) since.delete(el);
    window.__e2eToast.stuck = stuck.size;
    window.__e2eToast.ids = ids.slice(0, 20);
    window.__e2eToast.info = info.slice(0, 20);
    window.__e2eToast.visible = shown;
    window.__e2eToast.ids_visible = shownIds.slice(0, 20);
  }, 250);
  return true;
})()`;

/** the live-region watch of the page (installed once the document exists) */
export async function installToastWatch(page) {
  await page.waitForFunction(() => !!document.body, null, { timeout: 30000 });
  await page.evaluate(TOAST_WATCH);
}

export const toastState = (page) => page.evaluate(() => (window.__e2eToast ? { ...window.__e2eToast } : { stuck: 0, ids: [], info: [], visible: 0, ids_visible: [] }));

const CLIP_COUNT = `(() => { const c = navigator.clipboard; window.__e2eClip = { read: 0, readText: 0 }; for (const k of ['read', 'readText']) { const f = c[k].bind(c); c[k] = (...a) => { window.__e2eClip[k]++; return f(...a); }; } return true; })()`;
export async function countClipboardReads(page) { await page.evaluate(CLIP_COUNT); }
export const clipboardReads = (page) => page.evaluate(() => (window.__e2eClip ? { ...window.__e2eClip } : { read: 0, readText: 0 }));

export const permissionStates = (page) => page.evaluate(() => Promise.all(['clipboard-read', 'clipboard-write'].map((n) => navigator.permissions.query({ name: n }).then((p) => p.state, () => 'unknown'))).then(([read, write]) => ({ read, write })));

/** the engine's state, watched every 250 ms: the stops, and the time the state has been `running` (the longest run of it) */
export function watchEngine(guest) {
  const w = { stops: 0, stopReason: null, exit: null, runningMs: 0, stuckRunning: false, last: null, error: null };
  let runStart = null;
  let busy = false;
  const timer = setInterval(async () => {
    if (busy) return;
    busy = true;
    try {
      const s = await guest.state();
      const now = Date.now();
      if (s && s.state === 'running') {
        if (runStart === null) runStart = now;
        const ms = now - runStart;
        if (ms > w.runningMs) w.runningMs = ms;
        // this runner has no GPU and the 4K start is slow (DPR 2.86: 24 s to the first idle); a start that never ends is the boot check's (boot-bundle.mjs, tools/boot-hang)
        if (ms > 90000) w.stuckRunning = true;
      } else runStart = null;
      if (s && (s.state === 'exited' || s.state === 'error') && (w.last === null || w.last.state !== s.state)) {
        w.stops++;
        w.stopReason = s.exit ? `${s.exit.reason} code ${s.exit.code}: ${s.exit.message}` : (s.error ?? s.state);
        // kept at the transition: the engine may be started again before its stop report is read
        w.exit = s.exit && s.exit.reason ? { reason: s.exit.reason, code: s.exit.code, message: s.exit.message ?? '' } : null;
      }
      w.last = s;
    } catch (e) {
      w.error = String(e?.message ?? e).slice(0, 200);
    }
    busy = false;
  }, 250);
  return { w, stop: async () => { clearInterval(timer); for (let i = 0; i < 40 && busy; i++) await sleep(50); return w; } };
}
