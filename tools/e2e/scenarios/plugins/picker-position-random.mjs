// plugins/picker-position-random: the plugin picker of Insert 1's Slot 1 lies inside the desktop at the display's DPR (the 4k-random display draws the DPR at random).
import { defineScenario } from '../../lib/scenario.mjs';
import { openSlotPicker } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/picker-position-random',
  covers: ['plugins/window/mixer-slot-picker'],
  displays: ['4k-dpr1', '4k-random'],
  async run({ guest }) {
    const d = await guest.desk();
    const r = await openSlotPicker(guest);
    if (r.error) return { ok: false, notes: [r.error] };
    const p = r.pop;
    const inside = !!p && p.x >= 0 && p.y >= 0 && p.x + p.w <= d.gw && p.y + p.h <= d.gh;
    return { ok: inside, engineState: (await guest.state())?.state ?? null, notes: [p ? `picker at ${p.x},${p.y} ${p.w}x${p.h} in the desktop ${d.gw}x${d.gh}: ${inside}` : 'no picker after Slot 1 was clicked'] };
  },
});
