// plugins/builtin-effects-fl-studio-mobile-rack-fx-fl-studio-mobile-rack-fx-x64: FL Studio Mobile Rack FX (effects, DLL FL Studio Mobile Rack FX_x64.dll (8228864 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fl-studio-mobile-rack-fx-fl-studio-mobile-rack-fx-x64',
  covers: ['plugins/effects/fl-studio-mobile-rack-fx-fl-studio-mobile-rack-fx-x64', 'plugins/window/effects-fl-studio-mobile-rack-fx-fl-studio-mobile-rack-fx-x64', 'plugins/param/effects-fl-studio-mobile-rack-fx-fl-studio-mobile-rack-fx-x64', 'plugins/audio/effects-fl-studio-mobile-rack-fx-fl-studio-mobile-rack-fx-x64', 'plugins/stop/effects-fl-studio-mobile-rack-fx-fl-studio-mobile-rack-fx-x64'],
  expectFail: '#1436',
  async run({ guest }) {
    return pluginSteps(guest, { name: "FL Studio Mobile Rack FX", kind: 'effects' });
  },
});
