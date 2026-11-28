// resilience/menu-open-close-50: the File drop-down is opened by a click on its label and closed with Escape, 50 times. Expected: each time a drop-down is in the table after the
// click and none after the Escape; no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { openMenu, POPUP, windowTable } from '../../lib/common.mjs';
import { mainFormHere, press, stateName, waitUntil } from '../../lib/resilience.mjs';

const drops = async (guest) => (await windowTable(guest)).top.filter((w) => w.cls === POPUP).length;

export default defineScenario({
  id: 'resilience/menu-open-close-50',
  covers: ['resilience/menu/open-close-cycle'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    let opened = 0, closed = 0;
    const fails = [];
    for (let i = 1; i <= 50; i++) {
      await openMenu(guest, 'File');
      if (await waitUntil(async () => (await drops(guest)) >= 1, 3000)) opened++; else fails.push({ cycle: i, step: 'open' });
      await press(guest, 'Escape');
      if (await waitUntil(async () => (await drops(guest)) === 0, 2500)) closed++; else { fails.push({ cycle: i, step: 'close' }); await press(guest, 'Escape'); }
    }
    const state = await stateName(guest);
    const main = await mainFormHere(guest);
    return {
      ok: opened === 50 && closed === 50 && state === 'engine-idle' && main, engineState: state,
      notes: [`File drop-down opened ${opened} of 50, closed ${closed}; fails ${JSON.stringify(fails.slice(0, 5))}; main form ${main}`],
      results: { cycles: 50, opened, closed, fails: fails.length },
    };
  },
});
