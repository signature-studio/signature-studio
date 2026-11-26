// plugins/builtin-effects-fruity-limiter-fruity-limiter-x64: Fruity Limiter (effects, DLL Fruity Limiter_x64.dll (3868160 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-limiter-fruity-limiter-x64',
  covers: ['plugins/effects/fruity-limiter-fruity-limiter-x64', 'plugins/window/effects-fruity-limiter-fruity-limiter-x64', 'plugins/param/effects-fruity-limiter-fruity-limiter-x64', 'plugins/audio/effects-fruity-limiter-fruity-limiter-x64', 'plugins/stop/effects-fruity-limiter-fruity-limiter-x64'],
  expectFail: '#1443',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Limiter", kind: 'effects' });
  },
});
