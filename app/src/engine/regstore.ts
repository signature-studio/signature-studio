// Issue #1383: the page keeps the virtual registry's HKCU tree (lift/runtime/fle_reg.c). The records are one file in the browser's OPFS, /fl/state/hkcu.log: the page's origin, so it is
// kept when the page is reloaded or closed, and removed with the site's data. Only the engine's Worker opens it: a synchronous access handle, flushed after every record, so a record
// is on the disk when the guest's call returns. Nothing leaves the browser (no beacon, no request carries the bytes).
// Issue #1385: the same file keeps the user's files under the profile (C:\Users\user: the records filePut, dirMake and fileDel of reg-journal.ts). The name is kept from #1383.
// The file holds the live records (reg-journal.ts). It is rewritten (compacted) when it is over 4 MiB and over twice the live records, and it never grows past REG_STATE_LIMIT.
// /fl/state is a PROTECTED path of the file manager (paths.ts): the file manager cannot change it.

import { parseRecord, RegJournal } from './reg-journal.ts';

export const REG_STATE_DIR: readonly string[] = ['fl', 'state'];
export const REG_STATE_FILE = 'hkcu.log';
/** the most bytes the saved records may take (Issue #1383: 100 MB) */
export const REG_STATE_LIMIT = 100 * 1024 * 1024;
const COMPACT_MIN = 4 * 1024 * 1024;
const ACQUIRE_TRIES = 50, ACQUIRE_WAIT_MS = 100;

interface SyncAccess {
  getSize(): number;
  read(buf: Uint8Array, o: { at: number }): number;
  write(buf: Uint8Array, o: { at: number }): number;
  truncate(size: number): void;
  flush(): void;
  close(): void;
}
interface StateFile { createSyncAccessHandle(): Promise<SyncAccess> }
interface StateDir {
  getDirectoryHandle(name: string, o?: { create?: boolean }): Promise<StateDir>;
  getFileHandle(name: string, o?: { create?: boolean }): Promise<StateFile>;
}

export interface RegState {
  /** the records the engine replays before the guest runs (read once, when the store was opened) */
  readonly saved: Uint8Array;
  /** the bytes of the file now */
  readonly bytes: number;
  /** one change (one record of lift/runtime/fle_reg.c): checked, appended and flushed; false when it was not kept (the reason is logged) */
  put(record: Uint8Array): boolean;
  /** a run that stopped with an error drops the saved records: the saved registry may be what stopped it, and the next start must not stop the same way (it begins from the seed) */
  discard(): void;
  close(): void;
}

/** the access to the file: the browser refuses it while an earlier page's Worker still holds it (a reload), so it is asked again for a few seconds */
async function acquire(f: StateFile): Promise<SyncAccess> {
  for (let tries = 0; ; tries++) {
    try {
      return await f.createSyncAccessHandle();
    } catch (e) {
      if (tries >= ACQUIRE_TRIES) throw e;
      await new Promise((r) => setTimeout(r, ACQUIRE_WAIT_MS));
    }
  }
}

/** opens the saved registry of this page; null (with the reason logged) when the browser keeps no such file: the registry then starts from the seed, as before */
export async function openRegState(log: (level: 'info' | 'warn', text: string) => void): Promise<RegState | null> {
  const storage = (globalThis.navigator as unknown as { storage?: { getDirectory?: () => Promise<unknown> } } | undefined)?.storage;
  if (!storage?.getDirectory) {
    log('warn', 'the registry is not kept: this browser gives the page no file storage (OPFS)');
    return null;
  }
  let h: SyncAccess;
  try {
    let dir = (await storage.getDirectory()) as StateDir;
    for (const s of REG_STATE_DIR) dir = await dir.getDirectoryHandle(s, { create: true });
    h = await acquire(await dir.getFileHandle(REG_STATE_FILE, { create: true }));
  } catch (e) {
    log('warn', `the registry is not kept: ${String((e as Error)?.message ?? e).slice(0, 200)}`);
    return null;
  }
  const size = h.getSize();
  const buf = new Uint8Array(size);
  if (size > 0) h.read(buf, { at: 0 });
  const j = new RegJournal();
  const loaded = j.load(buf);
  let bytes = size;
  const rewrite = (): void => {
    const img = j.image();
    h.truncate(0);
    h.write(img, { at: 0 });
    h.flush();
    bytes = img.length;
  };
  if (loaded.dropped > 0 || (bytes > COMPACT_MIN && bytes > 2 * j.liveBytes)) rewrite();
  const saved = j.image();
  let broken = false, warned = false;
  const refuse = (why: string): false => {
    if (!warned) { warned = true; log('warn', `a change of the registry is not kept: ${why}`); }
    return false;
  };
  return {
    saved,
    get bytes() { return bytes; },
    put(record: Uint8Array): boolean {
      if (broken) return false;
      const r = parseRecord(record, 0);
      if (r === null || r.end !== record.length) return refuse('the record is malformed');
      if (bytes + r.bytes.length > REG_STATE_LIMIT && j.liveBytes + r.bytes.length <= REG_STATE_LIMIT) rewrite();
      if (bytes + r.bytes.length > REG_STATE_LIMIT || j.liveBytes + r.bytes.length > REG_STATE_LIMIT) return refuse(`the saved registry would be over ${REG_STATE_LIMIT} bytes`);
      try {
        h.write(r.bytes, { at: bytes });
        h.flush();
      } catch (e) {
        broken = true;
        log('warn', `the registry is not kept any more: ${String((e as Error)?.message ?? e).slice(0, 200)}`);
        return false;
      }
      bytes += r.bytes.length;
      j.add(r);
      if (bytes > COMPACT_MIN && bytes > 2 * j.liveBytes) rewrite();
      return true;
    },
    discard(): void {
      try {
        h.truncate(0);
        h.flush();
        bytes = 0;
      } catch { /* nothing to drop */ }
    },
    close(): void {
      try { h.close(); } catch { /* already closed */ }
    },
  };
}
