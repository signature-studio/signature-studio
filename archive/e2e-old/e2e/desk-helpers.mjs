// Decisions 0223 / desk2: what the real-engine scripts may know about the engine's windows. The page holds ONE canvas and no window list; what there is to read is
//   - the compositor's text about the windows it drew (window.__app.deskDumpBase64(true).windows): one line per window - lid, position, size, layer state;
//   - the title of the engine's foreground window (document.title), which is the title of the engine's main window when no other window of the engine is in front;
//   - the picture itself (window.__app.deskDumpBase64).
// A window is told apart by its class name when the compositor's text has one (a bundle built with desk2's fle_desk.c); a bundle built before (D8) has lines without class and title, and then by where it is, how big it is and
// what the foreground title says (classify() below: the weaker way, and it says so in each window it returns: `named: false`).
import { sleep } from './input-helpers.mjs';

/** "lid 14 at (-5, -6) 1930x1092 ... class \"TFruityLoopsMainForm\" title \"FL Studio 2026\"" lines -> [{ lid, x, y, w, h, rest, cls?, title? }]. cls and title are there when the engine was built with desk2's compositor text
 *  (a bundle built before has lines without them: cls and title are then undefined and a script falls back to what it can tell from the geometry and the foreground title) */
export function parseDeskWindows(text) {
  return (text ?? '').split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean)
    .map((m) => {
      const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
      return { lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), rest: m[6], ...(names ? { cls: names[1], title: names[2] } : {}) };
    });
}
/** the windows the compositor drew at its last compose (read from the compositor through window.__app.deskDumpBase64) */
export async function deskWindows(page) {
  const t = await page.ev(`(async () => (await window.__app.deskDumpBase64(true)).windows)()`);
  if (typeof t !== 'string') throw new Error('window.__app.deskDumpBase64 gave no window text');
  return parseDeskWindows(t);
}
/** the engine's main window: the largest of the windows drawn */
export const mainOf = (ws) => ws.slice().sort((a, b) => b.w * b.h - a.w * a.h)[0] ?? null;
/** the welcome wizard: 874 wide (570 to 590 high), the only window of that width in these runs */
export const isWizard = (w) => w.w === 874 && w.h >= 560 && w.h <= 590;
/** the windows of the engine other than its main window */
export const othersOf = (ws) => { const m = mainOf(ws); return ws.filter((w) => w !== m); };
/** the title of the engine's foreground window (the page's document.title once the engine has reported one) */
export const foregroundTitle = (page) => page.ev('document.title');

/** the classes of the engine's windows the real-engine scripts talk about */
export const CLS = { main: 'TFruityLoopsMainForm', wizard: 'TWelcomeWizard', popup: 'TQuickPopupMenuWindow', problems: 'TMissingPluginsForm', render: 'TWAVRenderForm', msg: 'TMsgForm', shop: 'TInAppShopForm' };
const KIND_OF_CLASS = Object.fromEntries(Object.entries(CLS).map(([k, c]) => [c, k]));
/**
 * Gives each window a `kind` (main, wizard, popup, problems, render, msg, shop, other) and a `title`.
 *  - with class and title in the text (w.cls defined): the kind is the window's class, the title its own; `named: true`.
 *  - without (a bundle built before desk2, D8; the weaker way, `named: false`): main = the largest, wizard = 874 x 570..590; any other window is a `dialog` when it is at least 300 x 100 (the engine's menus and hints are narrower: 206 and
 *    208 wide in these runs) or when it is the window drawn last and the foreground title says "Buy FL Studio" (that window is 206 x 104); the rest are `popup`s. The class of a dialog is not known (render, problems, msg and shop cannot
 *    be told apart: see isKind). The foreground title `fg` belongs to the topmost dialog (a stale popup drawn after it does not take it); no dialog: no window has a title.
 */
export function classify(ws, fg) {
  const main = mainOf(ws);
  const last = ws.length - 1;
  const dialog = (w, i) => w !== main && !isWizard(w) && ((w.w >= 300 && w.h >= 100) || (i === last && /^Buy FL Studio/i.test(fg ?? '')));
  const top = ws.map((w, i) => (dialog(w, i) ? i : -1)).reduce((a, b) => Math.max(a, b), -1);
  return ws.map((w, i) => {
    if (w.cls !== undefined) return { ...w, title: w.title, kind: KIND_OF_CLASS[w.cls] ?? 'other', named: true };
    const kind = w === main ? 'main' : isWizard(w) ? 'wizard' : dialog(w, i) ? 'dialog' : 'popup';
    return { ...w, title: i === top ? (fg ?? '') : '', kind, named: false };
  });
}
/** a window of kind `kind` (when its class is not known, any dialog stands for render, problems, msg and shop) */
export const isKind = (w, kind) => w.kind === kind || (!w.named && w.kind === 'dialog' && ['render', 'problems', 'msg', 'shop'].includes(kind));
/** the windows of the engine other than its main window and the wizard that are not a popup menu */
export const dialogsOf = (cws) => cws.filter((w) => w.kind !== 'main' && w.kind !== 'wizard' && w.kind !== 'popup');
/** the classified windows now */
export async function classifiedWindows(page) { return classify(await deskWindows(page), await foregroundTitle(page)); }

/** waits for the first idle and then until the compositor draws the main window and the welcome wizard */
export async function waitIdleWithWizard(page, ms = 400000) {
  await page.waitFor("window.__app && (window.__app.state === 'engine-idle' || window.__app.state === 'error' || window.__app.state === 'exited')", ms, 'engine idle');
  const t0 = Date.now();
  for (;;) {
    const ws = await deskWindows(page);
    if (ws.length >= 2 && ws.some(isWizard)) return ws;
    if (Date.now() - t0 > 60000) throw new Error(`the compositor never drew the main window and the welcome wizard: ${JSON.stringify(ws)}`);
    await sleep(100);
  }
}
