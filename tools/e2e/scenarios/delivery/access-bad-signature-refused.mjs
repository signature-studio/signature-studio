// delivery/access-bad-signature-refused: a token whose payload was changed after it was signed is refused (403): the Worker checks the RS256 signature, not only the shape.
import { defineScenario } from '../../lib/scenario.mjs';
import { live, tamperedToken, wireFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/access-bad-signature-refused',
  covers: ['delivery/access/bad-signature-refused'],
  displays: ['4k-dpr1'],
  async run() {
    const token = tamperedToken(live().newToken());
    const r = await wireFetch('/assets/manifest.json', { token });
    const ok = r.status === 403;
    return { ok, notes: [`a token with a changed payload: ${r.status}`], results: { status: r.status } };
  },
});
