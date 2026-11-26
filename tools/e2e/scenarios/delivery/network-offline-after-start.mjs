// delivery/network-offline-after-start: the network is cut while the lazy files are still downloading (after the engine has started): the engine does not stop (no error,
// no exit) for 10 s after the cut; the downloads that were cut fail where they are (their count is in results). A page of its own.
import { defineScenario } from '../../lib/scenario.mjs';
import { ownPage, sleep, waitPage } from '../../lib/delivery-tests.mjs';

const OFFLINE = { offline: true, latency: 0, downloadThroughput: -1, uploadThroughput: -1 };

export default defineScenario({
  id: 'delivery/network-offline-after-start',
  covers: ['delivery/network/offline-after-start'],
  displays: ['4k-dpr1'],
  async run({ page, origin }) {
    const fp = await ownPage(page, { viewport: page.viewportSize() ?? undefined });
    try {
      await fp.cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: 2 * 1024 * 1024, uploadThroughput: 1024 * 1024 });
      await fp.page.goto(`${origin}/`, { waitUntil: 'load', timeout: 180000 });
      const inLazy = await waitPage(fp.page, () => !!window.__app && !!window.__app.timeline.lazy_start && !window.__app.timeline.lazy_done, null, 300000);
      await fp.cdp.send('Network.emulateNetworkConditions', OFFLINE);
      await sleep(10000);
      const after = await fp.page.evaluate(() => ({ state: window.__app.state, error: window.__app.error, lazy: window.__app.lazy.length, lazyFailed: window.__app.lazy.filter((r) => !r.ok).length })).catch(() => null);
      const ok = !!inLazy && after !== null && after.state !== 'error' && after.state !== 'exited' && after.error === null && after.lazyFailed > 0 && fp.counts.pageErrors === 0;
      return { ok, notes: [`cut during the lazy downloads: ${!!inLazy}; 10 s after the cut the engine is ${after?.state}; lazy files that failed: ${after?.lazyFailed} of ${after?.lazy} done; uncaught errors ${fp.counts.pageErrors}`], results: { inLazy: !!inLazy, after: after?.state ?? null, error: after?.error ?? null, lazyFailed: after?.lazyFailed ?? null, lazyDone: after?.lazy ?? null, ...fp.counts } };
    } finally {
      await fp.close();
    }
  },
});
