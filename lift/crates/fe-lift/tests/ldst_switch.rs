//! fle_ld / fle_st of lift/runtime/fle_runtime.h (decision 0237): the access is a switch on n with constant-size accesses for 1, 2, 4, 8 and the memcpy with a run-time n for every other n.
//! A run-time n (the function is not inlined, or the argument is not a constant) made clang/emscripten emit a call to memcpy / a wasm memory.copy per access; the switch must give the same
//! bits. The program compares every n from 0 to 8, at 16 offsets, with a byte-wise little-endian reference (loads: the n bytes in the low bytes, the rest 0; stores: exactly the low n bytes,
//! the bytes around them untouched), across a page boundary, and checks that a rejected address (FLE_CHECK_HI32) is reported with the same n and kind and that nothing is written.
//! Built native at -O0, -O2 and with -DFLE_ALWAYS_INLINE, and (when emcc and node are installed) as wasm.

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

const PROGRAM: &str = r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <signal.h>
#include <unistd.h>
#ifndef __EMSCRIPTEN__
#include <sys/mman.h>
#endif
#include "fle_runtime.h"
#include "fle_host.h"
#ifdef __EMSCRIPTEN__
_Thread_local unsigned char fle_ctx[0x300] __attribute__((aligned(64), used));   /* the register block of the wasm build (fle_ctx_tls.c): R, FL and X live in it */
#else
uint64_t R[16];
#endif
/* the size reaches the access through a volatile: n is not a constant at the use */
static volatile unsigned vn;
__attribute__((noinline)) static uint64_t ld_rt(uint64_t a, unsigned n) { vn = n; return fle_ld(a, vn); }
__attribute__((noinline)) static void st_rt(uint64_t a, unsigned n, uint64_t v) { vn = n; fle_st(a, vn, v); }
static uint64_t ref_ld(const uint8_t *p, unsigned n) { uint64_t v = 0; for (unsigned i = 0; i < n; i++) v |= (uint64_t)p[i] << (8 * i); return v; }
static uint64_t seed = 0x9e3779b97f4a7c15ull;
static uint64_t rnd(void) { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; return seed; }
static uint8_t area[256] __attribute__((aligned(16)));
static uint8_t ref_area[256];
static int bad;
#define CHECK(c, ...) do { if (!(c)) { bad++; printf("FAIL " __VA_ARGS__); printf("\n"); } } while (0)

static int values(void)
{
    static const uint64_t vals[] = { 0, ~0ull, 0x0102030405060708ull, 0x8000000000000000ull, 0x00000000ffffffffull, 0xffffffff00000000ull, 0x7f, 0x80, 0xff, 0x8000, 0x80000000ull };
    int checks = 0;
    for (unsigned i = 0; i < sizeof area; i++) area[i] = (uint8_t)(rnd() >> 24);
    for (unsigned n = 0; n <= 8; n++) {
        for (unsigned off = 0; off < 16; off++) {
            uint64_t a = (uint64_t)(uintptr_t)(area + 64 + off);
            uint64_t got = ld_rt(a, n), want = ref_ld(area + 64 + off, n);
            CHECK(got == want, "ld n=%u off=%u got=%llx want=%llx", n, off, (unsigned long long)got, (unsigned long long)want); checks++;
            for (unsigned k = 0; k < sizeof vals / sizeof vals[0] + 6; k++) {
                uint64_t v = k < sizeof vals / sizeof vals[0] ? vals[k] : rnd();
                for (unsigned i = 0; i < sizeof area; i++) area[i] = (uint8_t)(0xa0 + i);
                memcpy(ref_area, area, sizeof area);
                for (unsigned i = 0; i < n; i++) ref_area[64 + off + i] = (uint8_t)(v >> (8 * i));
                st_rt(a, n, v);
                CHECK(memcmp(area, ref_area, sizeof area) == 0, "st n=%u off=%u v=%llx", n, off, (unsigned long long)v); checks++;
            }
            for (unsigned i = 0; i < sizeof area; i++) area[i] = (uint8_t)(rnd() >> 24);
        }
    }
    printf("values checks=%d bad=%d\n", checks, bad);
    return bad != 0;
}

/* an access that ends exactly at, straddles, and starts at a 64 KiB boundary of two adjacent pages */
static int straddle(void)
{
    enum { PG = 65536 };
    uint8_t *m = aligned_alloc(PG, 2 * PG);
    int checks = 0;
    if (!m) { printf("FAIL alloc\n"); return 1; }
    for (unsigned i = 0; i < 2 * PG; i++) m[i] = (uint8_t)(rnd() >> 24);
    for (unsigned n = 1; n <= 8; n++) {
        for (int d = -(int)n; d <= 0; d++) {   /* the access starts d bytes from the boundary: d = -n ends at it, -1 straddles by one byte, 0 starts at it */
            uint8_t *p = m + PG + d;
            uint64_t a = (uint64_t)(uintptr_t)p;
            CHECK(ld_rt(a, n) == ref_ld(p, n), "straddle ld n=%u d=%d", n, d); checks++;
            uint64_t v = rnd();
            memcpy(ref_area, p - 8, 24);
            for (unsigned i = 0; i < n; i++) ref_area[8 + i] = (uint8_t)(v >> (8 * i));
            st_rt(a, n, v);
            CHECK(memcmp(p - 8, ref_area, 24) == 0, "straddle st n=%u d=%d", n, d); checks++;
        }
    }
    printf("straddle checks=%d bad=%d\n", checks, bad);
    free(m);
    return bad != 0;
}


#ifndef __EMSCRIPTEN__
/* the last readable/writable byte of a mapping is followed by a PROT_NONE page: an access that ends exactly at it must not fault (no wider access than n bytes), one that crosses it must */
static uint8_t *guarded(void)
{
    long pg = sysconf(_SC_PAGESIZE);
    uint8_t *m = mmap(0, 2 * pg, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (m == MAP_FAILED || mprotect(m + pg, pg, PROT_NONE)) { printf("FAIL mmap\n"); exit(2); }
    for (long i = 0; i < pg; i++) m[i] = (uint8_t)(rnd() >> 24);
    return m + pg;
}
static int guard_end(void)
{
    uint8_t *g = guarded();
    for (unsigned n = 1; n <= 8; n++) {
        uint64_t a = (uint64_t)(uintptr_t)(g - n);
        CHECK(ld_rt(a, n) == ref_ld(g - n, n), "guard-end ld n=%u", n);
        uint64_t v = rnd();
        st_rt(a, n, v);
        CHECK(ld_rt(a, n) == (n == 8 ? v : v & ((1ull << (8 * n)) - 1)), "guard-end st n=%u", n);
    }
    printf("guard-end bad=%d\n", bad);
    return bad != 0;
}
static int guard_cross(int store, unsigned n)
{
    uint8_t *g = guarded();
    uint64_t a = (uint64_t)(uintptr_t)(g - 1);
    if (store) st_rt(a, n, ~0ull); else (void)ld_rt(a, n);
    printf("not reached\n");
    return 0;
}
#endif

#ifdef FLE_CHECK_HI32
static void on_abort(int s) { (void)s; unsigned sum = 0; for (unsigned i = 0; i < sizeof area; i++) sum += area[i] * (i + 1); printf("area-after=%u\n", sum); fflush(stdout); _exit(6); }
#endif
/* a rejected address: stops (abort) with the same n and kind in the message, before anything is read or written; MODE hi32-ld-N | hi32-st-N */
static int hi32(int store, unsigned n)
{
    for (unsigned i = 0; i < sizeof area; i++) area[i] = (uint8_t)(0xa0 + i);
    unsigned sum = 0; for (unsigned i = 0; i < sizeof area; i++) sum += area[i] * (i + 1);
    printf("area-before=%u\n", sum); fflush(stdout);
#ifdef FLE_CHECK_HI32
    signal(SIGABRT, on_abort);
#endif
    uint64_t a = (uint64_t)(uintptr_t)area + (1ull << 32);
    if (store) st_rt(a, n, ~0ull); else (void)ld_rt(a, n);
    printf("not reached\n");
    return 0;
}

int main(int argc, char **argv)
{
    const char *m = argc > 1 ? argv[1] : "values";
    if (!strcmp(m, "values")) return values();
    if (!strcmp(m, "straddle")) return straddle();
    if (!strncmp(m, "hi32-ld-", 8)) return hi32(0, (unsigned)atoi(m + 8));
    if (!strncmp(m, "hi32-st-", 8)) return hi32(1, (unsigned)atoi(m + 8));
#ifndef __EMSCRIPTEN__
    if (!strcmp(m, "guard-end")) return guard_end();
    if (!strncmp(m, "guard-cross-ld-", 15)) return guard_cross(0, (unsigned)atoi(m + 15));
    if (!strncmp(m, "guard-cross-st-", 15)) return guard_cross(1, (unsigned)atoi(m + 15));
#endif
    return 2;
}
"#;

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    code: Option<i32>,
}

fn run(exe: &PathBuf, mode: &str) -> Out {
    let r = Command::new(exe).arg(mode).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), code: r.status.code() }
}

fn build_native(name: &str, flags: &[&str]) -> PathBuf {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    std::fs::write(&c, PROGRAM).unwrap();
    let cc = Command::new("clang")
        .args(["-std=gnu11", "-w", "-fno-pie", "-no-pie"])
        .args(flags)
        .arg("-I")
        .arg(rt())
        .arg(&c)
        .arg(rt().join("fle_host.c"))
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn expect_equal_to_the_reference(exe: &PathBuf, what: &str) {
    for mode in ["values", "straddle"] {
        let o = run(exe, mode);
        assert_eq!(o.code, Some(0), "{what} {mode}: {}{}", o.stdout, o.stderr);
        assert!(o.stdout.contains("bad=0"), "{what} {mode}: {}", o.stdout);
    }
}

#[test]
fn native_fle_ld_and_fle_st_equal_the_byte_wise_reference_for_every_n() {
    for (name, flags) in [("ldst-o0", vec!["-O0"]), ("ldst-o2", vec!["-O2"]), ("ldst-o2-ai", vec!["-O2", "-DFLE_ALWAYS_INLINE"]), ("ldst-oz", vec!["-Oz"])] {
        let exe = build_native(name, &flags);
        expect_equal_to_the_reference(&exe, name);
    }
}

#[test]
fn native_access_is_exactly_n_bytes_wide_at_the_end_of_a_mapping() {
    for (name, flags) in [("ldst-guard-o0", vec!["-O0"]), ("ldst-guard-o2", vec!["-O2"]), ("ldst-guard-oz", vec!["-Oz"])] {
        let exe = build_native(name, &flags);
        let o = run(&exe, "guard-end");
        assert_eq!(o.code, Some(0), "{name} guard-end: {}{}", o.stdout, o.stderr);
        assert!(o.stdout.contains("bad=0"), "{name}: {}", o.stdout);
        // an access that starts one byte before the PROT_NONE page crosses it (n >= 2): SIGSEGV; n = 1 stays inside
        for n in 1u32..=8 {
            for (kind, mode) in [("ld", format!("guard-cross-ld-{n}")), ("st", format!("guard-cross-st-{n}"))] {
                let o = run(&exe, &mode);
                if n == 1 {
                    assert_eq!(o.code, Some(0), "{name} {mode}: {}{}", o.stdout, o.stderr);
                } else {
                    assert_eq!(o.signal, Some(11), "{name} {kind} n={n} did not fault: {}{}", o.stdout, o.stderr);
                }
            }
        }
    }
}

#[test]
fn a_rejected_address_stops_with_the_same_size_and_kind_and_touches_nothing() {
    let exe = build_native("ldst-hi32", &["-O2", "-DFLE_CHECK_HI32"]);
    for n in [1u32, 2, 3, 4, 5, 6, 7, 8] {
        for (kind, mode) in [("load", format!("hi32-ld-{n}")), ("store", format!("hi32-st-{n}"))] {
            let o = run(&exe, &mode);
            assert_eq!(o.code, Some(6), "{mode}: {}{}", o.stdout, o.stderr);
            assert!(o.stderr.contains(&format!("({n}-byte {kind})")) && o.stderr.contains("[fle_addr_hi32]"), "{mode}: {}", o.stderr);
            let line = |k: &str| o.stdout.lines().find_map(|l| l.strip_prefix(k).map(|s| s.to_string()));
            assert_eq!(line("area-before="), line("area-after="), "{mode}: the area changed: {}", o.stdout);
            assert!(!o.stdout.contains("not reached"), "{mode}");
        }
    }
}

#[test]
fn wasm_fle_ld_and_fle_st_equal_the_byte_wise_reference_for_every_n() {
    let emcc = PathBuf::from("/opt/emsdk/upstream/emscripten/emcc");
    let have_node = Command::new("node").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
    if !emcc.exists() || !have_node {
        eprintln!("SKIPPED: emcc ({}) or node is not installed", emcc.display());
        return;
    }
    let dir = work();
    let path = format!("/opt/emsdk:/opt/emsdk/upstream/emscripten:{}", std::env::var("PATH").unwrap_or_default());
    for (name, flags) in [("ldst-wasm-o2", vec!["-O2"]), ("ldst-wasm-o2-ai", vec!["-O2", "-DFLE_ALWAYS_INLINE"]), ("ldst-wasm-oz", vec!["-Oz"])] {
        let u = etxt_common::uniq(name);
        let c = dir.join(format!("{u}.c"));
        let js = dir.join(format!("{u}.js"));
        std::fs::write(&c, PROGRAM).unwrap();
        let cc = Command::new(&emcc)
            .args(["-std=gnu11", "-w"])
            .args(&flags)
            .arg("-I")
            .arg(rt())
            .args(["-sALLOW_MEMORY_GROWTH=1", "-sENVIRONMENT=node", "-sEXIT_RUNTIME=1", "-o"])
            .arg(&js)
            .arg(&c)
            .arg(rt().join("fle_host.c")) // fle_addr_fault: wasm32 builds check guest addresses
            .env("PATH", &path)
            .env("TMPDIR", dir.join("tmp"))
            .env("EM_CACHE", dir.join("emcache"))
            .output()
            .expect("run emcc");
        assert!(cc.status.success(), "emcc failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
        for mode in ["values", "straddle"] {
            let r = Command::new("node").arg(&js).arg(mode).output().expect("run node");
            let o = String::from_utf8_lossy(&r.stdout).to_string();
            assert!(r.status.success() && o.contains("bad=0"), "{name} {mode}: {o}{}", String::from_utf8_lossy(&r.stderr));
        }
        // a rejected address stops in wasm too (abort), with the n and kind of the access
        for (n, mode) in [(3u32, "hi32-ld-3"), (4, "hi32-st-4")] {
            let r = Command::new("node").arg(&js).arg(mode).output().expect("run node");
            let e = String::from_utf8_lossy(&r.stderr).to_string();
            assert!(!r.status.success() && e.contains(&format!("({n}-byte {})", if mode.contains("-st-") { "store" } else { "load" })), "{name} {mode}: {e}");
        }
    }
}
