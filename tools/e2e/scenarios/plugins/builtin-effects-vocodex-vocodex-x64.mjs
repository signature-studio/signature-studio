// plugins/builtin-effects-vocodex-vocodex-x64: Vocodex (effects, DLL Vocodex_x64.dll (5957632 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vocodex-vocodex-x64',
  covers: ['plugins/effects/vocodex-vocodex-x64', 'plugins/window/effects-vocodex-vocodex-x64', 'plugins/param/effects-vocodex-vocodex-x64', 'plugins/audio/effects-vocodex-vocodex-x64', 'plugins/stop/effects-vocodex-vocodex-x64'],
  expectFail: '#1461',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Vocodex", kind: 'effects' });
  },
});
