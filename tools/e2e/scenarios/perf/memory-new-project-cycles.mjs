// perf/memory-new-project-cycles: the engine's wasm memory before and after five File > New (Ctrl+N): the growth is at most 1.3 times.
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, memoryNow } from '../../lib/perf.mjs';
import { resetProject } from '../../lib/common.mjs';

export default defineScenario({
  id: 'perf/memory-new-project-cycles',
  covers: ['perf/memory/new-project-cycles'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(new Map());
    const before = await memoryNow(ctx.guest, ctx.cdp);
    for (let i = 0; i < 5; i++) await resetProject(ctx.guest);
    const after = await memoryNow(ctx.guest, ctx.cdp);
    const wasmOk = before.wasm !== null && after.wasm !== null && j.growth('memory.newProjectCycles.wasm', before.wasm, after.wasm, LIMIT.memory);
    const jsOk = before.js !== null && after.js !== null && j.growth('memory.newProjectCycles.js', before.js, after.js, LIMIT.memory);
    const ok = wasmOk && jsOk;
    j.notes.push('5 File > New');
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
