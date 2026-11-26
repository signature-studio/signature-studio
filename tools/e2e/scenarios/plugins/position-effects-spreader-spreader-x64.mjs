// plugins/position-effects-spreader-spreader-x64: the window of Spreader lies inside the desktop (the display's DPR may be random: the guest picture is the same size). A generated scenario. Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { openPluginDatabase, pluginInDesktop } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/position-effects-spreader-spreader-x64',
  covers: ['plugins/window/effects-spreader-spreader-x64'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: '#1457',
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const r = await pluginInDesktop(guest, "Spreader");
    return { ok: r.inside, engineState: (await guest.state())?.state ?? null, notes: r.notes };
  },
});
