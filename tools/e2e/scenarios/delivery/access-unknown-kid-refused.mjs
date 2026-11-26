// delivery/access-unknown-kid-refused: a token signed with a key the Access certificates do not name (another kid, another key) is refused (403).
import { defineScenario } from '../../lib/scenario.mjs';
import { foreignToken, live, wireFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/access-unknown-kid-refused',
  covers: ['delivery/access/unknown-kid-refused'],
  displays: ['4k-dpr1'],
  async run() {
    const d = live();
    const token = foreignToken(d.accessEnv.ACCESS_TEAM_DOMAIN, d.accessEnv.ACCESS_AUD);
    const r = await wireFetch('/assets/manifest.json', { token });
    const ok = r.status === 403;
    return { ok, notes: [`a token under a key the certificates do not name: ${r.status}`], results: { status: r.status } };
  },
});
