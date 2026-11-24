// Issue #1383: the records of the virtual registry's HKCU tree (the format is at reg_journal in lift/runtime/fle_reg.c) and the state they make. The saved log keeps the last record
// of each key and each value (names compare without regard to ASCII case, as the registry does). A deletion stays as a record: a seed value that the deletion removed must stay
// removed at the next start, and the seed is applied again at every start. Pure: the file is in regstore.ts.

export const REG_OP = { key: 1, value: 2, delValue: 3, delKey: 4 } as const;

export interface RegRecord {
  op: number;
  /** the key's path below HKCU */
  key: string;
  /** the value's name (empty for a key record) */
  name: string;
  type: number;
  /** the record's bytes, as they are kept */
  bytes: Uint8Array;
  /** the offset just after the record */
  end: number;
}

const utf16 = new TextDecoder('utf-16le');

/** the record at `at`, or null when the bytes there are not a whole record */
export function parseRecord(b: Uint8Array, at = 0): RegRecord | null {
  let p = at;
  const u32 = (): number | null => {
    if (p + 4 > b.length) return null;
    const v = (b[p] ?? 0) | ((b[p + 1] ?? 0) << 8) | ((b[p + 2] ?? 0) << 16) | ((b[p + 3] ?? 0) << 24);
    p += 4;
    return v >>> 0;
  };
  const op = b[p];
  if (op === undefined || op < REG_OP.key || op > REG_OP.delKey) return null;
  p++;
  const kl = u32();
  if (kl === null || kl % 2 !== 0 || p + kl > b.length) return null;
  const key = utf16.decode(b.slice(p, p + kl));
  p += kl;
  const nl = u32();
  if (nl === null || nl % 2 !== 0 || p + nl > b.length) return null;
  const name = utf16.decode(b.slice(p, p + nl));
  p += nl;
  const type = u32();
  if (type === null) return null;
  const dl = u32();
  if (dl === null || p + dl > b.length || (dl > 0 && op !== REG_OP.value)) return null;
  p += dl;
  return { op, key, name, type, bytes: b.slice(at, p), end: p };
}

const fold = (s: string): string => s.replace(/[A-Z]/g, (c) => c.toLowerCase());
const keyId = (key: string): string => `k\u0000${fold(key)}`;
const valueId = (key: string, name: string): string => `v\u0000${fold(key)}\u0000${fold(name)}`;

export class RegJournal {
  private readonly live = new Map<string, Uint8Array>();
  private size = 0;

  /** the bytes of the live records */
  get liveBytes(): number { return this.size; }

  /** the records of `buf`, in order; a record cut short at the end is left out (`dropped`: the bytes that were left out) */
  load(buf: Uint8Array): { records: number; dropped: number } {
    let at = 0, records = 0;
    while (at < buf.length) {
      const r = parseRecord(buf, at);
      if (r === null) break;
      this.add(r);
      records++;
      at = r.end;
    }
    return { records, dropped: buf.length - at };
  }

  /** one record (a change, in the order it happened) */
  add(r: RegRecord): void {
    if (r.op === REG_OP.key) {
      this.set(keyId(r.key), r.bytes);
    } else if (r.op === REG_OP.value || r.op === REG_OP.delValue) {
      this.set(valueId(r.key, r.name), r.bytes);
    } else {
      // the key and what is under it leave the live set; the deletion of the key itself stays as a record
      const k = fold(r.key), own = keyId(r.key);
      const under = [`k\u0000${k}\\`, `v\u0000${k}\\`, `v\u0000${k}\u0000`];
      for (const id of [...this.live.keys()]) {
        if (id === own || under.some((p) => id.startsWith(p))) this.delete(id);
      }
      this.set(own, r.bytes);
    }
  }

  /** the live records in the order of their first appearance: replayed, they make the same tree */
  image(): Uint8Array {
    const out = new Uint8Array(this.size);
    let at = 0;
    for (const b of this.live.values()) {
      out.set(b, at);
      at += b.length;
    }
    return out;
  }

  /** replaces in place: a key or value keeps the place of its first record (the registry lists keys in the order they were made) */
  private set(id: string, bytes: Uint8Array): void {
    const old = this.live.get(id);
    if (old !== undefined) this.size -= old.length;
    this.live.set(id, bytes);
    this.size += bytes.length;
  }

  private delete(id: string): void {
    const old = this.live.get(id);
    if (old === undefined) return;
    this.live.delete(id);
    this.size -= old.length;
  }
}
