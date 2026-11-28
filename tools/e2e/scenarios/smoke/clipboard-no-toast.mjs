// (i) clipboard-no-toast (Issue #1482): the clipboard permissions are 'denied'. A paste (Ctrl+V) and a copy (Ctrl+C) on the desktop show no notice: no visible live region
// (role=status, role=alert or aria-live) with text, sampled at once after each key and again after a moment, and the page has no #clipboard-status element. The page's own
// persistent regions (#status, #audio-status, #taskbar: detect.mjs) are not counted. The page's clipboard record (window.__app.clipboard: the reads, the writes and the reason
// kept for the record) is written to the JSON; the reason is never shown.
import { closeWelcome, dismissProblems, noThanks, sleep, waitProjectOpen, waitStable, WELCOME } from '../../lib/flow.mjs';
import { permissionStates, toastState } from '../../lib/detect.mjs';

const EXEMPT = ['status', 'audio-status', 'taskbar'];
const LIVE = `(() => {
  const exempt = ${JSON.stringify(EXEMPT)};
  const shown = [];
  for (const el of document.querySelectorAll('[role="status"],[role="alert"],[aria-live]')) {
    if (exempt.includes(el.id)) continue;
    if (!el.textContent.trim()) continue;
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    if (el.hidden || cs.display === 'none' || cs.visibility === 'hidden' || Number(cs.opacity) === 0 || r.width <= 0 || r.height <= 0) continue;
    shown.push(el.id || el.getAttribute('role') || el.tagName);
  }
  return { shown, clipboardStatusElement: !!document.querySelector('#clipboard-status') };
})()`;
const RECORD = `(() => { const c = window.__app ? window.__app.clipboard : null; return c ? { reads: c.reads, writes: c.writes, readPermission: c.readPermission, writePermission: c.writePermission, refusal: c.refusal } : null; })()`;

async function setDenied(browserCdp, browserContextId, origin) {
  for (const name of ['clipboard-read', 'clipboard-write']) {
    await browserCdp.send('Browser.setPermission', { permission: { name }, setting: 'denied', origin, browserContextId });
  }
}

export default {
  name: 'clipboard-no-toast',
  expectFail: null,
  async beforeLoad({ browserCdp, browserContextId, origin }) {
    await setDenied(browserCdp, browserContextId, origin);
  },
  async run({ guest, page, log }) {
    const notes = [];
    const first = await waitStable(guest, 180000);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes: [`first state ${first.state}`] };
    if (await guest.findWindow((w) => w.cls === WELCOME)) {
      await noThanks(guest);
      await closeWelcome(guest);
    }
    const end = await waitProjectOpen(guest, first.title, 120000);
    if (!end.opened) return { ok: false, engineState: end.state, notes: [...notes, 'the default project did not open'] };
    await dismissProblems(guest);
    const permissionBefore = await permissionStates(page);
    await guest.focusDesktop();
    const samples = [];
    const sample = async (at) => samples.push({ at, ...(await page.evaluate(LIVE)) });
    await sample('before');
    await guest.ctrl('KeyV', 'v', 0x56);
    await sample('after Ctrl+V');
    await guest.ctrl('KeyC', 'c', 0x43);
    await sample('after Ctrl+C');
    await sleep(6000);
    await sample('6 s later');
    const toast = await toastState(page);
    const record = await page.evaluate(RECORD);
    const permissionAfter = await permissionStates(page);
    const shownAny = samples.some((s) => s.shown.length > 0);
    const statusElement = samples.some((s) => s.clipboardStatusElement);
    notes.push(`visible live regions with text, per sample: ${samples.map((s) => `${s.at} ${s.shown.length}`).join(', ')}`);
    notes.push(`#clipboard-status element: ${statusElement ? 'yes' : 'no'}; the page's live-region watch shows ${toast.visible} (${JSON.stringify(toast.ids_visible ?? [])})`);
    notes.push(`clipboard record: ${JSON.stringify(record)}; permission before ${JSON.stringify(permissionBefore)}, after ${JSON.stringify(permissionAfter)}`);
    log(`notices: ${shownAny || statusElement ? 'yes' : 'none'}; record ${JSON.stringify(record)}`);
    const results = [{ samples, toastVisible: toast.visible, record, permissionBefore, permissionAfter }];
    const ok = !shownAny && !statusElement && toast.visible === 0;
    return { ok, engineState: first.state, notes, results };
  },
};
