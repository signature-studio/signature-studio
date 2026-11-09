//! Tests of decision 0206 (docs/0206-sample-decoders.md, PROVISIONAL, dec1): the additions that let the sample-decoder DLLs of the installer (oggio_x64.dll, wavpackdll_x64.dll, DelZip64.dll) run lifted.
//! All checks are on files of the tree (no product file is needed):
//!  1. every row of harness/seeds/dec1-imports.txt names a runtime function that exists (a non-static definition `void fle_imp_X(void)` in runtime/*.c), is not a row of the lift's own table
//!     (imports_table.rs) or of the earlier layers (ship-imports.txt, exp1-imports.txt) or of the ACM exploration rows, and has the shape `<dll> <API> fle_imp_<API>`;
//!  2. the three seed files: sorted, unique, one hex VA per line, every VA inside the image of its module at the base the decision used (0xc000000 / 0xd000000 / 0xe000000), the images do not overlap each
//!     other or the 0xb000000 + 0x500000 of Elastique (the last base of decision 0190) and stay below the virtual module handles;
//!  3. runtime/fle_realmod.c: the number of real modules is 8 (checked on the preprocessor output);
//!  4. runtime/fle_vfs.c: a read at the end of a file succeeds with 0 bytes read (decision 0206; always on since decision 0243);
//!  5. runtime/fle_vfs.c: CreateFileW accepts a SECURITY_ATTRIBUTES with a NULL descriptor and still stops on any other non-NULL value (text pins).

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn lift_dir() -> PathBuf {
    manifest().join("../..")
}
fn read(rel: &str) -> String {
    std::fs::read_to_string(lift_dir().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}
fn rows(text: &str) -> Vec<(String, String, String)> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(f.len(), 3, "bad row: {l}");
            (f[0].to_string(), f[1].to_string(), f[2].to_string())
        })
        .collect()
}
fn runtime_text() -> String {
    let mut all = String::new();
    for e in std::fs::read_dir(lift_dir().join("runtime")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "c").unwrap_or(false) {
            all.push_str(&std::fs::read_to_string(&p).unwrap());
            all.push('\n');
        }
    }
    all
}

#[test]
fn rows_name_runtime_functions_and_are_new() {
    let dec1 = rows(&read("harness/seeds/dec1-imports.txt"));
    assert_eq!(dec1.len(), 18, "eighteen rows (decision 0206)");
    let rt = runtime_text();
    let table = read("crates/fe-lift/src/imports_table.rs");
    let mut ship: HashSet<(String, String)> = rows(&read("harness/seeds/ship-imports.txt")).into_iter().map(|(d, a, _)| (d.to_lowercase(), a.to_lowercase())).collect();
    ship.extend(rows(&read("harness/seeds/exp1-imports.txt")).into_iter().map(|(d, a, _)| (d.to_lowercase(), a.to_lowercase())));
    ship.extend(rows(&read("harness/seeds/dec1-acm-imports.txt")).into_iter().map(|(d, a, _)| (d.to_lowercase(), a.to_lowercase())));
    let mut seen = HashSet::new();
    for (dll, api, func) in &dec1 {
        assert_eq!(func, &format!("fle_imp_{api}"), "row {api}: function name");
        assert!(dll.ends_with(".dll"));
        assert!(rt.contains(&format!("\nvoid {func}(void)")), "{func} is not defined in runtime/*.c");
        assert!(!table.contains(&format!("\"{api}\", \"{func}\"")), "{api} already has a row in imports_table.rs");
        assert!(!ship.contains(&(dll.to_lowercase(), api.to_lowercase())), "{api} already has a row in ship-imports.txt");
        assert!(seen.insert(api.clone()), "{api} twice");
    }
}

fn seeds(rel: &str, base: u64, size: u64) -> usize {
    let text = read(rel);
    let mut prev = 0u64;
    let mut n = 0;
    for l in text.lines().filter(|l| !l.trim().is_empty()) {
        let v = u64::from_str_radix(l.trim().trim_start_matches("0x"), 16).unwrap_or_else(|_| panic!("{rel}: bad line {l}"));
        assert!(v >= base && v < base + size, "{rel}: {v:#x} outside {base:#x}+{size:#x}");
        assert!(v > prev, "{rel}: not sorted/unique at {v:#x}");
        prev = v;
        n += 1;
    }
    n
}

#[test]
fn seed_files_are_inside_their_images_and_images_do_not_overlap() {
    let imgs = [
        ("harness/seeds/dec1-seeds-oggio-c000000.txt", 0xc00_0000u64, 0x1cf000u64, 280usize),
        ("harness/seeds/dec1-seeds-wavpack-d000000.txt", 0xd00_0000, 0x48000, 73),
        ("harness/seeds/dec1-seeds-delzip-e000000.txt", 0xe00_0000, 0x5c000, 103),
    ];
    for (rel, base, size, count) in imgs {
        assert_eq!(seeds(rel, base, size), count, "{rel}: seed count");
    }
    let known_end = 0xb00_0000u64 + 0x50_0000; // Elastique_x64.dll (decision 0190), the last earlier base
    let mut spans: Vec<(u64, u64)> = imgs.iter().map(|(_, b, s, _)| (*b, *b + *s)).collect();
    spans.sort();
    assert!(spans[0].0 >= known_end);
    for w in spans.windows(2) {
        assert!(w[0].1 <= w[1].0, "images overlap");
    }
    assert!(spans[2].1 < 0x7FF8_0000_0000u64, "below the virtual module handles");
}

fn pp(defs: &[&str]) -> String {
    let out = Command::new("clang").args(["-E", "-P", "-std=gnu11"]).args(defs).arg("-I").arg(lift_dir().join("runtime")).arg(lift_dir().join("runtime/fle_realmod.c")).output().expect("clang");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn rm_max_is_8() {
    let d = pp(&[]);
    assert!(d.contains("static rm_t mods[8u];"), "RM_MAX must stay 8 (decision 0243: the -DFLE_RM_MAX override is gone)");
}

fn vfs_pp(defs: &[&str]) -> String {
    let out = Command::new("clang").args(["-E", "-P", "-std=gnu11"]).args(defs).arg("-I").arg(lift_dir().join("runtime")).arg(lift_dir().join("runtime/fle_vfs.c")).output().expect("clang");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn after_readfile(s: &str) -> &str {
    let i = s.find("void fle_imp_ReadFile(void)").expect("ReadFile");
    let j = s[i..].find("e->pos >= e->node->size").expect("EOF test") + i;
    &s[j..j + 400]
}

#[test]
fn readfile_eof_success_is_always_on() {
    let o = vfs_pp(&[]);
    let w: String = after_readfile(&o).split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(w.contains("if (n != 0) {") && w.contains("R[0] = 1; return; } fail(38, 0);"), "success with 0 bytes read for n != 0 (Learn 'Testing for the End of a File'), 38 kept for n == 0: {w}");
}

#[test]
fn createfilew_accepts_only_a_null_descriptor() {
    let t = read("runtime/fle_vfs.c");
    assert!(t.contains("R[9] != 0 && !(R[9] >= 0x10000 && (R[9] & 7) == 0 && fle_ld(R[9], 4) == 24 && fle_ld(R[9] + 8, 8) == 0)"), "SECURITY_ATTRIBUTES rule (a bogus pointer must stop, not be dereferenced)");
    assert!(t.contains("lpSecurityAttributes is not NULL and not a SECURITY_ATTRIBUTES with a NULL descriptor"));
    assert!(t.contains("if (R[2] != 0) fle_stop(api, \"lpSecurityAttributes is not NULL (not implemented)\");"), "CreateDirectoryW keeps its stop");
}

#[test]
fn realmod_free_resident_is_always_on() {
    let o = pp(&[]);
    let w: String = o.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(w.contains("if (m->refs <= 1) return 1;"), "the last FreeLibrary keeps the module resident and returns 1");
    assert!(!w.contains("the reference count would reach zero"), "decision 0243: the stop of decision 0082 D11 is gone");
}

#[test]
fn loadlibrary_path_names_are_answered_by_the_volume_search_and_not_by_an_exploration_switch() {
    // Decision 0233 (load1; old: loadlibrary_path_names_are_missing_only_under_the_exploration_switch pinned the FLE_LOADLIB_MISSING branch of decision 0206 in normalize() and the default stop "a module name with a path is not
    // supported"): a name with a path is answered by the search of the virtual volume (fle_load.c load_with_path: NULL and 126 when the volume holds no file there, a named stop when it holds one); the switch and the other getenv
    // calls of the loader are gone (tests/load.rs checks behaviour: load1_*).
    let t = read("runtime/fle_load.c");
    let w: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(!w.contains("FLE_LOADLIB_MISSING\")"), "the exploration switch is gone");
    assert!(!w.contains("getenv("), "no getenv in the loader");
    assert!(!w.contains("a module name with a path is not supported"), "the default stop of decision 0182 is gone");
    assert!(w.contains("static void load_with_path("), "the path branch");
    assert!(w.contains("if (name_has_path(u, n)) { /* decision 0233 */ load_with_path(api, u, n); return; }"), "load_by_name hands a name with a path to it");
}
