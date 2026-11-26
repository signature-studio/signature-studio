// plugins/builtin-generators-boobass-boobass-x64: BooBass (generators, DLL BooBass_x64.dll (3685376 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-boobass-boobass-x64',
  covers: ['plugins/generators/boobass-boobass-x64', 'plugins/window/generators-boobass-boobass-x64', 'plugins/param/generators-boobass-boobass-x64', 'plugins/audio/generators-boobass-boobass-x64', 'plugins/stop/generators-boobass-boobass-x64'],
  expectFail: '#1464',
  async run({ guest }) {
    return pluginSteps(guest, { name: "BooBass", kind: 'generators' });
  },
});
