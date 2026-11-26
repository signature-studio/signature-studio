// plugins/builtin-generators-fruity-video-player-fruity-video-player-x64: Fruity Video Player (generators, DLL Fruity Video Player_x64.dll (4281856 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-generators-fruity-video-player-fruity-video-player-x64',
  covers: ['plugins/generators/fruity-video-player-fruity-video-player-x64', 'plugins/window/generators-fruity-video-player-fruity-video-player-x64', 'plugins/param/generators-fruity-video-player-fruity-video-player-x64', 'plugins/audio/generators-fruity-video-player-fruity-video-player-x64', 'plugins/stop/generators-fruity-video-player-fruity-video-player-x64'],
  expectFail: '#1470',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Video Player", kind: 'generators' });
  },
});
