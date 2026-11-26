// delivery/beacon-refuses-oversize: a beacon larger than the server's limit (PERF_MAX_BODY bytes) is refused 413 before it is read.
import { defineScenario } from '../../lib/scenario.mjs';
import { PERF_MAX_BODY, PERF_PATH, wireFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/beacon-refuses-oversize',
  covers: ['delivery/beacon/refuses-oversize'],
  displays: ['4k-dpr1'],
  async run() {
    const body = `{"x":"${'a'.repeat(PERF_MAX_BODY + 16)}"}`;
    const r = await wireFetch(PERF_PATH, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body });
    const ok = r.status === 413;
    return { ok, notes: [`a beacon of ${body.length} bytes (the limit ${PERF_MAX_BODY}): ${r.status}`], results: { status: r.status, bytes: body.length, limit: PERF_MAX_BODY } };
  },
});
