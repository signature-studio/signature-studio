//! Decision 0264 (mt3z): flag definitions whose saved-operand locals are not live at the same time share one pair of locals (`locshare.rs`). Each function below is
//! lowered with the sharing off (`LOCSHARE_OFF`) and on, with `LOCSHARE_VERIFY` set (the instruction-walking check of the assignment), and run under node on a set of
//! inputs; both must agree with the result computed here. The functions are chosen so that definitions that must not share (a flag read across a later definition
//! that writes other flags, a loop-carried use) and definitions that may (compare and branch in separate blocks, one after the other) are both present.
//! Skipped (and passing) when node is missing.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::{compile, locshare, Config};
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
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-locsh-{}-{}", std::process::id(), tag));
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


static GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn compile_one(code: &[u8], share: bool) -> fe_wasmgen::Compiled {
    use std::sync::atomic::Ordering::Relaxed;
    let _g = GUARD.lock().unwrap_or_else(|e| e.into_inner());
    locshare::LOCSHARE_OFF.store(!share, Relaxed);
    locshare::LOCSHARE_VERIFY.store(true, Relaxed);
    let c = compile(&pe(&[(0, code)]), &[0x401000], &[], &conf()).unwrap_or_else(|e| panic!("compile: {e}"));
    locshare::LOCSHARE_OFF.store(false, Relaxed);
    locshare::LOCSHARE_VERIFY.store(false, Relaxed);
    assert!(c.fidx.contains_key(&0x401000), "not lowered: {:?}", c.report.rows.iter().find(|r| r.va == 0x401000).map(|r| r.status.clone()));
    c
}

/// runs `f_401000` of both builds on every input (rcx, rdx, r8, r9) and returns the rax (index 0), rsi (6) and r9 (9) words of each (only rax is an output of the functions below)
fn both(code: &[u8], inputs: &[[u64; 4]], tag: &str) -> (Vec<(u64, u64, u64)>, Vec<(u64, u64, u64)>, bool) {
    let off = compile_one(code, false);
    let on = compile_one(code, true);
    let cases: Vec<String> = inputs
        .iter()
        .map(|i| format!("{{\"fn\":\"f_401000\",\"regs\":{{\"1\":\"{}\",\"2\":\"{}\",\"8\":\"{}\",\"9\":\"{}\",\"4\":\"524288\"}},\"read\":[]}}", i[0], i[1], i[2], i[3]))
        .collect();
    let cases = format!("[{}]", cases.join(","));
    let get = |r: Vec<Vec<u64>>| r.into_iter().map(|v| (v[0], v[6], v[9])).collect::<Vec<_>>();
    let a = get(run_node(&off.wasm, &cases, &format!("{tag}-off")));
    let b = get(run_node(&on.wasm, &cases, &format!("{tag}-on")));
    (a, b, off.wasm != on.wasm)
}

fn rel(from_next: usize, to: usize) -> u8 {
    (to as i64 - from_next as i64) as i8 as u8
}

fn mov_rax(v: u32) -> Vec<u8> {
    let mut b = vec![0x48, 0xc7, 0xc0];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

// three compare-and-branch pairs one after the other, each pair split into two blocks by `jmp +0`:
//   cmp rcx,rdx; jmp; jl X1;  cmp rdx,r8; jmp; jg X2;  cmp r8,r9; jmp; jb X3;  mov rax,0; ret;  X1: mov rax,1; ret;  X2: mov rax,2; ret;  X3: mov rax,3; ret
fn chain() -> Vec<u8> {
    let mut c: Vec<u8> = vec![];
    c.extend([0x48, 0x39, 0xd1, 0xeb, 0x00, 0x7c, 0x00]); // 0x00..0x07 (jl patched)
    c.extend([0x4c, 0x39, 0xc2, 0xeb, 0x00, 0x7f, 0x00]); // 0x07..0x0e
    c.extend([0x4d, 0x39, 0xc8, 0xeb, 0x00, 0x72, 0x00]); // 0x0e..0x15
    c.extend(mov_rax(0));
    c.push(0xc3);
    let x1 = c.len();
    c.extend(mov_rax(1));
    c.push(0xc3);
    let x2 = c.len();
    c.extend(mov_rax(2));
    c.push(0xc3);
    let x3 = c.len();
    c.extend(mov_rax(3));
    c.push(0xc3);
    c[6] = rel(7, x1);
    c[13] = rel(14, x2);
    c[20] = rel(21, x3);
    c
}

fn chain_want(i: &[u64; 4]) -> u64 {
    let (a, b, c, d) = (i[0] as i64, i[1] as i64, i[2] as i64, i[3]);
    if a < b {
        1
    } else if b > c {
        2
    } else if (c as u64) < d {
        3
    } else {
        0
    }
}

fn inputs() -> Vec<[u64; 4]> {
    let v: [u64; 6] = [0, 1, 5, u64::MAX, (-7i64) as u64, 1 << 63];
    let mut out = Vec::new();
    for &a in &v {
        for &b in &v {
            for &c in &[0u64, 3, u64::MAX] {
                for &d in &[0u64, 4, u64::MAX] {
                    out.push([a, b, c, d]);
                }
            }
        }
    }
    out
}

#[test]
fn definitions_in_separate_blocks_share_locals_and_every_branch_still_reads_its_own_compare() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let ins = inputs();
    let (off, on, differ) = both(&chain(), &ins, "chain");
    assert!(differ, "the sharing changed nothing: the test does not exercise it");
    for (k, i) in ins.iter().enumerate() {
        let w = chain_want(i);
        assert_eq!(off[k].0, w, "sharing off, input {i:?}");
        assert_eq!(on[k].0, w, "sharing on, input {i:?}");
    }
}

// a flag read across a later definition that writes other flags: cmp rcx,rdx (CF for jb); inc r9 (ZF for jz, CF untouched); jb X1; jz X2; mov rax,1; ret; X1: mov rax,2; ret; X2: mov rax,3; ret
fn crossing() -> Vec<u8> {
    let mut c: Vec<u8> = vec![0x48, 0x39, 0xd1, 0x49, 0xff, 0xc1, 0x72, 0x00, 0x74, 0x00];
    c.extend(mov_rax(1));
    c.push(0xc3);
    let x1 = c.len();
    c.extend(mov_rax(2));
    c.push(0xc3);
    let x2 = c.len();
    c.extend(mov_rax(3));
    c.push(0xc3);
    c[7] = rel(8, x1);
    c[9] = rel(10, x2);
    c
}

#[test]
fn a_flag_read_across_a_later_definition_of_other_flags_is_not_shared_over() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let ins = inputs();
    let (off, on, _) = both(&crossing(), &ins, "cross");
    for (k, i) in ins.iter().enumerate() {
        let w = if i[0] < i[1] {
            2
        } else if i[3].wrapping_add(1) == 0 {
            3
        } else {
            1
        };
        assert_eq!(off[k].0, w, "sharing off, input {i:?}");
        assert_eq!(on[k].0, w, "sharing on, input {i:?}");
    }
}

// a loop: xor eax,eax; xor esi,esi; L: cmp rax,rcx; jmp; jae done; add rsi,rax; inc rax; jmp; jnz L; done: mov rax,rsi; ret      (rsi = 0 + 1 + .. + (rcx-1))
fn looped() -> Vec<u8> {
    let mut c: Vec<u8> = vec![0x31, 0xc0, 0x31, 0xf6];
    let l = c.len();
    c.extend([0x48, 0x39, 0xc8, 0xeb, 0x00, 0x73, 0x00]);
    c.extend([0x48, 0x01, 0xc6, 0x48, 0xff, 0xc0, 0xeb, 0x00, 0x75, 0x00]);
    let done = c.len();
    c.extend([0x48, 0x89, 0xf0, 0xc3]);
    c[l + 6] = rel(l + 7, done);
    let jnz = l + 7 + 9;
    c[jnz] = rel(jnz + 1, l);
    c
}

#[test]
fn a_loop_whose_compare_and_increment_flags_are_read_in_the_next_block_gives_the_same_sum() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let ins: Vec<[u64; 4]> = [0u64, 1, 2, 3, 10, 57].iter().map(|&n| [n, 0, 0, 0]).collect();
    let (off, on, differ) = both(&looped(), &ins, "loop");
    assert!(differ, "the sharing changed nothing: the test does not exercise it");
    for (k, i) in ins.iter().enumerate() {
        let n = i[0];
        let w = if n == 0 { 0 } else { n * (n - 1) / 2 };
        assert_eq!(off[k].0, w, "sharing off, n = {n}");
        assert_eq!(on[k].0, w, "sharing on, n = {n}");
    }
}
