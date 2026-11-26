// plugins/builtin-effects-multiband-delay-multiband-delay-x64: Multiband Delay (effects, DLL Multiband Delay_x64.dll (3932160 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-multiband-delay-multiband-delay-x64',
  covers: ['plugins/effects/multiband-delay-multiband-delay-x64', 'plugins/window/effects-multiband-delay-multiband-delay-x64', 'plugins/param/effects-multiband-delay-multiband-delay-x64', 'plugins/audio/effects-multiband-delay-multiband-delay-x64', 'plugins/stop/effects-multiband-delay-multiband-delay-x64'],
  expectFail: '#1454',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Multiband Delay", kind: 'effects' });
  },
});
