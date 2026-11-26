// plugins/builtin-generators-sakura-sakura-x64: Sakura (generators, DLL Sakura_x64.dll (4294656 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-sakura-sakura-x64',
  covers: ['plugins/generators/sakura-sakura-x64', 'plugins/window/generators-sakura-sakura-x64', 'plugins/param/generators-sakura-sakura-x64', 'plugins/audio/generators-sakura-sakura-x64', 'plugins/stop/generators-sakura-sakura-x64'],
  expectFail: '#1475',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Sakura", kind: 'generators' });
  },
});
