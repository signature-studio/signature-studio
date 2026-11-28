// plugins/builtin-generators-flex-flex-x64: FLEX (generators, DLL FLEX_x64.dll (5194752 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-flex-flex-x64',
  covers: ['plugins/generators/flex-flex-x64', 'plugins/window/generators-flex-flex-x64', 'plugins/param/generators-flex-flex-x64', 'plugins/audio/generators-flex-flex-x64', 'plugins/stop/generators-flex-flex-x64'],
  expectFail: '#1535',
  async run({ guest }) {
    return pluginSteps(guest, { name: "FLEX", kind: 'generators' });
  },
});
