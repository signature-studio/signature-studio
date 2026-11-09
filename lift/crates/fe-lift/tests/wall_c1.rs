//! Tests of decision 0113 (PROVISIONAL, docs/decisions/0113-chain-walls-batch1.md): the walls of the 6-module chain (measurement bases and seeds, fonts placed)
//! that were fixed in the first batch of the measure-and-fix loop. The runtime walls tested here: HeapReAlloc (wall 1). The generator walls have their own
//! files: LDDQU (tests/sse_pack.rs), the lazy-initialisation slot of the selector stubs (fe-decode/tests/jt_cpu.rs, tests/jt_cpu.rs of fe-lift),
//! GetObjectW(DIBSECTION) (tests/gdi.rs), StretchDIBits (tests/gdi_bulk.rs).
//!
//! Expected values are written from the Microsoft Learn page of HeapReAlloc (https://learn.microsoft.com/en-us/windows/win32/api/heapapi/nf-heapapi-heaprealloc, read
//! 2026-10-28) and from decision 0113, not read back from the C code. The host is Linux: nothing is compared with a real Windows. The driver is linked with every
//! runtime/fle_*.c, like tests/wall_b.rs.
//!
//! Page sentences used: "HEAP_ZERO_MEMORY: If the reallocation request is for a larger size, the additional region of memory beyond the original size be initialized to zero.
//! The contents of the memory block up to its original size are unaffected."; "HEAP_REALLOC_IN_PLACE_ONLY: There can be no movement when reallocating a memory block. ... If this
//! value is specified and the block cannot be resized without moving, the function fails, leaving the original memory block unchanged."; "If the function fails and you have not
//! specified HEAP_GENERATE_EXCEPTIONS, the return value is NULL."; "If the function fails, it does not call SetLastError."; "If HeapReAlloc fails, the original memory is not
//! freed, and the original handle and pointer are still valid."; "HeapReAlloc is guaranteed to preserve the content of the memory being reallocated, even if the new memory is
//! allocated at a different location."; "The alignment of memory returned by HeapReAlloc is MEMORY_ALLOCATION_ALIGNMENT" (16 on Win64).

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    manifest().join("../../runtime")
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-wallc1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work().join(etxt_common::per_process("rt"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut srcs: Vec<PathBuf> = std::fs::read_dir(rt())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("fle_") && n.ends_with(".c")
            })
            .collect();
        srcs.sort();
        let mut v = Vec::new();
        for s in srcs {
            let o = dir.join(s.file_name().unwrap().to_string_lossy().replace(".c", ".o"));
            let cc = Command::new("clang")
                .args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-c", "-I"])
                .arg(rt())
                .arg(&s)
                .arg("-o")
                .arg(&o)
                .env("TMPDIR", work().join("tmp"))
                .output()
                .expect("clang not found");
            assert!(cc.status.success(), "clang failed for {}: {}", s.display(), String::from_utf8_lossy(&cc.stderr));
            v.push(o);
        }
        v
    })
}

const PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_proc.h"
#include "fle_gdi.h"
#include "fle_loadimage.h"
#include "fle_cursor.h"
#include <pthread.h>

uint64_t R[16];
fle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };
fle_xmm X[16];
const fle_fn_entry fle_dispatch_table[] = { { 0, 0 } };
const uint64_t fle_dispatch_count = 0;
static uint64_t stk[64];
#define U(x) ((unsigned long long)(x))
#define ENTER() do { R[4] = (uint64_t)(uintptr_t)&stk[16]; stk[16] = 0xdead0000ull; } while (0)
#define P(x) ((uint64_t)(uintptr_t)(x))
static uint64_t call1(void (*f)(void), uint64_t a) { R[1] = a; f(); return R[0]; }
static uint64_t call3(void (*f)(void), uint64_t a, uint64_t b, uint64_t c) { R[1] = a; R[2] = b; R[8] = c; f(); return R[0]; }
static uint64_t call4(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d) { R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); return R[0]; }
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
#define PH() call1(fle_imp_GetProcessHeap, 0)
#define HALLOC(h, fl, n) call3(fle_imp_HeapAlloc, (h), (fl), (n))
#define HREALLOC(h, fl, p, n) call4(fle_imp_HeapReAlloc, (h), (fl), (p), (n))
#define HSIZE(h, p) call3(fle_imp_HeapSize, (h), 0, (p))
#define HFREE(h, p) call3(fle_imp_HeapFree, (h), 0, (p))
static uint64_t g_ph, g_p;
static void *thr_hra(void *a) { (void)a; HREALLOC(g_ph, 0, g_p, 16); return 0; }
"#;

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn run(name: &str, body: &str) -> Out {
    let dir = work();
    let tag = etxt_common::uniq(name);
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    std::fs::write(&c, format!("{PRE}\nint main(void)\n{{\n    ENTER();\n{body}\n    return 0;\n}}\n")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-no-pie", "-pthread", "-I"]).arg(rt());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg(&c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn lines(o: &Out) -> Vec<String> {
    assert!(o.ok, "signal {:?}: {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// The body must stop the run: SIGABRT, `fle: <api>` on stderr, nothing returned. Returns stderr.
fn stops(name: &str, api: &str, body: &str) -> String {
    let o = run(name, &format!("{body}\n    printf(\"returned %llu\\n\", U(R[0]));"));
    assert!(!o.ok && !o.stdout.contains("returned"), "{name}: returned: {}", o.stdout);
    assert!(o.stderr.contains(&format!("fle: {api}")), "{name}: no 'fle: {api}': {}", o.stderr);
    assert_eq!(o.signal, Some(6), "{name}: not SIGABRT");
    o.stderr
}

// ---- wall 1: kernel32!HeapReAlloc ------------------------------------------------------------------------------------------------

/// Growing and shrinking keep the contents up to the smaller size, the 16-byte alignment holds, HeapSize follows the last request, and HeapFree takes the new block.
#[test]
fn heap_realloc_keeps_the_contents_the_alignment_and_the_requested_size() {
    let o = run(
        "c1-hra-basic",
        r#"
    uint64_t ph = PH(), p, q, r;
    uint8_t *b;
    int i, ok = 1;
    p = HALLOC(ph, 0, 16);
    b = (uint8_t *)p;
    for (i = 0; i < 16; i++) b[i] = (uint8_t)(0xA0 + i);
    q = HREALLOC(ph, 0, p, 4096);
    b = (uint8_t *)q;
    for (i = 0; i < 16; i++) ok &= b[i] == (uint8_t)(0xA0 + i);
    printf("grow nonnull %d aligned %d contents %d size %llu\n", q != 0, (q & 15) == 0, ok, U(HSIZE(ph, q)));
    for (i = 0; i < 4096; i++) b[i] = (uint8_t)i;
    r = HREALLOC(ph, 0, q, 8);
    b = (uint8_t *)r;
    for (ok = 1, i = 0; i < 8; i++) ok &= b[i] == (uint8_t)i;
    printf("shrink nonnull %d aligned %d contents %d size %llu\n", r != 0, (r & 15) == 0, ok, U(HSIZE(ph, r)));
    printf("free %llu\n", U(HFREE(ph, r)));
    /* a private heap: the block keeps its heap, and the size is per request */
    {
        uint64_t h = call3(fle_imp_HeapCreate, 0, 0, 0), a = HALLOC(h, 0, 10), c = HREALLOC(h, 0, a, 100);
        printf("private nonnull %d size %llu\n", c != 0, U(HSIZE(h, c)));
        printf("private free %llu\n", U(HFREE(h, c)));
    }
"#,
    );
    assert_eq!(
        lines(&o),
        ["grow nonnull 1 aligned 1 contents 1 size 4096", "shrink nonnull 1 aligned 1 contents 1 size 8", "free 1", "private nonnull 1 size 100", "private free 1"]
    );
}

/// HEAP_ZERO_MEMORY (8): the bytes from the old requested size to the new size are zero, the bytes below the old size are unaffected. Without the flag the new bytes are not
/// promised (not tested). The heap is first dirtied with 0xCC so that a block that grows into freed memory would show non-zero bytes if the zeroing were missing.
#[test]
fn heap_realloc_zero_memory_zeroes_the_grown_part_only() {
    let o = run(
        "c1-hra-zero",
        r#"
    uint64_t ph = PH(), junk, p, q;
    uint8_t *b;
    int i, low = 1, high = 1, k;
    for (k = 0; k < 8; k++) {
        junk = HALLOC(ph, 0, 4096);
        memset((void *)junk, 0xCC, 4096);
        HFREE(ph, junk);
        p = HALLOC(ph, 0, 24);
        b = (uint8_t *)p;
        memset(b, 0xAB, 24);
        q = HREALLOC(ph, 8, p, 600);
        b = (uint8_t *)q;
        for (i = 0; i < 24; i++) low &= b[i] == 0xAB;
        for (i = 24; i < 600; i++) high &= b[i] == 0;
        HFREE(ph, q);
    }
    printf("below the old size unaffected %d, grown part zero %d\n", low, high);
    /* shrinking with the flag changes nothing */
    p = HALLOC(ph, 0, 32);
    memset((void *)p, 0x5A, 32);
    q = HREALLOC(ph, 8, p, 16);
    b = (uint8_t *)q;
    for (low = 1, i = 0; i < 16; i++) low &= b[i] == 0x5A;
    printf("shrink with the flag %d size %llu\n", low, U(HSIZE(ph, q)));
"#,
    );
    assert_eq!(lines(&o), ["below the old size unaffected 1, grown part zero 1", "shrink with the flag 1 size 16"]);
}

/// dwBytes 0 is reallocated as one byte (as HeapAlloc does) and the requested size is 0; the block can grow again.
#[test]
fn heap_realloc_to_zero_bytes_keeps_a_valid_block_of_size_zero() {
    let o = run(
        "c1-hra-zero-size",
        r#"
    uint64_t ph = PH(), p = HALLOC(ph, 0, 16), q = HREALLOC(ph, 0, p, 0), r;
    printf("nonnull %d size %llu\n", q != 0, U(HSIZE(ph, q)));
    r = HREALLOC(ph, 0, q, 64);
    printf("again nonnull %d size %llu\n", r != 0, U(HSIZE(ph, r)));
"#,
    );
    assert_eq!(lines(&o), ["nonnull 1 size 0", "again nonnull 1 size 64"]);
}

/// A freed block is no longer known, and the table follows a block that moved: after many reallocations every live block has exactly its last requested size.
#[test]
fn heap_realloc_moves_the_table_entry_with_the_block() {
    let o = run(
        "c1-hra-table",
        r#"
    uint64_t ph = PH(), keep[8], n, i, ok = 1;
    for (i = 0; i < 8; i++) keep[i] = HALLOC(ph, 0, 8 + i);
    for (n = 0; n < 64; n++)
        for (i = 0; i < 8; i++) keep[i] = HREALLOC(ph, 0, keep[i], 100 * (n + 1) + i);
    for (i = 0; i < 8; i++) ok &= HSIZE(ph, keep[i]) == 100 * 64 + i;
    printf("sizes %d\n", (int)ok);
    for (i = 0; i < 8; i++) HFREE(ph, keep[i]);
"#,
    );
    assert_eq!(lines(&o), ["sizes 1"]);
}

/// A block that HeapReAlloc moved is gone under the old address (when the address changed): HeapSize on it stops. The test makes sure the address changes by growing a block
/// that is boxed in by two live neighbours.
#[test]
fn heap_realloc_forgets_the_old_address_of_a_block_that_moved() {
    let o = run(
        "c1-hra-moved",
        r#"
    uint64_t ph = PH(), a = HALLOC(ph, 0, 24), p = HALLOC(ph, 0, 24), c = HALLOC(ph, 0, 24), q;
    (void)a; (void)c;
    q = HREALLOC(ph, 0, p, 1 << 20);
    printf("moved %d\n", q != p);
    fflush(stdout);
    HSIZE(ph, p);
"#,
    );
    assert!(!o.ok && o.stdout.contains("moved 1"), "{}{}", o.stdout, o.stderr);
    assert!(o.stderr.contains("fle: HeapSize on") && o.stderr.contains("not a block that HeapAlloc returned"), "{}", o.stderr);
}

/// HEAP_REALLOC_IN_PLACE_ONLY (0x10): the same size or a smaller one keeps the block where it is (a choice: shrinking never needs to move a block), a larger one stops by
/// name (the page does not say when a block can grow in place).
#[test]
fn heap_realloc_in_place_only_keeps_the_address_for_the_same_or_a_smaller_size_and_stops_for_growth() {
    let o = run(
        "c1-hra-inplace",
        r#"
    uint64_t ph = PH(), p = HALLOC(ph, 0, 100), q;
    memset((void *)p, 0x77, 100);
    q = HREALLOC(ph, 0x10, p, 100);
    printf("same: same address %d size %llu\n", q == p, U(HSIZE(ph, q)));
    q = HREALLOC(ph, 0x10, p, 40);
    printf("smaller: same address %d size %llu byte %02x\n", q == p, U(HSIZE(ph, q)), ((uint8_t *)q)[39]);
    q = HREALLOC(ph, 0x10 | 8, p, 0);
    printf("zero bytes: same address %d size %llu\n", q == p, U(HSIZE(ph, q)));
"#,
    );
    assert_eq!(lines(&o), ["same: same address 1 size 100", "smaller: same address 1 size 40 byte 77", "zero bytes: same address 1 size 0"]);
    let e = stops("c1-hra-inplace-grow", "HeapReAlloc", "uint64_t ph = PH(), p = HALLOC(ph, 0, 100); HREALLOC(ph, 0x10, p, 101);");
    assert!(e.contains("HEAP_REALLOC_IN_PLACE_ONLY") && e.contains("from 100 to 101 bytes"), "{e}");
}

/// Failure: a request the host cannot satisfy returns NULL, leaves the original block valid with its size and contents, and does not touch the last error (the page: "it does
/// not call SetLastError"). With HEAP_GENERATE_EXCEPTIONS (in the call, or on the heap by HeapCreate) the run stops (there is no exception model).
#[test]
fn heap_realloc_failure_returns_null_and_leaves_the_block_and_the_last_error() {
    let o = run(
        "c1-hra-fail",
        r#"
    uint64_t ph = PH(), p = HALLOC(ph, 0, 40), q;
    memset((void *)p, 0x31, 40);
    call1(fle_imp_SetLastError, 0x1234);
    q = HREALLOC(ph, 0, p, UINT64_C(0x7FFFFFFFFFFFFFF0));
    printf("null %d size %llu byte %02x le %x\n", q == 0, U(HSIZE(ph, p)), ((uint8_t *)p)[39], LE());
    {
        uint64_t h = call3(fle_imp_HeapCreate, 0, 0, 0), a = HALLOC(h, 0, 8);
        q = HREALLOC(h, 0, a, UINT64_C(0x7FFFFFFFFFFFFFF0));
        printf("private null %d size %llu\n", q == 0, U(HSIZE(h, a)));
    }
"#,
    );
    assert_eq!(lines(&o), ["null 1 size 40 byte 31 le 1234", "private null 1 size 8"]);
    let e = stops("c1-hra-fail-exc-flag", "HeapReAlloc", "uint64_t ph = PH(), p = HALLOC(ph, 0, 40); HREALLOC(ph, 4, p, UINT64_C(0x7FFFFFFFFFFFFFF0));");
    assert!(e.contains("HEAP_GENERATE_EXCEPTIONS"), "{e}");
    let e = stops(
        "c1-hra-fail-exc-heap",
        "HeapReAlloc",
        "uint64_t h = call3(fle_imp_HeapCreate, 4, 0, 0), a = HALLOC(h, 0, 8); HREALLOC(h, 0, a, UINT64_C(0x7FFFFFFFFFFFFFF0));",
    );
    assert!(e.contains("HEAP_GENERATE_EXCEPTIONS"), "{e}");
}

/// What the page does not define stops by name: a NULL block, a pointer HeapAlloc never returned, a freed block, a block of another heap, an unknown heap handle, and a flag
/// bit outside the four the page lists (0x2, 0x20, 0x100, and the high bits).
#[test]
fn heap_realloc_stops_on_what_the_page_does_not_define() {
    let cases: &[(&str, &str, &str)] = &[
        ("null", "HREALLOC(PH(), 0, 0, 8);", "NULL block"),
        ("not-a-heap-block", "uint8_t x[8]; HREALLOC(PH(), 0, P(x), 8);", "not a block that HeapAlloc or HeapReAlloc returned"),
        ("freed", "uint64_t ph = PH(), p = HALLOC(ph, 0, 8); HFREE(ph, p); HREALLOC(ph, 0, p, 16);", "not a block that HeapAlloc or HeapReAlloc returned"),
        ("other-heap", "uint64_t ph = PH(), h = call3(fle_imp_HeapCreate, 0, 0, 0), p = HALLOC(ph, 0, 8); HREALLOC(h, 0, p, 16);", "belongs to heap"),
        ("bad-handle", "HREALLOC(0x1000, 0, 0x2000, 8);", "heap handle"),
        ("flag-0x2", "uint64_t ph = PH(), p = HALLOC(ph, 0, 8); HREALLOC(ph, 2, p, 16);", "dwFlags 0x2 is not defined"),
        ("flag-0x20", "uint64_t ph = PH(), p = HALLOC(ph, 0, 8); HREALLOC(ph, 0x20, p, 16);", "dwFlags 0x20 is not defined"),
        ("flag-0x100", "uint64_t ph = PH(), p = HALLOC(ph, 0, 8); HREALLOC(ph, 0x100, p, 16);", "dwFlags 0x100 is not defined"),
        ("flag-high", "uint64_t ph = PH(), p = HALLOC(ph, 0, 8); HREALLOC(ph, 0x80000000u, p, 16);", "dwFlags 0x80000000 is not defined"),
    ];
    for (label, body, word) in cases {
        let e = stops(&format!("c1-hra-stop-{label}"), "HeapReAlloc", body);
        assert!(e.contains(word), "{label}: {e}");
    }
}

/// HEAP_NO_SERIALIZE (1) is accepted (one thread), alone and with HEAP_ZERO_MEMORY (8); the growth keeps working through all four combinations.
#[test]
fn heap_realloc_accepts_no_serialize_and_zero_memory_together() {
    let o = run(
        "c1-hra-flags",
        r#"
    uint64_t ph = PH(), p = HALLOC(ph, 0, 8), q;
    static const uint32_t fls[4] = { 0, 1, 8, 9 };
    int k, ok = 1;
    for (k = 0; k < 4; k++) {
        q = HREALLOC(ph, fls[k], p, 16 + 8 * (k + 1));
        ok &= q != 0 && HSIZE(ph, q) == 16 + 8 * (k + 1);
        p = q;
    }
    printf("flags %d\n", ok);
"#,
    );
    assert_eq!(lines(&o), ["flags 1"]);
}

/// The second thread rule of the process heap functions (decision 0017) applies to HeapReAlloc too.
#[test]
fn heap_realloc_from_a_second_thread_stops() {
    let o = run(
        "c1-hra-thread",
        r#"
    pthread_t t;
    g_ph = PH();
    g_p = HALLOC(g_ph, 0, 8);
    pthread_create(&t, 0, thr_hra, 0);
    pthread_join(t, 0);
    printf("returned\n");
"#,
    );
    assert!(!o.ok && o.stderr.contains("fle: HeapReAlloc called from tid") && o.stderr.contains("another thread, not supported"), "{}{}", o.stdout, o.stderr);
    assert!(!o.stdout.contains("returned"));
}

/// The response table still has one HeapReAlloc row, with the same runtime function, at the position it had (decision 0015: the position fixes the pseudo-VA).
#[test]
fn the_heap_realloc_row_is_where_it_was_and_the_header_declares_a_returning_function() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let row = "(\"kernel32.dll\", \"HeapReAlloc\", \"fle_imp_HeapReAlloc\", \"jmp_imp:HeapReAlloc\")";
    assert_eq!(table.matches(row).count(), 1);
    // the row index (0-based) of HeapReAlloc: counted from the table at HEAD 5a39f6b (E1 and the walls of this batch append at the end, so it does not move)
    let rows_before = table[..table.find(row).unwrap()].matches("jmp_imp:").count();
    assert_eq!(rows_before, 257, "the number of response rows in front of HeapReAlloc (a pin of the position)");
    let h = std::fs::read_to_string(rt().join("fle_sys.h")).unwrap();
    assert!(h.contains("void fle_imp_HeapReAlloc(void);"), "declared as a function that returns");
    assert!(!h.contains("void fle_imp_HeapReAlloc(void) __attribute__((noreturn))"), "no longer noreturn");
}

// ---- wall 8: user32!DestroyCursor of a shared cursor (the GDI part is in tests/gdi_bulk.rs) -----------------------------------------

/// The shared cursors: the standard cursors (LoadCursorW, LoadImageW with a NULL hInst: base 0x7FCA0000 + 4 * (idc - 32512 + 1)) and the form of a LoadImageW cursor of a module
/// (decisions 0061 and 0104: base 0x7FDA0000 + stride 0x1000 * k (k = 0 the lifted DLL, 1..8 a real module) + 4 * (id - 32762 + 1), id 32762..32767).
#[test]
fn the_shared_cursor_handle_forms_are_recognised() {
    let o = run(
        "c1-lih",
        r#"
    uint64_t b = 0x7FDA0000ull;
    int id, k, ok = 1;
    /* the numbers written in fle_cursor.c are the constants of fle_loadimage.h, and the handles the loader answers have the form */
    printf("constants %d %d %d %d\n", FLE_LI_MODULE_HANDLE_BASE == b, FLE_LI_REALMOD_STRIDE == 0x1000, FLE_LI_MODULE_FIRST == 32762, FLE_LI_MODULE_LAST == 32767);
    for (id = 32762; id <= 32767; id++) ok &= fle_cursor_is_shared_handle(fle_loadimage_module_handle_of(id)) == 1;
    printf("handles of the loader %d\n", ok);
    ok = 1;
    for (k = 0; k <= 8; k++)
        for (id = 32762; id <= 32767; id++) ok &= fle_cursor_is_shared_handle(b + 0x1000ull * k + 4ull * (id - 32762 + 1)) == 1;
    printf("all module forms %d\n", ok);
    printf("dll ids 32762 and 32767: %d %d\n", fle_cursor_is_shared_handle(b + 4) == 1, fle_cursor_is_shared_handle(b + 24) == 1);
    printf("ILTools 32763: %d (0x7FDA5008)\n", fle_cursor_is_shared_handle(0x7FDA5008ull));
    printf("standard: arrow %d sizeall %d (0x7FCA021C)\n", fle_cursor_is_shared_handle(0x7FCA0004ull), fle_cursor_is_shared_handle(0x7FCA021Cull));
    printf("outside: base %d, base+2 %d, base+28 %d, k=9 %d, below %d, 0x7FCA0001 %d, 0x7FCA0018 %d, zero %d\n", fle_cursor_is_shared_handle(b), fle_cursor_is_shared_handle(b + 2),
           fle_cursor_is_shared_handle(b + 28), fle_cursor_is_shared_handle(b + 0x9000ull + 4), fle_cursor_is_shared_handle(b - 4), fle_cursor_is_shared_handle(0x7FCA0001ull),
           fle_cursor_is_shared_handle(0x7FCA0018ull), fle_cursor_is_shared_handle(0));
    printf("destroy cursor of ILTools 32763 %llu, DestroyIcon of it stops next\n", U(call1(fle_imp_DestroyCursor, 0x7FDA5008ull)));
    fflush(stdout);
    call1(fle_imp_DestroyIcon, 0x7FDA5008ull);
"#,
    );
    assert!(!o.ok, "DestroyIcon of a shared cursor must still stop");
    assert!(o.stderr.contains("fle: DestroyIcon: hIcon was not issued by this runtime's GDI table"), "{}", o.stderr);
    assert_eq!(
        o.stdout.lines().collect::<Vec<_>>(),
        [
            "constants 1 1 1 1",
            "handles of the loader 1",
            "all module forms 1",
            "dll ids 32762 and 32767: 1 1",
            "ILTools 32763: 1 (0x7FDA5008)",
            "standard: arrow 1 sizeall 1 (0x7FCA021C)",
            "outside: base 0, base+2 0, base+28 0, k=9 0, below 0, 0x7FCA0001 0, 0x7FCA0018 0, zero 0",
            "destroy cursor of ILTools 32763 1, DestroyIcon of it stops next",
        ]
    );
}
