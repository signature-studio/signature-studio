//! Decision 0270 (mt3w): the C <-> direct wasm boundary after a return, in the shape of the functions that 0270 turns into direct wasm (the mixer functions of Elastique: callee-saved registers pushed, xmm6/xmm7 saved to the
//! stack and restored, a stack-cookie check at the end whose `je` reads the ZF that the callee returns). F is called through its wrapper `f_<va>` (as the C path does) with sentinels in every callee-saved register (rbx rbp
//! rsi rdi r12-r15), in xmm6-xmm15 (X[]), in R[4] and in the flag bytes (FL); after the return all of them are compared, and rax is the branch F took.
//!   (1) F direct, the cookie function G on the C path (a JS function: it leaves ZF = 1 in FL as the lifted C does): the caller does not read FL, it uses the callee's summary. The same with a G that writes the wrong ZF to FL: the caller must not depend on FL;
//!   (2) F and G both direct: the registers and R[4] come back, X[6..15] come back, FL is exactly as it was (a direct function does not write FL);
//!   (3) the wrapper of a direct function does not carry flags to its caller on the C path: FL after `f_<G>` is what it was before (the invariant that 0270's flag summary relies on: no C-path code reads flags after calling a direct function;
//!       `lift/out-mt3y`-style scan `flaglive` found no such call site in the 8 modules, see decision 0270).
//! Skipped (and passing) when node is missing.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::{compile, Config};
use std::collections::{HashMap, HashSet};
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
        hybrid_only: Some(only.iter().copied().collect::<HashSet<u64>>()),
        ..Config::default()
    }
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn rel(from_next: usize, to: usize) -> [u8; 4] {
    (to as i32 - from_next as i32).to_le_bytes()
}

// G: cmp rcx,rdx; jne F2; ret; F2: ud2            (the cookie check: returns with ZF = 1 or does not return)
const G: [u8; 8] = [0x48, 0x39, 0xd1, 0x75, 0x01, 0xc3, 0x0f, 0x0b];

/// F (G at `g`):
///   push rbx; push rsi; push rdi; sub rsp,0x40; movaps [rsp+0x20],xmm6; movaps [rsp+0x30],xmm7
///   mov rbx,rcx; mov rsi,rdx; movq xmm6,rbx; movq xmm7,rsi; lea rdi,[rbx+rsi]
///   test edi,edi; jne M; call G; M: je L; mov eax,1; jmp D; L: mov eax,2; D:
///   movaps xmm6,[rsp+0x20]; movaps xmm7,[rsp+0x30]; add rsp,0x40; pop rdi; pop rsi; pop rbx; ret
fn code_f(g: usize) -> Vec<u8> {
    let mut c: Vec<u8> = vec![0x53, 0x56, 0x57, 0x48, 0x83, 0xec, 0x40, 0x0f, 0x29, 0x74, 0x24, 0x20, 0x0f, 0x29, 0x7c, 0x24, 0x30];
    c.extend_from_slice(&[0x48, 0x89, 0xcb, 0x48, 0x89, 0xd6, 0x66, 0x48, 0x0f, 0x6e, 0xf3, 0x66, 0x48, 0x0f, 0x6e, 0xfe, 0x48, 0x8d, 0x3c, 0x33]);
    c.extend_from_slice(&[0x85, 0xff, 0x75, 0x05, 0xe8]);
    let n = c.len();
    c.extend_from_slice(&rel(n + 4, g));
    // M: je L (L is after mov eax,1 (5 bytes) and jmp D (2 bytes))
    c.extend_from_slice(&[0x74, 0x07, 0xb8, 0x01, 0x00, 0x00, 0x00, 0xeb, 0x05, 0xb8, 0x02, 0x00, 0x00, 0x00]);
    c.extend_from_slice(&[0x0f, 0x28, 0x74, 0x24, 0x20, 0x0f, 0x28, 0x7c, 0x24, 0x30, 0x48, 0x83, 0xc4, 0x40, 0x5f, 0x5e, 0x5b, 0xc3]);
    c
}

const JS: &str = r#"
const fs = require('fs');
const [wasmF, casesF, outF, gva, mode] = process.argv.slice(2);
const mem = new WebAssembly.Memory({ initial: 16, maximum: 32768, shared: true });
const dv = new DataView(mem.buffer);
const m = new WebAssembly.Module(fs.readFileSync(wasmF));
const R = 0x2000, FL = 0x2100, X = 0x2200;
const imports = { c: { memory: mem, __tls_base: new WebAssembly.Global({ value: 'i32', mutable: true }, 0) } };
let gcalls = 0;
for (const i of WebAssembly.Module.imports(m)) {
  if (i.kind !== 'function') continue;
  imports[i.module] = imports[i.module] || {};
  imports[i.module][i.name] = () => { throw new Error('unexpected call of ' + i.module + '.' + i.name); };
}
// the C path's G (cmp rcx,rdx; jne ud2; ret): equal -> the flags of the lifted C (ZF = 1, CF = SF = OF = 0, PF = 1, AF = 0); mode 'lie' writes ZF = 0 instead
if (mode !== 'none') imports.c['f_' + gva] = () => {
  gcalls++;
  if (dv.getBigUint64(R + 8 * 1, true) !== dv.getBigUint64(R + 8 * 2, true)) throw new Error('G stops (ud2)');
  const z = mode === 'lie' ? 0 : 1;
  dv.setUint8(FL + 0, 0); dv.setUint8(FL + 1, 1); dv.setUint8(FL + 2, 0); dv.setUint8(FL + 3, z); dv.setUint8(FL + 4, 0); dv.setUint8(FL + 5, 0);
};
const inst = new WebAssembly.Instance(m, imports);
const cases = JSON.parse(fs.readFileSync(casesF, 'utf8'));
const out = [];
const CS = [3, 5, 6, 7, 12, 13, 14, 15];
for (const c of cases) {
  for (let r = 0; r < 16; r++) dv.setBigUint64(R + 8 * r, BigInt(c.regs[r] !== undefined ? c.regs[r] : '0'), true);
  for (let i = 0; i < 16; i++) { dv.setBigUint64(X + 16 * i, BigInt('0xaaaa0000' + i.toString(16).padStart(4, '0')), true); dv.setBigUint64(X + 16 * i + 8, BigInt('0xbbbb0000' + i.toString(16).padStart(4, '0')), true); }
  for (let b = 0; b < 8; b++) dv.setUint8(FL + b, 0x40 + b);
  gcalls = 0;
  inst.exports[c.fn]();
  const regs = []; for (let r = 0; r < 16; r++) regs.push(dv.getBigUint64(R + 8 * r, true).toString());
  const x = []; for (let i = 0; i < 16; i++) { x.push(dv.getBigUint64(X + 16 * i, true).toString()); x.push(dv.getBigUint64(X + 16 * i + 8, true).toString()); }
  const fl = []; for (let b = 0; b < 8; b++) fl.push(dv.getUint8(FL + b));
  out.push({ regs, x, fl, gcalls });
}
fs.writeFileSync(outF, JSON.stringify(out));
"#;

struct Ret {
    regs: Vec<u64>,
    x: Vec<u64>,
    fl: Vec<u8>,
    gcalls: u64,
}

fn run_node(wasm: &[u8], cases: &str, g: u64, mode: &str, tag: &str) -> Vec<Ret> {
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-bnd-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.wasm"), wasm).unwrap();
    std::fs::write(dir.join("cases.json"), cases).unwrap();
    std::fs::write(dir.join("run.cjs"), JS).unwrap();
    let o = Command::new("node").arg(dir.join("run.cjs")).arg(dir.join("t.wasm")).arg(dir.join("cases.json")).arg(dir.join("out.json")).arg(format!("{g:x}")).arg(mode).output().unwrap();
    assert!(o.status.success(), "node: {}", String::from_utf8_lossy(&o.stderr).chars().take(600).collect::<String>());
    let txt = std::fs::read_to_string(dir.join("out.json")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    // the JSON is [{"regs":[..],"x":[..],"fl":[..],"gcalls":n},..]
    let nums = |s: &str| -> Vec<u64> { s.split(',').filter(|v| !v.is_empty()).map(|v| v.trim_matches('"').parse().unwrap()).collect() };
    let mut v = Vec::new();
    for item in txt.split("{\"regs\":[").skip(1) {
        let (regs, rest) = item.split_once("],\"x\":[").unwrap();
        let (x, rest) = rest.split_once("],\"fl\":[").unwrap();
        let (fl, rest) = rest.split_once("],\"gcalls\":").unwrap();
        let g = rest.trim_end_matches(|c: char| !c.is_ascii_digit()).parse().unwrap();
        v.push(Ret { regs: nums(regs), x: nums(x), fl: nums(fl).iter().map(|&b| b as u8).collect(), gcalls: g });
    }
    v
}

const ENTRY_RSP: u64 = 0x80000;

fn cases(fn_va: u64, args: &[(u64, u64)]) -> String {
    let cs: [(usize, u64); 8] = [(3, 0x1111_1111_1111_1111), (5, 0x2222_2222_2222_2222), (6, 0x3333_3333_3333_3333), (7, 0x4444_4444_4444_4444), (12, 0x5555_5555_5555_5555), (13, 0x6666_6666_6666_6666), (14, 0x7777_7777_7777_7777), (15, 0x8888_8888_8888_8888)];
    let mut items: Vec<String> = Vec::new();
    for &(a, b) in args {
        let mut regs: Vec<String> = cs.iter().map(|(r, v)| format!("\"{r}\":\"{v}\"")).collect();
        regs.push(format!("\"1\":\"{a}\""));
        regs.push(format!("\"2\":\"{b}\""));
        regs.push(format!("\"4\":\"{ENTRY_RSP}\""));
        items.push(format!("{{\"fn\":\"f_{fn_va:x}\",\"regs\":{{{}}}}}", regs.join(",")));
    }
    format!("[{}]", items.join(","))
}

fn check_boundary(r: &Ret, what: &str) {
    for (reg, v) in [(3usize, 0x1111_1111_1111_1111u64), (5, 0x2222_2222_2222_2222), (6, 0x3333_3333_3333_3333), (7, 0x4444_4444_4444_4444), (12, 0x5555_5555_5555_5555), (13, 0x6666_6666_6666_6666), (14, 0x7777_7777_7777_7777), (15, 0x8888_8888_8888_8888)] {
        assert_eq!(r.regs[reg], v, "{what}: callee-saved R[{reg}] after the return");
    }
    assert_eq!(r.regs[4], ENTRY_RSP, "{what}: R[4] after the return");
    for i in 6..16u64 {
        let lo = 0xaaaa_0000_0000_0000u64 | i;
        let hi = 0xbbbb_0000_0000_0000u64 | i;
        // the sentinels are 0xaaaa0000_000<i> written as BigInt('0xaaaa0000' + hex4(i)) = 0xaaaa0000_0000_000i
        let _ = (lo, hi);
        assert_eq!(r.x[2 * i as usize], u64::from_str_radix(&format!("aaaa0000{:04x}", i), 16).unwrap(), "{what}: X[{i}] low half after the return");
        assert_eq!(r.x[2 * i as usize + 1], u64::from_str_radix(&format!("bbbb0000{:04x}", i), 16).unwrap(), "{what}: X[{i}] high half after the return");
    }
}

#[test]
fn the_registers_the_stack_pointer_and_xmm6_to_15_come_back_and_the_je_reads_the_callees_zf() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let (f_at, g_at) = (0usize, 0x100usize);
    let f = code_f(g_at);
    let args: [(u64, u64); 5] = [(0, 0), (0x8000_0000, 0x8000_0000), (7, 7), (1, 2), (5, 0x1_0000_0000)];
    // F takes `je` into the path eax = 2 exactly when (rcx + rdx) low half is 0 (G is called then and needs rcx = rdx)
    let want = |a: u64, b: u64| if (a.wrapping_add(b) as u32) == 0 { 2u64 } else { 1 };
    let ok_args: Vec<(u64, u64)> = args.iter().copied().filter(|&(a, b)| (a.wrapping_add(b) as u32) != 0 || a == b).collect();
    let (f_va, g_va) = (0x401000 + f_at as u64, 0x401000 + g_at as u64);
    // (1) F direct, G on the C path, truthful and lying FL
    for mode in ["truth", "lie"] {
        let comp = compile(&pe(&[(0, &f), (g_at as u32, &G)]), &[f_va, g_va], &[], &conf(&[f_va])).unwrap_or_else(|e| panic!("compile: {e}"));
        assert!(comp.fidx.contains_key(&f_va), "F is not lowered: {:?}", comp.report.rows.iter().find(|r| r.va == f_va).map(|r| r.status.clone()));
        assert!(comp.report.rows.iter().any(|r| r.va == g_va && r.status.contains("not selected")), "G stays on the C path");
        let out = run_node(&comp.wasm, &cases(f_va, &ok_args), g_va, mode, &format!("c{mode}"));
        for (k, &(a, b)) in ok_args.iter().enumerate() {
            let r = &out[k];
            assert_eq!(r.regs[0] as u32, want(a, b) as u32, "{mode}: rax for ({a:#x}, {b:#x})");
            check_boundary(r, &format!("C-path G, FL {mode}, ({a:#x}, {b:#x})"));
            assert_eq!(r.gcalls, (want(a, b) == 2) as u64, "{mode}: G is called exactly when the test path is skipped");
        }
    }
    // (2) F and G both direct: FL is exactly as it was before
    let comp = compile(&pe(&[(0, &f), (g_at as u32, &G)]), &[f_va, g_va], &[], &conf(&[f_va, g_va])).unwrap();
    assert!(comp.fidx.contains_key(&f_va) && comp.fidx.contains_key(&g_va));
    let out = run_node(&comp.wasm, &cases(f_va, &ok_args), g_va, "none", "dd");
    for (k, &(a, b)) in ok_args.iter().enumerate() {
        let r = &out[k];
        assert_eq!(r.regs[0] as u32, want(a, b) as u32, "direct G: rax for ({a:#x}, {b:#x})");
        check_boundary(r, &format!("direct G, ({a:#x}, {b:#x})"));
        assert_eq!(r.fl, vec![0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47], "direct F and G do not write FL");
    }
}

#[test]
fn the_wrapper_of_a_direct_function_leaves_the_flags_of_the_c_path_as_they_were() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    // G alone, direct: a C-path caller that calls f_<G> and then reads FL.zf would read the value from before the call. Decision 0270 relies on there being no such caller (scan in the decision's tables).
    let g_va = 0x401000u64;
    let comp = compile(&pe(&[(0, &G)]), &[g_va], &[], &conf(&[g_va])).unwrap();
    assert!(comp.fidx.contains_key(&g_va));
    let out = run_node(&comp.wasm, &cases(g_va, &[(9, 9)]), g_va, "none", "wrap");
    assert_eq!(out[0].fl, vec![0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47], "the wrapper writes no flag byte: ZF after `f_<G>` is the one before it");
}
