// Serves $FL_ASSETS_DIR at /assets/ for the Vite dev server and `vite preview` (the large, product-derived assets are not in git).
// Same behaviour as worker/index.ts serves from R2: single byte Range (206 / 416), ETag + If-None-Match (304), If-Range, HEAD, Content-Type application/wasm,
// pre-compressed variants (NAME.br / NAME.gz next to NAME are sent with Content-Encoding when the client accepts it), COOP / COEP / CORP / X-Robots-Tag on every response.
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { cacheControlFor, mimeFor, ISOLATION_HEADERS, PRIVACY_HEADERS } from '../shared/headers.ts';
import { ifNoneMatchHit, ifRangeAllows, parseRange } from '../shared/range.ts';
import { MAX_MANIFEST_BYTES, brokenManifestBody, isDefaultManifestKey, judgeManifestBytes, notDeployedBody, type Verdict } from '../shared/manifest-guard.ts';

type Next = (err?: unknown) => void;
const one = (v: string | string[] | undefined): string | undefined => (Array.isArray(v) ? v[0] : v);

function send(res: ServerResponse, status: number, body: string, extra: Record<string, string> = {}): void {
  res.writeHead(status, { ...ISOLATION_HEADERS, ...PRIVACY_HEADERS, 'Content-Type': 'text/plain; charset=utf-8', 'Content-Length': Buffer.byteLength(body), 'Cache-Control': 'no-store', ...extra });
  res.end(body);
}

function accepts(header: string | undefined, coding: string): boolean {
  if (!header) return false;
  return header.split(',').some((t) => {
    const [name, ...q] = t.trim().split(';');
    if ((name ?? '').trim().toLowerCase() !== coding) return false;
    const qv = q.map((s) => s.trim()).find((s) => s.startsWith('q='));
    return qv === undefined || Number(qv.slice(2)) > 0;
  });
}

function resolveInside(root: string, urlPath: string): string | null {
  let rel: string;
  try { rel = decodeURIComponent(urlPath); } catch { return null; }
  if (rel.includes('\0') || rel.includes('\\')) return null;
  const full = path.resolve(root, '.' + path.posix.normalize('/' + rel));
  return full === root || full.startsWith(root + path.sep) ? full : null;
}

const NO_DIR = 'FL_ASSETS_DIR is not set: no assets directory is served at /assets/';

function safeDecode(rel: string): string | null {
  try { return decodeURIComponent(rel); } catch { return null; }
}

// The default manifest (decision 0152) is read whole, judged, and then served from the very bytes that were judged: a file swapped in between cannot get past the check.
function inspectDefaultManifest(file: string, coding: string | null, size: number): { ok: true; stored: Buffer } | Extract<Verdict, { ok: false }> {
  const fail = (what: string): Extract<Verdict, { ok: false }> => ({ ok: false, status: 500, body: brokenManifestBody(what) });
  if (size > MAX_MANIFEST_BYTES) return fail(`${size} バイトは大きすぎて検査できません`);
  let stored: Buffer;
  try { stored = fs.readFileSync(file); } catch (e) { return fail(`読めません: ${String((e as Error)?.message ?? e)}`); }
  if (stored.length !== size) return fail('読んでいる間に変わりました');
  let plain: Buffer = stored;
  try {
    if (coding === 'br') plain = zlib.brotliDecompressSync(stored, { maxOutputLength: MAX_MANIFEST_BYTES });
    else if (coding === 'gzip') plain = zlib.gunzipSync(stored, { maxOutputLength: MAX_MANIFEST_BYTES });
  } catch (e) { return fail(`${coding} として展開できません: ${String((e as Error)?.message ?? e)}`); }
  const v = judgeManifestBytes(plain);
  return v.ok ? { ok: true, stored } : v;
}

export function assetsMiddleware(rootDir: string, prefix = '/assets/') {
  const root = rootDir ? path.resolve(rootDir) : '';
  return (req: IncomingMessage, res: ServerResponse, next: Next): void => {
    const url = new URL(req.url ?? '/', 'http://x');
    if (!url.pathname.startsWith(prefix)) { next(); return; }
    if (req.method !== 'GET' && req.method !== 'HEAD') { send(res, 405, 'method not allowed', { Allow: 'GET, HEAD' }); return; }
    const rel = url.pathname.slice(prefix.length);
    // the default manifest, by its logical key (decoded, dot segments and `//` resolved, case folded): see shared/manifest-guard.ts
    const decoded = safeDecode(rel);
    const guarded = decoded !== null && isDefaultManifestKey(decoded);
    if (!root) { send(res, 404, guarded ? notDeployedBody(NO_DIR + '\n') : NO_DIR); return; }
    const file = resolveInside(root, rel);
    if (!file) { send(res, 400, 'bad path'); return; }
    const ae = req.headers['accept-encoding'];
    const candidates: { file: string; coding: string | null }[] = [];
    if (accepts(ae, 'br')) candidates.push({ file: file + '.br', coding: 'br' });
    if (accepts(ae, 'gzip')) candidates.push({ file: file + '.gz', coding: 'gzip' });
    candidates.push({ file, coding: null });
    // an explicit request for NAME.br / NAME.gz is a request for the stored bytes, without Content-Encoding
    const explicit = /\.(br|gz)$/i.test(file);
    let chosen: { file: string; coding: string | null; st: fs.Stats } | null = null;
    for (const c of explicit ? [{ file, coding: null }] : candidates) {
      try { const st = fs.statSync(c.file); if (st.isFile()) { chosen = { ...c, st }; break; } } catch { /* try the next */ }
    }
    if (!chosen) { send(res, 404, guarded ? notDeployedBody() : `not found: ${url.pathname}`); return; }
    const { st, coding } = chosen;
    let judged: Buffer | null = null;
    if (guarded) {
      const v = inspectDefaultManifest(chosen.file, coding, st.size);
      if (!v.ok) { send(res, v.status, v.body); return; }
      judged = v.stored;
    }
    const etag = `"${st.size.toString(16)}-${Math.floor(st.mtimeMs).toString(16)}${coding ? '-' + coding : ''}"`;
    const type = mimeFor(file);
    const h: Record<string, string> = {
      ...ISOLATION_HEADERS, ...PRIVACY_HEADERS,
      'Content-Type': type, 'Accept-Ranges': 'bytes', ETag: etag, 'Cache-Control': cacheControlFor(url.pathname), Vary: 'Accept-Encoding',
    };
    if (coding) h['Content-Encoding'] = coding;
    if (ifNoneMatchHit(one(req.headers['if-none-match']), etag)) { res.writeHead(304, h); res.end(); return; }
    const rangeHeader = ifRangeAllows(one(req.headers['if-range']), etag) ? one(req.headers.range) : undefined;
    const r = parseRange(rangeHeader, st.size);
    if (r.kind === 'unsatisfiable') { res.writeHead(416, { ...h, 'Content-Range': `bytes */${st.size}`, 'Content-Length': '0' }); res.end(); return; }
    let start = 0, end = st.size - 1, status = 200;
    if (r.kind === 'partial') { start = r.start; end = r.end; status = 206; h['Content-Range'] = `bytes ${start}-${end}/${st.size}`; }
    h['Content-Length'] = String(st.size === 0 ? 0 : end - start + 1);
    res.writeHead(status, h);
    if (req.method === 'HEAD' || st.size === 0) { res.end(); return; }
    if (judged) { res.end(judged.subarray(start, end + 1)); return; }
    const stream = fs.createReadStream(chosen.file, { start, end });
    stream.on('error', () => res.destroy());
    res.on('close', () => stream.destroy());
    stream.pipe(res);
  };
}
