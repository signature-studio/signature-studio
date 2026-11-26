// plugins/builtin-effects-control-surface-control-surface-x64: Control Surface (effects, DLL Control Surface_x64.dll (5689344 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-control-surface-control-surface-x64',
  covers: ['plugins/effects/control-surface-control-surface-x64', 'plugins/window/effects-control-surface-control-surface-x64', 'plugins/param/effects-control-surface-control-surface-x64', 'plugins/audio/effects-control-surface-control-surface-x64', 'plugins/stop/effects-control-surface-control-surface-x64'],
  expectFail: '#1434',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Control Surface", kind: 'effects' });
  },
});
