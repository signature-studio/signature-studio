#!/usr/bin/env python3
"""fletest: the test runner behind harness/test.sh (decision 0149). Standard library only.

  fletest.py run   --mode full|fast|affected [options]   build, select, run the test executables in parallel, report
  fletest.py record [--only PAT ...]                      run the selected executables under strace, write the input sets
  fletest.py names                                        print the sorted test names of every executable and their SHA-256

What it does that `cargo test` does not:
  * runs the test executables of the workspace in parallel (cargo runs them one after the other) with a memory budget;
  * puts a caching `clang` first in PATH (harness/testfast/clangcache.c) and sets RLIMIT_CORE to 1 byte, so that a test program that aborts on
    purpose does not wait for the system core-dump handler (apport: 3-5 s per abort on this machine);
  * skips an executable whose result is already known: same executable, same environment, same content of every file it read the last time
    (the set is taken with strace by `record`), reported as "cached" and never as "run";
  * `affected` runs only the executables that can see a changed file.
FLE_TEST_NOCACHE=1 turns the clang cache and the result cache off (every executable runs, every compile is real).
"""
import argparse, concurrent.futures, hashlib, json, os, re, resource, shutil, subprocess, sys, threading, time
from pathlib import Path

HERE = Path(__file__).resolve().parent
LIFT = HERE.parents[1]
REPO = LIFT.parent
DEPS_FILE = HERE / "deps.tsv"
SKIP_RE = re.compile(r"^\s*(SKIPPED|skip(ped)?:)", re.I)
RESULT_RE = re.compile(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out")
ENV_PREFIXES = ("FLE_", "FLENGINE_", "ILTOOLS_", "BLEND2D_", "DSP_", "FREETYPE_", "FL64_", "X87_", "MXCSR_", "RUST_TEST", "RUST_BACKTRACE")
SYSTEM_PREFIXES = ("/usr/", "/lib/", "/lib64/", "/bin/", "/sbin/", "/etc/", "/proc/", "/sys/", "/dev/", "/opt/", "/run/", "/var/", "/snap/")


def sh(cmd, **kw):
    return subprocess.run(cmd, text=True, capture_output=True, **kw)


def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        while True:
            b = f.read(1 << 20)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def log(*a):
    print(*a, file=sys.stderr, flush=True)


class Bin:
    def __init__(self, name, pkg, target, kind, exe, cwd, src_path):
        self.name, self.pkg, self.target, self.kind, self.exe, self.cwd, self.src_path = name, pkg, target, kind, exe, cwd, src_path
        self.cratedirs = []  # directories of the crates compiled into it (its own and its path dependencies)
        self.sources = []    # rust sources the test crate itself is made of (rustc dep-info)

    def stamp(self):
        """sha256 of the executable (so a rebuild that gives the same bytes keeps its cached result); memoised by (size, mtime, inode)."""
        if not hasattr(self, "_stamp"):
            st = os.stat(self.exe)
            key = [st.st_size, st.st_mtime_ns, st.st_ino]
            memo = _exe_memo()
            e = memo.get(self.exe)
            if e and e[:3] == key:
                self._stamp = e[3]
            else:
                self._stamp = sha256_file(self.exe)
                memo[self.exe] = key + [self._stamp]
                _exe_memo_dirty[0] = True
        return self._stamp


_exe_memo_cache = {}
_exe_memo_dirty = [False]


def _exe_memo():
    if "m" not in _exe_memo_cache:
        _exe_memo_cache["m"] = load_json(cache_dir() / "exe-hash-memo.json") or {}
    return _exe_memo_cache["m"]


def exe_memo_save():
    if _exe_memo_dirty[0]:
        p = cache_dir() / "exe-hash-memo.json"
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(_exe_memo_cache["m"]))
        _exe_memo_dirty[0] = False


def target_dir():
    return Path(os.environ.get("CARGO_TARGET_DIR") or (LIFT / "target"))


def cache_dir():
    return Path(os.environ.get("FLE_TEST_CACHE") or (target_dir() / "fle-test-cache"))


def nocache():
    v = os.environ.get("FLE_TEST_NOCACHE", "")
    return v not in ("", "0")


# ------------------------------------------------------------------------------------------------ build
def build(no_build=False):
    t0 = time.time()
    cmd = ["cargo", "test", "--workspace", "--no-run", "--offline", "--message-format=json"]
    p = subprocess.run(cmd, cwd=LIFT, text=True, stdout=subprocess.PIPE, stderr=None)
    if p.returncode != 0:
        log("cargo test --no-run failed")
        sys.exit(2)
    bins = []
    for line in p.stdout.splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get("reason") != "compiler-artifact" or not m.get("executable") or not m["profile"]["test"]:
            continue
        pkgdir = m["package_id"].split("file://")[1].split("#")[0]
        pkg = pkgdir.rstrip("/").split("/")[-1]
        kind = m["target"]["kind"][0]
        tname = m["target"]["name"]
        name = f"{pkg}/{tname}" if kind == "test" else f"{pkg}/{'lib' if kind == 'lib' else 'bin-' + tname}"
        b = Bin(name, pkg, tname, kind, m["executable"], pkgdir, m["target"]["src_path"])
        dfile = m["executable"] + ".d"
        if os.path.exists(dfile):
            txt = Path(dfile).read_text()
            srcs = re.findall(r"(\S+\.rs)", txt.split(":", 1)[-1].replace("\\\n", " "))
            b.sources = [os.path.relpath(s, REPO) for s in srcs if os.path.isabs(s) and s.startswith(str(REPO))]
        bins.append(b)
    # crate closure from cargo metadata
    md = json.loads(sh(["cargo", "metadata", "--offline", "--format-version", "1"], cwd=LIFT).stdout)
    byname = {p["name"]: p for p in md["packages"]}
    ws = set(md["workspace_members"])
    ids = {p["id"]: p for p in md["packages"]}
    def closure(pn, seen):
        if pn in seen:
            return
        seen.add(pn)
        for d in byname[pn]["dependencies"]:
            if d.get("path") and d["name"] in byname:
                closure(d["name"], seen)
    for b in bins:
        s = set()
        closure(b.pkg, s)
        b.cratedirs = sorted(os.path.relpath(os.path.dirname(byname[n]["manifest_path"]), REPO) for n in s)
    bins.sort(key=lambda b: b.name)
    for b in bins:
        b.stamp()
    exe_memo_save()
    log(f"[build] cargo test --no-run: {time.time() - t0:.1f} s, {len(bins)} test executables")
    return bins, time.time() - t0


# ------------------------------------------------------------------------------------------------ runner
RUNNER = HERE / "runner.sh"


# ------------------------------------------------------------------------------------------------ env fingerprint / result cache
def env_fingerprint():
    h = hashlib.sha256()
    for k in sorted(os.environ):
        if k.startswith(ENV_PREFIXES):
            h.update(f"{k}={os.environ[k]}\0".encode())
    for tool in ("clang", "python3", "node", "nm"):
        p = shutil.which(tool, path=os.environ.get("FLE_CC_REAL_PATH") or None) or shutil.which(tool)
        if p:
            st = os.stat(os.path.realpath(p))
            h.update(f"{tool}:{os.path.realpath(p)}:{st.st_size}:{st.st_mtime_ns}\0".encode())
    return h.hexdigest()


_hmemo = {}


def hash_input(path):
    """sha256 of a file; files over 1 MiB (the DLLs) are memoised by (size, mtime, inode)."""
    try:
        st = os.stat(path)
    except OSError:
        return None
    if st.st_size > (1 << 20):
        k = (path, st.st_size, st.st_mtime_ns, st.st_ino)
        if k not in _hmemo:
            _hmemo[k] = sha256_file(path)
        return _hmemo[k]
    return sha256_file(path)


def dir_listing_hash(path):
    try:
        return hashlib.sha256("\0".join(sorted(os.listdir(path))).encode()).hexdigest()
    except OSError:
        return None


def result_path(b):
    return cache_dir() / "results" / (b.name.replace("/", "__") + ".json")


def record_path(b):
    return cache_dir() / "records" / (b.name.replace("/", "__") + ".json")


def snapshot_inputs(rec):
    """Hashes now of the files and directories of a record: {key: hash}."""
    snap = {}
    for p in rec.get("files", []):
        snap["F " + p] = hash_input(p)
    for p in rec.get("dirs", []):
        snap["D " + p] = dir_listing_hash(p)
    for p in rec.get("absent", []):
        snap["A " + p] = None if not os.path.lexists(p) else "present"
    return snap


def load_json(p):
    try:
        return json.loads(Path(p).read_text())
    except (OSError, ValueError):
        return None


def cache_lookup(b, envfp):
    """Returns the stored result when exe, environment and inputs are the same as in the passing run that stored it."""
    if nocache():
        return None
    r = load_json(result_path(b))
    if not r or r.get("exe") != b.stamp() or r.get("env") != envfp or r.get("failed", 1) != 0 or r.get("rc") != 0:
        return None
    rec = load_json(record_path(b))
    if not rec or rec.get("exe") != b.stamp():
        return None
    if snapshot_inputs(rec) != r.get("snap"):
        return None
    return r


# ------------------------------------------------------------------------------------------------ strace record
STRACE_SYS = "openat,open,execve,stat,newfstatat,statx,lstat,access,faccessat,faccessat2,getdents64,readlink,readlinkat,mkdir,mkdirat,unlink,unlinkat,rename,renameat,renameat2,creat,truncate,chdir"


def strace_calls(trace):
    """Yield (syscall, text after the name and the opening parenthesis) for every completed call of a `strace -f` trace: an
    `<unfinished ...>` line is joined with the `<... name resumed>` line of the same process."""
    pending = {}
    for line in trace:
        line = line.rstrip("\n")
        m = re.match(r"^(\d+)\s+(.*)$", line)
        if not m:
            continue
        pid, rest = m.group(1), m.group(2)
        if rest.endswith("<unfinished ...>"):
            pending[pid] = rest[: -len("<unfinished ...>")]
            continue
        mr = re.match(r"^<\.\.\. (\w+) resumed>(.*)$", rest)
        if mr:
            head = pending.pop(pid, None)
            if head is None:
                continue
            rest = head + mr.group(2)
        mc = re.match(r"^(\w+)\((.*)$", rest)
        if mc:
            yield mc.group(1), mc.group(2)


def parse_strace(trace, tmproots):
    """Files read, directories listed, paths probed and absent, programs executed, from a `strace -f -y` trace."""
    first = {}   # path -> 'r' | 'w' (first kind of access)
    absent = set()
    dirs = set()
    execs = set()
    for sc, rest in strace_calls(trace):
        failed = re.search(r"\) += -1 (\w+)", rest)
        err = failed.group(1) if failed else None
        ps = re.findall(r'"((?:[^"\\]|\\.)*)"', rest)
        dm = re.match(r"(?:AT_FDCWD|\d+)<([^>]*)>", rest)
        def full(q):
            q = bytes(q, "latin1").decode("unicode_escape") if "\\" in q else q
            if q.startswith("/"):
                return os.path.normpath(q)
            if sc in ("open", "stat", "lstat", "access", "creat", "mkdir", "unlink", "rename", "truncate", "readlink"):
                return None
            return os.path.normpath(os.path.join(dm.group(1), q)) if dm else None
        if sc == "execve":
            if ps and not err:
                execs.add(ps[0])
            continue
        if sc == "getdents64":
            m2 = re.match(r"\d+<([^>]*)>", rest)
            if m2:
                dirs.add(os.path.normpath(m2.group(1)))
            continue
        if not ps:
            continue
        p = full(ps[0])
        if not p:
            continue
        if sc in ("openat", "open"):
            if re.search(r"O_(WRONLY|RDWR|CREAT|TRUNC)", rest):
                first.setdefault(p, "w")
            elif err in ("ENOENT", "ENOTDIR"):
                absent.add(p)
            elif not err:
                first.setdefault(p, "r")
        elif sc in ("stat", "newfstatat", "statx", "lstat", "access", "faccessat", "faccessat2", "readlink", "readlinkat"):
            if err in ("ENOENT", "ENOTDIR"):
                absent.add(p)
            elif not err:
                first.setdefault(p, "r")
        elif sc in ("mkdir", "mkdirat", "creat", "truncate", "unlink", "unlinkat", "rename", "renameat", "renameat2"):
            for q in ps:
                pp = full(q)
                if pp:
                    first.setdefault(pp, "w")
    return first, absent, dirs, execs


def classify_paths(first, absent, dirs):
    """Drop what the run made itself and the system; returns (files, absent, dirs) of the inputs.
    A missing header (clang looks for `#include "x.h"` in the directory of the including file and in every -I directory) becomes a dependency on the
    listing of that directory: a new file there changes the compile. Other missing paths stay as they are (the test checks whether the file exists)."""
    cd = str(target_dir())
    root = str(REPO) + "/"
    def keep(p):
        if p.startswith(SYSTEM_PREFIXES) or p.startswith(cd) or p.startswith("/tmp/") or p == "/tmp":
            return False
        if "/.cargo/" in p or "/.rustup/" in p:
            return False
        return True
    files = sorted(p for p, k in first.items() if k == "r" and keep(p) and os.path.isfile(p))
    ab = set()
    written_in = set(os.path.dirname(p) for p, k in first.items() if k == "w")
    ds = set(p for p in dirs if keep(p) and p.startswith(root) and p not in written_in)
    for p in absent:
        if not (keep(p) and p.startswith(root)) or p in first or re.search(r"(/-l[^/]*|\.so(\.\d+)*|\.a)$", p):
            continue
        rel = p[len(root):]
        if re.match(r"lift/(out-[^/]+|target)/", rel):
            continue
        if p.endswith(".h"):
            if os.path.dirname(p) not in written_in:
                ds.add(os.path.dirname(p))
        else:
            ab.add(p)
    return files, sorted(ab), sorted(ds)


# ------------------------------------------------------------------------------------------------ running
class Result:
    pass


def run_one(b, args, extra_env, outdir, envfp, strace=False):
    r = Result()
    r.b = b
    r.cached = False
    env = dict(os.environ)
    env.update(extra_env)
    env["CARGO_MANIFEST_DIR"] = b.cwd
    env["LC_ALL"] = "C"
    cmd = ["bash", str(RUNNER), b.exe, "--test-threads", str(args.threads), "--nocapture"] + (args.test_args or [])
    trace = None
    if strace:
        trace = str(outdir / (b.name.replace("/", "__") + ".strace"))
        cmd = ["strace", "-f", "-qq", "-y", "-s", "512", "--seccomp-bpf", "-e", "trace=" + STRACE_SYS, "-o", trace] + cmd
    base = outdir / b.name.replace("/", "__")
    t0 = time.time()
    with open(str(base) + ".out", "w") as fo, open(str(base) + ".err", "w") as fe:
        p = subprocess.Popen(cmd, cwd=b.cwd, env=env, stdout=fo, stderr=fe)
        _, st, ru = os.wait4(p.pid, 0)
    r.wall = time.time() - t0
    r.rc = os.waitstatus_to_exitcode(st)
    r.cpu = ru.ru_utime + ru.ru_stime
    r.rss = ru.ru_maxrss
    r.passed = r.failed = r.ignored = 0
    r.has_result = False
    r.failures = []
    in_fail = False
    for line in open(str(base) + ".out", errors="replace"):
        m = RESULT_RE.match(line)
        if m:
            r.has_result = True
            r.passed += int(m.group(2)); r.failed += int(m.group(3)); r.ignored += int(m.group(4))
            in_fail = False
        elif line.rstrip() == "failures:":
            in_fail = True
        elif in_fail and re.match(r"^    \S+$", line.rstrip()):
            r.failures.append(line.strip())
        elif in_fail and not line.strip():
            in_fail = False
    r.skipped = 0
    r.skip_lines = []
    for fn in (str(base) + ".out", str(base) + ".err"):
        for line in open(fn, errors="replace"):
            if SKIP_RE.match(line):
                r.skipped += 1
                r.skip_lines.append(line.strip()[:160])
    if r.rc != 0 and not r.failures:
        r.failures.append(f"(exit status {r.rc}, {'no result line' if not r.has_result else 'result line present'})")
    r.trace = trace
    return r


def write_record(b, r, tmproots):
    first, absent, dirs, execs = parse_strace(open(r.trace, errors="replace"), tmproots)
    files, ab, ds = classify_paths(first, absent, dirs)
    own = {"runner.sh", "shim.sh", "prlimit", "cat", "cut", "dirname", "sha256sum", "readlink", "sh", "bash", "dash", "env", "cc", "gcc", "collect2", "ld", "x86_64-linux-gnu-ld.bfd", "ld.bfd", "ld.lld", "mkdir", "mv", "rm", "uname", "command"}
    spawns = set()
    for e in execs:
        n = os.path.basename(e)
        if e == b.exe or n in own:
            continue
        spawns.add(n if e.startswith(SYSTEM_PREFIXES) else "(generated program)")
    root = str(REPO) + "/"
    writes = sorted(p for p, k in first.items() if k == "w" and p.startswith(root) and not p.startswith(str(target_dir())))
    rec = {"exe": b.stamp(), "files": files, "absent": ab, "dirs": ds, "spawns": sorted(spawns), "writes": writes, "wall": round(r.wall, 1), "rss_mb": r.rss // 1024}
    p = record_path(b)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(rec))
    try:
        os.unlink(r.trace)
    except OSError:
        pass
    return rec


def store_result(b, r, envfp, snap):
    p = result_path(b)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps({"exe": b.stamp(), "env": envfp, "rc": r.rc, "passed": r.passed, "failed": r.failed, "ignored": r.ignored, "skipped": r.skipped,
                             "skip_lines": r.skip_lines, "wall": round(r.wall, 2), "snap": snap}))


def timings_load():
    return load_json(cache_dir() / "timings.json") or {}


def timings_save(t):
    p = cache_dir() / "timings.json"
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(t))


def ncpu():
    """CPUs this process may use: the cgroup quota (cpu.max) if there is one, else the CPUs of the machine."""
    n = os.cpu_count() or 4
    try:
        cg = Path("/sys/fs/cgroup" + Path("/proc/self/cgroup").read_text().strip().split("::")[-1] + "/cpu.max").read_text().split()
        if cg[0] != "max":
            n = min(n, max(1, int(int(cg[0]) / int(cg[1]) + 0.5)))
    except (OSError, ValueError, IndexError):
        pass
    return n


def mem_budget_mb():
    try:
        mx = Path("/sys/fs/cgroup" + Path("/proc/self/cgroup").read_text().strip().split("::")[-1] + "/memory.max").read_text().strip()
        if mx != "max":
            return int(mx) // (1 << 20) * 7 // 10
    except (OSError, ValueError):
        pass
    for line in open("/proc/meminfo"):
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) // 1024 * 6 // 10
    return 4096


def execute(selected, args, label):
    """Run the selected executables (a list of Bin) in parallel. Returns the list of Result (cached ones included)."""
    envfp = env_fingerprint()
    outdir = cache_dir() / "logs" / (time.strftime("%Y%m%d-%H%M%S") + "-" + label)
    outdir.mkdir(parents=True, exist_ok=True)
    os.environ.setdefault("FLE_TEST_CACHE", str(cache_dir()))
    stats = outdir / "cc-stats.tsv"
    shim_env = {} if nocache() else {"FLE_CC_STATS": str(stats)}
    results = []
    todo = []
    for b in selected:
        r = cache_lookup(b, envfp) if not args.fresh else None
        if r:
            c = Result()
            c.b, c.cached, c.wall, c.rc = b, True, r.get("wall", 0), 0
            c.passed, c.failed, c.ignored, c.skipped, c.skip_lines = r["passed"], 0, r["ignored"], r["skipped"], r.get("skip_lines", [])
            c.failures, c.cpu, c.rss, c.has_result, c.trace = [], 0, 0, True, None
            results.append(c)
        else:
            todo.append(b)
    tim = timings_load()
    _, dmeta = load_deps()
    def est_wall(b):
        if b.name in tim:
            return tim[b.name]["wall"]
        w = dmeta.get(b.name, {}).get("wall", "")
        return [1, 3, 10, 25, 60][int(w)] if w.isdigit() else 10.0
    def est_mem(b):
        if b.name in tim:
            return tim[b.name]["rss_mb"]
        m = dmeta.get(b.name, {}).get("mem", "")
        return [200, 500, 1000][int(m)] if m.isdigit() else 300
    todo.sort(key=lambda b: -est_wall(b))
    budget = mem_budget_mb()
    lock = threading.Lock()
    state = {"mem": 0}
    cv = threading.Condition(lock)

    def worker(b):
        est = max(150, est_mem(b))
        with cv:
            while state["mem"] + est > budget and state["mem"] > 0:
                cv.wait()
            state["mem"] += est
        try:
            rec = load_json(record_path(b))
            have_rec = bool(rec and rec.get("exe") == b.stamp())
            need_record = (args.autorecord or args.record) and not nocache() and not have_rec
            if args.record_only:
                need_record = True
            snap_before = snapshot_inputs(rec) if rec and rec.get("exe") == b.stamp() else None
            r = run_one(b, args, shim_env, outdir, envfp, strace=need_record and shutil.which("strace") is not None)
            if r.trace and os.path.exists(r.trace):
                rec = write_record(b, r, [])
                snap_before = snapshot_inputs(rec)
            r.cached = False
            if r.rc == 0 and r.failed == 0 and not nocache() and snap_before is not None and not args.record_only:
                store_result(b, r, envfp, snap_before)
            return r
        finally:
            with cv:
                state["mem"] -= est
                cv.notify_all()

    t0 = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as ex:
        futs = [ex.submit(worker, b) for b in todo]
        for f in concurrent.futures.as_completed(futs):
            r = f.result()
            results.append(r)
            tim[r.b.name] = {"wall": round(r.wall, 2), "rss_mb": r.rss // 1024}
            flag = "ok" if r.rc == 0 and r.failed == 0 else "FAIL"
            log(f"[{flag}] {r.b.name:42s} {r.wall:7.1f}s passed={r.passed} failed={r.failed} skipped={r.skipped}")
    exec_wall = time.time() - t0
    timings_save(tim)
    return results, exec_wall, outdir, stats


def gc_cache(limit_mb=None, quiet=False):
    """Keep the clang object store under FLE_TEST_CACHE_MAX_MB (default 1500): remove the least recently used blobs (a hit touches its blob) and the manifests older than them."""
    limit = (limit_mb or int(os.environ.get("FLE_TEST_CACHE_MAX_MB", "1500"))) * (1 << 20)
    b = cache_dir() / "cc" / "b"
    if not b.exists():
        return
    files = []
    total = 0
    for d in os.scandir(b):
        for f in os.scandir(d.path):
            st = f.stat()
            files.append((st.st_mtime, st.st_size, f.path))
            total += st.st_size
    if total <= limit:
        return
    files.sort()
    cutoff = 0
    for mt, sz, path in files:
        if total <= limit * 7 // 10:
            break
        try:
            os.unlink(path)
        except OSError:
            continue
        total -= sz
        cutoff = mt
    m = cache_dir() / "cc" / "m"
    if m.exists():
        for d in os.scandir(m):
            for f in os.scandir(d.path):
                if f.stat().st_mtime < cutoff:
                    try:
                        os.unlink(f.path)
                    except OSError:
                        pass
    if not quiet:
        log(f"[gc] clang object store trimmed to {total >> 20} MB (limit {limit >> 20} MB)")


def doc_tests(args, outdir):
    t0 = time.time()
    p = subprocess.run(["cargo", "test", "--workspace", "--doc", "--offline", "--no-fail-fast"], cwd=LIFT, text=True, capture_output=True)
    (outdir / "doctests.out").write_text(p.stdout + p.stderr)
    r = Result()
    r.passed = r.failed = r.ignored = 0
    n = 0
    for line in p.stdout.splitlines():
        m = RESULT_RE.match(line)
        if m:
            n += 1
            r.passed += int(m.group(2)); r.failed += int(m.group(3)); r.ignored += int(m.group(4))
    r.results = n
    r.rc = p.returncode
    r.wall = time.time() - t0
    return r


# ------------------------------------------------------------------------------------------------ selection
def load_deps():
    """deps.tsv -> ({('R'|'A'|'D', path): set(names)}, {name: meta dict}). Missing file: ({}, {})."""
    idx, meta = {}, {}
    if not DEPS_FILE.exists():
        return idx, meta
    for line in DEPS_FILE.read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        f = line.split("\t")
        if f[0] == "B":
            meta[f[1]] = dict(kv.split("=", 1) for kv in f[2:] if "=" in kv)
        elif f[0] in ("R", "A", "D") and len(f) >= 3:
            idx.setdefault((f[0], f[1]), set()).update(f[2].split(","))
    return idx, meta


def changed_files(args):
    """Repo-relative paths changed since the base, and which of them exist now: (set(modified-or-added), set(removed-or-added-or-renamed for directory listings))."""
    mod, shape = set(), set()
    if args.files:
        for l in Path(args.files).read_text().splitlines():
            l = l.strip()
            if not l:
                continue
            if l[:2] in ("A:", "D:"):   # a file that was added / removed (matters for directory listings)
                l = l[2:]
                shape.add(l)
            mod.add(l)
        return mod, shape
    if args.base_tree:
        base = Path(args.base_tree)
        skip = re.compile(r"(^|/)(target|\.git|node_modules|out-[^/]*)(/|$)")
        def walk(root):
            out = {}
            for dp, dn, fn in os.walk(root):
                rel = os.path.relpath(dp, root)
                dn[:] = [d for d in dn if not skip.search((rel + "/" + d).lstrip("./"))]
                for f in fn:
                    p = os.path.normpath(os.path.join(rel, f))
                    if not skip.search(p):
                        out[p] = os.path.join(dp, f)
            return out
        a, b = walk(base), walk(REPO)
        for p in set(a) | set(b):
            if p not in a or p not in b:
                mod.add(p); shape.add(p)
            elif not os.path.islink(b[p]) and (os.path.getsize(a[p]) != os.path.getsize(b[p]) or Path(a[p]).read_bytes() != Path(b[p]).read_bytes()):
                mod.add(p)
        return mod, shape
    top = sh(["git", "rev-parse", "--show-toplevel"], cwd=REPO).stdout.strip()
    pre = os.path.relpath(REPO, top)
    def rel(p):
        q = os.path.relpath(os.path.join(top, p), REPO)
        return q
    r = sh(["git", "diff", "--name-status", "-M", "--no-renames", args.base or "HEAD", "--", "."], cwd=REPO)
    for l in r.stdout.splitlines():
        st, p = l.split("\t", 1)
        mod.add(rel(p))
        if st[0] in "AD":
            shape.add(rel(p))
    r = sh(["git", "ls-files", "-o", "--exclude-standard", "--full-name"], cwd=REPO)
    for p in r.stdout.splitlines():
        if not p.startswith(pre.lstrip(".")) and pre != ".":
            continue
        mod.add(rel(p)); shape.add(rel(p))
    return mod, shape


DOC_ONLY = re.compile(r"^(docs|worklog)/|\.md$|^app/|^tools/")


def select_affected(bins, args):
    mod, shape = changed_files(args)
    mod = set(p for p in mod if p != "lift/harness/testfast/deps.tsv")   # the generated list itself is read by the runner only
    shape = set(p for p in shape if p in mod)
    idx, meta = load_deps()
    sel = {}
    why = {}
    code_changed = [p for p in mod if not DOC_ONLY.search(p)]
    def add(b, reason):
        if b.name not in sel:
            sel[b.name] = b
            why[b.name] = reason
    for p in sorted(mod):
        if p in ("lift/Cargo.toml", "lift/Cargo.lock", "Cargo.toml", "Cargo.lock") or p.startswith("lift/.cargo/") or p == "lift/rust-toolchain.toml":
            for b in bins:
                add(b, f"workspace file {p}")
            continue
        for b in bins:
            for cd in b.cratedirs:
                if p.startswith(cd + "/") and not p.startswith(cd + "/tests/") and not p.startswith(cd + "/benches/"):
                    add(b, f"crate source {p}")
            if p in b.sources:
                add(b, f"test source {p}")
            if b.src_path and os.path.relpath(b.src_path, REPO) == p:
                add(b, f"test source {p}")
        for key in (("R", p), ("A", p)):
            for n in idx.get(key, ()):
                for b in bins:
                    if b.name == n:
                        add(b, f"reads {p}")
    for p in sorted(shape):
        d = os.path.dirname(p)
        for n in idx.get(("D", d), ()):
            for b in bins:
                if b.name == n:
                    add(b, f"lists {d}/ ({p} added or removed)")
        for n in idx.get(("A", p), ()):
            for b in bins:
                if b.name == n:
                    add(b, f"probed {p} that was absent")
    for b in bins:
        if b.name not in meta and idx:
            add(b, "no record in deps.tsv")
    if not idx:
        for b in bins:
            add(b, "no deps.tsv: everything is affected")
    fast = []
    if code_changed:
        for b in bins:
            m = meta.get(b.name)
            if m and m.get("tier") == "fast" and b.name not in sel:
                fast.append(b)
                why[b.name] = "fast tier (always runs when code changed)"
    return [sel[n] for n in sorted(sel)] + fast, why, mod


def fast_tier(bins):
    idx, meta = load_deps()
    out = [b for b in bins if meta.get(b.name, {}).get("tier") == "fast"]
    return out


# ------------------------------------------------------------------------------------------------ commands
def report(results, doc, build_s, exec_wall, outdir, stats, mode, selected_n, total_n, args, t_start):
    n_exec = [r for r in results if not r.cached]
    n_cache = [r for r in results if r.cached]
    nres = len(results) + (doc.results if doc else 0)
    passed = sum(r.passed for r in results) + (doc.passed if doc else 0)
    failed = sum(r.failed for r in results) + (doc.failed if doc else 0)
    ignored = sum(r.ignored for r in results) + (doc.ignored if doc else 0)
    skipped = sum(r.skipped for r in results)
    bad = [r for r in results if r.rc != 0 or r.failed]
    log("")
    log(f"== fletest {mode}: {len(results)} of {total_n} test executables selected" + (f" + doc-tests ({doc.results} results)" if doc else ""))
    log(f"   results={nres} passed={passed} failed={failed} ignored={ignored} SKIPPED-lines={skipped}")
    miss = os.environ.get("FLE_TEST_MISSING_INPUTS", "")
    if miss not in ("", "0"):
        log(f"   MISSING INPUT FILES: {miss} (see harness/test-env.sh); the tests that need them print SKIPPED and pass without checking, or fail")
    log(f"   executables: run={len(n_exec)} cached={len(n_cache)} (a cached executable is NOT run: same executable, environment and input files as its last passing run)")
    cpass = sum(r.passed for r in n_cache)
    log(f"   tests: run={sum(r.passed for r in n_exec) + (doc.passed if doc else 0)} cached={cpass}")
    if stats.exists():
        hit = miss = pas = 0
        saved = spent = 0.0
        for line in stats.read_text().splitlines():
            f = line.split("\t")
            if f[0] == "hit": hit += 1; saved += float(f[1])
            elif f[0] == "miss": miss += 1; spent += float(f[1])
            elif f[0] == "pass": pas += 1
        log(f"   clang cache: compile hit={hit} (saved ~{saved:.0f} s of compile) miss={miss} (compiled {spent:.0f} s) passthrough={pas}")
    log(f"   wall: total={time.time() - t_start:.1f} s (build {build_s:.1f}, executables {exec_wall:.1f}" + (f", doc-tests {doc.wall:.1f}" if doc else "") + f"), logs: {outdir}")
    if skipped:
        by = {}
        for r in results:
            for l in r.skip_lines:
                by.setdefault(r.b.name, []).append(l)
        for n, ls in sorted(by.items()):
            log(f"   SKIPPED in {n}: {len(ls)} (e.g. {ls[0]})")
    for r in bad:
        log(f"   FAILED {r.b.name}: {', '.join(r.failures[:12])}")
    if doc and (doc.rc != 0 or doc.failed):
        log("   FAILED doc-tests")
    return 1 if bad or (doc and (doc.rc != 0 or doc.failed)) else 0


def cmd_run(args):
    t_start = time.time()
    resource.setrlimit(resource.RLIMIT_CORE, (1, 1))
    bins, build_s = build()
    total = len(bins)
    mode = args.mode
    args.autorecord = mode == "full" and not args.no_record   # a full run is the gate that fills the result cache: take the input sets of the executables that have none (strace, once)
    why = {}
    if mode == "full":
        selected = bins
    elif mode == "fast":
        selected = fast_tier(bins)
        if not selected:
            log("no deps.tsv tier information: run `harness/test.sh record` first")
            return 2
    else:
        selected, why, mod = select_affected(bins, args)
        log(f"[affected] {len(mod)} changed files; {len(selected)} of {total} executables selected")
        for b in selected:
            log(f"   {b.name}: {why[b.name]}")
    if args.only:
        selected = [b for b in selected if any(re.search(p, b.name) for p in args.only)]
    results, exec_wall, outdir, stats = ([], 0.0, cache_dir() / "logs", Path("/nonexistent"))
    doc = None
    if selected:
        results, exec_wall, outdir, stats = execute(selected, args, mode)
    docs_wanted = mode == "full" or (mode == "affected" and any(why.get(b.name, "").startswith(("crate source", "workspace file")) for b in selected))
    if docs_wanted and not args.no_doc:
        outdir.mkdir(parents=True, exist_ok=True)
        doc = doc_tests(args, outdir)
    gc_cache()
    return report(results, doc, build_s, exec_wall, outdir, stats, mode, len(selected), total, args, t_start)


def collate_deps(bins):
    """Write deps.tsv from the local records (records/*.json) of the given executables; the other executables keep their old lines."""
    allrecs = {}
    for b in bins:
        rec = load_json(record_path(b))
        if rec:
            allrecs[b.name] = rec
    old_idx, old_meta = load_deps()
    root = str(REPO) + "/"
    names = set(allrecs)
    lines = ["# fletest deps v1: generated by `harness/test.sh record` (strace -f of every test executable); do not edit.",
             "# R: path<TAB>test executables that read or stat it | A: probed and absent | D: directory listed (or searched for a header) | B: facts per executable",
             "# B fields: tier=fast (starts no other program and ran in under 3 s: `test.sh fast`, and always run by `affected` when code changed), spawns=, ext=1 (reads a file that is not in git),",
             "#           wall= and mem= are coarse buckets (0..4 / 0..2) that only order the parallel runs"]
    idxn = {}
    meta = []
    for n, rec in sorted(allrecs.items()):
        ext = False
        for kind, key in (("R", "files"), ("A", "absent"), ("D", "dirs")):
            for p in rec[key]:
                if p.startswith(root):
                    idxn.setdefault((kind, p[len(root):]), set()).add(n)
                elif kind == "R":
                    ext = True
        untracked = any(p.startswith(root) and re.match(r"(binaries|lift/out-[^/]+)/", p[len(root):]) for p in rec["files"])
        spawns = [x for x in rec["spawns"] if x not in ("sh", "dash", "bash")]
        w, mem = rec.get("wall"), rec.get("rss_mb")
        tier = "fast" if not spawns and w is not None and w < 3 else "slow"
        wb = "" if w is None else str(next(i for i, lim in enumerate([2, 5, 15, 40, 1e9]) if w < lim))
        mb = "" if mem is None else str(next(i for i, lim in enumerate([300, 700, 1e9]) if mem < lim))
        meta.append((n, f"tier={tier}", "spawns=" + ",".join(spawns), f"ext={int(ext or untracked)}", f"wall={wb}", f"mem={mb}"))
    for n, m in sorted(old_meta.items()):
        if n not in names:
            meta.append((n, *[f"{k}={v}" for k, v in m.items()]))
    for n, *kv in sorted(meta):
        lines.append("\t".join(["B", n, *kv]))
    for k, v in old_idx.items():
        rest = v - names
        if rest:
            idxn.setdefault(k, set()).update(rest)
    for (kind, p), ns in sorted(idxn.items()):
        lines.append("\t".join([kind, p, ",".join(sorted(ns))]))
    DEPS_FILE.write_text("\n".join(lines) + "\n")
    log(f"[record] wrote {DEPS_FILE} ({len(lines)} lines, {len(allrecs)} executables from records)")


def cmd_record(args):
    resource.setrlimit(resource.RLIMIT_CORE, (1, 1))
    bins, _ = build()
    if args.only:
        bins = [b for b in bins if any(re.search(p, b.name) for p in args.only)]
    results = []
    if not args.collate_only:
        args.record_only = True
        args.fresh = True
        results, wall, outdir, stats = execute(bins, args, "record")
    collate_deps(bins)
    bad = [r for r in results if r.rc != 0]
    for r in bad:
        log(f"   FAILED {r.b.name}: {', '.join(r.failures[:6])}")
    return 1 if bad else 0


def cmd_names(args):
    resource.setrlimit(resource.RLIMIT_CORE, (1, 1))
    bins, _ = build()
    out = []
    for b in bins:
        p = subprocess.run([b.exe, "--list", "--format", "terse"], cwd=b.cwd, text=True, capture_output=True)
        for l in p.stdout.splitlines():
            if l.endswith(": test"):
                out.append(f"{b.name}::{l[:-6]}")
    out.sort()
    txt = "\n".join(out) + "\n"
    if args.out:
        Path(args.out).write_text(txt)
    print(f"{len(out)} tests, sha256 {hashlib.sha256(txt.encode()).hexdigest()}")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = ap.add_subparsers(dest="cmd", required=True)
    def common(p):
        p.add_argument("--jobs", type=int, default=int(os.environ.get("FLE_TEST_JOBS", "0")) or max(2, ncpu() * 2 // 3))
        p.add_argument("--threads", type=int, default=int(os.environ.get("FLE_TEST_LIBTEST_THREADS", "0")) or 4)
        p.add_argument("--only", action="append", help="regex on executable names, repeatable")
        p.add_argument("--fresh", action="store_true", help="ignore cached results")
        p.add_argument("--no-doc", action="store_true")
        p.add_argument("--collate-only", action="store_true", help="record: rebuild deps.tsv from the local records without running anything")
        p.add_argument("--record", action="store_true", help="run under strace to refresh the input sets")
        p.add_argument("--no-record", action="store_true", help="full: do not take the input sets of executables that have none (no strace, but nothing is stored for the result cache)")
        p.add_argument("--base", help="git revision to diff against (default HEAD, plus untracked files)")
        p.add_argument("--base-tree", help="a pristine copy of the tree to compare with (when the tree is not a git checkout)")
        p.add_argument("--files", help="file with the changed repo-relative paths, one per line")
        p.add_argument("test_args", nargs="*", help="after --: passed to every test executable")
        p.set_defaults(record_only=False, autorecord=False)
    r = sp.add_parser("run"); r.add_argument("--mode", choices=["full", "fast", "affected"], default="affected"); common(r)
    rc = sp.add_parser("record"); common(rc)
    n = sp.add_parser("names"); n.add_argument("--out")
    g = sp.add_parser("gc"); g.add_argument("--limit-mb", type=int, default=0)
    args = ap.parse_args()
    if args.cmd == "run":
        sys.exit(cmd_run(args))
    if args.cmd == "record":
        sys.exit(cmd_record(args))
    if args.cmd == "gc":
        gc_cache(args.limit_mb or None)
        sys.exit(0)
    sys.exit(cmd_names(args))


main()
