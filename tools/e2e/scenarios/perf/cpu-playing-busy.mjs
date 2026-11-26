// perf/cpu-playing-busy: the sound is on (the output started, Space, the metronome) and the engine's busyPct over 20 s; judged against the baseline (at most 1.5 times).
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, baseOf, busyPctOver, startSound, stopSound } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/cpu-playing-busy',
  covers: ['perf/cpu/playing-busy'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(baseOf('base'));
    const dpr = await ctx.page.evaluate(() => window.devicePixelRatio);
    const s = await startSound(ctx.guest, dpr);
    if (!s.playing) return { ok: false, engineState: (await ctx.guest.state())?.state ?? null, notes: [`no sound: ${s.why}`], results: j.results() };
    const b = await busyPctOver(ctx.page, 20000);
    await stopSound(ctx.guest, dpr);
    const ok = b.pct !== null && j.ratio('cpu.playing.busyPct', b.pct, { kind: 'upper', limit: LIMIT.busy, unit: '%' });
    j.notes.push(`sound on (nonSilentQuanta from ${s.start}); ${b.idles} idle reports in ${b.seconds} s`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
