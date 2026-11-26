// plugins/builtin-effects-fruity-phaser-fruity-phaser-x64: Fruity Phaser (effects, DLL Fruity Phaser_x64.dll (3708928 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-phaser-fruity-phaser-x64',
  covers: ['plugins/effects/fruity-phaser-fruity-phaser-x64', 'plugins/window/effects-fruity-phaser-fruity-phaser-x64', 'plugins/param/effects-fruity-phaser-fruity-phaser-x64', 'plugins/audio/effects-fruity-phaser-fruity-phaser-x64', 'plugins/stop/effects-fruity-phaser-fruity-phaser-x64'],
  expectFail: '#1447',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Phaser", kind: 'effects' });
  },
});
