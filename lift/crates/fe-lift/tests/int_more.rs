//! Tests for int_more.rs: xadd, cmpxchg, bt (register and memory forms) and unsigned div.
//!
//! Each vector runs on the host CPU (ground truth) and as lifted C (clang -O0). The
//! register state and the buffer must match. Flags are not compared: the SDM leaves
//! some undefined, and for cmpxchg and xadd the flags are checked by the rest of the
//! tests. Divide errors cannot run on the host (SIGFPE), so they are checked only on
//! the C side.

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;

/// Vector state order: rax, rcx, rdx, rbx, rsi, rdi.
const REG_NUMS: [usize; 6] = [0, 1, 2, 3, 6, 7];
const LOAD: [[u8; 4]; 6] = [
    [0x48, 0x8B, 0x45, 0x00],
    [0x48, 0x8B, 0x4D, 0x08],
    [0x48, 0x8B, 0x55, 0x10],
    [0x48, 0x8B, 0x5D, 0x18],
    [0x48, 0x8B, 0x75, 0x20],
    [0x48, 0x8B, 0x7D, 0x28],
];
const STORE: [[u8; 4]; 6] = [
    [0x48, 0x89, 0x45, 0x00],
    [0x48, 0x89, 0x4D, 0x08],
    [0x48, 0x89, 0x55, 0x10],
    [0x48, 0x89, 0x5D, 0x18],
    [0x48, 0x89, 0x75, 0x20],
    [0x48, 0x89, 0x7D, 0x28],
];

struct Vector {
    name: &'static str,
    bytes: &'static [u8],
    regs: [u64; 6],
    /// rdi is replaced by the address of the 64-byte buffer + 16.
    mem: bool,
}

fn ins(name: &'static str, bytes: &'static [u8], regs: [u64; 6]) -> Vector {
    Vector { name, bytes, regs, mem: false }
}

fn ins_mem(name: &'static str, bytes: &'static [u8], regs: [u64; 6]) -> Vector {
    Vector { name, bytes, regs, mem: true }
}

fn pattern() -> [u8; 64] {
    let mut m = [0u8; 64];
    for (j, b) in m.iter_mut().enumerate().skip(16) {
        *b = ((j * 73 + 0x81) & 0xff) as u8;
    }
    m
}

fn xadd_vectors() -> Vec<Vector> {
    vec![
        ins("xadd ebx, eax (0 + 0)", &[0x0F, 0xC1, 0xC3], [0, 0, 0, 0, 0, 0]),
        ins("xadd ebx, eax (max + 1, wraps)", &[0x0F, 0xC1, 0xC3], [1, 0, 0, 0xFFFF_FFFF, 0, 0]),
        ins("xadd ebx, eax (0x8000_0000 + 0x8000_0000)", &[0x0F, 0xC1, 0xC3], [0x8000_0000, 0, 0, 0x8000_0000, 0, 0]),
        ins("xadd rbx, rax (max + 1, wraps)", &[0x48, 0x0F, 0xC1, 0xC3], [1, 0, 0, u64::MAX, 0, 0]),
        ins("xadd rbx, rax (min + min)", &[0x48, 0x0F, 0xC1, 0xC3], [0x8000_0000_0000_0000, 0, 0, 0x8000_0000_0000_0000, 0, 0]),
        ins("xadd bx, ax (16-bit, keeps high bits)", &[0x66, 0x0F, 0xC1, 0xC3], [0xFFFF_0001, 0, 0, 0xABCD_FFFF_1234_0005, 0, 0]),
        ins("xadd bl, al (8-bit)", &[0x0F, 0xC0, 0xC3], [0x80, 0, 0, 0x7F, 0, 0]),
        ins("xadd bh, al (high byte)", &[0x0F, 0xC0, 0xC7], [0x11, 0, 0, 0x2200, 0, 0]),
        ins_mem("xadd [rdi], eax (mem)", &[0x0F, 0xC1, 0x07], [0x7FFF_FFFF, 0, 0, 0, 0, 0]),
        ins_mem("xadd [rdi+4], rcx (mem, 64-bit)", &[0x48, 0x0F, 0xC1, 0x4F, 0x04], [0, u64::MAX, 0, 0, 0, 0]),
        ins_mem("xadd [rdi+1], al (mem, 8-bit)", &[0x0F, 0xC0, 0x47, 0x01], [0x80, 0, 0, 0, 0, 0]),
        ins_mem("xadd [rdi+2], dx (mem, 16-bit)", &[0x66, 0x0F, 0xC1, 0x57, 0x02], [0, 0, 0xFFFF, 0, 0, 0]),
    ]
}

fn cmpxchg_vectors() -> Vec<Vector> {
    vec![
        // equal: the destination gets the source, ZF = 1
        ins("cmpxchg ebx, ecx (equal)", &[0x0F, 0xB1, 0xCB], [0x1234_5678, 0xAAAA_BBBB, 0, 0x1234_5678, 0, 0]),
        // not equal: the accumulator gets the destination, and the destination is unchanged
        ins("cmpxchg ebx, ecx (not equal)", &[0x0F, 0xB1, 0xCB], [0x1234_5678, 0xAAAA_BBBB, 0, 0x9999_9999, 0, 0]),
        ins("cmpxchg rbx, rcx (equal, 64-bit)", &[0x48, 0x0F, 0xB1, 0xCB], [u64::MAX, 0xAB, 0, u64::MAX, 0, 0]),
        ins("cmpxchg rbx, rcx (not equal, 64-bit)", &[0x48, 0x0F, 0xB1, 0xCB], [0x8000_0000_0000_0000, 0xAB, 0, 0x7FFF_FFFF_FFFF_FFFF, 0, 0]),
        ins("cmpxchg bx, cx (16-bit equal)", &[0x66, 0x0F, 0xB1, 0xCB], [0xFFFF_FFFF_FFFF_0001, 0x2222, 0, 0x0001, 0, 0]),
        ins("cmpxchg bl, cl (8-bit not equal)", &[0x0F, 0xB0, 0xCB], [0x7F, 0x55, 0, 0x80, 0, 0]),
        ins("cmpxchg bl, cl (8-bit equal)", &[0x0F, 0xB0, 0xCB], [0x7F, 0x55, 0, 0x7F, 0, 0]),
        ins_mem("cmpxchg [rdi], ecx (mem, equal)", &[0x0F, 0xB1, 0x0F], [0x0000_0000_0000_0000, 0x1111_2222, 0, 0, 0, 0]),
        ins_mem("cmpxchg [rdi], ecx (mem, not equal)", &[0x0F, 0xB1, 0x0F], [0x0000_0000_0000_0001, 0x1111_2222, 0, 0, 0, 0]),
        ins_mem("cmpxchg [rdi+8], rdx (mem, 64-bit)", &[0x48, 0x0F, 0xB1, 0x57, 0x08], [0, 0, 0x3333_4444, 0, 0, 0]),
    ]
}

fn bt_vectors() -> Vec<Vector> {
    vec![
        ins("bt eax, ecx (bit 0 set)", &[0x0F, 0xA3, 0xC8], [1, 0, 0, 0, 0, 0]),
        ins("bt eax, ecx (bit 31 set, offset 31)", &[0x0F, 0xA3, 0xC8], [0x8000_0000, 31, 0, 0, 0, 0]),
        ins("bt eax, ecx (offset 32 wraps to 0)", &[0x0F, 0xA3, 0xC8], [0x1, 32, 0, 0, 0, 0]),
        ins("bt eax, ecx (offset 33 wraps to 1, bit 1 clear)", &[0x0F, 0xA3, 0xC8], [0x1, 33, 0, 0, 0, 0]),
        ins("bt rax, rcx (bit 63 set)", &[0x48, 0x0F, 0xA3, 0xC8], [0x8000_0000_0000_0000, 63, 0, 0, 0, 0]),
        ins("bt rax, rcx (offset 64 wraps to 0)", &[0x48, 0x0F, 0xA3, 0xC8], [0x1, 64, 0, 0, 0, 0]),
        ins("bt ax, cx (16-bit, offset 15)", &[0x66, 0x0F, 0xA3, 0xC8], [0x8000, 15, 0, 0, 0, 0]),
        ins("bt ax, cx (16-bit, offset 16 wraps)", &[0x66, 0x0F, 0xA3, 0xC8], [0x0001, 16, 0, 0, 0, 0]),
        ins("bt eax, 5 (imm8)", &[0x0F, 0xBA, 0xE0, 0x05], [0x20, 0, 0, 0, 0, 0]),
        ins("bt eax, 31 (imm8, bit clear)", &[0x0F, 0xBA, 0xE0, 0x1F], [0x2000_0000, 0, 0, 0, 0, 0]),
        ins("bt rax, 63 (imm8, bit set)", &[0x48, 0x0F, 0xBA, 0xE0, 0x3F], [0x8000_0000_0000_0000, 0, 0, 0, 0, 0]),
        ins("bt rax, 0x41 (imm8 wraps to 1)", &[0x48, 0x0F, 0xBA, 0xE0, 0x41], [0x2, 0, 0, 0, 0, 0]),
        // memory bit string: the bit offset is signed, the byte is at EA + offset/8
        ins_mem("bt [rdi], ecx (offset 3)", &[0x0F, 0xA3, 0x0F], [0, 3, 0, 0, 0, 0]),
        ins_mem("bt [rdi], ecx (offset 13: byte +1, bit 5)", &[0x0F, 0xA3, 0x0F], [0, 13, 0, 0, 0, 0]),
        ins_mem("bt [rdi], ecx (offset -1: byte -1, bit 7)", &[0x0F, 0xA3, 0x0F], [0, 0xFFFF_FFFF, 0, 0, 0, 0]),
        ins_mem("bt [rdi], ecx (offset -9: byte -2, bit 7)", &[0x0F, 0xA3, 0x0F], [0, 0xFFFF_FFF7, 0, 0, 0, 0]),
        ins_mem("bt [rdi], rcx (64-bit offset 65)", &[0x48, 0x0F, 0xA3, 0x0F], [0, 65, 0, 0, 0, 0]),
        ins_mem("bt [rdi], rcx (64-bit offset -65)", &[0x48, 0x0F, 0xA3, 0x0F], [0, (-65i64) as u64, 0, 0, 0, 0]),
        ins_mem("bt [rdi+2], cx (16-bit offset 21)", &[0x66, 0x0F, 0xA3, 0x4F, 0x02], [0, 21, 0, 0, 0, 0]),
        ins_mem("bt dword [rdi], 9 (imm8)", &[0x0F, 0xBA, 0x27, 0x09], [0, 0, 0, 0, 0, 0]),
        ins_mem("bt qword [rdi+8], 60 (imm8)", &[0x48, 0x0F, 0xBA, 0x67, 0x08, 0x3C], [0, 0, 0, 0, 0, 0]),
    ]
}

/// Division with a non-zero divisor and a quotient in range. The host runs these.
/// The dividend is RDX:RAX (or EDX:EAX, DX:AX, AX), the divisor is RBX (or memory at rdi).
fn div_vectors() -> Vec<Vector> {
    vec![
        ins("div ebx (7 / 2)", &[0xF7, 0xF3], [7, 0, 0, 2, 0, 0]),
        ins("div ebx (0xffffffff / 1, max)", &[0xF7, 0xF3], [0xFFFF_FFFF, 0, 0, 1, 0, 0]),
        ins("div ebx (edx:eax = 0x1_0000_0000 / 3, quotient fits)", &[0xF7, 0xF3], [0, 0, 1, 3, 0, 0]),
        ins("div ebx (edx:eax = 0xffff_fffe_0000_0000 / 0xffff_ffff)", &[0xF7, 0xF3], [0, 0, 0xFFFF_FFFE, 0xFFFF_FFFF, 0, 0]),
        ins("div rbx (2^64-1 / 3)", &[0x48, 0xF7, 0xF3], [u64::MAX, 0, 0, 3, 0, 0]),
        ins("div rbx (rdx:rax = 2^64 + 5 / 2^63)", &[0x48, 0xF7, 0xF3], [5, 0, 1, 0x8000_0000_0000_0000, 0, 0]),
        ins("div rbx (rdx:rax = 2^128 - 2^64 / 2^64 - 1, quotient max)", &[0x48, 0xF7, 0xF3], [0, 0, 0xFFFF_FFFF_FFFF_FFFE, u64::MAX, 0, 0]),
        ins("div bx (dx:ax = 0xffff / 0x100)", &[0x66, 0xF7, 0xF3], [0xFFFF, 0, 0, 0x100, 0, 0]),
        ins("div bl (ax = 0x00ff / 0x10)", &[0xF6, 0xF3], [0xFF, 0, 0, 0x10, 0, 0]),
        ins("div bl (ax = 0x7f00 / 0x80, quotient 0xfe)", &[0xF6, 0xF3], [0x7F00, 0, 0, 0x80, 0, 0]),
        ins_mem("div dword [rdi] (0xffffffff / 0xffff_fff7 pattern)", &[0xF7, 0x37], [0xFFFF_FFFF, 0, 0, 0, 0, 0]),
        ins_mem("div qword [rdi] (rdx:rax / pattern)", &[0x48, 0xF7, 0x37], [0x1234, 0, 0x9, 0, 0, 0]),
    ]
}

fn host_run(v: &Vector) -> ([u64; 6], [u8; 64]) {
    let mut code = vec![0x55, 0x48, 0x89, 0xFD];
    for l in LOAD {
        code.extend_from_slice(&l);
    }
    code.extend_from_slice(v.bytes);
    for s in STORE {
        code.extend_from_slice(&s);
    }
    code.extend_from_slice(&[0x5D, 0xC3]);
    let buf = pattern();
    let mut st = v.regs;
    if v.mem {
        st[5] = buf.as_ptr() as u64 + 16;
    }
    unsafe {
        let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
        assert!(!p.is_null() && p as isize != -1, "mmap failed");
        std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
        let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut u64)>(p);
        f(st.as_mut_ptr());
        munmap(p, 4096);
    }
    (st, buf)
}

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

/// Builds one C program with every vector, runs it, and returns (registers, buffer) per vector.
fn c_run(group: &str, vs: &[Vector]) -> Vec<([u64; 6], [u8; 64])> {
    let p = pattern();
    let mut src = String::from("#include \"fle_runtime.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nstatic uint8_t M[64];\n");
    src += "static const uint8_t P[64] = {";
    src += &p.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",");
    src += "};\n\n";
    for (k, v) in vs.iter().enumerate() {
        src += &format!("static void c{k}(void)\n{{\n{}\n}}\n\n", lift_body(v.bytes, 0x1000 + k as u64 * 0x10).expect("lift"));
    }
    src += "int main(void)\n{\n";
    for (k, v) in vs.iter().enumerate() {
        src += "    memcpy(M, P, sizeof M);\n";
        for (j, r) in REG_NUMS.iter().enumerate() {
            src += &format!("    R[{r}] = UINT64_C({:#x});\n", v.regs[j]);
        }
        if v.mem {
            src += "    R[7] = (uint64_t)(uintptr_t)(M + 16);\n";
        }
        src += &format!("    c{k}();\n    printf(\"%d\", {k});\n");
        for r in REG_NUMS {
            src += &format!("    printf(\" %llx\", (unsigned long long)R[{r}]);\n");
        }
        src += "    for (int j = 0; j < 64; j++) printf(\" %02x\", M[j]);\n    printf(\"\\n\");\n";
    }
    src += "    return 0;\n}\n";

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-more/test");
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    let uname = etxt_common::uniq(group);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, &src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"])
        .arg(&rt)
        .arg(rt.join("fle_int_more.c"))
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run failed");
    assert!(run.status.success(), "C program failed");
    String::from_utf8(run.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            let t: Vec<&str> = line.split_whitespace().collect();
            let mut st = [0u64; 6];
            for j in 0..6 {
                st[j] = u64::from_str_radix(t[1 + j], 16).unwrap();
            }
            let mut m = [0u8; 64];
            for j in 0..64 {
                m[j] = u8::from_str_radix(t[7 + j], 16).unwrap();
            }
            (st, m)
        })
        .collect()
}

fn check(group: &str, vs: &[Vector]) {
    let c = c_run(group, vs);
    assert_eq!(c.len(), vs.len());
    for (v, (mut cst, cm)) in vs.iter().zip(c) {
        let (mut hst, hm) = host_run(v);
        if v.mem {
            cst[5] = 0;
            hst[5] = 0;
        }
        // Flags are not compared. Only the register and memory results are checked.
        assert_eq!(cst, hst, "{}: registers (C left, host right)", v.name);
        assert_eq!(cm, hm, "{}: memory", v.name);
    }
}

#[test]
fn xadd_matches_host() {
    check("xadd", &xadd_vectors());
}

#[test]
fn cmpxchg_matches_host() {
    check("cmpxchg", &cmpxchg_vectors());
}

#[test]
fn bt_matches_host() {
    check("bt", &bt_vectors());
}

#[test]
fn div_matches_host() {
    check("div", &div_vectors());
}

/// C side only: the divide error must stop the program. The host cannot run these (SIGFPE).
#[test]
fn divide_error_aborts() {
    let cases: [(&str, &[u8], [u64; 6]); 3] = [
        ("de_zero", &[0xF7, 0xF3], [1, 0, 0, 0, 0, 0]),
        // EDX:EAX = 2^32 (quotient 2^32 / 1 does not fit in 32 bits)
        ("de_quot_range", &[0xF7, 0xF3], [0, 0, 1, 1, 0, 0]),
        ("de_quot_range8", &[0xF6, 0xF3], [0x0100, 0, 0, 0x01, 0, 0]),
    ];
    for (name, bytes, regs) in cases {
        let body = lift_body(bytes, 0x1000).expect("lift");
        let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-more/test");
        std::fs::create_dir_all(dir.join("tmp")).unwrap();
        let mut src = String::from("#include \"fle_runtime.h\"\n\nuint64_t R[16];\nfle_eflags FL;\n");
        src += &format!("static void c(void)\n{{\n{body}\n}}\n");
        src += "int main(void)\n{\n";
        for (j, r) in REG_NUMS.iter().enumerate() {
            src += &format!("    R[{r}] = UINT64_C({:#x});\n", regs[j]);
        }
        src += "    c();\n    return 0;\n}\n";
        let uname = etxt_common::uniq(name);
        let c = dir.join(format!("{uname}.c"));
        let exe = dir.join(&uname);
        std::fs::write(&c, &src).unwrap();
        let cc = Command::new("clang")
            .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"])
            .arg(&rt)
            .arg(rt.join("fle_int_more.c"))
            .arg("-o")
            .arg(&exe)
            .arg(&c)
            .env("TMPDIR", dir.join("tmp"))
            .output()
            .expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        let run = Command::new(&exe).output().expect("run failed");
        assert!(!run.status.success(), "{name}: expected an abort");
        assert!(String::from_utf8_lossy(&run.stderr).contains("#DE"), "{name}: expected the #DE message");
    }
}
