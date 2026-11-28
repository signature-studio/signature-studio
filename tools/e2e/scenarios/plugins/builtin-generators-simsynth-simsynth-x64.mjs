// plugins/builtin-generators-simsynth-simsynth-x64: SimSynth (generators, DLL SimSynth_x64.dll (3850752 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-simsynth-simsynth-x64',
  covers: ['plugins/generators/simsynth-simsynth-x64', 'plugins/window/generators-simsynth-simsynth-x64', 'plugins/param/generators-simsynth-simsynth-x64', 'plugins/audio/generators-simsynth-simsynth-x64', 'plugins/stop/generators-simsynth-simsynth-x64'],
  expectFail: '#1475',
  async run({ guest }) {
    return pluginSteps(guest, { name: "SimSynth", kind: 'generators' });
  },
});
