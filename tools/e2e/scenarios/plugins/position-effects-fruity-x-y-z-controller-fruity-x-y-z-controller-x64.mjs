// plugins/position-effects-fruity-x-y-z-controller-fruity-x-y-z-controller-x64: the window of Fruity X-Y-Z Controller lies inside the desktop (the display's DPR may be random: the guest picture is the same size). A generated scenario. Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { openPluginDatabase, pluginInDesktop } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/position-effects-fruity-x-y-z-controller-fruity-x-y-z-controller-x64',
  covers: ['plugins/window/effects-fruity-x-y-z-controller-fruity-x-y-z-controller-x64'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: '#1451',
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const r = await pluginInDesktop(guest, "Fruity X-Y-Z Controller");
    return { ok: r.inside, engineState: (await guest.state())?.state ?? null, notes: r.notes };
  },
});
