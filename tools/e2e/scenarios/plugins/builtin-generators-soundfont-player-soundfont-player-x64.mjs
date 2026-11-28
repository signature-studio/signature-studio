// plugins/builtin-generators-soundfont-player-soundfont-player-x64: Soundfont Player (generators, DLL Soundfont Player_x64.dll (4551168 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-soundfont-player-soundfont-player-x64',
  covers: ['plugins/generators/soundfont-player-soundfont-player-x64', 'plugins/window/generators-soundfont-player-soundfont-player-x64', 'plugins/param/generators-soundfont-player-soundfont-player-x64', 'plugins/audio/generators-soundfont-player-soundfont-player-x64', 'plugins/stop/generators-soundfont-player-soundfont-player-x64'],
  expectFail: '#1476',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Soundfont Player", kind: 'generators' });
  },
});
