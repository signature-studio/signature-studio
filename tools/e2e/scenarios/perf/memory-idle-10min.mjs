// perf/memory-idle-10min: the engine's wasm memory after the start (the page is idle) and after 10 minutes with no input: the growth is at most 1.3 times (it does not keep growing). The JS heap is in the notes.
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, memoryNow, sleep } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/memory-idle-10min',
  covers: ['perf/memory/idle-10min'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(new Map());
    const start = await memoryNow(ctx.guest, ctx.cdp);
    const trail = [start.wasm];
    for (let m = 0; m < 10; m++) { await sleep(60000); trail.push((await memoryNow(ctx.guest, ctx.cdp)).wasm); }
    const end = await memoryNow(ctx.guest, ctx.cdp);
    const wasmOk = start.wasm !== null && end.wasm !== null && j.growth('memory.idle10min.wasm', start.wasm, end.wasm, LIMIT.memory);
    const jsOk = start.js !== null && end.js !== null && j.growth('memory.idle10min.js', start.js, end.js, LIMIT.memory);
    const ok = wasmOk && jsOk;
    j.notes.push(`wasm bytes each minute: ${trail.join(', ')}`);
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
