// page/first-frame-drawn: after the start the guest picture is drawn: fewer than half of its 64 x 64 tiles are black (lib/common.mjs noBlackRegions).
import { defineScenario } from '../../lib/scenario.mjs';
import { noBlackRegions, picture } from '../../lib/common.mjs';

export default defineScenario({
  id: 'page/first-frame-drawn',
  covers: ['page/module/beacon'],
  async run({ guest }) {
    const pic = await picture(guest);
    const total = Math.ceil(pic.w / 64) * Math.ceil(pic.h / 64);
    const black = await noBlackRegions(guest);
    const state = (await guest.state())?.state ?? null;
    return { ok: state === 'engine-idle' && black < total / 2, engineState: state, notes: [`black tiles ${black} of ${total} (${pic.w}x${pic.h})`] };
  },
});
