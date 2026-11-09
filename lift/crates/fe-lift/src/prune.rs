//! Pruned functions (decision 0168, provisional): guest functions that the environment cannot reach, taken out of the generated C.
//!
//! A pruned function is a function entry of the closure only where kept code reaches it (a call, a jump, a case of a selector stub); it is not a seed,
//! its body is not decoded and nothing behind it is followed, so what only pruned functions reach is not generated either. Where kept code does reach it,
//! the generated C holds a stub that stops the run by name:
//!
//! ```c
//! void f_2741fa0(void)
//! {
//!     fle_pruned(UINT64_C(0x2741fa0));
//! }
//! ```
//!
//! `fle_pruned` is in runtime/fle_prune.h (included by the generated C only when a prune list is given; it prints the VA and aborts, like
//! `fle_dispatch_miss`). An indirect call to a pruned function that kept code never names is `fle_dispatch_miss`, which also prints the VA. Without
//! `--prune-list` the generator's output is byte for byte what it was.
//!
//! The file read by `fe-lift lift ... --prune-list FILE` is a TSV (docs/investigations/prune1/tables; written by judge.py of the former tools/prune): `#` starts a comment line, empty lines are skipped,
//! the first column is the address (a hex VA of the image as lifted, with or without `0x`, or `rva:` and a hex RVA), the second column (optional) is the
//! class of the judgement (`S` strong, `M` middle; other text is kept as a label), further columns are ignored. A row of class `U` is not a pruned
//! function: its address is a table jump (a selector stub, an inline or a switch table) that stays unresolved, i.e. a `fle_dispatch` jump, as it is in the
//! lift without a prune list. The checks that accept or reject a table look at the decoded code around it, and a smaller closure can accept a jump that the
//! complete closure rejects; the dispatch count feeds the guest's pseudo-preemption clock (decision 0147), so the pruned lift must decide as the complete one.

use crate::hook::Addr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub addr: Addr,
    pub class: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub va: u64,
    pub class: String,
}

fn hex(s: &str) -> Result<u64, String> {
    let h = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    u64::from_str_radix(h, 16).map_err(|e| format!("bad hex number {s:?}: {e}"))
}

/// Parses a prune list (see the module comment). A first line that starts with `va` (the header of the TSV) is skipped.
pub fn parse_file(text: &str) -> Result<Vec<Spec>, String> {
    let mut v = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with("va\t") || line == "va" {
            continue;
        }
        let err = |e: String| format!("line {}: {e}", n + 1);
        let f: Vec<&str> = line.split_whitespace().collect();
        let addr = match f[0].strip_prefix("rva:") {
            Some(r) => Addr::Rva(hex(r.trim()).map_err(&err)?),
            None => Addr::Va(hex(f[0]).map_err(&err)?),
        };
        v.push(Spec { addr, class: f.get(1).copied().unwrap_or("-").to_string() });
    }
    Ok(v)
}

/// The entries of `specs` for an image with this ImageBase, in file order.
pub fn resolve(specs: &[Spec], image_base: u64) -> Vec<Entry> {
    specs
        .iter()
        .map(|s| Entry {
            va: match s.addr {
                Addr::Va(v) => v,
                Addr::Rva(r) => image_base.wrapping_add(r),
            },
            class: s.class.clone(),
        })
        .collect()
}

/// Checks the list: a function is listed once.
pub fn check_unique(entries: &[Entry]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for e in entries {
        if !seen.insert(e.va) {
            return Err(format!("prune entry {:#x} is listed twice", e.va));
        }
    }
    Ok(())
}

/// The C text of the stub that stands in for the function at `va` (name, body, and a blank line, as the other functions end).
pub fn stub(va: u64) -> String {
    format!("void f_{va:x}(void)\n{{\n    fle_pruned(UINT64_C({va:#x}));\n}}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_has_va_rva_class_header_and_comments() {
        let t = "# prune\nva\tclass\tc_bytes\n\n0x2741fa0\tS\t100\nrva:0x1000 M # x\n2741fa8\n";
        let v = parse_file(t).unwrap();
        assert_eq!(
            v,
            [
                Spec { addr: Addr::Va(0x2741fa0), class: "S".into() },
                Spec { addr: Addr::Rva(0x1000), class: "M".into() },
                Spec { addr: Addr::Va(0x2741fa8), class: "-".into() },
            ]
        );
        let e = resolve(&v, 0x2000000);
        assert_eq!(e[1].va, 0x2001000);
    }

    #[test]
    fn bad_hex_and_duplicates_are_errors() {
        assert!(parse_file("zz S\n").unwrap_err().contains("line 1"));
        let e = resolve(&parse_file("0x10 S\n0x10 M\n").unwrap(), 0);
        assert!(check_unique(&e).is_err());
    }

    #[test]
    fn class_u_rows_parse_like_the_others() {
        let v = parse_file("0x2c227de\tU\t-\t0\t0\n").unwrap();
        assert_eq!(v, [Spec { addr: Addr::Va(0x2c227de), class: "U".into() }]);
    }

    #[test]
    fn stub_text() {
        assert_eq!(stub(0x2741fa0), "void f_2741fa0(void)\n{\n    fle_pruned(UINT64_C(0x2741fa0));\n}\n\n");
    }
}
