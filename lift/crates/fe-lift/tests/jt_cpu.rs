//! Selector stubs as a `switch` (decision 0079): `movsxd I,[rip+sel]; lea B,[rip+table]; jmp [B+I*8]`.
//!
//! 1. Synthetic PE (the one of fe-decode/tests/jt_cpu.rs): S1 (table A in .data, 4 entries) and S2 (table B in .text,
//!    3 entries T2,T0,T2). The generated C is built by clang -O0 with harness/image.c (the image is mapped at its
//!    ImageBase), so the native code runs on the host too (ms_abi call). For every selector value in the table, the
//!    native result and the lifted result are equal and are the one the table bytes give. A selector outside the table
//!    (-1, the entry after the table, large values) stops the lifted code in `fle_jumptab_miss`, before anything runs.
//! 2. Real lift/out-ext/dsp_ippv2_x64.dll ($DSP_IPPV2_PE; skipped when absent, as call_iat_dll.rs; if the variable is
//!    set and the file is missing the test fails): three real stubs (0x180001850: 6 entries in .data, 0x180001930:
//!    7 entries in .text, 0x181056520: 9 entries in .data), cut out of the generated C. The selector variable is
//!    written in the mapped image, every target is replaced by a probe. For selector k the lifted code and the real
//!    instructions (0xCC planted at the targets, SIGTRAP handler) reach table[k] (read from the file here, from the
//!    `movsxd`/`lea` bytes of the stub, not from the generator). A selector outside 0..N-1 stops by name, except -1
//!    (the value the variable has in the file, before the initialisation): since decision 0114 it takes the slot in front of the table, to
//!    the stub's own `call <init>`, and the real instructions and the lifted code both reach it (-2 stops by name).
//!    The selector being 0 at run time is not shown here (nothing is run through DllMain).
//!
//! Outputs go under lift/out-w3d/test.

mod etxt_common;

use fe_lift::generate_with_specs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-w3d/test");
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
const OPT_SIZE: usize = 240;
const SEC_TABLE: usize = LFANEW + 24 + OPT_SIZE;
const FILE_LEN: usize = 0xc00;

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

/// Table A (.data 0x2010): T0..T3. Table B (.text 0x1060): T2, T0, T2 (then T3 without a relocation).
/// Target Tk at 0x1100+0x10k: `mov eax, 0x500+k; ret`.
fn build() -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 3);
    w16(&mut b, f + 16, OPT_SIZE as u16);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, 0x1200);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x4000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    let section = |b: &mut Vec<u8>, i: usize, name: &[u8], rva: u32, rawsz: u32, raw: u32, flags: u32| {
        let s = SEC_TABLE + 40 * i;
        b[s..s + name.len()].copy_from_slice(name);
        w32(b, s + 8, 0x400);
        w32(b, s + 12, rva);
        w32(b, s + 16, rawsz);
        w32(b, s + 20, raw);
        w32(b, s + 36, flags);
    };
    section(&mut b, 0, b".text", 0x1000, 0x400, 0x400, 0x6000_0020);
    section(&mut b, 1, b".data", 0x2000, 0x200, 0x800, 0xc000_0040);
    section(&mut b, 2, b".reloc", 0x3000, 0x200, 0xa00, 0x4200_0040);

    let text = |b: &mut Vec<u8>, rva: u32, bytes: &[u8]| {
        let o = 0x400 + (rva - 0x1000) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    };
    text(&mut b, 0x1000, &[&[0x48, 0x63, 0x05][..], &rel(0x1007, 0x2000)].concat()); // movsxd rax,[selA]
    text(&mut b, 0x1007, &[&[0x4c, 0x8d, 0x15][..], &rel(0x100e, 0x2010)].concat()); // lea r10,[tabA]
    text(&mut b, 0x100e, &[0x41, 0xff, 0x24, 0xc2]); // jmp [r10+rax*8]
    text(&mut b, 0x1020, &[&[0x48, 0x63, 0x0d][..], &rel(0x1027, 0x2004)].concat()); // movsxd rcx,[selB]
    text(&mut b, 0x1027, &[&[0x48, 0x8d, 0x15][..], &rel(0x102e, 0x1060)].concat()); // lea rdx,[tabB]
    text(&mut b, 0x102e, &[0xff, 0x24, 0xca]); // jmp [rdx+rcx*8]
    for (k, t) in [0x1120u64, 0x1100, 0x1120, 0x1130].into_iter().enumerate() {
        text(&mut b, 0x1060 + 8 * k as u32, &(IB + t).to_le_bytes());
    }
    for k in 0..4u32 {
        text(&mut b, 0x1100 + 0x10 * k, &[0xb8, k as u8, 0x05, 0, 0, 0xc3]);
    }
    text(&mut b, 0x1200, &[&[0xe8][..], &rel(0x1205, 0x1000)].concat());
    text(&mut b, 0x1205, &[&[0xe8][..], &rel(0x120a, 0x1020)].concat());
    text(&mut b, 0x120a, &[0xc3]);
    let d = 0x800;
    w32(&mut b, d, 0xffff_ffff);
    w32(&mut b, d + 4, 0xffff_ffff);
    w64(&mut b, d + 8, IB + 0x1200);
    for k in 0..4u64 {
        w64(&mut b, d + 0x10 + 8 * k as usize, IB + 0x1100 + 0x10 * k);
    }
    w64(&mut b, d + 0x30, IB + 0x2000);
    let r = 0xa00;
    w32(&mut b, r, 0x1000);
    w32(&mut b, r + 4, 16);
    for (k, off) in [0x60u16, 0x68, 0x70, 0].into_iter().enumerate() {
        w16(&mut b, r + 8 + 2 * k, if off == 0 { 0 } else { 0xa000 | off });
    }
    w32(&mut b, r + 16, 0x2000);
    w32(&mut b, r + 20, 8 + 2 * 6);
    for (k, off) in [0x08u16, 0x10, 0x18, 0x20, 0x28, 0x30].into_iter().enumerate() {
        w16(&mut b, r + 24 + 2 * k, 0xa000 | off);
    }
    w32(&mut b, o + 112 + 8 * 5, 0x3000);
    w32(&mut b, o + 116 + 8 * 5, 16 + 8 + 12);
    b
}

const SYN_MAIN: &str = r#"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "boot.h"
extern uint64_t R[16];
void f_140001000(void);
void f_140001020(void);
typedef uint32_t (__attribute__((ms_abi)) *nfn)(void);

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
        int s2 = !strcmp(argv[2], "s2");
        *(volatile int32_t *)(uintptr_t)(0x140002000 + (s2 ? 4 : 0)) = (int32_t)i;
        if (argc < 6) nat = ((nfn)(uintptr_t)(s2 ? 0x140001020 : 0x140001000))();
        memset(R, 0, sizeof R);
        if (s2) f_140001020(); else f_140001000();
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
        generate_with_specs(&img, &[], &[], &mut out).expect("generate");
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

fn rows(o: &std::process::Output) -> Vec<Vec<String>> {
    String::from_utf8_lossy(&o.stdout).lines().map(|l| l.split(' ').map(String::from).collect()).collect()
}

#[test]
fn synthetic_generator_output() {
    let mut out = Vec::new();
    let st = generate_with_specs(&build(), &[], &[], &mut out).expect("generate");
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.jumptabs, 2);
    assert!(st.jumptab_rejected.is_empty(), "{:?}", st.jumptab_rejected);
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
    assert_eq!(st.handled["jmp_ind:sel_table"], 2);
    assert!(!st.handled.contains_key("jmp_ind:table"));
    // the memory-form stubs are not register jumps
    assert_eq!(st.register_jmp_sites, 0);
    // table A: slot addresses 0x2010..0x2028 are the cases; entry -1 (0x2008) and the slot after (0x2030) are not
    for k in 0..4u64 {
        assert!(c.contains(&format!("case UINT64_C({:#x}): ", IB + 0x2010 + 8 * k)), "slot A{k}");
    }
    assert!(!c.contains(&format!("case UINT64_C({:#x}): ", IB + 0x2008)), "entry -1 must not be a case");
    assert!(!c.contains(&format!("case UINT64_C({:#x}): ", IB + 0x2030)), "the slot after the table must not be a case");
    // table B: three slots (T3 is not relocated)
    for k in 0..3u64 {
        assert!(c.contains(&format!("case UINT64_C({:#x}): ", IB + 0x1060 + 8 * k)), "slot B{k}");
    }
    assert!(!c.contains(&format!("case UINT64_C({:#x}): ", IB + 0x1078)), "unrelocated slot after table B");
    assert_eq!(c.matches("fle_jumptab_miss(UINT64_C(0x14000100e), t)").count(), 1);
    assert_eq!(c.matches("fle_jumptab_miss(UINT64_C(0x14000102e), t)").count(), 1);
    // every table entry is a function of the closure
    for k in 0..4u64 {
        assert!(c.contains(&format!("void f_{:x}(void)\n{{\n", IB + 0x1100 + 0x10 * k)), "T{k} is a function");
    }
}

#[test]
fn synthetic_every_selector_matches_native_and_the_table() {
    let (exe, pe) = syn_exe();
    // S1: selector k -> Tk
    let r = Command::new(&exe).arg(&pe).args(["s1", "0", "3"]).output().unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let l = rows(&r);
    assert_eq!(l.len(), 4);
    for (k, row) in l.iter().enumerate() {
        let want = format!("{:x}", 0x500 + k);
        assert_eq!(row[0], format!("{k}"));
        assert_eq!(row[1], want, "native, selector {k}");
        assert_eq!(row[2], want, "lifted, selector {k}");
    }
    // S2: selector k -> T2, T0, T2
    let r = Command::new(&exe).arg(&pe).args(["s2", "0", "2"]).output().unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let l = rows(&r);
    assert_eq!(l.len(), 3);
    for (k, row) in l.iter().enumerate() {
        let want = format!("{:x}", 0x500 + [2, 0, 2][k]);
        assert_eq!(row[1], want, "native, selector {k}");
        assert_eq!(row[2], want, "lifted, selector {k}");
    }
}

#[test]
fn synthetic_selector_outside_the_table_stops_by_name() {
    let (exe, pe) = syn_exe();
    // lifted only (a sixth argument): the native code would jump to data or to the initialisation call
    for (s, sel) in [("s1", "-1"), ("s1", "4"), ("s1", "5"), ("s1", "100"), ("s2", "-1"), ("s2", "3"), ("s2", "100")] {
        let r = Command::new(&exe).arg(&pe).args([s, sel, sel, "lifted-only"]).output().unwrap();
        assert!(!r.status.success(), "{s} {sel} did not stop: {}", String::from_utf8_lossy(&r.stdout));
        let e = String::from_utf8_lossy(&r.stderr).to_string();
        assert!(e.contains("outside the table"), "{s} {sel}: {e}");
        let jmp = if s == "s1" { "0x14000100e" } else { "0x14000102e" };
        assert!(e.contains(&format!("jump table at {jmp}")), "{s} {sel}: {e}");
        assert!(String::from_utf8_lossy(&r.stdout).is_empty(), "{s} {sel} ran on");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. Real dsp_ippv2_x64.dll
// ---------------------------------------------------------------------------------------------

fn real_dll_path() -> Option<PathBuf> {
    let explicit = std::env::var_os("DSP_IPPV2_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => crate_dir().join("../../out-ext/dsp_ippv2_x64.dll"),
    };
    match std::fs::metadata(&path) {
        Ok(_) => Some(path),
        Err(e) if explicit.is_some() => panic!("DSP_IPPV2_PE={}: {e}", path.display()),
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
    /// `base + disp32` of a 7-byte `[rip+disp32]` instruction at `va`, with `opcode` as its first three bytes.
    fn rip_target(&self, va: u64, opcode: [u8; 3]) -> u64 {
        let b = self.rd(va, 7);
        assert_eq!(&b[..3], &opcode, "instruction at {va:#x}");
        (va + 7).wrapping_add(i32::from_le_bytes(b[3..7].try_into().unwrap()) as i64 as u64)
    }
}

/// (stub VA, expected selector variable, expected table, entries). The selector variable and the table are
/// also read from the stub's bytes and must equal these.
const STUBS: [(u64, u64, u64, usize); 3] = [
    (0x1_8000_1850, 0x1_8254_1000, 0x1_8254_1028, 6),
    (0x1_8000_1930, 0x1_8254_10a0, 0x1_8000_18e8, 7),
    (0x1_8105_6520, 0x1_8254_1000, 0x1_8254_b068, 9),
];

struct RealStub {
    va: u64,
    sel_var: u64,
    targets: Vec<u64>,
    /// decision 0114: the slot in front of the table (index -1, the value 0xffffffff of the variable in the file) holds this, read from the
    /// file: the stub's own lazy-initialisation `call` 16 bytes in front of it
    init: u64,
}

fn real_stubs(img: &Img<'_>) -> Vec<RealStub> {
    STUBS
        .iter()
        .map(|&(va, sel_var, table, n)| {
            assert_eq!(img.rip_target(va, [0x48, 0x63, 0x05]), sel_var, "movsxd of {va:#x}");
            assert_eq!(img.rip_target(va + 7, [0x4c, 0x8d, 0x15]), table, "lea of {va:#x}");
            assert_eq!(img.rd(va + 14, 4), &[0x41, 0xff, 0x24, 0xc2], "jmp [r10+rax*8] of {va:#x}");
            let init = img.abs64(table - 8);
            assert_eq!(init, va - 16, "slot -1 of {va:#x} holds the address 16 bytes in front of the stub");
            assert_eq!(img.rd(init, 1), &[0xe8], "a call rel32 at {init:#x}");
            assert_eq!(img.rd(init + 5, 11), &[0x66, 0x66, 0x66, 0x0f, 0x1f, 0x84, 0, 0, 0, 0, 0], "an 11-byte nop at {:#x}", init + 5);
            RealStub { va, sel_var, targets: (0..n as u64).map(|k| img.abs64(table + 8 * k)).collect(), init }
        })
        .collect()
}

fn cut(c: &str, f: u64) -> String {
    let head = format!("void f_{f:x}(void)\n{{\n");
    let i = c.find(&head).unwrap_or_else(|| panic!("no definition of f_{f:x}"));
    let rest = &c[i..];
    let e = rest.find("\n}\n\n").unwrap() + 3;
    rest[..e].to_string()
}

const REAL_MAIN: &str = r#"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <ucontext.h>
#include "boot.h"
#include "fle_dispatch.h"

extern uint64_t R[16];
const fle_fn_entry fle_dispatch_table[] = { { 0, 0 } };
const uint64_t fle_dispatch_count = 0;
void fle_hit(uint64_t va) { printf("%llx\n", (unsigned long long)va); fflush(stdout); _exit(0); }
@DECLS@
static const uint64_t stub[] = { @STUBS@ };
static const uint64_t selvar[] = { @SELVARS@ };
static const uint64_t patch[] = { @PATCH@ };

static void on_trap(int sig, siginfo_t *si, void *u)
{
    ucontext_t *uc = u;
    (void)sig; (void)si;
    printf("%llx\n", (unsigned long long)(uc->uc_mcontext.gregs[REG_RIP] - 1));
    fflush(stdout);
    _exit(0);
}

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    size_t i, k;
    int32_t sel;
    if (argc < 5) return 2;
    if (image_load(argv[2], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    k = (size_t)strtoul(argv[3], 0, 0);
    sel = (int32_t)strtol(argv[4], 0, 0);
    *(volatile int32_t *)(uintptr_t)selvar[k] = sel;
    if (!strcmp(argv[1], "native")) {
        struct sigaction sa;
        memset(&sa, 0, sizeof sa);
        sa.sa_sigaction = on_trap;
        sa.sa_flags = SA_SIGINFO;
        sigaction(SIGTRAP, &sa, 0);
        for (i = 0; i < sizeof patch / sizeof patch[0]; i++) *(volatile uint8_t *)(uintptr_t)patch[i] = 0xcc;
        ((void (__attribute__((ms_abi)) *)(void))(uintptr_t)stub[k])();
    } else {
        memset(R, 0, sizeof R);
@CALLS@
    }
    printf("none\n");
    return 0;
}
"#;

fn real_exe(dll: &Path, stubs: &[RealStub]) -> PathBuf {
    static ONCE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = out_dir();
        let data = std::fs::read(dll).unwrap();
        let seeds: Vec<u64> = stubs.iter().map(|s| s.va).collect();
        let mut out = Vec::new();
        let st = generate_with_specs(&data, &seeds, &[], &mut out).expect("generate");
        assert!(st.jumptab_rejected.is_empty(), "{:?}", st.jumptab_rejected);
        // the 434 stubs, and the 4 MSVC switches of the closure of these seeds: 3 that the reaching definitions of decision 0094
        // resolve (0x37cc80b, 0x37d9670, 0x37d986a; none before it) and the swapped form of decision 0095 (0x37cd3eb)
        assert_eq!((st.jumptabs, st.msvc_tables), (438, 4), "{:?}", st.msvc_rejected);
        let c = String::from_utf8(out).unwrap();
        let prelude = &c[..c.find("void f_").unwrap()];

        let mut body = String::new();
        let mut all_targets = std::collections::BTreeSet::new();
        for s in stubs {
            let t = cut(&c, s.va);
            let cases = t.matches("case UINT64_C(").count();
            let distinct_slots = s.targets.len() + 1;
            assert_eq!(cases, distinct_slots, "one case per table slot, and the index -1 (decision 0114), in f_{:x}", s.va);
            assert_eq!(t.matches("fle_jumptab_miss(").count(), 1);
            assert!(t.contains(&format!("f_{:x}();", s.init)), "f_{:x} calls the init path {:#x}", s.va, s.init);
            all_targets.insert(s.init);
            for tg in &s.targets {
                assert!(t.contains(&format!("f_{tg:x}();")), "f_{:x} calls the target {tg:#x}", s.va);
                all_targets.insert(*tg);
            }
            body.push_str(&t);
            body.push('\n');
        }
        let mut decls = String::new();
        for s in stubs {
            decls.push_str(&format!("void f_{:x}(void);\n", s.va));
        }
        for tg in &all_targets {
            decls.push_str(&format!("void f_{tg:x}(void) {{ fle_hit(UINT64_C({tg:#x})); }}\n"));
        }
        let lifted = dir.join(format!("{}.c", etxt_common::per_process("real_lift")));
        std::fs::write(&lifted, format!("{prelude}void fle_hit(uint64_t);\n{decls}\n{body}")).unwrap();

        let join = |f: &dyn Fn(&RealStub) -> String| stubs.iter().map(f).collect::<Vec<_>>().join(", ");
        let calls: String = stubs
            .iter()
            .enumerate()
            .map(|(k, s)| format!("        if (k == {k}) f_{:x}();\n", s.va))
            .collect();
        let patch: Vec<String> = all_targets.iter().map(|p| format!("{p:#x}ull")).collect();
        let main_c = REAL_MAIN
            .replace("@DECLS@", "")
            .replace("@STUBS@", &join(&|s| format!("{:#x}ull", s.va)))
            .replace("@SELVARS@", &join(&|s| format!("{:#x}ull", s.sel_var)))
            .replace("@PATCH@", &patch.join(", "))
            .replace("@CALLS@", &calls);
        // the stubs' declarations must precede main
        let main_c = main_c.replacen("static const uint64_t stub[]", &format!("{}static const uint64_t stub[]", decl_stubs(stubs)), 1);
        let m = dir.join(format!("{}.c", etxt_common::per_process("real_main")));
        std::fs::write(&m, main_c).unwrap();
        let exe = dir.join(etxt_common::per_process("real_exe"));
        clang(
            &["-o", exe.to_str().unwrap(), lifted.to_str().unwrap(), m.to_str().unwrap(), crate_dir().join("../../harness/image.c").to_str().unwrap()],
            &dir,
        );
        exe
    })
    .clone()
}

fn decl_stubs(stubs: &[RealStub]) -> String {
    stubs.iter().map(|s| format!("void f_{:x}(void);\n", s.va)).collect()
}

fn run(exe: &Path, mode: &str, dll: &Path, k: usize, sel: i64) -> std::process::Output {
    Command::new(exe).arg(mode).arg(dll).arg(k.to_string()).arg(sel.to_string()).output().expect("run harness")
}

#[test]
fn real_stub_selector_k_reaches_table_k_natively_and_lifted() {
    let Some(dll) = real_dll_path() else { return };
    let data = std::fs::read(&dll).unwrap();
    let stubs = real_stubs(&Img(&data));
    // the 7-entry .text table has distinct entries, the 9-entry .data table too
    for (k, n) in [(1usize, 7usize), (2, 9)] {
        let d: std::collections::BTreeSet<u64> = stubs[k].targets.iter().copied().collect();
        assert_eq!(d.len(), n, "stub {k}: entries distinct");
    }
    assert_eq!(stubs[0].targets[0], 0x1_8000_18a0, "selector 0 of the first stub: the entry the file holds");
    assert_eq!(stubs[1].targets[0], 0x1_8000_1a30);
    let exe = real_exe(&dll, &stubs);
    let mut n = 0;
    for (k, s) in stubs.iter().enumerate() {
        for (sel, want) in s.targets.iter().enumerate() {
            let want = format!("{want:x}\n");
            for mode in ["native", "lifted"] {
                let r = run(&exe, mode, &dll, k, sel as i64);
                assert!(r.status.success(), "{mode} stub {k:#x} sel {sel}: {}", String::from_utf8_lossy(&r.stderr));
                assert_eq!(String::from_utf8_lossy(&r.stdout), want, "{mode} stub {:#x} sel {sel}", s.va);
                n += 1;
            }
        }
    }
    assert_eq!(n, 2 * (6 + 7 + 9));
    // decision 0114: the selector -1 (the variable in the file, before its initialisation) takes the slot in front of the table, to the
    // stub's `call <init>`: the real instructions and the lifted code both reach S - 16 (probes at every target, as above)
    for (k, s) in stubs.iter().enumerate() {
        let want = format!("{:x}\n", s.init);
        for mode in ["native", "lifted"] {
            let r = run(&exe, mode, &dll, k, -1);
            assert!(r.status.success(), "{mode} stub {k:#x} sel -1: {}", String::from_utf8_lossy(&r.stderr));
            assert_eq!(String::from_utf8_lossy(&r.stdout), want, "{mode} stub {:#x} sel -1", s.va);
        }
    }
}

#[test]
fn real_stub_selector_outside_the_table_stops_by_name() {
    let Some(dll) = real_dll_path() else { return };
    let data = std::fs::read(&dll).unwrap();
    let stubs = real_stubs(&Img(&data));
    let exe = real_exe(&dll, &stubs);
    for (k, s) in stubs.iter().enumerate() {
        let n = s.targets.len() as i64;
        // -2: the slot before the init slot (-1 is a case since decision 0114); n: the slot after the table
        for sel in [-2, n, n + 1, 100, i32::MIN as i64] {
            let r = run(&exe, "lifted", &dll, k, sel);
            assert!(!r.status.success(), "stub {:#x} sel {sel} did not stop: {}", s.va, String::from_utf8_lossy(&r.stdout));
            let e = String::from_utf8_lossy(&r.stderr).to_string();
            assert!(e.contains("outside the table"), "stub {:#x} sel {sel}: {e}", s.va);
            assert!(e.contains(&format!("jump table at {:#x}", s.va + 14)), "stub {:#x} sel {sel}: {e}", s.va);
            assert!(String::from_utf8_lossy(&r.stdout).is_empty(), "stub {:#x} sel {sel} ran on", s.va);
        }
    }
}
