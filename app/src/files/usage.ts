// What the page counts about its use of the file manager and of the engine's file dialogs (decision 0216, opfs1): counts only, for the measurement beacon (src/perf/beacon.ts `files` block, optional).
// No file or folder name, no path, no size of a single file, no content, no time of day: whole numbers, MiB rounded down, true/false, and failure CODES from a fixed list (never the text of an error: it can hold a name).
// Why: the maintainers cannot see the client's browser; the beacon is how a Firefox 159 / Windows 10 run of Files (the real folder chooser, the real drop of a folder, persist()) becomes visible at all.

import { PERF_FILES_FAIL_CODES } from '../../shared/perf.ts';

export const FILES_FAIL_CODES = PERF_FILES_FAIL_CODES;
export type FailCode = (typeof FILES_FAIL_CODES)[number];

export interface FilesUsage {
  v: 1;
  /** the Files window was opened with the header button */
  opens: number;
  /** the three upload buttons: the browser's chooser returned something */
  pick: { files: number; folder: number; zip: number };
  /** drops on the window: one with loose files, one with a folder (a path with a '/'), one that held a zip */
  drop: { files: number; folders: number; zips: number };
  /** files over all uploads, extractions and pastes */
  written: number; skipped: number; failed: number;
  writtenMiB: number;
  extract: { zips: number; files: number; cancelled: number };
  download: { file: number; folderZip: number };
  ops: { mkdir: number; rename: number; delete: number; copy: number; move: number };
  /** navigator.storage.persist(): asked, answered true / false, still unanswered after 3 s (Firefox waits for the person) */
  persist: { asked: number; granted: number; denied: number; pending: number };
  /** the last navigator.storage.estimate() and persisted(): MiB, null when the browser did not say */
  storage: { usedMiB: number | null; quotaMiB: number | null; persisted: boolean | null };
  /** the engine's file dialogs answered by the window */
  engine: { open: number; save: number; folder: number; answered: number; cancelled: number; zipUnpacked: number; savedBack: number; savedBackFailed: number };
  /** failure codes -> how many times */
  fail: Partial<Record<FailCode, number>>;
}

/** the text of an error or of a failure reason -> one of the fixed codes (the text itself never leaves the page) */
export function failCode(x: unknown): FailCode {
  const t = typeof x === 'string' ? x : `${(x as { name?: string })?.name ?? ''} ${(x as { message?: string })?.message ?? ''}`;
  if (/CRC-32/i.test(t)) return 'zip-crc';
  if (/compression method/i.test(t)) return 'zip-method';
  if (/encrypted/i.test(t)) return 'zip-encrypted';
  if (/unsafe path/i.test(t)) return 'unsafe-path';
  if (/not a zip|end-of-central|zip64|central directory|local header|too small|runs past/i.test(t)) return 'zip-format';
  if (/size \d+ instead of/i.test(t)) return 'zip-size';
  if (/NoModificationAllowed|in use/i.test(t)) return 'locked';
  if (/Quota|room left|not enough room/i.test(t)) return 'quota';
  if (/NotFound|not found|does not exist|is gone/i.test(t)) return 'notfound';
  if (/TypeMismatch|file and a folder|in the way/i.test(t)) return 'kind';
  if (/cannot contain|name is empty|reserved device|longer than|cannot end with|is not a name|would not accept/i.test(t)) return 'name';
  if (/already|InvalidModification/i.test(t)) return 'conflict';
  if (/cancel/i.test(t)) return 'cancelled';
  if (/DecompressionStream|unsupported|not supported/i.test(t)) return 'unsupported';
  if (/Security|NotAllowed/i.test(t)) return 'security';
  if (/NetworkError|NotReadable|Abort|DataError|EncodingError|Invalid/i.test(t)) return 'io';
  return 'other';
}

const zero = (): FilesUsage => ({
  v: 1, opens: 0, pick: { files: 0, folder: 0, zip: 0 }, drop: { files: 0, folders: 0, zips: 0 }, written: 0, skipped: 0, failed: 0, writtenMiB: 0,
  extract: { zips: 0, files: 0, cancelled: 0 }, download: { file: 0, folderZip: 0 }, ops: { mkdir: 0, rename: 0, delete: 0, copy: 0, move: 0 },
  persist: { asked: 0, granted: 0, denied: 0, pending: 0 }, storage: { usedMiB: null, quotaMiB: null, persisted: null },
  engine: { open: 0, save: 0, folder: 0, answered: 0, cancelled: 0, zipUnpacked: 0, savedBack: 0, savedBackFailed: 0 }, fail: {},
});

export class UsageCounter {
  private u: FilesUsage = zero();
  private bytes = 0;
  private used = false;

  /** the counters are reachable for the code that counts; every change marks the page as having used Files */
  get c(): FilesUsage { this.used = true; return this.u; }
  fail(x: unknown): void { const k = failCode(x); const f = this.c.fail; f[k] = (f[k] ?? 0) + 1; }
  /** an operation's outcome (upload, extract, paste): files written / skipped / failed, bytes, and the failure codes */
  result(r: { written: number; skipped: number; failed: { reason: string }[]; bytes: number }): void {
    const c = this.c;
    c.written += r.written; c.skipped += r.skipped; c.failed += r.failed.length;
    for (const f of r.failed) this.fail(f.reason);
    this.bytes += r.bytes;
    c.writtenMiB = Math.floor(this.bytes / 1048576);
  }
  storage(usage: number | null, quota: number | null, persisted: boolean | null): void {
    const s = this.c.storage;
    s.usedMiB = usage === null ? null : Math.floor(usage / 1048576);
    s.quotaMiB = quota === null ? null : Math.floor(quota / 1048576);
    s.persisted = persisted;
  }
  /** null until Files (or an engine dialog) was used: a page that never touched it sends no block */
  snapshot(): FilesUsage | null { return this.used ? (structuredClone(this.u) as FilesUsage) : null; }
  reset(): void { this.u = zero(); this.bytes = 0; this.used = false; }
}

/** the page's one counter */
export const filesUsage = new UsageCounter();
