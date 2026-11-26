// plugins/builtin-effects-vfx-color-mapper-vfx-color-mapper-x64: VFX Color Mapper (effects, DLL VFX Color Mapper_x64.dll (3918848 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vfx-color-mapper-vfx-color-mapper-x64',
  covers: ['plugins/effects/vfx-color-mapper-vfx-color-mapper-x64', 'plugins/window/effects-vfx-color-mapper-vfx-color-mapper-x64', 'plugins/param/effects-vfx-color-mapper-vfx-color-mapper-x64', 'plugins/audio/effects-vfx-color-mapper-vfx-color-mapper-x64', 'plugins/stop/effects-vfx-color-mapper-vfx-color-mapper-x64'],
  expectFail: '#1458',
  async run({ guest }) {
    return pluginSteps(guest, { name: "VFX Color Mapper", kind: 'effects' });
  },
});
