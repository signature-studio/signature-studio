// plugins/builtin-effects-equo-equo-x64: EQUO (effects, DLL EQUO_x64.dll (3817472 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-equo-equo-x64',
  covers: ['plugins/effects/equo-equo-x64', 'plugins/window/effects-equo-equo-x64', 'plugins/param/effects-equo-equo-x64', 'plugins/audio/effects-equo-equo-x64', 'plugins/stop/effects-equo-equo-x64'],
  expectFail: '#1434',
  async run({ guest }) {
    return pluginSteps(guest, { name: "EQUO", kind: 'effects' });
  },
});
