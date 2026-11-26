// plugins/builtin-effects-vocodex-synthesizer-sytrus-x64: Vocodex (effects, DLL Sytrus_x64.dll (6308352 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vocodex-synthesizer-sytrus-x64',
  covers: ['plugins/effects/vocodex-synthesizer-sytrus-x64', 'plugins/window/effects-vocodex-synthesizer-sytrus-x64', 'plugins/param/effects-vocodex-synthesizer-sytrus-x64', 'plugins/audio/effects-vocodex-synthesizer-sytrus-x64', 'plugins/stop/effects-vocodex-synthesizer-sytrus-x64'],
  expectFail: '#1461',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Vocodex", kind: 'effects' });
  },
});
