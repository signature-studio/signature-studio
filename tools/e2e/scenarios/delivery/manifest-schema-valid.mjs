// delivery/manifest-schema-valid: the bundle's manifest passes the schema of app/src/bundle/manifest.ts (the page refuses any other, so the delivered one must pass).
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle, validateManifest } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/manifest-schema-valid',
  covers: ['delivery/manifest/schema-valid'],
  displays: ['4k-dpr1'],
  async run() {
    const r = validateManifest(bundle().manifest);
    const errors = r.ok ? [] : r.errors.slice(0, 5).map((e) => `${e.path ?? ''} ${e.message ?? ''}`.trim());
    return { ok: r.ok === true, notes: [r.ok ? 'the manifest passes the schema' : `schema errors: ${errors.join('; ')}`], results: { ok: r.ok, errors } };
  },
});
