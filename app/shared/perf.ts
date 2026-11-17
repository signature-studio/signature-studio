// The measurement beacon's side of the server (decision 0152, docs/milestones/performance-targets.md section 6): what POST /__perf accepts and what a stored record looks like.
// Used by tools/perf-middleware.ts (vite dev / vite preview: appends to a JSONL file) and worker/index.ts (the Cloudflare Worker: validates, stores ONE R2 object per beacon since decision 0224, answers 204).
// The page sends the beacon to its own origin only (CSP connect-src 'self'); nothing here talks to another host. No IP address is stored: only a class of the peer address.

export const PERF_PATH = '/__perf';
export const PERF_MAX_BODY = 16 * 1024;
/** the response header that tells the page whether the server kept the record ("yes" / "no") */
export const PERF_STORED_HEADER = 'X-Perf-Stored';
/** a log file bigger than this is not appended to any more (507): a beacon must not be a way to fill the disk */
export const PERF_LOG_MAX_BYTES = 64 * 1024 * 1024;

/** 'workers' (decision 0224): the record was stored by the Cloudflare Worker, behind Access; the peer's address class is not kept there (it would be Cloudflare's, and the address is personal data), this one word is all */
export type Remote = 'loopback' | 'lan' | 'tailnet' | 'other' | 'workers';

function ipv4(addr: string): number[] | null {
  const m = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/.exec(addr);
  if (!m) return null;
  const o = [m[1], m[2], m[3], m[4]].map(Number);
  return o.every((x) => x >= 0 && x <= 255) ? o : null;
}

/**
 * The class of the peer's socket address. Only the class is ever recorded, never the address.
 * loopback 127/8 and ::1 (this is also what a reverse proxy on this machine looks like, e.g. `tailscale serve`); tailnet 100.64/10 and fd7a:115c:a1e0::/48 (Tailscale);
 * lan 10/8, 172.16/12, 192.168/16, 169.254/16, fc00::/7, fe80::/10; anything else (or unreadable) is "other".
 */
export function classifyRemote(addr: string | undefined | null): Remote {
  if (!addr) return 'other';
  let a = addr.trim().toLowerCase();
  const mapped = /^::ffff:(\d{1,3}(?:\.\d{1,3}){3})$/.exec(a);
  if (mapped) a = mapped[1] as string;
  const v4 = ipv4(a);
  if (v4) {
    const [a0, a1] = v4 as [number, number, number, number];
    if (a0 === 127) return 'loopback';
    if (a0 === 100 && a1 >= 64 && a1 <= 127) return 'tailnet';
    if (a0 === 10 || (a0 === 172 && a1 >= 16 && a1 <= 31) || (a0 === 192 && a1 === 168) || (a0 === 169 && a1 === 254)) return 'lan';
    return 'other';
  }
  if (a === '::1') return 'loopback';
  if (!/^[0-9a-f:]+$/.test(a) || !a.includes(':')) return 'other';
  if (a.startsWith('fd7a:115c:a1e0:')) return 'tailnet';
  const first = a.split(':')[0] ?? '';
  if (/^f[cd][0-9a-f]{2}$/.test(first) || /^fe[89ab][0-9a-f]$/.test(first)) return 'lan';
  return 'other';
}

/**
 * The class of the peer of a beacon request (decision 0192). `tailscale serve` (what puts the page on the tailnet's HTTPS name) is a reverse proxy ON THIS MACHINE: the socket peer the server sees is 127.0.0.1 for every
 * visitor, and the visitor's tailnet address arrives in `X-Forwarded-For` (tailscale/ipn/ipnlocal/serve.go addProxyForwardedHeaders: the source address alone, no port). So when, and only when, the socket peer is a
 * loopback address, the first well-formed address of `X-Forwarded-For` decides the class; a header that is not an address (or is absent) leaves it `loopback`. Only the class is ever recorded, never the address.
 * A process on this machine can forge the header: harmless (it can also POST a beacon of any content; the log is a measurement log, not a security boundary).
 */
export function classifyPeer(socketAddr: string | undefined | null, forwardedFor?: string | undefined): Remote {
  const direct = classifyRemote(socketAddr);
  if (direct !== 'loopback' || !forwardedFor) return direct;
  const first = (forwardedFor.split(',')[0] ?? '').trim().toLowerCase();
  const a = /^::ffff:(\d{1,3}(?:\.\d{1,3}){3})$/.exec(first)?.[1] ?? first;
  const wellFormed = ipv4(a) !== null || (/^[0-9a-f:]+$/.test(a) && a.includes(':'));
  return wellFormed ? classifyRemote(a) : direct;
}

/** decision 0224: where the Worker keeps the beacons: bucket PERF_BUCKET (fl-web-cache), one object per beacon, key `perf/<UTC date>/<sid>-<seq>.json`; the object is one JSONL line (perfLine), so `cat` of a synced directory is a log */
export const PERF_PREFIX = 'perf/';
/** the summary route of the Worker (GET), behind the same Access check as everything else */
export const PERF_SUMMARY_PATH = '/__perf/summary';

/**
 * The key of a beacon's object. The page's session token (`sid`, 12 hex digits) and the beacon's `seq` make it; anything else (a version 1 beacon has no sid, a hand-made one may have any text) is NOT put into a key:
 * such a beacon gets `nosid-<spare>` (the caller's random 8 hex digits) and seq 0. The date is the UTC date of the time the Worker received it.
 */
export function perfObjectKey(value: Record<string, unknown>, now: Date, spare: string): string {
  const sid = typeof value.sid === 'string' && /^[0-9a-f]{12}$/.test(value.sid) ? value.sid : `nosid-${/^[0-9a-f]{8}$/.test(spare) ? spare : '00000000'}`;
  const seq = typeof value.seq === 'number' && Number.isInteger(value.seq) && value.seq >= 0 && value.seq <= 999_999_999 ? value.seq : 0;
  return `${PERF_PREFIX}${now.toISOString().slice(0, 10)}/${sid}-${seq}.json`;
}

export interface PerfRequestFacts {
  method: string;
  contentType?: string | undefined;
  contentLength?: string | undefined;
  origin?: string | undefined;
  host?: string | undefined;
  secFetchSite?: string | undefined;
}
export type PerfCheck = { ok: true } | { ok: false; status: 403 | 405 | 413 | 415; body: string; allow?: string };

/** the checks that need only the headers: method, same origin, JSON, announced size */
export function checkPerfRequest(f: PerfRequestFacts): PerfCheck {
  if (f.method !== 'POST') return { ok: false, status: 405, body: 'method not allowed: POST the beacon\n', allow: 'POST' };
  // Sec-Fetch-Site is set by the browser and cannot be set by a script: when it is there it decides (so a proxy that rewrites Host, as `tailscale serve` might, cannot make the page's own beacon look foreign);
  // without it (an old browser, a script) the Origin header, if any, must name this host
  if (f.secFetchSite !== undefined && f.secFetchSite !== 'same-origin' && f.secFetchSite !== 'none') return { ok: false, status: 403, body: 'the beacon is accepted from the same origin only (Sec-Fetch-Site)\n' };
  if (f.origin !== undefined && f.secFetchSite === undefined) {
    let host = '';
    try { host = new URL(f.origin).host; } catch { /* Origin: null and the like */ }
    if (host === '' || host !== f.host) return { ok: false, status: 403, body: 'the beacon is accepted from the same origin only (Origin)\n' };
  }
  const type = (f.contentType ?? '').split(';')[0]?.trim().toLowerCase();
  if (type !== 'application/json') return { ok: false, status: 415, body: 'unsupported media type: the beacon is application/json\n' };
  if (f.contentLength !== undefined) {
    const n = Number(f.contentLength);
    if (!Number.isFinite(n) || n < 0) return { ok: false, status: 413, body: 'bad Content-Length\n' };
    if (n > PERF_MAX_BODY) return { ok: false, status: 413, body: `the beacon is at most ${PERF_MAX_BODY} bytes\n` };
  }
  return { ok: true };
}

export type PerfBody = { ok: true; value: Record<string, unknown> } | { ok: false; status: 400 | 413; body: string };

/** the body: at most 16 KB, UTF-8, one JSON object (not an array, not a scalar) */
export function parsePerfBody(bytes: Uint8Array): PerfBody {
  if (bytes.length > PERF_MAX_BODY) return { ok: false, status: 413, body: `the beacon is at most ${PERF_MAX_BODY} bytes\n` };
  let json: unknown;
  try { json = JSON.parse(new TextDecoder('utf-8', { fatal: true, ignoreBOM: false }).decode(bytes)); } catch { return { ok: false, status: 400, body: 'the beacon is not valid UTF-8 JSON\n' }; }
  if (typeof json !== 'object' || json === null || Array.isArray(json)) return { ok: false, status: 400, body: 'the beacon is one JSON object\n' };
  return { ok: true, value: json as Record<string, unknown> };
}

/** one line of the JSONL log: the time, the class of the peer, the beacon. JSON.stringify never writes a raw newline, so one record is one line. */
export function perfLine(value: Record<string, unknown>, now: Date, remote: Remote): string {
  return JSON.stringify({ t: now.toISOString(), remote, perf: value }) + '\n';
}

// ---- the shape of a version 2 beacon's new blocks (decision 0192) and of a version 3 beacon's `input` block (decision 0202) ----
// The server does NOT reject a beacon for these (a refusal would stop the page from sending anything: the log is better with an odd record than without one); the tests, the summary script and a reader of the log use this to find
// a record whose numbers cannot be true. `PERF_RAF_BINS` = RAF_BIN_EDGES_MS.length + 1 (src/perf/display.ts).
export const PERF_RAF_BINS = 10;
/** decision 0209: the longest `engine.exit.message` (characters); src/perf/exit-message.ts EXIT_MESSAGE_MAX is the same number */
export const PERF_EXIT_MESSAGE_MAX = 400;
const HZ_FITS = ['match', 'engine-slower', 'display-slower', 'page-behind', 'unknown'];
/** decision 0216: the failure codes of the optional `files` block (src/files/usage.ts FailCode is this list); a code outside it is a problem, so a name cannot travel as a "code" */
export const PERF_FILES_FAIL_CODES = ['name', 'conflict', 'quota', 'locked', 'notfound', 'kind', 'zip-format', 'zip-crc', 'zip-method', 'zip-encrypted', 'zip-size', 'unsafe-path', 'cancelled', 'unsupported', 'security', 'io', 'other'] as const;

/** decision 0220: the fixed vocabulary of the optional `assets` block (src/bundle/assets-report.ts): which asset, where its bytes came from, what the stored copy did, why it did not; a word outside the lists is a problem, so a name or an error text cannot travel */
export const PERF_ASSET_KINDS = ['wasm', 'dll', 'seed', 'glue'] as const;
export const PERF_ASSET_ORIGINS = ['network', 'http-cache', 'storage-cache'] as const;
export const PERF_ASSET_STORE_STATES = ['off', 'hit', 'writing', 'stored', 'skipped', 'failed'] as const;
export const PERF_ASSET_FAIL_CODES = ['unsupported', 'quota', 'security', 'locked', 'io', 'other', 'verify', 'busy'] as const;
export const PERF_ASSET_HASH_MODES = ['subtle', 'js-stream'] as const;

type Obj = Record<string, unknown>;
const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v);

/** the reasons a beacon's `client`, `display` and `hz` blocks are not what the page writes (an empty list: they are). A version 1 beacon (no such blocks) has no problems of this kind. */
export function beaconProblems(b: unknown): string[] {
  const out: string[] = [];
  if (!isObj(b)) return ['the beacon is not an object'];
  const num = (path: string, v: unknown, lo: number, hi: number, integer = false, nullable = false): void => {
    if (v === null && nullable) return;
    if (typeof v !== 'number' || !Number.isFinite(v)) { out.push(`${path}: not a finite number (${JSON.stringify(v)})`); return; }
    if (v < lo || v > hi) out.push(`${path}: ${v} is outside ${lo}..${hi}`);
    if (integer && !Number.isInteger(v)) out.push(`${path}: ${v} is not a whole number`);
  };
  const str = (path: string, v: unknown, max: number, nullable = false): void => {
    if (v === null && nullable) return;
    if (typeof v !== 'string') { out.push(`${path}: not a string`); return; }
    if (v.length > max) out.push(`${path}: longer than ${max}`);
  };
  const q3 = (path: string, v: unknown): void => {
    if (v === null) return;
    if (!isObj(v)) { out.push(`${path}: not an object or null`); return; }
    num(`${path}.n`, v.n, 1, 1e9, true);
    for (const k of ['p50', 'p95', 'max']) num(`${path}.${k}`, v[k], 0, 3.6e6, false, true);
  };
  const raf = (path: string, v: unknown): void => {
    if (!isObj(v)) { out.push(`${path}: not an object`); return; }
    num(`${path}.n`, v.n, 0, 1e9, true);
    num(`${path}.perSecond`, v.perSecond, 0, 1e4, false, true);
    for (const k of ['medianMs', 'p95Ms', 'p99Ms', 'maxMs']) num(`${path}.${k}`, v[k], 0, 3.6e6, false, true);
    num(`${path}.hz`, v.hz, 0.001, 1e5, false, true);
    num(`${path}.peakHz`, v.peakHz, 0.001, 1e5, false, true);
    num(`${path}.slow`, v.slow, 0, 1e9, true);
    num(`${path}.over12p5`, v.over12p5, 0, 1e9, true);
    if (!Array.isArray(v.bins) || (v.bins.length !== 0 && v.bins.length !== PERF_RAF_BINS)) out.push(`${path}.bins: not ${PERF_RAF_BINS} counts (or none)`);
    else {
      v.bins.forEach((c, i) => num(`${path}.bins[${i}]`, c, 0, 1e9, true));
      if (v.bins.length && typeof v.n === 'number' && v.bins.reduce((a: number, c) => a + (typeof c === 'number' ? c : 0), 0) !== v.n) out.push(`${path}.bins: the counts do not add up to n`);
    }
    if (typeof v.slow === 'number' && typeof v.n === 'number' && v.slow > v.n) out.push(`${path}.slow: more slow frames than intervals`);
    if (typeof v.medianMs === 'number' && typeof v.p95Ms === 'number' && v.p95Ms < v.medianMs) out.push(`${path}: p95 below the median`);
    if (typeof v.p95Ms === 'number' && typeof v.maxMs === 'number' && v.maxMs < v.p95Ms - 0.06) out.push(`${path}: max below p95`);
  };
  const canvas = (path: string, v: unknown): void => {
    if (v === null) return;
    if (!isObj(v)) { out.push(`${path}: not an object or null`); return; }
    num(`${path}.draws`, v.draws, 0, 1e12, true);
    num(`${path}.drawsPerSecond`, v.drawsPerSecond, 0, 1e5, false, true);
    num(`${path}.ticks`, v.ticks, 0, 1e12, true);
    num(`${path}.freshTicks`, v.freshTicks, 0, 1e12, true);
    num(`${path}.freshRatio`, v.freshRatio, 0, 1, false, true);
    num(`${path}.freshPerSecond`, v.freshPerSecond, 0, 1e5, false, true);
    num(`${path}.drawsPerFreshTick`, v.drawsPerFreshTick, 1, 1e6, false, true);
    q3(`${path}.drawMs`, v.drawMs);
    q3(`${path}.transitMs`, v.transitMs);
    if (typeof v.freshTicks === 'number' && typeof v.ticks === 'number' && v.freshTicks > v.ticks) out.push(`${path}: more fresh ticks than ticks`);
  };
  if (b.v === 1 && b.client === undefined && b.display === undefined && b.hz === undefined) return out;
  num('v', b.v, 2, 3, true);
  if (typeof b.sid !== 'string' || !/^[0-9a-f]{12}$/.test(b.sid)) out.push('sid: not 12 hex digits');
  if (b.v === 3) inputBlock(b.input, num, out);
  audioBlock(b.audio, num, str, out);
  const eng = b.engine;
  // decision 0209: optional `engine.softStops`: at most 10 {api, n, ret?}
  if (isObj(eng) && eng.softStops !== undefined) {
    if (!Array.isArray(eng.softStops)) out.push('engine.softStops: not an array');
    else {
      if (eng.softStops.length > 10) out.push('engine.softStops: more than 10');
      eng.softStops.forEach((e: unknown, i: number) => {
        if (!isObj(e)) { out.push(`engine.softStops[${i}]: not an object`); return; }
        if (typeof e.api !== 'string' || !/^[A-Za-z0-9_:.!]{1,60}$/.test(e.api)) out.push(`engine.softStops[${i}].api: not an API name`);
        num(`engine.softStops[${i}].n`, e.n, 1, 1e12, true);
        if (e.ret !== undefined && (typeof e.ret !== 'string' || !/^0x[0-9a-f]{1,16}$/.test(e.ret))) out.push(`engine.softStops[${i}].ret: not a hex address`);
      });
    }
  }
  if (isObj(eng) && eng.exit !== null && eng.exit !== undefined) {
    if (!isObj(eng.exit)) out.push('engine.exit: not an object or null');
    else {
      str('engine.exit.reason', eng.exit.reason, 40);
      num('engine.exit.code', eng.exit.code, -1e9, 1e9, true);
      num('engine.exit.presents', eng.exit.presents, 0, 1e12, true);
      // decision 0209: optional; the engine's last words, at most 400 characters, one line, and no home directory, e-mail address or URL (the page removes them before sending)
      const m = eng.exit.message;
      if (m !== undefined) {
        str('engine.exit.message', m, PERF_EXIT_MESSAGE_MAX);
        if (typeof m === 'string') {
          if (/[\u0000-\u001f\u007f]/.test(m)) out.push('engine.exit.message: contains a control character or a line break');
          if (/(?:^|[\s"'(])(?:\/home\/|\/Users\/|\/root(?:\/|\b)|[A-Za-z]:\\Users\\)/i.test(m)) out.push('engine.exit.message: contains a home directory path');
          if (/[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+\.[A-Za-z]/.test(m)) out.push('engine.exit.message: contains an e-mail address');
          if (/[a-z][a-z0-9+.-]*:\/\//i.test(m)) out.push('engine.exit.message: contains a URL');
        }
      }
    }
  }
  const c = b.client;
  if (c !== null && c !== undefined) {
    if (!isObj(c)) out.push('client: not an object or null');
    else {
      const br = c.browser;
      if (!isObj(br)) out.push('client.browser: not an object');
      else { str('client.browser.name', br.name, 40); str('client.browser.version', br.version, 20, true); num('client.browser.major', br.major, 0, 10000, true, true); str('client.browser.os', br.os, 40, true); if (typeof br.headless !== 'boolean') out.push('client.browser.headless: not a boolean'); }
      num('client.colorDepth', c.colorDepth, 1, 64, true, true);
      if (c.viewport !== null) { if (!isObj(c.viewport)) out.push('client.viewport: not an object or null'); else { num('client.viewport.width', c.viewport.width, 0, 1e5); num('client.viewport.height', c.viewport.height, 0, 1e5); } }
      const pc = c.physicalCores;
      if (!isObj(pc) || pc.value !== null || typeof pc.reason !== 'string' || pc.reason === '') out.push('client.physicalCores: must say that the number is not available (value null and a reason)');
      if (!isObj(c.page)) out.push('client.page: not an object');
    }
  }
  const d = b.display;
  if (d !== null && d !== undefined) {
    if (!isObj(d)) out.push('display: not an object or null');
    else {
      if (typeof d.rafSupported !== 'boolean') out.push('display.rafSupported: not a boolean');
      // decision 0202: how the rAF was asked for; a record without `mode` is a continuous loop (decision 0192's)
      if (d.mode !== undefined && d.mode !== 'burst' && d.mode !== 'continuous') out.push(`display.mode: ${JSON.stringify(d.mode)} is not burst or continuous`);
      if (d.mode === 'burst') {
        if (!isObj(d.burst)) out.push('display.burst: a burst-mode record must say the burst length, the period and the count');
        else { num('display.burst.onMs', d.burst.onMs, 1, 3.6e6); num('display.burst.periodMs', d.burst.periodMs, 1, 3.6e7); num('display.burst.count', d.burst.count, 0, 1e9, true); if (typeof d.burst.onMs === 'number' && typeof d.burst.periodMs === 'number' && d.burst.onMs > d.burst.periodMs) out.push('display.burst: a burst longer than its period'); }
      } else if (d.burst !== undefined && d.burst !== null) out.push('display.burst: only a burst-mode record has it');
      num('display.runningMs', d.runningMs, 0, 1e10);
      const vis = d.visibility;
      if (!isObj(vis)) out.push('display.visibility: not an object');
      else { num('display.visibility.visibleMs', vis.visibleMs, 0, 1e10); num('display.visibility.hiddenMs', vis.hiddenMs, 0, 1e10); num('display.visibility.changes', vis.changes, 0, 1e9, true); }
      const r = d.raf;
      if (!isObj(r)) out.push('display.raf: not an object');
      else { raf('display.raf.recent10s', r.recent10s); raf('display.raf.total', r.total); if (r.sinceIdle !== null) raf('display.raf.sinceIdle', r.sinceIdle); if (!Array.isArray(r.binEdgesMs) || r.binEdgesMs.length !== PERF_RAF_BINS - 1) out.push('display.raf.binEdgesMs: wrong length'); }
      const cv = d.canvas;
      if (cv !== null) {
        if (!isObj(cv)) out.push('display.canvas: not an object or null');
        else {
          num('display.canvas.received', cv.received, 0, 1e12, true); num('display.canvas.drawn', cv.drawn, 0, 1e12, true);
          if (typeof cv.received === 'number' && typeof cv.drawn === 'number' && cv.drawn > cv.received) out.push('display.canvas: drew more frames than it received');
          canvas('display.canvas.recent10s', cv.recent10s); canvas('display.canvas.total', cv.total); if (cv.sinceIdle !== null) canvas('display.canvas.sinceIdle', cv.sinceIdle);
        }
      }
      const lt = d.longTasks;
      if (!isObj(lt)) out.push('display.longTasks: not an object');
      else { num('display.longTasks.total', lt.total, 0, 1e9, true); num('display.longTasks.recent30s', lt.recent30s, 0, 1e9, true); num('display.longTasks.maxMs', lt.maxMs, 0, 3.6e6, false, true); }
      const ev = d.events;
      if (!isObj(ev)) out.push('display.events: not an object');
      else { num('display.events.slow', ev.slow, 0, 1e9, true); num('display.events.inputsSent', ev.inputsSent, 0, 1e9, true); q3('display.events.delayMs', ev.delayMs); q3('display.events.durationMs', ev.durationMs); }
    }
  }
  const h = b.hz;
  if (h !== null && h !== undefined) {
    if (!isObj(h)) out.push('hz: not an object or null');
    else {
      if (typeof h.fit !== 'string' || !HZ_FITS.includes(h.fit)) out.push(`hz.fit: ${JSON.stringify(h.fit)} is not one of ${HZ_FITS.join(', ')}`);
      num('hz.engineHz', h.engineHz, 24, 1000, false, true); num('hz.engineEnvHz', h.engineEnvHz, 24, 1000, false, true); num('hz.rafHz', h.rafHz, 0.001, 1e5, false, true); num('hz.ratio', h.ratio, 0, 1e4, false, true);
      str('hz.param', h.param, 40, true); str('hz.note', h.note, 400);
    }
  }
  const fl = b.files;
  if (fl !== undefined) {
    if (!isObj(fl)) out.push('files: not an object');
    else {
      if (fl.v !== 1) out.push(`files.v: ${JSON.stringify(fl.v)} is not 1`);
      const cnt = (path: string, v: unknown, keys: string[]): void => {
        if (!isObj(v)) { out.push(`${path}: not an object`); return; }
        for (const k of keys) num(`${path}.${k}`, v[k], 0, 1e9, true);
        for (const k of Object.keys(v)) if (!keys.includes(k)) out.push(`${path}.${k}: not a counter of this block`);
      };
      num('files.opens', fl.opens, 0, 1e9, true);
      cnt('files.pick', fl.pick, ['files', 'folder', 'zip']);
      cnt('files.drop', fl.drop, ['files', 'folders', 'zips']);
      for (const k of ['written', 'skipped', 'failed', 'writtenMiB']) num(`files.${k}`, fl[k], 0, 1e9, true);
      cnt('files.extract', fl.extract, ['zips', 'files', 'cancelled']);
      cnt('files.download', fl.download, ['file', 'folderZip']);
      cnt('files.ops', fl.ops, ['mkdir', 'rename', 'delete', 'copy', 'move']);
      cnt('files.persist', fl.persist, ['asked', 'granted', 'denied', 'pending']);
      cnt('files.engine', fl.engine, ['open', 'save', 'folder', 'answered', 'cancelled', 'zipUnpacked', 'savedBack', 'savedBackFailed']);
      const st = fl.storage;
      if (!isObj(st)) out.push('files.storage: not an object');
      else { num('files.storage.usedMiB', st.usedMiB, 0, 1e9, true, true); num('files.storage.quotaMiB', st.quotaMiB, 0, 1e9, true, true); if (st.persisted !== null && typeof st.persisted !== 'boolean') out.push('files.storage.persisted: not a boolean or null'); }
      const fa = fl.fail;
      if (!isObj(fa)) out.push('files.fail: not an object');
      else for (const [k, v] of Object.entries(fa)) { if (!(PERF_FILES_FAIL_CODES as readonly string[]).includes(k)) out.push(`files.fail.${k}: not a failure code of the list`); num(`files.fail.${k}`, v, 0, 1e9, true); }
    }
  }
  assetsBlock(b.assets, num, out);
  // decision 0212: optional; which display the page asked for and what the engine uses
  const g = b.guestScreen;
  if (g !== null && g !== undefined) {
    if (!isObj(g)) out.push('guestScreen: not an object or null');
    else {
      if (typeof g.mode !== 'string' || !['fit', 'fit-device', 'fixed', 'legacy'].includes(g.mode)) out.push(`guestScreen.mode: ${JSON.stringify(g.mode)} is not one of fit, fit-device, fixed, legacy`);
      if (typeof g.source !== 'string' || !['url', 'stored', 'default'].includes(g.source)) out.push(`guestScreen.source: ${JSON.stringify(g.source)} is not one of url, stored, default`);
      if (typeof g.follow !== 'string' || !['reload', 'live'].includes(g.follow)) out.push(`guestScreen.follow: ${JSON.stringify(g.follow)} is not reload or live`);
      num('guestScreen.width', g.width, 1, 16384, true); num('guestScreen.height', g.height, 1, 16384, true);
      num('guestScreen.dpi', g.dpi, 96, 384, true, true); num('guestScreen.dpr', g.dpr, 0.1, 16);
      num('guestScreen.roomWidth', g.roomWidth, 0, 16384, false, true); num('guestScreen.roomHeight', g.roomHeight, 0, 16384, false, true);
      if (typeof g.reported !== 'boolean') out.push('guestScreen.reported: not a boolean');
    }
  }
  return out;
}

export const AUDIO_STATUSES = ['idle', 'starting', 'needs-gesture', 'resuming', 'running', 'blocked', 'error', 'closed'];
const AUDIO_CTX_STATES = ['suspended', 'running', 'closed', 'interrupted'];
const RESUME_RESULTS = ['running', 'pending', 'rejected', 'suspended', 'no-context'];

/**
 * The `audio` block (decision 0175; decision 0211 adds the fields that tell why a page is silent): `underruns` (null: nothing measured) and, when the page has an audio output, `detail`. A detail without `status` is the one of
 * decision 0175 (a page opened with ?audio=1 before decision 0211) and is only checked for being an object. The checks that couple fields are the ones a log reader relies on: a `running` status needs a running context, a
 * `needs-gesture` status means no gesture has been seen, a `blocked` status means a resume() that did not end in a running context.
 */
function audioBlock(v: unknown, num: (path: string, v: unknown, lo: number, hi: number, integer?: boolean, nullable?: boolean) => void, str: (path: string, v: unknown, max: number, nullable?: boolean) => void, out: string[]): void {
  if (v === undefined) return;
  if (!isObj(v)) { out.push('audio: not an object'); return; }
  num('audio.underruns', v.underruns, 0, 1e12, true, true);
  const d = v.detail;
  if (d === undefined) return;
  if (!isObj(d)) { out.push('audio.detail: not an object'); return; }
  if (d.status === undefined) return;
  if (typeof d.status !== 'string' || !AUDIO_STATUSES.includes(d.status)) out.push(`audio.detail.status: ${JSON.stringify(d.status)} is not one of ${AUDIO_STATUSES.join(', ')}`);
  if (d.ctxState !== null && (typeof d.ctxState !== 'string' || !AUDIO_CTX_STATES.includes(d.ctxState))) out.push(`audio.detail.ctxState: ${JSON.stringify(d.ctxState)} is not null or one of ${AUDIO_CTX_STATES.join(', ')}`);
  num('audio.detail.sampleRate', d.sampleRate, 3000, 768000, false, true);
  num('audio.detail.baseLatencyMs', d.baseLatencyMs, 0, 1e4, false, true);
  num('audio.detail.outputLatencyMs', d.outputLatencyMs, 0, 1e4, false, true);
  str('audio.detail.autoplayPolicy', d.autoplayPolicy, 24, true);
  if (typeof d.nodeReady !== 'boolean') out.push('audio.detail.nodeReady: not a boolean');
  num('audio.detail.gestures', d.gestures, 0, 1e9, true);
  num('audio.detail.resumeAttempts', d.resumeAttempts, 0, 1e9, true);
  num('audio.detail.softSinkFrames', d.softSinkFrames, 0, 1e15, true);
  num('audio.detail.capFrames', d.capFrames, 1024, 1 << 24, true);
  for (const k of ['engineGapFrames', 'engineSkippedFrames', 'engineGapFramesLast60s', 'engineLateWrites', 'engineLateWriteApplied', 'engineLateWriteLost']) num(`audio.detail.${k}`, d[k], 0, 1e15, true, true);
  if (typeof d.engineGapFrames === 'number' && typeof d.engineGapFramesLast60s === 'number' && d.engineGapFramesLast60s > d.engineGapFrames) out.push('audio.detail: more gap frames in the last minute than in total');
  num('audio.detail.fillFrames', d.fillFrames, -1e9, 1e9, true);
  num('audio.detail.fillMinFrames', d.fillMinFrames, -1e9, 1e9, true);
  num('audio.detail.underrunQuanta', d.underrunQuanta, 0, 1e12, true);
  num('audio.detail.quantaWhilePlaying', d.quantaWhilePlaying, 0, 1e12, true);
  num('audio.detail.nonSilentQuanta', d.nonSilentQuanta, 0, 1e12, true);
  str('audio.detail.error', d.error, 400, true);
  const r = d.lastResume;
  if (r !== null && r !== undefined) {
    if (!isObj(r)) out.push('audio.detail.lastResume: not an object or null');
    else {
      num('audio.detail.lastResume.at', r.at, 0, 1e12);
      str('audio.detail.lastResume.trigger', r.trigger, 24);
      if (typeof r.result !== 'string' || !RESUME_RESULTS.includes(r.result)) out.push(`audio.detail.lastResume.result: ${JSON.stringify(r.result)} is not one of ${RESUME_RESULTS.join(', ')}`);
      num('audio.detail.lastResume.ms', r.ms, 0, 1e7);
    }
  }
  if (typeof d.resumeAttempts === 'number' && typeof d.lastResume === 'object' && d.lastResume !== null && d.resumeAttempts < 1) out.push('audio.detail: a last resume() without any attempt');
  if (d.status === 'running' && d.ctxState !== 'running') out.push('audio.detail: status running but the context is not running');
  if (d.status === 'idle' && (d.ctxState !== null || d.nodeReady === true)) out.push('audio.detail: status idle but a context or a node exists');
  if (d.status === 'running' && d.nodeReady !== true) out.push('audio.detail: status running but the worklet node does not exist');
  if (d.status === 'needs-gesture' && (d.ctxState === 'running' || (typeof d.gestures === 'number' && d.gestures > 0))) out.push('audio.detail: status needs-gesture but the context runs or a gesture was seen');
  if (d.status === 'blocked' && (typeof d.gestures === 'number' && d.gestures === 0)) out.push('audio.detail: status blocked without a gesture');
  if (typeof d.underrunQuanta === 'number' && typeof d.quantaWhilePlaying === 'number' && d.underrunQuanta > d.quantaWhilePlaying) out.push('audio.detail: more underrun quanta than quanta played');
  if (typeof d.nonSilentQuanta === 'number' && typeof d.quantaWhilePlaying === 'number' && d.nonSilentQuanta > d.quantaWhilePlaying) out.push('audio.detail: more non-silent quanta than quanta played');
}

const INPUT_STAGES: readonly [string, boolean][] = [['s0', false], ['s0b', false], ['toWorker', true], ['worker', false], ['fromWorker', true], ['put', false], ['raf', false]];

/**
 * A version 3 beacon's `input` block (decision 0202, 0205): the counters (`answered` / `late` / `noAnswer` are decision 0205's: the processed inputs by how the engine answered them), `eventToRaf` (event.timeStamp -> the next animation frame after the answer was drawn, the P2 number) and `listenerToPut` (version 2's number) as {n, p50Ms, p95Ms, maxMs, lastMs}, and
 * `stages` (null, or the median and 95th percentile of each stage). A stage that crosses between the page's and the Worker's clock (`toWorker`, `fromWorker`) may be a little below 0; the others cannot be.
 */
function inputBlock(v: unknown, num: (path: string, v: unknown, lo: number, hi: number, integer?: boolean, nullable?: boolean) => void, out: string[]): void {
  if (!isObj(v)) { out.push('input: not an object'); return; }
  for (const k of ['sent', 'processed', 'dropped', 'pending', 'answered', 'late', 'noAnswer']) num(`input.${k}`, v[k], 0, 1e12, true);
  if (typeof v.answered === 'number' && typeof v.late === 'number' && typeof v.noAnswer === 'number' && typeof v.processed === 'number' && v.answered + v.late + v.noAnswer > v.processed) out.push('input: answered + late + noAnswer exceeds processed');
  num('input.targetP95Ms', v.targetP95Ms, 0, 1e4);
  // eventToRaf and listenerToPut are always there; lateVisible and eventToRafWithLate (decision 0205) are null when the page cannot tell a changed frame from a redraw (?dedupe is not 1)
  for (const [k, nullable] of [['eventToRaf', false], ['listenerToPut', false], ['lateVisible', true], ['eventToRafWithLate', true]] as const) {
    const s = v[k];
    if (nullable && s === null) continue;
    if (!isObj(s)) { out.push(`input.${k}: not an object${nullable ? ' or null' : ''}`); continue; }
    num(`input.${k}.n`, s.n, 0, 1e9, true);
    for (const f of ['p50Ms', 'p95Ms', 'maxMs']) num(`input.${k}.${f}`, s[f], 0, 3.6e6, false, true);
    if (!Array.isArray(s.lastMs)) { out.push(`input.${k}.lastMs: not an array`); continue; }
    s.lastMs.forEach((x, i) => num(`input.${k}.lastMs[${i}]`, x, 0, 3.6e6));
    if (typeof s.n === 'number' && s.lastMs.length > s.n) out.push(`input.${k}: more values than n`);
    if (typeof s.p50Ms === 'number' && typeof s.p95Ms === 'number' && s.p95Ms < s.p50Ms) out.push(`input.${k}: p95 below the median`);
    if (typeof s.p95Ms === 'number' && typeof s.maxMs === 'number' && s.maxMs < s.p95Ms) out.push(`input.${k}: max below p95`);
    if (typeof s.n === 'number' && s.n === 0 && s.p50Ms !== null) out.push(`input.${k}: no values but a median`);
  }
  if ((v.lateVisible === null) !== (v.eventToRafWithLate === null)) out.push('input: lateVisible and eventToRafWithLate must both be null or both be there');
  const st = v.stages;
  if (st === null) return;
  if (!isObj(st)) { out.push('input.stages: not an object or null'); return; }
  num('input.stages.n', st.n, 1, 1e9, true);
  const e = v.eventToRaf;
  if (typeof st.n === 'number' && isObj(e) && typeof e.n === 'number' && st.n > e.n) out.push('input.stages: more events than eventToRaf has');
  for (const w of ['p50', 'p95']) {
    const o = st[w];
    if (!isObj(o)) { out.push(`input.stages.${w}: not an object`); continue; }
    for (const [k, cross] of INPUT_STAGES) num(`input.stages.${w}.${k}`, o[k], cross ? -100 : 0, 3.6e6);
  }
}

const inList = (list: readonly string[], v: unknown): boolean => typeof v === 'string' && list.includes(v);

/** decision 0220: the optional `assets` block (still beacon version 3, like `files`): where each of the four start-up assets came from and where the time went. Whole numbers and words of fixed lists only. */
function assetsBlock(a: unknown, num: (path: string, v: unknown, lo: number, hi: number, integer?: boolean, nullable?: boolean) => void, out: string[]): void {
  if (a === undefined) return;
  if (!isObj(a)) { out.push('assets: not an object'); return; }
  if (a.v !== 1) out.push(`assets.v: ${JSON.stringify(a.v)} is not 1`);
  if (!inList(['on', 'off', 'none'], a.store)) out.push(`assets.store: ${JSON.stringify(a.store)} is not on, off or none`);
  if (!inList(PERF_ASSET_HASH_MODES, a.hash)) out.push(`assets.hash: ${JSON.stringify(a.hash)} is not one of ${PERF_ASSET_HASH_MODES.join(', ')}`);
  if (!Array.isArray(a.items)) out.push('assets.items: not an array');
  else {
    if (a.items.length > PERF_ASSET_KINDS.length) out.push(`assets.items: ${a.items.length} items, at most ${PERF_ASSET_KINDS.length}`);
    const seen = new Set<string>();
    a.items.forEach((it: unknown, i: number) => {
      const p = `assets.items[${i}]`;
      if (!isObj(it)) { out.push(`${p}: not an object`); return; }
      if (!inList(PERF_ASSET_KINDS, it.kind)) out.push(`${p}.kind: ${JSON.stringify(it.kind)} is not one of ${PERF_ASSET_KINDS.join(', ')}`);
      else if (seen.has(it.kind as string)) out.push(`${p}.kind: ${String(it.kind)} twice`);
      else seen.add(it.kind as string);
      if (typeof it.sha !== 'string' || !/^[0-9a-f]{8}$/.test(it.sha)) out.push(`${p}.sha: must be the first 8 hex digits`);
      num(`${p}.bytes`, it.bytes, 0, 1e12, true);
      num(`${p}.wire`, it.wire, 0, 1e12, true, true);
      num(`${p}.transfer`, it.transfer, 0, 1e12, true, true);
      if (!inList(PERF_ASSET_ORIGINS, it.origin)) out.push(`${p}.origin: ${JSON.stringify(it.origin)} is not one of ${PERF_ASSET_ORIGINS.join(', ')}`);
      if (!inList(PERF_ASSET_STORE_STATES, it.store)) out.push(`${p}.store: ${JSON.stringify(it.store)} is not one of ${PERF_ASSET_STORE_STATES.join(', ')}`);
      if (it.fail !== null && !inList(PERF_ASSET_FAIL_CODES, it.fail)) out.push(`${p}.fail: ${JSON.stringify(it.fail)} is not a code of the list`);
      if (it.origin === 'storage-cache' && it.store !== 'hit') out.push(`${p}: bytes from the storage copy must say store hit`);
      if (it.store === 'hit' && it.origin !== 'storage-cache') out.push(`${p}: store hit must say origin storage-cache`);
      const ms = it.ms;
      if (!isObj(ms)) out.push(`${p}.ms: not an object`);
      else { for (const k of ['headers', 'read', 'hash', 'copy', 'parse', 'store', 'total']) num(`${p}.ms.${k}`, ms[k], 0, 3.6e6, true); num(`${p}.ms.compile`, ms.compile, 0, 3.6e6, true, true); }
    });
  }
  const st = a.storage;
  if (st !== null) {
    if (!isObj(st)) out.push('assets.storage: not an object or null');
    else { num('assets.storage.usedMiB', st.usedMiB, 0, 1e9, true, true); num('assets.storage.quotaMiB', st.quotaMiB, 0, 1e9, true, true); if (st.persisted !== null && typeof st.persisted !== 'boolean') out.push('assets.storage.persisted: not a boolean or null'); }
  }
  const cl = a.clean;
  if (cl !== null) {
    if (!isObj(cl)) out.push('assets.clean: not an object or null');
    else for (const k of ['removed', 'kept', 'busy', 'failed']) num(`assets.clean.${k}`, cl[k], 0, 1e6, true);
  }
}
