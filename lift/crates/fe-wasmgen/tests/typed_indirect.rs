//! Decision 0285 (mt3w): the typed indirect call. A direct function F does `call rax` to a target T. With `hybrid_dispatch` set, a T that fits the uniform indirect signature (reads rcx, rdx, r8, r9, rax; preserves the callee-saved registers;
//! touches no XMM register) is called through the table of adapters without `c.fle_hyb_dispatch`; every other target, and every call made while a landing (`fle_landing_top`) or the census hook (`fle_dispatch_hook`) is set, goes through the
//! dispatch of the C path as before. Each case is run twice, once as it comes (typed) and once with the landing word set (the helper then gives the call to the dispatch: the path of before), and everything the caller can observe is compared:
//! rax, the callee-saved registers, R[4], X[6..15], the flag bytes, the progress count, the slice calls, the return address that the call leaves on the guest stack. The mock of the C dispatch counts like `fle_dispatch`: ++count, a slice
//! when count >= limit, then the call of the wrapper `f_<va>`. Skipped (and passing) when node is missing.
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

fn conf(only: &[u64], dispatch: bool) -> Config {
    let mut ci: HashMap<String, String> = HashMap::new();
    ci.insert("dummy".into(), "fle_imp_dummy".into());
    Config {
        hybrid_r: Some(0x2000),
        tls_ctx: Some(0x2000),
        hybrid_x: Some(0x2200),
        hybrid_mx: Some((0x3000, 0x3100)),
        hybrid_progress: Some(0x3300),
        hybrid_dispatch: if dispatch { Some((0x3308, 0x3310, 0x3318)) } else { None },
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

// T1: lea eax,[rcx*2+1]; ret                                   (fits: reads rcx, writes eax)
const T1: [u8; 8] = [0x8d, 0x04, 0x4d, 0x01, 0x00, 0x00, 0x00, 0xc3];
// T2: lea rax,[rcx+rdx]; add rax,r8; ret                       (fits: reads rcx, rdx, r8)
const T2: [u8; 8] = [0x48, 0x8d, 0x04, 0x11, 0x4c, 0x01, 0xc0, 0xc3];
// T3: mov rbx,rcx; mov rax,rbx; ret                            (does not preserve rbx: stays off the table)
const T3: [u8; 7] = [0x48, 0x89, 0xcb, 0x48, 0x89, 0xd8, 0xc3];
// T4: movq xmm0,rcx; movq rax,xmm0; ret                        (touches XMM0: stays off the table)
const T4: [u8; 11] = [0x66, 0x48, 0x0f, 0x6e, 0xc1, 0x66, 0x48, 0x0f, 0x7e, 0xc0, 0xc3];
// F: push rbx; sub rsp,0x20; mov rbx,r9; mov rax,rcx; mov rcx,rdx; mov rdx,r8; call rax; add rax,rbx; add rsp,0x20; pop rbx; ret      F(target, a, b, c) = target(a, b) + c
const F: [u8; 30] = [
    0x53, 0x48, 0x83, 0xec, 0x20, 0x4c, 0x89, 0xcb, 0x48, 0x89, 0xc8, 0x48, 0x89, 0xd1, 0x4c, 0x89, 0xc2, 0xff, 0xd0, 0x48, 0x01, 0xd8, 0x48, 0x83, 0xc4, 0x20, 0x5b, 0xc3, 0x90, 0x90,
];
const F_RET: u64 = 19; // the offset of the instruction after `call rax`
const ENTRY_RSP: u64 = 0x80000;

const JS: &str = r#"
const fs = require('fs');
const [wasmF, casesF, outF, t3, t4] = process.argv.slice(2);
const mem = new WebAssembly.Memory({ initial: 16, maximum: 32768, shared: true });
const dv = new DataView(mem.buffer);
const m = new WebAssembly.Module(fs.readFileSync(wasmF));
const R = 0x2000, FL = 0x2100, X = 0x2200, PC = 0x3300, LIM = 0x3308, LAND = 0x3310, HOOK = 0x3318;
let dispatches = 0, slices = 0, unknown = 0, inst;
const imports = { c: { memory: mem, __tls_base: new WebAssembly.Global({ value: 'i32', mutable: true }, 0) } };
for (const i of WebAssembly.Module.imports(m)) {
  if (i.kind !== 'function') continue;
  imports[i.module] = imports[i.module] || {};
  imports[i.module][i.name] = () => { throw new Error('unexpected call of ' + i.module + '.' + i.name); };
}
imports.c.fle_hyb_slice = () => { slices++; };
imports.c.fle_hyb_dispatch = (va) => {
  dispatches++;
  const c = dv.getBigUint64(PC, true) + 1n; dv.setBigUint64(PC, c, true);
  if (c >= dv.getBigUint64(LIM, true)) slices++;
  const name = 'f_' + va.toString(16);
  if (va.toString(16) === t3) { const rcx = dv.getBigUint64(R + 8, true); dv.setBigUint64(R + 24, rcx, true); dv.setBigUint64(R, rcx, true); }          // T3: mov rbx,rcx; mov rax,rbx (rbx is clobbered, as the lifted code would)
  else if (va.toString(16) === t4) { dv.setBigUint64(R, dv.getBigUint64(R + 8, true), true); }                                                          // T4: rax = rcx (through xmm0)
  else if (inst.exports[name]) inst.exports[name]();
  else { unknown++; dv.setBigUint64(R, 0xdeadn, true); }
};
inst = new WebAssembly.Instance(m, imports);
const cases = JSON.parse(fs.readFileSync(casesF, 'utf8'));
const out = [];
for (const c of cases) {
  for (let r = 0; r < 16; r++) dv.setBigUint64(R + 8 * r, BigInt(c.regs[r] !== undefined ? c.regs[r] : '0'), true);
  for (let i = 0; i < 16; i++) { dv.setBigUint64(X + 16 * i, BigInt('0xaaaa0000' + i.toString(16).padStart(4, '0')), true); dv.setBigUint64(X + 16 * i + 8, BigInt('0xbbbb0000' + i.toString(16).padStart(4, '0')), true); }
  for (let b = 0; b < 8; b++) dv.setUint8(FL + b, 0x40 + b);
  dv.setBigUint64(PC, BigInt(c.count), true); dv.setBigUint64(LIM, BigInt(c.limit), true);
  dv.setUint32(LAND, c.landing, true); dv.setUint32(HOOK, c.hook, true);
  const slot = BigInt(c.slot); dv.setBigUint64(Number(slot), 0n, true);
  dispatches = 0; slices = 0; unknown = 0;
  inst.exports[c.fn]();
  const regs = []; for (let r = 0; r < 16; r++) regs.push(dv.getBigUint64(R + 8 * r, true).toString());
  const x = []; for (let i = 0; i < 16; i++) { x.push(dv.getBigUint64(X + 16 * i, true).toString()); x.push(dv.getBigUint64(X + 16 * i + 8, true).toString()); }
  const fl = []; for (let b = 0; b < 8; b++) fl.push(dv.getUint8(FL + b));
  out.push(['regs=' + regs.join(','), 'x=' + x.join(','), 'fl=' + fl.join(','), 'count=' + dv.getBigUint64(PC, true), 'dispatches=' + dispatches, 'slices=' + slices, 'unknown=' + unknown, 'retslot=' + dv.getBigUint64(Number(slot), true)].join('\n'));
}
fs.writeFileSync(outF, out.join('\n--\n'));
"#;

struct Ret {
    regs: Vec<u64>,
    x: Vec<u64>,
    fl: Vec<u8>,
    count: u64,
    dispatches: u64,
    slices: u64,
    unknown: u64,
    retslot: u64,
}

#[derive(Clone)]
struct Case {
    target: u64,
    a: u64,
    b: u64,
    c: u64,
    landing: u32,
    hook: u32,
    count: u64,
    limit: u64,
    /// call this wrapper `f_<va>` instead of F (a caller on the C path calls the page-ABI wrapper of a direct function)
    fnva: Option<u64>,
}

fn run_node(wasm: &[u8], f_va: u64, cases: &[Case], t3: u64, t4: u64, tag: &str) -> Vec<Ret> {
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-ti-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.wasm"), wasm).unwrap();
    let cs: [(usize, u64); 8] = [
        (3, 0x1111_1111_1111_1111),
        (5, 0x2222_2222_2222_2222),
        (6, 0x3333_3333_3333_3333),
        (7, 0x4444_4444_4444_4444),
        (12, 0x5555_5555_5555_5555),
        (13, 0x6666_6666_6666_6666),
        (14, 0x7777_7777_7777_7777),
        (15, 0x8888_8888_8888_8888),
    ];
    let slot = ENTRY_RSP - 0x30;
    let mut items: Vec<String> = Vec::new();
    for c in cases {
        let mut regs: Vec<String> = cs.iter().map(|(r, v)| format!("\"{r}\":\"{v}\"")).collect();
        regs.push(format!("\"1\":\"{}\"", c.target));
        regs.push(format!("\"2\":\"{}\"", c.a));
        regs.push(format!("\"8\":\"{}\"", c.b));
        regs.push(format!("\"9\":\"{}\"", c.c));
        regs.push(format!("\"4\":\"{ENTRY_RSP}\""));
        items.push(format!(
            "{{\"fn\":\"f_{:x}\",\"regs\":{{{}}},\"count\":{},\"limit\":{},\"landing\":{},\"hook\":{},\"slot\":{slot}}}",
            c.fnva.unwrap_or(f_va),
            regs.join(","),
            c.count,
            c.limit,
            c.landing,
            c.hook
        ));
    }
    std::fs::write(dir.join("cases.json"), format!("[{}]", items.join(","))).unwrap();
    std::fs::write(dir.join("run.cjs"), JS).unwrap();
    let o = Command::new("node").arg(dir.join("run.cjs")).arg(dir.join("t.wasm")).arg(dir.join("cases.json")).arg(dir.join("out.txt")).arg(format!("{t3:x}")).arg(format!("{t4:x}")).output().unwrap();
    assert!(o.status.success(), "node: {}", String::from_utf8_lossy(&o.stderr).chars().take(800).collect::<String>());
    let txt = std::fs::read_to_string(dir.join("out.txt")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let nums = |s: &str| -> Vec<u64> { s.split(',').filter(|v| !v.is_empty()).map(|v| v.parse().unwrap()).collect() };
    let mut v = Vec::new();
    for item in txt.split("\n--\n") {
        let mut m: HashMap<&str, &str> = HashMap::new();
        for l in item.lines() {
            if let Some((k, val)) = l.split_once('=') {
                m.insert(k, val);
            }
        }
        v.push(Ret {
            regs: nums(m["regs"]),
            x: nums(m["x"]),
            fl: nums(m["fl"]).iter().map(|&b| b as u8).collect(),
            count: m["count"].parse().unwrap(),
            dispatches: m["dispatches"].parse().unwrap(),
            slices: m["slices"].parse().unwrap(),
            unknown: m["unknown"].parse().unwrap(),
            retslot: m["retslot"].parse().unwrap(),
        });
    }
    v
}

fn build(dispatch: bool) -> (Vec<u8>, u64, [u64; 4], fe_wasmgen::Compiled) {
    let offs: [u32; 5] = [0, 0x100, 0x120, 0x140, 0x160];
    let img = pe(&[(offs[0], &F), (offs[1], &T1), (offs[2], &T2), (offs[3], &T3), (offs[4], &T4)]);
    let vas: Vec<u64> = offs.iter().map(|o| 0x401000 + *o as u64).collect();
    let comp = compile(&img, &vas, &[], &conf(&[vas[0], vas[1], vas[2]], dispatch)).unwrap_or_else(|e| panic!("compile: {e}"));
    (comp.wasm.clone(), vas[0], [vas[1], vas[2], vas[3], vas[4]], comp)
}

fn case(target: u64, a: u64, b: u64, c: u64) -> Case {
    Case { target, a, b, c, landing: 0, hook: 0, count: 100, limit: 1_000_000, fnva: None }
}

/// everything the caller can observe, except what the two paths may leave differently in the volatile registers and in the scratch words of the C part
fn observed(r: &Ret) -> (Vec<u64>, Vec<u64>, Vec<u8>, u64, u64, u64) {
    let keep = [0usize, 3, 4, 5, 6, 7, 12, 13, 14, 15];
    (keep.iter().map(|&i| r.regs[i]).collect(), r.x[12..32].to_vec(), r.fl.clone(), r.count, r.slices, r.retslot)
}

#[test]
fn a_fitting_target_is_called_without_the_dispatch_and_everything_else_is_as_through_the_dispatch() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let (wasm, f_va, [t1, t2, t3, t4], comp) = build(true);
    let fit = |va: u64| comp.report.rows.iter().find(|r| r.va == va).map(|r| r.indirect.clone()).unwrap_or_default();
    assert!(fit(f_va).starts_with("nofit: xmm"), "F holds an indirect call: the callees it may reach read and write XMM registers, it is not a fitting target ({})", fit(f_va));
    assert_eq!(fit(t1), "fit");
    assert_eq!(fit(t2), "fit");
    assert_eq!(comp.report.indirect_adapters, 2, "two adapters: T1, T2");
    assert!(comp.report.typed_slots >= 16 && comp.report.typed_slots.is_power_of_two());
    // T3 and T4 are not in the hybrid list here, so they are not lowered: they stay on the C path (the mock dispatch)
    let ret = f_va + F_RET;
    let mut cases: Vec<Case> = Vec::new();
    for (t, a, b) in [(t1, 5u64, 0u64), (t1, 0x7fff_ffff, 0), (t1, 0x1_0000_0003, 0), (t2, 10, 20), (t2, u64::MAX, 2)] {
        cases.push(case(t, a, b, 1000));
    }
    let mut slow = cases.clone();
    for c in slow.iter_mut() {
        c.landing = 1;
    }
    let all: Vec<Case> = cases.iter().cloned().chain(slow.iter().cloned()).collect();
    let out = run_node(&wasm, f_va, &all, t3, t4, "eq");
    let n = cases.len();
    for k in 0..n {
        let (ty, sl) = (&out[k], &out[n + k]);
        let what = format!("case {k} (target {:#x})", cases[k].target);
        assert_eq!(ty.dispatches, 0, "{what}: the typed call does not use the dispatch of the C path");
        assert_eq!(sl.dispatches, 1, "{what}: with the landing set the call goes through the dispatch");
        assert_eq!(observed(ty), observed(sl), "{what}: the typed and the dispatched call leave the same rax, callee-saved registers, R[4], X[6..15], flags, count, slices and return address");
        assert_eq!(ty.count, cases[k].count + 1, "{what}: the progress count");
        assert_eq!(ty.unknown + sl.unknown, 0, "{what}");
    }
    // the values
    let lea = |a: u64| ((a << 1).wrapping_add(1)) & 0xffff_ffff;
    assert_eq!(out[0].regs[0], lea(5) + 1000);
    assert_eq!(out[1].regs[0], lea(0x7fff_ffff) + 1000);
    assert_eq!(out[2].regs[0], lea(0x1_0000_0003) + 1000);
    assert_eq!(out[3].regs[0], 10 + 20 + 20 + 1000, "T2 adds rcx, rdx and r8 (F leaves r8 = b)");
    assert_eq!(out[4].regs[0], u64::MAX.wrapping_add(2).wrapping_add(2).wrapping_add(1000));
    assert_eq!(out[0].retslot, ret, "the return address of the call is on the guest stack");
    // the callee-saved registers and X[6..15] have the values they had; R[4] is the entry value
    for (reg, v) in [(3usize, 0x1111_1111_1111_1111u64), (5, 0x2222_2222_2222_2222), (6, 0x3333_3333_3333_3333), (7, 0x4444_4444_4444_4444), (12, 0x5555_5555_5555_5555), (13, 0x6666_6666_6666_6666), (14, 0x7777_7777_7777_7777), (15, 0x8888_8888_8888_8888)] {
        assert_eq!(out[0].regs[reg], v, "callee-saved R[{reg}]");
    }
    assert_eq!(out[0].regs[4], ENTRY_RSP);
    assert_eq!(out[0].fl, vec![0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47], "no flag byte is written");
}

#[test]
fn a_target_off_the_table_the_landing_the_hook_a_pseudo_va_and_the_slice_limit_behave_as_through_the_dispatch() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let (wasm, f_va, [t1, _t2, t3, t4], _) = build(true);
    let mut cs: Vec<Case> = Vec::new();
    // 0: T3 (C path: clobbers rbx in the mock; F's own rbx is a wasm local and comes back)    1: T4    2: pseudo-VA target    3: hook set    4: slice at the limit    5: just below the limit
    cs.push(case(t3, 9, 0, 1000));
    cs.push(case(t4, 9, 0, 1000));
    cs.push(case(0x7ff9_0000_1000, 0, 0, 1000));
    cs.push(Case { hook: 1, ..case(t1, 4, 0, 1000) });
    cs.push(Case { count: 100, limit: 101, ..case(t1, 4, 0, 1000) });
    cs.push(Case { count: 100, limit: 102, ..case(t1, 4, 0, 1000) });
    // 6: the same as 4 with the landing set (the path of before)
    cs.push(Case { count: 100, limit: 101, landing: 1, ..case(t1, 4, 0, 1000) });
    // 7: a caller on the C path: it calls the wrapper `f_<T1>` (R[1] = 21), which neither counts nor looks anything up
    cs.push(Case { fnva: Some(t1), ..case(21, 0, 0, 0) });
    let out = run_node(&wasm, f_va, &cs, t3, t4, "slow");
    assert_eq!((out[7].dispatches, out[7].count, out[7].regs[0]), (0, 100, 43), "the wrapper of a fitting function called from the C path: rax = 2 * 21 + 1, no dispatch, no count");
    assert_eq!(out[0].dispatches, 1, "T3 is not in the table: the dispatch");
    assert_eq!(out[0].regs[0], 9 + 1000);
    assert_eq!(out[1].dispatches, 1, "T4 touches XMM0: not in the table");
    assert_eq!(out[1].regs[0], 9 + 1000);
    assert_eq!(out[2].dispatches, 1, "a pseudo-VA is not in the table");
    assert_eq!(out[2].unknown, 1);
    assert_eq!(out[3].dispatches, 1, "the census hook is set: the dispatch");
    assert_eq!(out[3].regs[0], ((4u64 << 1) + 1) + 1000);
    assert_eq!((out[4].dispatches, out[4].slices, out[4].count), (0, 1, 101), "the typed call counts once and takes the slice at the limit, without the dispatch");
    assert_eq!((out[5].dispatches, out[5].slices, out[5].count), (0, 0, 101), "below the limit: no slice");
    assert_eq!((out[6].dispatches, out[6].slices, out[6].count), (1, 1, 101), "through the dispatch: the same count and the same slice");
    for (k, v) in out.iter().enumerate() {
        for (reg, x) in [(5usize, 0x2222_2222_2222_2222u64), (6, 0x3333_3333_3333_3333), (7, 0x4444_4444_4444_4444), (12, 0x5555_5555_5555_5555)] {
            assert_eq!(v.regs[reg], x, "case {k}: callee-saved R[{reg}]");
        }
        assert_eq!(v.regs[4], ENTRY_RSP, "case {k}: R[4]");
    }
}

#[test]
fn a_hundred_random_calls_of_the_fitting_targets_give_the_same_values_through_the_table_and_through_the_dispatch() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let (wasm, f_va, [t1, t2, _t3, _t4], _) = build(true);
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15; // xorshift64*: the same sequence every run
    let mut next = || {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    let mut cases: Vec<Case> = Vec::new();
    for k in 0..100 {
        let t = if k % 2 == 0 { t1 } else { t2 };
        cases.push(case(t, next(), next(), next()));
    }
    let mut slow = cases.clone();
    for c in slow.iter_mut() {
        c.landing = 1;
    }
    let all: Vec<Case> = cases.iter().cloned().chain(slow.iter().cloned()).collect();
    let out = run_node(&wasm, f_va, &all, 0, 0, "rnd");
    for (k, c) in cases.iter().enumerate() {
        let (ty, sl) = (&out[k], &out[100 + k]);
        let want = if c.target == t1 { ((c.a << 1).wrapping_add(1) & 0xffff_ffff).wrapping_add(c.c) } else { c.a.wrapping_add(c.b).wrapping_add(c.b).wrapping_add(c.c) };
        assert_eq!(ty.regs[0], want, "case {k}: rax");
        assert_eq!(observed(ty), observed(sl), "case {k}: the typed and the dispatched call differ");
        assert_eq!((ty.dispatches, sl.dispatches), (0, 1), "case {k}");
    }
}

#[test]
fn without_the_dispatch_addresses_the_module_has_no_table_and_calls_the_dispatch_as_before() {
    let (wasm_on, f_va, _, comp_on) = build(true);
    let (wasm_off, _, _, comp_off) = build(false);
    assert!(comp_on.wasm.len() > comp_off.wasm.len());
    assert_eq!(comp_off.report.indirect_adapters, 0);
    assert_eq!(comp_off.report.typed_slots, 0);
    // the diagnostic column is there in both
    assert!(comp_off.report.rows.iter().find(|r| r.va == f_va).unwrap().indirect.starts_with("nofit: xmm"));
    assert_eq!(comp_off.report.rows.iter().find(|r| r.va == f_va + 0x100).unwrap().indirect, "fit");
    let _ = (wasm_on, wasm_off);
    // determinism: the same input gives the same bytes
    let (wasm_again, ..) = build(true);
    assert_eq!(comp_on.wasm, wasm_again);
}
