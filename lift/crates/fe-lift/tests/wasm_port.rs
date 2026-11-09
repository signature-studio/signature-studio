//! Native tests of the code paths that the wasm port adds (docs/decisions/0045-wasm-port.md). The wasm build itself
//! (emcc, node) is checked by lift/harness/wasm/build.sh, not here. Three groups:
//!
//! 1. fle_host_tid() on native is the Linux tid (the single-thread rule only); GetCurrentThreadId does not return it but the
//!    fixed virtual value of decision 0056 (tests/tid.rs has the rest of that decision's tests).
//! 2. The arena variant of VirtualAlloc, VirtualFree, VirtualProtect, VirtualQuery and the file mapping objects
//!    (FLE_VMEM_ARENA, the code wasm uses), compiled and run natively.
//! 3. The 32-bit address check of fle_ld / fle_st (FLE_CHECK_HI32): a native non-PIE program, whose data is below
//!    4 GiB, stops with the named message on an address above 4 GiB, and does not stop on a good one.
//!
//! The default (native) build of these functions is covered by the existing tests (sys.rs, vmap.rs, ...), unchanged.

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;

fn rt() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime")
}

fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-wasm2/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn build_run(name: &str, src: &str, defs: &[&str], pie: bool, files: &[&str], args: &[&str]) -> Out {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", if pie { "-fPIE" } else { "-fno-pie" }, if pie { "-pie" } else { "-no-pie" }]);
    for d in defs {
        cmd.arg(d);
    }
    cmd.arg("-I").arg(rt());
    cmd.arg(&c);
    for f in files {
        cmd.arg(rt().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).args(args).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

const PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <sys/syscall.h>
#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_sys.h"
#include "fle_vmap.h"
#include "fle_vfs.h"
#ifdef __EMSCRIPTEN__
_Thread_local unsigned char fle_ctx[0x300] __attribute__((aligned(64), used));   /* the register block of the wasm build (fle_ctx_tls.c): R, FL and X live in it */
#else
uint64_t R[16];
#endif
static uint64_t stk[32];
static uint64_t callv(void (*f)(void), const uint64_t *a)
{
    memset(stk, 0, sizeof stk);
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3]; stk[5] = a[4]; stk[6] = a[5];
    R[4] = (uint64_t)(uintptr_t)stk; R[0] = 0x99;
    f();
    return R[0];
}
#define CALL(f, ...) callv((void (*)(void))(f), (uint64_t[12]){ __VA_ARGS__ })
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
"#;

const RT_FILES: [&str; 8] = ["fle_sys.c", "fle_vmap.c", "fle_vfs.c", "fle_env.c", "fle_imports.c", "fle_host.c", "fle_proc.c", "fle_time.c"];

#[test]
fn native_host_tid_is_the_linux_tid_and_get_current_thread_id_returns_the_virtual_value() {
    // Decision 0056: the host tid names the owner thread of the single-thread rule; the guest gets 0x1001 (4097).
    let src = format!(
        r#"{PRE}int main(void)
{{
    long t = (long)syscall(SYS_gettid);
    printf("host_tid_is_gettid %d\n", fle_host_tid() == t);
    printf("get_current_thread_id %llu\n", U(CALL(fle_imp_GetCurrentThreadId, 0, 0, 0, 0)));
    printf("guest_value_is_not_the_host_tid %d\n", CALL(fle_imp_GetCurrentThreadId, 0, 0, 0, 0) != (uint32_t)t);
    printf("not_zero %d\n", fle_host_tid() != 0);
    return 0;
}}
"#
    );
    let o = build_run("wp-tid", &src, &[], true, &RT_FILES, &[]);
    assert!(o.ok, "{}", o.stderr);
    assert_eq!(
        o.stdout.lines().collect::<Vec<_>>(),
        ["host_tid_is_gettid 1", "get_current_thread_id 4097", "guest_value_is_not_the_host_tid 1", "not_zero 1"]
    );
}

#[test]
fn wasm_thread_id_is_a_fixed_nonzero_value_used_by_every_owner_check() {
    // fle_host.h with __EMSCRIPTEN__ defined by hand (clang native): the constant, and the files that include it compile.
    let src = format!(
        r#"{PRE}int main(void)
{{
    printf("tid %ld\n", fle_host_tid());
    printf("tid_again %ld\n", fle_host_tid());
    printf("get_current_thread_id %llu\n", U(CALL(fle_imp_GetCurrentThreadId, 0, 0, 0, 0)));
    return 0;
}}
"#
    );
    // the sources that include fle_host.h but do not need emscripten headers
    let o = build_run("wp-tid-wasm", &src, &["-D__EMSCRIPTEN__", "-DFLE_VMEM_ARENA"], true, &["fle_imports.c", "fle_sys.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_host.c", "fle_proc.c", "fle_time.c"], &[]);
    assert!(o.ok, "{}", o.stderr);
    // the host tid stays the constant 1 (owner slots); the guest value is the same 4097 as on native (decision 0056)
    assert_eq!(o.stdout.lines().collect::<Vec<_>>(), ["tid 1", "tid_again 1", "get_current_thread_id 4097"]);
}

#[test]
fn arena_virtual_memory_follows_the_same_rules_without_host_protection() {
    let src = format!(
        r#"{PRE}#define VA(a, s, t, p) CALL(fle_imp_VirtualAlloc, a, s, t, p)
#define VF(a, s, t) CALL(fle_imp_VirtualFree, a, s, t, 0)
static uint64_t le(void) {{ fle_imp_GetLastError(); return R[0]; }}
static int zero(const uint8_t *p, size_t n) {{ size_t i; for (i = 0; i < n; i++) if (p[i]) return 0; return 1; }}
int main(void)
{{
    uint64_t a, r, x, h, v1, v2, old = 0, n0 = fle_sys_region_count();
    uint8_t *p;
    uint8_t mbi[48];
    uint32_t st, pr;
    uint64_t rs;

    a = VA(0, 0x13fff0, 0x1000, 4);
    printf("a_nonnull %d granule %llu\n", a != 0, U(a % 65536));
    memset((void *)a, 0xAB, 0x140000);
    printf("release %llu\n", U(VF(a, 0, 0x8000)));
    a = VA(0, 0x13fff0, 0x1000, 4);
    printf("reused_range_is_zero %d\n", zero((uint8_t *)a, 0x140000));
    VF(a, 0, 0x8000);

    r = VA(0, 0x40000, 0x2000, 4);
    x = VA(r + 0x1800, 1, 0x1000, 4);
    printf("commit_rounds_down %d\n", x == r + 0x1000);
    p = (uint8_t *)r; p[0x1000] = 0xAA; p[0x1fff] = 0xBB;
    printf("commit_again_keeps %d\n", VA(r + 0x1000, 0x1000, 0x1000, 4) == r + 0x1000 && p[0x1000] == 0xAA && p[0x1fff] == 0xBB);
    printf("decommit %llu\n", U(VF(r + 0x1000, 0x1000, 0x4000)));
    printf("recommit_zero %d\n", VA(r + 0x1000, 0x1000, 0x1000, 4) == r + 0x1000 && zero(p + 0x1000, 0x1000));
    fle_set_last_error(0);
    printf("commit_past_end %llu le %llu\n", U(VA(r + 0x40000, 0x1000, 0x1000, 4)), U(le()));
    fle_set_last_error(0);
    printf("reserve_reserved %llu le %llu\n", U(VA(r, 0x1000, 0x2000, 4)), U(le()));
    fle_set_last_error(0);
    printf("outside_arena_fixed %llu le %llu\n", U(VA(0x500000012345ull, 0x1000, 0x2000, 4)), U(le()));
    x = VA(r + 0x80000, 0x10000, 0x2000, 4);
    printf("fixed_inside_arena %d\n", x == r + 0x80000);
    VF(x, 0, 0x8000);

    /* VirtualQuery and VirtualProtect work from the tables */
    VA(r + 0x2000, 0x3000, 0x1000, 2);
    printf("query %llu\n", U(CALL(fle_imp_VirtualQuery, r + 0x2000, A(mbi), 48, 0)));
    memcpy(&rs, mbi + 24, 8); memcpy(&st, mbi + 32, 4); memcpy(&pr, mbi + 36, 4);
    printf("query_state %x prot %x size %llx\n", st, pr, U(rs));
    printf("protect %llu\n", U(CALL(fle_imp_VirtualProtect, r + 0x2000, 0x1000, 4, A(&old))));
    printf("protect_old %llu\n", U(old));
    printf("release_r %llu regions_back %llu\n", U(VF(r, 0, 0x8000)), U(fle_sys_region_count() - n0));

    /* file mapping objects in the arena */
    h = CALL(fle_imp_CreateFileMappingW, (uint64_t)-1, 0, 4, 0, 0x3000, 0);
    v1 = CALL(fle_imp_MapViewOfFile, h, 2, 0, 0, 0);
    p = (uint8_t *)v1;
    printf("map zero %d\n", p[0] == 0 && p[0x2fff] == 0);
    p[0] = 0x11; p[0x2fff] = 0x22;
    v2 = CALL(fle_imp_MapViewOfFile, h, 1, 0, 0, 0);
    printf("copy_view_content %d\n", ((uint8_t *)v2)[0] == 0x11 && ((uint8_t *)v2)[0x2fff] == 0x22);
    ((uint8_t *)v2)[1] = 7;
    printf("copy_view_private %d\n", p[1] == 0);
    printf("unmap %llu %llu close %llu\n", U(CALL(fle_imp_UnmapViewOfFile, v1)), U(CALL(fle_imp_UnmapViewOfFile, v2)), U(CALL(fle_imp_CloseHandle, h)));
    printf("counts objs %llu views %llu handles %llu\n", U(fle_vmap_object_count()), U(fle_vmap_view_count()), U(fle_vmap_handle_count()));
    return 0;
}}
"#
    );
    let o = build_run("wp-arena", &src, &["-DFLE_VMEM_ARENA"], false, &["fle_sys.c", "fle_vmap.c", "fle_vfs.c", "fle_env.c", "fle_imports.c", "fle_host.c"], &[]);
    assert!(o.ok, "signal {:?}: {}\n{}", o.signal, o.stderr, o.stdout);
    let want = [
        "a_nonnull 1 granule 0",
        "release 1",
        "reused_range_is_zero 1",
        "commit_rounds_down 1",
        "commit_again_keeps 1",
        "decommit 1",
        "recommit_zero 1",
        "commit_past_end 0 le 487",
        "reserve_reserved 0 le 487",
        "outside_arena_fixed 0 le 487",
        "fixed_inside_arena 1",
        "query 48",
        "query_state 1000 prot 2 size 3000",
        "protect 1",
        "protect_old 2",
        "release_r 1 regions_back 0",
        "map zero 1",
        "copy_view_content 1",
        "copy_view_private 1",
        "unmap 1 1 close 1",
        "counts objs 0 views 0 handles 0",
    ];
    assert_eq!(o.stdout.lines().collect::<Vec<_>>(), want, "{}", o.stderr);
}

#[test]
fn arena_top_down_takes_the_highest_free_place() {
    // Decision 0052: MEM_TOP_DOWN (0x100000) with lpAddress NULL claims the highest free range of the arena (end of the
    // arena, or just below the lowest claimed span above; start rounded down to the allocation granularity); plain
    // calls still take the first free place from the bottom.
    let src = format!(
        r#"{PRE}#define VA(a, s, t, p) CALL(fle_imp_VirtualAlloc, a, s, t, p)
#define VF(a, s, t) CALL(fle_imp_VirtualFree, a, s, t, 0)
static int zero(const uint8_t *p, size_t n) {{ size_t i; for (i = 0; i < n; i++) if (p[i]) return 0; return 1; }}
int main(void)
{{
    uint64_t lo, t1, t2, t3, mid, big, n0 = fle_sys_region_count();
    uint8_t *p;

    lo = VA(0, 0x10000, 0x1000, 4);
    t1 = VA(0, 0x13fff0, 0x101000, 4);
    printf("t1_above_lo %d granule %llu\n", t1 > lo, U(t1 % 65536));
    p = (uint8_t *)t1; memset(p, 0xCD, 0x140000);
    VF(t1, 0, 0x8000);
    t1 = VA(0, 0x13fff0, 0x101000, 4);
    printf("t1_zero %d\n", zero((uint8_t *)t1, 0x140000));
    t2 = VA(0, 0x20000, 0x102000, 4);
    printf("t2_below_t1 %d t2_end_at_t1 %d\n", t2 < t1, t2 + 0x20000 == t1);
    t3 = VA(0, 0x1000, 0x101000, 4);
    printf("t3_below_t2 %d t3_granule %d\n", t3 < t2, t3 % 65536 == 0);
    mid = VA(0, 0x10000, 0x1000, 4);
    printf("bottom_first_fit %d\n", mid == lo + 0x10000);
    VF(t2, 0, 0x8000);
    t2 = VA(0, 0x10000, 0x101000, 4);
    printf("freed_range_reused %d\n", t2 + 0x10000 == t1 && t2 > t3);
    fle_set_last_error(0);
    big = VA(0, 0x20000000, 0x101000, 4);
    printf("too_big_fails %d\n", big == 0);
    VF(lo, 0, 0x8000); VF(mid, 0, 0x8000); VF(t1, 0, 0x8000); VF(t2, 0, 0x8000); VF(t3, 0, 0x8000);
    printf("regions_back %llu\n", U(fle_sys_region_count() - n0));
    return 0;
}}
"#
    );
    let o = build_run("wp-arena-top", &src, &["-DFLE_VMEM_ARENA"], false, &["fle_sys.c", "fle_vmap.c", "fle_vfs.c", "fle_env.c", "fle_imports.c", "fle_host.c"], &[]);
    assert!(o.ok, "signal {:?}: {}\n{}", o.signal, o.stderr, o.stdout);
    let want = [
        "t1_above_lo 1 granule 0",
        "t1_zero 1",
        "t2_below_t1 1 t2_end_at_t1 1",
        "t3_below_t2 1 t3_granule 1",
        "bottom_first_fit 1",
        "freed_range_reused 1",
        "too_big_fails 1",
        "regions_back 0",
    ];
    assert_eq!(o.stdout.lines().collect::<Vec<_>>(), want, "{}", o.stderr);
}

fn hi32_program(case: &str) -> String {
    format!(
        r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_host.h"
#ifdef __EMSCRIPTEN__
_Thread_local unsigned char fle_ctx[0x300] __attribute__((aligned(64), used));   /* the register block of the wasm build (fle_ctx_tls.c): R, FL and X live in it */
#else
uint64_t R[16];
#endif
static uint8_t buf[64];
int main(void)
{{
    uint64_t b = (uint64_t)(uintptr_t)buf;
    fle_st(b, 8, 0x1122334455667788ull);
    printf("good %llx\n", (unsigned long long)fle_ld(b, 8));
    fflush(stdout);
    {case}
    printf("not reached\n");
    return 0;
}}
"#
    )
}

#[test]
fn address_check_stops_on_bits_above_32_and_passes_a_good_address() {
    let cases = [
        ("wild-load", "(void)fle_ld(0x6e69727453657261ull, 8);", "load"),
        ("alias-store", "fle_st(b + (1ull << 32), 8, 1);", "store"),
        ("just-above", "(void)fle_ld(0x100000000ull, 1);", "load"),
    ];
    for (name, stmt, kind) in cases {
        let o = build_run(&format!("wp-hi32-{name}"), &hi32_program(stmt), &["-DFLE_CHECK_HI32"], false, &["fle_host.c"], &[]);
        assert_eq!(o.stdout, "good 1122334455667788\n", "{name}");
        assert_eq!(o.signal, Some(6), "{name}: {}", o.stderr);
        assert!(o.stderr.contains("[fle_addr_hi32]") && o.stderr.contains(&format!("-byte {kind}")), "{name}: {}", o.stderr);
    }
    // a good address, and the largest 32-bit one, do not stop here (fle_addr is only a conversion for them)
    let o = build_run("wp-hi32-ok", &hi32_program("(void)fle_ld(b + 8, 8);"), &["-DFLE_CHECK_HI32"], false, &["fle_host.c"], &[]);
    assert!(o.ok && o.stdout.ends_with("not reached\n"), "{}", o.stderr);
}

#[test]
fn native_build_has_no_address_check() {
    // The default native build converts exactly as before: an address above 4 GiB reaches the access (SIGSEGV).
    let o = build_run("wp-hi32-native", &hi32_program("(void)fle_ld(0x6e69727453657261ull, 8);"), &[], true, &[], &[]);
    assert_eq!(o.signal, Some(11), "{}", o.stderr);
}
