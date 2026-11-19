//! `call [IAT slot]` in the generator output (decision 0077; src/indirect.rs, src/imports_table.rs).
//!
//! The synthetic PE imports kernel32.dll!SetLastError and kernel32.dll!GetLastError (rows of the response
//! table) and k.dll!Fn (no row). The entry function (0x1000) is
//!
//!   mov ecx, 77 ; call [SetLastError] ; call [GetLastError] ; add rax, 1 ;
//!   call thunk ; mov rax, [GetLastError slot] ; call rax ; add rax, 3 ; ret
//!
//! and the thunk (0x1040) is the existing `jmp [GetLastError slot]`. Generated C is built with clang, linked with
//! every runtime/fle_*.c, and run natively: the value must come out of the runtime function (77), the code after
//! each call must run (+1, +3), the stack pointer must be what it was before, and the return address of the last
//! call must be in the slot below the guest stack top (the same shape as a direct call and the jmp thunk).
//! The unanswered variant calls k.dll!Fn: it stays `fle_unsupported`, counted as before, and stops the run.

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const DATA_RVA: u32 = 0x2000;
const DATA_RAW: usize = 0x800;
const FILE_LEN: usize = 0x1000;

const SLOT_SET: u32 = 0x2200;
const SLOT_GET: u32 = 0x2208;
const SLOT_FN: u32 = 0x2220;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn data_at(rva: u32) -> usize {
    DATA_RAW + (rva - DATA_RVA) as usize
}
fn put(b: &mut [u8], rva: u32, bytes: &[u8]) {
    let o = data_at(rva);
    b[o..o + bytes.len()].copy_from_slice(bytes);
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

fn build(code: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 120, DATA_RVA);
    w32(&mut b, o + 124, 40);

    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let s1 = SEC + 40;
    b[s1..s1 + 5].copy_from_slice(b".data");
    w32(&mut b, s1 + 8, 0x800);
    w32(&mut b, s1 + 12, DATA_RVA);
    w32(&mut b, s1 + 16, 0x800);
    w32(&mut b, s1 + 20, DATA_RAW as u32);
    w32(&mut b, s1 + 36, 0xc000_0040);

    // Descriptor 1: kernel32.dll (OFT 0x2100, Name 0x2300, FirstThunk 0x2200) with SetLastError, GetLastError.
    // Descriptor 2: k.dll (OFT 0x2120, Name 0x2320, FirstThunk 0x2220) with Fn.
    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, SLOT_SET);
    w32(&mut b, d + 20, 0x2120);
    w32(&mut b, d + 32, 0x2320);
    w32(&mut b, d + 36, SLOT_FN);
    put(&mut b, 0x2300, b"kernel32.dll\0");
    put(&mut b, 0x2320, b"k.dll\0");
    put(&mut b, 0x2400, b"\0\0SetLastError\0");
    put(&mut b, 0x2430, b"\0\0GetLastError\0");
    put(&mut b, 0x2450, b"\0\0Fn\0");
    for (rva, v) in [(0x2100, 0x2400u64), (0x2108, 0x2430), (SLOT_SET, 0x2400), (SLOT_GET, 0x2430), (0x2120, 0x2450), (SLOT_FN, 0x2450)] {
        w64(&mut b, data_at(rva), v);
    }

    for (rva, bytes) in code {
        let o = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

fn call_slot(at: u32, slot: u32) -> (u32, Vec<u8>) {
    (at, [&[0xff, 0x15][..], &rel(at + 6, slot)].concat())
}

fn answered() -> Vec<u8> {
    build(&[
        (0x1000, vec![0xb9, 77, 0, 0, 0]),
        call_slot(0x1005, SLOT_SET),
        call_slot(0x100b, SLOT_GET),
        (0x1011, vec![0x48, 0x83, 0xc0, 0x01]),
        (0x1015, [&[0xe8][..], &rel(0x101a, 0x1040)].concat()),
        (0x101a, [&[0x48, 0x8b, 0x05][..], &rel(0x1021, SLOT_GET)].concat()),
        (0x1021, vec![0xff, 0xd0]),
        (0x1023, vec![0x48, 0x83, 0xc0, 0x03]),
        (0x1027, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, SLOT_GET)].concat()),
    ])
}

fn unanswered() -> Vec<u8> {
    build(&[call_slot(0x1000, SLOT_FN), (0x1006, vec![0xc3])])
}

fn gen(img: &[u8]) -> (fe_lift::Stats, String) {
    let mut out = Vec::new();
    let st = generate(img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

fn rt() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime")
}

fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-call-iat/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

/// Every runtime/fle_*.c compiled once per process (the response rows can name any `fle_imp_*`).
fn runtime_objs() -> &'static Vec<PathBuf> {
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

const MAIN: &str = r#"
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include "fle_runtime.h"

extern void f_140001000(void);
static uint64_t stk[16];

int main(void)
{
    /* The loaded image has the IAT at 0x140002200: the register form reads the slot with fle_ld. */
    void *p = mmap((void *)0x140002000, 0x1000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    if (p != (void *)0x140002000) { printf("mmap failed\n"); return 2; }
    *(uint64_t *)0x140002208 = 0x140002208;
    stk[8] = 0xdead0000ull;
    R[4] = (uint64_t)(uintptr_t)&stk[8];
    R[0] = 0xBAD;
    f_140001000();
    printf("rax %llu\n", (unsigned long long)R[0]);
    printf("rsp_kept %d\n", R[4] == (uint64_t)(uintptr_t)&stk[8]);
    printf("ret_slot %llx\n", (unsigned long long)stk[7]);
    printf("guest_ret_untouched %llx\n", (unsigned long long)stk[8]);
    printf("returned\n");
    return 0;
}
"#;

struct Run {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn run(name: &str, c: &str) -> Run {
    let dir = work();
    let tag = etxt_common::uniq(name);
    let gen_c = dir.join(format!("{tag}.c"));
    let main_c = dir.join(format!("{tag}-main.c"));
    let exe = dir.join(&tag);
    std::fs::write(&gen_c, c).unwrap();
    std::fs::write(&main_c, MAIN).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-no-pie", "-I"]).arg(rt());
    for o in runtime_objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg(&gen_c).arg(&main_c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    Run {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

#[test]
fn call_through_iat_slot_uses_the_response_row_and_keeps_the_call_shape() {
    let (st, c) = gen(&answered());

    // Memory form (SetLastError, GetLastError) and register form (`mov rax,[slot]; call rax`, GetLastError).
    assert_eq!(st.handled["call_imp:SetLastError"], 1);
    assert_eq!(st.handled["call_imp:GetLastError"], 2);
    // The existing thunk form is unchanged.
    assert_eq!(st.handled["jmp_imp:GetLastError"], 1);
    assert!(!st.unsupported.contains_key("Call: indirect through IAT slot (not lifted)"));
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);

    // The same runtime function as the thunk, entered as every call is: push the return VA, call, pop.
    for (f, next) in [("SetLastError", 0x100b), ("GetLastError", 0x1011)] {
        let body = format!("{{ void fle_imp_{f}(void); R[4] -= 8; fle_st(R[4], 8, UINT64_C({:#x})); fle_imp_{f}(); R[4] += 8; }}", IB + next);
        assert!(c.contains(&body), "{f}: {body}");
    }
    assert!(c.contains("fle_imp_GetLastError(); return;"));

    let r = run("call-iat-answered", &c);
    assert!(r.ok, "signal {:?}: {}\n{}", r.signal, r.stderr, r.stdout);
    assert_eq!(
        r.stdout.lines().collect::<Vec<_>>(),
        [
            // SetLastError(77), GetLastError() = 77, +1, thunk GetLastError() = 77, register call = 77, +3.
            "rax 80".to_string(),
            "rsp_kept 1".to_string(),
            // the last call pushed the VA after `call rax` (0x1023), and ret does not pop: the slot keeps it.
            format!("ret_slot {:x}", IB + 0x1023),
            "guest_ret_untouched dead0000".to_string(),
            "returned".to_string(),
        ]
    );
}

#[test]
fn call_through_iat_slot_without_a_row_stays_unsupported_and_stops_the_run() {
    let (st, c) = gen(&unanswered());
    assert_eq!(st.unsupported["Call: indirect through IAT slot (not lifted)"], 1);
    assert_eq!(st.unsupported_total, 1);
    assert!(!st.handled.keys().any(|k| k.starts_with("call_imp:")));
    assert!(c.contains(&format!("L_{:x}: {{ /* {:#x} Call */ fle_unsupported(UINT64_C({:#x})); }}", IB + 0x1000, IB + 0x1000, IB + 0x1000)));

    let r = run("call-iat-unanswered", &c);
    assert!(!r.ok && !r.stdout.contains("returned"), "{}", r.stdout);
    assert_eq!(r.signal, Some(6));
    assert!(r.stderr.contains(&format!("fle: unsupported instruction at {:#x}", IB + 0x1000)), "{}", r.stderr);
}

#[test]
fn indirect_calls_outside_the_iat_are_unchanged() {
    // `call rbx` (unresolved) keeps the dispatch form; no import handler is involved.
    let img = build(&[(0x1000, vec![0xff, 0xd3]), (0x1002, vec![0xc3])]);
    let (st, c) = gen(&img);
    assert_eq!(st.handled["call_ind:dispatch"], 1);
    assert!(!st.handled.keys().any(|k| k.starts_with("call_imp:")));
    assert!(c.contains("fle_dispatch(t);"));
}
