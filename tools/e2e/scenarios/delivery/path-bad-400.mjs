// delivery/path-bad-400: a path that climbs out of the bundle (an encoded ".." segment) is refused 400 and nothing is read.
import { defineScenario } from '../../lib/scenario.mjs';
import { workerFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/path-bad-400',
  covers: ['delivery/path/bad-400'],
  displays: ['4k-dpr1'],
  async run() {
    const res = await workerFetch('/assets/..%2Fmanifest.json');
    const ok = res.status === 400;
    return { ok, notes: [`/assets/..%2Fmanifest.json: ${res.status}`], results: { status: res.status } };
  },
});
