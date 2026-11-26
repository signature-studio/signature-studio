// plugins/builtin-generators-fl-studio-mobile-fl-studio-mobile-x64: FL Studio Mobile (generators, DLL FL Studio Mobile_x64.dll (12141568 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fl-studio-mobile-fl-studio-mobile-x64',
  covers: ['plugins/generators/fl-studio-mobile-fl-studio-mobile-x64', 'plugins/window/generators-fl-studio-mobile-fl-studio-mobile-x64', 'plugins/param/generators-fl-studio-mobile-fl-studio-mobile-x64', 'plugins/audio/generators-fl-studio-mobile-fl-studio-mobile-x64', 'plugins/stop/generators-fl-studio-mobile-fl-studio-mobile-x64'],
  expectFail: '#1466',
  async run({ guest }) {
    return pluginSteps(guest, { name: "FL Studio Mobile", kind: 'generators' });
  },
});
