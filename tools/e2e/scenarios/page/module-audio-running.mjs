// page/module-audio-running: after a click (the first gesture) the audio output runs: its AudioContext is running and its worklet is ready (the page's audio detail).
import { defineScenario } from '../../lib/scenario.mjs';
import { deskInfo } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/module-audio-running',
  covers: ['page/module/audio-start'],
  async run({ guest, page }) {
    const d = await deskInfo(page);
    await guest.click(Math.round(d.guest.w * 0.9), Math.round(d.guest.h * 0.9));
    await sleep(1500);
    const a = await page.evaluate(() => (window.__app.audio ? window.__app.audio.detail() : null));
    const ok = a !== null && a.ctxState === 'running' && a.nodeReady === true;
    return { ok, engineState: d.state, notes: [`audio: context ${a?.ctxState ?? 'none'}, worklet ready ${a?.nodeReady ?? 'no'}, status ${a?.status ?? '-'}`], results: a ? { ctxState: a.ctxState, nodeReady: a.nodeReady, status: a.status } : null };
  },
});
