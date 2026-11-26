// plugins/builtin-generators-fruity-dance-fruity-dance-x64: Fruity Dance (generators, DLL Fruity dance_x64.dll (4163072 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-dance-fruity-dance-x64',
  covers: ['plugins/generators/fruity-dance-fruity-dance-x64', 'plugins/window/generators-fruity-dance-fruity-dance-x64', 'plugins/param/generators-fruity-dance-fruity-dance-x64', 'plugins/audio/generators-fruity-dance-fruity-dance-x64', 'plugins/stop/generators-fruity-dance-fruity-dance-x64'],
  expectFail: '#1468',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Dance", kind: 'generators' });
  },
});
