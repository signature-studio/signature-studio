// perf/cpu-after-audio-busy: the sound is played for 3 s and stopped; then the engine's busyPct over 20 s with no input; judged against the same run's quiet reference (at most 1.5 times; the reference is the same run, the same measure with no sound (20 s), not golden/perf/*.tsv (Issue #1509)).
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, sameRunBase, busyPctOver, sleep, startSound, stopSound } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/cpu-after-audio-busy',
  covers: ['perf/cpu/after-audio-busy'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const dpr = await ctx.page.evaluate(() => window.devicePixelRatio);
    const quiet = await busyPctOver(ctx.page, 20000);
    const j = new Judge(sameRunBase('cpu.afterAudio.busyPct', quiet.pct));
    const s = await startSound(ctx.guest, dpr);
    if (!s.playing) return { ok: false, engineState: (await ctx.guest.state())?.state ?? null, notes: [`no sound: ${s.why}`], results: j.results() };
    await sleep(3000);
    await stopSound(ctx.guest, dpr);
    const b = await busyPctOver(ctx.page, 20000);
    const ok = b.pct !== null && j.ratio('cpu.afterAudio.busyPct', b.pct, { kind: 'upper', limit: LIMIT.busy, unit: '%' });
    j.notes.push(`sound played 3 s and stopped; ${b.idles} idle reports in ${b.seconds} s`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
