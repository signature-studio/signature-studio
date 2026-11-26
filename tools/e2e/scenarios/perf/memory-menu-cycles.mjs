// perf/memory-menu-cycles: the engine's wasm memory before and after 10 File drop-down open/close cycles: the growth is at most 1.3 times.
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, LIMIT, memoryNow, openAndClose } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/memory-menu-cycles',
  covers: ['perf/memory/menu-cycles'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(new Map());
    const before = await memoryNow(ctx.guest, ctx.cdp);
    for (let i = 0; i < 10; i++) await openAndClose(ctx.guest, 'File');
    const after = await memoryNow(ctx.guest, ctx.cdp);
    const wasmOk = before.wasm !== null && after.wasm !== null && j.growth('memory.menuCycles.wasm', before.wasm, after.wasm, LIMIT.memory);
    const jsOk = before.js !== null && after.js !== null && j.growth('memory.menuCycles.js', before.js, after.js, LIMIT.memory);
    const ok = wasmOk && jsOk;
    j.notes.push('10 File open/close cycles');
    return { ok, engineState: (await ctx.guest.state())?.state ?? null, notes: j.notes, results: j.results() };
  },
});
