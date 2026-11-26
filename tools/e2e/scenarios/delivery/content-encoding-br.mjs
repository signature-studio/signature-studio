// delivery/content-encoding-br: the Worker serves each brotli object with Content-Encoding br, the Content-Length of the stored object (the manifest's encodedSize, which is the
// size of the file on disk), and no Content-Encoding for the manifest (its bytes are the JSON). Read through the Worker in this process (HEAD, every object).
import fs from 'node:fs';
import { defineScenario } from '../../lib/scenario.mjs';
import { brFileOf, bundle, workerFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/content-encoding-br',
  covers: ['delivery/compression/content-encoding-br', 'delivery/compression/encoded-size', 'delivery/compression/manifest-identity'],
  displays: ['4k-dpr1'],
  async run() {
    const bad = [];
    const refs = bundle().all.map((r) => r.ref);
    for (const ref of refs) {
      const res = await workerFetch(ref.url, { method: 'HEAD' });
      const h = Object.fromEntries([...res.headers]);
      const onDisk = fs.statSync(brFileOf(ref)).size;
      const problems = [];
      if (res.status !== 200) problems.push(`status ${res.status}`);
      if (h['content-encoding'] !== 'br') problems.push(`Content-Encoding ${h['content-encoding']}`);
      if (h['content-length'] !== String(ref.encodedSize)) problems.push(`Content-Length ${h['content-length']} (encodedSize ${ref.encodedSize})`);
      if (onDisk !== ref.encodedSize) problems.push(`file ${onDisk} B (encodedSize ${ref.encodedSize})`);
      if (problems.length) bad.push({ name: ref.url.slice(-48), problems });
    }
    const m = await workerFetch('/assets/manifest.json', { method: 'HEAD' });
    const mh = Object.fromEntries([...m.headers]);
    const manifestOk = m.status === 200 && mh['content-encoding'] === undefined;
    const ok = bad.length === 0 && manifestOk;
    return { ok, notes: [`${refs.length} brotli objects: br, the stored length and the file agree on ${refs.length - bad.length}; manifest without Content-Encoding: ${manifestOk}`], results: { checked: refs.length, bad: bad.slice(0, 10), badCount: bad.length, manifestContentEncoding: mh['content-encoding'] ?? null } };
  },
});
