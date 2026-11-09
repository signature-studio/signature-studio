//! Tests of decision 0190 (docs/0190-engine-serum-process.md, PROVISIONAL): wall W12 of decision 0183 (CreateThread read a wrong dwCreationFlags inside Serum2.vst3's `_beginthreadex`).
//!
//! Cause (measured with gdb on the native -O0 engine): runtime/fle_modules.h declared `fle_imp_GetModuleHandleExW` with `__attribute__((noreturn))` (it stops by name, decision 0078) while the
//! wrapper build (harness/two/wrp1_vst3_imp.c) answers it and returns. The lifted units that call it include fle_modules.h, so clang -O0 dropped the code after the call (the caller's `R[4] += 8` and
//! the next instructions of the lifted block): the guest stack lost 8 bytes per call.
//!
//!  1. The declaration is `noreturn` by default (nothing changes for any build that does not ask) and is a plain declaration with -DFLE_GMHEX_RETURNS: checked on the LLVM IR of a lifted-style
//!     call block (`unreachable` after the call exactly when the attribute is present).
//!  2. No function that fle_modules.h declares `noreturn` is wrapped by a harness (`__wrap_fle_imp_<name>`) except GetModuleHandleExW, whose declaration is conditional. A new wrapped name
//!     that the lifted units would see as noreturn would reintroduce the same defect.
//!  3. tools: the wrapper build script passes the macro only for the units that call the function (cycle.sh is a work-directory tool and is not in git; this part pins only the header).

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn lift_dir() -> PathBuf {
    manifest().join("../..")
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_WRP2_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => lift_dir().join("runtime"),
    }
}
fn work() -> PathBuf {
    let d = lift_dir().join("out-wrp2/test");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn ir(name: &str, extra: &[&str]) -> String {
    let dir = work();
    let c = dir.join(format!("{name}.c"));
    let ll = dir.join(format!("{name}.ll"));
    std::fs::write(
        &c,
        "#include <stdint.h>\n#include \"fle_modules.h\"\nextern uint64_t R[16];\nvoid lifted_block(void)\n{ R[4] -= 8; fle_imp_GetModuleHandleExW(); R[4] += 8; R[0] = R[3]; }\n",
    )
    .unwrap();
    let out = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-S", "-emit-llvm", "-fPIE"])
        .args(extra)
        .arg("-I")
        .arg(rt())
        .arg(&c)
        .arg("-o")
        .arg(&ll)
        .output()
        .expect("clang not found");
    assert!(out.status.success(), "clang failed: {}", String::from_utf8_lossy(&out.stderr));
    std::fs::read_to_string(&ll).unwrap()
}

#[test]
fn getmodulehandleexw_returns_in_every_build() {
    // decision 0243: always (old: opt-in with -DFLE_GMHEX_RETURNS, noreturn by default); the same IR with the macro defined is the same IR
    let plain = ir("gmhex_plain", &[]);
    let opt = ir("gmhex_opt", &["-DFLE_GMHEX_RETURNS"]);
    let body = |s: &str| {
        let a = s.find("define").expect("no function in the IR");
        s[a..].to_string()
    };
    let (p, o) = (body(&plain), body(&opt));
    assert!(!p.contains("unreachable"), "the call returns and the block continues:\n{p}");
    assert_eq!(p, o, "the macro is gone: defining it changes nothing");
    // the continuation is really emitted: the second stack-pointer update (`R[4] += 8`) is a second load/store pair of R+32
    let count = |s: &str| s.matches("getelementptr inbounds ([16 x i64], ptr @R, i64 0, i64 4)").count();
    assert!(count(&p) >= 2, "the code after the call is kept (R[4] += 8): {}", count(&p));
}

fn read_dir_c(dir: &PathBuf, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            if n.starts_with("out-") || n == "target" || n == "crates" {
                continue;
            }
            read_dir_c(&p, out);
        } else if p.extension().map(|x| x == "c").unwrap_or(false) {
            out.push(p);
        }
    }
}

#[test]
fn no_noreturn_name_of_fle_modules_h_is_wrapped_by_a_harness_except_the_conditional_one() {
    let hdr = std::fs::read_to_string(rt().join("fle_modules.h")).unwrap();
    let mut names: Vec<String> = Vec::new();
    let mut in_opt = false;
    for l in hdr.lines() {
        let t = l.trim();
        if t.starts_with("#ifdef FLE_GMHEX_RETURNS") {
            in_opt = true;
        } else if t.starts_with("#else") || t.starts_with("#endif") {
            in_opt = false;
        }
        if t.contains("__attribute__((noreturn))") && !t.starts_with("/*") && !t.starts_with("*") {
            if let Some(a) = t.find("fle_imp_") {
                let rest = &t[a..];
                let n: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                names.push(n);
            }
        }
        let _ = in_opt;
    }
    assert!(!names.iter().any(|n| n == "fle_imp_GetModuleHandleExW"), "decision 0243: GetModuleHandleExW returns, it is not declared noreturn: {names:?}");
    let mut files = Vec::new();
    read_dir_c(&lift_dir().join("harness"), &mut files);
    let mut wrapped: Vec<String> = Vec::new();
    for f in files {
        let s = std::fs::read_to_string(&f).unwrap_or_default();
        for l in s.lines() {
            if let Some(a) = l.find("__wrap_fle_imp_") {
                if l[..a].trim().is_empty() || l[..a].trim() == "void" {
                    let n: String = l[a + 7..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                    wrapped.push(n);
                }
            }
        }
    }
    let bad: Vec<&String> = names.iter().filter(|n| wrapped.contains(n) && n.as_str() != "fle_imp_GetModuleHandleExW").collect();
    assert!(bad.is_empty(), "a harness wraps (and so makes return) a function that fle_modules.h declares noreturn: {bad:?}");
    assert!(wrapped.iter().any(|n| n == "fle_imp_GetModuleHandleExW"), "wrp1_vst3_imp.c is expected to wrap GetModuleHandleExW (the reason for the macro)");
}
