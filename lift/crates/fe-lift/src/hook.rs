//! Inline hook points (decision 0105).
//!
//! The guest of a lifted module may hook one of its own functions by overwriting the first 5 bytes of the function with `jmp rel32`
//! (FLEngine unit 314 and QuickFontCache unit 162 do: a Delphi unit that patches three string functions of its own image). The lifted C
//! never reads an instruction from memory, so a call `f_X()` keeps running the original body whatever the image holds at X, and the hook
//! would be silently off (decision 0046, rule 4). A hook point is a function entry that is named in advance (`--hook-points FILE`):
//!
//! - the entry gets a guard in front of its first instruction: if the first 5 bytes of the image at X are not the ones recorded at lift
//!   time and are `e9 rel32`, the call is a tail dispatch to X + 5 + rel32 (what the CPU does: the stack and the registers are those of
//!   the entry); if they are the recorded ones the body runs; anything else stops by name (`fle_hook_target`, runtime/fle_vmap.c);
//! - the module registers its table of hook points (`fle_hook_register`) from a constructor at the end of the generated unit, and the
//!   runtime's VirtualProtect answers for exactly these 5-byte ranges of the image and stops for any other address of the image.
//!
//! This file holds the parts that do not depend on the other crates: the file format, the checks of one point, and the C text.

/// The size of `jmp rel32`: the opcode `e9` and a 32-bit displacement.
pub const HOOK_LEN: usize = 5;

/// IMAGE_SCN_MEM_EXECUTE, IMAGE_SCN_MEM_READ, IMAGE_SCN_MEM_WRITE (Microsoft Learn, "PE Format", Section Flags).
const SCN_EXECUTE: u32 = 0x2000_0000;
const SCN_READ: u32 = 0x4000_0000;
const SCN_WRITE: u32 = 0x8000_0000;

/// PAGE_EXECUTE_READ (Microsoft Learn, "Memory Protection Constants"). That a read + execute section is mapped with this protection is an
/// inference: the pages read do not state the mapping of section flags to page protection.
pub const PAGE_EXECUTE_READ: u32 = 0x20;

/// One checked hook point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookPoint {
    /// VA of the function entry.
    pub va: u64,
    /// The first 5 bytes of the image at `va`, as read from the PE: byte k in bits 8k..8k+7.
    pub orig5: u64,
    /// The protection the page has when the module is loaded (from the characteristics of the section).
    pub prot: u32,
}

/// One address of a hook-points file: a VA of the image as lifted, or an RVA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Addr {
    Va(u64),
    Rva(u64),
}

/// Parses a hook-points file: one address per line in hex, with or without `0x` (a VA of the image as lifted), or `rva:` and a hex number
/// (relative to the ImageBase of the image, so that the file does not depend on the base a module is measured at). `#` starts a comment.
/// Empty lines are skipped.
pub fn parse_file(text: &str) -> Result<Vec<Addr>, String> {
    let mut v = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (rva, num) = match line.strip_prefix("rva:") {
            Some(r) => (true, r.trim()),
            None => (false, line),
        };
        let hex = num.strip_prefix("0x").or_else(|| num.strip_prefix("0X")).unwrap_or(num);
        let x = u64::from_str_radix(hex, 16).map_err(|e| format!("line {}: bad address {line:?}: {e}", n + 1))?;
        v.push(if rva { Addr::Rva(x) } else { Addr::Va(x) });
    }
    Ok(v)
}

/// The VAs of `addrs` for an image with this ImageBase.
pub fn to_vas(addrs: &[Addr], image_base: u64) -> Vec<u64> {
    addrs
        .iter()
        .map(|a| match *a {
            Addr::Va(v) => v,
            Addr::Rva(r) => image_base.wrapping_add(r),
        })
        .collect()
}

/// Checks one hook point. `code` is the file-backed code at `va` (at least 5 bytes, or None), `is_entry` says that `va` is a function
/// entry of the lifted closure, `section` is the characteristics of the section that holds `va` (None: no section).
/// A mistake in the annotation is an error and the generator stops: nothing is skipped.
pub fn check_point(va: u64, code: Option<&[u8]>, is_entry: bool, section: Option<u32>) -> Result<HookPoint, String> {
    if !is_entry {
        return Err(format!(
            "hook point {va:#x} is not a function entry of the lifted closure (lift with --pdata, or seed it with --seeds)"
        ));
    }
    let Some(chars) = section else {
        return Err(format!("hook point {va:#x} is in no section of the image"));
    };
    if (chars & (SCN_EXECUTE | SCN_READ | SCN_WRITE)) != (SCN_EXECUTE | SCN_READ) {
        return Err(format!(
            "hook point {va:#x}: its section has characteristics {chars:#x}; only a read + execute section (PAGE_EXECUTE_READ) is modelled"
        ));
    }
    let Some(code) = code.filter(|c| c.len() >= HOOK_LEN) else {
        return Err(format!("hook point {va:#x}: the first {HOOK_LEN} bytes are not file-backed code"));
    };
    if code[0] == 0xe9 {
        return Err(format!(
            "hook point {va:#x} already starts with e9 (jmp rel32) in the file: a mistake in the annotation, or a function that is its own hook"
        ));
    }
    let mut orig5 = 0u64;
    for (k, b) in code[..HOOK_LEN].iter().enumerate() {
        orig5 |= (*b as u64) << (8 * k);
    }
    Ok(HookPoint { va, orig5, prot: PAGE_EXECUTE_READ })
}

/// Sorts the points, drops a repeated VA, and refuses two points that are less than 5 bytes apart (their `jmp rel32` would overlap).
pub fn finish(mut v: Vec<HookPoint>) -> Result<Vec<HookPoint>, String> {
    v.sort_by_key(|p| p.va);
    v.dedup_by_key(|p| p.va);
    for w in v.windows(2) {
        if w[1].va - w[0].va < HOOK_LEN as u64 {
            return Err(format!("hook points {:#x} and {:#x} are less than {HOOK_LEN} bytes apart", w[0].va, w[1].va));
        }
    }
    Ok(v)
}

/// The line that makes the generated unit know the hook types and functions. Only a unit with hook points has it.
pub const INCLUDE: &str = "#include \"fle_vmap.h\"\n";

/// The guard in front of the first instruction of the hook point (it goes inside the block of that instruction, so a jump back to the
/// entry runs it too). `fle_hook_target` returns 0 for the original bytes, the target of the `jmp rel32` that the guest wrote, or stops.
pub fn guard(p: &HookPoint) -> String {
    format!(
        "{{ uint64_t fle_hk = fle_hook_target(UINT64_C({:#x}), UINT64_C({:#x})); if (fle_hk) {{ fle_dispatch(fle_hk); return; }} }} ",
        p.va, p.orig5
    )
}

/// The table of the hook points of the module and the constructor that registers it. It follows the last function, so the tools that split
/// a generated file at function boundaries (harness/wasm/split_c.py, realmod_s1/tools/split_mod.py) keep it in the last part. The names
/// carry the base of the image, so the units of several modules do not clash.
pub fn tail(base: u64, size: u64, pts: &[HookPoint]) -> String {
    let mut s = format!("/* inline hook points (decision 0105) */\nstatic const fle_hook_point fle_hook_points_{base:x}[] = {{\n");
    for p in pts {
        s.push_str(&format!("    {{ UINT64_C({:#x}), UINT64_C({:#x}), {:#x} }},\n", p.va, p.orig5, p.prot));
    }
    s.push_str(&format!(
        "}};\nstatic void __attribute__((constructor)) fle_hook_init_{base:x}(void)\n{{\n    fle_hook_register(UINT64_C({base:#x}), UINT64_C({size:#x}), fle_hook_points_{base:x}, {});\n}}\n",
        pts.len()
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const RX: u32 = 0x6000_0020;

    #[test]
    fn file_has_vas_rvas_comments_and_blank_lines() {
        let t = "# FLEngine\n\n0x41c820  # VA\n43b130\nrva:0x3b150\nRVA:1\n";
        // "RVA:1" is not the prefix and is not hex either
        assert!(parse_file(t).is_err());
        let t = "# FLEngine\n\n0x41c820  # VA\n43b130\nrva:0x3b150\n  rva: 10 \n";
        assert_eq!(parse_file(t).unwrap(), [Addr::Va(0x41c820), Addr::Va(0x43b130), Addr::Rva(0x3b150), Addr::Rva(0x10)]);
        assert_eq!(parse_file("").unwrap(), []);
        assert_eq!(parse_file("# only\n").unwrap(), []);
    }

    #[test]
    fn file_errors_name_the_line() {
        for bad in ["0xzz\n", "rva:\n", "0x\n", "1 2\n", "rva:0x1g\n"] {
            let e = parse_file(&format!("0x10\n{bad}")).unwrap_err();
            assert!(e.starts_with("line 2: "), "{bad:?}: {e}");
        }
    }

    #[test]
    fn rva_gets_the_image_base() {
        let a = [Addr::Va(0x41c820), Addr::Rva(0x3b150)];
        assert_eq!(to_vas(&a, 0x400000), [0x41c820, 0x43b150]);
        assert_eq!(to_vas(&a, 0x1d00000), [0x41c820, 0x1d3b150]);
    }

    #[test]
    fn point_records_the_five_bytes_little_endian() {
        let code = [0x55, 0x56, 0x53, 0xc6, 0x44, 0x24, 0xe0];
        let p = check_point(0x41c820, Some(&code), true, Some(RX)).unwrap();
        assert_eq!(p, HookPoint { va: 0x41c820, orig5: 0x44c6_5356_55, prot: 0x20 });
    }

    #[test]
    fn point_mistakes_are_errors() {
        let code = [0x55, 0x56, 0x53, 0xc6, 0x44];
        assert!(check_point(1, Some(&code), false, Some(RX)).unwrap_err().contains("not a function entry"));
        assert!(check_point(1, Some(&code), true, None).unwrap_err().contains("no section"));
        // writable, not executable, not readable
        for c in [0xe000_0020u32, 0x6000_0020 | 0x8000_0000, 0x4000_0040, 0x2000_0020] {
            assert!(check_point(1, Some(&code), true, Some(c)).unwrap_err().contains("read + execute"), "{c:#x}");
        }
        assert!(check_point(1, None, true, Some(RX)).unwrap_err().contains("not file-backed"));
        assert!(check_point(1, Some(&code[..4]), true, Some(RX)).unwrap_err().contains("not file-backed"));
        let e = check_point(1, Some(&[0xe9, 0, 0, 0, 0]), true, Some(RX)).unwrap_err();
        assert!(e.contains("already starts with e9"), "{e}");
    }

    #[test]
    fn finish_sorts_dedups_and_refuses_overlap() {
        let p = |va| HookPoint { va, orig5: 1, prot: 0x20 };
        let v = finish(vec![p(0x30), p(0x10), p(0x30), p(0x20)]).unwrap();
        assert_eq!(v.iter().map(|p| p.va).collect::<Vec<_>>(), [0x10, 0x20, 0x30]);
        assert!(finish(vec![p(0x10), p(0x14)]).unwrap_err().contains("less than 5 bytes apart"));
        assert!(finish(vec![p(0x10), p(0x15)]).is_ok());
        assert_eq!(finish(vec![]).unwrap(), []);
    }

    #[test]
    fn guard_and_tail_text() {
        let p = HookPoint { va: 0x41c820, orig5: 0x44c6_5356_55, prot: 0x20 };
        assert_eq!(
            guard(&p),
            "{ uint64_t fle_hk = fle_hook_target(UINT64_C(0x41c820), UINT64_C(0x44c6535655)); if (fle_hk) { fle_dispatch(fle_hk); return; } } "
        );
        let q = HookPoint { va: 0x43b130, orig5: 0x20ec_8348_53, prot: 0x20 };
        let t = tail(0x400000, 0x18c0000, &[p, q]);
        assert_eq!(
            t,
            "/* inline hook points (decision 0105) */\n\
             static const fle_hook_point fle_hook_points_400000[] = {\n\
             \x20   { UINT64_C(0x41c820), UINT64_C(0x44c6535655), 0x20 },\n\
             \x20   { UINT64_C(0x43b130), UINT64_C(0x20ec834853), 0x20 },\n\
             };\n\
             static void __attribute__((constructor)) fle_hook_init_400000(void)\n\
             {\n\
             \x20   fle_hook_register(UINT64_C(0x400000), UINT64_C(0x18c0000), fle_hook_points_400000, 2);\n\
             }\n"
        );
    }
}
