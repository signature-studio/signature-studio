// plugins/position-effects-spreader-spreader-x64: the window of Spreader lies inside the desktop, after the plugin is put into Mixer Insert 1 from the Plugin database (the display's DPR may be random: the guest picture is the same size). A generated scenario. Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { insertFromDatabase, pluginInDesktop } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/position-effects-spreader-spreader-x64',
  covers: ['plugins/window/effects-spreader-spreader-x64'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: '#1457',
  async run({ guest }) {
    const ins = await insertFromDatabase(guest, "Spreader");
    if (!ins.ok) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ins.notes, results: ins.results };
    const r = await pluginInDesktop(guest, "Spreader");
    return { ok: r.inside, engineState: (await guest.state())?.state ?? null, notes: [...ins.notes, ...r.notes], results: ins.results };
  },
});
