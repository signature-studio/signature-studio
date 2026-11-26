// plugins/builtin-generators-fl-slayer-fl-slayer-x64: FL Slayer (generators, DLL FL Slayer_x64.dll (5743104 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fl-slayer-fl-slayer-x64',
  covers: ['plugins/generators/fl-slayer-fl-slayer-x64', 'plugins/window/generators-fl-slayer-fl-slayer-x64', 'plugins/param/generators-fl-slayer-fl-slayer-x64', 'plugins/audio/generators-fl-slayer-fl-slayer-x64', 'plugins/stop/generators-fl-slayer-fl-slayer-x64'],
  expectFail: '#1466',
  async run({ guest }) {
    return pluginSteps(guest, { name: "FL Slayer", kind: 'generators' });
  },
});
