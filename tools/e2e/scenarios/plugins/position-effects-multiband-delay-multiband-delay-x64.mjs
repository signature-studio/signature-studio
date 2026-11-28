// plugins/position-effects-multiband-delay-multiband-delay-x64: the window of Multiband Delay lies inside the desktop, after the plugin is put into Mixer Insert 1 from the Plugin database (the display's DPR may be random: the guest picture is the same size). A generated scenario. Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { insertFromDatabase, pluginInDesktop } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/position-effects-multiband-delay-multiband-delay-x64',
  covers: ['plugins/window/effects-multiband-delay-multiband-delay-x64'],
  displays: ['4k-dpr1', '4k-random'],
  expectFail: '#1454',
  async run({ guest }) {
    const ins = await insertFromDatabase(guest, "Multiband Delay");
    if (!ins.ok) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: ins.notes, results: ins.results };
    const r = await pluginInDesktop(guest, "Multiband Delay");
    return { ok: r.inside, engineState: (await guest.state())?.state ?? null, notes: [...ins.notes, ...r.notes], results: ins.results };
  },
});
