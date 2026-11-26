// plugins/builtin-generators-midi-out-midi-out-x64: MIDI Out (generators, DLL MIDI Out_x64.dll (4047872 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-midi-out-midi-out-x64',
  covers: ['plugins/generators/midi-out-midi-out-x64', 'plugins/window/generators-midi-out-midi-out-x64', 'plugins/param/generators-midi-out-midi-out-x64', 'plugins/audio/generators-midi-out-midi-out-x64', 'plugins/stop/generators-midi-out-midi-out-x64'],
  expectFail: '#1472',
  async run({ guest }) {
    return pluginSteps(guest, { name: "MIDI Out", kind: 'generators' });
  },
});
