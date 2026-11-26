// plugins/builtin-effects-hardcore-hardcore-x64: Hardcore (effects, DLL Hardcore_x64.dll (641536 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-hardcore-hardcore-x64',
  covers: ['plugins/effects/hardcore-hardcore-x64', 'plugins/window/effects-hardcore-hardcore-x64', 'plugins/param/effects-hardcore-hardcore-x64', 'plugins/audio/effects-hardcore-hardcore-x64', 'plugins/stop/effects-hardcore-hardcore-x64'],
  expectFail: '#1452',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Hardcore", kind: 'effects' });
  },
});
