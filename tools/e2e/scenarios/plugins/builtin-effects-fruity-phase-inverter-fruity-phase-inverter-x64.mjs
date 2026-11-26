// plugins/builtin-effects-fruity-phase-inverter-fruity-phase-inverter-x64: Fruity Phase Inverter (effects, DLL Fruity Phase Inverter_x64.dll (3689984 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-fruity-phase-inverter-fruity-phase-inverter-x64',
  covers: ['plugins/effects/fruity-phase-inverter-fruity-phase-inverter-x64', 'plugins/window/effects-fruity-phase-inverter-fruity-phase-inverter-x64', 'plugins/param/effects-fruity-phase-inverter-fruity-phase-inverter-x64', 'plugins/audio/effects-fruity-phase-inverter-fruity-phase-inverter-x64', 'plugins/stop/effects-fruity-phase-inverter-fruity-phase-inverter-x64'],
  expectFail: '#1446',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Fruity Phase Inverter", kind: 'effects' });
  },
});
