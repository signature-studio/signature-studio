// plugins/builtin-effects-fruity-mute-2-fruity-mute-2-x64: Fruity Mute 2 (effects, DLL Fruity Mute 2_x64.dll (3692032 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-mute-2-fruity-mute-2-x64',
  covers: ['plugins/effects/fruity-mute-2-fruity-mute-2-x64', 'plugins/window/effects-fruity-mute-2-fruity-mute-2-x64', 'plugins/param/effects-fruity-mute-2-fruity-mute-2-x64', 'plugins/audio/effects-fruity-mute-2-fruity-mute-2-x64', 'plugins/stop/effects-fruity-mute-2-fruity-mute-2-x64'],
  expectFail: '#1444',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Mute 2", kind: 'effects' });
  },
});
