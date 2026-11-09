// The serving-side rule of docs/policies/progress-integrity.md (decision 0152): the DEFAULT manifest path /assets/manifest.json (R2 key "manifest.json") is for the bundle of the REAL engine only.
// Anything else there (a manifest of a kind this page does not run, a broken file) is refused with 409 / 500 and a plain-text explanation.
// Used by tools/assets-middleware.ts (vite dev / vite preview) and worker/index.ts (Cloudflare Worker + R2), so that both answer the same way.

/** engine.kind values that run the lifted FL engine. Anything else at the default path is refused (an allow-list: nothing that is not the real engine slips through by being new). */
export const REAL_ENGINE_KINDS: readonly string[] = ['emscripten-fruity-v1'];
/** a manifest bigger than this is not inspected (a real manifest is a few KB) */
export const MAX_MANIFEST_BYTES = 2 * 1024 * 1024;
export const POLICY_DOC = 'docs/policies/progress-integrity.md';

export type Verdict = { ok: true; kind: string } | { ok: false; status: 404 | 409 | 500; body: string };

/**
 * The logical key of a request path below /assets/, for the one question "is this the default manifest?". `decodedRel` is the part after the prefix, percent-decoded once.
 * Dot segments and empty segments (`//`) are resolved, case is folded (NFKC + lower case: a case-insensitive file system or R2 key must not be a way around), and trailing dots / spaces of the
 * last segment are dropped (Windows). A trailing slash is kept: `manifest.json/` names a directory, not the manifest.
 */
export function logicalKey(decodedRel: string): string {
  const s = decodedRel.normalize('NFKC').replace(/\\/g, '/');
  const out: string[] = [];
  for (const seg of s.split('/')) {
    if (seg === '' || seg === '.') continue;
    if (seg === '..') { out.pop(); continue; }
    out.push(seg);
  }
  if (s.endsWith('/') || s.endsWith('/.') || s.endsWith('/..')) return out.join('/').toLowerCase() + '/';
  const last = out.length - 1;
  if (last >= 0) out[last] = (out[last] as string).replace(/[. ]+$/, '');
  return out.join('/').toLowerCase();
}

export function isDefaultManifestKey(decodedRel: string): boolean {
  return logicalKey(decodedRel) === 'manifest.json';
}

const short = (s: string, n = 64): string => (s.length > n ? s.slice(0, n) + '...' : s);

export const REFUSED_HEAD = 'このパスは実エンジンのバンドル専用です。';

export function notRealEngineRefusal(kind: string | null): string {
  const what = kind === null ? 'engine.kind が読めません' : `engine.kind "${short(kind)}" は実エンジンの種類(${REAL_ENGINE_KINDS.join(', ')})ではありません`;
  return `${REFUSED_HEAD}\n/assets/manifest.json: ${what}。\n理由: ${POLICY_DOC}\n`;
}

export function brokenManifestBody(detail: string): string {
  return `/assets/manifest.json が壊れています(${short(detail, 160)})。\n実エンジンのバンドルの manifest.json を置き直してください。中身を検査できないため、配信しません。\n理由: ${POLICY_DOC}\n`;
}

/** the 404 for a missing default manifest: says what is missing, not just "not found" */
export function notDeployedBody(extra = ''): string {
  return `実エンジンのバンドルが未配置です。\n/assets/manifest.json が見つかりません。実エンジン(emscripten-fruity-v1)のバンドルの manifest.json をここ(FL_ASSETS_DIR の直下、R2 ならキー manifest.json)に置いてください。\n代役で代わりにしてはいけません。\n理由: ${POLICY_DOC}\n${extra}`;
}

/** decides from the text of the manifest the default path would serve */
export function judgeManifestText(text: string): Verdict {
  let json: unknown;
  // the position only: V8's message quotes a piece of the text, and the content of a file that is not a manifest is not echoed
  try { json = JSON.parse(text); } catch (e) { const pos = /position (\d+)/.exec(String((e as Error)?.message ?? ''))?.[1]; return { ok: false, status: 500, body: brokenManifestBody(`JSON として読めません${pos ? `: 位置 ${pos}` : ''}`) }; }
  const engine = typeof json === 'object' && json !== null && !Array.isArray(json) ? (json as { engine?: unknown }).engine : undefined;
  const kind = typeof engine === 'object' && engine !== null && !Array.isArray(engine) ? (engine as { kind?: unknown }).kind : undefined;
  if (typeof kind !== 'string') return { ok: false, status: 409, body: notRealEngineRefusal(null) };
  if (!REAL_ENGINE_KINDS.includes(kind)) return { ok: false, status: 409, body: notRealEngineRefusal(kind) };
  return { ok: true, kind };
}

/** same, from bytes (UTF-8; TextDecoder drops a BOM) */
export function judgeManifestBytes(bytes: Uint8Array): Verdict {
  let text: string;
  try { text = new TextDecoder('utf-8', { fatal: true, ignoreBOM: false }).decode(bytes); } catch { return { ok: false, status: 500, body: brokenManifestBody('UTF-8 として読めません') }; }
  return judgeManifestText(text);
}
