// plugins/builtin-generators-fruity-wrapper-fruity-wrapper-x64: Fruity Wrapper (generators, DLL Fruity Wrapper_x64.dll (5998080 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-wrapper-fruity-wrapper-x64',
  covers: ['plugins/generators/fruity-wrapper-fruity-wrapper-x64', 'plugins/window/generators-fruity-wrapper-fruity-wrapper-x64', 'plugins/param/generators-fruity-wrapper-fruity-wrapper-x64', 'plugins/audio/generators-fruity-wrapper-fruity-wrapper-x64', 'plugins/stop/generators-fruity-wrapper-fruity-wrapper-x64'],
  expectFail: '#1471',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Wrapper", kind: 'generators' });
  },
});
