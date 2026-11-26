// plugins/builtin-effects-vintage-phaser-vintage-phaser-x64: Vintage Phaser (effects, DLL Vintage Phaser_x64.dll (3973632 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vintage-phaser-vintage-phaser-x64',
  covers: ['plugins/effects/vintage-phaser-vintage-phaser-x64', 'plugins/window/effects-vintage-phaser-vintage-phaser-x64', 'plugins/param/effects-vintage-phaser-vintage-phaser-x64', 'plugins/audio/effects-vintage-phaser-vintage-phaser-x64', 'plugins/stop/effects-vintage-phaser-vintage-phaser-x64'],
  expectFail: '#1461',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Vintage Phaser", kind: 'effects' });
  },
});
