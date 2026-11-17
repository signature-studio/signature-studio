//! Tests of decision 0204 (docs/0204-noreturn-audit.md, PROVISIONAL): a header that declares a function `__attribute__((noreturn))` lets clang drop the code after a call to it, so an
//! implementation that does return (a harness `__wrap_<name>`, a strong definition) contradicts the declaration. Decision 0190 (wall W12) was `fle_imp_GetModuleHandleExW`.
//! The detector is harness/noreturn-audit/noreturn_audit.py (clang AST for the declarations, `clang -S -emit-llvm` for the paths that reach a `ret`); the accepted findings, each with
//! its reason, are harness/noreturn-audit/allow.tsv.
//!
//!  1. The real tree: nothing outside the allow list, no allow row without a finding, and the five names that decision 0190 listed (GetModuleHandleExW, GetFileType, SetFilePointerEx,
//!     SleepConditionVariableSRW, RtlPcToFileHeader) are found (the detector is not blind).
//!  2. The detector on a small synthetic tree: an early return, an unconditional return, a macro-made wrapper, a strong definition and a macro-conditional declaration are found; a wrapper
//!     that only forwards to `__real_` (directly or by passing it to a helper) is not; a file that cannot be compiled is reported; a function that stops but is declared as returning is listed.
//!  3. Mutations of the real tree (each must make the tool fail): a new wrapper that returns for a name of fle_modules.h; a new noreturn declaration for a name that a harness answers; the declaring header of an accepted name coming into the include
//!     closure of the generated units; the opt-in macro of fle_modules.h made unconditional; an allow row that matches nothing; a declaration changed from noreturn to plain.

use std::path::{Path, PathBuf};
use std::process::Command;

fn lift_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}
fn tool() -> PathBuf {
    lift_dir().join("harness/noreturn-audit/noreturn_audit.py")
}
fn allow() -> PathBuf {
    lift_dir().join("harness/noreturn-audit/allow.tsv")
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fle-noreturn-audit-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_tool(args: &[&str]) -> (i32, String) {
    let out = Command::new("python3").arg(tool()).args(args).output().expect("python3 not found");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), s)
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let (p, q) = (e.path(), to.join(e.file_name()));
        if p.is_dir() {
            copy_dir(&p, &q);
        } else {
            std::fs::copy(&p, &q).unwrap();
        }
    }
}

/// A private copy of the pieces of lift/ that the tool reads: runtime/, the three harness directories that carry the accepted findings (two/, vst/, vst3a/) (plus whatever the caller adds), the generator's sources.
fn mutant_lift(name: &str) -> PathBuf {
    let root = scratch(name);
    let lift = lift_dir();
    copy_dir(&lift.join("runtime"), &root.join("runtime"));
    copy_dir(&lift.join("crates/fe-lift/src"), &root.join("crates/fe-lift/src"));
    std::fs::create_dir_all(root.join("harness")).unwrap();
    for e in std::fs::read_dir(lift.join("harness")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "h").unwrap_or(false) {
            std::fs::copy(&p, root.join("harness").join(p.file_name().unwrap())).unwrap();
        }
    }
    for d in ["two", "vst3a", "dec1"] {
        copy_dir(&lift.join("harness").join(d), &root.join("harness").join(d));
    }
    copy_dir(&lift.join("../archive/harness-attic/vst"), &root.join("harness/vst"));
    root
}

fn fast(root: &Path, allow: &Path, extra: &[&str]) -> (i32, String) {
    let work = root.join("work");
    let mut v = vec!["--lift", root.to_str().unwrap(), "--allow", allow.to_str().unwrap(), "--fast", "--no-gen-wraps", "--work", work.to_str().unwrap()];
    v.extend_from_slice(extra);
    run_tool(&v)
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

#[test]
fn the_real_tree_has_no_returning_implementation_of_a_noreturn_name_outside_the_allow_list() {
    let d = scratch("real");
    let t = d.join("tables");
    let work = d.join("work");
    let (rc, out) = run_tool(&["--lift", lift_dir().to_str().unwrap(), "--allow", allow().to_str().unwrap(), "--table", t.to_str().unwrap(), "--work", work.to_str().unwrap()]);
    assert_eq!(rc, 0, "noreturn_audit.py failed:\n{out}");
    let ret = read(&t.join("returning-implementations.tsv"));
    for n in ["SleepConditionVariableSRW", "RtlPcToFileHeader"] {   // decision 0238: GetFileType, SetFilePointerEx (and FlushFileBuffers, FindFirstFileExW) are answered by the runtime: no wrapper contradicts a declaration any more
        assert!(ret.contains(&format!("fle_imp_{n}\t")), "the detector must find the returning wrapper of {n} (decision 0190 lists it); table:\n{ret}");
    }
    let decl = read(&t.join("declared-noreturn.tsv"));
    assert!(!decl.contains("fle_imp_GetModuleHandleExW\t"), "decision 0243: GetModuleHandleExW returns, it is not declared noreturn:\n{decl}");
    let un = read(&t.join("unanalysed.tsv"));
    let unanalysed_with_names = un.lines().skip(1).filter(|l| !l.contains("(not a finding)")).count();
    assert_eq!(unanalysed_with_names, 0, "a file that spells a noreturn name could not be compiled:\n{un}");
    let _ = std::fs::remove_dir_all(&d);
}

const SYN_H: &str = r#"#include <stdint.h>
extern uint64_t R[16];
void fle_imp_Early(void) __attribute__((noreturn));
void fle_imp_Always(void) __attribute__((noreturn));
void fle_imp_Fwd(void) __attribute__((noreturn));
void fle_imp_Ptr(void) __attribute__((noreturn));
void fle_imp_Strong(void) __attribute__((noreturn));
void fle_imp_Macro(void) __attribute__((noreturn));
#ifdef FLE_GMHEX_RETURNS
void fle_imp_Cond(void);
#else
void fle_imp_Cond(void) __attribute__((noreturn));
#endif
void fle_imp_Plain(void);
void fle_imp_Stops(void);
"#;

#[test]
fn the_detector_finds_each_kind_of_return_path_and_not_a_forwarder() {
    let root = scratch("syn");
    let (rt, h) = (root.join("runtime"), root.join("harness"));
    std::fs::create_dir_all(&rt).unwrap();
    std::fs::create_dir_all(&h).unwrap();
    std::fs::create_dir_all(root.join("crates/fe-lift/src")).unwrap();
    std::fs::write(root.join("crates/fe-lift/src/g.rs"), "fn x() { let _ = \"#include \\\"syn.h\\\"\\n\"; }\n").unwrap();
    std::fs::write(rt.join("syn.h"), SYN_H).unwrap();
    // the runtime: the strong definitions stop (no ret); fle_imp_Stops stops but is declared as returning (reverse direction)
    std::fs::write(rt.join("syn.c"), "#include <stdlib.h>\n#include \"syn.h\"\nuint64_t R[16];\nvoid fle_imp_Plain(void) { R[0] = 1; }\nvoid fle_imp_Stops(void) { abort(); }\nvoid fle_imp_Always(void) { abort(); }\n").unwrap();
    std::fs::write(h.join("early.c"), "#include <stdint.h>\nextern uint64_t R[16];\nvoid __real_fle_imp_Early(void);\nvoid __wrap_fle_imp_Early(void) { if (R[1]) return; __real_fle_imp_Early(); }\n").unwrap();
    std::fs::write(h.join("always.c"), "#include <stdint.h>\nextern uint64_t R[16];\nvoid __wrap_fle_imp_Always(void) { R[0] = 1; }\n").unwrap();
    std::fs::write(h.join("fwd.c"), "void note(void);\nvoid __real_fle_imp_Fwd(void);\nvoid __wrap_fle_imp_Fwd(void) { note(); __real_fle_imp_Fwd(); }\n").unwrap();
    std::fs::write(h.join("ptr.c"), "void call_real(const char *n, void (*f)(void));\nvoid __real_fle_imp_Ptr(void);\nvoid __wrap_fle_imp_Ptr(void) { call_real(\"Ptr\", __real_fle_imp_Ptr); }\n").unwrap();
    std::fs::write(h.join("strong.c"), "#include \"syn.h\"\nvoid fle_imp_Strong(void) { R[0] = 7; }\n").unwrap();
    std::fs::write(h.join("macro.c"), "#define W(n) void __real_fle_imp_##n(void); void __wrap_fle_imp_##n(void) { }\nW(Macro)\n").unwrap();
    std::fs::write(h.join("cond.c"), "void __wrap_fle_imp_Cond(void) { }\n").unwrap();
    std::fs::write(h.join("plain.c"), "void __wrap_fle_imp_Plain(void) { }\n").unwrap();
    std::fs::write(h.join("nocompile.c"), "#include <no_such_header_for_the_audit.h>\nvoid __wrap_fle_imp_Early(void) { }\n").unwrap();
    let t = root.join("tables");
    let work = root.join("work");
    let (rc, out) = run_tool(&["--lift", root.to_str().unwrap(), "--table", t.to_str().unwrap(), "--no-gen-wraps", "--work", work.to_str().unwrap()]);
    assert_eq!(rc, 1, "findings expected:\n{out}");
    let ret = read(&t.join("returning-implementations.tsv"));
    let has = |k: &str, n: &str, f: &str| ret.lines().any(|l| l.starts_with(&format!("{k}\t{n}\t")) && l.contains(f));
    assert!(has("wrap", "fle_imp_Early", "early.c"), "early return on one path:\n{ret}");
    assert!(has("wrap", "fle_imp_Always", "always.c"), "unconditional return:\n{ret}");
    assert!(has("def", "fle_imp_Strong", "strong.c"), "a strong definition that returns (clang's own -Winvalid-noreturn in the TU that sees the declaration):\n{ret}");
    assert!(has("wrap", "fle_imp_Macro", "macro.c"), "a wrapper made by a macro:\n{ret}");
    assert!(has("wrap", "fle_imp_Cond", "cond.c") && ret.contains("[macro-conditional]"), "a macro-conditional declaration:\n{ret}");
    for (n, f) in [("fle_imp_Fwd", "fwd.c"), ("fle_imp_Ptr", "ptr.c"), ("fle_imp_Plain", "plain.c")] {
        assert!(!ret.lines().any(|l| l.contains(n) && l.contains(f)), "{f} must not be a finding:\n{ret}");
    }
    let fwd = read(&t.join("forwarding-wrappers.tsv"));
    assert!(fwd.contains("fle_imp_Ptr\t") && fwd.contains("ptr.c"), "the wrapper that passes __real_ to a helper is listed as a forwarder:\n{fwd}");
    assert!(!fwd.contains("fwd.c"), "a direct forwarder has no ret at all, so it is not listed:\n{fwd}");
    assert!(read(&t.join("unanalysed.tsv")).contains("nocompile.c"), "a file that does not compile is reported");
    let rev = read(&t.join("reverse-direction.tsv"));
    assert!(rev.contains("fle_imp_Stops\t") && !rev.contains("fle_imp_Plain\t"), "the reverse direction lists the function that stops but is declared as returning:\n{rev}");
    assert!(out.contains("NEW FINDING [returns] fle_imp_Strong") && out.contains("strong.c"), "{out}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn mutation_a_new_returning_wrapper_of_a_noreturn_name_is_caught() {
    let root = mutant_lift("mut-new-wrap");
    std::fs::write(root.join("harness/new_wrap.c"), "#include <stdint.h>\nextern uint64_t R[16];\nvoid __wrap_fle_imp_ILClone(void) { R[0] = 0; }\n").unwrap();
    let (rc, out) = fast(&root, &allow(), &[]);
    assert_eq!(rc, 1, "the new wrapper must be reported:\n{out}");
    assert!(out.contains("NEW FINDING [returns] fle_imp_ILClone") && out.contains("gen-visible=yes"), "{out}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn mutation_a_declaring_header_entering_the_generated_include_closure_invalidates_the_not_gen_visible_rows() {
    let root = mutant_lift("mut-closure");
    std::fs::write(root.join("crates/fe-lift/src/zz_mutant.rs"), "fn x() { let _ = \"#include \\\"fle_proc.h\\\"\\n\"; }\n").unwrap();   // decision 0238: was fle_vol.h (GetFileType, answered now; no allow row is declared there any more)
    let (rc, out) = fast(&root, &allow(), &[]);
    assert_eq!(rc, 1, "{out}");
    assert!(out.contains("not-gen-visible, but a header that declares it noreturn is in the include closure") && out.contains("fle_imp_RtlPcToFileHeader"), "{out}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn mutation_declaring_a_function_that_a_harness_answers_noreturn_in_a_generated_header_is_caught() {
    // the shape of decision 0190 again: fle_load.h is not in the closure, so declare LoadLibraryA (wrapped by vst_env.c and vst3a/vst_env3a.c, which return) noreturn in fle_modules.h, which is
    let root = mutant_lift("mut-new-decl");
    let p = root.join("runtime/fle_modules.h");
    let t = read(&p);
    let anchor = "void fle_imp_ILFree(void);";
    assert!(t.contains(anchor), "fle_modules.h changed shape; update this mutation");
    std::fs::write(&p, t.replace(anchor, &format!("{anchor}\nvoid fle_imp_LoadLibraryA(void) __attribute__((noreturn));"))).unwrap();
    let (rc, out) = fast(&root, &allow(), &[]);
    assert_eq!(rc, 1, "{out}");
    assert!(out.contains("NEW FINDING [returns] fle_imp_LoadLibraryA") && out.contains("gen-visible=yes"), "{out}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The `#include "fle_*.h"` string literals of the generator (crates/fe-lift/src/*.rs): what a generated unit includes.
fn generator_includes() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for e in std::fs::read_dir(lift_dir().join("crates/fe-lift/src")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "rs").unwrap_or(false) {
            let t = std::fs::read_to_string(&p).unwrap();
            let mut rest = t.as_str();
            while let Some(i) = rest.find("#include \\\"fle_") {
                let s = &rest[i + "#include \\\"".len()..];
                if let Some(j) = s.find("\\\"") {
                    let h = s[..j].to_string();
                    if h.ends_with(".h") && !v.contains(&h) {
                        v.push(h);
                    }
                }
                rest = &rest[i + 10..];
            }
        }
    }
    v.sort();
    v
}

#[test]
fn a_lifted_style_call_block_loses_the_code_after_the_call_exactly_for_the_names_the_tool_calls_gen_visible() {
    // The tool decides "gen-visible" from the include closure of the generator's headers. Here the compiler decides: a block written the way fe-lift writes a direct call to an API stub
    // (`R[4] -= 8; { void fle_imp_X(void); fle_imp_X(); } R[4] += 8;`) in a unit that includes what the generator includes. `unreachable` in the IR = the code after the call was dropped.
    let incs = generator_includes();
    for h in ["fle_runtime.h", "fle_sse.h", "fle_dispatch.h", "fle_modules.h"] {
        assert!(incs.iter().any(|x| x == h), "the generator no longer writes #include \"{h}\"? {incs:?}");
    }
    let names = [
        // (name, the code after the call is dropped in a generated unit)
        ("GetModuleHandleExW", false),
        ("ILClone", true),
        ("deferred_qfc_InitSharedPython", true),
        ("GetFileType", false),
        ("SetFilePointerEx", false),
        ("SleepConditionVariableSRW", false),
        ("RtlPcToFileHeader", false),
        ("ExitProcess", false),
    ];
    let d = scratch("blocks");
    let mut c = String::new();
    for h in &incs {
        c.push_str(&format!("#include \"{h}\"\n"));
    }
    c.push_str("\nuint64_t R[16];\n");
    for (n, _) in &names {
        c.push_str(&format!("void blk_{n}(void) {{ R[4] -= 8; {{ void fle_imp_{n}(void); fle_imp_{n}(); }} R[4] += 8; R[0] = R[3]; }}\n"));
    }
    std::fs::write(d.join("blocks.c"), c).unwrap();
    let out = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-S", "-emit-llvm", "-w", "-I"])
        .arg(lift_dir().join("runtime"))
        .arg(d.join("blocks.c"))
        .arg("-o")
        .arg(d.join("blocks.ll"))
        .output()
        .expect("clang not found");
    assert!(out.status.success(), "clang failed: {}", String::from_utf8_lossy(&out.stderr));
    let ll = read(&d.join("blocks.ll"));
    for (n, dropped) in names {
        let a = ll.find(&format!("define dso_local void @blk_{n}(")).unwrap_or_else(|| panic!("no IR for {n}"));
        let body = &ll[a..a + ll[a..].find("\n}\n").unwrap()];
        assert_eq!(body.contains("unreachable"), dropped, "fle_imp_{n}: expected the code after the call to be {} in a generated unit:\n{body}", if dropped { "dropped" } else { "kept" });
    }
    let _ = std::fs::remove_dir_all(&d);
}
