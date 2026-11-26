// plugins/builtin-effects-fruity-delay-bank-fruity-delay-bank-x64: Fruity Delay Bank (effects, DLL Fruity Delay Bank_x64.dll (3917312 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-delay-bank-fruity-delay-bank-x64',
  covers: ['plugins/effects/fruity-delay-bank-fruity-delay-bank-x64', 'plugins/window/effects-fruity-delay-bank-fruity-delay-bank-x64', 'plugins/param/effects-fruity-delay-bank-fruity-delay-bank-x64', 'plugins/audio/effects-fruity-delay-bank-fruity-delay-bank-x64', 'plugins/stop/effects-fruity-delay-bank-fruity-delay-bank-x64'],
  expectFail: '#1440',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Delay Bank", kind: 'effects' });
  },
});
