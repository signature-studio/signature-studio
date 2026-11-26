// plugins/builtin-generators-kepler-kepler-x64: Kepler (generators, DLL Kepler_x64.dll (3860480 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-kepler-kepler-x64',
  covers: ['plugins/generators/kepler-kepler-x64', 'plugins/window/generators-kepler-kepler-x64', 'plugins/param/generators-kepler-kepler-x64', 'plugins/audio/generators-kepler-kepler-x64', 'plugins/stop/generators-kepler-kepler-x64'],
  expectFail: 'new',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Kepler", kind: 'generators' });
  },
});
