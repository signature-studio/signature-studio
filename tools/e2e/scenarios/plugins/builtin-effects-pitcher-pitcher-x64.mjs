// plugins/builtin-effects-pitcher-pitcher-x64: Pitcher (effects, DLL Pitcher_x64.dll (446464 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-pitcher-pitcher-x64',
  covers: ['plugins/effects/pitcher-pitcher-x64', 'plugins/window/effects-pitcher-pitcher-x64', 'plugins/param/effects-pitcher-pitcher-x64', 'plugins/audio/effects-pitcher-pitcher-x64', 'plugins/stop/effects-pitcher-pitcher-x64'],
  expectFail: '#1456',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Pitcher", kind: 'effects' });
  },
});
