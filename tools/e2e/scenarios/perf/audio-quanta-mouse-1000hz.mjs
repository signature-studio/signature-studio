// perf/audio-quanta-mouse-1000hz: with the pointer at 1000 Hz for 20 s and the sound on, the output takes its quanta (the worklet's count while playing rises).
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, soundRun } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/audio-quanta-mouse-1000hz',
  covers: ['perf/audio/quanta-mouse-1000hz'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(new Map());
    const r = await soundRun(ctx, { ms: 20000, hz: 1000 });
    const ok = r.playing && j.flag('audio.quanta.mouse1000', r.quanta, r.quanta !== null && r.quanta > 0);
    j.notes.push(`${r.playing ? `quanta while playing +${r.quanta}; underruns +${r.underrun}` : `no sound: ${r.s.why}`}; status ${r.status ?? 'none'}, error ${r.error ?? 'none'}`);
    return { ok: ok === true, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
