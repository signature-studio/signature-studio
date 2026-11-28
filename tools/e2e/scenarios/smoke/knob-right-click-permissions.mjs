// (b) knob-right-click-permissions: a right click on the Volume knob of the first channel in the Channel rack, under each clipboard permission state
// (prompt, denied, granted). FL's context menu must open; the page must not read the clipboard (navigator.clipboard.read / readText: 0 calls); the
// page must show no notice (no visible role=status / role=alert / aria-live region: Issue #1482 removed the notice); no clipboard permission may change.
import { clearProblems, closeWelcome, dismissProblems, noThanks, sleep, WELCOME, waitProjectOpen, waitStable } from '../../lib/flow.mjs';
import { clipboardReads, countClipboardReads, permissionStates, toastState } from '../../lib/detect.mjs';

// The Channel rack is part of the main form (the window table lists the main form, TFruityLoopsMainForm, over the whole guest); the first channel's
// Volume knob is the large dial right of its LED: guest pixel (406, 145) from the main form's top-left corner (read from the picture of the default project
// at the room of 3840 x 2160; the pan dial is the smaller one right of it).
const MAIN_FORM = 'TFruityLoopsMainForm';
const MENU = 'TQuickPopupMenuWindow';
const REMIX = 'TDemucsForm';
export const VOLUME_KNOB = { dx: 406, dy: 145 };
// the place was measured at these DPRs only: the menu must open there (in each state). At another DPR the place is not verified (a run at 2.73 missed the
// knob twice, in every state), so the menu is not required there; the clipboard counters are judged at every DPR.
export const CALIBRATED_DPRS = [1, 2];
const calibrated = (dpr) => CALIBRATED_DPRS.some((c) => Math.abs(c - dpr) < 1e-3);
const STATES = ['prompt', 'denied', 'granted'];

async function setPermission(browserCdp, browserContextId, origin, state) {
  if (state === 'prompt') {
    await browserCdp.send('Browser.resetPermissions', { browserContextId });
    return;
  }
  for (const name of ['clipboard-read', 'clipboard-write']) {
    await browserCdp.send('Browser.setPermission', { permission: { name }, setting: state, origin, browserContextId });
  }
}

/** what takes the clicks of the rack: the "Problems loading the project" box (answered with Enter by clearProblems, which finds it by its class) and the
 *  "Remix a song" dialog (closed with Escape). `settleMs` keeps watching for a box that comes after the last one. Returns the answers and the dialog's closing */
async function clearBlockers(guest, settleMs = 0) {
  const problems = await clearProblems(guest, { settleMs, limitMs: 20000 });
  const remix = !!(await guest.findWindow((w) => w.cls === REMIX));
  if (remix) {
    await guest.escape();
    await guest.waitFor(async () => ((await guest.findWindow((w) => w.cls === REMIX)) ? null : true), 5000, 'the Remix dialog closes');
  }
  return { problems, remix };
}

/** a right click on the Volume knob, at the main form as the window table has it now; returns the menu, or null */
async function rightClickKnob(guest, main, k) {
  const at = (await guest.findWindow((w) => w.cls === MAIN_FORM)) ?? main;
  await guest.rightClick(at.x + VOLUME_KNOB.dx * k, at.y + VOLUME_KNOB.dy * k);
  await sleep(1500);
  return guest.findWindow((w) => w.cls === MENU);
}

/** the judge: no clipboard read, no visible notice and no permission change happens in any of the three states (and there are the three). The menu must open
 *  in each state at a calibrated DPR (the default is 1); at another DPR the menu is not required, the states where it opened are judged by the same counters */
export function judgeKnob(results, dpr = 1) {
  if (results.length !== STATES.length) return false;
  const counted = results.every((r) => r.clipboardReads === 0 && r.visibleStatusRegions === 0 && r.permissionChanges === 0);
  return counted && (!calibrated(dpr) || results.every((r) => r.menuShown));
}

/** the notes of a DPR that is not calibrated: the states where the menu did not open at the knob's place (none at a calibrated DPR) */
export function positionNotes(dpr, results) {
  if (calibrated(dpr)) return [];
  const missing = results.filter((r) => !r.menuShown).map((r) => r.permission);
  if (missing.length === 0) return [];
  const where = missing.length === STATES.length ? '' : ` in ${missing.join(', ')}`;
  return [`position-unverified at DPR ${dpr}: the menu did not open at the calibrated place${where}; the clipboard counters are judged only (reads 0, status 0, permission changes 0)`];
}

export default {
  name: 'knob-right-click-permissions',
  expectFail: null,
  displays: ['4k-dpr1', '4k-dpr2'],
  async run({ guest, page, browserCdp, browserContextId, origin, log }) {
    const notes = [];
    const dpr = await page.evaluate(() => window.devicePixelRatio);
    if (!calibrated(dpr)) notes.push(`the place of the Volume knob is calibrated at DPR 1 and 2 only (this run: DPR ${dpr}): the menu is judged only where it opens; the clipboard counters are judged at every DPR`);
    const first = await waitStable(guest, 180000);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes: [`first state ${first.state}`] };
    if (await guest.findWindow((w) => w.cls === WELCOME)) {
      await noThanks(guest);
      await closeWelcome(guest);
    }
    const end = await waitProjectOpen(guest, first.title, 120000);
    if (!end.opened) return { ok: false, engineState: end.state, notes: [...notes, 'the default project did not open'] };
    const byOk = await dismissProblems(guest);
    const start = await clearBlockers(guest, 2000);
    notes.push(`start blockers: the Problems box closed by its OK: ${byOk}; Problems answered after it: ${start.problems}; the "Remix a song" dialog of the default project found and closed with Escape: ${start.remix}`);
    const main = await guest.findWindow((w) => w.cls === MAIN_FORM);
    if (!main) return { ok: false, engineState: end.state, notes: [...notes, 'no main form window in the window table'] };
    notes.push(`the window in front after the start: title "${(await guest.state())?.title}" (the Welcome closed with Escape; its project is the one the Welcome had selected)`);
    await countClipboardReads(page);
    const results = [];
    const k = await guest.guestPerCss();
    for (const st of STATES) {
      await setPermission(browserCdp, browserContextId, origin, st);
      await sleep(300);
      const before = await permissionStates(page);
      const reads0 = await clipboardReads(page);
      const blockers = await clearBlockers(guest);
      let menu = await rightClickKnob(guest, main, k);
      // no menu: the blockers are cleared again and the same place is clicked once more (a box that has just closed can take the first press)
      const retried = !menu;
      if (retried) {
        await clearBlockers(guest);
        menu = await rightClickKnob(guest, main, k);
      }
      const reads1 = await clipboardReads(page);
      const toast = await toastState(page);
      const visibleIds = toast.ids_visible ?? [];
      const after = await permissionStates(page);
      const r = {
        permission: st,
        before, after,
        menuShown: !!menu,
        retried,
        blockers,
        clipboardReads: (reads1.read - reads0.read) + (reads1.readText - reads0.readText),
        visibleStatusRegions: toast.visible,
        visibleIds,
        permissionChanges: (before.read !== after.read ? 1 : 0) + (before.write !== after.write ? 1 : 0),
      };
      results.push(r);
      log(`knob ${st}: menu ${r.menuShown}${retried ? ' (second right click)' : ''}, reads ${r.clipboardReads}, visible status ${r.visibleStatusRegions}, permission changes ${r.permissionChanges}`);
      if (menu) await guest.escape();
      await sleep(800);
    }
    const ok = judgeKnob(results, dpr);
    if (ok) notes.push(...positionNotes(dpr, results));
    notes.push(`visible status regions: ${JSON.stringify(results.map((r) => r.visibleIds))}`);
    notes.push(...results.map((r) => `${r.permission}: menu ${r.menuShown}${r.retried ? ' (second right click)' : ''}, clipboard reads ${r.clipboardReads}, visible status regions ${r.visibleStatusRegions}, permission changes ${r.permissionChanges}; blockers before the click ${JSON.stringify(r.blockers)}`));
    return { ok, engineState: (await guest.state().catch(() => null))?.state ?? null, notes, results };
  },
};
