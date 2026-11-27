// delivery/network-slow-start: a start on a slow network (400 ms of latency per request, 4 MiB/s) completes (engine-idle), and the start-up shows its downloads while it waits
// (the progress rows, in the start-up modal that leaves the page before the engine starts, are seen with a row per start-up asset: the largest count while it waits; Issue #1520). The page is a page of its own (its own context: its failures are its own; no detector of the run).
import { defineScenario } from '../../lib/scenario.mjs';
import { ownPage, waitState } from '../../lib/delivery-tests.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'delivery/network-slow-start',
  covers: ['delivery/network/slow-start-completes'],
  displays: ['4k-dpr1'],
  async run({ page, origin }) {
    const fp = await ownPage(page, { viewport: page.viewportSize() ?? undefined });
    try {
      await fp.cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 400, downloadThroughput: 4 * 1024 * 1024, uploadThroughput: 1024 * 1024 });
      const t0 = Date.now();
      await fp.page.goto(`${origin}/`, { waitUntil: 'load', timeout: 180000 });
      let rows = 0, polling = true;
      const poll = (async () => {
        while (polling) {
          rows = Math.max(rows, await fp.page.evaluate(() => document.querySelectorAll('#progress li').length).catch(() => 0));
          await sleep(200);
        }
      })();
      const state = await waitState(fp.page, ['engine-idle', 'error', 'exited'], 300000).finally(() => { polling = false; });
      await poll;
      const secs = Math.round((Date.now() - t0) / 1000);
      const ok = state === 'engine-idle' && rows >= 4 && fp.counts.pageErrors === 0;
      return { ok, notes: [`state ${state} after ${secs} s on the slow network; progress rows ${rows}; uncaught errors ${fp.counts.pageErrors}`], results: { state, seconds: secs, progressRows: rows, ...fp.counts } };
    } finally {
      await fp.close();
    }
  },
});
