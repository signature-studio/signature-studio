// plugins/position-effects-gross-beat-gross-beat-x64: the window of Gross Beat lies inside the desktop, after the plugin is put into Mixer Insert 1 from the Plugin database (the display's DPR may be random: the guest picture is the same size). A generated scenario. Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { insertFromDatabase, pluginInDesktop } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/position-effects-gross-beat-gross-beat-x64',
  covers: ['plugins/window/effects-gross-beat-gross-beat-x64'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: '#1452',
  async run({ guest }) {
    const ins = await insertFromDatabase(guest, "Gross Beat");
    if (!ins.ok) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ins.notes, results: ins.results };
    const r = await pluginInDesktop(guest, "Gross Beat");
    return { ok: r.inside, engineState: (await guest.state())?.state ?? null, notes: [...ins.notes, ...r.notes], results: ins.results };
  },
});
