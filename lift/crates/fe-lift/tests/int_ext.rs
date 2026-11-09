//! Tests for int_ext.rs: movsx, cbw, cwde, cdqe, cdq, cqo, idiv and one-operand imul.
//!
//! Each vector runs on the host CPU (ground truth) and as lifted C (clang -O0), and the
//! register state and the buffer must match. Divide errors cannot run on the host (SIGFPE),
//! so they are checked only on the C side: the program must abort and print the #DE message.

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

fn movsx_vectors() -> Vec<Vector> {
    vec![
        ins("movsx eax, bl (0x80)", &[0x0F, 0xBE, 0xC3], [0, 0, 0, 0x80, 0, 0]),
        ins("movsx eax, bl (0x7f)", &[0x0F, 0xBE, 0xC3], [u64::MAX, 0, 0, 0x7F, 0, 0]),
        ins("movsx rax, bl (0xff)", &[0x48, 0x0F, 0xBE, 0xC3], [0, 0, 0, 0xFF, 0, 0]),
        ins("movsx rcx, dl (0x80, rcx high set)", &[0x48, 0x0F, 0xBE, 0xCA], [0, u64::MAX, 0x80, 0, 0, 0]),
        ins("movsx eax, bh (0x80)", &[0x0F, 0xBE, 0xC7], [u64::MAX, 0, 0, 0x8000, 0, 0]),
        ins("movsx eax, bh (0x7f)", &[0x0F, 0xBE, 0xC7], [u64::MAX, 0, 0, 0x7F00, 0, 0]),
        ins("movsx esi, dil (0x80)", &[0x40, 0x0F, 0xBE, 0xF7], [0, 0, 0, 0, u64::MAX, 0x80]),
        ins("movsx ax, bl (0x80, upper kept)", &[0x66, 0x0F, 0xBE, 0xC3], [0xFFFF_FFFF_FFFF_1234, 0, 0, 0x80, 0, 0]),
        ins("movsx cx, dl (0x7f)", &[0x66, 0x0F, 0xBE, 0xCA], [0, 0xAAAA_BBBB_CCCC_DDDD, 0x7F, 0, 0, 0]),
        ins("movsx eax, bx (0x8000)", &[0x0F, 0xBF, 0xC3], [u64::MAX, 0, 0, 0x8000, 0, 0]),
        ins("movsx rax, bx (0x7fff)", &[0x48, 0x0F, 0xBF, 0xC3], [0, 0, 0, 0x7FFF, 0, 0]),
        ins("movsx rdx, cx (0xffff)", &[0x48, 0x0F, 0xBF, 0xD1], [0, 0xFFFF, 0, 0, 0, 0]),
        ins_mem("movsx eax, byte [rdi+1] (0x80)", &[0x0F, 0xBE, 0x47, 0x01], [u64::MAX, 0, 0, 0, 0, 0]),
        ins_mem("movsx rcx, byte [rdi+3] (0x7f)", &[0x48, 0x0F, 0xBE, 0x4F, 0x03], [0, u64::MAX, 0, 0, 0, 0]),
        ins_mem("movsx rdx, word [rdi+4] (0x8000)", &[0x48, 0x0F, 0xBF, 0x57, 0x04], [0, 0, u64::MAX, 0, 0, 0]),
        ins_mem("movsx eax, word [rdi] (0x8000)", &[0x0F, 0xBF, 0x07], [u64::MAX, 0, 0, 0, 0, 0]),
    ]
}

fn sign_ext_vectors() -> Vec<Vector> {
    vec![
        ins("cbw (AL=0x80)", &[0x66, 0x98], [0x1234_5680, 0, 0, 0, 0, 0]),
        ins("cbw (AL=0x7f)", &[0x66, 0x98], [0xFFFF_FFFF_FFFF_007F, 0, 0, 0, 0, 0]),
        ins("cbw (AL=0xff, AH=0x12)", &[0x66, 0x98], [0x1234_12FF, 0, 0, 0, 0, 0]),
        ins("cwde (AX=0x8000)", &[0x98], [0xFFFF_FFFF_0000_8000, 0, 0, 0, 0, 0]),
        ins("cwde (AX=0x7fff)", &[0x98], [0xDEAD_BEEF_0000_7FFF, 0, 0, 0, 0, 0]),
        ins("cdqe (EAX=0x80000000)", &[0x48, 0x98], [0x0000_0000_8000_0000, 0, 0, 0, 0, 0]),
        ins("cdqe (EAX=0x7fffffff)", &[0x48, 0x98], [0xFFFF_FFFF_7FFF_FFFF, 0, 0, 0, 0, 0]),
        ins("cdqe (EAX=-1)", &[0x48, 0x98], [0x0000_0000_FFFF_FFFF, 0, 0, 0, 0, 0]),
        ins("cdq (EAX=0x80000000)", &[0x99], [0x1111_1111_8000_0000, 0, 0x2222_2222_2222_2222, 0, 0, 0]),
        ins("cdq (EAX=1)", &[0x99], [1, 0, u64::MAX, 0, 0, 0]),
        ins("cdq (EAX=-1)", &[0x99], [u64::MAX, 0, 0, 0, 0, 0]),
        ins("cqo (RAX=min)", &[0x48, 0x99], [0x8000_0000_0000_0000, 0, u64::MAX, 0, 0, 0]),
        ins("cqo (RAX=max)", &[0x48, 0x99], [0x7FFF_FFFF_FFFF_FFFF, 0, u64::MAX, 0, 0, 0]),
        ins("cqo (RAX=0)", &[0x48, 0x99], [0, 0, u64::MAX, 0, 0, 0]),
    ]
}

/// idiv and imul with one operand. The divisor is in RBX (or memory at rdi).
fn idiv_imul_vectors() -> Vec<Vector> {
    vec![
        // idiv ebx, with EDX:EAX = -7 (edx = -1)
        ins("idiv ebx (-7 / 2)", &[0xF7, 0xFB], [0xFFFF_FFFF_FFFF_FFF9, 0, 0xFFFF_FFFF_FFFF_FFFF, 2, 0, 0]),
        ins("idiv ebx (7 / -2)", &[0xF7, 0xFB], [7, 0, 0, 0xFFFF_FFFE, 0, 0]),
        ins("idiv ebx (-7 / -2, rem -1)", &[0xF7, 0xFB], [0xFFFF_FFF9, 0, 0xFFFF_FFFF, 0xFFFF_FFFE, 0, 0]),
        ins("idiv rbx (-7 / 2, 64-bit)", &[0x48, 0xF7, 0xFB], [0xFFFF_FFFF_FFFF_FFF9, 0, u64::MAX, 2, 0, 0]),
        ins("idiv rbx (2^63 / 3, 128-bit dividend)", &[0x48, 0xF7, 0xFB], [0x8000_0000_0000_0000, 0, 0, 3, 0, 0]),
        ins("idiv rbx (-1 * 2^64 / 2)", &[0x48, 0xF7, 0xFB], [0, 0, u64::MAX, 2, 0, 0]),
        ins("idiv rbx (max / min, 64-bit)", &[0x48, 0xF7, 0xFB], [u64::MAX, 0, 0, 0x8000_0000_0000_0000, 0, 0]),
        ins("idiv bx (-7 / 2, 16-bit)", &[0x66, 0xF7, 0xFB], [0xFFFF_FFFF_FFFF_FFF9, 0, 0xFFFF_FFFF_FFFF_FFFF, 0x2, 0, 0]),
        ins("idiv bl (-7 / 2, 8-bit AX)", &[0xF6, 0xFB], [0xFFFF_FFFF_FFFF_FFF9, 0, 0xFFFF_FFFF_FFFF_FFFF, 0x2, 0, 0]),
        ins("idiv bl (-128 / 1, 8-bit)", &[0xF6, 0xFB], [0xFF80, 0, 0xFFFF_FFFF_FFFF_FFFF, 0x01, 0, 0]),
        ins("idiv bl (-128 / -1 is #DE)", &[0xF6, 0xFB], [0x80, 0, 0, 0, 0, 0]),
        ins_mem("idiv dword [rdi] (-9 / pattern)", &[0xF7, 0x3F], [0xFFFF_FFF7, 0, u64::MAX, 0, 0, 0]),
        ins_mem("idiv qword [rdi] (-9 / pattern)", &[0x48, 0xF7, 0x3F], [0xFFFF_FFFF_FFFF_FFF7, 0, u64::MAX, 0, 0, 0]),
        ins("imul bl (-3 * 5, AX)", &[0xF6, 0xEB], [0xFFFF_FFFF_FFFF_FFFD, 0, 0, 5, 0, 0]),
        ins("imul bl (0x40 * 2, OF=CF=1)", &[0xF6, 0xEB], [0x40, 0, 0, 2, 0, 0]),
        ins("imul bx (0x4000 * 2)", &[0x66, 0xF7, 0xEB], [0x4000, 0xAAAA_AAAA_AAAA_AAAA, 0, 2, 0, 0]),
        ins("imul ebx (-1 * -1)", &[0xF7, 0xEB], [u64::MAX, 0, 0x1234_5678, 0xFFFF_FFFF, 0, 0]),
        ins("imul ebx (0x10000 * 0x10000 overflow)", &[0xF7, 0xEB], [0x1_0000, 0, 0, 0x1_0000, 0, 0]),
        ins("imul rbx (-2^63 * -1)", &[0x48, 0xF7, 0xEB], [0x8000_0000_0000_0000, 0, 0, u64::MAX, 0, 0]),
        ins("imul rbx (-3 * 4, RDX:RAX)", &[0x48, 0xF7, 0xEB], [u64::MAX - 2, 0, 0, 4, 0, 0]),
        ins("imul rbx (2^32 * 2^32)", &[0x48, 0xF7, 0xEB], [0x1_0000_0000, 0, 0, 0x1_0000_0000, 0, 0]),
        ins_mem("imul qword [rdi] (-5 * pattern)", &[0x48, 0xF7, 0x2F], [0xFFFF_FFFF_FFFF_FFFB, 0, 0, 0, 0, 0]),
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

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-int-ext/test");
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    let uname = etxt_common::uniq(group);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, &src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"])
        .arg(&rt)
        .arg(rt.join("fle_int_ext.c"))
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
fn movsx_matches_host() {
    check("movsx", &movsx_vectors());
}

#[test]
fn sign_extension_matches_host() {
    check("signext", &sign_ext_vectors());
}

/// Host-checked vectors only. The #DE cases below are excluded because the host would SIGFPE.
fn idiv_imul_host_vectors() -> Vec<Vector> {
    idiv_imul_vectors().into_iter().filter(|v| !v.name.contains("#DE")).collect()
}

#[test]
fn idiv_imul_matches_host() {
    check("idiv_imul", &idiv_imul_host_vectors());
}

/// C side only: the divide error must stop the program. The host cannot run these (SIGFPE).
#[test]
fn divide_error_aborts() {
    let cases: [(&str, &[u8], [u64; 6]); 3] = [
        ("de_zero", &[0xF7, 0xFB], [1, 0, 0, 0, 0, 0]),
        // EDX:EAX = -2^63 (32-bit idiv), divisor -1: the quotient 2^63 does not fit in 32 bits.
        ("de_min_over_m1", &[0xF7, 0xFB], [0, 0, 0x8000_0000, 0xFFFF_FFFF, 0, 0]),
        ("de_quot_range", &[0xF6, 0xFB], [0x8000, 0, 0, 0x01, 0, 0]),
    ];
    for (name, bytes, regs) in cases {
        let body = lift_body(bytes, 0x1000).expect("lift");
        let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-int-ext/test");
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
            .arg(rt.join("fle_int_ext.c"))
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
