//! Decision 0264 (mt3z, the E7b stop at start): the wrapper `f_<va>` of a function that leaves RSP above its entry value (a shared epilogue: `add rsp,N; pop rbx; ret`) must leave the callee-saved registers
//! that the epilogue restores as the epilogue restored them. The wrapper used to put back the callee-saved registers of the C caller from the values it read at entry, which is right for a function
//! that preserves them and wrong for a continuation entry that pops them from the frame of the function that jumped to it. Skipped (and passing) when node is missing.
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

fn conf(only: &[u64]) -> Config {
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
        hybrid_only: Some(only.iter().copied().collect()),
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
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-rspb-{}-{}", std::process::id(), tag));
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

/// `funcs[k]` for k in `direct` are lowered, the others stay on the C path (they are seeds of the closure, so that the direct ones are called from the C path by a tail jmp)
fn lowered(funcs: &[(u32, &[u8])], direct: &[usize]) -> (fe_wasmgen::Compiled, Vec<u64>) {
    let all: Vec<u64> = funcs.iter().map(|(o, _)| 0x401000 + *o as u64).collect();
    let only: Vec<u64> = direct.iter().map(|&k| all[k]).collect();
    let comp = compile(&pe(funcs), &all, &[], &conf(&only)).unwrap_or_else(|e| panic!("compile: {e}"));
    (comp, only)
}


// E: mov rcx,[rsp+8]; add rax,rcx; add rsp,0x20; pop rbx; ret
const E: [u8; 15] = [0x48, 0x8b, 0x4c, 0x24, 0x08, 0x48, 0x01, 0xc8, 0x48, 0x83, 0xc4, 0x20, 0x5b, 0xc3, 0x90];
// E2: add rsp,0x30; pop rdi; pop rsi; pop r12; pop r13; pop rbp; ret   (the whole epilogue of a function with five saved registers)
const E2: [u8; 14] = [0x48, 0x83, 0xc4, 0x30, 0x5f, 0x5e, 0x41, 0x5c, 0x41, 0x5d, 0x5d, 0xc3, 0x90, 0x90];


#[test]
fn a_shared_epilogue_entered_from_the_c_side_leaves_the_registers_that_it_pops() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    // P1: jmp E (rel32 from 0x401205 to 0x401000); P2: jmp E2 (to 0x401100): the C-path functions that jump into the shared epilogues
    let p1: [u8; 5] = [0xe9, 0xfb, 0xfd, 0xff, 0xff];
    let p2: [u8; 5] = [0xe9, 0xfb, 0xfd, 0xff, 0xff];
    let (comp, keys) = lowered(&[(0, &E), (0x100, &E2), (0x200, &p1), (0x300, &p2)], &[0, 1]);
    for k in &keys {
        assert!(comp.fidx.contains_key(k), "{k:#x} was not lowered: {:?}", comp.report.rows.iter().find(|r| r.va == *k).map(|r| r.status.clone()));
    }
    let rsp = 0x80000u64;
    // registers: rax 0, rcx 1, rbx 3, rsp 4, rbp 5, rsi 6, rdi 7, r12 12, r13 13
    // E entered with another function's frame: [rsp+8] = 100, the slot that `pop rbx` reads is [rsp+0x20] = 0x2222; rbx holds 0x1111 (the jumping function's temporary)
    // E2: the five saved values at rsp+0x30 .. rsp+0x50 are 0xa1.. and the registers hold the jumping function's temporaries 0xb1..
    let cases = format!(
        "[{{\"fn\":\"f_401000\",\"regs\":{{\"0\":\"1\",\"3\":\"4369\",\"4\":\"{rsp}\"}},\"mem\":[[{},100],[{},8738]],\"read\":[]}},\
          {{\"fn\":\"f_401100\",\"regs\":{{\"3\":\"4369\",\"4\":\"{rsp}\",\"5\":\"177\",\"6\":\"178\",\"7\":\"179\",\"12\":\"180\",\"13\":\"181\"}},\"mem\":[[{},161],[{},162],[{},163],[{},164],[{},165]],\"read\":[]}}]",
        rsp + 8, rsp + 0x20, rsp + 0x30, rsp + 0x38, rsp + 0x40, rsp + 0x48, rsp + 0x50
    );
    let r = run_node(&comp.wasm, &cases, "boundary");
    assert_eq!(r[0][0], 101, "E: rax");
    assert_eq!(r[0][4], rsp + 0x28, "E: R[4]");
    assert_eq!(r[0][3], 0x2222, "E: rbx is what `pop rbx` loaded from the frame, not the value it had at entry");
    // E2 pops rdi, rsi, r12, r13, rbp in that order from rsp+0x30
    assert_eq!(r[1][7], 161, "E2: rdi");
    assert_eq!(r[1][6], 162, "E2: rsi");
    assert_eq!(r[1][12], 163, "E2: r12");
    assert_eq!(r[1][13], 164, "E2: r13");
    assert_eq!(r[1][5], 165, "E2: rbp");
    assert_eq!(r[1][4], rsp + 0x58, "E2: R[4]");
}

// E3: the epilogue of a function with six saved registers that also leaves rsi modified (a plain write, no pop) and does not touch rbp:
//     mov rcx,[rsp+0x10]; lea rsi,[rcx+4]; add rsp,0x38; pop r15; pop r14; pop r13; pop r12; pop rdi; pop rbx; ret
const E3: [u8; 28] = [
    0x48, 0x8b, 0x4c, 0x24, 0x10, 0x48, 0x8d, 0x71, 0x04, 0x48, 0x83, 0xc4, 0x38, 0x41, 0x5f, 0x41, 0x5e, 0x41, 0x5d, 0x41, 0x5c, 0x5f, 0x5b, 0xc3, 0x90, 0x90, 0x90, 0x90,
];
// E4: a continuation with a branch: test rcx,rcx; je L; add rsp,0x20; pop rbx; pop rsi; ret; L: add rsp,0x30; pop rdi; pop r12; pop rbx; ret   (two different epilogues, k = 0x30 and 0x40: the exits disagree, so it stays on the C path)
const E4: [u8; 27] = [
    0x48, 0x85, 0xc9, 0x74, 0x08, 0x48, 0x83, 0xc4, 0x20, 0x5b, 0x5e, 0xc3, 0x90, 0x48, 0x83, 0xc4, 0x30, 0x5f, 0x41, 0x5c, 0x5b, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90,
];

#[test]
fn a_continuation_that_pops_six_registers_and_writes_one_more_leaves_all_of_them_as_the_c_version_does() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let p: [u8; 5] = [0xe9, 0xfb, 0xfd, 0xff, 0xff];
    let (comp, keys) = lowered(&[(0, &E3), (0x200, &p)], &[0]);
    for k in &keys {
        assert!(comp.fidx.contains_key(k), "{k:#x} was not lowered: {:?}", comp.report.rows.iter().find(|r| r.va == *k).map(|r| r.status.clone()));
    }
    let rsp = 0x80000u64;
    // the frame: [rsp+0x10] = 100 (read by the mov), the six saved values at rsp+0x38 .. rsp+0x60 are 0xc1..0xc6 (r15, r14, r13, r12, rdi, rbx)
    // the registers hold the jumping function's temporaries: rbx 0x1, rdi 0x2, r12 0x3, r13 0x4, r14 0x5, r15 0x6, rsi 0x7, rbp 0x8
    let cases = format!(
        "[{{\"fn\":\"f_401000\",\"regs\":{{\"3\":\"1\",\"7\":\"2\",\"12\":\"3\",\"13\":\"4\",\"14\":\"5\",\"15\":\"6\",\"6\":\"7\",\"5\":\"8\",\"4\":\"{rsp}\"}},\"mem\":[[{},100],[{},193],[{},194],[{},195],[{},196],[{},197],[{},198]],\"read\":[]}}]",
        rsp + 0x10, rsp + 0x38, rsp + 0x40, rsp + 0x48, rsp + 0x50, rsp + 0x58, rsp + 0x60
    );
    let r = run_node(&comp.wasm, &cases, "e3");
    assert_eq!(r[0][15], 193, "r15");
    assert_eq!(r[0][14], 194, "r14");
    assert_eq!(r[0][13], 195, "r13");
    assert_eq!(r[0][12], 196, "r12");
    assert_eq!(r[0][7], 197, "rdi");
    assert_eq!(r[0][3], 198, "rbx");
    assert_eq!(r[0][6], 104, "rsi: written by the function (lea rsi,[rcx+4]), not put back");
    assert_eq!(r[0][5], 8, "rbp: untouched by the function, still the entry value");
    assert_eq!(r[0][4], rsp + 0x68, "R[4]");
}

#[test]
fn a_continuation_whose_exits_pop_different_amounts_stays_on_the_c_path() {
    let p: [u8; 5] = [0xe9, 0xfb, 0xfd, 0xff, 0xff];
    let (comp, keys) = lowered(&[(0, &E4), (0x200, &p)], &[0]);
    let st = comp.report.rows.iter().find(|r| r.va == keys[0]).map(|r| r.status.clone()).unwrap_or_default();
    assert!(!comp.fidx.contains_key(&keys[0]), "lowered although the exits disagree: {st}");
    assert!(st.starts_with("RSP at a return"), "{st}");
}

#[test]
fn a_continuation_that_no_c_function_of_the_module_jumps_to_still_returns_its_registers() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    // E alone (no parent in the module: it is entered through a dispatch or a jump the module does not show)
    let (comp, keys) = lowered(&[(0, &E)], &[0]);
    assert!(comp.fidx.contains_key(&keys[0]), "{:?}", comp.report.rows.iter().find(|r| r.va == keys[0]).map(|r| r.status.clone()));
    let rsp = 0x80000u64;
    let cases = format!(
        "[{{\"fn\":\"f_401000\",\"regs\":{{\"0\":\"1\",\"3\":\"4369\",\"4\":\"{rsp}\"}},\"mem\":[[{},100],[{},8738]],\"read\":[]}}]",
        rsp + 8, rsp + 0x20
    );
    let r = run_node(&comp.wasm, &cases, "alone");
    assert_eq!(r[0][0], 101);
    assert_eq!(r[0][3], 0x2222, "rbx is what `pop rbx` loaded");
    assert_eq!(r[0][4], rsp + 0x28);
}
