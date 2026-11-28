// perf/cpu-playing-busy: the sound is on (the output started, Space, the metronome) and the engine's busyPct over 20 s; judged against the same run's quiet reference (at most 1.5 times; the reference is the same run, the same measure with no sound (20 s), not golden/perf/*.tsv (Issue #1509)).
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, sameRunBase, busyPctOver, startSound, stopSound } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/cpu-playing-busy',
  covers: ['perf/cpu/playing-busy'],
  displays: ['4k-dpr1'],
  expectFail: 'new', // run of 2026-11-28: busy 0.933 % with the sound on against 0.285 % with no sound in the same run (ratio 3.27 > 1.5); a candidate for PM triage, not confirmed
  async run(ctx) {
    const dpr = await ctx.page.evaluate(() => window.devicePixelRatio);
    const quiet = await busyPctOver(ctx.page, 20000);
    const j = new Judge(sameRunBase('cpu.playing.busyPct', quiet.pct));
    const s = await startSound(ctx.guest, dpr);
    if (!s.playing) return { ok: false, engineState: (await ctx.guest.state())?.state ?? null, notes: [`no sound: ${s.why}`], results: j.results() };
    const b = await busyPctOver(ctx.page, 20000);
    await stopSound(ctx.guest, dpr);
    const ok = b.pct !== null && j.ratio('cpu.playing.busyPct', b.pct, { kind: 'upper', limit: LIMIT.busy, unit: '%' });
    j.notes.push(`reference (no sound): ${quiet.idles} idle reports in ${quiet.seconds} s, busy ${quiet.pct === null ? 'none' : Math.round(quiet.pct * 1000) / 1000} %`);
    j.notes.push(`sound on (nonSilentQuanta from ${s.start}); ${b.idles} idle reports in ${b.seconds} s`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
