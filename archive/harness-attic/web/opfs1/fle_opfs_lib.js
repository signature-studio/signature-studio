// Stage 3 library of decision 0216 (opfs1, PROVISIONAL): the guest's files as OPFS files, read and written SYNCHRONOUSLY from the engine's Worker (FileSystemSyncAccessHandle), opened and listed through JSPI.
// An emscripten JS library like harness/web/filedlg/fle_filedlg_lib.js: link with -sJSPI --js-library <this file>; the C side is fle_opfs.h. Nothing here is wired to the engine's volume yet (lift/runtime/fle_vfs.c): this is the mechanism and its test.
//
// Why this shape (measured in the browsers, docs/0216-opfs-files-s3.md):
//  - opening is asynchronous (getFileHandle + createSyncAccessHandle: 1-5 ms) -> JSPI import (the engine is already built with -sJSPI; a suspension itself costs about 0.2 us in Chrome, 1 us in Firefox);
//  - read/write/flush/getSize/truncate are synchronous, but one call costs 0.04-0.2 ms in Firefox and 0.2-1 ms in Chrome on the test machine -> a block cache here turns the engine's small reads/writes into
//    calls of at least BLK bytes (read-ahead window, write-behind buffer);
//  - a sync access handle is EXCLUSIVE (a second createSyncAccessHandle on the file, createWritable, removeEntry and move fail with NoModificationAllowedError; getFile() reads still work) -> ONE handle per path,
//    shared by all guest handles of that path (refcount), closed when the last guest handle closes so that the page's file manager can touch the file again.
// Paths are UTF-8 '/'-separated, relative to the OPFS root (or to a subtree: fle_opfs_set_root). Errors are negative errno values (ENOENT 2, EIO 5, EACCES 13, EBUSY 16, EEXIST 17, ENOTDIR 20, EISDIR 21, EINVAL 22, ENOSPC 28).
addToLibrary({
  $FleOpfs__deps: ['$UTF8ToString', '$stringToUTF8', '$lengthBytesUTF8', 'malloc'],
  $FleOpfs: {
    rootDir: null, subtree: '', byPath: new Map(), fds: new Map(), nextFd: 3, BLK: 256 * 1024, stats: { opens: 0, reads: 0, writes: 0, directReads: 0, windowFills: 0, flushes: 0 },
    errno(e) {
      const n = e && e.name;
      if (n === 'NotFoundError') return -2;
      if (n === 'NoModificationAllowedError' || n === 'InvalidStateError') return -16;
      if (n === 'TypeMismatchError') return -21;
      if (n === 'QuotaExceededError') return -28;
      if (n === 'NotAllowedError' || n === 'SecurityError') return -13;
      if (n === 'TypeError') return -22;
      return -5;
    },
    parts(path) { return (FleOpfs.subtree ? FleOpfs.subtree + '/' + path : path).split('/').filter((s) => s !== '' && s !== '.'); },
    async root() { if (!FleOpfs.rootDir) FleOpfs.rootDir = await navigator.storage.getDirectory(); return FleOpfs.rootDir; },
    // Windows names are case-insensitive, OPFS names are not: a component is first tried exactly (one call), and only if that is NotFound the folder is listed for a name that differs in case only (FleOpfs.casefold = false turns it off)
    casefold: true, byLower: new Map(),
    // the entry of `dir` whose name differs from `name` in case only: {name, kind: 'file'|'directory'} or null
    async foldEntry(dir, name) {
      if (!FleOpfs.casefold) return null;
      const lower = name.toLowerCase();
      for await (const [n, h] of dir.entries()) if (n.toLowerCase() === lower) return { name: n, kind: h.kind };
      return null;
    },
    async fold(dir, name, wantDir) {
      const m = await FleOpfs.foldEntry(dir, name);
      return m && (wantDir === null || (m.kind === 'directory') === wantDir) ? m.name : null;
    },
    // [handle of the folder, its canonical (OPFS spelling) path parts]
    async dirOfC(parts, create) {
      let d = await FleOpfs.root();
      const canon = [];
      for (const s of parts) {
        let name = s;
        try { d = await d.getDirectoryHandle(s); }
        catch (x) {
          if (x.name !== 'NotFoundError') throw x;
          const f = await FleOpfs.fold(d, s, true);
          if (f !== null) { name = f; d = await d.getDirectoryHandle(f); } else d = await d.getDirectoryHandle(s, { create });
        }
        canon.push(name);
      }
      return [d, canon];
    },
    async dirOf(parts, create) { return (await FleOpfs.dirOfC(parts, create))[0]; },
    // the OPFS spelling of the entry `name` of `dir` ('file': a file, 'dir': a folder, null: either), or null when there is none
    async leaf(dir, name, kind) {
      try { if (kind === 'dir') await dir.getDirectoryHandle(name); else await dir.getFileHandle(name); return name; }
      catch (x) { if (x.name === 'TypeMismatchError') { if (kind === null) return name; } else if (x.name !== 'NotFoundError') throw x; }
      if (kind === null) { try { await dir.getDirectoryHandle(name); return name; } catch (x) { if (x.name !== 'NotFoundError' && x.name !== 'TypeMismatchError') throw x; } }
      return FleOpfs.fold(dir, name, kind === null ? null : kind === 'dir');
    },
    key(path) { return FleOpfs.parts(path).join('/'); },
    // ---- the block cache of one open file ----
    flushDirty(e) {
      if (e.dirty && e.dirty.len > 0) { e.h.write(e.dirty.buf.subarray(0, e.dirty.len), { at: e.dirty.start }); e.wrote = true; FleOpfs.stats.flushes++; }
      e.dirty = null;
    },
    read(e, dst, at) {
      // dst: a Uint8Array view of the guest memory; returns the number of bytes read (0 at end of file)
      FleOpfs.stats.reads++;
      if (e.dirty) FleOpfs.flushDirty(e);
      let n = Math.min(dst.length, Math.max(0, e.size - at));
      if (n <= 0) return 0;
      if (n >= FleOpfs.BLK) { FleOpfs.stats.directReads++; return e.h.read(dst.subarray(0, n), { at }); }
      let done = 0;
      // read-ahead that adapts: a read that continues where the previous one ended is sequential, so the window doubles (16 KiB ... BLK); any other read gets a small window (16 KiB), so that random 4 KiB reads do not pay for BLK bytes
      const sequential = at === e.lastEnd;
      while (done < n) {
        const pos = at + done, w = e.win;
        if (w && pos >= w.start && pos < w.start + w.len) {
          const c = Math.min(n - done, w.start + w.len - pos);
          dst.set(w.buf.subarray(pos - w.start, pos - w.start + c), done);
          done += c;
        } else {
          const size = sequential || done > 0 ? Math.min(FleOpfs.BLK, Math.max(16384, (e.winSize || 0) * 2)) : 16384;
          const start = Math.floor(pos / 4096) * 4096;
          const buf = w ? w.buf : new Uint8Array(FleOpfs.BLK);
          const len = e.h.read(buf.subarray(0, Math.min(size, Math.max(0, e.size - start))), { at: start });
          FleOpfs.stats.windowFills++;
          e.winSize = size;
          e.win = { start, len, buf };
          if (len === 0 || pos >= start + len) break;
        }
      }
      e.lastEnd = at + done;
      return done;
    },
    write(e, src, at) {
      FleOpfs.stats.writes++;
      e.win = null;
      const n = src.length;
      if (n >= FleOpfs.BLK) { FleOpfs.flushDirty(e); const w = e.h.write(src, { at }); e.wrote = true; e.size = Math.max(e.size, at + w); return w; }
      const d = e.dirty;
      if (d && at === d.start + d.len && d.len + n <= FleOpfs.BLK) { d.buf.set(src, d.len); d.len += n; }
      else { FleOpfs.flushDirty(e); const buf = new Uint8Array(FleOpfs.BLK); buf.set(src, 0); e.dirty = { start: at, len: n, buf }; }
      e.size = Math.max(e.size, at + n);
      return n;
    },
    // flush (fsync) only a file that was written since the last one; Windows does not fsync at CloseHandle either, so close() does it only with fsyncOnClose
    fsyncOnClose: false,
    sync(e) { FleOpfs.flushDirty(e); if (e.wrote) { e.h.flush(); e.wrote = false; } },
  },

  // fle_opfs_set_root("") = the OPFS root; fle_opfs_set_root("fl/C") = every path is relative to /fl/C (created if missing)
  fle_opfs_set_root__deps: ['$FleOpfs'],
  fle_opfs_set_root__sig: 'ip',
  fle_opfs_set_root__async: true,
  fle_opfs_set_root: async (pathPtr) => {
    try { FleOpfs.subtree = UTF8ToString(pathPtr).split('/').filter(Boolean).join('/'); FleOpfs.rootDir = null; await FleOpfs.dirOf(FleOpfs.parts(''), true); return 0; } catch (e) { return FleOpfs.errno(e); }
  },

  // flags: 1 read, 2 write, 4 create, 8 truncate, 16 must not exist. Returns a descriptor >= 3 or -errno.
  fle_opfs_open__deps: ['$FleOpfs'],
  fle_opfs_open__sig: 'ipi',
  fle_opfs_open__async: true,
  fle_opfs_open: async (pathPtr, flags) => {
    try {
      const path = UTF8ToString(pathPtr), parts = FleOpfs.parts(path);
      if (parts.length === 0) return -21;
      const asked = parts.join('/');
      let e = FleOpfs.byPath.get(asked) || (FleOpfs.casefold ? FleOpfs.byLower.get(asked.toLowerCase()) : undefined);
      if (e && (flags & 16) !== 0) return -17;
      if (!e) {
        const [dir, canon] = await FleOpfs.dirOfC(parts.slice(0, -1), (flags & 4) !== 0);
        const asked_name = parts[parts.length - 1];
        let name = asked_name, fh = null;
        try { fh = await dir.getFileHandle(asked_name); } catch (x) { if (x.name === 'TypeMismatchError') return -21; if (x.name !== 'NotFoundError') throw x; }
        if (fh === null) {
          const m = await FleOpfs.foldEntry(dir, asked_name);
          if (m) { if (m.kind === 'directory') return -21; name = m.name; fh = await dir.getFileHandle(name); }
        }
        if (fh !== null && (flags & 16) !== 0) return -17;
        const key = canon.concat(name).join('/');
        if (fh === null) fh = await dir.getFileHandle(name, { create: (flags & 4) !== 0 });
        e = FleOpfs.byPath.get(key);   // another guest thread may have opened it while we waited
        if (!e) {
          const h = await fh.createSyncAccessHandle();
          e = { key, h, refs: 0, size: h.getSize(), win: null, dirty: null, dir, name };
          FleOpfs.byPath.set(key, e);
          FleOpfs.byLower.set(key.toLowerCase(), e);
          FleOpfs.stats.opens++;
        }
      }
      if ((flags & 8) !== 0) { FleOpfs.flushDirty(e); e.h.truncate(0); e.wrote = true; e.size = 0; e.win = null; }
      e.refs++;
      const fd = FleOpfs.nextFd++;
      FleOpfs.fds.set(fd, e);
      return fd;
    } catch (x) { return FleOpfs.errno(x); }
  },

  fle_opfs_close__deps: ['$FleOpfs'],
  fle_opfs_close__sig: 'ii',
  fle_opfs_close: (fd) => {
    const e = FleOpfs.fds.get(fd);
    if (!e) return -9;
    FleOpfs.fds.delete(fd);
    if (--e.refs === 0) { try { if (FleOpfs.fsyncOnClose) FleOpfs.sync(e); else FleOpfs.flushDirty(e); e.h.close(); } catch (x) { FleOpfs.byPath.delete(e.key); FleOpfs.byLower.delete(e.key.toLowerCase()); return FleOpfs.errno(x); } FleOpfs.byPath.delete(e.key); FleOpfs.byLower.delete(e.key.toLowerCase()); }
    return 0;
  },

  fle_opfs_read__deps: ['$FleOpfs'],
  fle_opfs_read__sig: 'iipid',
  fle_opfs_read: (fd, bufPtr, n, at) => {
    const e = FleOpfs.fds.get(fd);
    if (!e) return -9;
    try { return FleOpfs.read(e, HEAPU8.subarray(bufPtr, bufPtr + n), at); } catch (x) { return FleOpfs.errno(x); }
  },

  fle_opfs_write__deps: ['$FleOpfs'],
  fle_opfs_write__sig: 'iipid',
  fle_opfs_write: (fd, bufPtr, n, at) => {
    const e = FleOpfs.fds.get(fd);
    if (!e) return -9;
    try { return FleOpfs.write(e, HEAPU8.subarray(bufPtr, bufPtr + n), at); } catch (x) { return FleOpfs.errno(x); }
  },

  fle_opfs_flush__deps: ['$FleOpfs'],
  fle_opfs_flush__sig: 'ii',
  fle_opfs_flush: (fd) => { const e = FleOpfs.fds.get(fd); if (!e) return -9; try { FleOpfs.sync(e); return 0; } catch (x) { return FleOpfs.errno(x); } },

  fle_opfs_truncate__deps: ['$FleOpfs'],
  fle_opfs_truncate__sig: 'iid',
  fle_opfs_truncate: (fd, size) => { const e = FleOpfs.fds.get(fd); if (!e) return -9; try { FleOpfs.flushDirty(e); e.h.truncate(size); e.wrote = true; e.size = size; e.win = null; return 0; } catch (x) { return FleOpfs.errno(x); } },

  // the size the guest sees: what was written, flushed or not
  fle_opfs_size__deps: ['$FleOpfs'],
  fle_opfs_size__sig: 'di',
  fle_opfs_size: (fd) => { const e = FleOpfs.fds.get(fd); return e ? e.size : -9; },

  // kind: 1 file, 2 directory, 0 missing (-errno on error); size and mtime(ms) are stored at the two doubles (the size of a file that is open is the guest's)
  fle_opfs_stat__deps: ['$FleOpfs'],
  fle_opfs_stat__sig: 'ipp',
  fle_opfs_stat__async: true,
  fle_opfs_stat: async (pathPtr, outPtr) => {
    try {
      const parts = FleOpfs.parts(UTF8ToString(pathPtr));
      const put = (size, mtime) => { const f = new Float64Array(HEAPU8.buffer, outPtr, 2); f[0] = size; f[1] = mtime; };
      if (parts.length === 0) { put(0, 0); return 2; }
      const [dir, canon] = await FleOpfs.dirOfC(parts.slice(0, -1), false);
      const asked = parts[parts.length - 1], file = await FleOpfs.leaf(dir, asked, 'file');
      if (file !== null) { const open = FleOpfs.byPath.get(canon.concat(file).join('/')); const f = await (await dir.getFileHandle(file)).getFile(); put(open ? open.size : f.size, f.lastModified); return 1; }
      if (await FleOpfs.leaf(dir, asked, 'dir') !== null) { put(0, 0); return 2; }
      return 0;
    } catch (x) { return x && x.name === 'NotFoundError' ? 0 : FleOpfs.errno(x); }
  },

  // a malloc'd UTF-8 JSON text [[name, kind(1|2), size, mtime], ...] sorted by name (case-insensitive); NULL on error (errno in *errPtr)
  fle_opfs_list__deps: ['$FleOpfs'],
  fle_opfs_list__sig: 'ppp',
  fle_opfs_list__async: true,
  fle_opfs_list: async (pathPtr, errPtr) => {
    try {
      const [d, canon] = await FleOpfs.dirOfC(FleOpfs.parts(UTF8ToString(pathPtr)), false), rows = [], pending = [];
      for await (const [name, h] of d.entries()) {
        if (h.kind === 'file') pending.push((async () => { const f = await h.getFile(); const o = FleOpfs.byPath.get(canon.concat(name).join('/')); rows.push([name, 1, o ? o.size : f.size, f.lastModified]); })());
        else rows.push([name, 2, 0, 0]);
      }
      await Promise.all(pending);
      rows.sort((a, b) => (a[0].toLowerCase() < b[0].toLowerCase() ? -1 : a[0].toLowerCase() > b[0].toLowerCase() ? 1 : 0));
      const text = JSON.stringify(rows), n = lengthBytesUTF8(text) + 1, p = _malloc(n);
      stringToUTF8(text, p, n);
      return p;
    } catch (x) { HEAP32[errPtr >> 2] = FleOpfs.errno(x); return 0; }
  },

  // names only: a malloc'd UTF-8 JSON text [[name, kind(1|2)], ...] in the order the browser gives; no getFile() per file (that is most of the cost of fle_opfs_list); a wildcard search can filter by name first and fle_opfs_stat only the matches
  fle_opfs_list_names__deps: ['$FleOpfs'],
  fle_opfs_list_names__sig: 'ppp',
  fle_opfs_list_names__async: true,
  fle_opfs_list_names: async (pathPtr, errPtr) => {
    try {
      const [d] = await FleOpfs.dirOfC(FleOpfs.parts(UTF8ToString(pathPtr)), false), rows = [];
      for await (const [name, h] of d.entries()) rows.push([name, h.kind === 'file' ? 1 : 2]);
      const text = JSON.stringify(rows), n = lengthBytesUTF8(text) + 1, p = _malloc(n);
      stringToUTF8(text, p, n);
      return p;
    } catch (x) { HEAP32[errPtr >> 2] = FleOpfs.errno(x); return 0; }
  },

  fle_opfs_mkdir__deps: ['$FleOpfs'],
  fle_opfs_mkdir__sig: 'ip',
  fle_opfs_mkdir__async: true,
  fle_opfs_mkdir: async (pathPtr) => { try { await FleOpfs.dirOf(FleOpfs.parts(UTF8ToString(pathPtr)), true); return 0; } catch (x) { return FleOpfs.errno(x); } },

  fle_opfs_remove__deps: ['$FleOpfs'],
  fle_opfs_remove__sig: 'ip',
  fle_opfs_remove__async: true,
  fle_opfs_remove: async (pathPtr) => {
    try {
      const parts = FleOpfs.parts(UTF8ToString(pathPtr));
      if (parts.length === 0) return -22;
      const d = await FleOpfs.dirOf(parts.slice(0, -1), false);
      const name = await FleOpfs.leaf(d, parts[parts.length - 1], null);
      if (name === null) return -2;
      await d.removeEntry(name, { recursive: false });
      return 0;
    } catch (x) { return x && x.name === 'InvalidModificationError' ? -39 : FleOpfs.errno(x); }
  },

  // rename within OPFS (FileSystemHandle.move: Chrome 102+, Firefox 111+); a file that is open cannot be moved (EBUSY). flags & 1: replace an existing destination (MOVEFILE_REPLACE_EXISTING); otherwise EEXIST.
  // The same entry under another case (a.txt -> A.txt) is a rename, not a collision.
  fle_opfs_rename__deps: ['$FleOpfs'],
  fle_opfs_rename__sig: 'ippi',
  fle_opfs_rename__async: true,
  fle_opfs_rename: async (fromPtr, toPtr, flags) => {
    try {
      const a = FleOpfs.parts(UTF8ToString(fromPtr)), b = FleOpfs.parts(UTF8ToString(toPtr));
      if (a.length === 0 || b.length === 0) return -22;
      const [da, ca] = await FleOpfs.dirOfC(a.slice(0, -1), false), [db, cb] = await FleOpfs.dirOfC(b.slice(0, -1), true);
      const from = await FleOpfs.leaf(da, a[a.length - 1], null);
      if (from === null) return -2;
      let h;
      try { h = await da.getFileHandle(from); } catch (x) { if (x.name !== 'TypeMismatchError') throw x; h = await da.getDirectoryHandle(from); }
      const src = ca.concat(from).join('/');
      if (FleOpfs.byPath.has(src)) return -16;
      const to = await FleOpfs.leaf(db, b[b.length - 1], null);
      const same = to !== null && cb.concat(to).join('/') === src;
      if (to !== null && !same) {
        if ((flags & 1) === 0) return -17;
        if (FleOpfs.byPath.has(cb.concat(to).join('/'))) return -16;
        if (h.kind === 'directory') return -17;
      }
      await h.move(db, same || to === null ? b[b.length - 1] : to);   // onto another entry that differs in case only: that entry's spelling
      return 0;
    } catch (x) { return FleOpfs.errno(x); }
  },

  // flush every open file (cheap when nothing is dirty): for a periodic call, so that a killed Worker loses at most the interval
  fle_opfs_flush_all__deps: ['$FleOpfs'],
  fle_opfs_flush_all__sig: 'i',
  fle_opfs_flush_all: () => { let n = 0; for (const e of FleOpfs.byPath.values()) { try { if (e.dirty) n++; FleOpfs.sync(e); } catch (x) { return FleOpfs.errno(x); } } return n; },
  fle_opfs_set_fsync_on_close__deps: ['$FleOpfs'],
  fle_opfs_set_fsync_on_close__sig: 'vi',
  fle_opfs_set_fsync_on_close: (on) => { FleOpfs.fsyncOnClose = on !== 0; },
  fle_opfs_set_casefold__deps: ['$FleOpfs'],
  fle_opfs_set_casefold__sig: 'vi',
  fle_opfs_set_casefold: (on) => { FleOpfs.casefold = on !== 0; },

  // counters of the cache, as a JSON text (malloc'd), for the tests
  fle_opfs_stats__deps: ['$FleOpfs'],
  fle_opfs_stats__sig: 'p',
  fle_opfs_stats: () => { const text = JSON.stringify({ ...FleOpfs.stats, openFiles: FleOpfs.byPath.size, blk: FleOpfs.BLK }), n = lengthBytesUTF8(text) + 1, p = _malloc(n); stringToUTF8(text, p, n); return p; },
  fle_opfs_set_block__deps: ['$FleOpfs'],
  fle_opfs_set_block__sig: 'vi',
  fle_opfs_set_block: (n) => { FleOpfs.BLK = n; },
  fle_opfs_now__sig: 'd',
  fle_opfs_now: () => performance.now(),
});
