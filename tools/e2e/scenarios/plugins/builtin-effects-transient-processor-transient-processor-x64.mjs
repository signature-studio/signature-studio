// plugins/builtin-effects-transient-processor-transient-processor-x64: Transient Processor (effects, DLL Transient Processor_x64.dll (3971584 B)) - a generated scenario (tools/e2e/scenarios/plugins/_generate.mjs from features/generated/plugins.tsv and pm-work/created-issues.tsv). Do not edit by hand.
import { defineScenario } from '../../lib/scenario.mjs';
import { pluginSteps } from '../../lib/plugins.mjs';

export default defineScenario({
  id: 'plugins/builtin-effects-transient-processor-transient-processor-x64',
  covers: ['plugins/effects/transient-processor-transient-processor-x64', 'plugins/window/effects-transient-processor-transient-processor-x64', 'plugins/param/effects-transient-processor-transient-processor-x64', 'plugins/audio/effects-transient-processor-transient-processor-x64', 'plugins/stop/effects-transient-processor-transient-processor-x64'],
  expectFail: '#1457',
  async run({ guest }) {
    return pluginSteps(guest, { name: "Transient Processor", kind: 'effects' });
  },
});
