//! Decision 0132 (`fe-lift --seh-handler`, the except landings of the guest's exception handler) on the real FLEngine_x64.dll ($FLENGINE_PE, else
//! <repo root>/binaries/win64/FLEngine_x64.dll; with FLENGINE_PE set and the file missing the tests fail, with the default missing they are skipped).
//!
//! 1. Without any SEH function the generated C is byte for byte the one of `generate_with_overrides` (the default of every build).
//! 2. With the landings as seeds and their owners as SEH functions: every owner has the entry wrapper and a body `fb_<va>`, every landing is a function, the
//!    wrappers are exactly the text the runtime expects (fle_seh.h), the numbers are those of the experiment of docs/investigations/seh-model.md section 4.1
//!    (functions 59,388 -> 62,326 with the thunk seeds of decision 0126; here without them: +2,938), `bad_nodes` stays 0 and `unsupported_total` does not grow.
//! 3. The function bodies that change because a landing became a function entry (a `goto` into the landing would become a tail call) are counted: measured 0.
//!    Every owner's body `fb_P` is the old `f_P` body.
//! 4. A landing that is not a seed, and an owner that is not a function entry, are errors, not guesses.
//!
//! The C is about 200 MB per run: the writer keeps a hash per function body and the text of a few functions, not the file.

use fe_lift::{generate_with_overrides, generate_with_seh, SehOptions};
use std::collections::{hash_map::DefaultHasher, BTreeMap};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

const HANDLER_RVA: u32 = 0x112c0;

fn load() -> Option<Vec<u8>> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(e) if explicit.is_some() => panic!("FLENGINE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

/// Hashes the whole output, and every function body (`void f_X(void)` ... `}`, and `void fb_X(void)` bodies under the name `fb_X`).
struct Funcs {
    partial: Vec<u8>,
    cur: Option<(String, Vec<u8>)>,
    keep: Vec<String>,
    whole: DefaultHasher,
    bytes: usize,
    include_lines: usize,
    fb_protos: usize,
    hashes: BTreeMap<String, u64>,
    texts: BTreeMap<String, String>,
}

impl Funcs {
    fn new(keep: &[&str]) -> Funcs {
        Funcs {
            partial: Vec::new(),
            cur: None,
            keep: keep.iter().map(|s| s.to_string()).collect(),
            whole: DefaultHasher::new(),
            bytes: 0,
            include_lines: 0,
            fb_protos: 0,
            hashes: BTreeMap::new(),
            texts: BTreeMap::new(),
        }
    }

    fn line(&mut self, l: &[u8]) {
        if l == b"#include \"fle_seh.h\"" {
            self.include_lines += 1;
        }
        if l.starts_with(b"__attribute__((no_instrument_function)) void fb_") && l.ends_with(b"(void);") {
            self.fb_protos += 1;
        }
        if let Some((name, body)) = &mut self.cur {
            body.extend_from_slice(l);
            body.push(b'\n');
            if l == b"}" {
                let mut h = DefaultHasher::new();
                body.hash(&mut h);
                self.hashes.insert(name.clone(), h.finish());
                if self.keep.contains(name) {
                    self.texts.insert(name.clone(), String::from_utf8_lossy(body).to_string());
                }
                self.cur = None;
            }
        } else if l.starts_with(b"void f_") && l.ends_with(b"(void)") {
            let name = String::from_utf8_lossy(&l[5..l.len() - 6]).to_string();
            self.cur = Some((name, Vec::new()));
        } else if l.starts_with(b"__attribute__((no_instrument_function)) void fb_") && l.ends_with(b"(void)") {
            let at = l.windows(5).position(|w| w == b"void ").unwrap() + 5;
            let name = String::from_utf8_lossy(&l[at..l.len() - 6]).to_string();
            self.cur = Some((name, Vec::new()));
        }
    }
}

impl std::io::Write for Funcs {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        buf.hash(&mut self.whole);
        self.bytes += buf.len();
        let mut rest = buf;
        while let Some(i) = rest.iter().position(|&b| b == b'\n') {
            self.partial.extend_from_slice(&rest[..i]);
            let l = std::mem::take(&mut self.partial);
            self.line(&l);
            rest = &rest[i + 1..];
        }
        self.partial.extend_from_slice(rest);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Run {
    st: fe_lift::Stats,
    f: Funcs,
}

fn seeds(data: &[u8]) -> Vec<u64> {
    let pe = fe_pe::Pe::parse(data).unwrap();
    fe_decode::pdata_and_adjustor_seeds(&pe).unwrap()
}

fn specs() -> Vec<fe_decode::Spec> {
    fe_decode::jumptab::FLENGINE.iter().chain(fe_decode::jumptab::ILTOOLS_6000000).copied().collect()
}

fn lift(data: &[u8], extra: &[u64], seh: Option<&SehOptions>, keep: &[&str]) -> Result<Run, String> {
    let mut f = Funcs::new(keep);
    let opts = fe_decode::Options::default();
    let st = match seh {
        None => generate_with_overrides(data, extra, &specs(), &[], &[], &opts, &mut f)?,
        Some(s) => generate_with_seh(data, extra, &specs(), &[], &[], &opts, s, &mut f)?,
    };
    Ok(Run { st, f })
}

fn landings(data: &[u8]) -> fe_decode::seh::SehLandings {
    let pe = fe_pe::Pe::parse(data).unwrap();
    fe_decode::seh::delphi_landings(&pe, &[HANDLER_RVA]).unwrap()
}

#[test]
fn without_an_seh_function_the_output_is_byte_for_byte_the_old_one() {
    let Some(data) = load() else { return };
    let ex = seeds(&data);
    let a = lift(&data, &ex, None, &[]).unwrap();
    let b = lift(&data, &ex, Some(&SehOptions::default()), &[]).unwrap();
    assert_eq!(a.f.bytes, b.f.bytes);
    assert_eq!(a.f.whole.finish(), b.f.whole.finish());
    assert_eq!(b.f.include_lines, 0, "no fle_seh.h include without an SEH function");
    assert_eq!(b.f.fb_protos, 0);
    assert!(!b.f.hashes.keys().any(|k| k.starts_with("fb_")));
    assert_eq!((b.st.seh_wrapped, b.st.seh_landings), (0, 0));
}

#[test]
fn owners_get_the_wrapper_and_landings_are_functions() {
    let Some(data) = load() else { return };
    let ex = seeds(&data);
    let l = landings(&data);
    let mut ex2 = ex.clone();
    ex2.extend(l.landing_vas());
    let base = lift(&data, &ex, None, &["f_63ae4e"]).unwrap();
    let sehopts = SehOptions { funcs: l.by_func.clone() };
    let r = lift(&data, &ex2, Some(&sehopts), &["f_5df610", "f_63ade0", "f_63ae4e", "fb_63ade0"]).unwrap();
    assert_eq!(r.st.seh_wrapped, 2_816);
    assert_eq!(r.st.seh_landings, 2_938);
    assert_eq!(r.f.include_lines, 1);
    assert_eq!(r.f.fb_protos, 2_816, "one prototype of a body per wrapped function");
    assert_eq!(r.f.hashes.keys().filter(|k| k.starts_with("fb_")).count(), 2_816);
    assert_eq!(r.st.outlined_heads, 0, "a module with SEH wrappers is not outlined");
    assert_eq!(r.st.functions, base.st.functions - base.st.outlined_heads + 2_938, "each landing is a new function entry (the base has outlined heads, the module with wrappers has none)");
    assert_eq!(r.st.bad_nodes, 0);
    assert_eq!(base.st.bad_nodes, 0);
    assert_eq!(r.st.unsupported_total, base.st.unsupported_total, "no instruction became unsupported");
    // the exact wrapper (fle_seh.h: enter, setjmp in this C frame, the body, or the landing, leave)
    let w = &r.f.texts["f_63ade0"];
    assert_eq!(
        w,
        "{\n    fle_seh_reg sr;\n    fle_seh_enter(&sr, UINT64_C(0x63ade0));\n    if (FLE_SEH_SETJMP(&sr) == 0) { fb_63ade0(); } else { fle_seh_land(&sr); }\n    fle_seh_leave(&sr);\n}\n"
    );
    assert!(r.f.texts["fb_63ade0"].starts_with("{\nL_63ade0: { /* 0x63ade0 Push */"), "the body starts at the function entry");
    // the landing of wall 16 is a function that starts at the landing (sim_scopes.py: lea rcx,[0x15f3320]; call f_41a770; call f_411fc0; jmp 0x63ae62)
    let t = &r.f.texts["f_63ae4e"];
    assert!(t.starts_with("{\nL_63ae4e: { /* 0x63ae4e Lea */ R[1] = UINT64_C(0x15f3320);}"), "{t}");
    assert!(t.contains("f_41a770();") && t.contains("f_411fc0();"));
    assert!(r.f.hashes.contains_key("f_5df673"));
    assert!(!base.f.hashes.contains_key("f_63ae4e"), "the base output has no function at the landing");
    // a function that is not wrapped is unchanged in shape: no `fle_seh` in its text is checked by the hash comparison below
    let owners: std::collections::BTreeSet<String> = l.by_func.keys().map(|f| format!("f_{f:x}")).collect();
    let owners_changed = base.f.hashes.iter().filter(|(k, v)| owners.contains(*k) && r.f.hashes.get(*k).map_or(false, |x| x != *v)).count();
    let removed = base.f.hashes.keys().filter(|k| !r.f.hashes.contains_key(*k)).count();
    assert_eq!(removed, base.st.outlined_heads, "the functions of the base that the module with wrappers does not have are its outlined heads");
    // the wrapped owners print a different `f_` text (wrapper instead of body): 2,816 changes; the other bodies that differ are those of the outlining (the base is outlined, the module with wrappers is not)
    assert_eq!(owners_changed, 2_816, "every owner prints differently");
}

#[test]
fn the_other_changed_bodies_are_the_ones_that_jumped_into_a_landing() {
    let Some(data) = load() else { return };
    let ex = seeds(&data);
    let l = landings(&data);
    let mut ex2 = ex.clone();
    ex2.extend(l.landing_vas());
    let base = lift(&data, &ex, None, &[]).unwrap();
    let sehopts = SehOptions { funcs: l.by_func.clone() };
    let r = lift(&data, &ex2, Some(&sehopts), &[]).unwrap();
    // an owner's `f_P` is the wrapper; its body `fb_P` must be the old `f_P` body unless it jumped into a landing. Count the owners whose body did not change.
    let mut same = 0usize;
    let mut diff = 0usize;
    for f in l.by_func.keys() {
        let k = format!("f_{f:x}");
        let b = format!("fb_{f:x}");
        if base.f.hashes[&k] == r.f.hashes[&b] {
            same += 1;
        } else {
            diff += 1;
        }
    }
    // everything that is not an owner: how many bodies changed
    let owners: std::collections::BTreeSet<String> = l.by_func.keys().map(|f| format!("f_{f:x}")).collect();
    let other = base.f.hashes.iter().filter(|(k, v)| !owners.contains(*k) && r.f.hashes.get(*k).map_or(false, |x| x != *v)).count();
    eprintln!("seh: owners with an unchanged body {same}, with a changed body {diff}, other changed bodies {other}");
    // measured: every owner's body fb_P is the old f_P body (a landing is reached only by an exception, so no old `goto` pointed into it); the other changed bodies are the 51 of the outlining (the base is outlined with 11 heads, the module with wrappers is not; before decision 0243 there was none)
    assert_eq!((same, diff, other), (2_816, 0, 51));
}

#[test]
fn a_landing_that_is_not_a_seed_and_an_owner_that_is_not_an_entry_are_errors() {
    let Some(data) = load() else { return };
    let ex = seeds(&data);
    let l = landings(&data);
    // landings not added to the seeds
    let e = lift(&data, &ex, Some(&SehOptions { funcs: l.by_func.clone() }), &[]).err().expect("must fail");
    assert!(e.contains("must be a seed"), "{e}");
    // an owner that is not a function entry of the closure
    let mut funcs = BTreeMap::new();
    funcs.insert(0x5df611u64, vec![0x5df673u64]);
    let e = lift(&data, &ex, Some(&SehOptions { funcs }), &[]).err().expect("must fail");
    assert!(e.contains("not a function entry"), "{e}");
}
