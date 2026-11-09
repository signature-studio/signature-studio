//! Regression test of the real-module mechanism (docs/decisions/0082-two-module-load-semantics.md, 0081; lift/runtime/fle_realmod.c) on
//! the synthetic PEs of stage 1 (docs/investigations/stage1-two-module.md). Native only; nothing here runs on a real Windows.
//!
//! The test assembles everything itself, once per test process, under lift/out-realmod/test/realmod-s1-<pid>/:
//!   - a synthetic "host" DLL (stands in for FLEngine, ImageBase 0x400000) and a synthetic second module (ImageBase 0x180000000,
//!     rebased to 0x2000000 with `fe_pe::rebase`), both built with mingw from tests/realmod_s1/;
//!   - the lifted C of both (fe_lift, `--pdata` seeds), split into units, with the state shared (one R/FL/X and one dispatch table);
//!   - all of runtime/fle_*.c, the harness image loader and tests/realmod_s1/driver.c, linked with clang.
//! The second module is made a real module only by realmod_table.c at link time (fle_realmod.h); the "base" driver is linked without it.
//!
//! 1. The 34 checks of the stage 1 `check_runs.py` (expectations written from Learn's DllMain/LoadLibrary/GetProcAddress pages, not from
//!    an output) pass on the runtime of this tree.
//! 2. Decision 0082: LoadLibraryExW, ordinal and forwarder stops, GetModuleFileNameW, the handles that must not change, FreeLibrary of
//!    the lifted DLL; decision 0081: the four API set names.
//! 3. A build without a table (or without fle_realmod.c) behaves as the runtime did before the real modules, and a wrong table stops.

mod etxt_common;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

const MOD_BASE: u64 = 0x200_0000;
const MOD_BASE_ARG: &str = "0x2000000";
const KERNEL32_HANDLE: u64 = 0x7FF8_0000_0000 + 0x10000 * 9;
/// Decision 0122: dwmapi.dll is virtual module N = 39 (handle 0x7FF800000000 + 0x10000 * N).
const DWMAPI_HANDLE: u64 = 0x7FF8_0000_0000 + 0x10000 * 39;
/// Decision 0081: N = 27..30 (position 27..30 in fle_modules[]).
const API_SET_HANDLES: [u64; 4] = [0x7FF8_0000_0000 + 0x10000 * 27, 0x7FF8_0000_0000 + 0x10000 * 28, 0x7FF8_0000_0000 + 0x10000 * 29, 0x7FF8_0000_0000 + 0x10000 * 30];

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn harness_dir() -> PathBuf {
    manifest().join("../../harness")
}
fn src_dir() -> PathBuf {
    manifest().join("tests/realmod_s1")
}

fn ok(mut cmd: Command, what: &str) -> String {
    let o = cmd.output().unwrap_or_else(|e| panic!("{what}: cannot run: {e}"));
    assert!(o.status.success(), "{what} failed ({:?}):\n{}\n{}", cmd, String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).to_string()
}

/// Runs the commands on a few threads; every one must succeed.
fn pool(jobs: Vec<(String, Command)>) {
    let q = Mutex::new(jobs);
    let fails = Mutex::new(Vec::<String>::new());
    std::thread::scope(|s| {
        for _ in 0..8 {
            s.spawn(|| loop {
                let job = q.lock().unwrap().pop();
                let Some((what, mut cmd)) = job else { break };
                match cmd.output() {
                    Ok(o) if o.status.success() => {}
                    Ok(o) => fails.lock().unwrap().push(format!("{what}: {}", String::from_utf8_lossy(&o.stderr))),
                    Err(e) => fails.lock().unwrap().push(format!("{what}: cannot run: {e}")),
                }
            });
        }
    });
    let f = fails.into_inner().unwrap();
    assert!(f.is_empty(), "build failed:\n{}", f.join("\n"));
}

fn clang_c(dir: &Path, src: &Path, obj: &Path, flags: &[&str], includes: &[PathBuf]) -> Command {
    let mut cmd = Command::new("clang");
    cmd.args(flags).arg("-c");
    for i in includes {
        cmd.arg("-I").arg(i);
    }
    cmd.arg(src).arg("-o").arg(obj).env("TMPDIR", dir.join("tmp"));
    cmd
}

const GEN_FLAGS: [&str; 5] = ["-std=gnu11", "-O0", "-w", "-fno-builtin", "-ffp-contract=off"];

struct Variant {
    /// host parts + module parts + common.c objects
    gen_objs: Vec<PathBuf>,
    mod_dll: PathBuf,
}

struct Built {
    dir: PathBuf,
    host_dll: PathBuf,
    rt_objs: Vec<PathBuf>,
    image_o: PathBuf,
    driver_o: PathBuf,
    table_o: PathBuf,
    main: Variant,
    neg: Variant,
    fwd: Variant,
    driver_rm: PathBuf,
    driver_base: PathBuf,
    driver_neg: PathBuf,
    driver_fwd: PathBuf,
    runs: PathBuf,
}

fn lift_c(pe_path: &Path, out_c: &Path) {
    let data = std::fs::read(pe_path).unwrap();
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let seeds = fe_decode::pdata_and_adjustor_seeds(&pe).unwrap();
    let mut w = BufWriter::new(File::create(out_c).unwrap());
    let st = fe_lift::generate_with_seeds(&data, &seeds, &mut w).unwrap();
    w.flush().unwrap();
    assert_eq!(st.bad_nodes, 0, "{}", pe_path.display());
    assert_eq!(st.unsupported_total, 0, "{}: {:?}", pe_path.display(), st.unsupported);
}

fn mingw(dir: &Path, base: &str, extra: &[PathBuf], out: &Path) {
    let mut cmd = Command::new("x86_64-w64-mingw32-gcc");
    cmd.args([
        "-O1", "-msse2", "-mno-avx", "-Wno-multichar", "-fno-stack-protector", "-fasynchronous-unwind-tables", "-fno-ident", "-ffreestanding",
        "-nostdlib", "-shared", "-Wl,--disable-auto-image-base", "-Wl,--dynamicbase", "-Wl,--file-alignment=0x200",
        "-Wl,--section-alignment=0x1000",
    ]);
    cmd.arg(format!("-Wl,--image-base={base}")).arg("-Wl,-e,DllMain").arg("-I").arg(src_dir()).arg("-o").arg(out);
    for e in extra {
        cmd.arg(e);
    }
    cmd.arg("-lkernel32").arg("-luser32").env("TMPDIR", dir.join("tmp"));
    ok(cmd, "mingw gcc (x86_64-w64-mingw32-gcc is needed)");
}

/// Splits the lifted C of one module into units (shared R/FL/X, tables written aside) and compiles the parts; returns the objects and the
/// tables file.
fn split_and_compile(dir: &Path, tag: &str, c: &Path, includes: &[PathBuf]) -> (Vec<PathBuf>, PathBuf) {
    let prefix = dir.join(format!("{tag}-part"));
    let mut cmd = Command::new("python3");
    cmd.arg(src_dir().join("tools/split_mod.py")).arg(c).arg(&prefix).arg("1000");
    ok(cmd, "split_mod.py");
    let tables = dir.join(format!("{tag}-part-tables.txt"));
    let mut parts: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with(&format!("{tag}-part-")) && n.ends_with(".c")
        })
        .collect();
    parts.sort();
    assert!(!parts.is_empty());
    let mut jobs = Vec::new();
    let mut objs = Vec::new();
    for p in &parts {
        let o = p.with_extension("o");
        jobs.push((p.display().to_string(), clang_c(dir, p, &o, &[&GEN_FLAGS[..], &["-fPIE"]].concat(), includes)));
        objs.push(o);
    }
    pool(jobs);
    (objs, tables)
}

fn build_variant(b: &Built0, tag: &str, mod_dll: PathBuf) -> Variant {
    let mod_c = b.dir.join(format!("{tag}-mod2.c"));
    lift_c(&mod_dll, &mod_c);
    let (mod_objs, mod_tables) = split_and_compile(&b.dir, &format!("{tag}-mod2"), &mod_c, &b.includes);
    let common_c = b.dir.join(format!("{tag}-common.c"));
    let mut cmd = Command::new("python3");
    cmd.arg(src_dir().join("tools/merge_tables.py")).arg(&b.host_tables).arg(&mod_tables).arg(&common_c);
    ok(cmd, "merge_tables.py");
    let common_o = common_c.with_extension("o");
    ok(clang_c(&b.dir, &common_c, &common_o, &[&GEN_FLAGS[..], &["-fPIE"]].concat(), &b.includes), "common.c");
    let mut gen_objs = b.host_objs.clone();
    gen_objs.extend(mod_objs);
    gen_objs.push(common_o);
    Variant { gen_objs, mod_dll }
}

/// What the first stage of the build leaves (before the variants).
struct Built0 {
    dir: PathBuf,
    includes: Vec<PathBuf>,
    host_objs: Vec<PathBuf>,
    host_tables: PathBuf,
}

fn link(dir: &Path, out: &Path, objs: &[&Path], extra: &[&Path], wrap: bool) {
    let mut cmd = Command::new("clang");
    cmd.arg("-pie").arg("-o").arg(out);
    for o in objs {
        cmd.arg(o);
    }
    for o in extra {
        cmd.arg(o);
    }
    cmd.arg("-lm");
    if wrap {
        cmd.arg("-Wl,--wrap=fle_call_guest");
    }
    cmd.env("TMPDIR", dir.join("tmp"));
    ok(cmd, "link");
}

fn built() -> &'static Built {
    static B: OnceLock<Built> = OnceLock::new();
    B.get_or_init(|| {
        let dir = manifest().join("../../out-realmod/test").join(etxt_common::per_process("realmod-s1"));
        std::fs::create_dir_all(dir.join("tmp")).unwrap();
        let rt = rt_dir();
        let includes = vec![rt.clone()];

        // 1. runtime, harness image loader, driver, table
        let mut rt_srcs: Vec<PathBuf> = std::fs::read_dir(&rt)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("fle_") && n.ends_with(".c")
            })
            .collect();
        rt_srcs.sort();
        assert!(rt_srcs.iter().any(|p| p.ends_with("fle_realmod.c")));
        std::fs::create_dir_all(dir.join("rt")).unwrap();
        let mut jobs = Vec::new();
        let mut rt_objs = Vec::new();
        for s in &rt_srcs {
            let o = dir.join("rt").join(s.file_name().unwrap()).with_extension("o");
            jobs.push((s.display().to_string(), clang_c(&dir, s, &o, &[&GEN_FLAGS[..], &["-fPIE"]].concat(), &includes)));
            rt_objs.push(o);
        }
        let image_o = dir.join("image.o");
        jobs.push(("image.c".into(), clang_c(&dir, &harness_dir().join("image.c"), &image_o, &["-std=gnu11", "-O1", "-Wall", "-Wextra", "-fPIE"], &[harness_dir()])));
        let driver_o = dir.join("driver.o");
        jobs.push((
            "driver.c".into(),
            clang_c(&dir, &src_dir().join("driver.c"), &driver_o, &["-std=gnu11", "-O1", "-Wall", "-Wextra", "-fPIE"], &[rt.clone(), harness_dir()]),
        ));
        let table_o = dir.join("realmod_table.o");
        jobs.push((
            "realmod_table.c".into(),
            clang_c(&dir, &src_dir().join("realmod_table.c"), &table_o, &["-std=gnu11", "-O1", "-Wall", "-Wextra", "-fPIE"], &[rt.clone()]),
        ));

        // 2. the synthetic PEs
        let host_dll = dir.join("host.dll");
        mingw(&dir, "0x400000", &[src_dir().join("guest_host.c")], &host_dll);
        // the string table of mod2.rc gives the second module an RT_STRING resource (decision 0102 W2)
        let rc_o = dir.join("mod2_rc.o");
        let mut rc = Command::new("x86_64-w64-mingw32-windres");
        rc.args(["-O", "coff"]).arg(src_dir().join("mod2.rc")).arg(&rc_o).env("TMPDIR", dir.join("tmp"));
        ok(rc, "x86_64-w64-mingw32-windres (mingw binutils) is needed");
        let orig = dir.join("mod2.orig.dll");
        mingw(&dir, "0x180000000", &[src_dir().join("guest_mod2.c"), rc_o], &orig);
        let orig_fwd = dir.join("mod2fwd.orig.dll");
        mingw(&dir, "0x180000000", &[src_dir().join("guest_mod2.c"), src_dir().join("mod2.def")], &orig_fwd);

        let rebase = |from: &Path, to: &Path| {
            let data = std::fs::read(from).unwrap();
            let (out, rep) = fe_pe::rebase::rebase(&data, MOD_BASE).unwrap();
            assert!(rep.dir64 >= 1, "the synthetic module has a DIR64 slot");
            let mut f = File::options().write(true).create_new(true).open(to).unwrap();
            f.write_all(&out).unwrap();
        };
        let mod_dll = dir.join("mod2.r2000000.dll");
        rebase(&orig, &mod_dll);
        let fwd_dll = dir.join("mod2fwd.r2000000.dll");
        rebase(&orig_fwd, &fwd_dll);
        // negative control: only the ImageBase field changes, no DIR64 is applied
        let neg_dll = dir.join("mod2neg.r2000000.dll");
        let mut cmd = Command::new("python3");
        cmd.arg(src_dir().join("tools/patch_base_only.py")).arg(&orig).arg(&neg_dll).arg(MOD_BASE_ARG);
        ok(cmd, "patch_base_only.py");

        // 3. lift and compile the host once, then one set of module units per variant
        let host_c = dir.join("host.c");
        lift_c(&host_dll, &host_c);
        let (host_objs, host_tables) = split_and_compile(&dir, "host", &host_c, &includes);
        pool(jobs);
        let b0 = Built0 { dir: dir.clone(), includes, host_objs, host_tables };
        let main = build_variant(&b0, "main", mod_dll);
        let neg = build_variant(&b0, "neg", neg_dll);
        let fwd = build_variant(&b0, "fwd", fwd_dll);

        // 4. link
        let rt_refs: Vec<&Path> = rt_objs.iter().map(|p| p.as_path()).collect();
        let gen_refs = |v: &Variant| -> Vec<PathBuf> { v.gen_objs.clone() };
        let drv = |name: &str, v: &Variant, with_table: bool| -> PathBuf {
            let out = dir.join(name);
            let g = gen_refs(v);
            let mut objs: Vec<&Path> = vec![driver_o.as_path(), image_o.as_path()];
            objs.extend(g.iter().map(|p| p.as_path()));
            objs.extend(rt_refs.iter().copied());
            if with_table {
                objs.push(table_o.as_path());
            }
            link(&dir, &out, &objs, &[], true);
            out
        };
        let driver_rm = drv("driver-rm", &main, true);
        let driver_base = drv("driver-base", &main, false);
        let driver_neg = drv("driver-neg", &neg, true);
        let driver_fwd = drv("driver-fwd", &fwd, true);

        let built = Built {
            dir: dir.clone(),
            host_dll,
            rt_objs,
            image_o,
            driver_o,
            table_o,
            main,
            neg,
            fwd,
            driver_rm,
            driver_base,
            driver_neg,
            driver_fwd,
            runs: dir.join("runs"),
        };
        run_all(&built);
        built
    })
}

struct Run {
    log: Vec<(String, u64)>,
    exit: i32,
    stdout: String,
    stderr: String,
}

fn exec(driver: &Path, host: &Path, module: &Path, scenario: &str, mode: u32) -> Run {
    let o = Command::new(driver).arg(host).arg(module).arg(scenario).arg(mode.to_string()).output().expect("run failed");
    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
    let mut log = Vec::new();
    for l in stdout.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() == 4 && f[0] == "log" {
            log.push((f[2].to_string(), u64::from_str_radix(f[3].trim_start_matches("0x"), 16).unwrap()));
        }
    }
    let exit = o.status.code().unwrap_or(128 + o.status.signal().unwrap_or(0));
    Run { log, exit, stdout, stderr: String::from_utf8_lossy(&o.stderr).to_string() }
}

fn tags(r: &Run) -> Vec<&str> {
    r.log.iter().map(|(t, _)| t.as_str()).collect()
}
fn val(r: &Run, tag: &str) -> Option<u64> {
    r.log.iter().find(|(t, _)| t == tag).map(|(_, v)| *v)
}
fn pos(r: &Run, tag: &str) -> usize {
    r.log.iter().position(|(t, _)| t == tag).unwrap_or_else(|| panic!("no {tag} in {:?}", tags(r)))
}

/// The scenario matrix of the stage 1 run_all.sh, written under runs/ with the file names that check_runs.py reads.
fn run_all(b: &Built) {
    std::fs::create_dir_all(&b.runs).unwrap();
    let host = &b.host_dll;
    let m = &b.main.mod_dll;
    let rows: [(&Path, &Path, &str, u32, &str); 12] = [
        (&b.driver_rm, m, "HostBasic", 0, "s1-basic"),
        (&b.driver_rm, m, "HostBasic", 2, "s1-basic-selfload"),
        (&b.driver_rm, m, "HostFail", 1, "s1-fail-always"),
        (&b.driver_rm, m, "HostFail", 8, "s1-fail-first"),
        (&b.driver_rm, m, "HostRefs", 0, "s1-refs"),
        (&b.driver_rm, m, "HostRefs", 2, "s1-refs-selfload"),
        (&b.driver_rm, m, "HostRefsVirtual", 0, "s1-refs-virtual"),
        (&b.driver_rm, m, "HostFreeBase", 0, "s1-freebase"),
        (&b.driver_base, m, "HostLoadOnly", 0, "base-loadonly"),
        (&b.driver_base, m, "HostBasic", 0, "base-basic"),
        (&b.driver_base, m, "HostRefsVirtual", 0, "base-refs-virtual"),
        (&b.driver_base, m, "HostFreeBase", 0, "base-freebase"),
    ];
    for (drv, module, scen, mode, tag) in rows {
        write_run(&b.runs, tag, &exec(drv, host, module, scen, mode));
    }
    write_run(&b.runs, "neg-header-only", &exec(&b.driver_neg, host, &b.neg.mod_dll, "HostBasic", 0));
}

fn write_run(dir: &Path, tag: &str, r: &Run) {
    let mut out = File::options().write(true).create_new(true).open(dir.join(format!("{tag}.out"))).unwrap();
    write!(out, "{}exit={}\n", r.stdout, r.exit).unwrap();
    let mut err = File::options().write(true).create_new(true).open(dir.join(format!("{tag}.err"))).unwrap();
    err.write_all(r.stderr.as_bytes()).unwrap();
}

// ---- 1. the stage 1 checks ----

#[test]
fn the_34_stage1_checks_pass_on_this_runtime() {
    let b = built();
    let o = Command::new("python3").arg(src_dir().join("check_runs.py")).arg(&b.runs).env("PYTHONDONTWRITEBYTECODE", "1").output().expect("python3");
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "check_runs.py failed:\n{out}\n{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(out.lines().filter(|l| l.starts_with("PASS ")).count(), 34, "{out}");
    assert_eq!(out.lines().filter(|l| l.starts_with("FAIL ")).count(), 0, "{out}");
}

// ---- 2. decision 0082 and 0081 ----

#[test]
fn load_library_ex_w_attaches_before_it_returns_and_get_proc_address_gives_base_plus_rva() {
    let b = built();
    let r = exec(&b.driver_rm, &b.host_dll, &b.main.mod_dll, "HostExFirst", 0);
    assert_eq!(r.exit, 0, "{}", r.stderr);
    assert!(pos(&r, "DENT") < pos(&r, "HAFT"), "{:?}", tags(&r));
    assert_eq!(val(&r, "DENT"), Some(1));
    assert_eq!(val(&r, "DHIN"), Some(MOD_BASE));
    assert_eq!(val(&r, "DRES"), Some(0));
    assert_eq!(val(&r, "HAFT"), Some(MOD_BASE));
    assert_eq!(val(&r, "HPAD"), Some(0x200_1000));
}

#[test]
fn get_proc_address_by_ordinal_stops_by_name() {
    let b = built();
    let r = exec(&b.driver_rm, &b.host_dll, &b.main.mod_dll, "HostOrdinal", 0);
    assert_eq!(r.exit, 134, "{}", r.stdout);
    assert!(r.stderr.contains("fle: GetProcAddress by ordinal (lpProcName 0x1) is not supported"), "{}", r.stderr);
    assert_eq!(val(&r, "HAFT"), Some(MOD_BASE));
    assert_eq!(val(&r, "HORD"), None);
}

#[test]
fn an_export_forwarder_stops_by_name() {
    let b = built();
    let r = exec(&b.driver_fwd, &b.host_dll, &b.fwd.mod_dll, "HostFwd", 0);
    assert_eq!(r.exit, 134, "{}", r.stdout);
    assert!(r.stderr.contains("fle: GetProcAddress: an export forwarder of a real module is not modelled"), "{}", r.stderr);
    assert_eq!(val(&r, "HAFT"), Some(MOD_BASE));
    assert_eq!(val(&r, "HFWD"), None);
    // the forwarder entry does not disturb the other exports
    let r = exec(&b.driver_fwd, &b.host_dll, &b.fwd.mod_dll, "HostExFirst", 0);
    assert_eq!(r.exit, 0, "{}", r.stderr);
}

#[test]
fn get_module_file_name_w_of_a_real_module_answers_the_shared_directory_and_null_keeps_its_answer() {
    // Decision 0104 (provisional) replaces the named stop of decision 0082 D6: the path of a real module is
    // "C:\Program Files\Image-Line\FL Studio 21\Shared\" (48 characters) + the Name field of the module's export directory, which the
    // linker sets to the output file name of mingw ("mod2.orig.dll", 13 characters; fe-pe rebase does not change it).
    let b = built();
    let r = exec(&b.driver_rm, &b.host_dll, &b.main.mod_dll, "HostGmfn", 0);
    assert_eq!(r.exit, 0, "{}", r.stderr);
    assert_eq!(val(&r, "HGN0"), Some(1), "GetModuleFileNameW(NULL) still answers (decision 0018)");
    assert_eq!(val(&r, "HAFT"), Some(MOD_BASE));
    let dir = "C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\";
    assert_eq!(dir.len(), 48);
    assert_eq!(val(&r, "HGN1"), Some(48 + 13));
    // the characters and the truncation rule (decision 0102 W1 added these observations to the guest; the path itself is that of decision 0104)
    assert_eq!(val(&r, "HGC0"), Some('C' as u64));
    assert_eq!(val(&r, "HGC1"), Some(':' as u64));
    assert_eq!(val(&r, "HGCL"), Some('l' as u64), "the last character of mod2.orig.dll");
    assert_eq!(val(&r, "HGCZ"), Some(0), "NUL after the path");
    assert_eq!(val(&r, "HGN2"), Some(5), "truncated: nSize is returned");
    assert_eq!(val(&r, "HGE2"), Some(122));
    assert_eq!(val(&r, "HGC4"), Some(0), "the truncated string ends with a NUL");
    assert_eq!(val(&r, "HGC3"), Some(dir.as_bytes()[3] as u64));
    assert_ne!(MOD_BASE, 0x400000);
}

/// Decision 0102 W2 (provisional; the implementation is that of decision 0104, fle_res.c): the handle (= base) of a real module selects that
/// module's PE resources in LoadStringW.
#[test]
fn load_string_w_of_a_real_module_reads_its_own_string_table() {
    let b = built();
    let r = exec(&b.driver_rm, &b.host_dll, &b.main.mod_dll, "HostResMod", 0);
    assert_eq!(r.exit, 0, "{}", r.stderr);
    assert_eq!(val(&r, "HAFT"), Some(MOD_BASE));
    assert_eq!(val(&r, "HLS1"), Some(8), "\"mod2 one\" is 8 characters");
    assert_eq!(val(&r, "HLC0"), Some('m' as u64));
    assert_eq!(val(&r, "HLC7"), Some('e' as u64));
    assert_eq!(val(&r, "HLZ1"), Some(0));
    assert_eq!(val(&r, "HLS2"), Some(3), "a buffer of 4 holds 3 characters + NUL");
    assert_eq!(val(&r, "HLC2"), Some('d' as u64), "ID 2 is \"mod2 two\"; the first 3 characters \"mod\" are copied, so buf[2] is 'd'");
    assert_eq!(val(&r, "HLZ2"), Some(0));
    assert_eq!(val(&r, "HLS3"), Some(0), "ID 3 is in the block but unused");
    assert_eq!(val(&r, "HLE3"), Some(1814), "ERROR_RESOURCE_NAME_NOT_FOUND, as for the lifted DLL (decision 0025)");
}

#[test]
fn the_exe_handle_and_virtual_handles_do_not_change() {
    let b = built();
    let with = exec(&b.driver_rm, &b.host_dll, &b.main.mod_dll, "HostHandles", 0);
    let without = exec(&b.driver_base, &b.host_dll, &b.main.mod_dll, "HostHandles", 0);
    for r in [&with, &without] {
        assert_eq!(r.exit, 0, "{}", r.stderr);
        assert_eq!(val(r, "HGN0"), Some(0x400000), "GetModuleHandleW(NULL)");
        assert_eq!(val(r, "HGK3"), Some(KERNEL32_HANDLE));
        assert_eq!(val(r, "HGM0"), Some(0), "a real module is not found before it is loaded");
        assert_eq!(val(r, "HGE0"), Some(126));
    }
    assert_eq!(with.log, without.log);
}

/// Decision 0091: GetModuleHandleA of virtual names, NULL, a name that does not exist and the real module that is not loaded gives what
/// GetModuleHandleW gives, with and without a real-module table (the last error is 5 before each call and 126 after a failure only).
#[test]
fn get_module_handle_a_names_that_need_no_load_answer_like_w() {
    let b = built();
    let with = exec(&b.driver_rm, &b.host_dll, &b.main.mod_dll, "HostGmhNames", 0);
    let without = exec(&b.driver_base, &b.host_dll, &b.main.mod_dll, "HostGmhNames", 0);
    for r in [&with, &without] {
        assert_eq!(r.exit, 0, "{}", r.stderr);
        assert_eq!(val(r, "HAN0"), Some(0x400000), "GetModuleHandleA(NULL)");
        assert_eq!(val(r, "HAE0"), Some(5), "NULL leaves the last error alone");
        assert_eq!(val(r, "HAK3"), Some(KERNEL32_HANDLE));
        assert_eq!(val(r, "HWK3"), val(r, "HAK3"));
        assert_eq!(val(r, "HAS1"), Some(API_SET_HANDLES[0]));
        assert_eq!(val(r, "HWS1"), val(r, "HAS1"));
        assert_eq!(val(r, "HAM0"), Some(0), "the real module is not found before it is loaded");
        assert_eq!(val(r, "HAE1"), Some(126));
        assert_eq!(val(r, "HWM0"), val(r, "HAM0"));
        assert_eq!(val(r, "HWE1"), val(r, "HAE1"));
        assert_eq!(val(r, "HAX0"), Some(0));
        assert_eq!(val(r, "HAE2"), Some(126));
    }
    assert_eq!(with.log, without.log);
}

/// Decision 0091 and 0082: GetModuleHandleA of a real module is NULL and 126 before the load, the base once loaded (any case, with or
/// without ".dll"; a trailing point means no extension), and, inside the module's own DllMain, "not loaded" exactly as GetModuleHandleW
/// answers (decision 0082 keeps that open; no new treatment here).
#[test]
fn get_module_handle_a_of_a_real_module_follows_w_before_during_and_after_the_load() {
    let b = built();
    let r = exec(&b.driver_rm, &b.host_dll, &b.main.mod_dll, "HostGmhA", 16);
    assert_eq!(r.exit, 0, "{}", r.stderr);
    // before the load
    assert_eq!(val(&r, "HAB0"), Some(0));
    assert_eq!(val(&r, "HAE0"), Some(126));
    // inside DllMain (mode bit 16 of the module): not loaded for A and W alike, NULL still the exe handle, virtual names unchanged
    assert_eq!(val(&r, "DGAN"), Some(0x400000));
    assert_eq!(val(&r, "DGAM"), Some(0), "A during DllMain");
    assert_eq!(val(&r, "DGWM"), Some(0), "W during DllMain");
    assert_eq!(val(&r, "DGAE"), Some(126));
    assert_eq!(val(&r, "DGWE"), Some(126));
    assert_eq!(val(&r, "DGAK"), Some(KERNEL32_HANDLE));
    // after the load
    assert_eq!(val(&r, "HAFT"), Some(MOD_BASE));
    assert_eq!(val(&r, "HAB1"), Some(MOD_BASE), "A, upper case, after the load");
    assert_eq!(val(&r, "HAE1"), Some(5), "success leaves the last error alone");
    assert_eq!(val(&r, "HAB2"), Some(MOD_BASE), "A, no extension");
    assert_eq!(val(&r, "HWB2"), Some(MOD_BASE));
    assert_eq!(val(&r, "HAB3"), Some(0), "a trailing point means no extension: 'mod2.' is 'mod2', which is not 'mod2.dll'");
    assert_eq!(val(&r, "HAE3"), Some(126));
    assert_eq!(val(&r, "HWB3"), val(&r, "HAB3"));
    assert_eq!(val(&r, "HAB4"), Some(MOD_BASE), "'mod2.dll.' is 'mod2.dll' (the point is removed, nothing is appended)");
    assert_eq!(val(&r, "HWB4"), val(&r, "HAB4"));
    // the order: the DllMain lines come between HAB0 and HAFT
    assert!(pos(&r, "HAB0") < pos(&r, "DENT") && pos(&r, "DGAM") < pos(&r, "HAFT"), "{:?}", tags(&r));
}

#[test]
fn free_library_of_the_lifted_dll_stops_with_and_without_a_real_module() {
    let b = built();
    for drv in [&b.driver_rm, &b.driver_base] {
        let r = exec(drv, &b.host_dll, &b.main.mod_dll, "HostFreeExe", 0);
        assert_eq!(r.exit, 134, "{}", r.stdout);
        assert!(r.stderr.contains("fle: FreeLibrary: hLibModule is the lifted DLL itself (not modelled)"), "{}", r.stderr);
        assert_eq!(val(&r, "HFRE"), None);
    }
}

#[test]
fn the_four_api_set_names_are_modules_with_nothing_to_find() {
    let b = built();
    let with = exec(&b.driver_rm, &b.host_dll, &b.main.mod_dll, "HostApiSet", 0);
    let without = exec(&b.driver_base, &b.host_dll, &b.main.mod_dll, "HostApiSet", 0);
    for r in [&with, &without] {
        assert_eq!(r.exit, 0, "{}", r.stderr);
        for (i, tag) in ["HAS1", "HAS2", "HAS3", "HAS4"].iter().enumerate() {
            assert_eq!(val(r, tag), Some(API_SET_HANDLES[i]), "{tag}");
        }
        assert_eq!(val(r, "HGM1"), Some(API_SET_HANDLES[0]));
        // no response row has an api-ms-* dll: NULL and ERROR_PROC_NOT_FOUND
        for tag in ["HPA1", "HPA2", "HPA3", "HPA4"] {
            assert_eq!(val(r, tag), Some(0), "{tag}");
        }
        assert_eq!(val(r, "HPE1"), Some(127));
        assert_eq!(val(r, "HPE4"), Some(127));
    }
    assert_eq!(with.log, without.log);
}

// ---- 3. no table, and a wrong table ----

// Decision 0233 (load1, PROVISIONAL; old: HEAD_STOP, the stop "fle: LoadLibraryW: module \"dsp_ippv2_x64.dll\" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no
// failure is invented for them)"): a name that no module claims and that is not a DLL of Windows' roster is searched for on the (empty) volume, which ends without a module: NULL and ERROR_MOD_NOT_FOUND (LoadLibraryW page: "If the
// function cannot find the module, the function fails"; "If the function fails, the return value is NULL"; 126 by decision 0015, Wine 10.0 UNOFFICIAL). The call returns instead of stopping.
const NOT_FOUND: &str = "returned 0 le 126";

struct Tiny {
    exe: PathBuf,
}

/// A small program that makes one loader call (tiny_main.c) over the generated units and the runtime, with the given table (None: no
/// realmod_table.c) and with or without fle_realmod.o.
fn tiny(name: &str, table_defs: Option<&[&str]>, with_realmod_obj: bool) -> Tiny {
    let b = built();
    let dir = &b.dir;
    let uname = etxt_common::uniq(name);
    let main_o = dir.join(format!("{uname}-main.o"));
    let mut cmd = clang_c(dir, &src_dir().join("tiny_main.c"), &main_o, &["-std=gnu11", "-O0", "-w", "-fno-builtin", "-fPIE"], &[rt_dir()]);
    cmd.env("TMPDIR", dir.join("tmp"));
    ok(cmd, "tiny_main.c");
    let mut table_obj = None;
    if let Some(defs) = table_defs {
        let o = dir.join(format!("{uname}-table.o"));
        let mut c = Command::new("clang");
        c.args(["-std=gnu11", "-O0", "-w", "-fPIE", "-c"]).arg("-I").arg(rt_dir());
        for d in defs {
            c.arg(format!("-D{d}"));
        }
        c.arg(src_dir().join("realmod_table.c")).arg("-o").arg(&o).env("TMPDIR", dir.join("tmp"));
        ok(c, "realmod_table.c");
        table_obj = Some(o);
    }
    let exe = dir.join(&uname);
    let mut objs: Vec<&Path> = vec![main_o.as_path()];
    objs.extend(b.main.gen_objs.iter().map(|p| p.as_path()));
    objs.extend(b.rt_objs.iter().filter(|p| with_realmod_obj || !p.ends_with("fle_realmod.o")).map(|p| p.as_path()));
    if let Some(t) = &table_obj {
        objs.push(t.as_path());
    }
    link(dir, &exe, &objs, &[], false);
    Tiny { exe }
}

impl Tiny {
    fn call(&self, api: &str, name: &str) -> (Option<i32>, String, String) {
        let o = Command::new(&self.exe).arg(api).arg(name).output().expect("run failed");
        (o.status.signal(), String::from_utf8_lossy(&o.stdout).trim().to_string(), String::from_utf8_lossy(&o.stderr).to_string())
    }
}

#[test]
fn without_a_table_the_loader_is_what_it_was() {
    let _ = built();
    // (a) linked with fle_realmod.c, no table: the weak table has count 0; (b) linked without fle_realmod.c at all: weak references
    let variants = [("notable", tiny("tiny-notable", None, true)), ("noobj", tiny("tiny-noobj", None, false))];
    for (label, t) in &variants {
        let (sig, out, _) = t.call("llw", "dsp_ippv2_x64.dll");
        assert_eq!((sig, out.as_str()), (None, NOT_FOUND), "{label}: decision 0233: the answer for a name that nothing claims is the page's failure (old: the stop of decision 0082)");
        let (sig, out, _) = t.call("llw", "mod2.dll");
        assert_eq!((sig, out.as_str()), (None, NOT_FOUND), "{label}");
        assert_eq!(t.call("gmh", "mod2.dll").1, "returned 0 le 126", "{label}");
        assert_eq!(t.call("llw", "kernel32.dll").1, format!("returned {KERNEL32_HANDLE:x} le 5"), "{label}");
        // decision 0122: dwmapi.dll is virtual module N = 39 (old: the absent answer, "returned 0 le 126"); the name that stays absent in this test is propsys
        assert_eq!(t.call("llw", "dwmapi").1, format!("returned {DWMAPI_HANDLE:x} le 5"), "{label}");
        assert_eq!(t.call("llw", "propsys").1, "returned 0 le 126", "{label}");
        assert_eq!(t.call("llw", "api-ms-win-core-fibers-l1-1-1").1, format!("returned {:x} le 5", API_SET_HANDLES[1]), "{label}");
    }
}

#[test]
fn a_table_does_not_change_what_it_does_not_name() {
    let _ = built();
    let t = tiny("tiny-table", Some(&[]), true);
    // the table names mod2.dll only: another external name keeps the old stop, virtual names keep their handles, the real module is not found
    // by GetModuleHandleW before it is loaded (the image is not even read)
    let (sig, out, _) = t.call("llw", "dsp_ippv2_x64.dll");
    assert_eq!((sig, out.as_str()), (None, NOT_FOUND));
    assert_eq!(t.call("gmh", "mod2.dll").1, "returned 0 le 126");
    assert_eq!(t.call("gmh", "MOD2").1, "returned 0 le 126");
    assert_eq!(t.call("llw", "kernel32.dll").1, format!("returned {KERNEL32_HANDLE:x} le 5"));
    // decision 0122: dwmapi.dll is virtual module N = 39 (old: "returned 0 le 126"); propsys.dll is still absent
    assert_eq!(t.call("llw", "dwmapi").1, format!("returned {DWMAPI_HANDLE:x} le 5"));
    assert_eq!(t.call("llw", "propsys").1, "returned 0 le 126");
}

#[test]
fn a_table_that_breaks_the_rules_stops_by_name() {
    let _ = built();
    let cases: [(&str, [&str; 3], &str); 7] = [
        ("virtual-name", ["RM_ENTRIES={ \"kernel32.dll\", 0x2000000 }", "RM_COUNT=1", "X=1"], "has the name of a virtual module"),
        ("exe-base", ["RM_ENTRIES={ \"mod2.dll\", 0x400000 }", "RM_COUNT=1", "X=1"], "is not usable"),
        ("unaligned-base", ["RM_ENTRIES={ \"mod2.dll\", 0x2000001 }", "RM_COUNT=1", "X=1"], "is not usable"),
        ("handle-region-base", ["RM_ENTRIES={ \"mod2.dll\", 0x7FF800010000 }", "RM_COUNT=1", "X=1"], "is not usable"),
        ("upper-case", ["RM_ENTRIES={ \"Mod2.dll\", 0x2000000 }", "RM_COUNT=1", "X=1"], "is not lower-case"),
        ("same-base", ["RM_ENTRIES={ \"mod2.dll\", 0x2000000 }, { \"mod3.dll\", 0x2000000 }", "RM_COUNT=2", "X=1"], "repeats a name or a base"),
        (
            "nine-modules",
            [
                "RM_ENTRIES={ \"a.dll\", 0x2000000 }, { \"b.dll\", 0x3000000 }, { \"c.dll\", 0x4000000 }, { \"d.dll\", 0x5000000 }, { \"e.dll\", 0x6000000 }, { \"f.dll\", 0x7000000 }, { \"g.dll\", 0x8000000 }, { \"h.dll\", 0x9000000 }, { \"i.dll\", 0xa000000 }",
                "RM_COUNT=9",
                "X=1",
            ],
            "larger than the 8 real modules",
        ),
    ];
    for (label, defs, say) in cases {
        let t = tiny(&format!("tiny-bad-{label}"), Some(&defs), true);
        let (sig, out, err) = t.call("llw", "mod2.dll");
        assert_eq!(sig, Some(6), "{label}: {out} {err}");
        assert!(err.contains("fle:") && err.contains(say), "{label}: {err}");
        assert!(!out.contains("returned"), "{label}");
    }
}

/// Decision 0106: RM_MAX is 8 (it was 4). A table of eight entries passes the table check, which runs at the first lookup, so a name
/// outside the table gets the loader's usual stop and not the "larger than" stop of the check (a table of nine entries gets that one,
/// in a_table_that_breaks_the_rules_stops_by_name). No image is mapped in this program, so no entry is loaded (that would read it).
#[test]
fn a_table_of_eight_modules_passes_the_table_check() {
    let _ = built();
    let t = tiny(
        "tiny-eight",
        Some(&[
            "RM_ENTRIES={ \"a.dll\", 0x2000000 }, { \"b.dll\", 0x3000000 }, { \"c.dll\", 0x4000000 }, { \"d.dll\", 0x5000000 }, { \"e.dll\", 0x6000000 }, { \"f.dll\", 0x7000000 }, { \"g.dll\", 0x8000000 }, { \"h.dll\", 0x9000000 }",
            "RM_COUNT=8",
            "X=1",
        ]),
        true,
    );
    let (sig, out, err) = t.call("llw", "dsp_ippv2_x64.dll");
    assert_eq!((sig, out.as_str()), (None, NOT_FOUND), "{err}");
    assert_eq!(t.call("gmh", "h.dll").1, "returned 0 le 126");
    assert_eq!(t.call("gmh", "e.dll").1, "returned 0 le 126");
    assert_eq!(t.call("llw", "kernel32.dll").1, format!("returned {KERNEL32_HANDLE:x} le 5"));
}

// ---- negative control of the harness itself ----

#[test]
fn the_module_is_a_real_module_only_through_the_link_table() {
    let b = built();
    // the same host/module images and the same runtime: with the table the module loads (stage 1 checks), without it the loader stops by name
    // decision 0233 (old: exit 134 and the stop "module \"mod2.dll\" is not a virtual module", tags ["HBEF"]): the loader fails the load like the page says, NULL, and the host goes on; DllMain of the module never ran
    let r = exec(&b.driver_base, &b.host_dll, &b.main.mod_dll, "HostLoadOnly", 0);
    assert_eq!(r.exit, 0, "{}", r.stderr);
    assert_eq!(val(&r, "HAFT"), Some(0));
    assert_eq!(tags(&r), ["HBEF", "HAFT"]);
    let r = exec(&b.driver_rm, &b.host_dll, &b.main.mod_dll, "HostLoadOnly", 0);
    assert_eq!(r.exit, 0, "{}", r.stderr);
    assert_eq!(val(&r, "HAFT"), Some(MOD_BASE));
    let _ = (&b.image_o, &b.driver_o, &b.table_o);
}
