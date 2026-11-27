// R06 (#1408) clipboard-permission-knob. Report (L-071, #1406): a right click on a plugin knob shows FL's menu; the page must not read the clipboard, must not show
// a notice, and a notice must not stay. Expectation, for each permission (prompt, denied, granted): the menu opens; the page's clipboard reads during the right
// click are 0; no live region is visible 1.5 s after the click, and none 5 s after it; no clipboard permission changes.
import { clipboardReads, countClipboardReads, permissionStates, toastState } from '../../lib/detect.mjs';
import { sleep } from '../../lib/flow.mjs';
import { MAIN, newTops, POPUP, startDefault, table, tops, result } from './common.mjs';

const SCENARIO = {
  name: 'clipboard-permission-knob',
  report: 'R06',
  expect: 'knob right click, for prompt, denied and granted: the menu opens; 0 clipboard reads; no visible notice at 1.5 s, and none 5 s later (at 6.5 s); no permission changes',
  expectFail: null,
};
const VOLUME_KNOB = { dx: 406, dy: 145 };
const STATES = ['prompt', 'denied', 'granted'];

async function setPermission(browserCdp, browserContextId, origin, state) {
  if (state === 'prompt') { await browserCdp.send('Browser.resetPermissions', { browserContextId }); return; }
  for (const name of ['clipboard-read', 'clipboard-write']) {
    await browserCdp.send('Browser.setPermission', { permission: { name }, setting: state, origin, browserContextId });
  }
}

export default {
  ...SCENARIO,
  async run({ guest, page, browserCdp, browserContextId, origin, log }) {
    const start = await startDefault(guest, log);
    if (!start.ok) return result(false, start.state, start.notes);
    const main = tops(await table(guest)).find((w) => w.cls === MAIN);
    if (!main) return result(false, start.state, [...start.notes, 'no main form']);
    await countClipboardReads(page);
    const rows = [];
    for (const st of STATES) {
      await setPermission(browserCdp, browserContextId, origin, st);
      await sleep(300);
      const perm0 = await permissionStates(page);
      const lids = (await table(guest)).map((w) => w.lid);
      const reads0 = await clipboardReads(page);
      const kk = await guest.guestPerCss();
      await guest.rightClick(main.ax + VOLUME_KNOB.dx * kk, main.ay + VOLUME_KNOB.dy * kk);
      await sleep(1500);
      const menu = newTops(await table(guest), lids).some((w) => w.cls === POPUP);
      const reads1 = await clipboardReads(page);
      const visible1 = (await toastState(page)).visible;
      await sleep(5000);
      const visible5 = (await toastState(page)).visible;
      const perm1 = await permissionStates(page);
      const calls = { read: reads1.read - reads0.read, readText: reads1.readText - reads0.readText };
      const reads = calls.read + calls.readText;
      const permChanged = (perm0.read !== perm1.read ? 1 : 0) + (perm0.write !== perm1.write ? 1 : 0);
      rows.push({ permission: st, menu, reads, calls, visible1, visible5, permChanged });
      log(`knob ${st}: menu ${menu}, reads ${reads}, visible at 1.5 s ${visible1}, at 6.5 s ${visible5}, permission changes ${permChanged}`);
      if (menu) await guest.escape();
      await sleep(800);
    }
    const ok = rows.every((r) => r.menu && r.reads === 0 && r.visible1 === 0 && r.visible5 === 0 && r.permChanged === 0);
    const notes = [...start.notes, ...rows.map((r) => `${r.permission}: menu ${r.menu}, clipboard reads ${r.reads} (read() ${r.calls.read}, readText() ${r.calls.readText}), visible notices ${r.visible1} (1.5 s) ${r.visible5} (6.5 s), permission changes ${r.permChanged}`)];
    return result(ok, (await guest.state())?.state ?? null, notes, { rows });
  },
};
