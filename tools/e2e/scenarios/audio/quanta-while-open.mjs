// audio/quanta-while-open: once the output is open the page keeps taking its quanta (the worklet's counter rises over 2 s). Mark new: the worklet loads (nodeReady true, status running),
// but the counter does not rise (quanta +0 over 2 s in this runner). Whether the cause is this runner's audio output or the product is not yet known; the mark is removed when the cause is found.
import { defineScenario } from '../../lib/scenario.mjs';
import { audioProbe } from '../../lib/common.mjs';

export default defineScenario({
  id: 'audio/quanta-while-open',
  expectFail: 'new',
  covers: ['page/module/audio-start'],
  async run({ guest }) {
    const p = await audioProbe(guest, 2000);
    if (!p.available) return { ok: false, notes: ['the page has no audio output'] };
    const q = p.after.quantaWhileOpen - p.before.quantaWhileOpen;
    return { ok: q > 0, engineState: (await guest.state())?.state ?? null, notes: [`quanta while open +${q}, status ${p.after.status}, device ${p.after.deviceState}, nodeReady ${p.after.nodeReady}, error ${p.after.error ?? 'none'}`] };
  },
});
