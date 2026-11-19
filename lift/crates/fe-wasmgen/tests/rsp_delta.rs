//! Decision 0261 (mt3y): a function that leaves RSP above its entry value by a constant k (a shared epilogue `add rsp, N; pop ...; ret` that another function reaches with `jmp`, with its own frame still allocated) is compiled,
//! and the function that jumps to it is balanced by the sum: -frame + k = 0. Tests under node with the hybrid layout (registers in R[] at 0x2000, as the wrappers `f_<va>` read them):
//!   - P (push rbx; sub rsp,0x20; ...; jmp E) and E (...; add rsp,0x20; pop rbx; ret) are both compiled; P's wrapper leaves R[4] at its entry value, E's wrapper leaves it at entry + 0x28 (the lifted C's `ret` does not pop: the
//!     call site adds the 8 afterwards), the stack slots P writes are the ones E reads, and rbx comes back from the slot;
//!   - a caller G of a `ret 16` function F (stdcall: the callee pops two pushed arguments) is balanced and computes with the right stack;
//!   - what stays on the C path: a function whose RSP is not a constant (`sub rsp, rax`), and a function whose exits disagree.
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
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-rspd-{}-{}", std::process::id(), tag));
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

// P: push rbx; sub rsp,0x20; mov rbx,rdx; mov [rsp+8],rcx; lea rax,[rdx+1]; jmp E        E: mov rcx,[rsp+8]; add rax,rcx; add rsp,0x20; pop rbx; ret
const P: [u8; 22] = [0x53, 0x48, 0x83, 0xec, 0x20, 0x48, 0x89, 0xd3, 0x48, 0x89, 0x4c, 0x24, 0x08, 0x48, 0x8d, 0x42, 0x01, 0xe9, 0xea, 0x00, 0x00, 0x00];
const E: [u8; 15] = [0x48, 0x8b, 0x4c, 0x24, 0x08, 0x48, 0x01, 0xc8, 0x48, 0x83, 0xc4, 0x20, 0x5b, 0xc3, 0x90];

#[test]
fn a_jmp_into_a_shared_epilogue_balances_the_jumping_function_and_both_are_compiled() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let (comp, keys) = lowered(&[(0, &P), (0x100, &E)]);
    for k in &keys {
        assert!(comp.fidx.contains_key(k), "{k:#x} was not lowered (it stays on the C path): {:?}", comp.report.rows.iter().find(|r| r.va == *k).map(|r| r.status.clone()));
    }
    let rsp = 0x80000u64;
    // regs: rax = 0 (index 0), rcx = 5 (1), rdx = 7 (2), rbx = 0x1111 (3), rsp (4)
    let cases = format!(
        "[{{\"fn\":\"f_401000\",\"regs\":{{\"0\":\"0\",\"1\":\"5\",\"2\":\"7\",\"3\":\"{}\",\"4\":\"{rsp}\"}},\"read\":[{}]}},\
          {{\"fn\":\"f_401100\",\"regs\":{{\"0\":\"1\",\"3\":\"4369\",\"4\":\"{rsp}\"}},\"mem\":[[{},100],[{},8738]],\"read\":[]}}]",
        0x1111, rsp - 0x28 + 8, rsp + 8, rsp + 0x20
    );
    let r = run_node(&comp.wasm, &cases, "pair");
    // P: rax = (rdx + 1) + rcx = 13; rbx restored from the slot P wrote (0x1111); R[4] at the entry value; the slot [entry - 0x28 + 8] holds rcx = 5
    assert_eq!(r[0][0], 13, "P: rax");
    assert_eq!(r[0][3], 0x1111, "P: rbx restored by the epilogue");
    assert_eq!(r[0][4], rsp, "P: R[4] at the entry value");
    assert_eq!(r[0][16], 5, "P: the slot that E reads");
    // E alone, entered with another function's frame: rax = 1 + [rsp+8]; R[4] = entry + 0x28 (the lifted C's ret does not pop; the call site adds 8). (rbx is not checked here: nothing in this module reads rbx after E, so E's signature does not return it)
    assert_eq!(r[1][0], 101, "E: rax");
    assert_eq!(r[1][4], rsp + 0x28, "E: R[4] after the epilogue");
}

// G: sub rsp,0x28; mov rax,rdx; push rax; mov rax,rcx; push rax; call F; mov qword [rsp+8],0x77; add rsp,0x28; ret        F: mov rax,[rsp+8]; add rax,[rsp+16]; ret 16
// (the arguments are pushed from rax, not straight from rcx / rdx: a `push` of a register that still holds its entry value is taken for a save of that register and left out when nothing restores it)
const G: [u8; 31] = [0x48, 0x83, 0xec, 0x28, 0x48, 0x89, 0xd0, 0x50, 0x48, 0x89, 0xc8, 0x50, 0xe8, 0xef, 0x00, 0x00, 0x00, 0x48, 0xc7, 0x44, 0x24, 0x08, 0x77, 0x00, 0x00, 0x00, 0x48, 0x83, 0xc4, 0x28, 0xc3];
const F: [u8; 11] = [0x48, 0x8b, 0x44, 0x24, 0x08, 0x48, 0x03, 0x44, 0x24, 0x10, 0xc2];

#[test]
fn a_callee_that_pops_its_arguments_moves_the_callers_rsp_by_k() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    // F ends with `ret 16` (c2 10 00)
    let mut f = F.to_vec();
    f.extend_from_slice(&[0x10, 0x00]);
    let (comp, keys) = lowered(&[(0, &G), (0x100, &f)]);
    for k in &keys {
        assert!(comp.fidx.contains_key(k), "{k:#x} was not lowered (it stays on the C path): {:?}", comp.report.rows.iter().find(|r| r.va == *k).map(|r| r.status.clone()));
    }
    let rsp = 0x80000u64;
    let cases = format!("[{{\"fn\":\"f_401000\",\"regs\":{{\"1\":\"30\",\"2\":\"12\",\"4\":\"{rsp}\"}},\"read\":[{}]}},{{\"fn\":\"f_401100\",\"regs\":{{\"4\":\"{rsp}\"}},\"mem\":[[{},1],[{},2]],\"read\":[]}}]", rsp - 0x20, rsp + 8, rsp + 16);
    let r = run_node(&comp.wasm, &cases, "stdcall");
    assert_eq!(r[0][0], 42, "G: rax = rcx + rdx through the callee that pops its arguments");
    assert_eq!(r[0][4], rsp, "G: R[4] at the entry value");
    assert_eq!(r[0][16], 0x77, "G: the store [rsp+8] after the call is at entry - 0x20, which needs the caller's RSP moved by the callee's 16");
    assert_eq!(r[1][0], 3, "F alone");
    assert_eq!(r[1][4], rsp + 16, "F: R[4] after `ret 16` (the call site adds the 8)");
}

#[test]
fn a_variable_rsp_or_exits_that_disagree_stay_on_the_c_path() {
    // sub rsp, rax; add rsp, rax; ret          (the offset is not a constant)
    let variable: Vec<u8> = vec![0x48, 0x29, 0xc4, 0x48, 0x01, 0xc4, 0xc3];
    // test rcx,rcx; jz L; ret; L: push rbx; pop rbx?? no: L: add rsp,8; ret      (two returns at different offsets)
    let disagree: Vec<u8> = vec![0x48, 0x85, 0xc9, 0x74, 0x01, 0xc3, 0x48, 0x83, 0xc4, 0x08, 0xc3];
    for (name, code) in [("variable", variable), ("disagree", disagree)] {
        let comp = compile(&pe(&[(0, &code)]), &[0x401000], &[], &conf()).unwrap();
        assert!(!comp.fidx.contains_key(&0x401000), "{name}: the function stays on the C path");
    }
}

// H: mov eax,0x1020; call CS; sub rsp,rax; mov [rsp+8],rcx; mov rax,[rsp+8]; add rsp,0x1020; ret        CS (the stack probe __chkstk: preserves rax, balanced): ret
const H: [u8; 31] = [0xb8, 0x20, 0x10, 0x00, 0x00, 0xe8, 0xf6, 0x00, 0x00, 0x00, 0x48, 0x29, 0xc4, 0x48, 0x89, 0x4c, 0x24, 0x08, 0x48, 0x8b, 0x44, 0x24, 0x08, 0x48, 0x81, 0xc4, 0x20, 0x10, 0x00, 0x00, 0xc3];

#[test]
fn sub_rsp_rax_after_mov_eax_imm_and_a_stack_probe_is_a_constant_frame() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let (comp, keys) = lowered(&[(0, &H), (0x100, &[0xc3])]);
    for k in &keys {
        assert!(comp.fidx.contains_key(k), "{k:#x} was not lowered (it stays on the C path): {:?}", comp.report.rows.iter().find(|r| r.va == *k).map(|r| r.status.clone()));
    }
    let rsp = 0x80000u64;
    let cases = format!("[{{\"fn\":\"f_401000\",\"regs\":{{\"1\":\"4660\",\"4\":\"{rsp}\"}},\"read\":[{}]}}]", rsp - 0x1020 + 8);
    let r = run_node(&comp.wasm, &cases, "chkstk");
    assert_eq!(r[0][0], 0x1234, "rax read back from the slot in the 0x1020-byte frame");
    assert_eq!(r[0][4], rsp, "R[4] at the entry value");
    assert_eq!(r[0][16], 0x1234, "the slot is at entry - 0x1020 + 8");
}

#[test]
fn a_register_that_is_not_a_known_constant_still_makes_rsp_variable() {
    // mov rax, rcx; sub rsp, rax; add rsp, rax; ret   (rax is rcx: not a constant)
    let code: Vec<u8> = vec![0x48, 0x89, 0xc8, 0x48, 0x29, 0xc4, 0x48, 0x01, 0xc4, 0xc3];
    let comp = compile(&pe(&[(0, &code)]), &[0x401000], &[], &conf()).unwrap();
    assert!(!comp.fidx.contains_key(&0x401000), "the function stays on the C path");
}

// P2: test rcx,rcx; jz L; push rbx; sub rsp,0x20; mov [rsp+8],rcx; lea rax,[rdx+5]; jmp E; L: xor eax,eax; ret     (one exit is a `ret` at 0, the other is the jmp into E at -0x28 + 0x28: they agree)
const P2: [u8; 27] = [0x48, 0x85, 0xc9, 0x74, 0x13, 0x53, 0x48, 0x83, 0xec, 0x20, 0x48, 0x89, 0x4c, 0x24, 0x08, 0x48, 0x8d, 0x42, 0x05, 0xe9, 0xe8, 0x00, 0x00, 0x00, 0x31, 0xc0, 0xc3];

#[test]
fn a_function_with_a_plain_ret_and_a_jmp_into_the_shared_epilogue_is_balanced() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let (comp, keys) = lowered(&[(0, &P2), (0x100, &E)]);
    for k in &keys {
        assert!(comp.fidx.contains_key(k), "{k:#x} was not lowered (it stays on the C path): {:?}", comp.report.rows.iter().find(|r| r.va == *k).map(|r| r.status.clone()));
    }
    let rsp = 0x80000u64;
    let case = |rcx: u64| format!("{{\"fn\":\"f_401000\",\"regs\":{{\"0\":\"99\",\"1\":\"{rcx}\",\"2\":\"7\",\"3\":\"4369\",\"4\":\"{rsp}\"}},\"read\":[]}}");
    let r = run_node(&comp.wasm, &format!("[{},{}]", case(0), case(5)), "p2");
    assert_eq!(r[0][0], 0, "rcx = 0: the plain ret path, eax = 0");
    assert_eq!(r[0][4], rsp, "rcx = 0: R[4] at the entry value");
    assert_eq!(r[1][0], 17, "rcx = 5: (rdx + 5) + rcx through the epilogue");
    assert_eq!(r[1][3], 4369, "rcx = 5: rbx comes back");
    assert_eq!(r[1][4], rsp, "rcx = 5: R[4] at the entry value");
}
