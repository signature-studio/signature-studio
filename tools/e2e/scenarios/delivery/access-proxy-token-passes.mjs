// delivery/access-proxy-token-passes: the page reaches the bundle through the Access in front of the Worker (the proxy of lib/delivery.mjs adds the token the page does not carry).
import { defineScenario } from '../../lib/scenario.mjs';
import { pageFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/access-proxy-token-passes',
  covers: ['delivery/access/proxy-token-passes'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const r = await pageFetch(page, '/assets/manifest.json');
    const ok = r.status === 200;
    return { ok, notes: [`the page's manifest request through Access: ${r.status}`], results: { status: r.status } };
  },
});
