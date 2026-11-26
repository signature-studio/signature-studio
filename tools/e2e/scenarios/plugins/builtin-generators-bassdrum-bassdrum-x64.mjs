// plugins/builtin-generators-bassdrum-bassdrum-x64: BassDrum (generators, DLL BassDrum_x64.dll (4068352 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-bassdrum-bassdrum-x64',
  covers: ['plugins/generators/bassdrum-bassdrum-x64', 'plugins/window/generators-bassdrum-bassdrum-x64', 'plugins/param/generators-bassdrum-bassdrum-x64', 'plugins/audio/generators-bassdrum-bassdrum-x64', 'plugins/stop/generators-bassdrum-bassdrum-x64'],
  expectFail: '#1463',
  async run({ guest }) {
    return pluginSteps(guest, { name: "BassDrum", kind: 'generators' });
  },
});
