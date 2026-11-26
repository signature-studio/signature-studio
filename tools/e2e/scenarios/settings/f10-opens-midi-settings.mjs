// settings/f10-opens-midi-settings: F10 opens the settings window; its MIDI page is a top-level window (TMIDIForm, the title "Settings - MIDI ..."). Escape closes it.
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, newWindows, openLids } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'settings/f10-opens-midi-settings',
  covers: ['settings/page/midi'],
  async run({ guest }) {
    const base = await openLids(guest);
    await guest.focusDesktop();
    await guest.key('F10', 'F10', 121);
    await sleep(1500);
    const { others } = await newWindows(guest, base);
    const w = others.find((x) => x.cls === 'TMIDIForm');
    const left = await closeNew(guest, base);
    return { ok: !!w && left.length === 0, engineState: (await guest.state())?.state ?? null, notes: [w ? `opened "${w.title}"` : 'F10 opened no TMIDIForm'] };
  },
});
