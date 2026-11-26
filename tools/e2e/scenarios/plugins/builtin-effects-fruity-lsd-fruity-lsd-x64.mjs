// plugins/builtin-effects-fruity-lsd-fruity-lsd-x64: Fruity LSD (effects, DLL Fruity LSD_x64.dll (4504064 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-lsd-fruity-lsd-x64',
  covers: ['plugins/effects/fruity-lsd-fruity-lsd-x64', 'plugins/window/effects-fruity-lsd-fruity-lsd-x64', 'plugins/param/effects-fruity-lsd-fruity-lsd-x64', 'plugins/audio/effects-fruity-lsd-fruity-lsd-x64', 'plugins/stop/effects-fruity-lsd-fruity-lsd-x64'],
  expectFail: '#1443',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity LSD", kind: 'effects' });
  },
});
