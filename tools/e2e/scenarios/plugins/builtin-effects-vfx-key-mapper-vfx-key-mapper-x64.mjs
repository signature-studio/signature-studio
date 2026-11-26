// plugins/builtin-effects-vfx-key-mapper-vfx-key-mapper-x64: VFX Key Mapper (effects, DLL VFX Key Mapper_x64.dll (4001280 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-vfx-key-mapper-vfx-key-mapper-x64',
  covers: ['plugins/effects/vfx-key-mapper-vfx-key-mapper-x64', 'plugins/window/effects-vfx-key-mapper-vfx-key-mapper-x64', 'plugins/param/effects-vfx-key-mapper-vfx-key-mapper-x64', 'plugins/audio/effects-vfx-key-mapper-vfx-key-mapper-x64', 'plugins/stop/effects-vfx-key-mapper-vfx-key-mapper-x64'],
  expectFail: '#1459',
  async run({ guest }) {
    return pluginSteps(guest, { name: "VFX Key Mapper", kind: 'effects' });
  },
});
