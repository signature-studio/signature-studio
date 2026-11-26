// delivery/document-security-headers: the document is served with its security headers (the Content-Security-Policy is present).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'delivery/document-security-headers',
  covers: [],
  async run({ page, origin }) {
    const r = await page.evaluate(async (o) => {
      const res = await fetch(`${o}/`);
      await res.text();
      return { status: res.status, csp: res.headers.get('content-security-policy') };
    }, origin);
    return { ok: r.status === 200 && !!r.csp, notes: [`status ${r.status}; CSP ${r.csp ? 'present' : 'absent'}`] };
  },
});
