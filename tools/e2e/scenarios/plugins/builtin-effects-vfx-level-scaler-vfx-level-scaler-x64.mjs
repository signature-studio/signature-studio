// plugins/builtin-effects-vfx-level-scaler-vfx-level-scaler-x64: VFX Level Scaler (effects, DLL VFX Level Scaler_x64.dll (4106752 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vfx-level-scaler-vfx-level-scaler-x64',
  covers: ['plugins/effects/vfx-level-scaler-vfx-level-scaler-x64', 'plugins/window/effects-vfx-level-scaler-vfx-level-scaler-x64', 'plugins/param/effects-vfx-level-scaler-vfx-level-scaler-x64', 'plugins/audio/effects-vfx-level-scaler-vfx-level-scaler-x64', 'plugins/stop/effects-vfx-level-scaler-vfx-level-scaler-x64'],
  expectFail: '#1460',
  async run({ guest }) {
    return pluginSteps(guest, { name: "VFX Level Scaler", kind: 'effects' });
  },
});
