// Issue #1502: the Recent list of a project the person opened, kept across a reload. FL keeps its list in the registry (the values "1" to "49" under MRU: "1" is the newest, a REG_SZ
// with the project's path; the reads are in #1409). FL's exit does not write it here (its shutdown does not return: pm-work/findings/recent-1502.md), so the page records the open:
// the project's content goes into the journal as a user's file (reg-journal.ts op 5) and its path into the list (the value records, as lift/runtime/fle_reg.c writes them).
// Pure: the Worker holds the journal (regstore.ts). The key and the value names are FL's own (the ones FL reads), not a table of files.

import { parseRecord, REG_OP } from './reg-journal.ts';

/** FL's list: the registry key (below HKCU) and its value names (decimal, from "1") */
export const RECENT_KEY = 'Software\\Image-Line\\FL Studio 26\\MRU';
/** the value's type (REG_SZ) */
export const RECENT_TYPE = 1;
/** the values FL reads: "1" to "49" */
export const RECENT_MAX = 49;

const utf16 = new TextDecoder('utf-16le');
const fold = (s: string): string => s.replace(/[A-Z]/g, (c) => c.toLowerCase());

/** a string as UTF-16LE bytes (no terminator) */
export function utf16Bytes(s: string): Uint8Array {
  const out = new Uint8Array(s.length * 2);
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    out[2 * i] = c & 0xff;
    out[2 * i + 1] = c >> 8;
  }
  return out;
}

/** a record as lift/runtime/fle_reg.c writes it: u8 op | u32 key bytes | key | u32 name bytes | name | u32 type | u32 data bytes | data (data for op 2 and 5 only) */
export function recordBytes(op: number, key: string, name: string, type: number, data: Uint8Array): Uint8Array {
  const k = utf16Bytes(key), n = utf16Bytes(name);
  const body = op === REG_OP.value || op === REG_OP.filePut ? data : new Uint8Array(0);
  const out = new Uint8Array(1 + 4 + k.length + 4 + n.length + 4 + 4 + body.length);
  const dv = new DataView(out.buffer);
  let at = 0;
  out[at++] = op;
  dv.setUint32(at, k.length, true); at += 4;
  out.set(k, at); at += k.length;
  dv.setUint32(at, n.length, true); at += 4;
  out.set(n, at); at += n.length;
  dv.setUint32(at, type, true); at += 4;
  dv.setUint32(at, body.length, true); at += 4;
  out.set(body, at);
  return out;
}

/** the data of a record (its last field: the u32 before it gives the length) */
function dataOf(bytes: Uint8Array): Uint8Array {
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const kl = dv.getUint32(1, true);
  const nl = dv.getUint32(5 + kl, true);
  const at = 1 + 4 + kl + 4 + nl + 4;
  const dl = dv.getUint32(at, true);
  return bytes.subarray(at + 4, at + 4 + dl);
}

/** the list FL reads, from the saved records (index 0 is "1"): the values under RECENT_KEY that are set (a deleted value stays a record, and is skipped) */
export function recentListOf(saved: Uint8Array): string[] {
  const list: string[] = [];
  for (let at = 0; at < saved.length;) {
    const r = parseRecord(saved, at);
    if (r === null) break;
    at = r.end;
    if (r.op !== REG_OP.value || fold(r.key) !== fold(RECENT_KEY) || r.type !== RECENT_TYPE) continue;
    const i = Number(r.name);
    if (!Number.isInteger(i) || i < 1 || i > RECENT_MAX || String(i) !== r.name) continue;
    list[i - 1] = utf16.decode(dataOf(r.bytes)).replace(/\u0000+$/, '');
  }
  return Array.from(list, (s) => s ?? '');
}

/** the list after the person opens `path`: the path is "1" (the newest), the others move down, a path that is already listed leaves its place, and the list holds RECENT_MAX entries.
 * The records are the values whose text changed */
export function recentAfterOpen(list: readonly string[], path: string): { list: string[]; records: Uint8Array[] } {
  const next = [path, ...list.filter((p) => p !== '' && fold(p) !== fold(path))].slice(0, RECENT_MAX);
  const records: Uint8Array[] = [];
  for (let i = 0; i < next.length; i++) {
    if (list[i] === next[i]) continue;
    records.push(recordBytes(REG_OP.value, RECENT_KEY, String(i + 1), RECENT_TYPE, utf16Bytes(`${next[i]}\u0000`)));
  }
  return { list: next, records };
}

/** a user's file as the journal keeps it (op 5: the path is the key, the name and the type are empty, the data is the content) */
export function fileRecord(path: string, data: Uint8Array): Uint8Array {
  return recordBytes(REG_OP.filePut, path, '', 0, data);
}
