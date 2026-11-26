// delivery/network-offline-mid-start: the network is cut while the start-up is downloading: the requests in flight fail and the page shows its error panel (#error:
// what went wrong, and where). The page does not stop without a message: no uncaught error of the page. A page of its own (the failures are the cut's, counted in results).
import { defineScenario } from '../../lib/scenario.mjs';
import { ownPage, waitPage, waitState } from '../../lib/delivery-tests.mjs';

const OFFLINE = { offline: true, latency: 0, downloadThroughput: -1, uploadThroughput: -1 };

export default defineScenario({
  id: 'delivery/network-offline-mid-start',
  covers: ['delivery/network/offline-mid-start', 'delivery/error/panel-shown', 'delivery/error/no-uncaught'],
  displays: ['4k-dpr1'],
  async run({ page, origin }) {
    const fp = await ownPage(page, { viewport: page.viewportSize() ?? undefined });
    try {
      await fp.cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: 2 * 1024 * 1024, uploadThroughput: 1024 * 1024 });
      await fp.page.goto(`${origin}/`, { waitUntil: 'load', timeout: 180000 });
      const started = await waitPage(fp.page, () => !!window.__app && window.__app.state === 'loading' && document.querySelectorAll('#progress li').length >= 1, null, 120000);
      await fp.cdp.send('Network.emulateNetworkConditions', OFFLINE);
      const state = await waitState(fp.page, ['error', 'engine-idle', 'exited'], 180000);
      const shown = await fp.page.evaluate(() => { const e = document.getElementById('error'); return e && !e.hidden ? e.textContent : null; });
      const hasWhere = shown !== null && /where:/.test(shown);
      const ok = !!started && state === 'error' && hasWhere && fp.counts.pageErrors === 0;
      return { ok, notes: [`downloads started ${!!started}; state ${state}; the error panel ${shown === null ? 'hidden' : 'shown, with its where'}; uncaught errors ${fp.counts.pageErrors}`], results: { started: !!started, state, panel: shown === null ? null : shown.slice(0, 300), ...fp.counts } };
    } finally {
      await fp.close();
    }
  },
});
