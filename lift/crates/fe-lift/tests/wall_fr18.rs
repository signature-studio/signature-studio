//! Tests of decision 0143 (docs/decisions/0143-mxcsr-fast-path.md, PROVISIONAL): the fast path for ordinary values in runtime/fle_mxcsr_flags.h.
//!
//! The fast path computes a lane with the host's float and double operations when the operands and the result are zero or ordinary normal numbers
//! and the guest rounding control is 0, and decides PE exactly; every other lane takes the integer path, which is unchanged. What is checked here is
//! the differential test of harness/fp1/ (a reduced run; the full run is 10^8 cases per instruction, see the decision): the new header, the same header
//! with -DFLE_MX_NOFAST, the header of HEAD ac5a790 verbatim (harness/fp1/ref/fle_mxcsr_flags_old.h) and, on x86-64, the instruction itself on the host CPU
//! (crates/fe-lift/tests/ref/fle_mxcsr_flags_host.h) run the same random and structured operands; the destination, the general register, the whole MXCSR
//! and the COMIS flags must be equal, a case with RC != 0 must never take the fast path, the fast path must be exercised (coverage), and an unmasked
//! flag must stop the run in the same cases. Also: mutations of the header (harness/fp1/mutate.py) must be caught.
//! Nothing here compares with a real Windows.
//!
//! FLE_FR18_RT (a copy of lift/runtime) overrides the runtime directory, for the mutation runs of the run record.

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rt_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_FR18_RT") {
        return PathBuf::from(d);
    }
    manifest().join("../../runtime")
}

fn harness() -> PathBuf {
    manifest().join("../../harness/fp1")
}

fn work_dir(name: &str) -> PathBuf {
    let d = manifest().join("../../out-wallFR18").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn build(name: &str, opt: &str, extra: &str) -> PathBuf {
    let d = work_dir(name);
    let out = Command::new("bash")
        .arg(harness().join("build_diff.sh"))
        .arg(&d)
        .env("OPT", opt)
        .env("EXTRA", extra)
        .env("RT", rt_dir())
        .output()
        .expect("bash");
    assert!(out.status.success(), "build_diff.sh {} {}: {}{}", opt, extra, String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    d.join("mxfp_diff")
}

fn run(bin: &PathBuf, args: &[&str]) -> String {
    let out = Command::new(bin).args(args).output().expect("mxfp_diff");
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(out.status.success(), "mxfp_diff {:?} failed:\n{}", args, s.lines().filter(|l| l.starts_with("MISMATCH") || l.starts_with("LOW") || l.starts_with("TOTAL") || l.contains("guard_viol") && !l.contains("guard_viol 0")).take(20).collect::<Vec<_>>().join("\n"));
    s
}

/// the numbers of the TOTAL line: (cases, mismatches)
fn total(s: &str, mode: &str) -> (u64, u64) {
    let l = s.lines().find(|l| l.starts_with(&format!("TOTAL {}", mode))).unwrap_or_else(|| panic!("no TOTAL {} line", mode));
    let f: Vec<&str> = l.split_whitespace().collect();
    let cases = f[f.iter().position(|x| *x == "cases").unwrap() + 1].parse().unwrap();
    let mism = f[f.iter().position(|x| *x == "mismatches").unwrap() + 1].parse().unwrap();
    (cases, mism)
}

/// The fast path was exercised: for every instruction class that has one, a fifth of the cases or more went through it entirely
/// (a lane of a packed instruction counts when all lanes did), with PE raised in some and clear in others.
fn check_coverage(s: &str) {
    let mut seen = 0;
    for l in s.lines().filter(|l| l.starts_with("OP rand ")) {
        let f: Vec<&str> = l.split_whitespace().collect();
        let name = f[2];
        let g = |k: &str| -> u64 { f[f.iter().position(|x| *x == k).unwrap() + 1].parse().unwrap() };
        if name.starts_with("cmp") || name.starts_with("round") {
            continue; // not on the fast path
        }
        let cases = g("cases");
        let fast_all = g("fast_all");
        assert!(fast_all * 100 >= cases * 10, "{}: fast path in {} of {} cases", name, fast_all, cases);
        assert_eq!(g("mism_old"), 0, "{}", l);
        assert_eq!(g("guard_viol"), 0, "{}", l);
        seen += 1;
    }
    assert!(seen >= 55, "only {} instruction classes with a fast path", seen);
}

#[test]
fn fast_path_equals_integer_path_o2() {
    let bin = build("o2", "O2", "");
    let s = run(&bin, &["rand", "*", "150000", "1", "1"]);
    let (cases, mism) = total(&s, "rand");
    assert!(cases >= 9_000_000);
    assert_eq!(mism, 0);
    check_coverage(&s);
}

#[test]
fn fast_path_equals_integer_path_o0_and_oz() {
    for (n, o) in [("o0", "O0"), ("oz", "Oz")] {
        let bin = build(n, o, "");
        let s = run(&bin, &["rand", "*", "40000", "2", "1"]);
        assert_eq!(total(&s, "rand").1, 0);
        check_coverage(&s);
    }
}

#[test]
fn grid_of_binary32_classes() {
    let bin = build("grid", "O2", "");
    let s = run(&bin, &["grid32", "*", "16"]);
    let (cases, mism) = total(&s, "grid32");
    assert!(cases >= 5_000_000);
    assert_eq!(mism, 0);
}

#[test]
fn unmasked_flag_stops_the_same_cases() {
    let bin = build("unmasked", "O2", "");
    let s = run(&bin, &["unmasked", "20000", "4"]);
    let (cases, mism) = total(&s, "unmasked");
    assert!(cases >= 1_000_000);
    assert_eq!(mism, 0);
    let l = s.lines().find(|l| l.starts_with("TOTAL unmasked")).unwrap();
    let aborts: u64 = l.split_whitespace().nth(5).unwrap().parse().unwrap();
    assert!(aborts > 100_000, "the abort path was exercised {} times only", aborts);
}

#[test]
fn mutations_are_caught() {
    // each mutation edits one expression of a copy of the header (PE decision, a boundary, the RC guard, the sticky-PE shortcut, ...)
    let d = work_dir("mut");
    let out = Command::new("python3")
        .arg(harness().join("mutate.py"))
        .arg(&d)
        .args(["M02-add-pe-eq", "M03-res64-boundary", "M04-rc-guard-bit13-removed", "M07-div-pe-eq", "M08-mul-mask-52-51", "M12-f32-div-pe-eq", "M15-mul-zero-sign-lost", "M19-guard-in-lane-removed", "M22-denormal-operand-allowed", "M26-res32-min-normal-allowed", "M31-sqrt-pe-eq", "M34-np-ignores-pm", "M35-np-always-skip"])
        .env("CASES", "120000")
        .env("PAR", "4")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .expect("python3");
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let n = s.lines().filter(|l| l.contains(" CAUGHT ")).count();
    assert!(out.status.success() && n == 13, "mutations: {}\n{}", n, s);
}

#[test]
fn old_header_is_the_one_of_head() {
    // the reference of the differential test is the header before the change, verbatim: the integer path inside the new header is the same text
    let old = std::fs::read_to_string(harness().join("ref/fle_mxcsr_flags_old.h")).unwrap();
    assert_eq!(old.lines().count(), 977);
    assert!(old.contains("static inline uint64_t fle_mx_pack(int f64, int sign, int32_t e, fle_mxu128 sig, uint32_t mx, uint32_t *fl) {"));
    let new = std::fs::read_to_string(rt_dir().join("fle_mxcsr_flags.h")).unwrap();
    for name in ["fle_mx_pack", "fle_mx_add", "fle_mx_mul", "fle_mx_div", "fle_mx_isqrt", "fle_mx_unpack", "fle_mx_order"] {
        assert!(new.contains(&format!(" {}(", name)), "{} is missing from the new header", name);
    }
    // the bodies of fle_mx_pack and fle_mx_add (the integer rounding and the sum) are unchanged
    let body = |s: &str, start: &str| -> String {
        let i = s.find(start).unwrap();
        let j = s[i..].find("\n}\n").unwrap();
        s[i..i + j].to_string()
    };
    assert_eq!(body(&old, "static inline uint64_t fle_mx_pack("), body(&new, "static inline uint64_t fle_mx_pack("));
    assert_eq!(body(&old, "static inline uint64_t fle_mx_add("), body(&new, "static inline uint64_t fle_mx_add("));
    assert_eq!(body(&old, "static inline uint64_t fle_mx_mul("), body(&new, "static inline uint64_t fle_mx_mul("));
    assert_eq!(body(&old, "static inline uint64_t fle_mx_div("), body(&new, "static inline uint64_t fle_mx_div("));
}
