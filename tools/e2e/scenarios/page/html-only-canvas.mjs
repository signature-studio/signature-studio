// page/html-only-canvas: the page shows the engine's canvas and nothing else: no visible element but the canvas, its holders, the standing elements (#status, #audio-status, #taskbar) and the WebView2 faces of FL's windows.
import { defineScenario } from '../../lib/scenario.mjs';
import { VISIBLE_CHECK } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/html-only-canvas',
  covers: ['page/html/only-canvas'],
  async run({ guest, page }) {
    const v = await page.evaluate(VISIBLE_CHECK);
    const state = (await guest.state())?.state ?? null;
    return { ok: v.others.length === 0, engineState: state, notes: [`visible elements besides the canvas: ${v.others.length}${v.others.length ? ` (${v.others.slice(0, 10).join(', ')})` : ''}; standing: ${v.allowed.join(', ') || 'none shown'}; WebView2 faces: ${v.faces}`], results: { others: v.others, allowed: v.allowed, faces: v.faces } };
  },
});
