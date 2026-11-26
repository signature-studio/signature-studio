// plugins/builtin-generators-fl-studio-mobile-rack-fl-studio-mobile-rack-x64: FL Studio Mobile Rack (generators, DLL FL Studio Mobile Rack_x64.dll (8231936 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fl-studio-mobile-rack-fl-studio-mobile-rack-x64',
  covers: ['plugins/generators/fl-studio-mobile-rack-fl-studio-mobile-rack-x64', 'plugins/window/generators-fl-studio-mobile-rack-fl-studio-mobile-rack-x64', 'plugins/param/generators-fl-studio-mobile-rack-fl-studio-mobile-rack-x64', 'plugins/audio/generators-fl-studio-mobile-rack-fl-studio-mobile-rack-x64', 'plugins/stop/generators-fl-studio-mobile-rack-fl-studio-mobile-rack-x64'],
  expectFail: '#1467',
  async run({ guest }) {
    return pluginSteps(guest, { name: "FL Studio Mobile Rack", kind: 'generators' });
  },
});
