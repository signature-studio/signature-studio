// plugins/builtin-generators-wave-traveller-wave-traveller-x64: Wave Traveller (generators, DLL Wave Traveller_x64.dll (4473856 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-wave-traveller-wave-traveller-x64',
  covers: ['plugins/generators/wave-traveller-wave-traveller-x64', 'plugins/window/generators-wave-traveller-wave-traveller-x64', 'plugins/param/generators-wave-traveller-wave-traveller-x64', 'plugins/audio/generators-wave-traveller-wave-traveller-x64', 'plugins/stop/generators-wave-traveller-wave-traveller-x64'],
  expectFail: '#1477',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Wave Traveller", kind: 'generators' });
  },
});
