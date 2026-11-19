//! Swapped entries (decision 0160, provisional): a guest function whose body the host implements instead.
//!
//! An entry names the first instruction of one function of the module and the name of a host implementation. The function is a function
//! entry of the closure (it is in the dispatch table, a call or a tail call reaches it as `f_<va>()`), but its body is not decoded, nothing it
//! calls is followed, and the generated C holds a stub in place of the body:
//!
//! ```c
//! void f_72b50(void)
//! {
//!     extern void fle_swap_path_reset(void);
//!     fle_swap_path_reset();
//! }
//! ```
//!
//! `fle_swap_<name>()` (host code, outside the generated C) is called with the guest state at the entry of the function (arguments in
//! R[1], R[2], R[8], R[9], X[0..3] and the stack, return address at [R[4]]) and must leave what the guest function leaves for its caller: the
//! result in R[0] / X[0], the memory the function writes, and the callee-saved registers and R[4] as they were. A missing host function is a
//! link error. Without `--swap-entries` the generator's output is byte for byte what it was.
//!
//! The file read by `fe-lift lift ... --swap-entries FILE`: one entry per line, `ADDR NAME`, `ADDR` as in a hook-points file (a hex VA of the
//! image as lifted, with or without `0x`, or `rva:` and a hex RVA), `NAME` a C identifier suffix (letters, digits and `_`). `#` starts a
//! comment, empty lines are skipped.

use crate::hook::Addr;

/// One line of a swap file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub addr: Addr,
    pub name: String,
}

/// One resolved entry: `va` is the VA of the first instruction of the function that the host implementation `name` runs instead.
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

/// Parses a swap file (see the module comment).
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

/// Checks the list: a function has one host implementation.
pub fn check_unique(entries: &[Entry]) -> Result<(), String> {
    for (k, a) in entries.iter().enumerate() {
        if entries[..k].iter().any(|b| b.va == a.va) {
            return Err(format!("swap entry {:#x} is listed twice", a.va));
        }
    }
    Ok(())
}

/// The C text of the stub that stands in for the function at `e.va` (name, body, and a blank line, as the other functions end).
pub fn stub(e: &Entry) -> String {
    format!(
        "void f_{va:x}(void)\n{{\n    extern void fle_swap_{n}(void);\n    fle_swap_{n}();\n}}\n\n",
        va = e.va,
        n = e.name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_has_va_rva_comments_and_blank_lines() {
        let t = "# swap\n\nrva:0x72b50 path_reset   # a\n0x4772b50 path_reset2\n4772b50\tx_9\n";
        let v = parse_file(t).unwrap();
        assert_eq!(
            v,
            [
                Spec { addr: Addr::Rva(0x72b50), name: "path_reset".into() },
                Spec { addr: Addr::Va(0x4772b50), name: "path_reset2".into() },
                Spec { addr: Addr::Va(0x4772b50), name: "x_9".into() },
            ]
        );
        assert_eq!(parse_file("").unwrap(), []);
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
        let s = parse_file("rva:0x72b50 a\n0x1000 b\n").unwrap();
        let a = resolve(&s, 0x4700000);
        assert_eq!(a[0].va, 0x4772b50);
        assert_eq!(a[1].va, 0x1000);
        assert!(check_unique(&a).is_ok());
        let d = resolve(&parse_file("0x10 a\n0x10 b\n").unwrap(), 0);
        assert!(check_unique(&d).unwrap_err().contains("listed twice"));
    }

    #[test]
    fn stub_text() {
        let e = Entry { va: 0x4772b50, name: "path_reset".into() };
        assert_eq!(stub(&e), "void f_4772b50(void)\n{\n    extern void fle_swap_path_reset(void);\n    fle_swap_path_reset();\n}\n\n");
    }
}
