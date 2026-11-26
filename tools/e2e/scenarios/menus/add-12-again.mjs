// menus/add-12-again: Add > #12: the same choice twice (the window closed by Escape between them): the same kind of effect both times.
import { defineScenario } from '../../lib/scenario.mjs';
import { chooseTwice, withNote } from '../../lib/menus.mjs';

export default defineScenario({
  id: 'menus/add-12-again',
  covers: ['menus/item/add-12'],
  displays: ['4k-dpr1'],
  expectFail: null,
  async run(ctx) {
    return withNote(await chooseTwice(ctx, ["Add","#12"]), "displays: 4k-dpr1 (one display for the item set: a scenario takes about 30 s, so 400 and more fit the shards' budget)");
  },
});
