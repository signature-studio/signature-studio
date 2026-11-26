// (h) clipboard-load-permission (Issue #1482): the clipboard permissions are 'prompt' when the page loads (Browser.setPermission, not a granted context). The page asks for the
// read permission at its load: navigator.clipboard.readText() under 'prompt' is the browser's request (the browser asks the person). A request the browser refuses at the
// load (the document has no focus) is made again at the first press or key. The requests are counted by a wrapper of navigator.clipboard.readText that is set before the
// page's scripts run (beforeLoad). Judged: at least one readText request by the time of the first click on the desktop. Recorded: the requests at the load, the requests
// after the click, and the permission states before and after. The prompt itself is not visible in headless Chromium: only the requests are measured.
import { sleep, waitStable } from '../../lib/flow.mjs';
import { clipboardReads, permissionStates } from '../../lib/detect.mjs';

const COUNT_FROM_LOAD = `(() => {
  window.__e2eClip = { read: 0, readText: 0 };
  const c = navigator.clipboard;
  if (!c) return false;
  for (const k of ['read', 'readText']) {
    if (typeof c[k] !== 'function') continue;
    const f = c[k].bind(c);
    c[k] = (...a) => { window.__e2eClip[k]++; return f(...a); };
  }
  return true;
})()`;

const RECORD = `(() => { const c = window.__app ? window.__app.clipboard : null; return c ? { readPermission: c.readPermission, reads: c.reads, refusal: c.refusal } : null; })()`;

async function setPrompt(browserCdp, browserContextId, origin) {
  for (const name of ['clipboard-read', 'clipboard-write']) {
    await browserCdp.send('Browser.setPermission', { permission: { name }, setting: 'prompt', origin, browserContextId });
  }
}

export default {
  name: 'clipboard-load-permission',
  expectFail: null,
  async beforeLoad({ page, browserCdp, browserContextId, origin }) {
    await setPrompt(browserCdp, browserContextId, origin);
    await page.addInitScript(COUNT_FROM_LOAD);
  },
  async run({ guest, page, log }) {
    const notes = [];
    const permissionBefore = await permissionStates(page);
    const atLoad = await clipboardReads(page);
    log(`at the load: readText ${atLoad.readText}, read ${atLoad.read}; permission ${JSON.stringify(permissionBefore)}`);
    const first = await waitStable(guest, 180000);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes: [`first state ${first.state}`] };
    const beforeClick = await clipboardReads(page);
    const d = await guest.desk();
    await guest.clickCss(d.x + d.w / 2, d.y + d.h / 2);
    await sleep(800);
    const afterClick = await clipboardReads(page);
    const permissionAfter = await permissionStates(page);
    const record = await page.evaluate(RECORD);
    const results = [{ atLoad, beforeClick, afterClick, permissionBefore, permissionAfter, record }];
    notes.push(`readText requests: at the load ${atLoad.readText}, before the first click ${beforeClick.readText}, after it ${afterClick.readText}`);
    notes.push(`permission before ${JSON.stringify(permissionBefore)}, after the first click ${JSON.stringify(permissionAfter)}; the page's record ${JSON.stringify(record)}`);
    const ok = afterClick.readText >= 1;
    return { ok, engineState: first.state, notes, results };
  },
};
