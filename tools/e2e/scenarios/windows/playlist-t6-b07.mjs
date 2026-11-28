// windows/playlist-t6-b07: a click on the Playlist grid (track 6, bar 7) with the default tool: the grid's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { playlistCell } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/playlist-t6-b07',
  covers: ['windows/playlist/clip-place'],
  displays: ['4k-dpr1', '4k-random', '4k-dpr2'],
  async run({ guest }) {
    return playlistCell(guest, 5, 6);
  },
});
