// file/import-midi: File > Import > MIDI file (the first item of the Import sub-menu): the engine's file box shows; a MIDI file is given to it (a minimal standard MIDI file that
// the scenario writes into .tmp-e2e-import, and removes after); the box closes and the engine stays idle.
// expectFail #1527: in this build File > #8 is not the Import sub-menu (the drop-down's item #8 opens "Enter a template name", the Save as template box), so no item reaches
// the file box. The File drop-down's item numbers are #1527's (the same cause as the drop-downs shown 0 of file/import-submenu and the other sub-menus).
import fs from 'node:fs';
import path from 'node:path';
import { defineScenario } from '../../lib/scenario.mjs';
import { closeNew, menuPath, openLids } from '../../lib/common.mjs';
import { dismissProblems } from '../../lib/flow.mjs';
import { giveFiles, waitBox } from '../../lib/file.mjs';

// a format 0 file with one track that holds only the end of track: MThd (6 bytes: format 0, 1 track, 96 ticks), MTrk (4 bytes: 00 FF 2F 00)
const MIDI = Buffer.from([0x4d, 0x54, 0x68, 0x64, 0, 0, 0, 6, 0, 0, 0, 1, 0, 96, 0x4d, 0x54, 0x72, 0x6b, 0, 0, 0, 4, 0, 0xff, 0x2f, 0]);

export default defineScenario({
  id: 'file/import-midi',
  covers: ['file/import/midi'],
  expectFail: '#1527',
  async run(ctx) {
    const { guest, cdp, repo } = ctx;
    const dir = path.join(repo, '.tmp-e2e-import');
    fs.mkdirSync(dir, { recursive: true });
    const mid = path.join(dir, 'import-test.mid');
    fs.writeFileSync(mid, MIDI);
    try {
      await dismissProblems(guest);
      const base = await openLids(guest);
      const id = await menuPath(guest, ['File', '#8', '#0']);
      const box = await waitBox(guest, 30000);
      const diag = box === 'explorer' ? '' : JSON.stringify(await guest.ev("(() => { const a = window.__app; const r = a.fileDlg.requests; return { requests: r.length, last: r.length ? { save: r[r.length - 1].save, options: r[r.length - 1].request.options, filters: r[r.length - 1].request.filters.length } : null, history: a.fileDlg.history.slice(-3), open: a.fileDlg.open }; })()").catch((e) => String(e?.message ?? e)));
      const g = box === 'explorer' ? await giveFiles(guest, cdp, [mid]) : { given: false, gone: null };
      await closeNew(guest, base);
      const st = await guest.state();
      return {
        ok: box === 'explorer' && g.given === true && g.gone === true && st.state === 'engine-idle',
        engineState: st.state,
        notes: [`${id}: box ${box}; the ${MIDI.length}-byte .mid given ${g.given}; the box closed ${g.gone}; state ${st.state}${diag ? `; page ${diag}` : ''}`],
      };
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  },
});
