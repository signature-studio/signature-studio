// plugins/builtin-effects-emphasis-emphasis-x64: Emphasis (effects, DLL Emphasis_x64.dll (4142080 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-emphasis-emphasis-x64',
  covers: ['plugins/effects/emphasis-emphasis-x64', 'plugins/window/effects-emphasis-emphasis-x64', 'plugins/param/effects-emphasis-emphasis-x64', 'plugins/audio/effects-emphasis-emphasis-x64', 'plugins/stop/effects-emphasis-emphasis-x64'],
  expectFail: '#1435',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Emphasis", kind: 'effects' });
  },
});
