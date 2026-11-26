// plugins/builtin-generators-dashboard-dashboard-x64: Dashboard (generators, DLL Dashboard_x64.dll (5036544 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-dashboard-dashboard-x64',
  covers: ['plugins/generators/dashboard-dashboard-x64', 'plugins/window/generators-dashboard-dashboard-x64', 'plugins/param/generators-dashboard-dashboard-x64', 'plugins/audio/generators-dashboard-dashboard-x64', 'plugins/stop/generators-dashboard-dashboard-x64'],
  expectFail: '#1464',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Dashboard", kind: 'generators' });
  },
});
