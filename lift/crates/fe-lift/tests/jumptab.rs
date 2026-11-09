//! Annotated jump tables as a `switch` (decision 0036, W2; fe-decode/src/jumptab.rs, fe-lift/src/jumptab.rs).
//!
//! 1. Synthetic PE, two switch forms: `jmp [r10+rdx*8+0x40]` over 8-byte absolute VAs (the f_40d210 form) and
//!    `lea; movsxd; add; jmp rcx` over 4-byte offsets with repeated targets (the MSVC form). The generated C is built by
//!    clang -O0 together with harness/image.c, which maps the synthetic image at its ImageBase, so the same code also
//!    runs natively on the host (ms_abi call) and the lifted result is compared with the native one for every index. An
//!    index outside the table stops the lifted code in `fle_jumptab_miss`.
//! 2. Real FLEngine_x64.dll ($FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll; if FLENGINE_PE is set and
//!    the file is missing the test fails; if only the default is missing it is skipped, as the other real-DLL tests): the
//!    four annotated functions are cut out of the generated C, an entry `goto` to the block that holds the guard is
//!    inserted (the function itself for f_40d210) and a hit probe is put on each table target and on the guard's
//!    out-of-range target. For every index the lifted code and the real instructions (mapped by image.c, 0xCC planted at the
//!    same addresses, run natively under a SIGTRAP handler) must reach the same address, and it must be the one the table
//!    bytes give (read here, from the file, not from the generator). f_40d210 is also run to the end as a memset against
//!    the host's memset.
//!
//! Outputs go under lift/out-ijmp/test.

mod etxt_common;

use fe_lift::generate_with_specs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-ijmp/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

fn clang(args: &[&str], dir: &Path) {
    let r = Command::new("clang")
        .env("TMPDIR", dir.join("tmp"))
        .args(["-std=gnu11", "-O0", "-w", "-fno-builtin", "-ffp-contract=off", "-fPIE", "-mno-red-zone", "-I"])
        .arg(crate_dir().join("../../runtime"))
        .arg("-I")
        .arg(crate_dir().join("../../harness"))
        .args(args)
        .output()
        .expect("run clang (is clang installed?)");
    assert!(r.status.success(), "clang failed: {}", String::from_utf8_lossy(&r.stderr));
}

// ---------------------------------------------------------------------------------------------
// 1. Synthetic PE
// ---------------------------------------------------------------------------------------------

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const FILE_LEN: usize = 0x800;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

/// Block `k` of group `base`: `mov eax, k; ret` at `base + 8*slot`.
const F1_PERM: [u32; 8] = [3, 1, 4, 0, 6, 2, 5, 7]; // slot of idx -7+j
const F2_ENT: [u32; 4] = [0, 1, 0, 2]; // block of entry j

fn build() -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 1);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, 0x1200);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x2000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);

    let mut put = |rva: u32, bytes: &[u8]| {
        let o = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    };
    // F1 at 0x1000: lea r10,[rip+5]; jmp [r10+rdx*8+0x40]; table at 0x100c.. (idx -8 at 0x100c)
    put(0x1000, &[0x4c, 0x8d, 0x15, 0x05, 0, 0, 0]);
    put(0x1007, &[0x41, 0xff, 0x64, 0xd2, 0x40]);
    put(0x100c, &(IB + 0x1060).to_le_bytes()); // idx -8: not part of the annotated table
    for j in 0..8u32 {
        put(0x1014 + 8 * j, &(IB + 0x1060 + 8 * F1_PERM[j as usize] as u64).to_le_bytes());
    }
    for s in 0..8u32 {
        put(0x1060 + 8 * s, &[0xb8, (0x30 + s) as u8, 0x03, 0, 0, 0xc3]); // mov eax,0x330+s; ret
    }
    // F2 at 0x1100: lea rax,[rip+0x29]; movsxd rcx,[rax+rcx*4]; add rcx,rax; jmp rcx; table at 0x1130
    put(0x1100, &[0x48, 0x8d, 0x05, 0x29, 0, 0, 0]);
    put(0x1107, &[0x48, 0x63, 0x0c, 0x88]);
    put(0x110b, &[0x48, 0x01, 0xc1]);
    put(0x110e, &[0xff, 0xe1]);
    for (j, m) in F2_ENT.iter().enumerate() {
        put(0x1130 + 4 * j as u32, &((0x1140 + 8 * *m as i32) - 0x1130).to_le_bytes());
    }
    for m in 0..3u32 {
        put(0x1140 + 8 * m, &[0xb8, m as u8, 0x04, 0, 0, 0xc3]); // mov eax,0x400+m; ret
    }
    // main at 0x1200: call F1; call F2; ret
    put(0x1200, &[&[0xe8][..], &rel(0x1205, 0x1000)].concat());
    put(0x1205, &[&[0xe8][..], &rel(0x120a, 0x1100)].concat());
    put(0x120a, &[0xc3]);
    b
}

fn specs() -> [fe_decode::Spec; 2] {
    use fe_decode::jumptab::{Kind, Spec};
    [
        Spec {
            jmp_va: IB + 0x1007,
            code: &[0x41, 0xff, 0x64, 0xd2, 0x40],
            func: (IB + 0x1000, IB + 0x1100),
            table: IB + 0x1014,
            count: 8,
            kind: Kind::Abs64,
        },
        Spec {
            jmp_va: IB + 0x110e,
            code: &[0xff, 0xe1],
            func: (IB + 0x1100, IB + 0x1180),
            table: IB + 0x1130,
            count: 4,
            kind: Kind::Rel32 { base: IB + 0x1130 },
        },
    ]
}

const SYN_MAIN: &str = r#"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "boot.h"
extern uint64_t R[16];
void f_140001000(void);
void f_140001100(void);
typedef uint32_t (__attribute__((ms_abi)) *nfn)(int64_t, int64_t);

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    long lo, hi, i;
    if (argc < 5) return 2;
    if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    lo = strtol(argv[3], 0, 0);
    hi = strtol(argv[4], 0, 0);
    for (i = lo; i <= hi; i++) {
        uint32_t nat = 0, lif;
        if (!strcmp(argv[2], "f1")) {
            if (argc < 6) nat = ((nfn)(uintptr_t)0x140001000)(0, i);
            R[2] = (uint64_t)i;
            f_140001000();
        } else {
            if (argc < 6) nat = ((nfn)(uintptr_t)0x140001100)(i, 0);
            R[1] = (uint64_t)i;
            f_140001100();
        }
        lif = (uint32_t)R[0];
        printf("%ld %x %x\n", i, nat, lif);
        fflush(stdout);
    }
    return 0;
}
"#;

fn syn_exe() -> (PathBuf, PathBuf) {
    static ONCE: std::sync::OnceLock<(PathBuf, PathBuf)> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = out_dir();
        let img = build();
        let pe = dir.join(format!("{}.pe", etxt_common::per_process("syn")));
        std::fs::write(&pe, &img).unwrap();
        let mut out = Vec::new();
        generate_with_specs(&img, &[], &specs(), &mut out).expect("generate");
        let c = dir.join(format!("{}.c", etxt_common::per_process("syn_lift")));
        std::fs::write(&c, &out).unwrap();
        let m = dir.join(format!("{}.c", etxt_common::per_process("syn_main")));
        std::fs::write(&m, SYN_MAIN).unwrap();
        let exe = dir.join(etxt_common::per_process("syn_exe"));
        clang(
            &["-o", exe.to_str().unwrap(), c.to_str().unwrap(), m.to_str().unwrap(), crate_dir().join("../../harness/image.c").to_str().unwrap()],
            &dir,
        );
        (exe, pe)
    })
    .clone()
}

#[test]
fn synthetic_generator_output() {
    let mut out = Vec::new();
    let st = generate_with_specs(&build(), &[], &specs(), &mut out).expect("generate");
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.jumptabs, 2);
    assert!(st.jumptab_rejected.is_empty(), "{:?}", st.jumptab_rejected);
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
    assert_eq!(st.handled["jmp_ind:table"], 2);
    // only the register-operand jump counts as a register jmp (F1's jmp is memory-form)
    assert_eq!(st.register_jmp_sites, 1);
    assert_eq!(st.register_jmps["jmp_ind:table"], 1, "{:?}", st.register_jmps);
    // F1: one case per slot address, 8 cases, selector is the effective address
    for j in 0..8u64 {
        assert!(c.contains(&format!("case UINT64_C({:#x}): ", IB + 0x1014 + 8 * j)), "slot {j}");
    }
    assert!(!c.contains(&format!("case UINT64_C({:#x}): ", IB + 0x100c)), "idx -8 must not be a case");
    // F2: 3 distinct targets for 4 entries
    for m in 0..3u64 {
        assert!(c.contains(&format!("case UINT64_C({:#x}): ", IB + 0x1140 + 8 * m)), "target {m}");
    }
    assert_eq!(c.matches("fle_jumptab_miss(UINT64_C(0x14000110e), t)").count(), 1);
    assert_eq!(c.matches("fle_jumptab_miss(UINT64_C(0x140001007), t)").count(), 1);
    // the targets of the table enter the function bodies
    assert!(c.contains(&format!("L_{:x}: ", IB + 0x1098)), "last F1 target block");
}

fn lines(o: &std::process::Output) -> Vec<Vec<String>> {
    String::from_utf8_lossy(&o.stdout).lines().map(|l| l.split(' ').map(String::from).collect()).collect()
}

#[test]
fn synthetic_every_index_matches_native_and_the_table() {
    let (exe, pe) = syn_exe();
    // F1: idx -7..0 -> block F1_PERM[idx+7]
    let r = Command::new(&exe).arg(&pe).args(["f1", "-7", "0"]).output().unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let l = lines(&r);
    assert_eq!(l.len(), 8);
    for (j, row) in l.iter().enumerate() {
        let want = format!("{:x}", 0x330 + F1_PERM[j]);
        assert_eq!(row[0], format!("{}", j as i64 - 7));
        assert_eq!(row[1], want, "native, idx {}", j as i64 - 7);
        assert_eq!(row[2], want, "lifted, idx {}", j as i64 - 7);
    }
    // F2: idx 0..3 -> block F2_ENT[idx]
    let r = Command::new(&exe).arg(&pe).args(["f2", "0", "3"]).output().unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let l = lines(&r);
    assert_eq!(l.len(), 4);
    for (j, row) in l.iter().enumerate() {
        let want = format!("{:x}", 0x400 + F2_ENT[j]);
        assert_eq!(row[1], want, "native, idx {j}");
        assert_eq!(row[2], want, "lifted, idx {j}");
    }
}

#[test]
fn synthetic_index_outside_the_table_aborts() {
    let (exe, pe) = syn_exe();
    // lifted only (argc 6): the native code would jump to garbage
    for (f, idx) in [("f1", "-8"), ("f1", "1"), ("f1", "100"), ("f2", "4"), ("f2", "100")] {
        let r = Command::new(&exe).arg(&pe).args([f, idx, idx, "lifted-only"]).output().unwrap();
        assert!(!r.status.success(), "{f} {idx} did not stop: {}", String::from_utf8_lossy(&r.stdout));
        let e = String::from_utf8_lossy(&r.stderr).to_string();
        assert!(e.contains("outside the table"), "{f} {idx}: {e}");
        assert!(String::from_utf8_lossy(&r.stdout).is_empty(), "{f} {idx} ran on");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. Real DLL
// ---------------------------------------------------------------------------------------------

fn real_dll_path() -> Option<PathBuf> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => crate_dir().join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    match std::fs::metadata(&path) {
        Ok(_) => Some(path),
        Err(e) if explicit.is_some() => panic!("FLENGINE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

struct Img<'a>(&'a [u8]);
impl Img<'_> {
    fn rd(&self, va: u64, n: usize) -> &[u8] {
        let d = self.0;
        let lf = u32::from_le_bytes(d[0x3c..0x40].try_into().unwrap()) as usize;
        let ns = u16::from_le_bytes(d[lf + 6..lf + 8].try_into().unwrap()) as usize;
        let osz = u16::from_le_bytes(d[lf + 20..lf + 22].try_into().unwrap()) as usize;
        let ib = u64::from_le_bytes(d[lf + 48..lf + 56].try_into().unwrap());
        let rva = (va - ib) as u32;
        for k in 0..ns {
            let s = lf + 24 + osz + 40 * k;
            let u = |o: usize| u32::from_le_bytes(d[s + o..s + o + 4].try_into().unwrap());
            let (vs, sva, rs, rp) = (u(8), u(12), u(16), u(20));
            if rva >= sva && rva < sva + vs.max(rs) {
                let off = rp as usize + (rva - sva) as usize;
                return &d[off..off + n];
            }
        }
        panic!("va {va:#x} is in no section");
    }
    fn abs64(&self, va: u64) -> u64 {
        u64::from_le_bytes(self.rd(va, 8).try_into().unwrap())
    }
    fn rel32(&self, base: u64, k: u64) -> u64 {
        base.wrapping_add(i32::from_le_bytes(self.rd(base + 4 * k, 4).try_into().unwrap()) as i64 as u64)
    }
}

struct Kase {
    f: u32,
    blk: u64,
    rcx: u64,
    rdx: u64,
    r8: u64,
    r15: u64,
    want: u64,
}

struct Fun {
    id: u32,
    va: u64,
    blk: u64,
}

const FUNS: [Fun; 4] = [
    Fun { id: 0, va: 0x5e5860, blk: 0x5e5e9c },
    Fun { id: 1, va: 0x5e8da0, blk: 0x5e9ac8 },
    Fun { id: 2, va: 0x5f05f0, blk: 0x5f1480 },
    Fun { id: 3, va: 0x40d210, blk: 0 },
];

fn real_cases(img: &Img<'_>) -> Vec<Kase> {
    let mut v = Vec::new();
    // A: add ecx,-0x45; cmp ecx,0x22; ja 0x5e63ae; table 0x5e6d44 (35)
    for idx in 0..=36u64 {
        let want = if idx <= 34 { img.rel32(0x5e6d44, idx) } else { 0x5e63ae };
        v.push(Kase { f: 0, blk: 0x5e5e9c, rcx: idx + 0x45, rdx: 0, r8: 0, r15: 0, want });
    }
    // B: add ecx,-0x3f34; cmp ecx,0x1e; ja 0x5eaf38; table 0x5eb358 (31), r15 = table
    for idx in 0..=32u64 {
        let want = if idx <= 30 { img.rel32(0x5eb358, idx) } else { 0x5eaf38 };
        v.push(Kase { f: 1, blk: 0x5e9ac8, rcx: idx + 0x3f34, rdx: 0, r8: 0, r15: 0x5eb358, want });
    }
    // C: cmp edx,5; ja 0x5f161c; table 0x5f1664 (6)
    for idx in 0..=7u64 {
        let want = if idx <= 5 { img.rel32(0x5f1664, idx) } else { 0x5f161c };
        v.push(Kase { f: 2, blk: 0x5f1480, rcx: 0, rdx: idx, r8: 0, r15: 0, want });
    }
    // D: memset-like, n = 1..8 reaches `jmp [r10+rdx*8+0x40]` with rdx = n-8 = -7..0; slot 0x40d260 + 8*(n-1)
    for n in 1..=8u64 {
        v.push(Kase { f: 3, blk: 0, rcx: 0, rdx: n, r8: 0x41, r15: 0, want: img.abs64(0x40d260 + 8 * (n - 1)) });
    }
    v
}

fn cut(c: &str, f: u64) -> String {
    let head = format!("void f_{f:x}(void)\n{{\n");
    let i = c.find(&head).unwrap_or_else(|| panic!("no definition of f_{f:x}"));
    let rest = &c[i..];
    let e = rest.find("\n}\n\n").unwrap() + 3;
    rest[..e].to_string()
}

fn idents(s: &str, prefix: &str) -> std::collections::BTreeSet<String> {
    s.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|t| t.starts_with(prefix) && t.len() > prefix.len())
        .map(String::from)
        .collect()
}

const REAL_MAIN: &str = r#"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>
#include <ucontext.h>
#include "boot.h"
#include "fle_dispatch.h"

extern uint64_t R[16];
const fle_fn_entry fle_dispatch_table[] = { { 0, 0 } };
const uint64_t fle_dispatch_count = 0;
static int hit_mode;
void fle_hit(uint64_t va) { if (hit_mode) { printf("%llx\n", (unsigned long long)va); fflush(stdout); _exit(0); } }
void fle_stub_abort(const char *n) { fprintf(stderr, "stub %s called\n", n); abort(); }
void f_5e5860(void); void f_5e8da0(void); void f_5f05f0(void); void f_40d210(void);

typedef struct { int f; uint64_t blk, rcx, rdx, r8, r15; } kase;
static const kase cases[] = {
@CASES@
};
static const uint64_t patch[] = { @PATCH@ };
static const uint64_t entry[4] = { 0x5e5860, 0x5e8da0, 0x5f05f0, 0x40d210 };
static uint8_t dbuf[0x200];

static void on_trap(int sig, siginfo_t *si, void *u)
{
    ucontext_t *uc = u;
    (void)sig; (void)si;
    printf("%llx\n", (unsigned long long)(uc->uc_mcontext.gregs[REG_RIP] - 1));
    fflush(stdout);
    _exit(0);
}

static void run_native(const kase *k)
{
    struct { uint64_t rcx, rdx, r8, r13, r15, go; } r;
    r.rcx = k->f == 3 ? (uint64_t)(uintptr_t)dbuf : k->rcx;
    r.rdx = k->rdx; r.r8 = k->r8; r.r13 = 0; r.r15 = k->r15;
    r.go = k->blk ? k->blk : entry[k->f];
    asm volatile("mov %0, %%r11\n\t"
                 "mov 0(%%r11), %%rcx\n\t"
                 "mov 8(%%r11), %%rdx\n\t"
                 "mov 16(%%r11), %%r8\n\t"
                 "mov 24(%%r11), %%r13\n\t"
                 "mov 32(%%r11), %%r15\n\t"
                 "jmp *40(%%r11)"
                 : : "r"(&r) : "memory");
}

static void run_lifted(const kase *k)
{
    memset(R, 0, sizeof R);
    R[1] = k->f == 3 ? (uint64_t)(uintptr_t)dbuf : k->rcx;
    R[2] = k->rdx; R[8] = k->r8; R[15] = k->r15;
    hit_mode = 1;
    switch (k->f) {
    case 0: f_5e5860(); break;
    case 1: f_5e8da0(); break;
    case 2: f_5f05f0(); break;
    default: f_40d210(); break;
    }
}

typedef void (__attribute__((ms_abi)) *msfn)(void *, int64_t, int64_t);

static int mem_mode(void)
{
    static uint8_t b1[0x200], b2[0x200], b3[0x200];
    int n, off, bad = 0, total = 0;
    static const int vals[] = { 0x00, 0x5a, 0xff, 0x41 };
    size_t vi;
    for (n = 0; n <= 0x60; n++) {
        for (off = 0; off < 16; off += 3) {
            for (vi = 0; vi < sizeof vals / sizeof vals[0]; vi++) {
                int val = vals[vi];
                memset(b1, 0xcc, sizeof b1); memset(b2, 0xcc, sizeof b2); memset(b3, 0xcc, sizeof b3);
                memset(b3 + 0x20 + off, val, n);
                ((msfn)(uintptr_t)0x40d210)(b1 + 0x20 + off, n, val);
                memset(R, 0, sizeof R);
                R[1] = (uint64_t)(uintptr_t)(b2 + 0x20 + off); R[2] = (uint64_t)n; R[8] = (uint64_t)val;
                hit_mode = 0;
                f_40d210();
                total++;
                if (memcmp(b1, b3, sizeof b1) != 0 || memcmp(b2, b3, sizeof b2) != 0) {
                    bad++;
                    printf("mismatch n=%d off=%d val=%x native_ok=%d lifted_ok=%d\n", n, off, val,
                           memcmp(b1, b3, sizeof b1) == 0, memcmp(b2, b3, sizeof b2) == 0);
                }
            }
        }
    }
    printf("mem cases=%d mismatches=%d\n", total, bad);
    return bad != 0;
}

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    size_t i, k;
    int native;
    if (argc < 3) return 2;
    if (image_load(argv[2], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    if (!strcmp(argv[1], "mem")) return mem_mode();
    native = !strcmp(argv[1], "native");
    for (k = 0; k < sizeof cases / sizeof cases[0]; k++) {
        pid_t p;
        int st;
        printf("%zu ", k);
        fflush(stdout);
        p = fork();
        if (p == 0) {
            alarm(10);
            if (native) {
                struct sigaction sa;
                memset(&sa, 0, sizeof sa);
                sa.sa_sigaction = on_trap;
                sa.sa_flags = SA_SIGINFO;
                sigaction(SIGTRAP, &sa, 0);
                for (i = 0; i < sizeof patch / sizeof patch[0]; i++) *(volatile uint8_t *)(uintptr_t)patch[i] = 0xcc;
                run_native(&cases[k]);
            } else {
                run_lifted(&cases[k]);
            }
            printf("none\n");
            _exit(0);
        }
        waitpid(p, &st, 0);
        if (!WIFEXITED(st) || WEXITSTATUS(st) != 0) printf("child-status-%d\n", st);
    }
    return 0;
}
"#;

/// Builds the real-DLL harness once. Returns (exe, DLL path).
fn real_exe(dll: &Path, cases: &[Kase]) -> PathBuf {
    static ONCE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = out_dir();
        let data = std::fs::read(dll).unwrap();
        // the three switch functions are not in the default closure: add them as seeds
        let seeds = [0x5e5860u64, 0x5e8da0, 0x5f05f0];
        let mut out = Vec::new();
        let st = generate_with_specs(&data, &seeds, fe_decode::jumptab::FLENGINE, &mut out).expect("generate");
        assert!(st.jumptab_rejected.is_empty(), "{:?}", st.jumptab_rejected);
        assert_eq!(st.jumptabs, 4);
        let c = String::from_utf8(out).unwrap();
        let prelude = &c[..c.find("void f_").unwrap()];

        let mut probes: std::collections::BTreeMap<u32, std::collections::BTreeSet<u64>> = Default::default();
        for k in cases {
            probes.entry(k.f).or_default().insert(k.want);
        }
        let mut body = String::new();
        let mut defined = std::collections::BTreeSet::new();
        let mut patch = std::collections::BTreeSet::new();
        for f in &FUNS {
            let mut t = cut(&c, f.va);
            defined.insert(format!("f_{:x}", f.va));
            for &p in &probes[&f.id] {
                let lab = format!("L_{p:x}: {{ ");
                assert_eq!(t.matches(&lab).count(), 1, "label for {p:#x} in f_{:x}", f.va);
                t = t.replacen(&lab, &format!("{lab}fle_hit(UINT64_C({p:#x})); "), 1);
                patch.insert(p);
            }
            if f.blk != 0 {
                let head = format!("void f_{:x}(void)\n{{\n", f.va);
                t = t.replacen(&head, &format!("{head}goto L_{:x};\n", f.blk), 1);
            }
            body.push_str(&t);
            body.push('\n');
        }
        let mut stubs = String::from("void fle_stub_abort(const char *);\nvoid fle_hit(uint64_t);\n");
        for s in idents(&body, "f_").into_iter().filter(|s| s[2..].chars().all(|ch| ch.is_ascii_hexdigit())) {
            if !defined.contains(&s) {
                stubs.push_str(&format!("void {s}(void) {{ fle_stub_abort(\"{s}\"); }}\n"));
            }
        }
        for s in idents(&body, "fle_imp_") {
            stubs.push_str(&format!("void {s}(void) {{ fle_stub_abort(\"{s}\"); }}\n"));
        }
        for f in &FUNS {
            stubs.push_str(&format!("void f_{:x}(void);\n", f.va));
        }
        let lifted = dir.join(format!("{}.c", etxt_common::per_process("real_lift")));
        std::fs::write(&lifted, format!("{prelude}{stubs}\n{body}")).unwrap();

        let mut tab = String::new();
        for k in cases {
            tab.push_str(&format!(
                "    {{ {}, {:#x}ull, {:#x}ull, {:#x}ull, {:#x}ull, {:#x}ull }},\n",
                k.f, k.blk, k.rcx, k.rdx, k.r8, k.r15
            ));
        }
        let pl: Vec<String> = patch.iter().map(|p| format!("{p:#x}ull")).collect();
        let main_c = REAL_MAIN.replace("@CASES@", &tab).replace("@PATCH@", &pl.join(", "));
        let m = dir.join(format!("{}.c", etxt_common::per_process("real_main")));
        std::fs::write(&m, main_c).unwrap();
        let exe = dir.join(etxt_common::per_process("real_exe"));
        clang(
            &[
                "-o",
                exe.to_str().unwrap(),
                lifted.to_str().unwrap(),
                m.to_str().unwrap(),
                crate_dir().join("../../harness/image.c").to_str().unwrap(),
                crate_dir().join("../../runtime/fle_string.c").to_str().unwrap(),
            ],
            &dir,
        );
        exe
    })
    .clone()
}

fn results(exe: &Path, mode: &str, dll: &Path) -> Vec<String> {
    let r = Command::new(exe).arg(mode).arg(dll).output().expect("run harness");
    assert!(r.status.success(), "{mode}: {}{}", String::from_utf8_lossy(&r.stdout), String::from_utf8_lossy(&r.stderr));
    let s = String::from_utf8_lossy(&r.stdout).to_string();
    s.lines().map(String::from).collect()
}

#[test]
fn real_dll_every_index_reaches_the_table_target_natively_and_lifted() {
    let Some(dll) = real_dll_path() else { return };
    let data = std::fs::read(&dll).unwrap();
    let img = Img(&data);
    let cases = real_cases(&img);
    assert_eq!(cases.len(), 37 + 33 + 8 + 8);
    // sanity of the expectation itself: distinct targets per function as read from the file
    let a: std::collections::BTreeSet<u64> = (0..35).map(|i| img.rel32(0x5e6d44, i)).collect();
    let b: std::collections::BTreeSet<u64> = (0..31).map(|i| img.rel32(0x5eb358, i)).collect();
    let c: std::collections::BTreeSet<u64> = (0..6).map(|i| img.rel32(0x5f1664, i)).collect();
    assert_eq!((a.len(), b.len(), c.len()), (5, 31, 6));
    let exe = real_exe(&dll, &cases);
    let nat = results(&exe, "native", &dll);
    let lif = results(&exe, "lifted", &dll);
    assert_eq!(nat.len(), cases.len(), "{nat:?}");
    assert_eq!(lif.len(), cases.len(), "{lif:?}");
    let mut bad = Vec::new();
    for (k, kc) in cases.iter().enumerate() {
        let want = format!("{k} {:x}", kc.want);
        if nat[k] != want {
            bad.push(format!("native case {k} (fn {} want {:#x}): {}", kc.f, kc.want, nat[k]));
        }
        if lif[k] != want {
            bad.push(format!("lifted case {k} (fn {} want {:#x}): {}", kc.f, kc.want, lif[k]));
        }
    }
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn real_dll_f_40d210_is_a_memset_native_and_lifted() {
    let Some(dll) = real_dll_path() else { return };
    let data = std::fs::read(&dll).unwrap();
    let img = Img(&data);
    let cases = real_cases(&img);
    let exe = real_exe(&dll, &cases);
    let r = results(&exe, "mem", &dll);
    // n in 0..=0x60 (table sizes 1..8, 0, the 9..0x20 loop, the SSE path above 0x20), 6 offsets, 4 values
    assert_eq!(r.last().unwrap(), "mem cases=2328 mismatches=0", "{r:?}");
}
