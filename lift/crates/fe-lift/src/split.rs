//! The translation units of one lifted module (`fe-lift lift IN PREFIX --split-bytes N`), written directly from the generated C.
//! The cut is the rule of `harness/two/split_bytes.py` (decision 0130): a unit is the prologue (up to the dispatch table) and a run of
//! whole functions, and a unit ends before the function that would take it over N bytes (a function larger than N is a unit of its own).
//! The sizes are those of the functions as generated, bodies included, so the cuts do not depend on `--omit-list`.
//! Differences from the Python rule, which the build needs: the dispatch table goes to PREFIX-tables.txt (as `split_mod.py` writes it),
//! the declarations of R, FL and X are wrapped in `#ifndef FLE_LOWREGS` (`FLE_LOWREGS_X` for X) as `harness/two/lowregs.py` wraps them
//! (the wasm build defines both macros in fle_runtime.h, so it gets no declaration; the native build gets the declarations), and the bodies
//! of the functions in the omit list are left out of the units with their prototypes (what `harness/hyb/strip_units.py` does).

use std::collections::BTreeSet;
use std::io::Write;

pub struct Split<'a> {
    pub prefix: &'a str,
    pub max_bytes: usize,
    pub omit: &'a BTreeSet<u64>,
}

pub struct Report {
    pub functions: usize,
    pub parts: usize,
    pub omitted: usize,
    pub largest: usize,
}

/// The hex part of a function head `void f_<hex>(void)`.
fn head_hex(l: &str) -> Option<&str> {
    let h = l.strip_prefix("void f_")?.strip_suffix("(void)")?;
    (!h.is_empty() && h.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))).then_some(h)
}

/// A prologue line: R, FL and X become extern declarations, each in its guard (what split_bytes.py and lowregs.py make of them).
fn prologue_line(l: &str, out: &mut String) {
    let (guard, decl) = if l.starts_with("uint64_t R[16];") {
        ("FLE_LOWREGS", "extern uint64_t R[16];")
    } else if l.starts_with("fle_eflags FL = ") && l.ends_with(';') {
        ("FLE_LOWREGS", "extern fle_eflags FL;")
    } else if l.starts_with("fle_xmm X[16];") {
        ("FLE_LOWREGS_X", "extern fle_xmm X[16];")
    } else {
        out.push_str(l);
        out.push('\n');
        return;
    };
    out.push_str(&format!("#ifndef {guard}\n{decl}\n#endif\n"));
}

/// The VAs of a list (`--omit-list`): the first field of each non-empty line, hex with an optional 0x.
pub fn parse_omit(text: &str) -> Result<BTreeSet<u64>, String> {
    let mut set = BTreeSet::new();
    for l in text.lines() {
        let Some(tok) = l.split_whitespace().next() else { continue };
        set.insert(u64::from_str_radix(tok.trim_start_matches("0x"), 16).map_err(|e| format!("bad VA {tok}: {e}"))?);
    }
    Ok(set)
}

pub fn write_units(text: &str, sp: &Split) -> Result<Report, String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let first_def = (0..lines.len().saturating_sub(1))
        .find(|&i| head_hex(lines[i]).is_some() && lines[i + 1] == "{")
        .ok_or("no function in the generated C")?;
    let table = lines
        .iter()
        .position(|l| l.starts_with("const fle_fn_entry fle_dispatch_table[]"))
        .ok_or("no dispatch table in the generated C")?;
    if table >= first_def {
        return Err("the dispatch table is not before the first function".into());
    }
    let mut pro = String::new();
    for l in &lines[..table] {
        prologue_line(l, &mut pro);
    }
    let body = &lines[first_def..];
    let mut starts: Vec<usize> = (0..body.len().saturating_sub(1)).filter(|&i| head_hex(body[i]).is_some() && body[i + 1] == "{").collect();
    starts.push(body.len());
    let nf = starts.len() - 1;
    let sizes: Vec<usize> = (0..nf).map(|k| body[starts[k]..starts[k + 1]].iter().map(|l| l.chars().count() + 1).sum()).collect();
    let mut parts: Vec<(usize, usize)> = Vec::new();
    let (mut a, mut cur) = (0usize, 0usize);
    for (k, &sz) in sizes.iter().enumerate() {
        if k > a && cur + sz > sp.max_bytes {
            parts.push((a, k));
            a = k;
            cur = 0;
        }
        cur += sz;
    }
    parts.push((a, nf));
    if parts.len() >= 100 {
        return Err(format!("{} units: the budget {} is too small (two digits)", parts.len(), sp.max_bytes));
    }
    let mut close: Vec<Option<usize>> = Vec::with_capacity(nf);
    for k in 0..nf {
        let (s0, s1) = (starts[k], starts[k + 1]);
        let va = u64::from_str_radix(head_hex(body[s0]).unwrap_or(""), 16).map_err(|e| format!("{}: {e}", body[s0]))?;
        if sp.omit.contains(&va) {
            let j = (s0 + 2..s1).find(|&j| body[j] == "}").ok_or_else(|| format!("{}: no closing brace", body[s0]))?;
            close.push(Some(j));
        } else {
            close.push(None);
        }
    }
    let omitted = close.iter().filter(|c| c.is_some()).count();
    let tables = lines[table..first_def].join("\n") + "\n";
    write_new(&format!("{}-tables.txt", sp.prefix), &tables)?;
    let mut largest = 0;
    for (n, &(a, b)) in parts.iter().enumerate() {
        let mut piece: Vec<&str> = Vec::new();
        for k in a..b {
            match close[k] {
                Some(j) => piece.extend_from_slice(&body[j + 1..starts[k + 1]]),
                None => piece.extend_from_slice(&body[starts[k]..starts[k + 1]]),
            }
        }
        largest = largest.max(sizes[a..b].iter().sum());
        write_new(&format!("{}-{:02}.c", sp.prefix, n), &format!("{pro}{}\n", piece.join("\n")))?;
    }
    Ok(Report { functions: nf, parts: parts.len(), omitted, largest })
}

/// Nothing is written over an existing file (mode 'x' of the Python tools).
fn write_new(path: &str, text: &str) -> Result<(), String> {
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| format!("{path}: {e}"))?;
    f.write_all(text.as_bytes()).map_err(|e| format!("{path}: {e}"))
}
