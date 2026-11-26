// perf/engine-idle-after-menu: after the File drop-down is opened and closed, the engine is idle again (its state engine-idle; not running).
import { defineScenario } from '../../lib/scenario.mjs';
import { Judge, openAndClose, sleep } from '../../lib/perf.mjs';

export default defineScenario({
  id: 'perf/engine-idle-after-menu',
  covers: ['perf/engine/idle-after-menu'],
  displays: ['4k-dpr1'],
  async run(ctx) {
    const j = new Judge(new Map());
    await openAndClose(ctx.guest, 'File');
    let st = null;
    for (let i = 0; i < 25; i++) {
      st = await ctx.guest.state();
      if (st?.state === 'engine-idle') break;
      await sleep(200);
    }
    const ok = j.flag('engine.idleAfterMenu', st?.state === 'engine-idle' ? 1 : 0, st?.state === 'engine-idle');
    j.notes.push(`state ${st?.state ?? 'none'} after the close`);
    return { ok, engineState: st?.state ?? null, notes: j.notes, results: j.results() };
  },
});
