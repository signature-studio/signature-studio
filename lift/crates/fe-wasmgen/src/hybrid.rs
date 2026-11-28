//! The hybrid of the engine build (decision 0230): the stages hybn0 and hybrid of app/scripts/build-engine-bundle.sh, in this program.
//! hybrid-n0: for each module of the list, fe-wasmgen with dummy addresses (the arguments of the file --args); the C functions that the
//! direct code calls (the imports of module c) go to exports.rsp, sorted and without duplicates; a listed function that is not emitted is an error.
//! hybrid: fe-wasmgen with the addresses of the link map of the C part; wasm-opt -Oz of each direct module; wasm-merge of the direct modules,
//! then into the C part; the output has no import of module c and no import of a lifted function (f_<va>) from env.
//! --separate NAMES (Issue #1342): the named modules are not merged; each is written to --separate-dir as <DLL stem>.wasm, a module of its own
//! that the page instantiates, without a names section (its table is wasm-func-names-<module>.tsv in the work directory). Its functions are the C part's f_<va> imports (the page supplies them), its imports are the C part's exports.
//! A module whose list is missing or empty is skipped (as the shell did). The lists are lowered-<module>.txt in --lists, one VA per line.
//! The merges keep the names (-g): the table of the function names (names_table) is written to the work directory, and the output has no names section.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The table of the functions of the engine (names_table), written to the work directory; fe-build copies it to lift/out-app/gen.
const FUNC_NAMES: &str = "wasm-func-names.tsv";

struct Module {
    name: String,
    pe: String,
    args: Vec<String>,
}

fn opts(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut m = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        if !args[i].starts_with("--") || i + 1 >= args.len() {
            return Err(format!("expected --NAME VALUE, got {}", args[i]));
        }
        m.insert(args[i][2..].to_string(), args[i + 1].clone());
        i += 2;
    }
    Ok(m)
}

fn req<'a>(m: &'a BTreeMap<String, String>, k: &str) -> Result<&'a str, String> {
    m.get(k).map(String::as_str).ok_or_else(|| format!("missing --{k}"))
}

/// The modules: one line each, `NAME<TAB>DLL FILE<TAB>ARGS` (the arguments of the one-module fe-wasmgen; paths relative to the repository).
fn read_modules(path: &str) -> Result<Vec<Module>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    modules_of(&text).map_err(|e| format!("{path}: {e}"))
}

fn modules_of(text: &str) -> Result<Vec<Module>, String> {
    let mut out = Vec::new();
    for l in text.lines() {
        if l.trim().is_empty() || l.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 2 {
            return Err(format!("a line without the DLL file: {l}"));
        }
        let args = f[2..].join(" ").split_whitespace().map(str::to_string).collect();
        out.push(Module { name: f[0].to_string(), pe: f[1].to_string(), args });
    }
    Ok(out)
}

fn parse_hex(s: &str) -> Result<u64, String> {
    u64::from_str_radix(s.trim().trim_start_matches("0x"), 16).map_err(|e| format!("bad VA {s}: {e}"))
}

/// The list of a module (its VAs), or None when the list is missing or empty (the shell skipped such a module).
fn list_of(dir: &Path, name: &str) -> Result<Option<Vec<u64>>, String> {
    let p = dir.join(format!("lowered-{name}.txt"));
    match std::fs::metadata(&p) {
        Ok(m) if m.len() > 0 => {}
        _ => return Ok(None),
    }
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut v = Vec::new();
    for l in text.lines().filter(|l| !l.trim().is_empty()) {
        v.push(parse_hex(l.split_whitespace().next().unwrap())?);
    }
    Ok(Some(v))
}

/// One module through the one-module fe-wasmgen (the same arguments, in the same meaning, as the shell gave it).
fn compile_module(m: &Module, dll_dir: &Path, extra: &[String], list: &Path, out: &Path, report: &Path) -> Result<(), String> {
    let mut argv: Vec<String> = vec!["fe-wasmgen".into(), dll_dir.join(&m.pe).display().to_string(), out.display().to_string()];
    argv.extend(m.args.iter().cloned());
    argv.extend(extra.iter().cloned());
    argv.extend(["--hybrid-only".into(), list.display().to_string(), "--report".into(), report.display().to_string()]);
    if crate::compile_cli(&argv) != ExitCode::SUCCESS {
        return Err(format!("fe-wasmgen failed for module {}", m.name));
    }
    Ok(())
}

/// The listed functions that the report shows as emitted (status ok); the same check as chk_lowered.py.
fn check_listed(name: &str, list: &[u64], report: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(report).map_err(|e| format!("{}: {e}", report.display()))?;
    let mut ok = BTreeSet::new();
    let mut why = BTreeMap::new();
    for l in text.lines() {
        let f: Vec<&str> = l.split('\t').collect();
        if f[0] == "va" || f.len() < 8 {
            continue;
        }
        let va = parse_hex(f[0])?;
        if f[7] == "ok" {
            ok.insert(va);
        } else {
            why.insert(va, f[7].to_string());
        }
    }
    let want: BTreeSet<u64> = list.iter().copied().collect();
    let missing: Vec<u64> = want.difference(&ok).copied().collect();
    println!("{name}: listed {}, emitted ok {}, listed but not emitted {}, emitted but not listed {}", want.len(), want.intersection(&ok).count(), missing.len(), ok.difference(&want).count());
    if missing.is_empty() {
        return Ok(());
    }
    for v in missing.iter().take(8) {
        println!("  not emitted: 0x{v:x}: {}", why.get(v).map(String::as_str).unwrap_or("not in the report of fe-wasmgen"));
    }
    Err(format!("fe-wasmgen did not emit the listed functions of {name} ({} of them)", missing.len()))
}

fn leb(b: &[u8], i: &mut usize) -> Result<u64, String> {
    let mut r = 0u64;
    let mut s: u32 = 0;
    loop {
        let x = *b.get(*i).ok_or("truncated wasm")?;
        *i += 1;
        r |= ((x & 0x7f) as u64) << s;
        s += 7;
        if x & 0x80 == 0 {
            return Ok(r);
        }
    }
}

/// The function imports of a wasm module as (module, name), in the order of the import section (what wasm_c_imports.py reads).
fn function_imports(b: &[u8]) -> Result<Vec<(String, String)>, String> {
    if b.len() < 8 || &b[..4] != b"\0asm" {
        return Err("not a wasm module".into());
    }
    let mut out = Vec::new();
    let mut i = 8;
    while i < b.len() {
        let sid = b[i];
        i += 1;
        let size = leb(b, &mut i)? as usize;
        let end = i + size;
        if sid == 2 {
            let n = leb(b, &mut i)?;
            for _ in 0..n {
                let name_of = |j: &mut usize| -> Result<String, String> {
                    let l = leb(b, j)? as usize;
                    let s = std::str::from_utf8(b.get(*j..*j + l).ok_or("truncated import")?).map_err(|e| e.to_string())?.to_string();
                    *j += l;
                    Ok(s)
                };
                let module = name_of(&mut i)?;
                let name = name_of(&mut i)?;
                let kind = *b.get(i).ok_or("truncated import")?;
                i += 1;
                match kind {
                    0 => {
                        leb(b, &mut i)?;
                        out.push((module, name));
                    }
                    1 => {
                        i += 1;
                        let flag = *b.get(i).ok_or("truncated import")?;
                        i += 1;
                        leb(b, &mut i)?;
                        if flag & 1 != 0 {
                            leb(b, &mut i)?;
                        }
                    }
                    2 => {
                        let flag = *b.get(i).ok_or("truncated import")?;
                        i += 1;
                        leb(b, &mut i)?;
                        if flag & 1 != 0 {
                            leb(b, &mut i)?;
                        }
                    }
                    3 => i += 2,
                    4 => {
                        i += 1;
                        leb(b, &mut i)?;
                    }
                    _ => return Err(format!("unknown import kind {kind}")),
                }
            }
            return Ok(out);
        }
        i = end;
    }
    Ok(out)
}

/// A section of a wasm module: its id, the offsets of its first byte and of its end, and its payload.
struct Section<'a> {
    id: u8,
    start: usize,
    end: usize,
    body: &'a [u8],
}

fn sections(b: &[u8]) -> Result<Vec<Section<'_>>, String> {
    if b.len() < 8 || &b[..4] != b"\0asm" {
        return Err("not a wasm module".into());
    }
    let mut out = Vec::new();
    let mut i = 8;
    while i < b.len() {
        let start = i;
        let id = b[i];
        i += 1;
        let size = leb(b, &mut i)? as usize;
        let end = i.checked_add(size).filter(|&e| e <= b.len()).ok_or("truncated section")?;
        out.push(Section { id, start, end, body: &b[i..end] });
        i = end;
    }
    Ok(out)
}

/// The rest of a custom section after its name, when the name is `name` (the names section); None for any other custom section.
fn name_section(body: &[u8]) -> Result<Option<&[u8]>, String> {
    let mut i = 0;
    let n = leb(body, &mut i)? as usize;
    Ok(if body.get(i..i + n) == Some(&b"name"[..]) { Some(&body[i + n..]) } else { None })
}

/// The names of the functions (the subsection 1 of the names section), by function index.
fn function_names(b: &[u8]) -> Result<BTreeMap<u32, String>, String> {
    let mut out = BTreeMap::new();
    for s in sections(b)? {
        let Some(mut rest) = (if s.id == 0 { name_section(s.body)? } else { None }) else { continue };
        while !rest.is_empty() {
            let id = rest[0];
            let mut i = 1;
            let size = leb(rest, &mut i)? as usize;
            let sub = rest.get(i..i + size).ok_or("truncated name subsection")?;
            rest = &rest[i + size..];
            if id != 1 {
                continue;
            }
            let mut j = 0;
            for _ in 0..leb(sub, &mut j)? {
                let idx = leb(sub, &mut j)? as u32;
                let l = leb(sub, &mut j)? as usize;
                let name = std::str::from_utf8(sub.get(j..j + l).ok_or("truncated function name")?).map_err(|e| e.to_string())?;
                j += l;
                out.insert(idx, name.to_string());
            }
        }
    }
    Ok(out)
}

/// The kind, the guest VA and the name of a function name of the generator: `f_<va>` (a lifted function), `ia_<va>` (its adapter for an indirect call)
/// and `t_<va>` (its test adapter). wasm-opt renames a second function of the same name to `<name>_<n>` (the hybrid wrapper of a lifted function
/// has the name of the function, which is also the function's own name): the suffix is dropped. Any other name is a C function, kept as it is.
fn classify(name: &str) -> (&'static str, String, String) {
    for (prefix, kind) in [("f_", "f"), ("ia_", "ia"), ("t_", "t")] {
        if let Some(rest) = name.strip_prefix(prefix) {
            let hex = match rest.split_once('_') {
                Some((hex, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => hex,
                Some(_) => continue,
                None => rest,
            };
            if !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return (kind, format!("0x{hex}"), format!("{prefix}{hex}"));
            }
        }
    }
    ("c", String::new(), name.to_string())
}

/// The table of the functions of a module (lift/out-app/gen/wasm-func-names.tsv): one line per function index, from the imports and then the defined functions,
/// with the kind (import, f, ia, t or c), the guest VA and the name. A defined function without a name in the names section is a c function named
/// unnamed_<index>: wasm-opt -Oz makes functions with none (the C part's, and some of the direct modules'), and some helpers of the generator have none.
/// The hybrid wrappers are named f_<va> by the generator (their export), so they are lifted code and not in this group.
fn names_table(b: &[u8]) -> Result<String, String> {
    let names = function_names(b)?;
    if names.is_empty() {
        return Err("the module has no function names (the names section is missing)".into());
    }
    let imports = function_imports(b)?;
    let mut defined = 0u64;
    for s in sections(b)? {
        if s.id == 3 {
            let mut i = 0;
            defined = leb(s.body, &mut i)?;
        }
    }
    let mut out = String::from("index\tkind\tva\tname\n");
    for idx in 0..imports.len() as u64 + defined {
        let (kind, va, name) = match imports.get(idx as usize) {
            Some((module, name)) => ("import", String::new(), format!("{module}.{name}")),
            None => match names.get(&(idx as u32)) {
                Some(name) => classify(name),
                None => ("c", String::new(), format!("unnamed_{idx}")),
            },
        };
        out.push_str(&format!("{idx}\t{kind}\t{va}\t{name}\n"));
    }
    Ok(out)
}

/// The module without its names section: the shipped engine has none (the table of names_table has them).
fn without_names(b: &[u8]) -> Result<Vec<u8>, String> {
    let secs = sections(b)?;
    let mut out = b[..8].to_vec();
    for s in secs {
        if s.id == 0 && name_section(s.body)?.is_some() {
            continue;
        }
        out.extend_from_slice(&b[s.start..s.end]);
    }
    Ok(out)
}

/// The names of the functions a wasm module exports (Issue #1342: what the C part's imports of a separate module must match).
fn function_exports(b: &[u8]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for s in sections(b)? {
        if s.id != 7 {
            continue;
        }
        let body = s.body;
        let mut i = 0;
        for _ in 0..leb(body, &mut i)? {
            let l = leb(body, &mut i)? as usize;
            let name = std::str::from_utf8(body.get(i..i + l).ok_or("truncated export")?).map_err(|e| e.to_string())?.to_string();
            i += l;
            let kind = *body.get(i).ok_or("truncated export")?;
            i += 1;
            leb(body, &mut i)?;
            if kind == 0 {
                out.push(name);
            }
        }
    }
    Ok(out)
}

/// The addresses of the C part from its link map: the first line whose fields are the given shape and whose last field is the name.
/// The shapes are the regular expressions of build-engine-bundle.sh: "hex", "dig" (decimal digits) or a literal in the field.
struct Addrs {
    teb: u64,
    _mxcsr: u64,
    _hyb_buf: u64,
    progress_count: u64,
    progress_limit: u64,
    landing_top: u64,
    dispatch_hook: u64,
    ctx_off: u64,
}

fn find_addr(map: &str, name: &str, shape: [&str; 3]) -> Option<u64> {
    for line in map.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 4 || f[3] != name {
            continue;
        }
        let ok = shape.iter().zip(&f[..3]).all(|(s, v)| match *s {
            "hex" => !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "dig" => !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()),
            lit => *v == lit,
        });
        if ok {
            return u64::from_str_radix(f[0], 16).ok();
        }
    }
    None
}

fn link_addrs(map_path: &str) -> Result<Addrs, String> {
    let map = std::fs::read_to_string(map_path).map_err(|e| format!("{map_path}: {e}"))?;
    let get = |name: &str, shape: [&str; 3]| find_addr(&map, name, shape).ok_or_else(|| format!("an address is missing in the link map: {name}"));
    Ok(Addrs {
        teb: get("fle_teb", ["hex", "hex", "hex"])?,
        _mxcsr: get("fle_mxcsr", ["hex", "hex", "4"])?,
        _hyb_buf: get("fle_hyb_buf", ["hex", "0", "40"])?,
        progress_count: get("fle_progress_count", ["hex", "hex", "8"])?,
        progress_limit: get("fle_progress_limit", ["hex", "hex", "dig"])?,
        landing_top: get("fle_landing_top", ["hex", "hex", "dig"])?,
        dispatch_hook: get("fle_dispatch_hook", ["hex", "hex", "dig"])?,
        ctx_off: get("fle_ctx", ["hex", "hex", "300"])?,
    })
}

fn words(path: &str) -> Result<Vec<String>, String> {
    Ok(std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?.split_whitespace().map(str::to_string).collect())
}

fn run_tool(name: &str, prog: &str, args: &[String]) -> Result<(), String> {
    let st = Command::new(prog).args(args).status().map_err(|e| format!("{name}: {prog}: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("{name} failed ({st})"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_map_addresses_follow_the_shapes_of_the_shell() {
        let map = "    1000    40    8 fle_teb\n    2200 0 40 fle_hyb_buf\n    3000 40 4 fle_mxcsr\n";
        assert_eq!(find_addr(map, "fle_teb", ["hex", "hex", "hex"]), Some(0x1000));
        assert_eq!(find_addr(map, "fle_hyb_buf", ["hex", "0", "40"]), Some(0x2200));
        assert_eq!(find_addr(map, "fle_teb", ["hex", "hex", "4"]), None);
        assert_eq!(find_addr(map, "fle_mxcsr", ["hex", "hex", "4"]), Some(0x3000));
    }

    #[test]
    fn function_imports_are_read_in_order_with_their_modules() {
        let b: Vec<u8> = [
            b"\0asm\x01\0\0\0".as_slice(),
            &[0x02, 0x16, 0x02],
            &[0x01, b'c', 0x02, b'f', b'1', 0x00, 0x00],
            &[0x03, b'e', b'n', b'v', 0x06, b'm', b'e', b'm', b'o', b'r', b'y', 0x02, 0x00, 0x01],
        ]
        .concat();
        assert_eq!(function_imports(&b).unwrap(), vec![("c".to_string(), "f1".to_string())]);
    }

    fn sec(id: u8, body: &[u8]) -> Vec<u8> {
        [&[id, body.len() as u8][..], body].concat()
    }

    #[test]
    fn names_table_lists_every_function_and_the_names_section_is_dropped() {
        let sub: Vec<u8> = [&[2u8, 1, 4][..], b"f_40", &[2, 18][..], b"fle_progress_slice"].concat();
        let names = sec(0, &[&[4u8][..], b"name", &[1, sub.len() as u8][..], &sub].concat());
        let imp = sec(2, &[&[1u8, 1][..], b"c", &[2], b"f1", &[0, 0]].concat());
        let module = [&b"\0asm\x01\0\0\0"[..], &sec(1, &[1, 0x60, 0, 0]), &imp, &sec(3, &[2, 0, 0]), &names].concat();
        let table = names_table(&module).unwrap();
        assert_eq!(table, "index\tkind\tva\tname\n0\timport\t\tc.f1\n1\tf\t0x40\tf_40\n2\tc\t\tfle_progress_slice\n");
        assert_eq!(without_names(&module).unwrap(), [&b"\0asm\x01\0\0\0"[..], &sec(1, &[1, 0x60, 0, 0]), &imp, &sec(3, &[2, 0, 0])].concat());
    }

    #[test]
    fn a_defined_function_without_a_name_is_a_c_function_named_by_its_index() {
        let sub: Vec<u8> = [&[1u8, 1, 4][..], b"f_40"].concat();
        let names = sec(0, &[&[4u8][..], b"name", &[1, sub.len() as u8][..], &sub].concat());
        let module = [&b"\0asm\x01\0\0\0"[..], &sec(1, &[1, 0x60, 0, 0]), &sec(3, &[2, 0, 0]), &names].concat();
        assert_eq!(names_table(&module).unwrap(), "index\tkind\tva\tname\n0\tc\t\tunnamed_0\n1\tf\t0x40\tf_40\n");
    }

    #[test]
    fn a_name_that_wasm_opt_renamed_keeps_its_lift_name() {
        let sub: Vec<u8> = [&[2u8, 0, 4][..], b"f_40", &[1, 9][..], b"f_40_5688"].concat();
        let names = sec(0, &[&[4u8][..], b"name", &[1, sub.len() as u8][..], &sub].concat());
        let module = [&b"\0asm\x01\0\0\0"[..], &sec(1, &[1, 0x60, 0, 0]), &sec(3, &[2, 0, 0]), &names].concat();
        assert_eq!(names_table(&module).unwrap(), "index\tkind\tva\tname\n0\tf\t0x40\tf_40\n1\tf\t0x40\tf_40\n");
    }

    /// Issue #1395: a module row's arguments are its row's (the `--rva-tables FILE` of a plugin row is one argument pair, as the row gives it).
    #[test]
    fn the_module_rows_keep_the_arguments_of_their_row_and_the_rva_table_pair() {
        let m = modules_of("effects.x.x_x64.dll\teffects.x.x_x64.6800000.dll\t--pdata --reloc-seeds --rva-tables lift/plugins/x/rva-tables.txt\n").unwrap();
        assert_eq!(m[0].pe, "effects.x.x_x64.6800000.dll");
        assert_eq!(m[0].args, ["--pdata", "--reloc-seeds", "--rva-tables", "lift/plugins/x/rva-tables.txt"]);
    }

    #[test]
    fn function_exports_are_the_function_entries_of_the_export_section() {
        let b: Vec<u8> = [b"\0asm\x01\0\0\0".as_slice(), &[0x07, 0x0b, 0x02, 0x03, b'f', b'_', b'1', 0x00, 0x00, 0x01, b'm', 0x02, 0x00]].concat();
        assert_eq!(function_exports(&b).unwrap(), vec!["f_1".to_string()]);
    }
}

pub fn run_n0(args: &[String]) -> ExitCode {
    match n0(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hybrid-n0: {e}");
            ExitCode::FAILURE
        }
    }
}

fn n0(args: &[String]) -> Result<(), String> {
    let o = opts(args)?;
    let modules = read_modules(req(&o, "modules")?)?;
    let lists = PathBuf::from(req(&o, "lists")?);
    let dll_dir = PathBuf::from(req(&o, "dll-dir")?);
    let work = PathBuf::from(req(&o, "work")?);
    let fixed = words(req(&o, "args")?)?;
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let mut used = BTreeSet::new();
    for m in &modules {
        let Some(want) = list_of(&lists, &m.name)? else { continue };
        let list_path = lists.join(format!("lowered-{}.txt", m.name));
        let out = work.join(format!("n0-{}.wasm", m.name));
        let report = work.join(format!("n0-{}.tsv", m.name));
        compile_module(m, &dll_dir, &fixed, &list_path, &out, &report)?;
        check_listed(&m.name, &want, &report)?;
        for (module, name) in function_imports(&std::fs::read(&out).map_err(|e| e.to_string())?)? {
            if module == "c" {
                used.insert(name);
            }
        }
    }
    let rsp = req(&o, "out-rsp")?;
    // Issue #1342: a name the link wraps (--wrap, link.rsp) is defined as __wrap_<name>; the export of that wrapper lets a separate module's import reach it (xmod.ts)
    let text: String = used.iter().map(|n| format!("-Wl,--export-if-defined={n}\n-Wl,--export-if-defined=__wrap_{n}\n")).collect();
    std::fs::write(rsp, text).map_err(|e| format!("{rsp}: {e}"))?;
    println!("exports.rsp: {} names", used.len());
    Ok(())
}

pub fn run(args: &[String]) -> ExitCode {
    match hybrid(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hybrid: {e}");
            ExitCode::FAILURE
        }
    }
}

fn hybrid(args: &[String]) -> Result<(), String> {
    let o = opts(args)?;
    let modules = read_modules(req(&o, "modules")?)?;
    let lists = PathBuf::from(req(&o, "lists")?);
    let dll_dir = PathBuf::from(req(&o, "dll-dir")?);
    let work = PathBuf::from(req(&o, "work")?);
    let fixed = words(req(&o, "args")?)?;
    let a = link_addrs(req(&o, "link-map")?)?;
    let c_part = req(&o, "c-part")?;
    let out = PathBuf::from(req(&o, "out")?);
    let opt_bin = req(&o, "wasm-opt")?;
    let opt_args = words(req(&o, "wasm-opt-args")?)?;
    let merge_bin = req(&o, "wasm-merge")?;
    let mut features = words(req(&o, "features")?)?;
    features.push("-g".into());
    let dir = work.join("dir");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut extra: Vec<String> = vec![
        "--teb".into(),
        format!("0x{:x}", a.teb),
        "--tls-ctx".into(),
        format!("0x{:x}", a.ctx_off),
        "--hybrid-progress".into(),
        format!("0x{:x}", a.progress_count),
        "--hybrid-dispatch".into(),
        format!("0x{:x}", a.progress_limit),
        format!("0x{:x}", a.landing_top),
        format!("0x{:x}", a.dispatch_hook),
    ];
    extra.extend(fixed);
    // Issue #1342: the modules of --separate are not merged. Each is written to --separate-dir as <DLL stem>.wasm, and the page
    // instantiates it at the guest's first LoadLibrary of the module (app/src/engine/xmod.ts). Its C imports are the engine's exports.
    let separate: BTreeSet<String> = o.get("separate").map(|s| s.split(',').filter(|n| !n.is_empty()).map(str::to_string).collect()).unwrap_or_default();
    let sep_dir = PathBuf::from(o.get("separate-dir").map(String::as_str).unwrap_or("."));
    let mut ins: Vec<(PathBuf, String)> = Vec::new();
    let mut separated: Vec<PathBuf> = Vec::new();
    let mut built: BTreeSet<String> = BTreeSet::new();
    for m in &modules {
        let Some(want) = list_of(&lists, &m.name)? else { continue };
        let list_path = lists.join(format!("lowered-{}.txt", m.name));
        let raw = dir.join(format!("n-{}.wasm", m.name));
        let oz = dir.join(format!("n-{}.oz.wasm", m.name));
        let report = dir.join(format!("n-{}.tsv", m.name));
        compile_module(m, &dll_dir, &extra, &list_path, &raw, &report)?;
        check_listed(&m.name, &want, &report)?;
        let mut argv = opt_args.clone();
        argv.extend(["-o".into(), oz.display().to_string(), raw.display().to_string()]);
        run_tool(&format!("wasm-opt of the direct module {}", m.name), opt_bin, &argv)?;
        if separate.contains(&m.name) {
            let stem = Path::new(&m.pe).file_stem().and_then(|s| s.to_str()).ok_or_else(|| format!("no file stem in {}", m.pe))?;
            std::fs::create_dir_all(&sep_dir).map_err(|e| format!("{}: {e}", sep_dir.display()))?;
            let dst = sep_dir.join(format!("{stem}.wasm"));
            // the shipped module has no names section (as the engine); its names are the table wasm-func-names-<module>.tsv of the work directory
            let bytes = std::fs::read(&oz).map_err(|e| format!("{}: {e}", oz.display()))?;
            std::fs::write(work.join(format!("wasm-func-names-{}.tsv", m.name)), names_table(&bytes)?).map_err(|e| format!("wasm-func-names-{}.tsv: {e}", m.name))?;
            std::fs::write(&dst, without_names(&bytes)?).map_err(|e| format!("{}: {e}", dst.display()))?;
            separated.push(dst);
            built.insert(m.name.clone());
            continue;
        }
        ins.push((oz, format!("d{}", m.name)));
    }
    if let Some(name) = separate.difference(&built).next() {
        return Err(format!("separate module {name} is not a module with a list"));
    }
    let direct = work.join("direct.wasm");
    let mut argv = features.clone();
    argv.push("--rename-export-conflicts".into());
    for (p, n) in &ins {
        argv.push(p.display().to_string());
        argv.push(n.clone());
    }
    argv.extend(["-o".into(), direct.display().to_string()]);
    run_tool("wasm-merge of the direct modules", merge_bin, &argv)?;
    let merged = work.join("merged.wasm");
    let mut argv = features;
    argv.extend(["--rename-export-conflicts".into(), direct.display().to_string(), "env".into(), c_part.to_string(), "c".into(), "-o".into(), merged.display().to_string()]);
    run_tool("wasm-merge into the C part", merge_bin, &argv)?;
    let bytes = std::fs::read(&merged).map_err(|e| e.to_string())?;
    let imps = function_imports(&bytes)?;
    if imps.iter().any(|(m, _)| m == "c") {
        return Err("the merged module still imports functions of the module c".into());
    }
    // Issue #1342: a lifted function of a separate module is the one import of env that the merged module keeps (the page supplies it)
    let c_exports: BTreeSet<String> = function_exports(&std::fs::read(c_part).map_err(|e| format!("{c_part}: {e}"))?)?.into_iter().collect();
    let mut provided: BTreeSet<String> = BTreeSet::new();
    for dst in &separated {
        let b = std::fs::read(dst).map_err(|e| format!("{}: {e}", dst.display()))?;
        for (m, n) in function_imports(&b)? {
            if m != "c" {
                return Err(format!("the separate module {} imports {m}.{n}: a separate module imports only the C part (module c), so no call reaches another separate module", dst.display()));
            }
            if !c_exports.contains(&n) && !c_exports.contains(&format!("__wrap_{n}")) {
                return Err(format!("the separate module {} imports c.{n}, which the C part does not export", dst.display()));
            }
        }
        provided.extend(function_exports(&b)?.into_iter().filter(|n| n.starts_with("f_")));
    }
    if imps.iter().any(|(m, n)| m == "env" && n.starts_with("f_") && !provided.contains(n)) {
        return Err("the merged module still imports a lifted function (f_<va>) from env: a listed function is not defined by the direct code".into());
    }
    std::fs::write(work.join(FUNC_NAMES), names_table(&bytes)?).map_err(|e| format!("{FUNC_NAMES}: {e}"))?;
    let shipped = without_names(&bytes)?;
    std::fs::write(&out, &shipped).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("hybrid: {} bytes, {} direct modules, {} separate modules, the function names in {}", shipped.len(), ins.len(), separated.len(), work.join(FUNC_NAMES).display());
    Ok(())
}
