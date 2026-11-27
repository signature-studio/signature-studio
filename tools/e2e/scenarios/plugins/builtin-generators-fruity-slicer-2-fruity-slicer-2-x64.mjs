// plugins/builtin-generators-fruity-slicer-2-fruity-slicer-2-x64: Fruity Slicer 2 (generators, DLL Fruity Slicer 2_x64.dll (5930496 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-slicer-2-fruity-slicer-2-x64',
  covers: ['plugins/generators/fruity-slicer-2-fruity-slicer-2-x64', 'plugins/window/generators-fruity-slicer-2-fruity-slicer-2-x64', 'plugins/param/generators-fruity-slicer-2-fruity-slicer-2-x64', 'plugins/audio/generators-fruity-slicer-2-fruity-slicer-2-x64', 'plugins/stop/generators-fruity-slicer-2-fruity-slicer-2-x64'],
  expectFail: '#1535',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Slicer 2", kind: 'generators' });
  },
});
