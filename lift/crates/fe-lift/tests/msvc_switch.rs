//! MSVC switch tables with a proven base as a `switch` (decisions 0089 and 0090; fe-decode/src/jumptab.rs,
//! fe-lift/src/jumptab.rs).
//!
//! 1. Synthetic PE (`fe-decode/tests/msvc_common`): (c1) and (c2). The generated C is built with clang -O0 together with
//!    harness/image.c (the image is mapped at its ImageBase), so the same function also runs natively on the host (ms_abi
//!    call). For every index 0..=N+3 the lifted result equals the native one and the one the table bytes give; an index
//!    above N takes the guard's `ja` in both (the original code, not `fle_jumptab_miss`). A register value that is not a
//!    table target (the idiom entered past the guard with a wrong base) stops in `fle_jumptab_miss`. The `lea` is in the
//!    entry block, after a `je` to the next instruction, or after a diamond (decision 0090: dominance).
//!    Sites that are not resolved (the `lea` on one path only, another indirect jump in the function, ...) stay
//!    `fle_dispatch(R[x]); return;` in the generated C.
//! 2. Real Blend2D (rebased 0x4700000; $BLEND2D_PE, else lift/out-seed58/s58/dll/Blend2D.4700000.dll; skipped when
//!    absent; if BLEND2D_PE is set and the file is missing the test fails), `f_4709ea0`, site 0x470a89e (original
//!    0x18000a89e): `movzx eax,byte [rax+r13+0xab8c]; mov ecx,[r13+rax*4+0xab74]; add rcx,r13; jmp rcx`.
//!    - With the default base proof (decision 0090: dominance) the site is resolved (`lea r13,[ImageBase]` is at 0x4709f18,
//!      outside the entry block, on both paths of the `jne` at 0x4709efa; the function has no other indirect jump). Entered
//!      at 0x470a881 (the `cmp eax,0x1f`), every index 0..=32 and some larger ones reach the address that the 32 index
//!      bytes and the 6 RVAs of the file give (read here, from the file, not from the generator), natively (real
//!      instructions, 0xCC at the 6 targets, SIGTRAP handler) and lifted (hit probe at each target label).
//!
//! Outputs go under lift/out-jtb2d90/test.

mod etxt_common;
#[path = "../../fe-decode/tests/msvc_common/mod.rs"]
mod msvc_common;

use fe_lift::{generate_with_options, generate_with_specs};
use msvc_common::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-jtb2d90/test");
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

const AT: u32 = 0x1100;

fn fva() -> u64 {
    IB + AT as u64
}

fn gen(o: &Sw) -> (Vec<u8>, Built, String, fe_lift::Stats) {
    let built = build_switch(AT, o);
    let img = built.img.build();
    let mut out = Vec::new();
    let st = generate_with_specs(&img, &[], &[], &mut out).expect("generate");
    (img, built, String::from_utf8(out).unwrap(), st)
}

const SYN_MAIN: &str = r#"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "boot.h"
extern uint64_t R[16];
void @FN@(void);
void @PAST@(void);
typedef uint32_t (__attribute__((ms_abi)) *nfn)(int64_t);
static uint8_t stk[0x1000];

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    long lo, hi, i;
    if (argc < 5) return 2;
    if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    lo = strtol(argv[2], 0, 0);
    hi = strtol(argv[3], 0, 0);
    for (i = lo; i <= hi; i++) {
        uint32_t nat = 0, lif;
        memset(R, 0, sizeof R);
        R[4] = (uint64_t)(uintptr_t)(stk + 0x800);
        if (!strcmp(argv[4], "bad-base")) {
            /* enter past the guard (the copy of the function that starts at the idiom) with a base register that is
               not the image base: the jump register is then not a table target */
            R[0] = (uint64_t)i;
            R[@B@] = 0x140000000ull + 8;
            @PAST@();
        } else {
            nat = ((nfn)(uintptr_t)@VA@)(i);
            memset(R, 0, sizeof R);
            R[4] = (uint64_t)(uintptr_t)(stk + 0x800);
            R[1] = (uint64_t)i;
            @FN@();
        }
        lif = (uint32_t)R[0];
        printf("%ld %x %x\n", i, nat, lif);
        fflush(stdout);
    }
    return 0;
}
"#;

struct Exe {
    exe: PathBuf,
    pe: PathBuf,
}

fn syn_exe(o: &Sw, tag: &str) -> Exe {
    let dir = out_dir();
    let (img, b, c, st) = gen(o);
    assert_eq!(st.msvc_tables, 1, "{o:?}: {:?}", st.msvc_rejected);
    let pe = dir.join(format!("{}.pe", etxt_common::uniq(&format!("syn-{tag}"))));
    std::fs::write(&pe, &img).unwrap();
    let fname = format!("f_{:x}", fva());
    let past = format!("{fname}_past_guard");
    // a copy of the function that starts at the idiom (past the guard `cmp; ja`)
    let idiom = IB + b.labels["idiom"] as u64;
    let head = format!("void {fname}(void)\n{{\n");
    let body = &c[c.find(&head).expect("function")..];
    let func = &body[..body.find("\n}\n\n").unwrap() + 3];
    let copy = func.replacen(&head, &format!("void {past}(void)\n{{\ngoto L_{idiom:x};\n"), 1);
    let lifted = dir.join(format!("{}.c", etxt_common::uniq(&format!("syn-lift-{tag}"))));
    std::fs::write(&lifted, format!("{c}\nvoid {past}(void);\n{copy}")).unwrap();
    let main_c = SYN_MAIN
        .replace("@FN@", &fname)
        .replace("@PAST@", &past)
        .replace("@VA@", &format!("{:#x}ull", fva()))
        .replace("@B@", &format!("{}", o.base));
    let m = dir.join(format!("{}.c", etxt_common::uniq(&format!("syn-main-{tag}"))));
    std::fs::write(&m, main_c).unwrap();
    let exe = dir.join(etxt_common::uniq(&format!("syn-exe-{tag}")));
    clang(
        &["-o", exe.to_str().unwrap(), lifted.to_str().unwrap(), m.to_str().unwrap(), crate_dir().join("../../harness/image.c").to_str().unwrap()],
        &dir,
    );
    Exe { exe, pe }
}

fn rows(e: &Exe, lo: i64, hi: i64, mode: &str) -> std::process::Output {
    Command::new(&e.exe).arg(&e.pe).arg(lo.to_string()).arg(hi.to_string()).arg(mode).output().unwrap()
}

fn parse(o: &std::process::Output) -> Vec<(i64, String, String)> {
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .map(|l| {
            let p: Vec<&str> = l.split(' ').collect();
            (p[0].parse().unwrap(), p[1].to_string(), p[2].to_string())
        })
        .collect()
}

/// The value the function returns for index `i` as the table bytes give it: block k returns 0x100 + k, the guard's
/// default returns 0xdead.
fn expected(b: &Built, o: &Sw, i: i64) -> u32 {
    // the index the table sees is `(i - K) as u32` (`sub eax,K`); above N the guard's `ja` takes the default
    let j = (i as u32).wrapping_sub(o.sub_k as u32) as u64;
    if j > o.n as u64 {
        return 0xdead;
    }
    let target_rva = if o.two_stage { b.table[b.byte_table[j as usize] as usize] } else { b.table[j as usize] };
    // blocks t0, t1, t2 return 0x100 + k; the table may also hold the bad entry (another function / chained piece)
    for (k, l) in ["t0", "t1", "t2"].iter().enumerate() {
        if b.labels[l] == target_rva {
            return 0x100 + k as u32;
        }
    }
    unreachable!("target {target_rva:#x} is no block")
}

fn every_index_matches_native_and_the_table(o: &Sw, tag: &str) {
    let (img, b, _, _) = gen(o);
    let _ = img;
    let e = syn_exe(o, tag);
    let hi = o.n as i64 + o.sub_k as i64 + 3;
    let r = rows(&e, 0, hi, "both");
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let l = parse(&r);
    assert_eq!(l.len() as i64, hi + 1);
    for (i, nat, lif) in l {
        let want = format!("{:x}", expected(&b, o, i));
        assert_eq!(nat, want, "native, idx {i}, {o:?}");
        assert_eq!(lif, want, "lifted, idx {i}, {o:?}");
    }
}

#[test]
fn c1_every_index_matches_native_and_the_table() {
    every_index_matches_native_and_the_table(&Sw::c1(), "c1");
}

#[test]
fn c2_every_index_matches_native_and_the_table() {
    every_index_matches_native_and_the_table(&Sw::c2(), "c2");
}

#[test]
fn c1_with_cdqe_and_a_64_bit_guard_and_a_call() {
    every_index_matches_native_and_the_table(&Sw { cdqe: true, guard: Guard::Rax, call: true, ..Sw::c1() }, "c1x");
    every_index_matches_native_and_the_table(&Sw { cdqe: true, guard: Guard::Rax, ..Sw::c2() }, "c2x");
}

#[test]
fn a_lea_outside_the_entry_block_on_every_path_every_index_matches_native_and_the_table() {
    // decision 0090: dominance. `test; je next; lea B` and a diamond before the `lea` (rcx = the index, set by the harness)
    every_index_matches_native_and_the_table(&Sw { lea: Lea::AfterBranch, ..Sw::c1() }, "c1ab");
    every_index_matches_native_and_the_table(&Sw { lea: Lea::AfterBranch, ..Sw::c2() }, "c2ab");
    every_index_matches_native_and_the_table(&Sw { lea: Lea::AfterDiamond, ..Sw::c1() }, "c1ad");
    every_index_matches_native_and_the_table(&Sw { lea: Lea::AfterDiamond, cdqe: true, guard: Guard::Rax, call: true, ..Sw::c2() }, "c2ad");
    every_index_matches_native_and_the_table(&Sw { other_jmp: OtherJmp::TailCall, lea: Lea::AfterDiamond, ..Sw::c1() }, "c1tc");
}

#[test]
fn sub_k_before_the_guard_every_index_matches_native_and_the_table() {
    every_index_matches_native_and_the_table(&Sw { sub_k: 5, ..Sw::c1() }, "c1s");
    every_index_matches_native_and_the_table(&Sw { sub_k: 5, ..Sw::c2() }, "c2s");
}

#[test]
fn synthetic_generator_output() {
    for o in [Sw::c1(), Sw::c2()] {
        let (_, b, c, st) = gen(&o);
        assert_eq!((st.jumptabs, st.msvc_tables), (1, 1));
        assert!(st.msvc_rejected.is_empty(), "{:?}", st.msvc_rejected);
        assert!(st.jumptab_rejected.is_empty());
        assert_eq!(st.bad_nodes, 0);
        assert_eq!(st.handled["jmp_ind:msvc_table"], 1);
        assert_eq!(st.register_jmps["jmp_ind:msvc_table"], 1, "{:?}", st.register_jmps);
        assert!(!st.register_jmps.contains_key("jmp_ind:reg_dispatch"));
        // one case per distinct target, each a goto to a label of the same function
        for l in ["t0", "t1", "t2"] {
            let va = IB + b.labels[l] as u64;
            assert!(c.contains(&format!("case UINT64_C({va:#x}): goto L_{va:x}; ")), "{l}");
            assert!(c.contains(&format!("L_{va:x}: {{ ")), "label of {l}");
        }
        assert_eq!(c.matches("case UINT64_C(").count(), 3);
        let jmp = IB + b.labels["jmp"] as u64;
        assert_eq!(c.matches(&format!("fle_jumptab_miss(UINT64_C({jmp:#x}), t)")).count(), 1);
        assert!(!c.contains("fle_dispatch(R["), "no dispatch left in the function");
        assert!(c.contains("uint64_t t = R[1]; switch (t)"), "the selector is the jump register");
    }
}

#[test]
fn a_register_value_that_is_not_a_table_target_stops_by_name() {
    // enter past the guard with r12/r13 = ImageBase + 8: the register is then base + RVA + 8, no case matches
    for (o, tag) in [(Sw::c1(), "bb1"), (Sw::c2(), "bb2")] {
        let e = syn_exe(&o, tag);
        let r = rows(&e, 0, 0, "bad-base");
        assert!(!r.status.success(), "{tag} did not stop: {}", String::from_utf8_lossy(&r.stdout));
        let err = String::from_utf8_lossy(&r.stderr).to_string();
        assert!(err.contains("outside the table"), "{tag}: {err}");
        assert!(String::from_utf8_lossy(&r.stdout).is_empty(), "{tag} ran on");
        let (_, b, _, _) = gen(&o);
        assert!(err.contains(&format!("jump table at {:#x}", IB + b.labels["jmp"] as u64)), "{err}");
    }
}

#[test]
fn unresolved_sites_stay_a_tail_dispatch() {
    for o in [
        Sw { lea: Lea::OneSided, ..Sw::c1() },
        // decision 0094: a definition that is not `lea` reaches the jump, or reaches another indirect jump (a wildcard)
        Sw { other_jmp: OtherJmp::ReachedAfterWrite, ..Sw::c1() },
        Sw { other_jmp: OtherJmp::ReachedAfterWriteMem, ..Sw::c2() },
        Sw { other_jmp: OtherJmp::ReachedBeforeLea, ..Sw::c1() },
        Sw { lea: Lea::LeaOrArg, ..Sw::c1() },
        Sw { lea: Lea::Never, ..Sw::c2() },
        Sw { lea: Lea::Twice, ..Sw::c2() },
        Sw { lea: Lea::FromArg, ..Sw::c1() },
        Sw { guard: Guard::Missing, ..Sw::c1() },
        Sw { bad: Bad::OtherFunction, ..Sw::c2() },
        Sw { base: 8, call: true, ..Sw::c1() },
    ] {
        let built = build_switch(AT, &o);
        let mut out = Vec::new();
        let st = generate_with_specs(&built.img.build(), &[], &[], &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        // the function of `ReachedAfterWrite` / `ReachedAfterWriteMem` / `ReachedBeforeLea` has a second indirect jump (`jmp rdx`
        // or `jmp [rdx]`), which the closure reaches too: it is a `jmp reg` that is no table idiom (`Mem`: not a register jump)
        let others = match o.other_jmp {
            OtherJmp::ReachedAfterWrite | OtherJmp::ReachedBeforeLea => 1,
            _ => 0,
        };
        assert_eq!((st.jumptabs, st.msvc_tables), (0, 0), "{o:?}");
        assert_eq!(st.msvc_rejected.len(), 1, "{o:?}");
        assert_eq!(st.handled["jmp_ind:reg_dispatch"], 1 + others, "{o:?}");
        assert!(!st.handled.contains_key("jmp_ind:msvc_table"));
        assert!(c.contains("fle_dispatch(R[1]); return;"), "{o:?}");
        assert!(!c.contains("fle_jumptab_miss"), "{o:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. Real Blend2D, f_4709ea0
// ---------------------------------------------------------------------------------------------

const F: u64 = 0x4709ea0;
const SITE: u64 = 0x470a89e;
const GUARD_CMP: u64 = 0x470a881;
const IBASE: u64 = 0x4700000;

fn blend2d_path() -> Option<PathBuf> {
    let explicit = std::env::var_os("BLEND2D_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => crate_dir().join("../../out-seed58/s58/dll/Blend2D.4700000.dll"),
    };
    match std::fs::metadata(&path) {
        Ok(_) => Some(path),
        Err(e) if explicit.is_some() => panic!("BLEND2D_PE={}: {e}", path.display()),
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
}

/// (tables read from the file through the displacements of the instructions at the site, not from the generator)
struct Tables {
    idx_bytes: Vec<u8>,
    rvas: Vec<u32>,
}

fn read_tables(img: &Img<'_>) -> Tables {
    // 0x470a88a: movzx eax, byte [rax+r13+0xab8c] = 42 0f b6 84 28 8c ab 00 00
    assert_eq!(img.rd(0x470a88a, 9), &[0x42, 0x0f, 0xb6, 0x84, 0x28, 0x8c, 0xab, 0x00, 0x00]);
    // 0x470a893: mov ecx, [r13+rax*4+0xab74] = 41 8b 8c 85 74 ab 00 00
    assert_eq!(img.rd(0x470a893, 8), &[0x41, 0x8b, 0x8c, 0x85, 0x74, 0xab, 0x00, 0x00]);
    // 0x470a89b: add rcx, r13 ; 0x470a89e: jmp rcx
    assert_eq!(img.rd(0x470a89b, 5), &[0x49, 0x03, 0xcd, 0xff, 0xe1]);
    // 0x470a881: cmp eax,0x1f ; ja 0x470aafc
    assert_eq!(img.rd(0x470a881, 3), &[0x83, 0xf8, 0x1f]);
    let idx_bytes = img.rd(IBASE + 0xab8c, 32).to_vec();
    let nrva = *idx_bytes.iter().max().unwrap() as usize + 1;
    let rvas = (0..nrva).map(|k| u32::from_le_bytes(img.rd(IBASE + 0xab74 + 4 * k as u64, 4).try_into().unwrap())).collect();
    Tables { idx_bytes, rvas }
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
void fle_hit(uint64_t va) { printf("%llx\n", (unsigned long long)va); fflush(stdout); _exit(0); }
void f_4709ea0(void);
static const uint64_t idx[] = { @IDX@ };
static const uint64_t patch[] = { @PATCH@ };

static void on_trap(int sig, siginfo_t *si, void *u)
{
    ucontext_t *uc = u;
    (void)sig; (void)si;
    printf("%llx\n", (unsigned long long)(uc->uc_mcontext.gregs[REG_RIP] - 1));
    fflush(stdout);
    _exit(0);
}

static void run_native(uint64_t rax)
{
    struct { uint64_t rax, r13, go; } r;
    r.rax = rax; r.r13 = 0x4700000ull; r.go = 0x470a881ull;
    asm volatile("mov %0, %%r11\n\t"
                 "mov 0(%%r11), %%rax\n\t"
                 "mov 8(%%r11), %%r13\n\t"
                 "jmp *16(%%r11)"
                 : : "r"(&r) : "memory");
}

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    size_t i, k;
    int native;
    if (argc < 3) return 2;
    if (image_load(argv[2], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    native = !strcmp(argv[1], "native");
    for (k = 0; k < sizeof idx / sizeof idx[0]; k++) {
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
                run_native(idx[k]);
            } else {
                memset(R, 0, sizeof R);
                R[0] = idx[k];
                R[13] = 0x4700000ull;
                f_4709ea0();
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

fn cut(c: &str, f: u64) -> String {
    let head = format!("void f_{f:x}(void)\n{{\n");
    let i = c.find(&head).unwrap_or_else(|| panic!("no definition of f_{f:x}"));
    let rest = &c[i..];
    let e = rest.find("\n}\n\n").unwrap() + 3;
    rest[..e].to_string()
}

fn gen_blend2d(data: &[u8]) -> (String, fe_lift::Stats) {
    let opts = fe_decode::Options::default();
    let pe = fe_pe::Pe::parse(data).unwrap();
    let seeds = fe_decode::pdata_seeds(&pe).unwrap();
    let mut out = Vec::new();
    let st = generate_with_options(data, &seeds, fe_decode::jumptab::FLENGINE, &opts, &mut out).expect("generate");
    (String::from_utf8(out).unwrap(), st)
}

#[test]
fn real_blend2d_site_is_resolved_by_default() {
    // decision 0094: the default options prove the base by reaching definitions: 30 sites in the closure of --pdata (3 with
    // decision 0090; the three sites below are the ones of 0090), and the swapped site 0x489f8fb of decision 0095: 31, and the two
    // scheduled sites of decision 0109 (0x4756752, 0x47596c2): 33. 0x470a89e is resolved by both
    let Some(dll) = blend2d_path() else { return };
    let data = std::fs::read(&dll).unwrap();
    let (c, st) = gen_blend2d(&data);
    assert_eq!(st.bad_nodes, 0);
    assert_eq!((st.msvc_tables, st.jumptabs), (33, 33), "{:?}", st.msvc_rejected);
    assert!(st.msvc_rejected.iter().all(|(va, _)| *va != SITE), "{:?}", st.msvc_rejected);
    for va in [0x4701384u64, 0x4701cbb, SITE, 0x4756752, 0x47596c2] {
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ {{ uint64_t t = R[")), "site {va:#x}");
    }
    assert!(!c.contains(&format!("L_{SITE:x}: {{ /* {SITE:#x} Jmp */ {{ fle_dispatch(R[1]); return; }}}}")));
}

#[test]
fn real_blend2d_f_4709ea0_every_index_reaches_the_table_target_by_default() {
    let Some(dll) = blend2d_path() else { return };
    let data = std::fs::read(&dll).unwrap();
    let img = Img(&data);
    let t = read_tables(&img);
    // expectation from the file (not from the generator): index i in 0..=31 -> rva[idx_bytes[i]]; above 31 the `ja`'s 0x470aafc
    let want_for = |i: u64| -> u64 {
        if i <= 31 {
            IBASE + t.rvas[t.idx_bytes[i as usize] as usize] as u64
        } else {
            0x470aafc
        }
    };
    assert_eq!(t.rvas.iter().map(|&r| IBASE + r as u64).collect::<Vec<_>>(), [0x470a8a0, 0x470a8af, 0x470aa40, 0x470aa6b, 0x470aab1, 0x470aafc]);
    assert_eq!(t.idx_bytes.len(), 32);
    let indices: Vec<u64> = (0..=32).chain([33, 34, 63, 100, 0xff, 0x8000_0000, 0xffff_ffff]).collect();
    assert_eq!(indices.iter().filter(|&&i| i <= 32).count(), 33);

    let (c, st) = gen_blend2d(&data);
    assert!(st.msvc_rejected.iter().all(|(va, _)| *va != SITE), "{:?}", st.msvc_rejected);
    assert_eq!(st.bad_nodes, 0);
    let f = cut(&c, F);
    // the six cases of the switch at 0x470a89e, each a goto to a label of f_4709ea0
    let sw = f.lines().find(|l| l.starts_with(&format!("L_{SITE:x}: "))).expect("site line");
    assert_eq!(sw.matches("case UINT64_C(").count(), 6, "{sw}");
    for r in &t.rvas {
        let va = IBASE + *r as u64;
        assert!(sw.contains(&format!("case UINT64_C({va:#x}): goto L_{va:x}; ")), "{va:#x}: {sw}");
        assert_eq!(f.matches(&format!("L_{va:x}: {{ ")).count(), 1, "label {va:#x}");
    }
    assert!(sw.contains(&format!("fle_jumptab_miss(UINT64_C({SITE:#x}), t)")));

    // lifted copy of the function with a hit probe at each of the 6 labels; entered at the guard's `cmp`
    let dir = out_dir();
    let mut body = f.clone();
    let mut patch = std::collections::BTreeSet::new();
    for r in &t.rvas {
        let va = IBASE + *r as u64;
        let lab = format!("L_{va:x}: {{ ");
        assert_eq!(body.matches(&lab).count(), 1);
        body = body.replacen(&lab, &format!("{lab}fle_hit(UINT64_C({va:#x})); "), 1);
        patch.insert(va);
    }
    let head = format!("void f_{F:x}(void)\n{{\n");
    body = body.replacen(&head, &format!("{head}goto L_{GUARD_CMP:x};\n"), 1);
    let prelude = &c[..c.find("void f_").unwrap()];
    let lifted = dir.join(format!("{}.c", etxt_common::per_process("b2d_lift")));
    std::fs::write(&lifted, format!("{prelude}void fle_hit(uint64_t);\nvoid f_{F:x}(void);\n{body}\n")).unwrap();
    let idx_c: Vec<String> = indices.iter().map(|i| format!("{i:#x}ull")).collect();
    let pl: Vec<String> = patch.iter().map(|p| format!("{p:#x}ull")).collect();
    let main_c = REAL_MAIN.replace("@IDX@", &idx_c.join(", ")).replace("@PATCH@", &pl.join(", "));
    let m = dir.join(format!("{}.c", etxt_common::per_process("b2d_main")));
    std::fs::write(&m, main_c).unwrap();
    let exe = dir.join(etxt_common::per_process("b2d_exe"));
    let rt = crate_dir().join("../../runtime");
    clang(
        &[
            "-o",
            exe.to_str().unwrap(),
            lifted.to_str().unwrap(),
            m.to_str().unwrap(),
            crate_dir().join("../../harness/image.c").to_str().unwrap(),
            rt.join("fle_cpuid.c").to_str().unwrap(),
            rt.join("fle_int_more.c").to_str().unwrap(),
        ],
        &dir,
    );
    let mut n = 0;
    for mode in ["native", "lifted"] {
        let r = Command::new(&exe).arg(mode).arg(&dll).output().expect("run harness");
        assert!(r.status.success(), "{mode}: {}{}", String::from_utf8_lossy(&r.stdout), String::from_utf8_lossy(&r.stderr));
        let s = String::from_utf8_lossy(&r.stdout).to_string();
        let got: Vec<&str> = s.lines().collect();
        assert_eq!(got.len(), indices.len(), "{mode}: {s}");
        for (k, i) in indices.iter().enumerate() {
            assert_eq!(got[k], format!("{k} {:x}", want_for(*i)), "{mode}, index {i:#x}");
            n += 1;
        }
    }
    assert_eq!(n, 2 * indices.len());
}

// ---------------------------------------------------------------------------------------------
// 3. Decision 0093: padding `nop` between the guard's `ja` and the table idiom
// ---------------------------------------------------------------------------------------------

fn pads(list: &[&[u8]]) -> Vec<Vec<u8>> {
    list.iter().map(|b| b.to_vec()).collect()
}

#[test]
fn a_padding_nop_between_the_ja_and_the_idiom_every_index_matches_native_and_the_table() {
    // the 12-byte `data16 x3; nop word [rax+rax]` of Blend2D f_48a2060, and other lengths, (c1) and (c2): native (real
    // instructions, the padding is executed) and generated C agree with the table bytes for every index 0..=N+3
    every_index_matches_native_and_the_table(&Sw { pad: pads(&[NOP_12]), ..Sw::c1() }, "c1p12");
    every_index_matches_native_and_the_table(&Sw { pad: pads(&[NOP_12]), ..Sw::c2() }, "c2p12");
    every_index_matches_native_and_the_table(&Sw { pad: pads(&[NOP_1]), ..Sw::c1() }, "c1p1");
    every_index_matches_native_and_the_table(&Sw { pad: pads(&[NOP_2_XCHG_AX_AX]), ..Sw::c2() }, "c2p2");
    every_index_matches_native_and_the_table(&Sw { pad: pads(&[NOP_15]), ..Sw::c1() }, "c1p15");
    // two padding nops (the whole MSVC_SKIP), with a `cdqe` and a 64-bit guard, with a call, after a diamond, with `sub`
    every_index_matches_native_and_the_table(&Sw { pad: pads(&[NOP_5, NOP_3]), ..Sw::c1() }, "c1pp");
    every_index_matches_native_and_the_table(&Sw { cdqe: true, guard: Guard::Rax, pad: pads(&[NOP_9]), ..Sw::c2() }, "c2pc");
    every_index_matches_native_and_the_table(&Sw { cdqe: true, guard: Guard::Rax, pad: pads(&[NOP_8]), pad_before_cdqe: true, call: true, ..Sw::c1() }, "c1pbc");
    every_index_matches_native_and_the_table(&Sw { lea: Lea::AfterDiamond, pad: pads(&[NOP_7]), ..Sw::c2() }, "c2pd");
    every_index_matches_native_and_the_table(&Sw { sub_k: 5, pad: pads(&[NOP_12]), ..Sw::c1() }, "c1ps");
    // a volatile base without a call, as r10 in f_48a2060
    every_index_matches_native_and_the_table(&Sw { base: 10, push_pop: false, pad: pads(&[NOP_12]), ..Sw::c1() }, "c1pr10");
}

#[test]
fn a_padded_site_is_a_switch_in_the_generated_c() {
    for o in [Sw { pad: pads(&[NOP_12]), ..Sw::c1() }, Sw { pad: pads(&[NOP_12]), ..Sw::c2() }] {
        let (_, b, c, st) = gen(&o);
        assert_eq!((st.jumptabs, st.msvc_tables), (1, 1));
        assert!(st.msvc_rejected.is_empty(), "{:?}", st.msvc_rejected);
        assert_eq!(st.bad_nodes, 0);
        assert_eq!(st.handled["jmp_ind:msvc_table"], 1);
        assert!(!st.register_jmps.contains_key("jmp_ind:reg_dispatch"));
        assert_eq!(c.matches("case UINT64_C(").count(), 3);
        let jmp = IB + b.labels["jmp"] as u64;
        assert_eq!(c.matches(&format!("fle_jumptab_miss(UINT64_C({jmp:#x}), t)")).count(), 1);
        assert!(!c.contains("fle_dispatch(R["), "no dispatch left in the function");
    }
}

#[test]
fn padded_sites_that_decision_0093_rejected_are_resolved_by_the_scheduled_form_by_default() {
    // decision 0109: with the default base proof the gap between the `ja` and the idiom may hold anything that does not write the
    // compared register (here `mov edx,1`, `pause`, three nops); the generated C is a `switch` as for a padded site
    for o in [
        Sw { pad: pads(&[MOV_EDX_1]), ..Sw::c1() },
        Sw { pad: pads(&[MOV_EDX_1]), ..Sw::c2() },
        Sw { pad: pads(&[PAUSE]), ..Sw::c1() },
        Sw { pad: pads(&[NOP_5, NOP_3, NOP_1]), ..Sw::c1() },
    ] {
        let (_, b, c, st) = gen(&o);
        assert_eq!((st.jumptabs, st.msvc_tables), (1, 1), "{o:?}: {:?}", st.msvc_rejected);
        assert!(st.msvc_rejected.is_empty(), "{o:?}: {:?}", st.msvc_rejected);
        assert_eq!(st.bad_nodes, 0);
        assert_eq!(st.handled["jmp_ind:msvc_table"], 1);
        assert!(!st.register_jmps.contains_key("jmp_ind:reg_dispatch"));
        let jmp = IB + b.labels["jmp"] as u64;
        assert_eq!(c.matches(&format!("fle_jumptab_miss(UINT64_C({jmp:#x}), t)")).count(), 1, "{o:?}");
        assert!(!c.contains("fle_dispatch(R["), "{o:?}: no dispatch left in the function");
    }
}

// ---- real Blend2D f_48a2060: the native side only ----

const REAL_MAIN_48A: &str = r#"
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

static const uint64_t sizes[] = { @SIZES@ };
static const uint64_t patch[] = { @PATCH@ };
static uint8_t src[0x400], dst[0x400];

static void on_trap(int sig, siginfo_t *si, void *u)
{
    ucontext_t *uc = u;
    (void)sig; (void)si;
    printf("%llx\n", (unsigned long long)(uc->uc_mcontext.gregs[REG_RIP] - 1));
    fflush(stdout);
    _exit(0);
}

static void run_native(uint64_t r8)
{
    struct { uint64_t rcx, rdx, r8, go; } r;
    r.rcx = (uint64_t)(uintptr_t)dst; r.rdx = (uint64_t)(uintptr_t)src; r.r8 = r8; r.go = 0x48a2060ull;
    asm volatile("mov %0, %%r11\n\t"
                 "mov 0(%%r11), %%rcx\n\t"
                 "mov 8(%%r11), %%rdx\n\t"
                 "mov 16(%%r11), %%r8\n\t"
                 "jmp *24(%%r11)"
                 : : "r"(&r) : "memory");
}

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    size_t i, k;
    if (argc < 2) return 2;
    if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    for (k = 0; k < sizeof sizes / sizeof sizes[0]; k++) {
        pid_t p;
        int st;
        printf("%zu ", k);
        fflush(stdout);
        p = fork();
        if (p == 0) {
            struct sigaction sa;
            alarm(10);
            memset(&sa, 0, sizeof sa);
            sa.sa_sigaction = on_trap;
            sa.sa_flags = SA_SIGINFO;
            sigaction(SIGTRAP, &sa, 0);
            for (i = 0; i < sizeof patch / sizeof patch[0]; i++) *(volatile uint8_t *)(uintptr_t)patch[i] = 0xcc;
            run_native(sizes[k]);
            printf("none\n");
            _exit(0);
        }
        waitpid(p, &st, 0);
        if (!WIFEXITED(st) || WEXITSTATUS(st) != 0) printf("child-status-%d\n", st);
    }
    return 0;
}
"#;

#[test]
fn real_blend2d_f_48a2060_native_every_size_reaches_the_table_target() {
    // The native side only (decision 0093's test; the generated C side is `real_f_48a2060_and_f_37cc7e0_...` below, since the
    // reaching definitions of decision 0094 resolve the site). The real instructions run from the function
    // entry (0x48a2060) with r8 = 0..15 and some larger sizes; 0xCC is put at the 16 table targets and at the `ja`'s target
    // 0x48a2180, and the SIGTRAP handler prints where it stopped. The expectation is read from the file (the RVA table at
    // 0x2fd050, 16 entries), not from the generator.
    let Some(dll) = blend2d_path() else { return };
    let data = std::fs::read(&dll).unwrap();
    let img = Img(&data);
    // head of the function and the idiom, from the file
    assert_eq!(img.rd(0x48a2063, 7), &[0x4c, 0x8d, 0x15, 0x96, 0xdf, 0xe5, 0xff], "lea r10,[ImageBase]");
    assert_eq!(img.rd(0x48a206a, 4), &[0x49, 0x83, 0xf8, 0x0f], "cmp r8,0xf");
    assert_eq!(img.rd(0x48a206e, 6), &[0x0f, 0x87, 0x0c, 0x01, 0x00, 0x00], "ja 0x48a2180");
    assert_eq!(img.rd(0x48a2074, 12), &[0x66, 0x66, 0x66, 0x66, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00], "12-byte nop");
    assert_eq!(img.rd(0x48a2080, 7), &[0x47, 0x8b, 0x8c, 0x82, 0x50, 0xd0, 0x2f], "mov r9d,[r10+r8*4+0x2fd050]");
    assert_eq!(img.rd(0x48a2088, 6), &[0x4d, 0x03, 0xca, 0x41, 0xff, 0xe1], "add r9,r10; jmp r9");
    let table: Vec<u64> = (0..16).map(|k| IBASE + u32::from_le_bytes(img.rd(IBASE + 0x2fd050 + 4 * k, 4).try_into().unwrap()) as u64).collect();
    assert_eq!(
        table,
        [0x48a208e, 0x48a2174, 0x48a20c8, 0x48a20ff, 0x48a217a, 0x48a215f, 0x48a2150, 0x48a20d0, 0x48a216d, 0x48a2135, 0x48a2126, 0x48a20b0, 0x48a2143, 0x48a2110, 0x48a20e8, 0x48a2090]
    );
    let sizes: Vec<u64> = (0..=15).chain([16, 17, 31, 32, 33, 0x100, 0xffff_ffff, u64::MAX]).collect();
    let want = |s: u64| -> u64 {
        if s <= 15 {
            table[s as usize]
        } else {
            0x48a2180
        }
    };
    let mut patch: std::collections::BTreeSet<u64> = table.iter().copied().collect();
    patch.insert(0x48a2180);
    let dir = out_dir();
    let size_c: Vec<String> = sizes.iter().map(|s| format!("{s:#x}ull")).collect();
    let patch_c: Vec<String> = patch.iter().map(|p| format!("{p:#x}ull")).collect();
    let main_c = REAL_MAIN_48A.replace("@SIZES@", &size_c.join(", ")).replace("@PATCH@", &patch_c.join(", "));
    let m = dir.join(format!("{}.c", etxt_common::per_process("b2d_48a_main")));
    std::fs::write(&m, main_c).unwrap();
    let exe = dir.join(etxt_common::per_process("b2d_48a_exe"));
    clang(&["-o", exe.to_str().unwrap(), m.to_str().unwrap(), crate_dir().join("../../harness/image.c").to_str().unwrap()], &dir);
    let r = Command::new(&exe).arg(&dll).output().expect("run harness");
    assert!(r.status.success(), "{}{}", String::from_utf8_lossy(&r.stdout), String::from_utf8_lossy(&r.stderr));
    let s = String::from_utf8_lossy(&r.stdout).to_string();
    let got: Vec<&str> = s.lines().collect();
    assert_eq!(got.len(), sizes.len(), "{s}");
    for (k, sz) in sizes.iter().enumerate() {
        assert_eq!(got[k], format!("{k} {:x}", want(*sz)), "native, r8 = {sz:#x}");
    }
}

// ---------------------------------------------------------------------------------------------
// 4. Decision 0094: f_48a2060 (Blend2D) and f_37cc7e0 (dsp_ippv2), the `jmp r9` of a memcpy-like function, native and lifted
// ---------------------------------------------------------------------------------------------

/// The same function in two images: `mov rax,rcx; lea r10,[ImageBase]; cmp r8,0xf; ja default; nop; mov r9d,[r10+r8*4+T];
/// add r9,r10; jmp r9`, 16 entries. `rcx` = destination, `rdx` = source, `r8` = the size.
struct F16 {
    tag: &'static str,
    /// the function entry and the jump
    go: u64,
    site: u64,
    /// the target of the `ja`
    default_target: u64,
    ibase: u64,
    table_rva: u64,
    /// the targets, r8 = 0..entries-1, read with objdump and capstone (lift/out-rdefs/logs/table-facts.txt)
    targets: &'static [u64],
    /// the bytes of the idiom, at (VA, bytes)
    bytes: &'static [(u64, &'static [u8])],
    /// the number of entries of the table (N + 1): 16 for `f_48a2060`, 17 for `f_489f7d0` (decision 0095)
    entries: u64,
    /// the 8 sizes above the table
    out_of_range: [u64; 8],
}

const F48A2060_TARGETS_16: [u64; 16] =
    [0x48a208e, 0x48a2174, 0x48a20c8, 0x48a20ff, 0x48a217a, 0x48a215f, 0x48a2150, 0x48a20d0, 0x48a216d, 0x48a2135, 0x48a2126, 0x48a20b0, 0x48a2143, 0x48a2110, 0x48a20e8, 0x48a2090];
const F37CC7E0_TARGETS_16: [u64; 16] =
    [0x37cc80e, 0x37cc8f4, 0x37cc848, 0x37cc87f, 0x37cc8fa, 0x37cc8df, 0x37cc8d0, 0x37cc850, 0x37cc8ed, 0x37cc8b5, 0x37cc8a6, 0x37cc830, 0x37cc8c3, 0x37cc890, 0x37cc868, 0x37cc810];

const REAL_MAIN_LIFT16: &str = r#"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>
#include "boot.h"
#include "fle_dispatch.h"

extern uint64_t R[16];
const fle_fn_entry fle_dispatch_table[] = { { 0, 0 } };
const uint64_t fle_dispatch_count = 0;
void fle_hit(uint64_t va) { printf("%llx\n", (unsigned long long)va); fflush(stdout); _exit(0); }
void @FN@(void);
static const uint64_t sizes[] = { @SIZES@ };
static uint8_t src[0x400], dst[0x400], stk[0x1000];

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    size_t k;
    if (argc < 2) return 2;
    if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    for (k = 0; k < sizeof sizes / sizeof sizes[0]; k++) {
        pid_t p;
        int st;
        printf("%zu ", k);
        fflush(stdout);
        p = fork();
        if (p == 0) {
            alarm(10);
            memset(R, 0, sizeof R);
            R[4] = (uint64_t)(uintptr_t)(stk + 0x800);
            R[1] = (uint64_t)(uintptr_t)dst;
            R[2] = (uint64_t)(uintptr_t)src;
            R[8] = sizes[k];
            @FN@();
            printf("none\n");
            _exit(0);
        }
        waitpid(p, &st, 0);
        if (!WIFEXITED(st) || WEXITSTATUS(st) != 0) printf("child-status-%d\n", st);
    }
    return 0;
}
"#;

/// What the table bytes give for a size: the target of entry `s` for s < entries, else the guard's other side (`default_target`).
fn f16_want(f: &F16, table: &[u64], s: u64) -> u64 {
    if s < f.entries {
        table[s as usize]
    } else {
        f.default_target
    }
}

fn f16_native_and_lifted(f: &F16, dll: &Path, data: &[u8], seeds: &[u64]) {
    let img = Img(data);
    for (va, want) in f.bytes {
        assert_eq!(img.rd(*va, want.len()), *want, "bytes at {va:#x}");
    }
    // the RVAs read from the file, not from the generator, equal the objdump/capstone list
    let table: Vec<u64> = (0..f.entries).map(|k| f.ibase + u32::from_le_bytes(img.rd(f.ibase + f.table_rva + 4 * k, 4).try_into().unwrap()) as u64).collect();
    assert_eq!(table, f.targets, "{}", f.tag);
    assert_eq!(table.iter().collect::<std::collections::BTreeSet<_>>().len() as u64, f.entries);
    let sizes: Vec<u64> = (0..f.entries).chain(f.out_of_range).collect();
    assert_eq!((sizes.len() as u64, sizes.iter().filter(|&&s| s < f.entries).count() as u64), (f.entries + 8, f.entries));
    assert!(f.out_of_range.iter().all(|&s| s >= f.entries));

    // the generated C: a `switch` over the 16 targets, each a goto to a label of the function
    let opts = fe_decode::Options::default();
    let mut out = Vec::new();
    let st = generate_with_options(data, seeds, fe_decode::jumptab::FLENGINE, &opts, &mut out).expect("generate");
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.bad_nodes, 0, "{}", f.tag);
    assert!(st.msvc_rejected.iter().all(|(va, _)| *va != f.site), "{}: {:?}", f.tag, st.msvc_rejected.iter().filter(|(va, _)| *va == f.site).collect::<Vec<_>>());
    let func = cut(&c, f.go);
    let sw = func.lines().find(|l| l.starts_with(&format!("L_{:x}: ", f.site))).unwrap_or_else(|| panic!("{}: no line of the site", f.tag));
    assert_eq!(sw.matches("case UINT64_C(").count() as u64, f.entries, "{}: {sw}", f.tag);
    for t in &table {
        assert!(sw.contains(&format!("case UINT64_C({t:#x}): goto L_{t:x}; ")), "{}: {t:#x}", f.tag);
        assert_eq!(func.matches(&format!("L_{t:x}: {{ ")).count(), 1, "{}: label {t:#x}", f.tag);
    }
    assert!(sw.contains(&format!("fle_jumptab_miss(UINT64_C({:#x}), t)", f.site)), "{}", f.tag);
    assert!(!sw.contains("fle_dispatch(R["), "{}", f.tag);

    let dir = out_dir();
    // native: real instructions from the entry, 0xCC at the 16 targets and at the `ja`'s target
    let size_c: Vec<String> = sizes.iter().map(|s| format!("{s:#x}ull")).collect();
    let mut patch: std::collections::BTreeSet<u64> = table.iter().copied().collect();
    patch.insert(f.default_target);
    let patch_c: Vec<String> = patch.iter().map(|p| format!("{p:#x}ull")).collect();
    let main_n = REAL_MAIN_48A.replace("@SIZES@", &size_c.join(", ")).replace("@PATCH@", &patch_c.join(", ")).replace("0x48a2060ull", &format!("{:#x}ull", f.go));
    let m = dir.join(format!("{}.c", etxt_common::per_process(&format!("f16_{}_nmain", f.tag))));
    std::fs::write(&m, main_n).unwrap();
    let exe_n = dir.join(etxt_common::per_process(&format!("f16_{}_nexe", f.tag)));
    clang(&["-o", exe_n.to_str().unwrap(), m.to_str().unwrap(), crate_dir().join("../../harness/image.c").to_str().unwrap()], &dir);

    // lifted: the function of the generated C with a probe at each of the 17 labels, entered at its first instruction
    let mut body = func.clone();
    for p in &patch {
        let lab = format!("L_{p:x}: {{ ");
        assert_eq!(body.matches(&lab).count(), 1, "{}: {p:#x}", f.tag);
        body = body.replacen(&lab, &format!("{lab}fle_hit(UINT64_C({p:#x})); "), 1);
    }
    // the function calls other functions of the closure (a tail call to `f_48a2040` at 0x48a21d6, in the same C file): declare
    // every function of the generated C, define only this one (the others are never reached by the sizes below)
    let decl_lines: Vec<&str> = c.lines().filter(|l| l.starts_with("void f_") && l.ends_with("(void);")).collect();
    let fname = format!("f_{:x}", f.go);
    // a stub for each of them except the function itself: a size above 15 never gets there (the `ja` of the guard is the
    // probe at `default_target`, which stops first), and a stub that is reached says so
    let stubs: String = decl_lines
        .iter()
        .filter(|l| **l != format!("void {fname}(void);"))
        .map(|l| format!("{} {{ printf(\"stub %s\\n\", __func__); _exit(9); }}\n", l.trim_end_matches(';')))
        .collect();
    let decls: String = decl_lines.iter().map(|l| format!("{l}\n")).collect();
    let prelude = &c[..c.find("void f_").unwrap()];
    let lifted = dir.join(format!("{}.c", etxt_common::per_process(&format!("f16_{}_lift", f.tag))));
    std::fs::write(&lifted, format!("{prelude}#include <unistd.h>\n#include <stdio.h>\nvoid fle_hit(uint64_t);\n{decls}{stubs}{body}\n")).unwrap();
    let main_l = REAL_MAIN_LIFT16.replace("@FN@", &fname).replace("@SIZES@", &size_c.join(", "));
    let ml = dir.join(format!("{}.c", etxt_common::per_process(&format!("f16_{}_lmain", f.tag))));
    std::fs::write(&ml, main_l).unwrap();
    let exe_l = dir.join(etxt_common::per_process(&format!("f16_{}_lexe", f.tag)));
    let rt = crate_dir().join("../../runtime");
    clang(
        &[
            "-o",
            exe_l.to_str().unwrap(),
            lifted.to_str().unwrap(),
            ml.to_str().unwrap(),
            crate_dir().join("../../harness/image.c").to_str().unwrap(),
            rt.join("fle_cpuid.c").to_str().unwrap(),
            rt.join("fle_int_more.c").to_str().unwrap(),
            rt.join("fle_string.c").to_str().unwrap(),
        ],
        &dir,
    );
    let mut n = 0;
    for (mode, exe) in [("native", &exe_n), ("lifted", &exe_l)] {
        let r = Command::new(exe).arg(dll).output().expect("run harness");
        assert!(r.status.success(), "{} {mode}: {}{}", f.tag, String::from_utf8_lossy(&r.stdout), String::from_utf8_lossy(&r.stderr));
        let s = String::from_utf8_lossy(&r.stdout).to_string();
        let got: Vec<&str> = s.lines().collect();
        assert_eq!(got.len(), sizes.len(), "{} {mode}: {s}", f.tag);
        for (k, sz) in sizes.iter().enumerate() {
            assert_eq!(got[k], format!("{k} {:x}", f16_want(f, &table, *sz)), "{} {mode}, r8 = {sz:#x}", f.tag);
            n += 1;
        }
    }
    assert_eq!(n, 2 * sizes.len());
}

#[test]
fn real_blend2d_f_48a2060_every_size_reaches_the_table_target_natively_and_lifted() {
    // decision 0094: `jmp r9` at 0x48a208b is resolved (r10: 3 writes, 3 other indirect jumps; only the `lea` reaches the jump).
    // Seeds as the 3-module measurement but without the 58 (--pdata only): the function is in the closure
    let Some(dll) = blend2d_path() else { return };
    let data = std::fs::read(&dll).unwrap();
    let pe = fe_pe::Pe::parse(&data).unwrap();
    assert_eq!(pe.optional.image_base, IBASE);
    let seeds = fe_decode::pdata_seeds(&pe).unwrap();
    let f = F16 {
        tag: "b2d",
        go: 0x48a2060,
        site: 0x48a208b,
        default_target: 0x48a2180,
        ibase: IBASE,
        table_rva: 0x2fd050,
        targets: &F48A2060_TARGETS_16,
        entries: 16,
        out_of_range: [16, 17, 31, 32, 33, 0x100, 0xffff_ffff, u64::MAX],
        bytes: &[
            (0x48a2063, &[0x4c, 0x8d, 0x15, 0x96, 0xdf, 0xe5, 0xff]),
            (0x48a206a, &[0x49, 0x83, 0xf8, 0x0f]),
            (0x48a2074, &[0x66, 0x66, 0x66, 0x66, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00]),
            (0x48a2080, &[0x47, 0x8b, 0x8c, 0x82, 0x50, 0xd0, 0x2f]),
            (0x48a2088, &[0x4d, 0x03, 0xca, 0x41, 0xff, 0xe1]),
        ],
    };
    f16_native_and_lifted(&f, &dll, &data, &seeds);
}

#[test]
fn real_dsp_ippv2_f_37cc7e0_every_size_reaches_the_table_target_natively_and_lifted() {
    // the same function in dsp_ippv2 (rebased 0x2000000; $DSP_IPPV2_PE_2000000, else lift/out-jtb2d90/three/dll/
    // dsp_ippv2_x64.2000000.dll; skipped when absent): `jmp r9` at 0x37cc80b. The function entry is the one seed
    let explicit = std::env::var_os("DSP_IPPV2_PE_2000000");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => crate_dir().join("../../out-jtb2d90/three/dll/dsp_ippv2_x64.2000000.dll"),
    };
    let dll = match std::fs::metadata(&path) {
        Ok(_) => path,
        Err(e) if explicit.is_some() => panic!("DSP_IPPV2_PE_2000000={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            return;
        }
    };
    let data = std::fs::read(&dll).unwrap();
    let pe = fe_pe::Pe::parse(&data).unwrap();
    if pe.optional.image_base != 0x2000000 {
        eprintln!("SKIPPED: the image base is {:#x}, not the rebased 0x2000000", pe.optional.image_base);
        return;
    }
    let f = F16 {
        tag: "dsp",
        go: 0x37cc7e0,
        site: 0x37cc80b,
        default_target: 0x37cc900,
        ibase: 0x2000000,
        table_rva: 0x2615000,
        targets: &F37CC7E0_TARGETS_16,
        entries: 16,
        out_of_range: [16, 17, 31, 32, 33, 0x100, 0xffff_ffff, u64::MAX],
        bytes: &[
            (0x37cc7e3, &[0x4c, 0x8d, 0x15, 0x16, 0x38, 0x83, 0xfe]),
            (0x37cc7ea, &[0x49, 0x83, 0xf8, 0x0f]),
            (0x37cc7f4, &[0x66, 0x66, 0x66, 0x66, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00]),
            (0x37cc800, &[0x47, 0x8b, 0x8c, 0x82, 0x00, 0x50, 0x61, 0x02]),
            (0x37cc808, &[0x4d, 0x03, 0xca, 0x41, 0xff, 0xe1]),
        ],
    };
    f16_native_and_lifted(&f, &dll, &data, &[f.go]);
}

// ---------------------------------------------------------------------------------------------
// 5. Decision 0095: the swapped form. f_489f7d0 (Blend2D) and f_37cd2c0 (dsp_ippv2), a memset-like function, native and lifted
// ---------------------------------------------------------------------------------------------

const F489F7D0_TARGETS_17: [u64; 17] = [
    0x489f90e, 0x489f90b, 0x489f937, 0x489f907, 0x489f914, 0x489f924, 0x489f934, 0x489f904, 0x489f93c, 0x489f918, 0x489f950, 0x489f940, 0x489f910,
    0x489f920, 0x489f930, 0x489f900, 0x489f958,
];
const F37CD2C0_TARGETS_17: [u64; 17] = [
    0x37cd3fe, 0x37cd3fb, 0x37cd427, 0x37cd3f7, 0x37cd404, 0x37cd414, 0x37cd424, 0x37cd3f4, 0x37cd42c, 0x37cd408, 0x37cd440, 0x37cd430, 0x37cd400,
    0x37cd410, 0x37cd420, 0x37cd3f0, 0x37cd448,
];
/// 17 and 18 (just above the table), 31 and 32, 0x100, 0xffffffff, 0x100000000 (a 32-bit compare would take it for 0) and u64::MAX
const OUT_OF_RANGE_17: [u64; 8] = [17, 18, 31, 32, 0x100, 0xffff_ffff, 0x1_0000_0000, u64::MAX];

#[test]
fn real_blend2d_f_489f7d0_every_size_reaches_the_table_target_natively_and_lifted() {
    // decision 0095: `jmp r9` at 0x489f8fb is resolved (N = 0x10 from `cmp r8,0x10; jbe`, 17 entries). Seeds as the other tests
    // of this file: --pdata only. Natively the real instructions run from the entry (0x489f7d0), with 0xCC at the 17 targets and at
    // 0x489f7ee, where the `jbe` falls through when r8 > 0x10; the generated C runs with a probe at each of those labels
    let Some(dll) = blend2d_path() else { return };
    let data = std::fs::read(&dll).unwrap();
    let pe = fe_pe::Pe::parse(&data).unwrap();
    assert_eq!(pe.optional.image_base, IBASE);
    let seeds = fe_decode::pdata_seeds(&pe).unwrap();
    let f = F16 {
        tag: "b2d_memset",
        go: 0x489f7d0,
        site: 0x489f8fb,
        default_target: 0x489f7ee,
        ibase: IBASE,
        table_rva: 0x2fd000,
        targets: &F489F7D0_TARGETS_17,
        entries: 17,
        out_of_range: OUT_OF_RANGE_17,
        bytes: &[
            (0x489f7e4, &[0x49, 0x83, 0xf8, 0x10]),
            (0x489f7e8, &[0x0f, 0x86, 0xf2, 0x00, 0x00, 0x00]),
            (0x489f7ee, &[0x66, 0x49, 0x0f, 0x6e, 0xc1]),
            (0x489f8e3, &[0x4c, 0x8d, 0x0d, 0x16, 0x07, 0xe6, 0xff]),
            (0x489f8ea, &[0x43, 0x8b, 0x84, 0x81, 0x00, 0xd0, 0x2f, 0x00]),
            (0x489f8f2, &[0x4c, 0x03, 0xc8]),
            (0x489f8f5, &[0x49, 0x03, 0xc8]),
            (0x489f8f8, &[0x49, 0x8b, 0xc3]),
            (0x489f8fb, &[0x41, 0xff, 0xe1]),
        ],
    };
    f16_native_and_lifted(&f, &dll, &data, &seeds);
}

#[test]
fn real_dsp_ippv2_f_37cd2c0_every_size_reaches_the_table_target_natively_and_lifted() {
    // the same function in dsp_ippv2 (rebased 0x2000000), `jmp r9` at 0x37cd3eb; the function entry is the one seed
    let explicit = std::env::var_os("DSP_IPPV2_PE_2000000");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => crate_dir().join("../../out-jtb2d90/three/dll/dsp_ippv2_x64.2000000.dll"),
    };
    let dll = match std::fs::metadata(&path) {
        Ok(_) => path,
        Err(e) if explicit.is_some() => panic!("DSP_IPPV2_PE_2000000={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            return;
        }
    };
    let data = std::fs::read(&dll).unwrap();
    let pe = fe_pe::Pe::parse(&data).unwrap();
    if pe.optional.image_base != 0x2000000 {
        eprintln!("SKIPPED: the image base is {:#x}, not the rebased 0x2000000", pe.optional.image_base);
        return;
    }
    let f = F16 {
        tag: "dsp_memset",
        go: 0x37cd2c0,
        site: 0x37cd3eb,
        default_target: 0x37cd2de,
        ibase: 0x2000000,
        table_rva: 0x26150b0,
        targets: &F37CD2C0_TARGETS_17,
        entries: 17,
        out_of_range: OUT_OF_RANGE_17,
        bytes: &[
            (0x37cd2d4, &[0x49, 0x83, 0xf8, 0x10]),
            (0x37cd2d8, &[0x0f, 0x86, 0xf2, 0x00, 0x00, 0x00]),
            (0x37cd2de, &[0x66, 0x49, 0x0f, 0x6e, 0xc1]),
            (0x37cd3d3, &[0x4c, 0x8d, 0x0d, 0x26, 0x2c, 0x83, 0xfe]),
            (0x37cd3da, &[0x43, 0x8b, 0x84, 0x81, 0xb0, 0x50, 0x61, 0x02]),
            (0x37cd3e2, &[0x4c, 0x03, 0xc8]),
            (0x37cd3e5, &[0x49, 0x03, 0xc8]),
            (0x37cd3e8, &[0x49, 0x8b, 0xc3]),
            (0x37cd3eb, &[0x41, 0xff, 0xe1]),
        ],
    };
    f16_native_and_lifted(&f, &dll, &data, &[f.go]);
}

// ---- synthetic images: the swapped form runs natively and lifted with the same result for every index ----

const SWP_MAIN: &str = r#"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "boot.h"
extern uint64_t R[16];
void @FN@(void);
typedef uint32_t (__attribute__((ms_abi)) *nfn)(int64_t, int64_t);
static uint8_t stk[0x1000];

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    long lo, hi, i;
    if (argc < 4) return 2;
    if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    lo = strtol(argv[2], 0, 0);
    hi = strtol(argv[3], 0, 0);
    for (i = lo; i <= hi; i++) {
        uint32_t nat, lif;
        nat = ((nfn)(uintptr_t)@VA@)(i, 0);
        memset(R, 0, sizeof R);
        R[4] = (uint64_t)(uintptr_t)(stk + 0x800);
        R[1] = (uint64_t)i;
        @FN@();
        lif = (uint32_t)R[0];
        printf("%ld %x %x\n", i, nat, lif);
        fflush(stdout);
    }
    return 0;
}
"#;

/// What the function returns for index `i`: block k returns 0x100 + k (the entry `i` of the table), 0xdead above N.
fn swp_expected(b: &Built, o: &Swp, i: i64) -> u32 {
    if (i as u64) > o.n as u64 {
        return 0xdead;
    }
    let target_rva = b.table[i as usize];
    for (k, l) in ["t0", "t1", "t2"].iter().enumerate() {
        if b.labels[l] == target_rva {
            return 0x100 + k as u32;
        }
    }
    unreachable!("target {target_rva:#x} is no block")
}

fn swp_every_index_matches_native_and_the_table(o: &Swp, tag: &str) {
    let built = build_swapped(AT, o);
    let img = built.img.build();
    let mut out = Vec::new();
    let st = generate_with_specs(&img, &[], &[], &mut out).expect("generate");
    assert_eq!(st.msvc_tables, 1, "{o:?}: {:?}", st.msvc_rejected);
    assert_eq!(st.bad_nodes, 0);
    let c = String::from_utf8(out).unwrap();
    assert_eq!(c.matches("case UINT64_C(").count(), 3, "{o:?}");
    let jmp = IB + built.labels["jmp"] as u64;
    assert_eq!(c.matches(&format!("fle_jumptab_miss(UINT64_C({jmp:#x}), t)")).count(), 1);
    assert!(!c.contains("fle_dispatch(R["), "{o:?}: no dispatch left in the function");
    let dir = out_dir();
    let pe = dir.join(format!("{}.pe", etxt_common::uniq(&format!("swp-{tag}"))));
    std::fs::write(&pe, &img).unwrap();
    let fname = format!("f_{:x}", fva());
    let lifted = dir.join(format!("{}.c", etxt_common::uniq(&format!("swp-lift-{tag}"))));
    std::fs::write(&lifted, &c).unwrap();
    let main_c = SWP_MAIN.replace("@FN@", &fname).replace("@VA@", &format!("{:#x}ull", fva()));
    let m = dir.join(format!("{}.c", etxt_common::uniq(&format!("swp-main-{tag}"))));
    std::fs::write(&m, main_c).unwrap();
    let exe = dir.join(etxt_common::uniq(&format!("swp-exe-{tag}")));
    clang(&["-o", exe.to_str().unwrap(), lifted.to_str().unwrap(), m.to_str().unwrap(), crate_dir().join("../../harness/image.c").to_str().unwrap()], &dir);
    // every index in 0..=N, then N+1..N+3; the image must be mapped at its base for the native side
    let hi = o.n as i64 + 3;
    let r = Command::new(&exe).arg(&pe).arg("0").arg(hi.to_string()).output().unwrap();
    assert!(r.status.success(), "{o:?}: {}", String::from_utf8_lossy(&r.stderr));
    let rows: Vec<(i64, String, String)> = String::from_utf8_lossy(&r.stdout)
        .lines()
        .map(|l| {
            let p: Vec<&str> = l.split(' ').collect();
            (p[0].parse().unwrap(), p[1].to_string(), p[2].to_string())
        })
        .collect();
    assert_eq!(rows.len() as i64, hi + 1);
    for (i, nat, lif) in rows {
        let want = format!("{:x}", swp_expected(&built, o, i));
        assert_eq!(nat, want, "native, idx {i}, {o:?}");
        assert_eq!(lif, want, "lifted, idx {i}, {o:?}");
    }
}

#[test]
fn swapped_every_index_matches_native_and_the_table() {
    // the shape of f_489f7d0 (`cmp rcx,N; jbe`, a nop, `lea r9`, `add r9,rax; add rdx,rcx; mov rax,r11; jmp r9`); r11 holds rcx,
    // set by `mov r11,rcx` in the real function, here the gap `mov rax,r11` reads whatever r11 is: only the table's result counts
    swp_every_index_matches_native_and_the_table(&Swp::real(), "real");
    swp_every_index_matches_native_and_the_table(&Swp { guard: SwGuard::JaFall, pad: Vec::new(), ..Swp::real() }, "jafall");
    swp_every_index_matches_native_and_the_table(&Swp { guard: SwGuard::JaFall32, n: 5, ..Swp::real() }, "jafall32");
    swp_every_index_matches_native_and_the_table(&Swp { guard: SwGuard::JaeFall, n: 7, ..Swp::real() }, "jaefall");
    swp_every_index_matches_native_and_the_table(&Swp { guard: SwGuard::JbTaken, n: 9, ..Swp::real() }, "jbtaken");
    swp_every_index_matches_native_and_the_table(&Swp { lea: SwLea::AtEntry, base: 10, x: 3, ..Swp::real() }, "r10rbx");
}

#[test]
fn swapped_sites_that_are_not_resolved_stay_a_tail_dispatch() {
    for o in [
        Swp { guard: SwGuard::JaTaken, ..Swp::real() },
        Swp { guard: SwGuard::ExtraBranch, ..Swp::real() },
        Swp { guard: SwGuard::WriteBetween, ..Swp::real() },
        Swp { lea: SwLea::Never, ..Swp::real() },
        Swp { lea: SwLea::Wrong(0x10), ..Swp::real() },
        Swp { bad: Bad::OtherFunction, ..Swp::real() },
    ] {
        let built = build_swapped(AT, &o);
        let mut out = Vec::new();
        let st = generate_with_specs(&built.img.build(), &[], &[], &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert_eq!((st.jumptabs, st.msvc_tables), (0, 0), "{o:?}");
        assert_eq!(st.msvc_rejected.len(), 1, "{o:?}: {:?}", st.msvc_rejected);
        assert_eq!(st.handled["jmp_ind:reg_dispatch"], 1, "{o:?}");
        assert!(c.contains("fle_dispatch(R[9]); return;"), "{o:?}");
        assert!(!c.contains("fle_jumptab_miss"), "{o:?}");
    }
}

const SWP_PAST_MAIN: &str = r#"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "boot.h"
extern uint64_t R[16];
void @PAST@(void);
static uint8_t stk[0x1000];

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    if (argc < 3) return 2;
    if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    memset(R, 0, sizeof R);
    R[4] = (uint64_t)(uintptr_t)(stk + 0x800);
    R[1] = strtoull(argv[2], 0, 0);
    @PAST@();
    printf("returned %x\n", (uint32_t)R[0]);
    return 0;
}
"#;

#[test]
fn swapped_an_index_above_n_past_the_guard_stops_in_fle_jumptab_miss() {
    // decision 0095 item 3: the generated `switch` is the one of the standard form (`Select::Target`). The guard's `jbe` never lets
    // an index above N reach it, so it is entered past the guard (a copy of the function that starts at the table block): an
    // index in 0..=N reaches the table's target (control), an index above N reads the entry after the table (another function's RVA)
    // and the jump register is no case: `fle_jumptab_miss`, by name, before anything of the target runs
    let o = Swp::real();
    let built = build_swapped(AT, &o);
    let img = built.img.build();
    let mut out = Vec::new();
    let st = generate_with_specs(&img, &[], &[], &mut out).expect("generate");
    assert_eq!(st.msvc_tables, 1, "{:?}", st.msvc_rejected);
    let c = String::from_utf8(out).unwrap();
    let fname = format!("f_{:x}", fva());
    let past = format!("{fname}_past_guard");
    let idiom = IB + built.labels["idiom"] as u64;
    let head = format!("void {fname}(void)\n{{\n");
    let body = &c[c.find(&head).expect("function")..];
    let func = &body[..body.find("\n}\n\n").unwrap() + 3];
    let copy = func.replacen(&head, &format!("void {past}(void)\n{{\ngoto L_{idiom:x};\n"), 1);
    let dir = out_dir();
    let tag = etxt_common::uniq("swp-past");
    let pe = dir.join(format!("{tag}.pe"));
    std::fs::write(&pe, &img).unwrap();
    let lifted = dir.join(format!("{tag}-lift.c"));
    std::fs::write(&lifted, format!("{c}\nvoid {past}(void);\n{copy}")).unwrap();
    let m = dir.join(format!("{tag}-main.c"));
    std::fs::write(&m, SWP_PAST_MAIN.replace("@PAST@", &past)).unwrap();
    let exe = dir.join(format!("{tag}-exe"));
    clang(&["-o", exe.to_str().unwrap(), lifted.to_str().unwrap(), m.to_str().unwrap(), crate_dir().join("../../harness/image.c").to_str().unwrap()], &dir);
    let jmp = IB + built.labels["jmp"] as u64;
    // control: the indexes 0..=N reach the table's target
    for i in 0..=o.n as i64 {
        let r = Command::new(&exe).arg(&pe).arg(i.to_string()).output().unwrap();
        assert!(r.status.success(), "idx {i}: {}", String::from_utf8_lossy(&r.stderr));
        let want = swp_target_value(&built, i as usize);
        assert_eq!(String::from_utf8_lossy(&r.stdout).trim(), format!("returned {want:x}"), "idx {i}");
    }
    // above N: the entry after the table is the RVA of another function, which is no case
    for i in [17u64, 18, 0x100] {
        let r = Command::new(&exe).arg(&pe).arg(i.to_string()).output().unwrap();
        assert!(!r.status.success(), "idx {i} did not stop: {}", String::from_utf8_lossy(&r.stdout));
        let err = String::from_utf8_lossy(&r.stderr).to_string();
        assert!(err.contains("outside the table"), "idx {i}: {err}");
        assert!(err.contains(&format!("jump table at {jmp:#x}")), "idx {i}: {err}");
        assert!(String::from_utf8_lossy(&r.stdout).is_empty(), "idx {i} ran on");
    }
}

/// What block `t0`/`t1`/`t2` returns for the table entry `i` of `build_swapped`: 0x100 + k.
fn swp_target_value(b: &Built, i: usize) -> u32 {
    let rva = b.table[i];
    for (k, l) in ["t0", "t1", "t2"].iter().enumerate() {
        if b.labels[l] == rva {
            return 0x100 + k as u32;
        }
    }
    panic!("entry {i} ({rva:#x}) is no block")
}

// ---- decision 0109: the scheduled form runs natively and lifted with the same result for every index ----

/// What the function of `build_sched` returns for index `i`: 0xdead above the first guard's N; else block k returns 0x100 + k,
/// except that the loop block (`t2`, and `t1` with `t1_loop`) counts edx down (`sub edx,1`) and goes back to the head while the new
/// value is at most N2 (unsigned), where the table is read again.
fn sch_expected(b: &Built, o: &Sch, i: i64) -> u32 {
    if (i as u64) > o.n as u64 {
        return 0xdead;
    }
    // two-stage: the index byte of entry k is k % 3 (`build_sched`)
    let block = |idx: i64| {
        if o.two_stage {
            (idx % 3) as usize
        } else {
            ["t0", "t1", "t2"].iter().position(|l| b.labels[l] == b.table[idx as usize]).unwrap()
        }
    };
    let (mut idx, mut k) = (i, block(i));
    loop {
        let looping = (k == 2 && o.t2 == SchT2::Loop) || (k == 1 && o.t1_loop);
        if !looping {
            return 0x100 + k as u32;
        }
        idx -= 1;
        if (idx as i32 as u32) > o.n2 {
            return 0x100 + k as u32;
        }
        k = block(idx);
    }
}

fn sch_every_index_matches_native_and_the_table(o: &Sch, tag: &str) {
    let built = build_sched(AT, o);
    let img = built.img.build();
    let mut out = Vec::new();
    let st = generate_with_specs(&img, &[], &[], &mut out).expect("generate");
    assert_eq!(st.msvc_tables, 1, "{o:?}: {:?}", st.msvc_rejected);
    assert_eq!(st.bad_nodes, 0);
    let c = String::from_utf8(out).unwrap();
    assert_eq!(c.matches("case UINT64_C(").count(), 3, "{o:?}");
    let jmp = IB + built.labels["jmp"] as u64;
    assert_eq!(c.matches(&format!("fle_jumptab_miss(UINT64_C({jmp:#x}), t)")).count(), 1);
    assert!(!c.contains("fle_dispatch(R["), "{o:?}: no dispatch left in the function");
    let dir = out_dir();
    let pe = dir.join(format!("{}.pe", etxt_common::uniq(&format!("sch-{tag}"))));
    std::fs::write(&pe, &img).unwrap();
    let fname = format!("f_{:x}", fva());
    let lifted = dir.join(format!("{}.c", etxt_common::uniq(&format!("sch-lift-{tag}"))));
    std::fs::write(&lifted, &c).unwrap();
    let main_c = SWP_MAIN.replace("@FN@", &fname).replace("@VA@", &format!("{:#x}ull", fva()));
    let m = dir.join(format!("{}.c", etxt_common::uniq(&format!("sch-main-{tag}"))));
    std::fs::write(&m, main_c).unwrap();
    let exe = dir.join(etxt_common::uniq(&format!("sch-exe-{tag}")));
    clang(&["-o", exe.to_str().unwrap(), lifted.to_str().unwrap(), m.to_str().unwrap(), crate_dir().join("../../harness/image.c").to_str().unwrap()], &dir);
    // every index in 0..=max(N, N2), then three more
    let hi = o.n.max(o.n2) as i64 + 3;
    let r = Command::new(&exe).arg(&pe).arg("0").arg(hi.to_string()).output().unwrap();
    assert!(r.status.success(), "{o:?}: {}", String::from_utf8_lossy(&r.stderr));
    let rows: Vec<(i64, String, String)> = String::from_utf8_lossy(&r.stdout)
        .lines()
        .map(|l| {
            let p: Vec<&str> = l.split(' ').collect();
            (p[0].parse().unwrap(), p[1].to_string(), p[2].to_string())
        })
        .collect();
    assert_eq!(rows.len() as i64, hi + 1);
    for (i, nat, lif) in rows {
        let want = format!("{:x}", sch_expected(&built, o, i));
        assert_eq!(nat, want, "native, idx {i}, {o:?}");
        assert_eq!(lif, want, "lifted, idx {i}, {o:?}");
    }
}

#[test]
fn scheduled_every_index_matches_native_and_the_table() {
    // the shape of freetype f_4b166e0 (two guards, the second at the bottom of the loop), a larger and a smaller second bound
    sch_every_index_matches_native_and_the_table(&Sch::real(), "real");
    sch_every_index_matches_native_and_the_table(&Sch { n2: 30, ..Sch::real() }, "n2big");
    sch_every_index_matches_native_and_the_table(&Sch { n2: 7, ..Sch::real() }, "n2small");
    sch_every_index_matches_native_and_the_table(&Sch { two_stage: true, ..Sch::real() }, "real2");
    // one guard: a `cdqe`, a `movsxd rax,edx` of the compared register, the idiom on the taken side
    sch_every_index_matches_native_and_the_table(&Sch::simple(), "simple");
    sch_every_index_matches_native_and_the_table(&Sch { cmp: SchCmp::Edx, q: vec![SchQ::Raw(MOVSXD_RAX_EDX.to_vec())], ..Sch::simple() }, "alias");
    sch_every_index_matches_native_and_the_table(&Sch { taken: true, ..Sch::simple() }, "taken");
}

#[test]
fn scheduled_sites_that_are_not_resolved_stay_a_tail_dispatch() {
    for o in [
        // the gap writes the checked register in a way that is not a sign extension
        Sch { q: vec![SchQ::Raw(MOV_EAX_EDX.to_vec()), SchQ::Raw(CDQE.to_vec())], ..Sch::simple() },
        // a way into the idiom that bounds nothing, a third way in, a second guard on the wrong side
        Sch { t2: SchT2::JmpHead, ..Sch::real() },
        Sch { t1_loop: true, ..Sch::real() },
        Sch { second_op: 0x77, ..Sch::real() },
        // the base: never written, a non-`lea` write on the loop's way in; a target in another function
        Sch { q: vec![SchQ::Raw(CDQE.to_vec())], entry_lea: false, ..Sch::simple() },
        Sch { loop_clobbers_base: true, ..Sch::real() },
        Sch { bad: Bad::OtherFunction, ..Sch::real() },
    ] {
        let built = build_sched(AT, &o);
        let mut out = Vec::new();
        let st = generate_with_specs(&built.img.build(), &[], &[], &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert_eq!((st.jumptabs, st.msvc_tables), (0, 0), "{o:?}");
        assert_eq!(st.msvc_rejected.len(), 1, "{o:?}: {:?}", st.msvc_rejected);
        assert_eq!(st.handled["jmp_ind:reg_dispatch"], 1, "{o:?}");
        assert!(c.contains("fle_dispatch(R[1]); return;"), "{o:?}");
        assert!(!c.contains("fle_jumptab_miss"), "{o:?}");
    }
}

// ---- decision 0109 on the real freetype (rebased 0x4b00000; $FREETYPE_PE, else lift/out-wallQ/freetype/freetype_x64.4b00000.dll) ----

#[test]
fn real_freetype_the_stop_point_0x4b1674d_is_a_switch_in_the_generated_c() {
    let explicit = std::env::var_os("FREETYPE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => crate_dir().join("../../out-wallQ/freetype/freetype_x64.4b00000.dll"),
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) if explicit.is_some() => panic!("FREETYPE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            return;
        }
    };
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let mut seeds = fe_decode::pdata_seeds(&pe).unwrap();
    let file = std::fs::read_to_string(crate_dir().join("../../harness/seeds/ft-seeds-4b00000.txt")).unwrap();
    seeds.extend(file.lines().filter(|l| !l.trim().is_empty()).map(|l| u64::from_str_radix(l.trim().trim_start_matches("0x"), 16).unwrap()));
    let mut out = Vec::new();
    let st = generate_with_options(&data, &seeds, fe_decode::jumptab::FLENGINE, &fe_decode::Options::default(), &mut out).expect("generate");
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.bad_nodes, 0);
    // 29 = 17 + 11 scheduled + 0x4b58f68 (a standard-form switch reached through the table of 0x4b58c24), 5 rejected (measured)
    assert_eq!((st.msvc_tables, st.msvc_rejected.len()), (29, 5), "{:?}", st.msvc_rejected);
    assert!(c.contains("L_4b58f68: { /* 0x4b58f68 Jmp */ { uint64_t t = R["));
    // the stop of the 6-module run (`indirect call to 0x4b1674f is not a known function entry`): a switch with the 10 distinct targets,
    // and the loop head 0x4b16740 is a label
    let line = c.lines().find(|l| l.starts_with("L_4b1674d: ")).expect("site 0x4b1674d");
    assert!(line.contains("switch (t)") && !line.contains("fle_dispatch"), "{line}");
    assert_eq!(line.matches("case UINT64_C(").count(), 10, "{line}");
    assert!(line.contains("case UINT64_C(0x4b1674f): goto L_4b1674f;"), "{line}");
    assert!(c.contains("\nL_4b16740: "), "the head of the loop");
    // the five that stay rejected are still tail dispatches
    for va in [0x4b2f423u64, 0x4b2ffb4, 0x4b30485, 0x4b3dcfe, 0x4b434ce] {
        assert!(st.msvc_rejected.iter().any(|(v, _)| *v == va), "{va:#x}: {:?}", st.msvc_rejected);
        let line = c.lines().find(|l| l.starts_with(&format!("L_{va:x}: "))).unwrap_or_else(|| panic!("{va:#x}"));
        assert!(line.contains("fle_dispatch("), "{line}");
    }
}

const SCH_PAST_MAIN: &str = r#"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "boot.h"
extern uint64_t R[16];
void @PAST@(void);
static uint8_t stk[0x1000];

int main(int argc, char **argv)
{
    harness_image img;
    char err[128];
    if (argc < 3) return 2;
    if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    memset(R, 0, sizeof R);
    R[4] = (uint64_t)(uintptr_t)(stk + 0x800);
    R[0] = strtoull(argv[2], 0, 0);
    @PAST@();
    printf("returned %x\n", (uint32_t)R[0]);
    return 0;
}
"#;

#[test]
fn scheduled_an_index_above_n_past_the_guard_stops_in_fle_jumptab_miss() {
    // the generated `switch` is the one of the standard form (`Select::Target`); the guard never lets an index above N reach it, so it is
    // entered past the guard (a copy of the function that starts at the gap after the branch, rax = the index): 0..=N reach the table's target, an
    // index above N reads the entry after the table (another function's RVA) or zeros, and the jump register is no case
    // (the `lea B` is the first instruction of the gap, so the copy that starts at the gap has the base; no push/pop of B)
    let o = Sch { entry_lea: false, q: vec![SchQ::Lea, SchQ::Raw(CDQE.to_vec())], push_pop: false, ..Sch::simple() };
    let built = build_sched(AT, &o);
    let img = built.img.build();
    let mut out = Vec::new();
    let st = generate_with_specs(&img, &[], &[], &mut out).expect("generate");
    assert_eq!(st.msvc_tables, 1, "{:?}", st.msvc_rejected);
    let c = String::from_utf8(out).unwrap();
    let fname = format!("f_{:x}", fva());
    let past = format!("{fname}_past_guard");
    let gap = IB + built.labels["tbl"] as u64;
    let head = format!("void {fname}(void)\n{{\n");
    let body = &c[c.find(&head).expect("function")..];
    let func = &body[..body.find("\n}\n\n").unwrap() + 3];
    let copy = func.replacen(&head, &format!("void {past}(void)\n{{\ngoto L_{gap:x};\n"), 1);
    let dir = out_dir();
    let tag = etxt_common::uniq("sch-past");
    let pe = dir.join(format!("{tag}.pe"));
    std::fs::write(&pe, &img).unwrap();
    let lifted = dir.join(format!("{tag}-lift.c"));
    std::fs::write(&lifted, format!("{c}\nvoid {past}(void);\n{copy}")).unwrap();
    let m = dir.join(format!("{tag}-main.c"));
    std::fs::write(&m, SCH_PAST_MAIN.replace("@PAST@", &past)).unwrap();
    let exe = dir.join(format!("{tag}-exe"));
    clang(&["-o", exe.to_str().unwrap(), lifted.to_str().unwrap(), m.to_str().unwrap(), crate_dir().join("../../harness/image.c").to_str().unwrap()], &dir);
    let jmp = IB + built.labels["jmp"] as u64;
    for i in 0..=o.n as i64 {
        let r = Command::new(&exe).arg(&pe).arg(i.to_string()).output().unwrap();
        assert!(r.status.success(), "idx {i}: {}", String::from_utf8_lossy(&r.stderr));
        let want = swp_target_value(&built, i as usize);
        assert_eq!(String::from_utf8_lossy(&r.stdout).trim(), format!("returned {want:x}"), "idx {i}");
    }
    for i in [o.n as u64 + 1, o.n as u64 + 2, 0x100] {
        let r = Command::new(&exe).arg(&pe).arg(i.to_string()).output().unwrap();
        assert!(!r.status.success(), "idx {i} did not stop: {}", String::from_utf8_lossy(&r.stdout));
        let err = String::from_utf8_lossy(&r.stderr).to_string();
        assert!(err.contains("outside the table"), "idx {i}: {err}");
        assert!(err.contains(&format!("jump table at {jmp:#x}")), "idx {i}: {err}");
        assert!(String::from_utf8_lossy(&r.stdout).is_empty(), "idx {i} ran on");
    }
}
