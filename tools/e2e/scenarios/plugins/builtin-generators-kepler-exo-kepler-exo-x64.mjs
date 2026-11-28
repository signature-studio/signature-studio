// plugins/builtin-generators-kepler-exo-kepler-exo-x64: Kepler Exo (generators, DLL Kepler Exo_x64.dll (4418048 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-kepler-exo-kepler-exo-x64',
  covers: ['plugins/generators/kepler-exo-kepler-exo-x64', 'plugins/window/generators-kepler-exo-kepler-exo-x64', 'plugins/param/generators-kepler-exo-kepler-exo-x64', 'plugins/audio/generators-kepler-exo-kepler-exo-x64', 'plugins/stop/generators-kepler-exo-kepler-exo-x64'],
  expectFail: '#1535',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Kepler Exo", kind: 'generators' });
  },
});
