//! A synthetic PE with the integer kernel K3 (xorshift, table lookup, rotate; lift/out-wasmgen1 kbench) goes through the whole pipeline.
//! The wasm module is validated and run with the tools that the machine has (`wasm-validate`, `node`); a test whose tool is missing prints
//! SKIPPED and passes, as the other tests of this repository do.

use fe_wasmgen::{compile, Config};
use std::io::Write;
use std::process::Command;

/// k3_int of lift/out-wasmgen1/kbench/k.s
const K3: &[u8] = &[
    0xb8, 0x91, 0xf4, 0x45, 0x25, // mov eax,0x2545f491
    0x45, 0x31, 0xd2, // xor r10d,r10d
    0x41, 0x89, 0xc3, // 1: mov r11d,eax
    0x41, 0xc1, 0xe3, 0x0d, // shl r11d,13
    0x44, 0x31, 0xd8, // xor eax,r11d
    0x41, 0x89, 0xc3, // mov r11d,eax
    0x41, 0xc1, 0xeb, 0x11, // shr r11d,17
    0x44, 0x31, 0xd8, // xor eax,r11d
    0x41, 0x89, 0xc3, // mov r11d,eax
    0x41, 0xc1, 0xe3, 0x05, // shl r11d,5
    0x44, 0x31, 0xd8, // xor eax,r11d
    0x44, 0x0f, 0xb6, 0xd8, // movzx r11d,al
    0x46, 0x8b, 0x1c, 0x99, // mov r11d,[rcx+r11*4]
    0x45, 0x01, 0xda, // add r10d,r11d
    0x41, 0xc1, 0xc2, 0x03, // rol r10d,3
    0x44, 0x89, 0x12, // mov [rdx],r10d
    0x48, 0x83, 0xc2, 0x04, // add rdx,4
    0x49, 0xff, 0xc8, // dec r8
    0x75, 0xc7, // jnz 1b
    0xc3, // ret
];

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

/// A minimal PE32+ with .text (the code at RVA 0x1000) and .pdata (one function), image base 0x400000.
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

#[test]
fn k3_compiles_validates_and_computes_the_reference_result() {
    let data = pe(K3);
    let conf = Config { export_funcs: true, ..Config::default() };
    let c = compile(&data, &[], &[], &conf).expect("compile");
    assert_eq!(&c.wasm[..4], b"\0asm");
    let ok = c.report.rows.iter().filter(|r| r.status == "ok").count();
    assert_eq!(ok, 1, "the kernel must be lowered");
    assert!(c.fidx.contains_key(&0x401000));

    let dir = std::env::temp_dir().join(format!("fe-wasmgen-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = dir.join("k3.wasm");
    std::fs::write(&wasm, &c.wasm).unwrap();
    if have("wasm-validate") {
        let o = Command::new("wasm-validate").arg(&wasm).output().unwrap();
        assert!(o.status.success(), "wasm-validate: {}", String::from_utf8_lossy(&o.stderr));
    } else {
        eprintln!("SKIPPED wasm-validate: not installed");
    }
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    // reference: xorshift32 + table lookup + add + rol 3, 64 samples
    let tab: Vec<u32> = (0..256u32).map(|i| i.wrapping_mul(2654435761)).collect();
    let mut want: Vec<u32> = Vec::new();
    let (mut eax, mut r10) = (0x2545f491u32, 0u32);
    for _ in 0..64 {
        eax ^= eax << 13;
        eax ^= eax >> 17;
        eax ^= eax << 5;
        r10 = r10.wrapping_add(tab[(eax & 0xff) as usize]).rotate_left(3);
        want.push(r10);
    }
    let js = dir.join("run.cjs");
    let mut f = std::fs::File::create(&js).unwrap();
    write!(
        f,
        "const fs=require('fs');const mem=new WebAssembly.Memory({{initial:16}});const m=new WebAssembly.Module(fs.readFileSync(process.argv[2]));\
         const i=new WebAssembly.Instance(m,{{env:{{mem}}}});const dv=new DataView(mem.buffer);\
         for(let k=0;k<256;k++)dv.setUint32(0x10000+4*k,Math.imul(k,2654435761)>>>0,true);\
         i.exports.f_401000(0x80000n,0x10000n,0x20000n,64n,0n);\
         const o=[];for(let k=0;k<64;k++)o.push(dv.getUint32(0x20000+4*k,true));console.log(o.join(' '));"
    )
    .unwrap();
    drop(f);
    let o = Command::new("node").arg(&js).arg(&wasm).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(o.status.success(), "node: {}", String::from_utf8_lossy(&o.stderr));
    let got: Vec<u32> = String::from_utf8_lossy(&o.stdout).split_whitespace().map(|x| x.parse().unwrap()).collect();
    assert_eq!(got, want);
}
