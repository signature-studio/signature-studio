// windows/playlist-t1-b04: a click on the Playlist grid (track 1, bar 4) with the default tool: the grid's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { playlistCell } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/playlist-t1-b04',
  covers: ['windows/playlist/clip-place'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return playlistCell(guest, 0, 3);
  },
});
