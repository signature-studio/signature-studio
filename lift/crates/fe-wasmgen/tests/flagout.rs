//! Decision 0270 (mt3w): a caller that reads a flag after `call g` is compiled when `g` returns with that flag in a known state on every path that returns (`flagout.rs`). Tests under node with the hybrid layout (registers
//! in R[] at 0x2000, as the wrappers `f_<va>` read them):
//!   - G: `cmp rcx,rdx; jne F; ret; F: ud2` returns with ZF = 1 (the `jne` not taken; the other path stops). C: `test edi,edi; jne M; call G; M: je L; ...` merges that definition with the `test` of the path that jumps over the call
//!     (the shape of the stack-cookie check): C is compiled, and takes `je` exactly when edi = 0;
//!   - all five flags a callee defines (`xor eax,eax; ret`: ZF = PF = 1, CF = SF = OF = 0) are read by setcc after the call;
//!   - a callee that tail-jumps to a function that cannot return (`ud2`) has the summary of its other path;
//!   - what stays on the C path: a callee whose flags are not known (`cmp rcx,rdx; ret`) and a callee with two `ret`s that leave different ZF.
//! Skipped (and passing) when node is missing.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::{compile, Config};
use std::collections::HashMap;
use std::process::Command;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

/// a PE with functions at 0x401000 + `at` (each `(offset from 0x401000, code)`), listed in .pdata
fn pe(funcs: &[(u32, &[u8])]) -> Vec<u8> {
    let mut b = vec![0u8; 0xc00];
    let (lf, opt) = (0x80usize, 0x80 + 24);
    let sec = opt + 240;
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, lf as u32);
    w32(&mut b, lf, 0x4550);
    let f = lf + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    w16(&mut b, opt, 0x20b);
    w32(&mut b, opt + 16, 0x1000);
    w64(&mut b, opt + 24, 0x400000);
    w32(&mut b, opt + 32, 0x1000);
    w32(&mut b, opt + 36, 0x200);
    w32(&mut b, opt + 56, 0x3000);
    w32(&mut b, opt + 60, 0x400);
    w16(&mut b, opt + 68, 3);
    w32(&mut b, opt + 108, 16);
    w32(&mut b, opt + 112 + 3 * 8, 0x2000);
    w32(&mut b, opt + 112 + 3 * 8 + 4, 12 * funcs.len() as u32);
    let mut section = |i: usize, name: &[u8], vs: u32, rva: u32, rs: u32, rp: u32, ch: u32| {
        let s = sec + 40 * i;
        b[s..s + name.len()].copy_from_slice(name);
        w32(&mut b, s + 8, vs);
        w32(&mut b, s + 12, rva);
        w32(&mut b, s + 16, rs);
        w32(&mut b, s + 20, rp);
        w32(&mut b, s + 36, ch);
    };
    section(0, b".text", 0x400, 0x1000, 0x400, 0x400, 0x60000020);
    section(1, b".pdata", 0x200, 0x2000, 0x400, 0x800, 0x40000040);
    for (n, (off, code)) in funcs.iter().enumerate() {
        let at = 0x400 + *off as usize;
        b[at..at + code.len()].copy_from_slice(code);
        let p = 0x800 + 12 * n;
        w32(&mut b, p, 0x1000 + off);
        w32(&mut b, p + 4, 0x1000 + off + code.len() as u32);
        w32(&mut b, p + 8, 0x2100);
    }
    b[0x900..0x904].copy_from_slice(&[1, 0, 0, 0]);
    b
}

fn conf() -> Config {
    let mut ci: HashMap<String, String> = HashMap::new();
    ci.insert("dummy".into(), "fle_imp_dummy".into());
    Config {
        hybrid_r: Some(0x2000),
        tls_ctx: Some(0x2000),
        hybrid_x: Some(0x2200),
        hybrid_mx: Some((0x3000, 0x3100)),
        c_imports: Some(ci),
        mem_import: Some(("c".into(), "memory".into())),
        teb: Some(0x1000),
        ..Config::default()
    }
}

const JS: &str = r#"
const fs = require('fs');
const [wasmF, casesF, outF] = process.argv.slice(2);
const mem = new WebAssembly.Memory({ initial: 16, maximum: 32768, shared: true });
const dv = new DataView(mem.buffer);
const m = new WebAssembly.Module(fs.readFileSync(wasmF));
const R = 0x2000;
const imports = { c: { memory: mem, __tls_base: new WebAssembly.Global({ value: 'i32', mutable: true }, 0) } };
for (const i of WebAssembly.Module.imports(m)) {
  if (i.kind !== 'function') continue;
  imports[i.module] = imports[i.module] || {};
  imports[i.module][i.name] = () => { throw new Error('unexpected call of ' + i.module + '.' + i.name); };
}
const inst = new WebAssembly.Instance(m, imports);
const cases = JSON.parse(fs.readFileSync(casesF, 'utf8'));
const out = [];
for (const c of cases) {
  for (let r = 0; r < 16; r++) dv.setBigUint64(R + 8 * r, BigInt(c.regs && c.regs[r] !== undefined ? c.regs[r] : '0'), true);
  for (const [a, v] of (c.mem || [])) dv.setBigUint64(a, BigInt(v), true);
  inst.exports[c.fn]();
  const regs = []; for (let r = 0; r < 16; r++) regs.push(dv.getBigUint64(R + 8 * r, true).toString());
  out.push({ regs, mem: (c.read || []).map((a) => dv.getBigUint64(a, true).toString()) });
}
fs.writeFileSync(outF, JSON.stringify({ out }));
"#;

fn have(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn run_node(wasm: &[u8], cases: &str, tag: &str) -> Vec<Vec<u64>> {
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-flagout-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.wasm"), wasm).unwrap();
    std::fs::write(dir.join("cases.json"), cases).unwrap();
    std::fs::write(dir.join("run.cjs"), JS).unwrap();
    let o = Command::new("node").arg(dir.join("run.cjs")).arg(dir.join("t.wasm")).arg(dir.join("cases.json")).arg(dir.join("out.json")).output().unwrap();
    assert!(o.status.success(), "node: {}", String::from_utf8_lossy(&o.stderr));
    let txt = std::fs::read_to_string(dir.join("out.json")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    // each case: its 16 registers, then its `read` words
    txt.split("{\"regs\":[").skip(1).map(|c| {
        let re = c.find(']').unwrap();
        let mut v: Vec<u64> = c[..re].split(',').map(|x| x.trim_matches('"').parse().unwrap()).collect();
        let rest = &c[re..];
        let ms = &rest[rest.find("\"mem\":[").unwrap() + 7..];
        let me = ms.find(']').unwrap();
        v.extend(ms[..me].split(',').filter(|x| !x.is_empty()).map(|x| x.trim_matches('"').parse::<u64>().unwrap()));
        v
    }).collect()
}

fn lowered(funcs: &[(u32, &[u8])]) -> (fe_wasmgen::Compiled, Vec<u64>) {
    let keys: Vec<u64> = funcs.iter().map(|(o, _)| 0x401000 + *o as u64).collect();
    let comp = compile(&pe(funcs), &keys, &[], &conf()).unwrap_or_else(|e| panic!("compile: {e}"));
    (comp, keys)
}

fn assert_lowered(comp: &fe_wasmgen::Compiled, keys: &[u64]) {
    for k in keys {
        assert!(comp.fidx.contains_key(k), "{k:#x} was not lowered (it stays on the C path): {:?}", comp.report.rows.iter().find(|r| r.va == *k).map(|r| r.status.clone()));
    }
}

fn status(comp: &fe_wasmgen::Compiled, k: u64) -> String {
    comp.report.rows.iter().find(|r| r.va == k).map(|r| r.status.clone()).unwrap_or_default()
}

const RSP: u64 = 0x80000;

/// one call of `f_<va>` with rcx, rdx, rdi (the others 0)
fn case(va: u64, rcx: u64, rdx: u64, rdi: u64) -> String {
    format!("{{\"fn\":\"f_{va:x}\",\"regs\":{{\"1\":\"{rcx}\",\"2\":\"{rdx}\",\"4\":\"{RSP}\",\"7\":\"{rdi}\"}},\"read\":[]}}")
}

// G: cmp rcx,rdx; jne F; ret; F: ud2
const G: [u8; 8] = [0x48, 0x39, 0xd1, 0x75, 0x01, 0xc3, 0x0f, 0x0b];
// C: test edi,edi; jne M; call G; M: je L; mov eax,1; ret; L: mov eax,2; ret      (G at +0x100: the call is at 4, rel32 = 0x100 - 9)
const C: [u8; 23] = [0x85, 0xff, 0x75, 0x05, 0xe8, 0xf7, 0x00, 0x00, 0x00, 0x74, 0x06, 0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3, 0xb8, 0x02, 0x00, 0x00, 0x00, 0xc3];

#[test]
fn a_flag_read_after_a_call_of_a_function_that_returns_with_zf_set_merges_with_the_path_that_skips_the_call() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let (comp, keys) = lowered(&[(0, &C), (0x100, &G)]);
    assert_lowered(&comp, &keys);
    // edi = 0 (the low half; the upper half is set in the last two): the call happens (G stops unless rcx = rdx), ZF = 1 from G, `je` taken: eax = 2. edi != 0: the call is skipped, ZF = 0 from the test, `je` not taken: eax = 1
    let cases = format!("[{},{},{},{}]", case(0x401000, 9, 9, 0), case(0x401000, 1, 2, 5), case(0x401000, 1, 2, 0x1_0000_0003), case(0x401000, 4, 4, 0x1_0000_0000));
    let r = run_node(&comp.wasm, &cases, "cookie");
    assert_eq!(r[0][0], 2, "edi = 0: through G, ZF = 1");
    assert_eq!(r[1][0], 1, "edi = 5: the call is skipped, ZF = 0");
    assert_eq!(r[2][0], 1, "edi = 0x100000003: the low half is 3");
    assert_eq!(r[3][0], 2, "edi = 0x100000000: the low half is 0, through G");
}

fn rel(from_next: i32, to: i32) -> [u8; 4] {
    (to - from_next).to_le_bytes()
}

// D: xor eax,eax; ret          (ZF = 1, PF = 1, CF = SF = OF = 0)
const D: [u8; 3] = [0x31, 0xc0, 0xc3];
// R (D at +0x100): call D; setz cl; setb dl; setp bl; sets al; seto sil;
//                  movzx eax,al; movzx ecx,cl; movzx edx,dl; movzx ebx,bl; movzx esi,sil; lea ecx,[rcx+rdx*2]; lea ecx,[rcx+rbx*4]; lea ecx,[rcx+rax*8]; shl esi,4; lea eax,[rcx+rsi]; ret
fn r_bytes() -> Vec<u8> {
    let mut r = vec![0xe8];
    r.extend_from_slice(&rel(5, 0x100));
    r.extend_from_slice(&[0x0f, 0x94, 0xc1, 0x0f, 0x92, 0xc2, 0x0f, 0x9a, 0xc3, 0x0f, 0x98, 0xc0, 0x40, 0x0f, 0x90, 0xc6]);
    r.extend_from_slice(&[0x0f, 0xb6, 0xc0, 0x0f, 0xb6, 0xc9, 0x0f, 0xb6, 0xd2, 0x0f, 0xb6, 0xdb, 0x40, 0x0f, 0xb6, 0xf6]);
    r.extend_from_slice(&[0x8d, 0x0c, 0x51, 0x8d, 0x0c, 0x99, 0x8d, 0x0c, 0xc1, 0xc1, 0xe6, 0x04, 0x8d, 0x04, 0x31, 0xc3]);
    r
}

// S (D2 at +0x100, or D3 at +0x200 from S0 at +0x180): call D2; mov eax,5; adc eax,0; ret
fn s_bytes(from: i32, to: i32) -> Vec<u8> {
    let mut r = vec![0xe8];
    r.extend_from_slice(&rel(from + 5, to));
    r.extend_from_slice(&[0xb8, 0x05, 0x00, 0x00, 0x00, 0x83, 0xd0, 0x00, 0xc3]);
    r
}
// D2: cmp rcx,rdx; jae F; ret; F: ud2     (CF = 1 where it returns)       D3: cmp rcx,rdx; jb F; ret; F: ud2     (CF = 0 where it returns)
const D2: [u8; 8] = [0x48, 0x39, 0xd1, 0x73, 0x01, 0xc3, 0x0f, 0x0b];
const D3: [u8; 8] = [0x48, 0x39, 0xd1, 0x72, 0x01, 0xc3, 0x0f, 0x0b];

#[test]
fn all_five_flags_a_callee_defines_are_read_after_the_call() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let r = r_bytes();
    let (comp, keys) = lowered(&[(0, &r), (0x100, &D)]);
    assert_lowered(&comp, &keys);
    // after the callee's `xor eax,eax`: ZF = 1 (bit 0 of the result), CF = 0 (bit 1), PF = 1 (bit 2), SF = 0 (bit 3), OF = 0 (bit 4). rcx, rdx, rbx, rsi start with a pattern so that a setcc that is not executed shows
    let mut c = case(0x401000, 0xaaaa, 0, 0);
    c = c.replace("\"regs\":{", "\"regs\":{\"2\":\"43690\",\"3\":\"43690\",\"6\":\"43690\",");
    let out = run_node(&comp.wasm, &format!("[{c}]"), "five");
    assert_eq!(out[0][0], 0b00101, "ZF CF PF SF OF = 1 0 1 0 0");
}

#[test]
fn a_known_carry_is_the_input_of_adc_after_the_call() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let (set, clear) = (s_bytes(0, 0x100), s_bytes(0x180, 0x200));
    let (comp, keys) = lowered(&[(0, &set), (0x100, &D2), (0x180, &clear), (0x200, &D3)]);
    assert_lowered(&comp, &keys);
    let out = run_node(&comp.wasm, &format!("[{},{}]", case(0x401000, 1, 2, 0), case(0x401180, 2, 1, 0)), "adc");
    assert_eq!(out[0][0], 6, "the callee returns with CF = 1: adc eax,0 with eax = 5");
    assert_eq!(out[1][0], 5, "the callee returns with CF = 0");
}

// H: ud2          T: cmp rcx,rdx; jne J; ret; J: jmp H          (the failure path of the stack-cookie check is a jump to a handler that ends in a stop)
const H: [u8; 2] = [0x0f, 0x0b];
// T at 0x200: cmp rcx,rdx (48 39 d1); jne +1 (75 01); ret (c3); jmp H (e9 rel32: from 0x200+7 = 0x207, to 0x100 -> -0x107 + ... ) is computed below
fn t_bytes() -> Vec<u8> {
    let mut t = vec![0x48, 0x39, 0xd1, 0x75, 0x01, 0xc3, 0xe9];
    let rel = 0x100i32 - (0x200 + 11);
    t.extend_from_slice(&rel.to_le_bytes());
    t
}
// C2: test edi,edi; jne M; call T; M: je L; mov eax,1; ret; L: mov eax,2; ret       (T at +0x200: the rel32 of the call = 0x200 - 9)
fn c2_bytes() -> Vec<u8> {
    let mut c = C.to_vec();
    c[5..9].copy_from_slice(&(0x200i32 - 9).to_le_bytes());
    c
}

#[test]
fn a_tail_jump_to_a_function_that_cannot_return_leaves_the_other_exit_as_the_summary() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let c2 = c2_bytes();
    let t = t_bytes();
    let (comp, keys) = lowered(&[(0, &c2), (0x100, &H), (0x200, &t)]);
    assert_lowered(&comp, &keys);
    let cases = format!("[{},{}]", case(0x401000, 7, 7, 0), case(0x401000, 7, 8, 1));
    let r = run_node(&comp.wasm, &cases, "tail");
    assert_eq!(r[0][0], 2);
    assert_eq!(r[1][0], 1);
}

// X: cmp rcx,rdx; ret          Y: test rcx,rcx; jz A; cmp rdx,rdx; ret; A: cmp rdx,rcx?? no: A: or rdx,1; ret (ZF differs between the two ret: not known)
#[test]
fn a_callee_without_a_known_flag_keeps_its_caller_on_the_c_path() {
    // X: cmp rcx,rdx; ret
    let x: Vec<u8> = vec![0x48, 0x39, 0xd1, 0xc3];
    // Y: test rcx,rcx; jz A; xor eax,eax; ret; A: or edx,1; ret     (ZF = 1 at the first ret, 0 at the second)
    let y: Vec<u8> = vec![0x48, 0x85, 0xc9, 0x74, 0x03, 0x31, 0xc0, 0xc3, 0x83, 0xca, 0x01, 0xc3];
    for (name, g) in [("x", x), ("y", y)] {
        let mut c = C.to_vec();
        c[5..9].copy_from_slice(&(0x100i32 - 9).to_le_bytes());
        let (comp, _) = lowered(&[(0, &c), (0x100, &g)]);
        assert!(!comp.fidx.contains_key(&0x401000), "{name}: the caller reads ZF that the callee leaves unknown: it stays on the C path");
        assert!(status(&comp, 0x401000).contains("without recipe"), "{name}: {}", status(&comp, 0x401000));
    }
}
