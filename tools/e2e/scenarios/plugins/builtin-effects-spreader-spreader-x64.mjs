// plugins/builtin-effects-spreader-spreader-x64: Spreader (effects, DLL Spreader_x64.dll (3912704 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-spreader-spreader-x64',
  covers: ['plugins/effects/spreader-spreader-x64', 'plugins/window/effects-spreader-spreader-x64', 'plugins/param/effects-spreader-spreader-x64', 'plugins/audio/effects-spreader-spreader-x64', 'plugins/stop/effects-spreader-spreader-x64'],
  expectFail: '#1457',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Spreader", kind: 'effects' });
  },
});
