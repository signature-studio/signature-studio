// plugins/builtin-generators-fl-keys-fl-keys-x64: FL Keys (generators, DLL FL Keys_x64.dll (4213248 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fl-keys-fl-keys-x64',
  covers: ['plugins/generators/fl-keys-fl-keys-x64', 'plugins/window/generators-fl-keys-fl-keys-x64', 'plugins/param/generators-fl-keys-fl-keys-x64', 'plugins/audio/generators-fl-keys-fl-keys-x64', 'plugins/stop/generators-fl-keys-fl-keys-x64'],
  expectFail: '#1466',
  async run({ guest }) {
    return pluginSteps(guest, { name: "FL Keys", kind: 'generators' });
  },
});
