// plugins/position-effects-zgameeditor-visualizer-zgameeditor-visualizer-x64: the window of ZGameEditor Visualizer lies inside the desktop (the display's DPR may be random: the guest picture is the same size). A generated scenario. Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { openPluginDatabase, pluginInDesktop } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/position-effects-zgameeditor-visualizer-zgameeditor-visualizer-x64',
  covers: ['plugins/window/effects-zgameeditor-visualizer-zgameeditor-visualizer-x64'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: 'new',
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const r = await pluginInDesktop(guest, "ZGameEditor Visualizer");
    return { ok: r.inside, engineState: (await guest.state())?.state ?? null, notes: r.notes };
  },
});
