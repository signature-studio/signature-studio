// resilience/fkeys-toggle-all: each of F5 to F12 is pressed twice (a window opens and closes, or the keys do what the engine says), the window table is read after each press,
// then Escape four times. Expected: the table is readable after every press, no stop, the engine idle and the main form in place at the end.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { mainFormHere, press, readable, stateName } from '../../lib/resilience.mjs';
import { sleep } from '../../lib/flow.mjs';

const KEYS = ['F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12'];

export default defineScenario({
  id: 'resilience/fkeys-toggle-all',
  covers: ['resilience/window/fkey-toggle'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    await guest.focusDesktop();
    const reads = [];
    for (const k of KEYS) {
      for (let j = 0; j < 2; j++) {
        await press(guest, k);
        await sleep(400);
        reads.push(await readable(guest));
      }
    }
    for (let j = 0; j < 4; j++) { await press(guest, 'Escape'); reads.push(await readable(guest)); }
    const state = await stateName(guest);
    const main = await mainFormHere(guest);
    const okReads = reads.filter(Boolean).length;
    return { ok: okReads === reads.length && state === 'engine-idle' && main, engineState: state, notes: [`F5 to F12 twice each, Escape 4 times: table readable ${okReads} of ${reads.length}, main form ${main}, state ${state}`], results: { presses: reads.length, readsOk: okReads, readsFailed: reads.length - okReads } };
  },
});
