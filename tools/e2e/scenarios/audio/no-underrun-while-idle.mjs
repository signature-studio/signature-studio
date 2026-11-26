// audio/no-underrun-while-idle: with the engine idle, the output takes its quanta without an underrun over 3 s (P3: no cut in the sound). Mark new: the counter of the quanta does not rise
// in this runner (see quanta-while-open), so this check cannot pass yet.
import { defineScenario } from '../../lib/scenario.mjs';
import { audioProbe } from '../../lib/common.mjs';

export default defineScenario({
  id: 'audio/no-underrun-while-idle',
  expectFail: 'new',
  covers: ['page/module/audio-start'],
  async run({ guest }) {
    const p = await audioProbe(guest, 3000);
    if (!p.available) return { ok: false, notes: ['the page has no audio output'] };
    const u = p.after.underrunQuanta - p.before.underrunQuanta;
    const q = p.after.quantaWhileOpen - p.before.quantaWhileOpen;
    return { ok: q > 0 && u === 0, engineState: (await guest.state())?.state ?? null, notes: [`underruns +${u}, quanta +${q}, status ${p.after.status}, nodeReady ${p.after.nodeReady}, error ${p.after.error ?? 'none'}`] };
  },
});
