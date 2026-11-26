// audio/output-starts-on-click: a click on the page's status line (a user gesture) starts the audio context (its state is running). The worklet node is checked by the other audio scenarios.
import { defineScenario } from '../../lib/scenario.mjs';
import { sleep } from '../../lib/flow.mjs';

const AT_STATUS = "(() => { const e = document.getElementById('status') || document.body; const b = e.getBoundingClientRect(); return { x: b.x + b.width / 2, y: b.y + b.height / 2 }; })()";

export default defineScenario({
  id: 'audio/output-starts-on-click',
  covers: ['page/module/audio-start'],
  async run({ guest }) {
    const c = await guest.ev(AT_STATUS);
    await guest.move(c);
    await guest.press(c, 'left', 1);
    const t0 = Date.now();
    let d = null;
    while (Date.now() - t0 < 15000) {
      d = await guest.ev('window.__app.audio ? window.__app.audio.detail() : null');
      if (d && d.ctxState === 'running') break;
      await sleep(300);
    }
    const ok = !!d && d.ctxState === 'running';
    return { ok, engineState: (await guest.state())?.state ?? null, notes: [`status ${d?.status ?? 'none'}, ctxState ${d?.ctxState ?? 'none'}, nodeReady ${d?.nodeReady ?? 'none'}, device ${d?.deviceState ?? 'none'}, error ${d?.error ?? 'none'}, after ${Date.now() - t0} ms`] };
  },
});
