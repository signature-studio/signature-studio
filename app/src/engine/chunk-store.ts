// Decision 0314 (PROVISIONAL): where the chunks of the lazy install tree come from. A chunk is an immutable object `vfs/<SHA-256 of its decoded bytes>.chk`; a brotli stream's objects are stored encoded and served with
// Content-Encoding: br, so fetch() hands the page the DECODED bytes, whose SHA-256 is the name (docs/decisions/0314-lazy-install-tree.md 'Format'). Every chunk is checked against its name before anyone sees it.

export interface ChunkStore {
  /** the decoded bytes of the chunk named `sha` (lower-case hex of the SHA-256 of those bytes); `coding` 0 identity, 1 brotli; `length` is what the index says they must be */
  get(sha: string, coding: 0 | 1, length: number): Promise<Uint8Array>;
}

export class ChunkStoreError extends Error {
  constructor(what: string) { super(what); this.name = 'ChunkStoreError'; }
}

export async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const d = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes as Uint8Array<ArrayBuffer>));
  let s = '';
  for (const x of d) s += x.toString(16).padStart(2, '0');
  return s;
}

export interface FetchChunkStoreOptions {
  /** how many times a chunk is asked for when the network fails or the bytes do not hash to their name (the later tries bypass the HTTP cache) */
  attempts?: number;
  fetchFn?: typeof fetch;
}

/** the chunks as objects of a URL prefix: `${base}vfs/${sha}.chk` (the base ends with a slash) */
export class FetchChunkStore implements ChunkStore {
  private readonly base: string;
  private readonly attempts: number;
  private readonly fetchFn: typeof fetch;
  fetched = 0;
  bytes = 0;
  retries = 0;

  constructor(base: string, opts: FetchChunkStoreOptions = {}) {
    this.base = base.endsWith('/') ? base : `${base}/`;
    this.attempts = opts.attempts ?? 3;
    this.fetchFn = opts.fetchFn ?? ((...a) => fetch(...a));
  }

  async get(sha: string, _coding: 0 | 1, length: number): Promise<Uint8Array> {
    if (!/^[0-9a-f]{64}$/.test(sha)) throw new ChunkStoreError(`${sha} is not a SHA-256`);
    let last = 'no try';
    for (let k = 0; k < this.attempts; k++) {
      if (k > 0) this.retries++;
      try {
        const r = await this.fetchFn(`${this.base}vfs/${sha}.chk`, k === 0 ? {} : { cache: 'reload' });
        if (!r.ok) { last = `HTTP ${r.status}`; continue; }
        const b = new Uint8Array(await r.arrayBuffer());
        if (b.length !== length) { last = `${b.length} bytes, the index says ${length}`; continue; }
        if ((await sha256Hex(b)) !== sha) { last = 'the bytes do not hash to the name of the object'; continue; }
        this.fetched++;
        this.bytes += b.length;
        return b;
      } catch (e) {
        last = String((e as Error)?.message ?? e);
      }
    }
    throw new ChunkStoreError(`chunk ${sha}: ${last} (after ${this.attempts} tries)`);
  }
}
