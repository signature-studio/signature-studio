// plugins/builtin-effects-maximus-maximus-x64: Maximus (effects, DLL Maximus_x64.dll (5904384 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-maximus-maximus-x64',
  covers: ['plugins/effects/maximus-maximus-x64', 'plugins/window/effects-maximus-maximus-x64', 'plugins/param/effects-maximus-maximus-x64', 'plugins/audio/effects-maximus-maximus-x64', 'plugins/stop/effects-maximus-maximus-x64'],
  expectFail: '#1454',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Maximus", kind: 'effects' });
  },
});
