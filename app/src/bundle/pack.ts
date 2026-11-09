// "flpack1": one object that holds many files (the guest's virtual files, DLL data). One object instead of hundreds keeps the R2 object count and the request count low.
//   bytes 0..3   "FLPK"
//   bytes 4..7   version (u32 LE) = 1
//   bytes 8..11  index length in bytes (u32 LE)
//   then         index: UTF-8 JSON {"files":[{"path","offset","size","sha256"}]}, padded with spaces to a multiple of 8 (the payload starts at 12 + padded length - the header
//                is padded to 16-byte alignment: see PAYLOAD_ALIGN)
//   payload      the files, each starting at a multiple of 8 relative to the payload start; `offset` is relative to the payload start
// Integers are little-endian. The file SHA-256 is of the file content. The whole pack is also hashed by the manifest (PackRef.sha256).
//
// parsePack() also reads "flseedp1", the seed pack of decision 0134 (lift/harness/web/final0129/pack_seed.py, which is what the engine's Worker of that decision reads):
//   'FLSEEDP1' (8 bytes) | u32 header_length | header (UTF-8 JSON) | data region
//   header = {"version":1,"root":"seed","count":N,"data_bytes":B,"files":[{"path":"seed/<path>","size","offset","sha256"}]}   offset: from the start of the data region
// so that a pack built by that script can be named in a manifest as it is (PackRef.format = "flseedp1"). Building is only implemented for flpack1.

import { isRelativePackPath } from './schema.ts';
import { sha256HexSync } from './sha256.ts';

export const PACK_MAGIC = 'FLPK';
export const PACK_VERSION = 1;
export const SEED_MAGIC = 'FLSEEDP1';
export type PackFormat = 'flpack1' | 'flseedp1';
const ALIGN = 8;

export interface PackFile { path: string; data: Uint8Array }
export interface PackEntry { data: Uint8Array; sha256: string }

const align = (n: number): number => (n + ALIGN - 1) & ~(ALIGN - 1);

export function buildPack(files: readonly PackFile[]): Uint8Array {
  const seen = new Set<string>();
  const entries: { path: string; offset: number; size: number; sha256: string }[] = [];
  let off = 0;
  for (const f of files) {
    if (!isRelativePackPath(f.path)) throw new Error(`buildPack: bad path ${JSON.stringify(f.path)}`);
    if (seen.has(f.path)) throw new Error(`buildPack: duplicate path ${f.path}`);
    seen.add(f.path);
    entries.push({ path: f.path, offset: off, size: f.data.length, sha256: sha256HexSync(f.data) });
    off = align(off + f.data.length);
  }
  const indexBytes = new TextEncoder().encode(JSON.stringify({ files: entries }));
  const indexPadded = align(12 + indexBytes.length) - 12;
  const out = new Uint8Array(12 + indexPadded + off);
  const dv = new DataView(out.buffer);
  out.set(new TextEncoder().encode(PACK_MAGIC), 0);
  dv.setUint32(4, PACK_VERSION, true);
  dv.setUint32(8, indexPadded, true);
  out.set(indexBytes, 12);
  out.fill(0x20, 12 + indexBytes.length, 12 + indexPadded);
  const payload = 12 + indexPadded;
  files.forEach((f, i) => out.set(f.data, payload + (entries[i] as { offset: number }).offset));
  return out;
}

export class PackError extends Error {}

function parseFlpack1(bytes: Uint8Array, opts: { verifyFiles?: boolean }): Map<string, PackEntry> {
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const version = dv.getUint32(4, true);
  if (version !== PACK_VERSION) throw new PackError(`unsupported pack version ${version}`);
  const indexLen = dv.getUint32(8, true);
  if (12 + indexLen > bytes.length) throw new PackError('index runs past the end of the pack');
  let index: unknown;
  try { index = JSON.parse(new TextDecoder().decode(bytes.subarray(12, 12 + indexLen))); } catch (e) { throw new PackError(`index is not JSON: ${String(e)}`); }
  const files = (index as { files?: unknown }).files;
  if (!Array.isArray(files)) throw new PackError('index has no "files" array');
  const payload = 12 + indexLen;
  const payloadLen = bytes.length - payload;
  const out = new Map<string, PackEntry>();
  for (const e of files as Record<string, unknown>[]) {
    const { path, offset, size, sha256 } = e as { path: string; offset: number; size: number; sha256: string };
    checkEntry(path, offset, size, sha256, payloadLen, out);
    const data = bytes.subarray(payload + offset, payload + offset + size);
    if (opts.verifyFiles) {
      const got = sha256HexSync(data);
      if (got !== sha256) throw new PackError(`${path}: content SHA-256 ${got} does not match the index ${sha256}`);
    }
    out.set(path, { data, sha256 });
  }
  return out;
}

export function detectPackFormat(bytes: Uint8Array): PackFormat | null {
  if (bytes.length >= 8 && new TextDecoder().decode(bytes.subarray(0, 8)) === SEED_MAGIC) return 'flseedp1';
  if (bytes.length >= 4 && new TextDecoder().decode(bytes.subarray(0, 4)) === PACK_MAGIC) return 'flpack1';
  return null;
}

function checkEntry(path: unknown, offset: unknown, size: unknown, sha256: unknown, payloadLen: number, seen: Map<string, PackEntry>): void {
  if (typeof path !== 'string' || !isRelativePackPath(path)) throw new PackError(`bad path ${JSON.stringify(path)}`);
  if (!Number.isInteger(offset) || !Number.isInteger(size) || (offset as number) < 0 || (size as number) < 0) throw new PackError(`${path}: bad offset/size`);
  if ((offset as number) + (size as number) > payloadLen) throw new PackError(`${path}: runs past the end of the pack (offset ${String(offset)} size ${String(size)} payload ${payloadLen})`);
  if (typeof sha256 !== 'string' || !/^[0-9a-f]{64}$/.test(sha256)) throw new PackError(`${path}: bad sha256`);
  if (seen.has(path)) throw new PackError(`duplicate path ${path}`);
}

function parseSeedPack1(bytes: Uint8Array, opts: { verifyFiles?: boolean }): Map<string, PackEntry> {
  if (bytes.length < 12) throw new PackError(`pack too short (${bytes.length} bytes)`);
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const headerLen = dv.getUint32(8, true);
  if (12 + headerLen > bytes.length) throw new PackError('header runs past the end of the pack');
  let header: { version?: unknown; files?: unknown };
  try { header = JSON.parse(new TextDecoder().decode(bytes.subarray(12, 12 + headerLen))) as typeof header; } catch (e) { throw new PackError(`header is not JSON: ${String(e)}`); }
  if (header.version !== 1) throw new PackError(`unsupported flseedp1 version ${String(header.version)}`);
  if (!Array.isArray(header.files)) throw new PackError('header has no "files" array');
  const base = 12 + headerLen;
  const out = new Map<string, PackEntry>();
  for (const e of header.files as Record<string, unknown>[]) {
    checkEntry(e.path, e.offset, e.size, e.sha256, bytes.length - base, out);
    const path = e.path as string, off = base + (e.offset as number);
    const data = bytes.subarray(off, off + (e.size as number));
    if (opts.verifyFiles) {
      const got = sha256HexSync(data);
      if (got !== e.sha256) throw new PackError(`${path}: content SHA-256 ${got} does not match the index ${String(e.sha256)}`);
    }
    out.set(path, { data, sha256: e.sha256 as string });
  }
  return out;
}

/** Reads a pack of either format (the magic decides). `opts.format`: the format the manifest names; a pack of another format is refused. */
export function parsePack(bytes: Uint8Array, opts: { verifyFiles?: boolean; format?: PackFormat } = {}): Map<string, PackEntry> {
  if (bytes.length < 12) throw new PackError(`pack too short (${bytes.length} bytes)`);
  const fmt = detectPackFormat(bytes);
  if (fmt === null) throw new PackError(`bad magic ${JSON.stringify(new TextDecoder().decode(bytes.subarray(0, 8)))} (want ${PACK_MAGIC} or ${SEED_MAGIC})`);
  if (opts.format !== undefined && opts.format !== fmt) throw new PackError(`the manifest says ${opts.format} but the bytes are ${fmt}`);
  return fmt === 'flpack1' ? parseFlpack1(bytes, opts) : parseSeedPack1(bytes, opts);
}
