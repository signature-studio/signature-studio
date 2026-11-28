// windows/main-form-present: the main form (TFruityLoopsMainForm) is a top-level window of the table after the start, with the Channel rack, the Playlist and the Mixer inside it.
import { defineScenario } from '../../lib/scenario.mjs';
import { MAIN_FORM, windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'windows/main-form-present',
  covers: [],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    const t = await windowTable(guest);
    const main = t.top.find((w) => w.cls === MAIN_FORM);
    const inside = ['TStepSeqForm', 'TEventEditForm', 'TFXForm'].filter((c) => t.children.some((x) => x.cls === c && x.vis));
    return { ok: !!main && inside.length >= 1, engineState: (await guest.state())?.state ?? null, notes: [`main form ${main ? `${main.w}x${main.h}` : 'missing'}; shown inside: ${inside.join(', ')}`] };
  },
});
