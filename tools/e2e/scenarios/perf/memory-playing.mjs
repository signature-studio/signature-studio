// perf/memory-playing: the engine's wasm memory before and after 20 s of sound (the output started, Space, the metronome): the growth is at most 1.3 times.
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, memoryNow, sleep, startSound, stopSound } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/memory-playing',
  covers: ['perf/memory/playing'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(new Map());
    const dpr = await ctx.page.evaluate(() => window.devicePixelRatio);
    const before = await memoryNow(ctx.guest, ctx.cdp);
    const s = await startSound(ctx.guest, dpr);
    if (!s.playing) return { ok: false, engineState: (await ctx.guest.state())?.state ?? null, notes: [`no sound: ${s.why}`], results: j.results() };
    await sleep(20000);
    const after = await memoryNow(ctx.guest, ctx.cdp);
    await stopSound(ctx.guest, dpr);
    const wasmOk = before.wasm !== null && after.wasm !== null && j.growth('memory.playing.wasm', before.wasm, after.wasm, LIMIT.memory);
    const jsOk = before.js !== null && after.js !== null && j.growth('memory.playing.js', before.js, after.js, LIMIT.memory);
    const ok = wasmOk && jsOk;
    j.notes.push('20 s of sound');
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
