// plugins/builtin-effects-newtime-newtime-x64: Newtime (effects, DLL Newtime_x64.dll (5082112 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-newtime-newtime-x64',
  covers: ['plugins/effects/newtime-newtime-x64', 'plugins/window/effects-newtime-newtime-x64', 'plugins/param/effects-newtime-newtime-x64', 'plugins/audio/effects-newtime-newtime-x64', 'plugins/stop/effects-newtime-newtime-x64'],
  expectFail: '#1454',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Newtime", kind: 'effects' });
  },
});
