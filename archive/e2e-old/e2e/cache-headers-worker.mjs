// Decision 0220, job 1(a): the response headers of the production Worker code (wrangler dev --local, local R2) for the five assets of a real bundle,
// stored the way the deploy stores them (upload-commands.txt: Content-Encoding br, Cache-Control immutable; the manifest no-cache).
// Usage: node test/e2e/cache-headers-worker.mjs <bundle dir with upload-commands.txt> <out.md> [--no-cache-control]
//   --no-cache-control   puts the objects WITHOUT an R2 Cache-Control (what the Worker does then: cacheControlFor(path))
import fs from 'node:fs';
import path from 'node:path';
import { rawRequest } from '../helpers/raw-http.mjs';
import { putObject, startWorker, workDirOf } from '../helpers/wrangler.mjs';

const [bundle, out] = process.argv.slice(2);
const noCc = process.argv.includes('--no-cache-control');
if (!bundle || !out) { console.error('usage: node test/e2e/cache-headers-worker.mjs <bundle dir> <out.md> [--no-cache-control]'); process.exit(64); }
const work = workDirOf('cache-headers' + (noCc ? '-nocc' : ''));
fs.rmSync(work, { recursive: true, force: true });
fs.mkdirSync(work, { recursive: true });

const lines = fs.readFileSync(path.join(bundle, 'upload-commands.txt'), 'utf8').split('\n').filter((l) => l.startsWith('wrangler r2 object put '));
const tok = (l) => [...l.matchAll(/'([^']*)'|(\S+)/g)].map((m) => m[1] ?? m[2]);
const items = lines.map((l) => {
  const t = tok(l);
  const opt = (f) => { const i = t.indexOf('--' + f); return i >= 0 ? t[i + 1] : undefined; };
  return { key: t[4].split('/').slice(1).join('/'), file: path.join(bundle, path.basename(opt('file'))), contentType: opt('content-type'), contentEncoding: opt('content-encoding'), cacheControl: opt('cache-control') };
});
for (const it of items) await putObject(work, it.key, it.file, { contentType: it.contentType, contentEncoding: it.contentEncoding, ...(noCc ? {} : { cacheControl: it.cacheControl }) });
const mf = items.find((i) => i.key.endsWith('manifest.json'));
await putObject(work, 'manifest.json', mf.file, { contentType: mf.contentType, ...(noCc ? {} : { cacheControl: mf.cacheControl }) });

const w = await startWorker(work, { ACCESS_REQUIRED: 'false' });
const rows = [];
try {
  const AE = { 'accept-encoding': 'gzip, deflate, br, zstd' };
  const all = [...items.map((i) => ({ key: i.key, stored: i })), { key: 'manifest.json', stored: { ...mf, key: 'manifest.json' } }];
  const first = {};
  for (const a of all) {
    const url = `${w.url}/assets/${a.key}`;
    const head = await rawRequest(url, { method: 'HEAD', headers: AE });
    const get = await rawRequest(url, { headers: AE });
    const etag = get.headers.etag;
    first[a.key] = etag;
    const inm = await rawRequest(url, { headers: { ...AE, 'if-none-match': etag } });
    const rng = await rawRequest(url, { headers: { ...AE, range: 'bytes=0-99' } });
    const stale = await rawRequest(url, { headers: { ...AE, 'if-none-match': '"0000"' }, readBody: false });
    const h = (r, n) => r.headers[n] ?? '-';
    rows.push({ key: a.key, storedCc: noCc ? '(none)' : a.stored.cacheControl ?? '(none)', status: get.status, cc: h(get, 'cache-control'), etag, ce: h(get, 'content-encoding'), vary: h(get, 'vary'), len: h(get, 'content-length'), body: get.bytes, ar: h(get, 'accept-ranges'), age: h(get, 'age'), ct: h(get, 'content-type'), headLen: h(head, 'content-length'), headStatus: head.status, inm: inm.status, inmBody: inm.bytes, inmCc: h(inm, 'cache-control'), inmEtag: h(inm, 'etag'), range: rng.status, rangeBody: rng.bytes, rangeCr: h(rng, 'content-range'), stale: stale.status, rawNames: get.rawHeaders.filter((_, i) => i % 2 === 0).join(', ') });
  }
} finally { await w.stop(); }

const md = [];
md.push(`# Worker response headers (wrangler dev --local, production Worker code, R2 objects ${noCc ? 'WITHOUT' : 'WITH'} a stored Cache-Control)\n`);
md.push('| asset | stored Cache-Control | GET | Cache-Control sent | ETag | Content-Encoding | Vary | Content-Length (body bytes) | Accept-Ranges | Age | Content-Type |');
md.push('|---|---|---:|---|---|---|---|---:|---|---|---|');
for (const r of rows) md.push(`| ${r.key.replace(/^fl-engine-[^/]+\//, '…/')} | ${r.storedCc} | ${r.status} | \`${r.cc}\` | ${r.etag} | ${r.ce} | ${r.vary} | ${r.len} (${r.body}) | ${r.ar} | ${r.age} | ${r.ct} |`);
md.push('\n| asset | HEAD (status, length) | If-None-Match same ETag | (body bytes, Cache-Control, ETag) | If-None-Match other | Range 0-99 | Content-Range |');
md.push('|---|---|---:|---|---:|---:|---|');
for (const r of rows) md.push(`| ${r.key.replace(/^fl-engine-[^/]+\//, '…/')} | ${r.headStatus}, ${r.headLen} | ${r.inm} | ${r.inmBody} B, \`${r.inmCc}\`, ${r.inmEtag} | ${r.stale} | ${r.range} (${r.rangeBody} B) | ${r.rangeCr} |`);
md.push('\nHeader names the Worker sets (raw, one GET): ' + rows[0].rawNames);
fs.writeFileSync(out, md.join('\n') + '\n');
console.log(md.join('\n'));
