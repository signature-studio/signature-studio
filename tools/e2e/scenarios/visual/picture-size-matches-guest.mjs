// visual/picture-size-matches-guest: the picture the page keeps has the guest's size (the #desktop guest size of lib/guest.mjs desk()).
import { defineScenario } from '../../lib/scenario.mjs';
import { picture } from '../../lib/common.mjs';

export default defineScenario({
  id: 'visual/picture-size-matches-guest',
  covers: [],
  async run({ guest }) {
    const pic = await picture(guest);
    const d = await guest.desk();
    return { ok: pic.w === d.gw && pic.h === d.gh, engineState: (await guest.state())?.state ?? null, notes: [`picture ${pic.w}x${pic.h}, guest ${d.gw}x${d.gh}`] };
  },
});
