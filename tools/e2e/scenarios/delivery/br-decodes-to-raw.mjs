// delivery/br-decodes-to-raw: every stored brotli object, read through the Worker, is the file on disk (its SHA-256 and size), and decoded it is the bytes the manifest names
// (the SHA-256 and the size of the raw file). So the compression is lossless for every object of the bundle.
import fs from 'node:fs';
import { defineScenario } from '../../lib/scenario.mjs';
import { brFileOf, brotliDecoded, bundle, sha256Hex, storedBytesOf } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/br-decodes-to-raw',
  covers: ['delivery/compression/br-decodes-to-raw', 'delivery/compression/body-is-stored-bytes'],
  displays: ['4k-dpr1'],
  async run() {
    const refs = bundle().all;
    const bad = [];
    let bytes = 0;
    for (const { ref, name } of refs) {
      const s = await storedBytesOf(ref);
      const disk = fs.readFileSync(brFileOf(ref));
      bytes += ref.size;
      const problems = [];
      if (s.status !== 200) problems.push(`status ${s.status}`);
      if (sha256Hex(s.buf) !== sha256Hex(disk)) problems.push('the body is not the stored file');
      const raw = s.status === 200 ? brotliDecoded(s.buf) : { sha256: null, size: -1 };
      if (raw.sha256 !== ref.sha256) problems.push(`decoded sha256 ${raw.sha256} (manifest ${ref.sha256})`);
      if (raw.size !== ref.size) problems.push(`decoded ${raw.size} B (manifest ${ref.size})`);
      if (problems.length) bad.push({ name, problems });
    }
    return { ok: bad.length === 0, notes: [`${refs.length} objects (${bytes} decoded bytes): the stored body is the file, and decoded it has the manifest's SHA-256 and size on ${refs.length - bad.length}`], results: { checked: refs.length, decodedBytes: bytes, bad: bad.slice(0, 10), badCount: bad.length } };
  },
});
