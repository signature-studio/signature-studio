// POST /__perf for the Vite dev server and `vite preview` (decision 0152): the page's measurement beacon (src/perf/) is appended, one JSON object per line, to a JSONL file.
//   path of the file: FL_PERF_LOG, else perf-log.jsonl next to FL_ASSETS_DIR (its parent directory); with neither the endpoint answers 503 and stores nothing.
//   accepted: POST, same origin (Origin / Sec-Fetch-Site), Content-Type application/json, at most 16 KB (413 / 415 / 403 / 405 otherwise), one JSON object (400 otherwise).
//   stored: {"t": ISO time, "remote": "loopback" | "lan" | "tailnet" | "other", "perf": <the beacon>}. The peer's address is NOT stored, only its class (decision 0192: behind `tailscale serve` the socket peer is 127.0.0.1, so a loopback peer is classified by the first address of X-Forwarded-For). Answer 204 + X-Perf-Stored: yes.
import fs from 'node:fs';
import path from 'node:path';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { ISOLATION_HEADERS, PRIVACY_HEADERS } from '../shared/headers.ts';
import { PERF_LOG_MAX_BYTES, PERF_MAX_BODY, PERF_PATH, PERF_STORED_HEADER, checkPerfRequest, classifyPeer, parsePerfBody, perfLine } from '../shared/perf.ts';

type Next = (err?: unknown) => void;
const one = (v: string | string[] | undefined): string | undefined => (Array.isArray(v) ? v[0] : v);

/** where the log goes: FL_PERF_LOG, else perf-log.jsonl in the parent directory of FL_ASSETS_DIR, else '' (not configured) */
export function perfLogPath(env: Record<string, string | undefined> = process.env): string {
  if (env.FL_PERF_LOG) return path.resolve(env.FL_PERF_LOG);
  if (env.FL_ASSETS_DIR) return path.join(path.dirname(path.resolve(env.FL_ASSETS_DIR)), 'perf-log.jsonl');
  return '';
}

function reply(res: ServerResponse, status: number, body: string, stored: boolean, extra: Record<string, string> = {}, done?: () => void): void {
  const h: Record<string, string> = { ...ISOLATION_HEADERS, ...PRIVACY_HEADERS, 'Cache-Control': 'no-store', [PERF_STORED_HEADER]: stored ? 'yes' : 'no', ...extra };
  if (status !== 204) { h['Content-Type'] = 'text/plain; charset=utf-8'; h['Content-Length'] = String(Buffer.byteLength(body)); }
  res.writeHead(status, h);
  res.end(status === 204 ? undefined : body, done);
}

export function perfMiddleware(logPath: string, now: () => Date = () => new Date()) {
  return (req: IncomingMessage, res: ServerResponse, next: Next): void => {
    const url = new URL(req.url ?? '/', 'http://x');
    if (url.pathname !== PERF_PATH) { next(); return; }
    const c = checkPerfRequest({
      method: req.method ?? '', contentType: one(req.headers['content-type']), contentLength: one(req.headers['content-length']),
      origin: one(req.headers.origin), host: one(req.headers.host), secFetchSite: one(req.headers['sec-fetch-site']),
    });
    const abort = (status: number, body: string, extra: Record<string, string> = {}): void => reply(res, status, body, false, { Connection: 'close', ...extra }, () => req.destroy());
    if (!c.ok) {
      if (c.status === 413) abort(c.status, c.body);
      else { req.resume(); reply(res, c.status, c.body, false, c.allow ? { Allow: c.allow } : {}); }
      return;
    }
    if (!logPath) { req.resume(); reply(res, 503, 'the beacon is not stored: set FL_PERF_LOG (or FL_ASSETS_DIR) for this server\n', false); return; }
    const chunks: Buffer[] = [];
    let size = 0;
    let over = false;
    req.on('data', (d: Buffer) => {
      if (over) return;
      size += d.length;
      if (size > PERF_MAX_BODY) { over = true; abort(413, `the beacon is at most ${PERF_MAX_BODY} bytes\n`); return; }
      chunks.push(d);
    });
    req.on('error', () => { /* the client went away */ });
    req.on('end', () => {
      if (over) return;
      const body = parsePerfBody(Buffer.concat(chunks));
      if (!body.ok) { reply(res, body.status, body.body, false); return; }
      try {
        fs.mkdirSync(path.dirname(logPath), { recursive: true });
        let have = 0;
        try { have = fs.statSync(logPath).size; } catch { /* no log yet */ }
        if (have > PERF_LOG_MAX_BYTES) { reply(res, 507, `the perf log is full (${have} bytes): move it away\n`, false); return; }
        fs.appendFileSync(logPath, perfLine(body.value, now(), classifyPeer(req.socket.remoteAddress, one(req.headers['x-forwarded-for']))));
      } catch (e) { reply(res, 500, `the beacon could not be stored: ${String((e as Error)?.message ?? e)}\n`, false); return; }
      reply(res, 204, '', true);
    });
  };
}
