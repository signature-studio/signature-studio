// (d) browser-face-tabs (Issue #1419): the Browser panel's WebView2 face (app/src/ui/webview2-surface.ts) is shown and its tabs are switched. The Welcome is closed (the default
// project opens), then the first tab and the cloud tab are pressed three times (the cloud tab's Navigate loads the offline page into the face), and the page waits 10 s.
// The page must log no console error. The face's documents are kept from what the frame and the page's CSP stop (webview2-surface.ts: withoutScripts, withoutCspBlocked):
// a script logs "Blocked script execution in 'about:srcdoc'", an inline style attribute or a data: font logs a CSP error, each on every load of the document.
// Reported, not judged: the face's iframes, the loads of the face's documents, and the engine's Navigate lines (with their URIs) and fire lines (the engine's log, up to its 600-line cap).
import { closeWelcome, dismissProblems, noThanks, sleep, waitProjectOpen, waitStable } from '../../lib/flow.mjs';

const FACE_ATTR = 'data-fl-webview2';
// the Browser panel's two tabs, in the guest's pixels at DPR 1 (tools/browser-panel/measure.mjs: the first tab 35,125, the cloud tab 267,125). The layout is in CSS
// pixels of the desktop (a DPR 2 guest has two guest pixels per CSS pixel), so the tabs are scaled by guest px per CSS px (as dismissProblems does)
const TAB_OTHER = { gx: 35, gy: 125 };
const TAB_CLOUD = { gx: 267, gy: 125 };
const SWITCHES = 3;
const SWITCH_SETTLE_MS = 1500;
const SETTLE_MS = 10000;

const FACE_LOADS = `(() => { window.__e2eFaceLoads = 0; document.addEventListener('load', (e) => { const t = e.target; if (t && t.hasAttribute && t.hasAttribute('${FACE_ATTR}')) window.__e2eFaceLoads++; }, true); return true; })()`;
const FACE_COUNT = `document.querySelectorAll('iframe[${FACE_ATTR}]').length`;
const NAVIGATE = /^(?:\[err\] )?webview2: Navigate \(/;
const FIRE = /^(?:\[err\] )?webview2: fire /;

export default {
  name: 'browser-face-tabs',
  expectFail: null,
  async run({ guest, page, log }) {
    const notes = [];
    const first = await waitStable(guest, 180000);
    notes.push(`first state ${first.state} after ${first.waitedMs} ms, title "${first.title}"`);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes };
    await page.evaluate(FACE_LOADS);
    await noThanks(guest);
    await closeWelcome(guest);
    const open = await waitProjectOpen(guest, first.title, 120000);
    notes.push(`project open ${open.opened} (waited ${open.waitedMs} ms): state ${open.state}`);
    if (!open.opened) return { ok: false, engineState: open.state, notes };
    await dismissProblems(guest);
    const faces = await page.evaluate(FACE_COUNT);
    const k = await guest.guestPerCss();
    let presses = 0;
    for (let i = 0; i < SWITCHES; i++) {
      await guest.click(TAB_OTHER.gx * k, TAB_OTHER.gy * k);
      await sleep(SWITCH_SETTLE_MS);
      await guest.click(TAB_CLOUD.gx * k, TAB_CLOUD.gy * k);
      await sleep(SWITCH_SETTLE_MS);
      presses += 2;
    }
    await sleep(SETTLE_MS);
    const end = (await guest.state().catch(() => null)) ?? { state: 'unreadable', error: null };
    const lines = (await guest.ev('window.__app.engineLog()').catch(() => null)) ?? [];
    const navigates = lines.filter((l) => NAVIGATE.test(l)).length;
    const fires = lines.filter((l) => FIRE.test(l)).length;
    const loads = await page.evaluate('window.__e2eFaceLoads');
    const endFaces = await page.evaluate(FACE_COUNT);
    const navigateUris = lines.filter((l) => NAVIGATE.test(l)).map((l) => l.slice(0, 200));
    const results = { presses, facesAtOpen: faces, facesAtEnd: endFaces, faceLoads: loads, navigateLines: navigates, navigateUris, fireLines: fires, engineLogLines: lines.length };
    log(`faces ${endFaces}, face loads ${loads}, Navigate lines ${navigates}, fire lines ${fires}, engine ${end.state}`);
    notes.push(`after ${presses} tab presses and ${SETTLE_MS} ms: state ${end.state}, faces ${endFaces}, face loads ${loads}, Navigate lines ${navigates}`);
    const ok = end.state === 'engine-idle' && end.error === null && faces >= 1 && endFaces >= 1;
    return { ok, engineState: end.state, notes, results };
  },
};
