// HTTP helpers shared by the dev/preview middleware (tools/assets-middleware.ts) and the Cloudflare Worker (worker/index.ts).
// Pure functions, no DOM / Node / Workers types. Range semantics: RFC 9110 section 14 (single byte range; everything else is ignored, which the RFC allows).

export type RangeResult =
  | { kind: 'none' }
  | { kind: 'partial'; start: number; end: number }
  | { kind: 'unsatisfiable' };

const MAX_DIGITS = 15;

export function parseRange(header: string | null | undefined, size: number): RangeResult {
  if (!header) return { kind: 'none' };
  const m = /^\s*bytes\s*=\s*(.*)$/i.exec(header);
  if (!m) return { kind: 'none' };
  const spec = (m[1] ?? '').trim();
  if (spec.includes(',')) return { kind: 'none' };
  const r = /^(\d*)\s*-\s*(\d*)$/.exec(spec);
  if (!r) return { kind: 'none' };
  const a = r[1] ?? '';
  const b = r[2] ?? '';
  if (a === '' && b === '') return { kind: 'none' };
  if (a.length > MAX_DIGITS || b.length > MAX_DIGITS) return a === '' ? { kind: 'partial', start: 0, end: size - 1 } : { kind: 'unsatisfiable' };
  if (a === '') {
    const n = Number(b);
    if (n === 0 || size === 0) return { kind: 'unsatisfiable' };
    return { kind: 'partial', start: Math.max(0, size - n), end: size - 1 };
  }
  const start = Number(a);
  if (b !== '' && Number(b) < start) return { kind: 'none' };
  if (start >= size) return { kind: 'unsatisfiable' };
  const end = b === '' ? size - 1 : Math.min(Number(b), size - 1);
  return { kind: 'partial', start, end };
}

const stripWeak = (t: string): string => t.trim().replace(/^W\//, '');

// If-None-Match uses the weak comparison (RFC 9110 section 13.1.2). `*` matches any existing representation.
export function ifNoneMatchHit(header: string | null | undefined, etag: string): boolean {
  if (!header) return false;
  if (header.trim() === '*') return true;
  const want = stripWeak(etag);
  return header.split(',').some((t) => stripWeak(t) === want);
}

// If-Range (RFC 9110 section 13.1.5): a validator that must match for the Range to apply. Only strong ETags are compared here.
export function ifRangeAllows(header: string | null | undefined, etag: string): boolean {
  if (!header) return true;
  const t = header.trim();
  if (t.startsWith('W/')) return false;
  return t === etag;
}
