// menus/file-8-1-again: File > #8 > #1: the same choice twice (the window closed by Escape between them): the same kind of effect both times.
import { defineScenario } from '../../lib/scenario.mjs';
import { chooseTwice, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/file-8-1-again',
  covers: ['menus/item/file-8-1'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await chooseTwice(ctx, ["File","#8","#1"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
