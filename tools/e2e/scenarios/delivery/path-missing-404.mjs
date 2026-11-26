// delivery/path-missing-404: a name the bucket does not hold is answered 404 (not a page, not another object).
import { defineScenario } from '../../lib/scenario.mjs';
import { workerFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/path-missing-404',
  covers: ['delivery/path/missing-404'],
  displays: ['4k-dpr1'],
  async run() {
    const res = await workerFetch('/assets/fl-Oz/no-such-object.0000000000ff.bin', { method: 'HEAD' });
    const ok = res.status === 404;
    return { ok, notes: [`a name the bucket does not hold: ${res.status}`], results: { status: res.status } };
  },
});
