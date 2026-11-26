// plugins/builtin-effects-fruity-delay-fruity-delay-x64: Fruity Delay (effects, DLL Fruity Delay_x64.dll (3773440 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-delay-fruity-delay-x64',
  covers: ['plugins/effects/fruity-delay-fruity-delay-x64', 'plugins/window/effects-fruity-delay-fruity-delay-x64', 'plugins/param/effects-fruity-delay-fruity-delay-x64', 'plugins/audio/effects-fruity-delay-fruity-delay-x64', 'plugins/stop/effects-fruity-delay-fruity-delay-x64'],
  expectFail: '#1440',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Delay", kind: 'effects' });
  },
});
