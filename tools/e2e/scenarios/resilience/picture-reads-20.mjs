// resilience/picture-reads-20: the whole picture is read 20 times in a row. Expected: each read answers with a picture the size of the guest's screen (the size is read at run time).
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { pictureMatchesGuest, stateName } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/picture-reads-20',
  covers: ['resilience/window/picture-readable-repeat'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    let ok = 0, bad = 0, failed = 0;
    for (let i = 0; i < 20; i++) {
      try {
        if (await pictureMatchesGuest(guest)) ok++; else bad++;
      } catch (e) {
        failed++;
      }
    }
    const state = await stateName(guest);
    return {
      ok: bad === 0 && failed === 0 && ok === 20 && state === 'engine-idle', engineState: state,
      notes: [`20 picture reads: the guest's size ${ok}, another size ${bad}, failed ${failed}`],
      results: { reads: 20, sizeOk: ok, sizeOther: bad, failed },
    };
  },
});
