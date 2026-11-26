// delivery/url-names-version: the bundle's version is the first 12 hex digits of the engine's SHA-256, and the engine's URL names that version. A new engine is a new version,
// and so a new URL (the immutable rule above is then safe); the bundle's manifest says the same.
import { defineScenario } from '../../lib/scenario.mjs';
import { bundle } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/url-names-version',
  covers: ['delivery/cache/url-names-version'],
  displays: ['4k-dpr1'],
  async run() {
    const m = bundle().manifest;
    const want = m.engine.wasm.sha256.slice(0, 12);
    const versionOk = m.bundle.version === want;
    const urlOk = m.engine.wasm.url.includes(`.${want}.`);
    return { ok: versionOk && urlOk, notes: [`bundle.version ${m.bundle.version}; the engine's SHA-256 starts ${want}; the engine URL names it: ${urlOk}`], results: { version: m.bundle.version, want, versionOk, urlOk } };
  },
});
