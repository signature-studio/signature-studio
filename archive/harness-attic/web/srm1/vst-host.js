// srm1 (decision 0177): mt1's host (decision 0173) with srm1_thread_cpu_s (the CPU clock of lift/harness/vst3a/wasm_cpuclock.c: wall time in a Worker), a 3.75 GiB memory limit (the build's --max-memory),
// memBytes(), and the unimplemented syscalls reported by name instead of a TypeError. Original text: mt1 (decision 0173): a minimal browser host for the emscripten-built OTT VST3 harness wasm (lift/harness/vst/build_vst_wasm.sh, built for node with NODERAWFS).
// The 22 imports of that wasm are the emscripten syscall/WASI set plus the fle_js_* audio hooks, which the harness never calls; this file implements them without the
// emscripten glue (no node, no FS): an in-memory file table (the plugin file and the VFS seed list), stdout capture, wall-clock time for every clock id
// (a browser Worker has no thread CPU clock; the CPU time of a Worker is read from /proc by the runner).
// Used from a Worker (vst-worker.js) and from node (self-check). Nothing here changes the harness or the lifted code.
export function createHost({ files, args, env, print, hooks = {} }) {
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  let inst = null, mem = null;
  const fds = new Map(); // fd -> {name, data, pos}
  let nextFd = 10;
  const u8 = () => new Uint8Array(mem.buffer);
  const dv = () => new DataView(mem.buffer);
  const cstr = (p) => { const m = u8(); let e = p; while (m[e]) e++; return dec.decode(m.subarray(p, e)); };
  let stdout = '';
  const flushLine = () => { let i; while ((i = stdout.indexOf('\n')) >= 0) { const l = stdout.slice(0, i); stdout = stdout.slice(i + 1); print(l); if (hooks.line) hooks.line(l); } };
  const now = () => globalThis.performance.now();
  const ENOENT = 44, EBADF = 8, EINVAL = 28, ENOTTY = 59;
  const imports = {
    env: {
      exit: (c) => { throw new Error('exit ' + c); },
      _abort_js: () => { throw new Error('abort'); },
      srm1_thread_cpu_s: () => globalThis.performance.now() * 1e-3,
      fle_js_init: () => 0, fle_js_now: () => 0, fle_js_consumed: () => 0n, fle_js_unlock: () => 0, fle_js_state: () => 0, fle_js_lead_ms: () => 0, fle_js_publish: () => 0,
      emscripten_resize_heap: (req) => {
        req >>>= 0;
        const max = 0xF0000000; if (req > max) return 0;
        const old = mem.buffer.byteLength;
        const target = Math.min(max, Math.ceil(Math.max(req, old * 1.2) / 65536) * 65536);
        try { inst.exports.memory.grow((target - old) / 65536); return 1; } catch { return 0; }
      },
      __syscall_openat: (dirfd, pathp, flags) => {
        const name = cstr(pathp);
        const data = files[name];
        if (!data) { (globalThis.__missing = globalThis.__missing || []).push(name); return -ENOENT; }
        const fd = nextFd++;
        fds.set(fd, { name, data, pos: 0 });
        return fd;
      },
      __syscall_fcntl64: () => 0,
      __syscall_ioctl: (fd, op) => ((fd === 1 || fd === 2) && op === 0x5413 ? 0 : -ENOTTY),   // TIOCGWINSZ answers 'a terminal' on stdout/stderr: stdio becomes line buffered, lines reach print() as they are written
      __syscall_stat64: (pathp, buf) => {
        const name = cstr(pathp), data = files[name];
        if (!data) return -ENOENT;
        const d = dv();
        for (let i = 0; i < 112; i += 4) d.setUint32(buf + i, 0, true);
        d.setUint32(buf + 4, 0o100644, true); d.setUint32(buf + 8, 1, true); d.setBigInt64(buf + 24, BigInt(data.length), true); d.setInt32(buf + 32, 4096, true);
        return 0;
      },
    },
    wasi_snapshot_preview1: {
      clock_time_get: (id, prec, p) => { if (hooks.clock) hooks.clock(id); const t = id === 0 ? Date.now() * 1e6 : now() * 1e6; dv().setBigUint64(p, BigInt(Math.round(t)), true); return 0; },
      fd_close: (fd) => { fds.delete(fd); return 0; },
      fd_write: (fd, iov, cnt, pnum) => {
        const d = dv(), m = u8(); let n = 0;
        for (let i = 0; i < cnt; i++) {
          const p = d.getUint32(iov + 8 * i, true), l = d.getUint32(iov + 8 * i + 4, true);
          if (fd === 1 || fd === 2) { stdout += dec.decode(m.subarray(p, p + l)); }
          n += l;
        }
        if (fd === 1 || fd === 2) flushLine();
        d.setUint32(pnum, n, true);
        return 0;
      },
      fd_read: (fd, iov, cnt, pnum) => {
        const f = fds.get(fd); if (!f) return EBADF;
        const d = dv(), m = u8(); let n = 0;
        for (let i = 0; i < cnt; i++) {
          const p = d.getUint32(iov + 8 * i, true), l = d.getUint32(iov + 8 * i + 4, true);
          const k = Math.min(l, f.data.length - f.pos);
          if (k > 0) { m.set(f.data.subarray(f.pos, f.pos + k), p); f.pos += k; n += k; }
          if (k < l) break;
        }
        d.setUint32(pnum, n, true);
        return 0;
      },
      fd_pread: (fd, iov, cnt, off, pnum) => {
        const f = fds.get(fd); if (!f) return EBADF;
        const save = f.pos; f.pos = Number(off);
        const r = imports.wasi_snapshot_preview1.fd_read(fd, iov, cnt, pnum);
        f.pos = save;
        return r;
      },
      fd_seek: (fd, off, whence, pnew) => {
        const f = fds.get(fd); if (!f) return EBADF;
        const o = Number(off);
        const np = whence === 0 ? o : whence === 1 ? f.pos + o : f.data.length + o;
        if (np < 0) return EINVAL;
        f.pos = np; dv().setBigInt64(pnew, BigInt(np), true);
        return 0;
      },
      environ_sizes_get: (pc, pb) => {
        const d = dv(); d.setUint32(pc, env.length, true);
        d.setUint32(pb, env.reduce((s, e) => s + enc.encode(e).length + 1, 0), true);
        return 0;
      },
      environ_get: (pp, pb) => {
        const d = dv(), m = u8(); let b = pb;
        env.forEach((e, i) => { d.setUint32(pp + 4 * i, b, true); const bytes = enc.encode(e); m.set(bytes, b); m[b + bytes.length] = 0; b += bytes.length + 1; });
        return 0;
      },
    },
  };
  return {
    imports,
    attach(instance) { inst = instance; mem = instance.exports.memory; },
    run() {
      const x = inst.exports;
      x.__wasm_call_ctors();
      const all = ['./this.program', ...args];
      const argv = x._emscripten_stack_alloc((all.length + 1) * 4);
      all.forEach((a, i) => { const b = enc.encode(a + '\0'); const p = x._emscripten_stack_alloc(b.length); u8().set(b, p); dv().setUint32(argv + 4 * i, p, true); });
      dv().setUint32(argv + 4 * all.length, 0, true);
      const rc = x.__main_argc_argv(all.length, argv);
      try { x.fflush(0); } catch { /* ignore */ }
      if (stdout) { print(stdout); stdout = ''; }
      return rc;
    },
    missing: () => globalThis.__missing || [],
    memBytes: () => (mem ? mem.buffer.byteLength : 0),
  };
}
