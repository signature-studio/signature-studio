// plugins/builtin-generators-fruity-slicer-fruity-slicer-x64: Fruity Slicer (generators, DLL Fruity Slicer_x64.dll (4591104 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-slicer-fruity-slicer-x64',
  covers: ['plugins/generators/fruity-slicer-fruity-slicer-x64', 'plugins/window/generators-fruity-slicer-fruity-slicer-x64', 'plugins/param/generators-fruity-slicer-fruity-slicer-x64', 'plugins/audio/generators-fruity-slicer-fruity-slicer-x64', 'plugins/stop/generators-fruity-slicer-fruity-slicer-x64'],
  expectFail: '#1470',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Slicer", kind: 'generators' });
  },
});
