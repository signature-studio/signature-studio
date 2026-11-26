// perf/audio-underrun-mouse-1000hz: the sound is on and the pointer moves at 1000 Hz for 20 s: the output's underruns (the worklet's count) over the 20 s are 0 (P3: no cut in the sound).
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, soundRun } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/audio-underrun-mouse-1000hz',
  covers: ['perf/audio/underrun-mouse-1000hz'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(new Map());
    const r = await soundRun(ctx, { ms: 20000, hz: 1000 });
    const ok = r.playing && j.flag('audio.underrun.mouse1000', r.underrun, r.underrun === 0);
    j.notes.push(`${r.playing ? `sound on (nonSilent +${r.nonSilent}); mouse ${r.mouseSent} events in ${r.mouseSeconds} s, received ${r.receivedHz}/s` : `no sound: ${r.s.why}`}; status ${r.status ?? 'none'}, error ${r.error ?? 'none'}`);
    return { ok: ok === true, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
