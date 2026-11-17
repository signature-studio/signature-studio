//! Decision 0169: differential test of XADD, CMPXCHG and XCHG with a memory destination (the forms the C path has; fe-lift emits no register form). One small function per form
//! (`mov r9,[rcx]; mov [rcx+32],r9; mov r8,[rcx+8]; mov rax,[rcx+16]; <insn> [rcx+32], r8; ...; the memory, the register and the flags CF ZF SF OF PF stored through rdx`)
//! is lowered and run under node on random and edge inputs (0, 1, 0x7f.., 0x80.., all ones, equal values for CMPXCHG, carries and overflows at every operand size); the result is
//! compared with the architectural definition (computed here). Skipped (and passing) when node is missing.
use fe_wasmgen::{compile, Config};
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

fn pe(code: &[u8]) -> Vec<u8> {
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
    w32(&mut b, opt + 112 + 3 * 8 + 4, 12);
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
    b[0x400..0x400 + code.len()].copy_from_slice(code);
    w32(&mut b, 0x800, 0x1000);
    w32(&mut b, 0x804, 0x1000 + code.len() as u32);
    w32(&mut b, 0x808, 0x2100);
    b[0x900..0x904].copy_from_slice(&[1, 0, 0, 0]);
    b
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

#[derive(Clone, Copy, PartialEq)]
enum Op {
    Xadd,
    Cmpxchg,
    Xchg,
    /// `xchg [r13+r12*1+0], rax` as in the delay-load helper of FLEngine (0x41dbe1): base r13 (disp8 = 0), index r12, the register is rax
    XchgSib,
    /// the 8-bit forms with the high-byte register AH as the source (no REX prefix; the engine's spin lock `mov eax,0x100; lock cmpxchg [rbx],ah` at 0x407ebe)
    HCmpxchg,
    HXchg,
    HXadd,
}

fn code_for(op: Op, n: usize) -> Vec<u8> {
    if matches!(op, Op::HCmpxchg | Op::HXchg | Op::HXadd) {
        let mut c: Vec<u8> = vec![0x4c, 0x8b, 0x09, 0x4c, 0x89, 0x49, 0x20, 0x48, 0x8b, 0x41, 0x10]; // mov r9,[rcx]; mov [rcx+32],r9; mov rax,[rcx+16]
        match op {
            Op::HCmpxchg => c.extend_from_slice(&[0xf0, 0x0f, 0xb0, 0x61, 0x20]),
            Op::HXadd => c.extend_from_slice(&[0xf0, 0x0f, 0xc0, 0x61, 0x20]),
            _ => c.extend_from_slice(&[0x86, 0x61, 0x20]),
        }
        c.extend_from_slice(&[0x4c, 0x8b, 0x49, 0x20, 0x4c, 0x89, 0x0a, 0x48, 0x89, 0x42, 0x08]); // mov r9,[rcx+32]; mov [rdx],r9; mov [rdx+8],rax
        if op != Op::HXchg {
            c.extend_from_slice(&[0x0f, 0x92, 0x42, 0x10, 0x0f, 0x94, 0x42, 0x11, 0x0f, 0x98, 0x42, 0x12, 0x0f, 0x90, 0x42, 0x13, 0x0f, 0x9a, 0x42, 0x14]);
        }
        c.push(0xc3);
        return c;
    }
    if op == Op::XchgSib {
        return vec![
            0x4c, 0x8b, 0x09, 0x4c, 0x89, 0x49, 0x20, // mov r9,[rcx]; mov [rcx+32],r9
            0x41, 0x55, 0x41, 0x54, // push r13; push r12
            0x4c, 0x8d, 0x69, 0x20, // lea r13,[rcx+0x20]
            0x45, 0x31, 0xe4, // xor r12d,r12d
            0x48, 0x8b, 0x41, 0x08, // mov rax,[rcx+8]
            0x4b, 0x87, 0x44, 0x25, 0x00, // xchg [r13+r12*1+0],rax
            0x48, 0x89, 0x42, 0x08, // mov [rdx+8],rax
            0x4c, 0x8b, 0x49, 0x20, 0x4c, 0x89, 0x0a, // mov r9,[rcx+32]; mov [rdx],r9
            0x41, 0x5c, 0x41, 0x5d, 0xc3, // pop r12; pop r13; ret
        ];
    }
    let mut c: Vec<u8> = vec![
        0x4c, 0x8b, 0x09, // mov r9,[rcx]
        0x4c, 0x89, 0x49, 0x20, // mov [rcx+32],r9
        0x4c, 0x8b, 0x41, 0x08, // mov r8,[rcx+8]
        0x48, 0x8b, 0x41, 0x10, // mov rax,[rcx+16]
    ];
    let (pfx, rex): (&[u8], u8) = match n {
        2 => (&[0x66], 0x44),
        8 => (&[], 0x4c),
        _ => (&[], 0x44),
    };
    c.push(0xf0); // lock
    c.extend_from_slice(pfx);
    c.push(rex);
    match (op, n) {
        (Op::Xadd, 1) => c.extend_from_slice(&[0x0f, 0xc0, 0x41, 0x20]),
        (Op::Xadd, _) => c.extend_from_slice(&[0x0f, 0xc1, 0x41, 0x20]),
        (Op::Cmpxchg, 1) => c.extend_from_slice(&[0x0f, 0xb0, 0x41, 0x20]),
        (Op::Cmpxchg, _) => c.extend_from_slice(&[0x0f, 0xb1, 0x41, 0x20]),
        (Op::XchgSib, _) | (Op::HCmpxchg, _) | (Op::HXchg, _) | (Op::HXadd, _) => unreachable!(),
        (Op::Xchg, 1) => c.extend_from_slice(&[0x86, 0x41, 0x20]),
        (Op::Xchg, _) => c.extend_from_slice(&[0x87, 0x41, 0x20]),
    }
    c.extend_from_slice(&[0x4c, 0x8b, 0x49, 0x20, 0x4c, 0x89, 0x0a]); // mov r9,[rcx+32]; mov [rdx],r9
    if op == Op::Cmpxchg {
        c.extend_from_slice(&[0x48, 0x89, 0x42, 0x08]); // mov [rdx+8],rax
    } else {
        c.extend_from_slice(&[0x4c, 0x89, 0x42, 0x08]); // mov [rdx+8],r8
    }
    if op != Op::Xchg {
        c.extend_from_slice(&[0x0f, 0x92, 0x42, 0x10, 0x0f, 0x94, 0x42, 0x11, 0x0f, 0x98, 0x42, 0x12, 0x0f, 0x90, 0x42, 0x13, 0x0f, 0x9a, 0x42, 0x14]); // setb setz sets seto setp
    }
    c.push(0xc3);
    c
}

fn rng(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

fn inputs(n: usize) -> Vec<[u64; 3]> {
    let edge: [u64; 14] = [0, 1, 2, 0x7f, 0x80, 0xff, 0x7fff, 0x8000, 0xffff, 0x7fffffff, 0x80000000, 0xffffffff, 0x7fffffffffffffff, u64::MAX];
    let mut s = 0x2545f4914f6cdd1du64;
    let mut v = Vec::new();
    for k in 0..n {
        let pick = |s: &mut u64| match rng(s) % 4 {
            0 => edge[(rng(s) % 14) as usize],
            1 => edge[(rng(s) % 14) as usize].wrapping_neg(),
            _ => rng(s),
        };
        let a = pick(&mut s);
        let b = pick(&mut s);
        let c = if k % 3 == 0 { a } else if k % 3 == 1 { (a & 0xffffffff00000000) | (rng(&mut s) & 0xff) } else { pick(&mut s) };
        v.push([a, b, c]);
    }
    v
}

/// XADD, CMPXCHG and XCHG with a memory operand, and every LOCK-prefixed instruction, are atomic read-modify-writes (mt4 stage A): not lowered here, so that their functions stay on the C path, where fe-lift emits fle_xadd,
/// fle_cas, fle_xchg (runtime/fle_rmw.h). A non-atomic lowering would be wrong once guest threads run at the same time.
#[test]
fn memory_rmw_stays_on_the_c_path() {
    for (op, n, name) in [
        (Op::Xadd, 8, "xadd"),
        (Op::Cmpxchg, 8, "cmpxchg"),
        (Op::Xchg, 8, "xchg"),
        (Op::XchgSib, 8, "xchg [r13+r12], rax"),
        (Op::HCmpxchg, 1, "lock cmpxchg [rbx+0x20], ah"),
        (Op::HXchg, 1, "xchg [rbx+0x20], ah"),
        (Op::HXadd, 1, "lock xadd [rbx+0x20], ah"),
    ] {
        let data = pe(&code_for(op, n));
        let conf = Config { export_funcs: true, ..Config::default() };
        let lowered = compile(&data, &[], &[], &conf).map(|c| c.fidx.contains_key(&0x401000)).unwrap_or(false);
        assert!(!lowered, "{name}: refused");
    }
}

/// `xchg reg, reg` (the register forms of decision 0169's xchg; the unit that found the missing coverage is the engine's size-swap): 64/32/16/8-bit pairs including r8/r9, and `xchg al, ah`. The expected values are
/// the architectural definition (32-bit forms clear the upper halves of both registers; 16/8-bit forms keep the rest).
#[test]
fn xchg_register_forms_match_the_architecture() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    // prologue: mov r8,[rcx+8]; mov rax,[rcx]; mov r9,[rcx+16]; epilogue: mov [rdx],rax; mov [rdx+8],r8; mov [rdx+16],r9; ret
    let pro: [u8; 11] = [0x4c, 0x8b, 0x41, 0x08, 0x48, 0x8b, 0x01, 0x4c, 0x8b, 0x49, 0x10];
    let epi: [u8; 13] = [0x48, 0x89, 0x02, 0x4c, 0x89, 0x42, 0x08, 0x4c, 0x89, 0x4a, 0x10, 0xc3, 0x90];
    type Exp = fn(u64, u64, u64) -> (u64, u64, u64);
    let forms: Vec<(&str, Vec<u8>, Exp)> = vec![
        ("xchg rax,r8", vec![0x49, 0x87, 0xc0], |a, b, c| (b, a, c)),
        ("xchg eax,r8d", vec![0x41, 0x87, 0xc0], |a, b, c| (b & 0xffff_ffff, a & 0xffff_ffff, c)),
        ("xchg ax,r8w", vec![0x66, 0x41, 0x87, 0xc0], |a, b, c| ((a & !0xffff) | (b & 0xffff), (b & !0xffff) | (a & 0xffff), c)),
        ("xchg al,r8b", vec![0x41, 0x86, 0xc0], |a, b, c| ((a & !0xff) | (b & 0xff), (b & !0xff) | (a & 0xff), c)),
        ("xchg r8,r9", vec![0x4d, 0x87, 0xc8], |a, b, c| (a, c, b)),
        ("xchg r8d,r9d", vec![0x45, 0x87, 0xc8], |a, b, c| (a, c & 0xffff_ffff, b & 0xffff_ffff)),
        ("xchg al,ah", vec![0x86, 0xe0], |a, b, c| ((a & !0xffff) | ((a & 0xff) << 8) | ((a >> 8) & 0xff), b, c)),
        ("xchg r8b,ah", vec![0x44, 0x86, 0xc0], |a, b, c| ((a & !0xff) | (b & 0xff), (b & !0xff) | (a & 0xff), c)),
    ];
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-xchgrr-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ins = inputs(2000);
    let mut flat: Vec<u8> = Vec::new();
    for t in &ins {
        for x in [t[0], t[1], t[2]] {
            flat.extend_from_slice(&x.to_le_bytes());
        }
        flat.extend_from_slice(&[0u8; 8]);
    }
    let inp = dir.join("in.bin");
    std::fs::write(&inp, &flat).unwrap();
    let js = dir.join("run.cjs");
    std::fs::write(
        &js,
        "const fs=require('fs');const mem=new WebAssembly.Memory({initial:16});const m=new WebAssembly.Module(fs.readFileSync(process.argv[2]));\
         const i=new WebAssembly.Instance(m,{env:{mem}});const inp=fs.readFileSync(process.argv[3]);const n=inp.length/32;const u8=new Uint8Array(mem.buffer);const out=Buffer.alloc(24*n);\
         for(let k=0;k<n;k++){u8.set(inp.subarray(32*k,32*k+32),0x10000);u8.fill(0,0x20000,0x20020);i.exports.f_401000(0x80000n,0x10000n,0x20000n,0n,0n);out.set(u8.subarray(0x20000,0x20018),24*k);}\
         fs.writeFileSync(process.argv[4],out);",
    )
    .unwrap();
    let mut failures: Vec<String> = Vec::new();
    for (name, op, exp) in &forms {
        let mut code = pro.to_vec();
        code.extend_from_slice(op);
        code.extend_from_slice(&epi);
        let conf = Config { export_funcs: true, ..Config::default() };
        let comp = match compile(&pe(&code), &[], &[], &conf) {
            Ok(x) => x,
            Err(e) => {
                failures.push(format!("{name}: compile: {e}"));
                continue;
            }
        };
        if !comp.fidx.contains_key(&0x401000) {
            failures.push(format!("{name}: not lowered"));
            continue;
        }
        let w = dir.join("t.wasm");
        std::fs::write(&w, &comp.wasm).unwrap();
        let outp = dir.join("out.bin");
        let o = Command::new("node").arg(&js).arg(&w).arg(&inp).arg(&outp).output().unwrap();
        if !o.status.success() {
            failures.push(format!("{name}: node: {}", String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
            continue;
        }
        let got = std::fs::read(&outp).unwrap();
        let (mut bad, mut first) = (0, String::new());
        for (k, t) in ins.iter().enumerate() {
            let (a, b, c) = (t[0], t[1], t[2]);
            let want = exp(a, b, c);
            let g = &got[24 * k..24 * k + 24];
            let have3 = (u64::from_le_bytes(g[0..8].try_into().unwrap()), u64::from_le_bytes(g[8..16].try_into().unwrap()), u64::from_le_bytes(g[16..24].try_into().unwrap()));
            if have3 != want {
                bad += 1;
                if first.is_empty() {
                    first = format!("rax {a:#x} r8 {b:#x} r9 {c:#x}: want {want:x?} got {have3:x?}");
                }
            }
        }
        if bad > 0 {
            failures.push(format!("{name}: {bad} of {} inputs differ; first: {first}", ins.len()));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
