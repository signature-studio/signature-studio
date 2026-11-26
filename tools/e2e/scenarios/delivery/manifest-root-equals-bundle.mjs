// delivery/manifest-root-equals-bundle: the two manifests the deploy writes (/assets/manifest.json and /assets/fl-Oz/manifest.json) are the bundle's own manifest, byte for byte.
import fs from 'node:fs';
import { defineScenario } from '../../lib/scenario.mjs';
import { rawManifestPath, sha256Hex, workerFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/manifest-root-equals-bundle',
  covers: ['delivery/manifest/root-equals-bundle'],
  displays: ['4k-dpr1'],
  async run() {
    const want = sha256Hex(fs.readFileSync(rawManifestPath()));
    const rows = [];
    for (const p of ['/assets/manifest.json', '/assets/fl-Oz/manifest.json']) {
      const res = await workerFetch(p);
      const buf = Buffer.from(await res.arrayBuffer());
      rows.push({ path: p, status: res.status, same: sha256Hex(buf) === want });
    }
    const ok = rows.every((r) => r.status === 200 && r.same);
    return { ok, notes: rows.map((r) => `${r.path}: ${r.status}, ${r.same ? 'the bundle\'s manifest' : 'differs'}`), results: rows };
  },
});
