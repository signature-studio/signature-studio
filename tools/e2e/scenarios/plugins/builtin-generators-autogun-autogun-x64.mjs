// plugins/builtin-generators-autogun-autogun-x64: Autogun (generators, DLL Autogun_x64.dll (6291456 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-autogun-autogun-x64',
  covers: ['plugins/generators/autogun-autogun-x64', 'plugins/window/generators-autogun-autogun-x64', 'plugins/param/generators-autogun-autogun-x64', 'plugins/audio/generators-autogun-autogun-x64', 'plugins/stop/generators-autogun-autogun-x64'],
  expectFail: '#1463',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Autogun", kind: 'generators' });
  },
});
