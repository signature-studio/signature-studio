// plugins/builtin-effects-soundgoodizer-soundgoodizer-x64: Soundgoodizer (effects, DLL Soundgoodizer_x64.dll (6407680 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-soundgoodizer-soundgoodizer-x64',
  covers: ['plugins/effects/soundgoodizer-soundgoodizer-x64', 'plugins/window/effects-soundgoodizer-soundgoodizer-x64', 'plugins/param/effects-soundgoodizer-soundgoodizer-x64', 'plugins/audio/effects-soundgoodizer-soundgoodizer-x64', 'plugins/stop/effects-soundgoodizer-soundgoodizer-x64'],
  expectFail: '#1456',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Soundgoodizer", kind: 'effects' });
  },
});
