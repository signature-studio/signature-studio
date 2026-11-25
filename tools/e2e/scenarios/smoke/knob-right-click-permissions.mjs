// (b) knob-right-click-permissions: a right click on the Volume knob of the first channel in the Channel rack, under each clipboard permission state
// (prompt, denied, granted). FL's context menu must open; the page must not read the clipboard (navigator.clipboard.read / readText: 0 calls); the
// page's status line must stay hidden (no visible role=status region); no clipboard permission may change. Issue #1406 (the permission notice does not go away) is the expected failure.
import { closeWelcome, dismissProblems, noThanks, sleep, WELCOME, waitProjectOpen, waitStable } from '../../lib/flow.mjs';
import { clipboardReads, countClipboardReads, permissionStates, toastState } from '../../lib/detect.mjs';

// The Channel rack is part of the main form (the window table lists the main form, TFruityLoopsMainForm, over the whole guest); the first channel's
// Volume knob is the large dial right of its LED: guest pixel (406, 145) from the main form's top-left corner (read from the picture of the default project
// at the room of 3840 x 2160; the pan dial is the smaller one right of it).
const MAIN_FORM = 'TFruityLoopsMainForm';
const MENU = 'TQuickPopupMenuWindow';
const REMIX = 'TDemucsForm';
export const VOLUME_KNOB = { dx: 406, dy: 145 };
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

export default {
  name: 'knob-right-click-permissions',
  expectFail: '#1406',
  async run({ guest, page, browserCdp, browserContextId, origin, log }) {
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
    // the "Remix a song" dialog of the default project (a window of its own) is closed with Escape: it takes the clicks of the rack otherwise
    if (await guest.findWindow((w) => w.cls === REMIX)) {
      await guest.escape();
      await guest.waitFor(async () => ((await guest.findWindow((w) => w.cls === REMIX)) ? null : true), 5000, 'the Remix dialog closes');
      notes.push('the "Remix a song" dialog of the default project was closed with Escape');
    }
    const main = await guest.findWindow((w) => w.cls === MAIN_FORM);
    if (!main) return { ok: false, engineState: end.state, notes: [...notes, 'no main form window in the window table'] };
    notes.push(`the project in front: title "${end.title}" (the Welcome closed with Escape; its project is the one the Welcome had selected)`);
    await countClipboardReads(page);
    const results = [];
    const k = await guest.guestPerCss();
    for (const st of STATES) {
      await setPermission(browserCdp, browserContextId, origin, st);
      await sleep(300);
      const before = await permissionStates(page);
      const reads0 = await clipboardReads(page);
      await guest.rightClick(main.x + VOLUME_KNOB.dx * k, main.y + VOLUME_KNOB.dy * k);
      await sleep(1500);
      const menu = await guest.findWindow((w) => w.cls === MENU);
      const reads1 = await clipboardReads(page);
      const toast = await toastState(page);
      const visibleIds = toast.ids_visible ?? [];
      const after = await permissionStates(page);
      const r = {
        permission: st,
        before, after,
        menuShown: !!menu,
        clipboardReads: (reads1.read - reads0.read) + (reads1.readText - reads0.readText),
        visibleStatusRegions: toast.visible,
        visibleIds,
        permissionChanges: (before.read !== after.read ? 1 : 0) + (before.write !== after.write ? 1 : 0),
      };
      results.push(r);
      log(`knob ${st}: menu ${r.menuShown}, reads ${r.clipboardReads}, visible status ${r.visibleStatusRegions}, permission changes ${r.permissionChanges}`);
      if (menu) await guest.escape();
      await sleep(800);
    }
    const ok = results.every((r) => r.menuShown && r.clipboardReads === 0 && r.visibleStatusRegions === 0 && r.permissionChanges === 0);
    notes.push(`visible status regions: ${JSON.stringify(results.map((r) => r.visibleIds))}`);
    notes.push(...results.map((r) => `${r.permission}: menu ${r.menuShown}, clipboard reads ${r.clipboardReads}, visible status regions ${r.visibleStatusRegions}, permission changes ${r.permissionChanges}`));
    return { ok, engineState: (await guest.state().catch(() => null))?.state ?? null, notes, results };
  },
};
