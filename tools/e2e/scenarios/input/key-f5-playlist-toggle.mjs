// input/key-f5-playlist-toggle: F5 changes the desk's picture (the Playlist shows or hides), and a second press brings the picture back.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f5-playlist-toggle',
  covers: ['input/key/f5'],
  run: (ctx) => I.fkeyToggle(ctx, 5, 'toggle'),
});
