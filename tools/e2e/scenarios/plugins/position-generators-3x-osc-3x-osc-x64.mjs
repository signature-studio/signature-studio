// plugins/position-generators-3x-osc-3x-osc-x64: the window of 3x Osc lies inside the desktop (the display's DPR may be random: the guest picture is the same size). A generated scenario. Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { openPluginDatabase, pluginInDesktop } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/position-generators-3x-osc-3x-osc-x64',
  covers: ['plugins/window/generators-3x-osc-3x-osc-x64'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: '#1463',
  async run({ guest }) {
    const db = await openPluginDatabase(guest);
    if (!db.opened) return { ok: false, notes: [`the picker did not open (${db.error})`] };
    const r = await pluginInDesktop(guest, "3x Osc");
    return { ok: r.inside, engineState: (await guest.state())?.state ?? null, notes: r.notes };
  },
});
