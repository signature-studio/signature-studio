// plugins/builtin-effects-vfx-script-vfx-script-x64: VFX Script (effects, DLL VFX Script_x64.dll (4495872 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vfx-script-vfx-script-x64',
  covers: ['plugins/effects/vfx-script-vfx-script-x64', 'plugins/window/effects-vfx-script-vfx-script-x64', 'plugins/param/effects-vfx-script-vfx-script-x64', 'plugins/audio/effects-vfx-script-vfx-script-x64', 'plugins/stop/effects-vfx-script-vfx-script-x64'],
  expectFail: '#1460',
  async run({ guest }) {
    return pluginSteps(guest, { name: "VFX Script", kind: 'effects' });
  },
});
