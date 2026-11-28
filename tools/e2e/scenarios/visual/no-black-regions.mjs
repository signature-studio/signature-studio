// visual/no-black-regions: at most 10 % of the guest picture's 64 x 64 tiles are all black (the share is recorded; the empty desktop around the windows is black).
import { defineScenario } from '../../lib/scenario.mjs';
import { noBlackRegions, picture } from '../../lib/common.mjs';

export default defineScenario({
  id: 'visual/no-black-regions',
  covers: [],
  async run({ guest }) {
    const pic = await picture(guest);
    const total = Math.ceil(pic.w / 64) * Math.ceil(pic.h / 64);
    const black = await noBlackRegions(guest);
    return { ok: black <= total * 0.1, engineState: (await guest.state())?.state ?? null, notes: [`black tiles ${black} of ${total} (${(100 * black / total).toFixed(1)} %)`] };
  },
});
