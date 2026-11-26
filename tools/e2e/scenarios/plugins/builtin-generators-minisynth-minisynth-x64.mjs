// plugins/builtin-generators-minisynth-minisynth-x64: Minisynth (generators, DLL MiniSynth_x64.dll (555008 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-minisynth-minisynth-x64',
  covers: ['plugins/generators/minisynth-minisynth-x64', 'plugins/window/generators-minisynth-minisynth-x64', 'plugins/param/generators-minisynth-minisynth-x64', 'plugins/audio/generators-minisynth-minisynth-x64', 'plugins/stop/generators-minisynth-minisynth-x64'],
  expectFail: '#1472',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Minisynth", kind: 'generators' });
  },
});
