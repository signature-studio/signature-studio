// topbar/control-bar-present: the top bar's control bar (TQuickControlBar) is shown across the main form's width.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'topbar/control-bar-present',
  covers: ['topbar/part/tquickcontrolbar'],
  async run({ guest }) {
    const t = await windowTable(guest);
    const bars = t.children.filter((c) => c.cls === 'TQuickControlBar' && c.vis && c.w > 1000);
    return { ok: bars.length >= 1, engineState: (await guest.state())?.state ?? null, notes: [`control bars shown wider than 1000 px: ${bars.length}`] };
  },
});
