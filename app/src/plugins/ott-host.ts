// A browser / node host for the emscripten-built VST3 harness wasm of a lifted plugin (lift/harness/vst, decision 0156/0164; the external-input variant is
// lift/harness/web/mt1/plugin-worker/main_vst_ext.c, decision 0173). The wasm has no JS glue here: its 24 imports (emscripten syscalls, WASI, two mt1_ext_* hooks, the fle_js_* audio hooks the harness
// never calls) are implemented below on an in-memory file table, with stdout captured as lines and every clock answering with the host's monotonic clock (a Worker has no thread-CPU clock).
// The Worker (plugin-worker.ts) is the only user; the file is environment-neutral (browser Worker or node worker_threads).

export interface ExtHooks {
  /** The harness asks for the next input block: copy it to the two float buffers in wasm memory (inL, inR are byte addresses). Return 1 to continue, 0 to leave the loop. */
  wait(mem: WebAssembly.Memory, inL: number, inR: number, frames: number, block: number): number;
  /** The harness hands over the output block of `block` (addresses of the two float buffers). */
  done(mem: WebAssembly.Memory, outL: number, outR: number, frames: number, block: number): void;
}

export interface HostOptions {
  files: Record<string, Uint8Array>;
  args: string[];
  env: string[];
  print: (line: string) => void;
  ext?: ExtHooks;
}

export interface Host {
  imports: WebAssembly.Imports;
  attach(instance: WebAssembly.Instance): void;
  /** Run __wasm_call_ctors and main(); returns main's exit code. Blocks until the harness leaves its process loop and tears the plugin down. */
  run(): number;
  missingFiles(): string[];
}

const ENOENT = 44;
const EBADF = 8;
const EINVAL = 28;
const ENOTTY = 59;
const TIOCGWINSZ = 0x5413;

interface OpenFile {
  data: Uint8Array;
  pos: number;
}

type Num = number | bigint;

export function createHost(opts: HostOptions): Host {
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  let inst: WebAssembly.Instance | null = null;
  let memory: WebAssembly.Memory | null = null;
  const fds = new Map<number, OpenFile>();
  let nextFd = 10;
  let stdout = '';
  const missing: string[] = [];
  const mem = (): WebAssembly.Memory => {
    if (!memory) throw new Error('host: instance not attached');
    return memory;
  };
  const u8 = (): Uint8Array => new Uint8Array(mem().buffer);
  const dv = (): DataView => new DataView(mem().buffer);
  const cstr = (p: number): string => {
    const m = u8();
    let e = p;
    while (m[e] !== 0) e++;
    return dec.decode(m.subarray(p, e));
  };
  const flushLines = (): void => {
    for (let i = stdout.indexOf('\n'); i >= 0; i = stdout.indexOf('\n')) {
      const l = stdout.slice(0, i);
      stdout = stdout.slice(i + 1);
      opts.print(l);
    }
  };

  const read = (fd: number, iov: number, cnt: number, pnum: number): number => {
    const f = fds.get(fd);
    if (!f) return EBADF;
    const d = dv();
    const m = u8();
    let n = 0;
    for (let i = 0; i < cnt; i++) {
      const p = d.getUint32(iov + 8 * i, true);
      const l = d.getUint32(iov + 8 * i + 4, true);
      const k = Math.min(l, f.data.length - f.pos);
      if (k > 0) {
        m.set(f.data.subarray(f.pos, f.pos + k), p);
        f.pos += k;
        n += k;
      }
      if (k < l) break;
    }
    d.setUint32(pnum, n, true);
    return 0;
  };

  const imports: WebAssembly.Imports = {
    env: {
      exit: (c: number) => {
        throw new Error('wasm exit ' + c);
      },
      _abort_js: () => {
        throw new Error('wasm abort');
      },
      fle_js_init: () => 0,
      fle_js_now: () => 0,
      fle_js_consumed: () => 0n,
      fle_js_unlock: () => 0,
      fle_js_state: () => 0,
      fle_js_lead_ms: () => 0,
      fle_js_publish: () => 0,
      mt1_ext_wait: (inL: number, inR: number, frames: number, block: number): number => {
        if (!opts.ext) return 0;
        return opts.ext.wait(mem(), inL, inR, frames, block);
      },
      mt1_ext_done: (outL: number, outR: number, frames: number, block: number): void => {
        opts.ext?.done(mem(), outL, outR, frames, block);
      },
      emscripten_resize_heap: (req: number): number => {
        const want = req >>> 0;
        const max = 2147483648;
        if (want > max) return 0;
        const old = mem().buffer.byteLength;
        const target = Math.min(max, Math.ceil(Math.max(want, old * 1.2) / 65536) * 65536);
        try {
          mem().grow((target - old) / 65536);
          return 1;
        } catch {
          return 0;
        }
      },
      __syscall_openat: (_dirfd: number, pathp: number): number => {
        const name = cstr(pathp);
        const data = opts.files[name];
        if (!data) {
          missing.push(name);
          return -ENOENT;
        }
        const fd = nextFd++;
        fds.set(fd, { data, pos: 0 });
        return fd;
      },
      __syscall_fcntl64: (): number => 0,
      // TIOCGWINSZ answers "a terminal" on stdout/stderr so that stdio is line buffered and every line reaches print() as it is written.
      __syscall_ioctl: (fd: number, op: number): number => ((fd === 1 || fd === 2) && op === TIOCGWINSZ ? 0 : -ENOTTY),
      __syscall_stat64: (pathp: number, buf: number): number => {
        const data = opts.files[cstr(pathp)];
        if (!data) return -ENOENT;
        const d = dv();
        for (let i = 0; i < 112; i += 4) d.setUint32(buf + i, 0, true);
        d.setUint32(buf + 4, 0o100644, true);
        d.setUint32(buf + 8, 1, true);
        d.setBigInt64(buf + 24, BigInt(data.length), true);
        d.setInt32(buf + 32, 4096, true);
        return 0;
      },
    },
    wasi_snapshot_preview1: {
      clock_time_get: (id: number, _prec: Num, p: number): number => {
        const t = id === 0 ? Date.now() * 1e6 : performance.now() * 1e6;
        dv().setBigUint64(p, BigInt(Math.round(t)), true);
        return 0;
      },
      fd_close: (fd: number): number => {
        fds.delete(fd);
        return 0;
      },
      fd_write: (fd: number, iov: number, cnt: number, pnum: number): number => {
        const d = dv();
        const m = u8();
        let n = 0;
        for (let i = 0; i < cnt; i++) {
          const p = d.getUint32(iov + 8 * i, true);
          const l = d.getUint32(iov + 8 * i + 4, true);
          if (fd === 1 || fd === 2) stdout += dec.decode(m.subarray(p, p + l));
          n += l;
        }
        if (fd === 1 || fd === 2) flushLines();
        d.setUint32(pnum, n, true);
        return 0;
      },
      fd_read: read,
      fd_pread: (fd: number, iov: number, cnt: number, off: Num, pnum: number): number => {
        const f = fds.get(fd);
        if (!f) return EBADF;
        const save = f.pos;
        f.pos = Number(off);
        const r = read(fd, iov, cnt, pnum);
        f.pos = save;
        return r;
      },
      fd_seek: (fd: number, off: Num, whence: number, pnew: number): number => {
        const f = fds.get(fd);
        if (!f) return EBADF;
        const o = Number(off);
        const np = whence === 0 ? o : whence === 1 ? f.pos + o : f.data.length + o;
        if (np < 0) return EINVAL;
        f.pos = np;
        dv().setBigInt64(pnew, BigInt(np), true);
        return 0;
      },
      environ_sizes_get: (pc: number, pb: number): number => {
        const d = dv();
        d.setUint32(pc, opts.env.length, true);
        d.setUint32(pb, opts.env.reduce((s, e) => s + enc.encode(e).length + 1, 0), true);
        return 0;
      },
      environ_get: (pp: number, pb: number): number => {
        const d = dv();
        const m = u8();
        let b = pb;
        opts.env.forEach((e, i) => {
          d.setUint32(pp + 4 * i, b, true);
          const bytes = enc.encode(e);
          m.set(bytes, b);
          m[b + bytes.length] = 0;
          b += bytes.length + 1;
        });
        return 0;
      },
    },
  };

  return {
    imports,
    attach(instance: WebAssembly.Instance): void {
      inst = instance;
      memory = instance.exports['memory'] as WebAssembly.Memory;
    },
    run(): number {
      if (!inst) throw new Error('host: instance not attached');
      const x = inst.exports as Record<string, (...a: number[]) => number>;
      const call = (name: string, ...a: number[]): number => {
        const f = x[name];
        if (!f) throw new Error('wasm export missing: ' + name);
        return f(...a);
      };
      call('__wasm_call_ctors');
      const all = ['./this.program', ...opts.args];
      const argv = call('_emscripten_stack_alloc', (all.length + 1) * 4);
      all.forEach((a, i) => {
        const b = enc.encode(a + '\0');
        const p = call('_emscripten_stack_alloc', b.length);
        u8().set(b, p);
        dv().setUint32(argv + 4 * i, p, true);
      });
      dv().setUint32(argv + 4 * all.length, 0, true);
      const rc = call('__main_argc_argv', all.length, argv);
      try {
        call('fflush', 0);
      } catch {
        /* nothing buffered */
      }
      if (stdout) {
        opts.print(stdout);
        stdout = '';
      }
      return rc;
    },
    missingFiles: () => missing,
  };
}
