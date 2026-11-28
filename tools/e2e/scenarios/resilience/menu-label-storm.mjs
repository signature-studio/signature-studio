// resilience/menu-label-storm: the eight labels of the menu bar are clicked in turn, 30 rounds, each drop-down closed with Escape. Expected: the table is readable after every step,
// no stop, the engine idle and the main form in place at the end.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { MENU_LABELS, openMenu } from '../../lib/common.mjs';
import { mainFormHere, press, readable, stateName } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/menu-label-storm',
  covers: ['resilience/menu/label-storm'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const reads = [];
    for (let round = 0; round < 30; round++) {
      for (const [label] of MENU_LABELS) {
        await openMenu(guest, label);
        reads.push(await readable(guest));
        await press(guest, 'Escape');
        reads.push(await readable(guest));
      }
    }
    const state = await stateName(guest);
    const main = await mainFormHere(guest);
    const okReads = reads.filter(Boolean).length;
    return {
      ok: okReads === reads.length && state === 'engine-idle' && main, engineState: state,
      notes: [`8 labels x 30 rounds: table readable ${okReads} of ${reads.length}; main form ${main}; state ${state}`],
      results: { clicks: 240, readsOk: okReads, readsFailed: reads.length - okReads },
    };
  },
});
