//! Tests for GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, AddAtomW, DeleteAtom, FindAtomW, GlobalAddAtomW,
//! GlobalFindAtomW, MulDiv and lstrlenW in runtime/fle_glob.c (docs/decisions/0024-apis-a3-a5-a13.md; sources:
//! docs/investigations/apis-a3-a5-a13-run.md).
//!
//! Expected values are written here from the rules on the Microsoft pages (GlobalAlloc, GlobalFree, GlobalLock,
//! GlobalUnlock, AddAtom, DeleteAtom, FindAtom, GlobalAddAtom, GlobalFindAtom, About Atom Tables, MulDiv, lstrlenW,
//! System Error Codes), not read back from the C code. A value the pages do not give is marked "assumption" or
//! "inference" and is the only thing such a test pins down. The host is Linux: nothing is compared with a real Windows.
//!
//! The C sources come from runtime/ when fle_vmap.c (the last file of the unit set) is there, otherwise from
//! out-apis/work/ (development).

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if runtime_dir().join("fle_vmap.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-apis/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-apis/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd
        .arg("-I")
        .arg(src_dir())
        .arg("-I")
        .arg(runtime_dir())
        .arg(src)
        .arg("-o")
        .arg(obj)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-glob"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_glob.c", true), ("fle_imports.c", false), ("fle_env.c", false)] {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
            v.push(o);
        }
        v
    })
}

pub struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_glob.h"

uint64_t R[16];
static uint64_t stk[32];

static uint64_t callv(void (*f)(void), const uint64_t *a)
{
    int i;
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    for (i = 4; i < 12; i++) stk[i + 1] = a[i];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define CALL(f, ...) callv((void (*)(void))(f), (uint64_t[12]){ __VA_ARGS__ })
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
"#;

fn run_src(name: &str, top: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn run(name: &str, body: &str) -> Out {
    run_src(name, "", body)
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn assert_aborts(prefix: &str, setup: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{label}: no fle: message: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

/// A C expression for a guest UTF-16 string.
fn cw(s: &str) -> String {
    let mut v: Vec<String> = s.encode_utf16().map(|u| u.to_string()).collect();
    v.push("0".into());
    format!("A(((const uint16_t[]){{ {} }}))", v.join(", "))
}

fn header_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.contains(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let tok = line.split("UINT64_C(").nth(1).unwrap().split(')').next().unwrap();
    u64::from_str_radix(tok.trim_start_matches("0x"), 16).unwrap()
}

// ---- 1. Table, header, handle region ----

const A3: [&str; 11] = [
    "GlobalAlloc",
    "GlobalFree",
    "GlobalLock",
    "GlobalUnlock",
    "AddAtomW",
    "DeleteAtom",
    "FindAtomW",
    "GlobalAddAtomW",
    "GlobalFindAtomW",
    "MulDiv",
    "lstrlenW",
];

#[test]
fn rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_glob.h")).unwrap();
    for n in A3 {
        let row = format!("(\"kernel32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_glob.h");
    }
}

#[test]
fn movable_handle_region_does_not_overlap_the_other_regions() {
    let base = header_const("fle_glob.h", "FLE_GLOBAL_HANDLE_BASE");
    let size = header_const("fle_glob.h", "FLE_GLOBAL_REGION_SIZE");
    // other users of 0x7FF7.. / 0x7FF8.. / 0x7FF9.. / 0x7FFA.. (fle_sys.h, fle_modules.h, imports_table.rs, fle_vfs.h)
    let heap = 0x7FF7_0000_0000u64;
    let mods_end = 0x7FF8_0000_0000u64 + 0x10000 * 34;
    let resp_end = 0x7FF9_0000_0000u64 + 0x10 * 5000;
    assert!(base > heap + 0x1000_0000 && base > mods_end && base > resp_end);
    for n in ["FLE_VFS_FILE_BASE", "FLE_VFS_FIND_BASE", "FLE_REG_KEY_BASE"] {
        let o = header_const("fle_vfs.h", n);
        let osz = header_const("fle_vfs.h", "FLE_HANDLE_REGION_SIZE");
        assert!(base >= o + osz || o >= base + size, "overlaps {n}");
    }
    assert!(base + size < (1u64 << 47));
}

// ---- 2. GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock ----

#[test]
fn fixed_memory_handle_is_the_pointer_and_the_lock_count_is_zero() {
    let l = lines(
        "glob-fixed",
        r#"
    uint64_t p = CALL(fle_imp_GlobalAlloc, 0, 16), l, r;
    printf("a %d %d\n", p != 0, (p & 7) == 0);
    memset((void *)p, 0x5A, 16);
    l = CALL(fle_imp_GlobalLock, p);
    printf("lock_same %d\n", l == p);
    r = CALL(fle_imp_GlobalUnlock, p); printf("unlock %llu\n", U(r));
    r = CALL(fle_imp_GlobalUnlock, p); printf("unlock2 %llu\n", U(r));
    printf("count %llu\n", U(fle_glob_fixed_count()));
    r = CALL(fle_imp_GlobalFree, p); printf("free %llu\n", U(r));
    printf("count %llu\n", U(fle_glob_fixed_count()));
"#,
    );
    assert_eq!(
        l,
        [
            // page: "aligned on an 8-byte boundary"; the return value is a pointer for GMEM_FIXED
            "a 1 1",
            // page: "For these objects, the value of the returned pointer is equal to the value of the specified handle"
            "lock_same 1",
            // page: "If the specified memory block is fixed memory, this function returns TRUE"
            "unlock 1",
            "unlock2 1",
            "count 1",
            // page: success returns NULL
            "free 0",
            "count 0",
        ]
    );
}

#[test]
fn zero_init_flags_give_zeroed_memory() {
    let l = lines(
        "glob-zero",
        r#"
    uint64_t p, h, k, sum;
    static const uint32_t flags[] = { 0x40 /* GPTR */, 0x42 /* GHND */, 0x2042, 0x40 };
    int i;
    for (i = 0; i < 4; i++) {
        h = CALL(fle_imp_GlobalAlloc, flags[i], 4096);
        p = (flags[i] & 2) ? CALL(fle_imp_GlobalLock, h) : h;
        sum = 0;
        for (k = 0; k < 4096; k++) sum += ((uint8_t *)p)[k];
        printf("%#x nonzero_handle %d sum %llu\n", flags[i], h != 0, U(sum));
    }
"#,
    );
    assert_eq!(
        l,
        ["0x40 nonzero_handle 1 sum 0", "0x42 nonzero_handle 1 sum 0", "0x2042 nonzero_handle 1 sum 0", "0x40 nonzero_handle 1 sum 0"]
    );
}

#[test]
fn movable_memory_has_a_distinct_handle_and_a_lock_count() {
    let l = lines(
        "glob-movable",
        r#"
    uint64_t h = CALL(fle_imp_GlobalAlloc, 2, 32), h2 = CALL(fle_imp_GlobalAlloc, 2, 32);
    uint64_t p, p2, r, k;
    int same = 1;
    printf("handles %d %d %d\n", h != 0, h2 != 0, h != h2);
    printf("mult16 %d\n", (h % 16) == 0 && (h2 % 16) == 0);
    printf("movable_count %llu\n", U(fle_glob_movable_count()));
    p = CALL(fle_imp_GlobalLock, h);
    printf("lock1 %d handle_is_not_pointer %d\n", p != 0, p != h);
    memset((void *)p, 0x11, 32);
    p2 = CALL(fle_imp_GlobalLock, h);
    printf("lock2_same %d\n", p2 == p);
    fle_set_last_error(77);
    r = CALL(fle_imp_GlobalUnlock, h); printf("unlock_a %llu\n", U(r));
    r = CALL(fle_imp_GlobalUnlock, h); printf("unlock_b %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_GlobalUnlock, h); printf("unlock_c %llu le %u\n", U(r), LE());
    p = CALL(fle_imp_GlobalLock, h);
    for (k = 0; k < 32; k++) if (((uint8_t *)p)[k] != 0x11) same = 0;
    printf("content_kept %d\n", same);
    r = CALL(fle_imp_GlobalFree, h); printf("free_locked %llu\n", U(r));
    r = CALL(fle_imp_GlobalFree, h2); printf("free %llu\n", U(r));
    printf("movable_count %llu\n", U(fle_glob_movable_count()));
"#,
    );
    assert_eq!(
        l,
        [
            "handles 1 1 1",
            // assumption: the page gives no handle values; a multiple of 16 is this runtime's choice
            "mult16 1",
            "movable_count 2",
            "lock1 1 handle_is_not_pointer 1",
            "lock2_same 1",
            // page: still locked after the decrement -> nonzero
            "unlock_a 1",
            // page: unlocked now -> zero and GetLastError returns NO_ERROR
            "unlock_b 0 le 0",
            // page: already unlocked -> FALSE and ERROR_NOT_LOCKED (158 in System Error Codes)
            "unlock_c 0 le 158",
            "content_kept 1",
            // page: "The GlobalFree function will free a locked memory object"
            "free_locked 0",
            "free 0",
            "movable_count 0",
        ]
    );
}

#[test]
fn zero_byte_objects_and_blocks() {
    let l = lines(
        "glob-zero-size",
        r#"
    uint64_t h = CALL(fle_imp_GlobalAlloc, 2, 0), p, r, f;
    printf("movable0_handle %d\n", h != 0);
    p = CALL(fle_imp_GlobalLock, h); printf("lock_null %d\n", p == 0);
    fle_set_last_error(5);
    r = CALL(fle_imp_GlobalUnlock, h); printf("unlock %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_GlobalFree, h); printf("free %llu\n", U(r));
    f = CALL(fle_imp_GlobalAlloc, 0, 0);
    printf("fixed0_nonnull %d\n", f != 0);
    r = CALL(fle_imp_GlobalFree, f); printf("free %llu\n", U(r));
"#,
    );
    assert_eq!(
        l,
        [
            // page: GMEM_MOVEABLE with 0 bytes returns a handle to a discarded object
            "movable0_handle 1",
            // page: a zero-byte block makes GlobalLock return NULL
            "lock_null 1",
            // page: "Discarded objects always have a lock count of zero" -> unlock is FALSE with ERROR_NOT_LOCKED
            "unlock 0 le 158",
            "free 0",
            // inference: the page does not define GMEM_FIXED with 0 bytes; one byte is allocated (as LocalAlloc does)
            "fixed0_nonnull 1",
            "free 0",
        ]
    );
}

#[test]
fn obsolete_flag_0x2000_is_ignored_and_free_null_succeeds() {
    let l = lines(
        "glob-flags",
        r#"
    uint64_t a = CALL(fle_imp_GlobalAlloc, 0x2002, 8), b = CALL(fle_imp_GlobalAlloc, 0x2000, 8), r;
    printf("a_is_handle %d\n", a >= FLE_GLOBAL_HANDLE_BASE && a < FLE_GLOBAL_HANDLE_BASE + FLE_GLOBAL_REGION_SIZE);
    printf("b_is_pointer %d\n", b != 0 && !(b >= FLE_GLOBAL_HANDLE_BASE && b < FLE_GLOBAL_HANDLE_BASE + FLE_GLOBAL_REGION_SIZE));
    fle_set_last_error(9);
    r = CALL(fle_imp_GlobalFree, 0); printf("free_null %llu le %u\n", U(r), LE());
    CALL(fle_imp_GlobalFree, a); CALL(fle_imp_GlobalFree, b);
"#,
    );
    // inference: 0x2000 is GMEM_DDESHARE/GMEM_SHARE (mingw-w64 winbase.h, unofficial); the page names the flags without numbers.
    assert_eq!(l, ["a_is_handle 1", "b_is_pointer 1", "free_null 0 le 9"]);
}

#[test]
fn allocation_failure_returns_null_with_not_enough_memory() {
    let l = lines(
        "glob-fail",
        r#"
    uint64_t r;
    fle_set_last_error(0);
    r = CALL(fle_imp_GlobalAlloc, 0, UINT64_C(1) << 62); printf("fixed %llu le %u\n", U(r), LE());
    fle_set_last_error(0);
    r = CALL(fle_imp_GlobalAlloc, 2, UINT64_C(1) << 62); printf("movable %llu le %u\n", U(r), LE());
    printf("movable_count %llu\n", U(fle_glob_movable_count()));
"#,
    );
    // page: failure returns NULL; the code is not named: ERROR_NOT_ENOUGH_MEMORY (8) is an assumption
    assert_eq!(l, ["fixed 0 le 8", "movable 0 le 8", "movable_count 0"]);
}

#[test]
fn global_memory_cases_outside_the_pages_stop_the_run() {
    assert_aborts(
        "glob-abort-mem",
        "static char stray[8];",
        &[
            ("flag-fixed-1", "CALL(fle_imp_GlobalAlloc, 1, 8);"),
            ("flag-0x10000", "CALL(fle_imp_GlobalAlloc, 0x10000, 8);"),
            ("flag-0x100", "CALL(fle_imp_GlobalAlloc, 0x100, 8);"),
            ("free-stray", "CALL(fle_imp_GlobalFree, A(stray));"),
            ("free-twice-fixed", "uint64_t p = CALL(fle_imp_GlobalAlloc, 0, 8); CALL(fle_imp_GlobalFree, p); CALL(fle_imp_GlobalFree, p);"),
            ("free-twice-movable", "uint64_t h = CALL(fle_imp_GlobalAlloc, 2, 8); CALL(fle_imp_GlobalFree, h); CALL(fle_imp_GlobalFree, h);"),
            ("lock-after-free", "uint64_t h = CALL(fle_imp_GlobalAlloc, 2, 8); CALL(fle_imp_GlobalFree, h); CALL(fle_imp_GlobalLock, h);"),
            ("unlock-after-free", "uint64_t h = CALL(fle_imp_GlobalAlloc, 2, 8); CALL(fle_imp_GlobalFree, h); CALL(fle_imp_GlobalUnlock, h);"),
            ("lock-stray", "CALL(fle_imp_GlobalLock, A(stray));"),
            ("lock-null", "CALL(fle_imp_GlobalLock, 0);"),
            ("unlock-stray", "CALL(fle_imp_GlobalUnlock, A(stray));"),
            ("handle-never-issued", "CALL(fle_imp_GlobalLock, FLE_GLOBAL_HANDLE_BASE + 16 * 500);"),
            ("handle-not-aligned", "uint64_t h = CALL(fle_imp_GlobalAlloc, 2, 8); CALL(fle_imp_GlobalLock, h + 1);"),
            ("localalloc-block", "uint64_t p; R[1] = 0; R[2] = 8; fle_imp_LocalAlloc(); p = R[0]; CALL(fle_imp_GlobalFree, p);"),
        ],
    );
}

#[test]
fn a_second_thread_stops_the_run() {
    let o = run_src(
        "glob-thread",
        "static void *th(void *x) { (void)x; CALL(fle_imp_GlobalAlloc, 0, 8); printf(\"returned-from-thread\\n\"); return 0; }",
        "    pthread_t t;\n    CALL(fle_imp_GlobalAlloc, 0, 8);\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
    );
    assert!(!o.ok && !o.stdout.contains("returned"));
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: GlobalAlloc") && o.stderr.contains("second thread"), "{}", o.stderr);
}

// ---- 3. Atoms ----

const STR_FIRST: u64 = 0xC000;
const STR_LAST: u64 = 0xFFFF;
const INT_LAST: u64 = 0xBFFF;

fn in_string_range(a: u64) -> bool {
    (STR_FIRST..=STR_LAST).contains(&a)
}

#[test]
fn string_atoms_are_in_range_ignore_case_and_count_references() {
    let l = lines(
        "atom-basic",
        &format!(
            r#"
    uint64_t a = CALL(fle_imp_AddAtomW, {abc}), b = CALL(fle_imp_AddAtomW, {ABC}), c = CALL(fle_imp_AddAtomW, {other}), r;
    printf("a %llu\nb %llu\nc %llu\n", U(a), U(b), U(c));
    printf("find %llu\n", U(CALL(fle_imp_FindAtomW, {aBc})));
    printf("count %llu\n", U(fle_glob_atom_count(0)));
    /* two references to abc: the first delete keeps the atom */
    r = CALL(fle_imp_DeleteAtom, a); printf("del1 %llu\n", U(r));
    printf("find_after_del1 %llu\n", U(CALL(fle_imp_FindAtomW, {abc})));
    r = CALL(fle_imp_DeleteAtom, a); printf("del2 %llu\n", U(r));
    r = CALL(fle_imp_FindAtomW, {abc}); printf("find_after_del2 %llu le %u\n", U(r), LE());
    printf("count %llu\n", U(fle_glob_atom_count(0)));
    /* an atom that is not in the table: the return value is nAtom */
    r = CALL(fle_imp_DeleteAtom, a); printf("del3 %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_DeleteAtom, 0xC123); printf("del_unknown %llu le %u\n", U(r), LE());
"#,
            abc = cw("abc"),
            ABC = cw("ABC"),
            aBc = cw("aBc"),
            other = cw("Other Name"),
        ),
    );
    let v = |s: &str| -> u64 { s.split(' ').nth(1).unwrap().parse().unwrap() };
    let (a, b, c) = (v(&l[0]), v(&l[1]), v(&l[2]));
    // page: string atoms are 0xC000..0xFFFF; strings that differ only in case are identical; others get another atom
    assert!(in_string_range(a) && in_string_range(c), "{a:#x} {c:#x}");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(v(&l[3]), a, "FindAtom is not case sensitive and returns the atom of the string");
    assert_eq!(l[4], "count 2");
    // page (DeleteAtom): success returns zero; the string goes away only when the reference count reaches zero
    assert_eq!(l[5], "del1 0");
    assert_eq!(v(&l[6]), a);
    assert_eq!(l[7], "del2 0");
    // page (FindAtom): failure returns zero; the code is not named: 2 is an assumption
    assert_eq!(l[8], "find_after_del2 0 le 2");
    assert_eq!(l[9], "count 1");
    // page (DeleteAtom): failure returns nAtom; the code is not named: 6 is an assumption
    assert_eq!(l[10], format!("del3 {a} le 6"));
    assert_eq!(l[11], "del_unknown 49443 le 6");
    assert_eq!(a, 0xC000, "inference: the first atom handed out is the first value of the range");
}

#[test]
fn local_and_global_tables_are_independent() {
    let l = lines(
        "atom-tables",
        &format!(
            r#"
    uint64_t r;
    CALL(fle_imp_AddAtomW, {x});
    r = CALL(fle_imp_GlobalFindAtomW, {x}); printf("global_find_of_local %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_GlobalAddAtomW, {y}); printf("gadd %d\n", r >= 0xC000 && r <= 0xFFFF);
    r = CALL(fle_imp_FindAtomW, {y}); printf("local_find_of_global %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_GlobalFindAtomW, {Y}); printf("gfind_ci %d\n", r >= 0xC000 && r <= 0xFFFF);
    printf("counts %llu %llu\n", U(fle_glob_atom_count(0)), U(fle_glob_atom_count(1)));
"#,
            x = cw("only-local"),
            y = cw("only-global"),
            Y = cw("ONLY-GLOBAL"),
        ),
    );
    assert_eq!(l, ["global_find_of_local 0 le 2", "gadd 1", "local_find_of_global 0 le 2", "gfind_ci 1", "counts 1 1"]);
}

#[test]
fn global_add_atom_counts_references_but_delete_is_not_in_the_closure() {
    // GlobalDeleteAtom is not one of the 11 APIs; a second GlobalAddAtomW of the same string returns the same atom.
    let l = lines(
        "atom-gadd",
        &format!(
            r#"
    uint64_t a = CALL(fle_imp_GlobalAddAtomW, {s}), b = CALL(fle_imp_GlobalAddAtomW, {S});
    printf("%d %d\n", a == b, a >= 0xC000);
"#,
            s = cw("Shared"),
            S = cw("sHARED"),
        ),
    );
    assert_eq!(l, ["1 1"]);
}

#[test]
fn integer_atoms_and_hash_strings() {
    // (call, argument as a C expression, expected result)
    let rows: Vec<(&str, String, u64)> = vec![
        // MAKEINTATOM(i) is the pointer with only the low word set: low word 0x0001..0xBFFF is the atom itself
        ("fle_imp_AddAtomW", "0x1234".into(), 0x1234),
        ("fle_imp_AddAtomW", "0x0001".into(), 1),
        ("fle_imp_AddAtomW", "0xBFFF".into(), 0xBFFF),
        ("fle_imp_AddAtomW", "0xC000".into(), 0),
        ("fle_imp_AddAtomW", "0xFFFF".into(), 0),
        ("fle_imp_AddAtomW", "0".into(), 0),
        ("fle_imp_GlobalAddAtomW", "0x1234".into(), 0x1234),
        ("fle_imp_GlobalAddAtomW", "0xC000".into(), 0),
        ("fle_imp_FindAtomW", "0x00FF".into(), 0xFF),
        ("fle_imp_FindAtomW", "0xBFFF".into(), 0xBFFF),
        ("fle_imp_FindAtomW", "0xC000".into(), 0),
        ("fle_imp_GlobalFindAtomW", "0x0042".into(), 0x42),
        ("fle_imp_GlobalFindAtomW", "0xD000".into(), 0),
        // "#1234" is the decimal number (0x04D2 in the page); 0 and 0xC000 and above fail; leading zeros are ignored
        ("fle_imp_AddAtomW", cw("#1234"), 0x04D2),
        ("fle_imp_AddAtomW", cw("#1"), 1),
        ("fle_imp_AddAtomW", cw("#00012"), 12),
        ("fle_imp_AddAtomW", cw("#49151"), 0xBFFF),
        ("fle_imp_AddAtomW", cw("#49152"), 0),
        ("fle_imp_AddAtomW", cw("#0"), 0),
        ("fle_imp_AddAtomW", cw("#00000"), 0),
        ("fle_imp_AddAtomW", cw("#99999999999999"), 0),
        ("fle_imp_GlobalAddAtomW", cw("#1234"), 0x04D2),
        ("fle_imp_GlobalAddAtomW", cw("#65535"), 0),
        ("fle_imp_FindAtomW", cw("#1234"), 0x04D2),
        ("fle_imp_GlobalFindAtomW", cw("#7"), 7),
    ];
    let mut body = String::new();
    for (f, arg, _) in &rows {
        body += &format!("    printf(\"%llu\\n\", U(CALL({f}, {arg})));\n");
    }
    let l = lines("atom-int", &body);
    assert_eq!(l.len(), rows.len());
    for (i, (f, arg, want)) in rows.iter().enumerate() {
        assert_eq!(l[i], want.to_string(), "{f}({arg})");
    }
}

#[test]
fn integer_atom_delete_has_no_effect_and_returns_zero() {
    let l = lines(
        "atom-int-del",
        r#"
    uint64_t r;
    r = CALL(fle_imp_DeleteAtom, 0x1234); printf("%llu\n", U(r));
    r = CALL(fle_imp_DeleteAtom, 1); printf("%llu\n", U(r));
    r = CALL(fle_imp_DeleteAtom, 0xBFFF); printf("%llu\n", U(r));
"#,
    );
    // page: "The function always returns zero for an integer atom."
    assert_eq!(l, ["0", "0", "0"]);
    assert!(INT_LAST == 0xBFFF);
}

#[test]
fn only_the_low_16_bits_of_an_atom_argument_count() {
    // ATOM is a WORD: DeleteAtom takes the low 16 bits of rcx
    let l = lines(
        "atom-word",
        &format!(
            r#"
    uint64_t a = CALL(fle_imp_AddAtomW, {n}), r;
    r = CALL(fle_imp_DeleteAtom, 0xABCD0000u | a); printf("%llu\n", U(r));
    r = CALL(fle_imp_FindAtomW, {n}); printf("%llu\n", U(r));
"#,
            n = cw("word"),
        ),
    );
    assert_eq!(l, ["0", "0"]);
}

#[test]
fn name_length_limits_and_many_atoms() {
    let long126 = "a".repeat(126);
    let l = lines(
        "atom-many",
        &format!(
            r#"
    uint64_t seen[16384], n = 0, i, j, ok = 1, a;
    char buf[32];
    static uint16_t w[32];
    a = CALL(fle_imp_AddAtomW, {long126});
    printf("long126 %d\n", a >= 0xC000);
    for (i = 0; i < 1000; i++) {{
        int k;
        snprintf(buf, sizeof buf, "name%llu", U(i));
        for (k = 0; k <= (int)strlen(buf); k++) w[k] = (uint16_t)buf[k];
        a = CALL(fle_imp_AddAtomW, A(w));
        if (a < 0xC000 || a > 0xFFFF) ok = 0;
        seen[n++] = a;
    }}
    for (i = 0; i < n; i++) for (j = i + 1; j < n; j++) if (seen[i] == seen[j]) ok = 0;
    printf("distinct_in_range %d\n", ok);
    printf("count %llu\n", U(fle_glob_atom_count(0)));
"#,
            long126 = cw(&long126),
        ),
    );
    assert_eq!(l, ["long126 1", "distinct_in_range 1", "count 1001"]);
}

#[test]
fn a_full_atom_table_and_a_deleted_atom_s_slot() {
    // The range 0xC000..0xFFFF has 16384 values. After they are all in use, the next new name stops the run; after a
    // delete a new name fits again.
    let body = r#"
    char buf[32];
    static uint16_t w[32];
    uint64_t i, a, last = 0, first = 0;
    int k;
    for (i = 0; i < 16384; i++) {
        snprintf(buf, sizeof buf, "n%llu", U(i));
        for (k = 0; k <= (int)strlen(buf); k++) w[k] = (uint16_t)buf[k];
        a = CALL(fle_imp_AddAtomW, A(w));
        if (i == 0) first = a;
        last = a;
    }
    printf("first %llx last %llx count %llu\n", U(first), U(last), U(fle_glob_atom_count(0)));
    CALL(fle_imp_DeleteAtom, first);
    for (k = 0; k < 4; k++) w[k] = (uint16_t)"new"[k];
    a = CALL(fle_imp_AddAtomW, A(w));
    printf("reuse_in_range %d count %llu\n", a >= 0xC000 && a <= 0xFFFF, U(fle_glob_atom_count(0)));
"#;
    let l = lines("atom-full", body);
    assert_eq!(l, ["first c000 last ffff count 16384", "reuse_in_range 1 count 16384"]);
    assert_aborts(
        "atom-full-abort",
        "",
        &[(
            "overflow",
            r#"char buf[32]; static uint16_t w[32]; uint64_t i; int k;
    for (i = 0; i < 16385; i++) { snprintf(buf, sizeof buf, "n%llu", U(i)); for (k = 0; k <= (int)strlen(buf); k++) w[k] = (uint16_t)buf[k]; CALL(fle_imp_AddAtomW, A(w)); }"#,
        )],
    );
}

#[test]
fn atom_names_outside_the_pages_stop_the_run() {
    let long127 = "a".repeat(127);
    assert_aborts(
        "atom-abort",
        "",
        &[
            ("empty-add", &format!("CALL(fle_imp_AddAtomW, {});", cw(""))),
            ("empty-find", &format!("CALL(fle_imp_FindAtomW, {});", cw(""))),
            ("too-long-add", &format!("CALL(fle_imp_AddAtomW, {});", cw(&long127))),
            ("too-long-global", &format!("CALL(fle_imp_GlobalAddAtomW, {});", cw(&long127))),
            ("non-ascii-add", &format!("CALL(fle_imp_AddAtomW, {});", cw("caf\u{e9}"))),
            ("non-ascii-find", &format!("CALL(fle_imp_GlobalFindAtomW, {});", cw("\u{3b1}"))),
            ("find-hash-zero", &format!("CALL(fle_imp_FindAtomW, {});", cw("#0"))),
            ("find-hash-out-of-range", &format!("CALL(fle_imp_GlobalFindAtomW, {});", cw("#49152"))),
        ],
    );
}

#[test]
fn a_hash_sign_without_digits_is_a_string_atom_name() {
    let l = lines(
        "atom-hash-name",
        &format!(
            r#"
    uint64_t r;
    r = CALL(fle_imp_AddAtomW, {a}); printf("%d\n", r >= 0xC000);
    r = CALL(fle_imp_AddAtomW, {b}); printf("%d\n", r >= 0xC000);
    r = CALL(fle_imp_AddAtomW, {c}); printf("%d\n", r >= 0xC000);
"#,
            a = cw("#"),
            b = cw("#12a"),
            c = cw("# 1"),
        ),
    );
    // page: "#dddd" with decimal digits is an integer atom; "any other form" is a string atom
    assert_eq!(l, ["1", "1", "1"]);
}

// ---- 4. MulDiv ----

/// MulDiv from the page: (a * b) / d with a 64-bit intermediate, rounded to the nearest integer, a positive half
/// rounds up and a negative half rounds down; overflow or d == 0 gives -1. i128 exact arithmetic.
fn mul_div_model(a: i32, b: i32, d: i32) -> i32 {
    if d == 0 {
        return -1;
    }
    let num = a as i128 * b as i128;
    let den = d as i128;
    let neg = (num < 0) != (den < 0);
    let (n, dd) = (num.abs(), den.abs());
    let mut q = n / dd;
    let r = n % dd;
    if 2 * r >= dd {
        q += 1;
    }
    let q = if neg { -q } else { q };
    if q < i32::MIN as i128 || q > i32::MAX as i128 {
        -1
    } else {
        q as i32
    }
}

#[test]
fn mul_div_model_matches_the_page_examples() {
    // hand-checked: the model itself is pinned before it is used as the reference
    assert_eq!(mul_div_model(1, 1, 2), 1); // positive half integer rounds up
    assert_eq!(mul_div_model(-1, 1, 2), -1); // negative half integer rounds down
    assert_eq!(mul_div_model(1, -1, 2), -1);
    assert_eq!(mul_div_model(-1, -1, 2), 1);
    assert_eq!(mul_div_model(3, 1, 2), 2);
    assert_eq!(mul_div_model(-3, 1, 2), -2);
    assert_eq!(mul_div_model(4, 1, 3), 1);
    assert_eq!(mul_div_model(5, 1, 3), 2);
    assert_eq!(mul_div_model(7, 5, 0), -1);
    assert_eq!(mul_div_model(i32::MAX, i32::MAX, 1), -1);
    assert_eq!(mul_div_model(i32::MAX, 2, 2), i32::MAX);
}

#[test]
fn mul_div_follows_the_rounding_and_overflow_rules() {
    let mut cases: Vec<(i32, i32, i32)> = vec![
        (1, 1, 2),
        (-1, 1, 2),
        (1, -1, 2),
        (-1, -1, 2),
        (3, 1, 2),
        (-3, 1, 2),
        (4, 1, 3),
        (5, 1, 3),
        (-5, 1, 3),
        (100, 3, 7),
        (0, 5, 3),
        (0, 5, -3),
        (5, 0, 0),
        (7, 5, 0),
        (-7, 5, 0),
        (i32::MAX, 1, 1),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MAX, i32::MAX, 1),
        (i32::MAX, 2, 2),
        (i32::MAX, 3, 2),
        (i32::MIN, 1, 1),
        (i32::MIN, -1, 1),
        (i32::MIN, 1, -1),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX, i32::MAX),
        (65536, 65536, 65536),
        (65536, 65536, 1),
        (1 << 30, 4, 2),
        (1 << 30, 2, 1),
        (-(1 << 30), 2, 1),
        (-(1 << 30), 4, 2),
    ];
    // xorshift64: a fixed pseudo-random stream, mixed magnitudes
    let mut s = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    for k in 0..4000 {
        let shift = |v: u64, bits: u32| -> i32 { ((v as u32) >> (32 - bits)) as i32 * if v & 1 == 0 { 1 } else { -1 } };
        let bits = [4u32, 8, 16, 24, 31][k % 5];
        cases.push((shift(next(), bits), shift(next(), bits), shift(next(), bits.min(24)).max(1) * if k % 3 == 0 { -1 } else { 1 }));
    }
    let mut body = String::from("    static const int32_t c[][3] = {\n");
    for (a, b, d) in &cases {
        body += &format!("        {{ {}, {}, {} }},\n", a, b, d);
    }
    body += &format!(
        "    }};\n    int i;\n    for (i = 0; i < {}; i++) {{\n        uint64_t r = CALL(fle_imp_MulDiv, (uint32_t)c[i][0], (uint32_t)c[i][1], (uint32_t)c[i][2]);\n        printf(\"%d\\n\", (int32_t)(uint32_t)r);\n    }}\n",
        cases.len()
    );
    let l = lines("muldiv", &body);
    assert_eq!(l.len(), cases.len());
    for (i, (a, b, d)) in cases.iter().enumerate() {
        assert_eq!(l[i], mul_div_model(*a, *b, *d).to_string(), "MulDiv({a}, {b}, {d})");
    }
    // inference: a result of exactly INT_MIN fits in an int and is not an overflow
    assert_eq!(mul_div_model(i32::MIN, 1, 1), i32::MIN);
}

#[test]
fn mul_div_uses_only_the_low_32_bits_of_each_argument() {
    let l = lines(
        "muldiv-width",
        r#"
    uint64_t r = CALL(fle_imp_MulDiv, 0xFFFFFFFF00000006ull, 0x1234567800000007ull, 0xABCDEF0100000002ull);
    printf("%d\n", (int32_t)(uint32_t)r);
"#,
    );
    assert_eq!(l, [mul_div_model(6, 7, 2).to_string()]);
}

// ---- 5. lstrlenW ----

#[test]
fn lstrlen_counts_utf16_units_and_null_gives_zero() {
    let l = lines(
        "lstrlenw",
        &format!(
            r#"
    static uint16_t big[100001];
    uint64_t r;
    int i;
    r = CALL(fle_imp_lstrlenW, 0); printf("null %llu\n", U(r));
    r = CALL(fle_imp_lstrlenW, {empty}); printf("empty %llu\n", U(r));
    r = CALL(fle_imp_lstrlenW, {abc}); printf("abc %llu\n", U(r));
    r = CALL(fle_imp_lstrlenW, {pair}); printf("surrogates %llu\n", U(r));
    r = CALL(fle_imp_lstrlenW, {bmp}); printf("bmp %llu\n", U(r));
    for (i = 0; i < 100000; i++) big[i] = 'x';
    big[100000] = 0;
    r = CALL(fle_imp_lstrlenW, A(big)); printf("big %llu\n", U(r));
    big[3] = 0;
    r = CALL(fle_imp_lstrlenW, A(big)); printf("cut %llu\n", U(r));
"#,
            empty = cw(""),
            abc = cw("abc"),
            pair = cw("a\u{1F600}b"),
            bmp = cw("\u{3042}\u{3044}"),
        ),
    );
    // page: the length in characters (UTF-16 units) not including the NUL; NULL returns 0
    assert_eq!(l, ["null 0", "empty 0", "abc 3", "surrogates 4", "bmp 2", "big 100000", "cut 3"]);
}

// ---- 6. No host state ----

fn strip_comments(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut o = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}

#[test]
fn source_uses_no_host_process_time_environment_or_file_state() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_glob.c")).unwrap());
    for bad in [
        "getpid", "getppid", "clock_gettime", "gettimeofday", "getenv", "environ", "fopen", "open(", "<time.h>", "mmap", "GlobalMemoryStatus",
        "sysinfo", "/proc", "/etc", "/home", "exit(", "_exit",
    ] {
        assert!(!code.contains(bad), "fle_glob.c must not use {bad}");
    }
}
