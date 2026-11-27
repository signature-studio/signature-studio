// resilience/wait-idle-30min: nothing is sent for 30 minutes (no action; the time is the difference of the clock, checked every 5 s; the limit is 35 minutes).
// Expected: no stop; the engine's memory at 30 min is at most 1.10 times the value at 10 min (the engine's own report, every minute); while the sound was playing during the wait,
// the output's underrun count did not rise (when it did not play, no underrun is counted, and the note says so).
// It does not start with less than 30 min left in its display row's budget (needsMs: the runner skips it, skipped: true); results.budgetLeftMs is the row's budget left at the start.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { sleep } from '../../lib/flow.mjs';
import { audioNumbers, memoryBytes, MEM_RATIO, stateName } from '../../lib/resilience.mjs';

const SPAN = 30 * 60 * 1000;
const LIMIT = 35 * 60 * 1000;
const MEM_EVERY = 60 * 1000;
const CHECK_EVERY = 5000;

export default defineScenario({
  id: 'resilience/wait-idle-30min',
  covers: ['resilience/soak/idle-30min-no-stop', 'resilience/soak/idle-30min-no-underrun', 'resilience/soak/idle-30min-memory-flat'],
  displays: ['4k-dpr1'],
  needsMs: SPAN,
  async run({ guest, page, log, budget }) {
    await startIfStopped(guest, page, log);
    const budgetLeftMs = Math.round(budget.remainingMs());
    const t0 = Date.now();
    const a0 = await audioNumbers(guest);
    const mem = [];
    const seen = {};
    let stops = 0, lastMem = -Infinity;
    while (Date.now() - t0 < SPAN && Date.now() - t0 < LIMIT) {
      const s = await stateName(guest);
      seen[s] = (seen[s] ?? 0) + 1;
      if (s === 'error' || s === 'exited') { stops++; break; }
      const t = Date.now() - t0;
      if (t - lastMem >= MEM_EVERY) { mem.push({ t: Math.round(t / 1000), bytes: await memoryBytes(guest) }); lastMem = t; }
      await sleep(CHECK_EVERY);
    }
    const elapsed = Date.now() - t0;
    mem.push({ t: Math.round(elapsed / 1000), bytes: await memoryBytes(guest) });
    const a1 = await audioNumbers(guest);
    const at10 = mem.find((m) => m.t >= 600 && m.bytes !== null) ?? null;
    const last = mem[mem.length - 1];
    const ratio = at10 && last.bytes !== null ? Math.round((last.bytes / at10.bytes) * 1000) / 1000 : null;
    const played = a0 && a1 ? a1.quantaWhilePlaying - a0.quantaWhilePlaying : null;
    const playing = played !== null && played > 0;
    const under = a0 && a1 ? a1.underrunQuanta - a0.underrunQuanta : null;
    const memOk = ratio !== null && ratio <= MEM_RATIO;
    const underOk = !playing || under === 0;
    const state = await stateName(guest);
    const audioNote = a0 === null ? 'no audio output on the page' : playing ? `playing during the wait (quanta +${played}): underruns +${under}` : `not playing during the wait (quanta +${played ?? 0}): underruns not counted`;
    return {
      ok: stops === 0 && elapsed >= SPAN && memOk && underOk && state === 'engine-idle', engineState: state,
      notes: [`${Math.round(elapsed / 60000)} min with nothing sent: stops ${stops}; memory at 10 min ${at10?.bytes ?? 'unknown'}, at the end ${last.bytes ?? 'unknown'}, ratio ${ratio} (limit ${MEM_RATIO}); ${audioNote}`],
      results: {
        minutes: Math.round((elapsed / 60000) * 100) / 100, stops, states: seen, memory: mem, memoryRatio: ratio, memoryLimit: MEM_RATIO, budgetLeftMs,
        audio: { playing, quantaWhilePlaying: played, underruns: under, counted: playing },
      },
    };
  },
});
