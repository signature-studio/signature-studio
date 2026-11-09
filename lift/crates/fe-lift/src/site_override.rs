//! Call-site memory overrides (decision 0120).
//!
//! A module of the guest may ask its environment for something that this environment does not have; the guest then fails far from the
//! cause (Blend2D's `Agg2D::attach` creates its drawing context with an all-zero `BLContextCreateInfo`, which makes Blend2D generate machine
//! code at the first drawing call; a browser cannot run it). An override is an annotation of one instruction of the lifted module: just
//! before that instruction runs, a 32-bit word in memory that a register points to is changed from the value the guest wrote (`expect`) to
//! another value (`new`). The generated C gets one guard in front of that instruction and nothing else changes; the module's other
//! functions are byte for byte what they were.
//!
//! Nothing is silently turned into something else: the guard (`fle_site_store32`, runtime/fle_vmap.c) stores `new` when the word is
//! `expect`, leaves it when it already is `new`, and stops by name for any other value.
//!
//! This file holds the parts that do not depend on the other crates: the file format, the checks of one site, and the C text. The file is
//! read by `fe-lift lift ... --site-overrides FILE`.

use crate::hook::Addr;

/// The register names, in the order of the `R[]` array of the generated C (the order of the x86-64 register numbers).
pub const REGS: [&str; 16] = [
    "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
];

/// One line of an overrides file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub addr: Addr,
    /// Index into `R[]`.
    pub reg: usize,
    /// Signed displacement added to the register.
    pub disp: i64,
    pub expect: u32,
    pub new: u32,
}

/// One checked site: `addr` is the VA of the instruction before which the override happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub va: u64,
    pub reg: usize,
    pub disp: i64,
    pub expect: u32,
    pub new: u32,
}

fn hex(s: &str) -> Result<u64, String> {
    let h = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    u64::from_str_radix(h, 16).map_err(|e| format!("bad hex number {s:?}: {e}"))
}

fn hex32(s: &str) -> Result<u32, String> {
    let v = hex(s)?;
    u32::try_from(v).map_err(|_| format!("{s:?} does not fit in 32 bits"))
}

/// `r8`, `r8+0x10`, `rsp-0x8`: a register of `REGS` and an optional signed hex displacement.
fn parse_operand(s: &str) -> Result<(usize, i64), String> {
    let (name, disp) = match s.find(['+', '-']) {
        None => (s, 0i64),
        Some(k) => {
            let d = i64::try_from(hex(&s[k + 1..])?).map_err(|_| format!("displacement of {s:?} is too large"))?;
            (&s[..k], if s.as_bytes()[k] == b'-' { -d } else { d })
        }
    };
    let reg = REGS.iter().position(|r| *r == name).ok_or_else(|| format!("{name:?} is not a 64-bit register ({})", REGS.join(" ")))?;
    Ok((reg, disp))
}

/// Parses an overrides file. One override per line: `ADDR store32 REG[+DISP|-DISP] EXPECT NEW`, fields separated by white space.
/// `ADDR` is a hex VA of the image as lifted (with or without `0x`) or `rva:` and a hex RVA (the ImageBase is added), as in a hook-points
/// file; `EXPECT` and `NEW` are 32-bit hex numbers. `#` starts a comment, empty lines are skipped. Only `store32` exists.
pub fn parse_file(text: &str) -> Result<Vec<Spec>, String> {
    let mut v = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let err = |e: String| format!("line {}: {e}", n + 1);
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 5 {
            return Err(err(format!("expected `ADDR store32 REG[+DISP] EXPECT NEW`, got {} fields: {line:?}", f.len())));
        }
        let addr = match f[0].strip_prefix("rva:") {
            Some(r) => Addr::Rva(hex(r.trim()).map_err(&err)?),
            None => Addr::Va(hex(f[0]).map_err(&err)?),
        };
        if f[1] != "store32" {
            return Err(err(format!("unknown operation {:?} (only store32)", f[1])));
        }
        let (reg, disp) = parse_operand(f[2]).map_err(&err)?;
        let expect = hex32(f[3]).map_err(&err)?;
        let new = hex32(f[4]).map_err(&err)?;
        if expect == new {
            return Err(err(format!("EXPECT and NEW are both {expect:#x}: the override would change nothing")));
        }
        v.push(Spec { addr, reg, disp, expect, new });
    }
    Ok(v)
}

/// The sites of `specs` for an image with this ImageBase, in file order.
pub fn resolve(specs: &[Spec], image_base: u64) -> Vec<Site> {
    specs
        .iter()
        .map(|s| Site {
            va: match s.addr {
                Addr::Va(v) => v,
                Addr::Rva(r) => image_base.wrapping_add(r),
            },
            reg: s.reg,
            disp: s.disp,
            expect: s.expect,
            new: s.new,
        })
        .collect()
}

/// Checks the list of sites: the same override twice is a mistake of the file. (That each VA is an instruction of the closure is
/// checked by the generator, which has the decoded instructions.)
pub fn check_unique(sites: &[Site]) -> Result<(), String> {
    for (k, a) in sites.iter().enumerate() {
        if sites[..k].contains(a) {
            return Err(format!("site override at {:#x} is listed twice", a.va));
        }
    }
    Ok(())
}

/// The guard in front of the instruction at the site (it goes inside the block of that instruction, so a jump to the instruction runs it
/// too). `fle_site_store32(va, address, expect, new)` is in runtime/fle_vmap.c.
pub fn guard(s: &Site) -> String {
    format!(
        "{{ fle_site_store32(UINT64_C({:#x}), R[{}] + UINT64_C({:#x}), UINT64_C({:#x}), UINT64_C({:#x})); }} ",
        s.va, s.reg, s.disp as u64, s.expect, s.new
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_has_va_rva_disp_comments_and_blank_lines() {
        let t = "# Blend2D\n\nrva:0x56ce9 store32 r8 0 1   # createInfo.flags\n0x4756ce9 store32 rsp+0x60 0x0 0x1\n4756ce9\tstore32\tr9-0x8\t0x10\t0xffffffff\n";
        let v = parse_file(t).unwrap();
        assert_eq!(
            v,
            [
                Spec { addr: Addr::Rva(0x56ce9), reg: 8, disp: 0, expect: 0, new: 1 },
                Spec { addr: Addr::Va(0x4756ce9), reg: 4, disp: 0x60, expect: 0, new: 1 },
                Spec { addr: Addr::Va(0x4756ce9), reg: 9, disp: -8, expect: 0x10, new: 0xffff_ffff },
            ]
        );
        assert_eq!(parse_file("").unwrap(), []);
        assert_eq!(parse_file("# only\n\n").unwrap(), []);
    }

    #[test]
    fn registers_are_in_the_order_of_r() {
        for (k, r) in REGS.iter().enumerate() {
            assert_eq!(parse_operand(r).unwrap(), (k, 0));
        }
        assert_eq!(parse_operand("rcx").unwrap().0, 1);
        assert_eq!(parse_operand("rdx").unwrap().0, 2);
        assert_eq!(parse_operand("rbx").unwrap().0, 3);
        assert_eq!(parse_operand("rsp").unwrap().0, 4);
        assert_eq!(parse_operand("rdi").unwrap().0, 7);
        assert_eq!(parse_operand("r8").unwrap().0, 8);
        assert_eq!(parse_operand("r15").unwrap().0, 15);
    }

    #[test]
    fn file_errors_name_the_line() {
        for bad in [
            "0x10 store32 r8 0 1 extra\n",
            "0x10 store32 r8 0\n",
            "0x10 store64 r8 0 1\n",
            "0x10 store32 eax 0 1\n",
            "0x10 store32 r16 0 1\n",
            "0x10 store32 r8+ 0 1\n",
            "0x10 store32 r8 0x1ffffffff 1\n",
            "0x10 store32 r8 zz 1\n",
            "0x10 store32 r8 1 1\n",
            "rva: store32 r8 0 1\n",
            "zz store32 r8 0 1\n",
        ] {
            let e = parse_file(&format!("0x10 store32 r8 0 1\n{bad}")).unwrap_err();
            assert!(e.starts_with("line 2: "), "{bad:?}: {e}");
        }
    }

    #[test]
    fn rva_gets_the_image_base() {
        let s = parse_file("rva:0x56ce9 store32 r8 0 1\n0x1000 store32 rcx+8 0 2\n").unwrap();
        let a = resolve(&s, 0x4700000);
        assert_eq!(a[0].va, 0x4756ce9);
        assert_eq!(a[1].va, 0x1000);
        let b = resolve(&s, 0x180000000);
        assert_eq!(b[0].va, 0x180056ce9);
    }

    #[test]
    fn a_site_listed_twice_is_a_mistake() {
        let s = |new| Site { va: 0x10, reg: 8, disp: 0, expect: 0, new };
        assert!(check_unique(&[s(1), s(2)]).is_ok());
        assert!(check_unique(&[s(1), s(2), s(1)]).unwrap_err().contains("listed twice"));
        assert!(check_unique(&[]).is_ok());
    }

    #[test]
    fn guard_text() {
        let s = Site { va: 0x4756ce9, reg: 8, disp: 0, expect: 0, new: 1 };
        assert_eq!(
            guard(&s),
            "{ fle_site_store32(UINT64_C(0x4756ce9), R[8] + UINT64_C(0x0), UINT64_C(0x0), UINT64_C(0x1)); } "
        );
        let n = Site { va: 0x10, reg: 4, disp: -8, expect: 0x10, new: 0xffff_ffff };
        assert_eq!(
            guard(&n),
            "{ fle_site_store32(UINT64_C(0x10), R[4] + UINT64_C(0xfffffffffffffff8), UINT64_C(0x10), UINT64_C(0xffffffff)); } "
        );
    }
}
