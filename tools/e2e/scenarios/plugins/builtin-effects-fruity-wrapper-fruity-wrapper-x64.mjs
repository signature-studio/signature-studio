// plugins/builtin-effects-fruity-wrapper-fruity-wrapper-x64: Fruity Wrapper (effects, DLL Fruity Wrapper_x64.dll (5997056 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-wrapper-fruity-wrapper-x64',
  covers: ['plugins/effects/fruity-wrapper-fruity-wrapper-x64', 'plugins/window/effects-fruity-wrapper-fruity-wrapper-x64', 'plugins/param/effects-fruity-wrapper-fruity-wrapper-x64', 'plugins/audio/effects-fruity-wrapper-fruity-wrapper-x64', 'plugins/stop/effects-fruity-wrapper-fruity-wrapper-x64'],
  expectFail: '#1451',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Wrapper", kind: 'effects' });
  },
});
