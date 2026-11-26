// plugins/picker-opens: a slow double-click on Mixer Insert 1 selects it; a click on its Slot 1 opens the plugin picker (a drop-down: More plugins..., Plugin database, the view modes).
import { defineScenario } from '../../lib/scenario.mjs';
import { openSlotPicker } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/picker-opens',
  covers: ['plugins/window/mixer-slot-picker'],
  async run({ guest }) {
    const r = await openSlotPicker(guest);
    if (r.error) return { ok: false, notes: [r.error] };
    const p = r.pop;
    return {
      ok: !!p,
      engineState: (await guest.state())?.state ?? null,
      notes: [p ? `picker ${p.cls ?? 'TQuickPopupMenuWindow'} at ${p.x},${p.y} ${p.w}x${p.h}` : 'no picker after Slot 1 was clicked'],
      results: { picker: p ? { x: p.x, y: p.y, w: p.w, h: p.h } : null },
    };
  },
});
