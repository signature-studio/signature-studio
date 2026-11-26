// The steps a person takes at the start: the usage-data box ("No thanks"), the Welcome window and its Default row, the "Problems loading the project" box,
// and the engine's open box (Ctrl+O). The positions are guest pixels from the window table (the Welcome window, the Problems box) and the offsets
// measured by tools/smoke/welcome-flow.mjs and tools/clipboard/measure.mjs. A window's offsets do not depend on the guest screen's size.
import fs from 'node:fs';
import path from 'node:path';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
export const WELCOME = 'TWelcomeWizard';
const NO_THANKS = { dx: 512, dy: 307 };
const LIST = { dx: 235, dy: 180 };
const DEFAULT_ROW = { dx: 235, dy: 117 };
const PROBLEMS = /Problems loading/;
const PROBLEMS_OK = { dx: 368, dyFromBottom: 29 };

/** waits until the engine is idle, or has stopped (error, exited), or the limit; returns the last state with `waitedMs` */
export async function waitStable(guest, ms) {
  const t0 = Date.now();
  let s = null;
  while (Date.now() - t0 < ms) {
    s = await guest.state().catch(() => null);
    if (s && (s.state === 'engine-idle' || s.state === 'error' || s.state === 'exited')) break;
    await sleep(300);
  }
  return { ...(s ?? { state: 'unreadable' }), waitedMs: Date.now() - t0 };
}

/** the usage-data box: "No thanks". The box is centred on the #desktop box, and its "No thanks" is 75 and 20 CSS px from the centre (measured at 1836 x 1056
 *  and at 1580 x 1044 CSS px: the same offset), so it is placed in CSS px, not in guest pixels */
const NO_THANKS_CSS = { dx: 75, dy: 20 };
export async function noThanks(guest) {
  const d = await guest.desk();
  await guest.clickCss(d.x + d.w / 2 + NO_THANKS_CSS.dx, d.y + d.h / 2 + NO_THANKS_CSS.dy);
  await sleep(2500);
  return true;
}

/** the Welcome window is closed with Escape: the default project opens (tools/smoke/welcome-flow.mjs, flow B). The start list's rows are not read by
 *  name here (the pixels are the only text), so the Default project is taken this way and not by a row's place */
export async function closeWelcome(guest) {
  await guest.focusDesktop();
  await guest.escape();
}

/** the title and the Welcome window: the project is open when the title has changed and the Welcome window is gone, and the engine is idle */
export async function waitProjectOpen(guest, titleBefore, ms) {
  const t0 = Date.now();
  let last = null;
  while (Date.now() - t0 < ms) {
    last = await guest.state().catch(() => null);
    if (last && (last.state === 'error' || last.state === 'exited')) break;
    if (last && last.state === 'engine-idle' && last.title !== titleBefore && !(await guest.findWindow((x) => x.cls === WELCOME))) {
      await sleep(1500);
      return { ...(await guest.state()), opened: true, waitedMs: Date.now() - t0 };
    }
    await sleep(300);
  }
  return { ...(last ?? { state: 'unreadable' }), opened: false, waitedMs: Date.now() - t0 };
}

/** the "Problems loading the project" box (it comes with the default project): its Ok is clicked; returns true when it was there.
 *  The offsets are CSS px (measured at DPR 1); a window's size in guest px grows with the DPR, so they are scaled by guest px per CSS px */
export async function dismissProblems(guest) {
  const box = await guest.findWindow((x) => PROBLEMS.test(x.title));
  if (!box) return false;
  const k = await guest.guestPerCss();
  await guest.click(box.x + PROBLEMS_OK.dx * k, box.y + box.h - PROBLEMS_OK.dyFromBottom * k);
  await guest.waitFor(async () => ((await guest.findWindow((x) => PROBLEMS.test(x.title))) ? null : true), 10000, 'the Problems box closes');
  await sleep(500);
  return true;
}

export const MISSING_PLUGINS = 'TMissingPluginsForm';

/**
 * the "Problems loading the project" box (TMissingPluginsForm) that a project with missing plugins brings: Enter answers it (its default button), and Escape when it stays.
 * It is looked for until it has not been seen for `settleMs` (a box can come after the project's load); `limitMs` bounds the whole. Returns the number of answers.
 */
export async function clearProblems(guest, { settleMs = 0, limitMs = 20000 } = {}) {
  const isBox = async () => (await guest.windows().catch(() => [])).some((x) => x.cls === MISSING_PLUGINS);
  const t0 = Date.now();
  let quietSince = Date.now();
  let n = 0;
  while (Date.now() - t0 < limitMs) {
    if (await isBox()) {
      n++;
      await guest.key('Enter', 'Enter', 13);
      await sleep(1000);
      if (await isBox()) {
        await guest.escape();
        await sleep(800);
      }
      quietSince = Date.now();
    } else {
      if (Date.now() - quietSince >= settleMs) break;
      await sleep(500);
    }
  }
  return n;
}

/** Ctrl+O opens the engine's Open box: the page's file box (#filedlg-input) or the file manager in selection mode (#fx-input-files) */
export async function openProjectFile(guest, cdp, projectPath, log) {
  if (!fs.existsSync(projectPath)) throw new Error(`no project file ${projectPath}`);
  // Issue #1481: the page's file chooser (its input is hidden) is taken by the tool, which gives the file itself; without this the headless browser cancels the chooser at once
  await cdp.send('Page.setInterceptFileChooserDialog', { enabled: true });
  await guest.focusDesktop();
  await guest.ctrl('KeyO', 'o', 79);
  let box = null;
  const t0 = Date.now();
  while (!box && Date.now() - t0 < 40000) {
    box = await guest.ev("document.getElementById('filedlg-input') ? 'filedlg' : (document.getElementById('fx-input-files') ? 'explorer' : null)");
    if (!box) await sleep(500);
  }
  if (!box) {
    // what the page knows of the engine's Open request (the count, the answers, the page's own lines about it): the names of files are not in these
    const diag = await guest.ev("(() => { const a = window.__app; const lg = document.getElementById('log'); return { requests: a.fileDlg.requests.length, history: a.fileDlg.history, open: a.fileDlg.open, lines: (lg ? lg.textContent : '').split('\\n').filter((l) => /dialog|chooser|Open|Save/.test(l)).slice(-8) }; })()").catch((e) => String(e?.message ?? e));
    throw new Error(`no open box (#filedlg-input or #fx-input-files) 40 s after Ctrl+O: ${JSON.stringify(diag)}`);
  }
  const setFile = async (selector) => {
    const doc = await cdp.send('DOM.getDocument', {});
    const q = await cdp.send('DOM.querySelector', { nodeId: doc.root.nodeId, selector });
    if (!q.nodeId) throw new Error(`${selector} is not a node`);
    await cdp.send('DOM.setFileInputFiles', { nodeId: q.nodeId, files: [projectPath] });
  };
  const clickSelector = async (selector) => {
    const r = await guest.ev(`(() => { const e = document.querySelector(${JSON.stringify(selector)}); if (!e) return null; const b = e.getBoundingClientRect(); return { x: b.x + b.width / 2, y: b.y + b.height / 2 }; })()`);
    if (!r) throw new Error(`no element ${selector}`);
    await guest.move(r);
    await guest.press(r, 'left', 1);
  };
  const name = path.basename(projectPath);
  if (box === 'filedlg') {
    await setFile('#filedlg-input');
    log(`${name} given to #filedlg-input`);
  } else {
    await setFile('#fx-input-files');
    const row = `#fx-list [data-name="${name}"]`;
    const t1 = Date.now();
    while (!(await guest.ev(`!!document.querySelector(${JSON.stringify(row)})`))) {
      if (Date.now() - t1 >= 30000) throw new Error(`${name} is not listed in the file manager 30 s after the upload`);
      await sleep(500);
    }
    await clickSelector(row);
    await clickSelector('#fx-ok');
    log(`${name} uploaded into the file manager, chosen and Open pressed`);
  }
  return box;
}
