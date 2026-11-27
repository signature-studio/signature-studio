// file/open-shows-opfs-window: File > Open (File > #3) shows the page's file manager window (the OPFS window, its Upload input #fx-input-files inside #filedlg), and the browser's
// own file chooser never opens: the page's 'filechooser' event (Playwright, from Page.fileChooserOpened) stays at 0. The window is closed with its Cancel and the engine is idle again.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuPath } from '../../lib/common.mjs';
import { clearMissingPlugins, clickElement } from '../../lib/file.mjs';
import { sleep } from '../../lib/flow.mjs';

const WINDOW = "(() => { const i = document.getElementById('fx-input-files'); if (!i) return null; const w = i.closest('.fx-window'); return { cls: w ? w.className : null, parent: w && w.parentElement ? w.parentElement.id : null, title: w ? (w.querySelector('.fx-title strong') ? w.querySelector('.fx-title strong').textContent : null) : null }; })()";

async function untilGone(guest, ms) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    if ((await guest.ev(WINDOW)) === null) return true;
    await sleep(300);
  }
  return false;
}

export default defineScenario({
  id: 'file/open-shows-opfs-window',
  covers: ['file/open/opfs-window'],
  displays: ['4k-dpr1', '4k-dpr2'],
  async run(ctx) {
    const { guest, cdp, page } = ctx;
    await clearMissingPlugins(guest);
    let choosers = 0;
    const onChooser = () => { choosers++; };
    page.on('filechooser', onChooser);
    await cdp.send('Page.setInterceptFileChooserDialog', { enabled: true });
    try {
      await guest.focusDesktop();
      const id = await menuPath(guest, ['File', '#3']);
      let win = null;
      const t0 = Date.now();
      while (!win && Date.now() - t0 < 20000) {
        win = await guest.ev(WINDOW);
        if (!win) await sleep(300);
      }
      await sleep(1500);
      const seen = win ?? (await guest.ev(WINDOW));
      let closed = false;
      if (seen) {
        await clickElement(guest, '#fx-close');
        closed = await untilGone(guest, 10000);
      }
      const st = (await guest.state())?.state ?? null;
      const req = (await guest.ev("(() => { const r = window.__app.fileDlg.requests; const l = r[r.length - 1]; return l ? { n: r.length, save: l.save, options: l.request.options, activationActive: l.activationActive, msSinceInput: l.msSinceInput } : { n: 0 }; })()")) ?? { n: 0 };
      return {
        ok: seen !== null && choosers === 0 && closed && st === 'engine-idle',
        engineState: st,
        notes: [`${id}: window ${seen ? `class ${seen.cls} in #${seen.parent}, title "${seen.title}"` : 'none (no #fx-input-files in the page)'}; filechooser ${choosers}; closed with Cancel ${closed}; state ${st}`, `engine requests ${req.n}, last: options ${req.options === undefined ? '?' : `0x${req.options.toString(16)}`} (multi-select ${req.options === undefined ? '?' : (req.options & 0x200) !== 0}), activation ${req.activationActive}, ms since input ${req.msSinceInput}`],
      };
    } finally {
      page.off('filechooser', onChooser);
    }
  },
});
