//! `fe-lift gen-all` (decision 0405, F1): the rebase, the lift and the merge of FLEngine and of the real
//! modules in one run. Every lift is `lift_main` (the `fe-lift lift` of main.rs) called once per file with
//! the arguments of its row, so the units are the ones of `--split-bytes` and `--omit-list`. The merged
//! dispatch table (`common.c`) is the output of merge_tables_n.py with the lowering of R, FL and X of lowregs.py,
//! and `realmod.c` the output of realmod_tables.awk (gen-direct.sh).
//!
//! usage: fe-lift gen-all --modules TSV --fle-args FILE --host DIR --out DIR --split-bytes N
//!        [--omit-list FILE] [--extra-rows FILE]
//! TSV: one module per line, five TAB-separated fields: realmod name, file (relative to --host), base, lift
//! arguments (whitespace-separated, may be empty when given as the last field), realmod path suffix (or empty).
//! Output in DIR: dll/<stem>.<base hex>.dll (rebased), src/fle-NN.c and src/fle-tables.txt, src/mK-NN.c and
//! src/mK-tables.txt, common.c, realmod.c.

use std::collections::BTreeMap;
use std::fs;
use std::process::ExitCode;

const RESP_BASE: u64 = 0x7FF9_0000_0000;

const USAGE: &str = "usage: fe-lift gen-all --modules TSV --fle-args FILE --host DIR --out DIR --split-bytes N [--omit-list FILE] [--extra-rows FILE]";

struct Module {
    name: String,
    src: String,
    base: u64,
    base_text: String,
    args: Vec<String>,
    suffix: String,
}

pub fn main(argv: &[String]) -> ExitCode {
    match run(argv) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: gen-all: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(argv: &[String]) -> Result<(), String> {
    let (mut modules, mut fle_args, mut host, mut out) = (None, None, None, None);
    let (mut split, mut omit, mut extra_rows): (Option<usize>, Option<String>, Option<String>) = (None, None, None);
    let mut i = 0;
    while i < argv.len() {
        let v = argv.get(i + 1).ok_or_else(|| format!("{} needs a value\n{USAGE}", argv[i]))?.clone();
        match argv[i].as_str() {
            "--modules" => modules = Some(v),
            "--fle-args" => fle_args = Some(v),
            "--host" => host = Some(v),
            "--out" => out = Some(v),
            "--split-bytes" => split = Some(v.parse::<usize>().map_err(|_| format!("bad --split-bytes {v}"))?),
            "--omit-list" => omit = Some(v),
            "--extra-rows" => extra_rows = Some(v),
            _ => return Err(format!("unknown option {}\n{USAGE}", argv[i])),
        }
        i += 2;
    }
    let need = |o: Option<String>, n: &str| o.ok_or_else(|| format!("{n} is required\n{USAGE}"));
    let modules_path = need(modules, "--modules")?;
    let fle_args_path = need(fle_args, "--fle-args")?;
    let host = need(host, "--host")?;
    let out = need(out, "--out")?;
    let split = split.ok_or_else(|| format!("--split-bytes is required\n{USAGE}"))?;

    let mods = read_modules(&modules_path)?;
    let fle_text = fs::read_to_string(&fle_args_path).map_err(|e| format!("{fle_args_path}: {e}"))?;
    let mut extra: Option<String> = None;
    let fle_argv = strip_extra(fle_text.split_whitespace().map(String::from).collect(), &mut extra)?;
    let mod_argv: Vec<Vec<String>> = mods.iter().map(|m| strip_extra(m.args.clone(), &mut extra)).collect::<Result<_, _>>()?;
    // The extra import rows are a process-wide table (fe_lift::imports_table::set_extra, once): set here, before any lift.
    if let Some(p) = &extra {
        let text = fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
        fe_lift::imports_table::set_extra(&text).map_err(|e| format!("{p}: {e}"))?;
    }

    let dll_dir = format!("{out}/dll");
    let src_dir = format!("{out}/src");
    fs::create_dir_all(&dll_dir).map_err(|e| format!("{dll_dir}: {e}"))?;
    fs::create_dir_all(&src_dir).map_err(|e| format!("{src_dir}: {e}"))?;

    // The rebase (fe-pe rebase, the same function): the module files at their base.
    let mut rebased = Vec::new();
    for m in &mods {
        let src = format!("{host}/{}", m.src);
        let data = fs::read(&src).map_err(|e| format!("{src}: {e}"))?;
        let (bytes, _) = fe_pe::rebase::rebase(&data, m.base).map_err(|e| format!("{src}: {e}"))?;
        let path = format!("{dll_dir}/{}.{:x}.dll", stem(&m.src), m.base);
        fs::write(&path, bytes).map_err(|e| format!("{path}: {e}"))?;
        rebased.push(path);
    }

    let fle_in = format!("{host}/binaries/win64/FLEngine_x64.dll");
    let mut prefixes = vec![format!("{src_dir}/fle")];
    lift_one(&fle_in, &prefixes[0], &fle_argv, split, omit.as_deref())?;
    for k in 0..mods.len() {
        let prefix = format!("{src_dir}/m{}", k + 1);
        lift_one(&rebased[k], &prefix, &mod_argv[k], split, omit.as_deref())?;
        prefixes.push(prefix);
    }

    let mut tables = Vec::new();
    for p in &prefixes {
        let path = format!("{p}-tables.txt");
        tables.push(fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?);
    }
    let common = merge(&tables, extra_rows.as_deref())?;
    fs::write(format!("{out}/common.c"), common).map_err(|e| format!("{out}/common.c: {e}"))?;
    fs::write(format!("{out}/realmod.c"), realmod(&mods)).map_err(|e| format!("{out}/realmod.c: {e}"))?;
    println!("gen-all: {} modules, {} tables merged", mods.len(), tables.len());
    Ok(())
}

/// One `fe-lift lift FILE PREFIX ... --split-bytes N [--omit-list F]` call through `lift_main`.
fn lift_one(input: &str, prefix: &str, args: &[String], split: usize, omit: Option<&str>) -> Result<(), String> {
    let mut v: Vec<String> = vec!["fe-lift".into(), "lift".into(), input.into(), prefix.into()];
    v.extend(args.iter().cloned());
    v.push("--split-bytes".into());
    v.push(split.to_string());
    if let Some(o) = omit {
        v.push("--omit-list".into());
        v.push(o.into());
    }
    match crate::lift_main(&v) {
        c if c == ExitCode::SUCCESS => Ok(()),
        _ => Err(format!("lift of {input} failed")),
    }
}

/// Takes `--extra-imports FILE` out of an argument list. Every row must name the same file (one table per process).
fn strip_extra(args: Vec<String>, extra: &mut Option<String>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--extra-imports" {
            let p = args.get(i + 1).ok_or("--extra-imports needs a file")?.clone();
            match extra {
                Some(q) if *q != p => return Err(format!("--extra-imports: {q} and {p} differ (one table per run)")),
                _ => *extra = Some(p),
            }
            i += 2;
        } else {
            out.push(args[i].clone());
            i += 1;
        }
    }
    Ok(out)
}

fn stem(src: &str) -> String {
    let name = src.rsplit('/').next().unwrap_or(src);
    name.strip_suffix(".dll").unwrap_or(name).to_string()
}

fn read_modules(path: &str) -> Result<Vec<Module>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut mods = Vec::new();
    for (no, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 5 {
            return Err(format!("{path}:{}: want five TAB-separated fields", no + 1));
        }
        let base = u64::from_str_radix(f[2].trim_start_matches("0x"), 16).map_err(|_| format!("{path}:{}: bad base {}", no + 1, f[2]))?;
        mods.push(Module {
            name: f[0].to_string(),
            src: f[1].to_string(),
            base,
            base_text: f[2].to_string(),
            args: f[3].split_whitespace().map(String::from).collect(),
            suffix: f[4].to_string(),
        });
    }
    Ok(mods)
}

/// `    { UINT64_C(0x<hex>), <name> },` (the row form of merge_tables_n.py), or None.
fn parse_row(line: &str) -> Option<(u64, String)> {
    let rest = line.strip_prefix("    { UINT64_C(0x")?;
    let (hex, rest) = rest.split_once("), ")?;
    let name = rest.strip_suffix(" },")?;
    if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return None;
    }
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    Some((u64::from_str_radix(hex, 16).ok()?, name.to_string()))
}

/// The merge of merge_tables_n.py: rows by VA (sorted), the collisions of VAs below the pseudo-VA region,
/// the extra rows, and the response table of the first table. R, FL and X are written as lowregs.py wraps them.
fn merge(tables: &[String], extra_rows: Option<&str>) -> Result<String, String> {
    let mut rows: BTreeMap<u64, String> = BTreeMap::new();
    let mut count: BTreeMap<u64, usize> = BTreeMap::new();
    let mut resp: Option<String> = None;
    for text in tables {
        let lines: Vec<&str> = text.split('\n').collect();
        let mut intab = false;
        for l in &lines {
            if l.starts_with("const fle_fn_entry fle_dispatch_table[]") {
                intab = true;
                continue;
            }
            if !intab {
                continue;
            }
            if let Some((va, name)) = parse_row(l) {
                *count.entry(va).or_insert(0) += 1;
                match rows.get(&va) {
                    Some(old) if *old != name => return Err(format!("VA {va:#x} is {old} and {name}")),
                    Some(_) => {}
                    None => {
                        rows.insert(va, name);
                    }
                }
            } else if l.starts_with("};") {
                intab = false;
            }
        }
        let i = lines.iter().position(|l| l.starts_with("const fle_resp_entry fle_resp_table[] = {")).ok_or("no response table")?;
        let j = (i..lines.len()).find(|&k| lines[k].starts_with("const uint64_t fle_resp_count")).ok_or("no response count")?;
        let r = lines[i..=j].join("\n");
        match &resp {
            None => resp = Some(r),
            Some(old) if *old != r => return Err("response tables differ".into()),
            Some(_) => {}
        }
    }
    let coll: Vec<u64> = count.iter().filter(|(va, c)| **c > 1 && **va < RESP_BASE).map(|(va, _)| *va).collect();
    if !coll.is_empty() {
        return Err(format!("f_<VA> collision between modules: {} (first {:#x})", coll.len(), coll[0]));
    }
    if let Some(p) = extra_rows {
        let text = fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
        for (no, l) in text.split('\n').enumerate() {
            let t = l.split('#').next().unwrap_or("").trim();
            if t.is_empty() {
                continue;
            }
            let parts: Vec<&str> = t.split_whitespace().collect();
            let bad = |why: &str| format!("{p}:{}: {why}", no + 1);
            if parts.len() != 2 {
                return Err(bad("want \"<0x VA> <name>\""));
            }
            let va = u64::from_str_radix(parts[0].strip_prefix("0x").ok_or_else(|| bad("bad VA"))?, 16).map_err(|_| bad("bad VA"))?;
            let name = parts[1];
            if va < RESP_BASE || rows.contains_key(&va) || rows.values().any(|n| n == name) {
                return Err(bad("VA or name already used, or below the pseudo-VA region"));
            }
            rows.insert(va, name.to_string());
        }
    }
    let word = match tables.len() {
        2 => "two".to_string(),
        3 => "three".to_string(),
        n => n.to_string(),
    };
    let mut s = format!("/* design aid: merged dispatch table of {word} modules */\n#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n");
    s += "#include \"fle_dispatch.h\"\n#include \"fle_modules.h\"\n\n";
    s += "#ifndef FLE_LOWREGS\nuint64_t R[16];\n#endif\n";
    s += "#ifndef FLE_LOWREGS\nfle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };\n#endif\n";
    s += "#ifndef FLE_LOWREGS_X\nfle_xmm X[16];\n#endif\n\n";
    for name in rows.values() {
        s += &format!("void {name}(void);\n");
    }
    s += "\nconst fle_fn_entry fle_dispatch_table[] = {\n";
    for (va, name) in &rows {
        s += &format!("    {{ UINT64_C(0x{va:x}), {name} }},\n");
    }
    s += &format!("}};\nconst uint64_t fle_dispatch_count = {};\n\n", rows.len());
    s += resp.as_deref().ok_or("no response table")?;
    s += "\n";
    Ok(s)
}

/// realmod_tables.awk with the gen-direct.sh names and paths.
fn realmod(mods: &[Module]) -> String {
    let mut s = String::from("/* generated by lift/harness/b2d2/gen-direct.sh from modules.txt as lift/harness/two/build_two_native.sh does */\n");
    s += "#include \"fle_realmod.h\"\n";
    s += "const fle_realmod_entry fle_realmod_table[] = {\n";
    for m in mods {
        s += &format!("    {{ \"{}\", UINT64_C({}) }},\n", m.name, m.base_text);
    }
    s += "};\n";
    s += &format!("const uint64_t fle_realmod_count = {};\n", mods.len());
    let paths: Vec<&Module> = mods.iter().filter(|m| !m.suffix.is_empty()).collect();
    if !paths.is_empty() {
        s += "const fle_realmod_path_entry fle_realmod_path_table[] = {\n";
        for m in &paths {
            s += &format!("    {{ \"{}\", \"{}\" }},\n", m.suffix.replace('/', "\\\\"), m.name);
        }
        s += "};\n";
        s += &format!("const uint64_t fle_realmod_path_count = {};\n", paths.len());
    }
    s
}
