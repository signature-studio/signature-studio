//! HLE entries (decision 0151, provisional; adopted as an opt-in by decision 0162, provisional): a guest function that the host may run instead.
//!
//! An entry names the first instruction of one lifted function and the name of a host implementation. The generated C gets one guard in
//! the block of that first instruction and nothing else changes:
//!
//! ```c
//! L_6a73a0: { { extern int fle_hle_blend_row(void); if (fle_hle_blend_row()) return; } /* 0x6a73a0 Push */ ... }
//! ```
//!
//! `fle_hle_<name>()` (runtime/fle_hle.c) returns 1 when it did everything the function does, with the same registers, flags and memory as
//! the lifted body would leave (the lifted function then just returns), and 0 when it does not handle this call (it has not touched
//! anything; the lifted body runs). The run chooses what the guard does (`FLE_HLE=0` off, `check` compares with the lifted body, otherwise on; the build's default `-DFLE_HLE_DEFAULT`, runtime/fle_hle.h). Without `--hle-entries` the
//! generator's output is byte for byte what it was. The guard names its function inside the block (a block-scope `extern`), so no
//! `#include` line and no declaration of the unit changes: only the unit that holds the function differs, and a missing host function is a
//! link error.
//!
//! The file read by `fe-lift lift ... --hle-entries FILE`: one entry per line, `ADDR NAME`, `ADDR` as in a hook-points file (a hex VA of the
//! image as lifted, with or without `0x`, or `rva:` and a hex RVA), `NAME` a C identifier suffix (letters, digits and `_`). `#` starts a
//! comment, empty lines are skipped.

use crate::hook::Addr;

/// One line of an HLE file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub addr: Addr,
    pub name: String,
}

/// One checked entry: `va` is the VA of the first instruction of the function that the host implementation `name` may run instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub va: u64,
    pub name: String,
}

fn hex(s: &str) -> Result<u64, String> {
    let h = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    u64::from_str_radix(h, 16).map_err(|e| format!("bad hex number {s:?}: {e}"))
}

fn valid_name(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Parses an HLE file (see the module comment).
pub fn parse_file(text: &str) -> Result<Vec<Spec>, String> {
    let mut v = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let err = |e: String| format!("line {}: {e}", n + 1);
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 2 {
            return Err(err(format!("expected `ADDR NAME`, got {} fields: {line:?}", f.len())));
        }
        let addr = match f[0].strip_prefix("rva:") {
            Some(r) => Addr::Rva(hex(r.trim()).map_err(&err)?),
            None => Addr::Va(hex(f[0]).map_err(&err)?),
        };
        if !valid_name(f[1]) {
            return Err(err(format!("{:?} is not a name (letters, digits and `_`)", f[1])));
        }
        v.push(Spec { addr, name: f[1].to_string() });
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
            name: s.name.clone(),
        })
        .collect()
}

/// Checks the list: a function has one host implementation. (That each VA is a function entry of the closure is checked by the generator.)
pub fn check_unique(entries: &[Entry]) -> Result<(), String> {
    for (k, a) in entries.iter().enumerate() {
        if entries[..k].iter().any(|b| b.va == a.va) {
            return Err(format!("HLE entry {:#x} is listed twice", a.va));
        }
    }
    Ok(())
}

/// The guard in front of the first instruction of the function (inside the block of that instruction, so a jump back to the entry runs it too).
pub fn guard(e: &Entry) -> String {
    format!("{{ extern int fle_hle_{n}(void); if (fle_hle_{n}()) return; }} ", n = e.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_has_va_rva_comments_and_blank_lines() {
        let t = "# HLE\n\nrva:0x2a73a0 blend_row   # a\n0x6a73a0 blend_row2\n6a73a0\tx_9\n";
        let v = parse_file(t).unwrap();
        assert_eq!(
            v,
            [
                Spec { addr: Addr::Rva(0x2a73a0), name: "blend_row".into() },
                Spec { addr: Addr::Va(0x6a73a0), name: "blend_row2".into() },
                Spec { addr: Addr::Va(0x6a73a0), name: "x_9".into() },
            ]
        );
        assert_eq!(parse_file("").unwrap(), []);
        assert_eq!(parse_file("# only\n\n").unwrap(), []);
    }

    #[test]
    fn file_errors_name_the_line() {
        for bad in ["0x10 a b\n", "0x10\n", "0x10 a-b\n", "zz a\n", "rva: a\n", "0x10 a;b\n"] {
            let e = parse_file(&format!("0x10 a\n{bad}")).unwrap_err();
            assert!(e.starts_with("line 2: "), "{bad:?}: {e}");
        }
    }

    #[test]
    fn rva_gets_the_image_base_and_duplicates_are_refused() {
        let s = parse_file("rva:0x2a73a0 blend\n0x1000 other\n").unwrap();
        let a = resolve(&s, 0x400000);
        assert_eq!(a[0].va, 0x6a73a0);
        assert_eq!(a[1].va, 0x1000);
        assert!(check_unique(&a).is_ok());
        let d = resolve(&parse_file("0x10 a\n0x10 b\n").unwrap(), 0);
        assert!(check_unique(&d).unwrap_err().contains("listed twice"));
    }

    #[test]
    fn guard_text() {
        let e = Entry { va: 0x6a73a0, name: "blend_row".into() };
        assert_eq!(guard(&e), "{ extern int fle_hle_blend_row(void); if (fle_hle_blend_row()) return; } ");
    }
}
