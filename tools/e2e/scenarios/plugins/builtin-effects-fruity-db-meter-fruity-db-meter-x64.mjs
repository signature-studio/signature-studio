// plugins/builtin-effects-fruity-db-meter-fruity-db-meter-x64: Fruity dB Meter (effects, DLL Fruity dB meter_x64.dll (3662336 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-db-meter-fruity-db-meter-x64',
  covers: ['plugins/effects/fruity-db-meter-fruity-db-meter-x64', 'plugins/window/effects-fruity-db-meter-fruity-db-meter-x64', 'plugins/param/effects-fruity-db-meter-fruity-db-meter-x64', 'plugins/audio/effects-fruity-db-meter-fruity-db-meter-x64', 'plugins/stop/effects-fruity-db-meter-fruity-db-meter-x64'],
  expectFail: '#1452',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity dB Meter", kind: 'effects' });
  },
});
