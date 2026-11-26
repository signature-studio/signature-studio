// plugins/builtin-generators-fruity-drumsynth-live-fruity-drumsynth-live-x64: Fruity DrumSynth Live (generators, DLL Fruity DrumSynth Live_x64.dll (4187136 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-drumsynth-live-fruity-drumsynth-live-x64',
  covers: ['plugins/generators/fruity-drumsynth-live-fruity-drumsynth-live-x64', 'plugins/window/generators-fruity-drumsynth-live-fruity-drumsynth-live-x64', 'plugins/param/generators-fruity-drumsynth-live-fruity-drumsynth-live-x64', 'plugins/audio/generators-fruity-drumsynth-live-fruity-drumsynth-live-x64', 'plugins/stop/generators-fruity-drumsynth-live-fruity-drumsynth-live-x64'],
  expectFail: '#1468',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity DrumSynth Live", kind: 'generators' });
  },
});
