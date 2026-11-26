// resilience/window-open-all-close: F5 to F12 pressed once each (the windows they open stay), then Escape eight times. Expected: the table is readable after every press, no stop,
// the engine idle and the main form in place at the end.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { allWindows, mainFormHere, press, readable, stateName } from '../../lib/resilience.mjs';
import { sleep } from '../../lib/flow.mjs';
import { windowTable } from '../../lib/common.mjs';

const KEYS = ['F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12'];

export default defineScenario({
  id: 'resilience/window-open-all-close',
  covers: ['resilience/window/open-all-close'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    await guest.focusDesktop();
    const reads = [];
    for (const k of KEYS) {
      await press(guest, k);
      await sleep(500);
      reads.push(await readable(guest));
    }
    const opened = allWindows(await windowTable(guest)).length;
    for (let j = 0; j < 8; j++) { await press(guest, 'Escape'); await sleep(300); reads.push(await readable(guest)); }
    const left = allWindows(await windowTable(guest)).length;
    const state = await stateName(guest);
    const main = await mainFormHere(guest);
    const okReads = reads.filter(Boolean).length;
    return {
      ok: okReads === reads.length && state === 'engine-idle' && main, engineState: state,
      notes: [`F5 to F12 once each: ${opened} windows in the table; after 8 Escape: ${left}; table readable ${okReads} of ${reads.length}; main form ${main}`],
      results: { windowsOpened: opened, windowsLeft: left, readsOk: okReads, readsFailed: reads.length - okReads },
    };
  },
});
