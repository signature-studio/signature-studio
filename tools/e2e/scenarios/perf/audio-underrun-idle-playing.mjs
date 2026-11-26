// perf/audio-underrun-idle-playing: the sound is on and nothing is done for 20 s: the output's underruns over the 20 s are 0.
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, soundRun } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/audio-underrun-idle-playing',
  covers: ['perf/audio/underrun-idle'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(new Map());
    const r = await soundRun(ctx, { ms: 20000, hz: 0 });
    const ok = r.playing && j.flag('audio.underrun.idle', r.underrun, r.underrun === 0);
    j.notes.push(`${r.playing ? `sound on (nonSilent +${r.nonSilent}), no input for 20 s` : `no sound: ${r.s.why}`}; status ${r.status ?? 'none'}, error ${r.error ?? 'none'}`);
    return { ok: ok === true, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
