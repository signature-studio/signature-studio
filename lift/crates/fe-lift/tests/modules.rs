//! Tests for the virtual modules (docs/decisions/0015-virtual-modules.md): kernel32!GetModuleHandleW and
//! GetProcAddress in runtime/fle_modules.c, and the response table that the generator emits.
//!
//! 1. Generator: both thunks call the runtime; `fle_resp_table` and the pseudo-VA entries of
//!    `fle_dispatch_table` are emitted only when GetProcAddress is in the closure; the VAs are sorted and unique.
//! 2. Runtime, GetModuleHandleW: case, extension rules, NULL, unknown module (126), distinct handles, the 24
//!    DLLs of the PE import table (read from the real DLL when it is present).
//! 3. Runtime, GetProcAddress: found, not found (NULL and 127), exact case, module match, a call through
//!    `fle_dispatch`, and the run without a table.
//! 4. Cases the MSDN pages leave open abort (ordinal, NULL name, path, bad handle, the lifted DLL's own handle).
//!
//! The host is Linux; the Windows loader cannot run here, so nothing is compared with a real Windows.

mod etxt_common;

use fe_lift::generate;
use std::path::PathBuf;
use std::process::Command;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const DATA_RVA: u32 = 0x2000;
const DATA_RAW: usize = 0x800;
const FILE_LEN: usize = 0x1000;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn data_at(rva: u32) -> usize {
    DATA_RAW + (rva - DATA_RVA) as usize
}
fn put(b: &mut [u8], rva: u32, bytes: &[u8]) {
    let o = data_at(rva);
    b[o..o + bytes.len()].copy_from_slice(bytes);
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

/// A PE with kernel32.dll imports `names` (at most 3); entry 0x1000 calls the thunks at 0x1040, 0x1050, 0x1060.
fn build(names: &[&str]) -> Vec<u8> {
    assert!(names.len() <= 3);
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 120, DATA_RVA);
    w32(&mut b, o + 124, 40);

    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let s1 = SEC + 40;
    b[s1..s1 + 5].copy_from_slice(b".data");
    w32(&mut b, s1 + 8, 0x800);
    w32(&mut b, s1 + 12, DATA_RVA);
    w32(&mut b, s1 + 16, 0x800);
    w32(&mut b, s1 + 20, DATA_RAW as u32);
    w32(&mut b, s1 + 36, 0xc000_0040);

    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, 0x2200);
    put(&mut b, 0x2300, b"kernel32.dll\0");
    for (k, n) in names.iter().enumerate() {
        let name_rva = 0x2400 + 0x40 * k as u32;
        let mut hn = vec![0u8, 0u8];
        hn.extend_from_slice(n.as_bytes());
        hn.push(0);
        put(&mut b, name_rva, &hn);
        w64(&mut b, data_at(0x2100 + 8 * k as u32), name_rva as u64);
        w64(&mut b, data_at(0x2200 + 8 * k as u32), name_rva as u64);
    }

    let t = TEXT_RAW;
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    for k in 0..names.len() {
        let at = 0x1000 + 5 * k as u32;
        code.push((at, [&[0xe8][..], &rel(at + 5, 0x1040 + 0x10 * k as u32)].concat()));
        code.push((0x1040 + 0x10 * k as u32, [&[0xff, 0x25][..], &rel(0x1046 + 0x10 * k as u32, 0x2200 + 8 * k as u32)].concat()));
    }
    code.push((0x1000 + 5 * names.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn dirs() -> (PathBuf, PathBuf) {
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-modules/test");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    (rt, dir)
}

/// Every runtime source in runtime/ (fle_*.c), sorted. The response table can name any
/// `fle_imp_*` function, so a unit that links the table must link all of them; listing them by
/// hand breaks each time a new runtime file is added.
fn runtime_sources(rt: &std::path::Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(rt)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with("fle_") && n.ends_with(".c")
        })
        .collect();
    v.sort();
    v
}

fn clang(name: &str, src: &str, extra_c: &[&str]) -> PathBuf {
    let (rt, dir) = dirs();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-I"]).arg(&rt);
    for f in ["fle_modules.c", "fle_imports.c", "fle_string.c", "fle_sys.c"].iter().chain(extra_c) {
        cmd.arg(rt.join(f));
    }

    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn gen(names: &[&str]) -> (fe_lift::Stats, String) {
    let img = build(names);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

// ---- 1. Generator ----

#[test]
fn thunks_call_runtime_and_tables_are_emitted() {
    let (st, c) = gen(&["GetModuleHandleW", "GetProcAddress"]);
    for name in ["GetModuleHandleW", "GetProcAddress"] {
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{name}");
        assert!(c.contains(&format!("fle_imp_{name}(); return;")), "{name}");
    }
    // The row count is whatever the table holds; other tests must not pin it, because
    // every new import response adds a row. It must cover at least the 2 rows asserted here.
    let n = st.resp_entries;
    assert!(n >= 13, "{n}");
    assert!(c.contains(&format!("const uint64_t fle_resp_count = {n};")));
    // Row k has VA 0x7ff900000000 + 0x10 * (k + 1); GetModuleHandleW is row 11, GetProcAddress row 12.
    assert!(c.contains("{ \"kernel32.dll\", \"GetModuleHandleW\", UINT64_C(0x7ff9000000c0), fle_imp_GetModuleHandleW }"));
    assert!(c.contains("{ \"kernel32.dll\", \"GetProcAddress\", UINT64_C(0x7ff9000000d0), fle_imp_GetProcAddress }"));
    assert!(c.contains("{ UINT64_C(0x7ff9000000d0), fle_imp_GetProcAddress }"));
    // The dispatch count is the function entries plus one pseudo-VA per response row.
    let funcs = c.matches("\nvoid f_").count();
    assert!(funcs >= 1);
    assert!(
        c.contains(&format!("const uint64_t fle_dispatch_count = {};", st.functions + n)),
        "{}",
        st.functions
    );
}

#[test]
fn dispatch_table_stays_sorted_and_pseudo_vas_are_unique() {
    let (st, c) = gen(&["GetProcAddress"]);
    let start = c.find("const fle_fn_entry fle_dispatch_table[] = {").unwrap();
    let end = c[start..].find("};").unwrap() + start;
    let vas: Vec<u64> = c[start..end]
        .lines()
        .filter_map(|l| l.split("UINT64_C(").nth(1))
        .map(|r| u64::from_str_radix(r.split(')').next().unwrap().trim_start_matches("0x"), 16).unwrap())
        .collect();
    assert!(vas.len() >= 14);
    assert!(vas.windows(2).all(|w| w[0] < w[1]), "dispatch table not strictly ascending");
    let rstart = c.find("const fle_resp_entry fle_resp_table[] = {").unwrap();
    let rend = c[rstart..].find("};").unwrap() + rstart;
    let rvas: Vec<u64> = c[rstart..rend]
        .lines()
        .filter_map(|l| l.split("UINT64_C(").nth(1))
        .map(|r| u64::from_str_radix(r.split(')').next().unwrap().trim_start_matches("0x"), 16).unwrap())
        .collect();
    assert_eq!(rvas.len(), st.resp_entries);
    // Every response VA is a dispatch entry; the VAs are above the module handles (0x7FF800000000 + 0x10000 * 34).
    for v in &rvas {
        assert!(vas.contains(v));
        assert!(*v > 0x7FF8_0000_0000 + 0x10000 * 34);
    }
}

#[test]
fn tables_absent_without_get_proc_address() {
    let (st, c) = gen(&["GetModuleHandleW"]);
    assert_eq!(st.handled["jmp_imp:GetModuleHandleW"], 1);
    assert_eq!(st.resp_entries, 0);
    assert!(!c.contains("fle_resp_table"));
    assert!(!c.contains("fle_modules.h"));
}

#[test]
fn generated_unit_with_tables_compiles_and_links_with_runtime() {
    let (_, c) = gen(&["GetModuleHandleW", "GetProcAddress"]);
    let (rt, dir) = dirs();
    let tag = etxt_common::uniq("gen-with-tables");
    let src = dir.join(format!("{tag}.c"));
    std::fs::write(&src, &c).unwrap();
    // A main that asks the generated table for GetLastError and calls the result through fle_dispatch.
    let main = "#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_dispatch.h\"\n#include \"fle_modules.h\"\n#include \"fle_imports.h\"\n\n\
int main(void)\n{\n\
    static const uint16_t k[] = { 'K','e','r','n','e','l','3','2','.','D','L','L', 0 };\n\
    static const char gle[] = \"GetLastError\";\n\
    static const char miss[] = \"GetLogicalProcessorInformationEx\";\n\
    fle_set_last_error(77);\n\
    R[1] = (uint64_t)(uintptr_t)k; fle_imp_GetModuleHandleW();\n\
    R[1] = R[0]; R[2] = (uint64_t)(uintptr_t)gle; fle_imp_GetProcAddress();\n\
    { uint64_t va = R[0]; if (va == 0) { printf(\"FAIL no GetLastError\\n\"); return 1; } fle_dispatch(va); }\n\
    printf(\"gle %llu\\n\", (unsigned long long)R[0]);\n\
    R[1] = fle_module_handle(8); R[2] = (uint64_t)(uintptr_t)miss; fle_imp_GetProcAddress();\n\
    printf(\"miss %llu\\n\", (unsigned long long)R[0]);\n\
    fle_imp_GetLastError(); printf(\"le %llu\\n\", (unsigned long long)R[0]);\n\
    return 0;\n}\n";
    let mainc = dir.join(format!("{tag}-main.c"));
    std::fs::write(&mainc, main).unwrap();
    let exe = dir.join(&tag);
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt);
    // main has no R of its own: the generated unit defines R, FL, X.
    for p in runtime_sources(&rt) {
        cmd.arg(p);
    }
    let cc = cmd
        .arg(&src)
        .arg(&mainc)
        .arg("-lm")
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    assert!(run.status.success(), "{out}\n{}", String::from_utf8_lossy(&run.stderr));
    // GetLastError, reached through GetModuleHandleW -> GetProcAddress -> fle_dispatch, returns the 77 set before;
    // a function the table does not have gives NULL and ERROR_PROC_NOT_FOUND.
    assert_eq!(out.lines().collect::<Vec<_>>(), ["gle 77", "miss 0", "le 127"]);
}

// ---- 2. GetModuleHandleW ----

fn c_wide(s: &str) -> String {
    let mut v: Vec<String> = s.encode_utf16().map(|u| format!("{u}")).collect();
    v.push("0".into());
    format!("(const uint16_t[]){{ {} }}", v.join(", "))
}

const PRE: &str = "#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_dispatch.h\"\n#include \"fle_modules.h\"\n#include \"fle_imports.h\"\n\nuint64_t R[16];\n";

/// Calls GetModuleHandleW with `name`, prints "<handle hex> <lasterror>" (last error set to 5 first).
fn gmh_line(name: Option<&str>) -> String {
    let arg = match name {
        None => "0".to_string(),
        Some(s) => format!("(uint64_t)(uintptr_t){}", c_wide(s)),
    };
    format!("    fle_set_last_error(5); R[1] = {arg}; R[0] = 99; fle_imp_GetModuleHandleW(); {{ uint64_t h = R[0]; fle_imp_GetLastError(); printf(\"%llx %llu\\n\", (unsigned long long)h, (unsigned long long)R[0]); }}\n")
}

fn handle_of(index: u64) -> u64 {
    0x7FF8_0000_0000 + 0x10000 * (index + 1)
}

/// (name, expected module index or None for "not found"): the rows of GetModuleHandleW and, since decision 0091, of GetModuleHandleA
/// (the A test skips the names that are not ASCII).
fn name_rows() -> Vec<(Option<&'static str>, Option<u64>)> {
    vec![
        (Some("kernel32.dll"), Some(8)),
        (Some("KERNEL32.DLL"), Some(8)),
        (Some("Kernel32.Dll"), Some(8)),
        (Some("kernel32"), Some(8)),
        (Some("KERNEL32"), Some(8)),
        (Some("kernel32."), None),
        (Some("user32"), Some(18)),
        (Some("USER32.dll"), Some(18)),
        (Some("ntdll.dll"), Some(24)),
        (Some("NTDLL"), Some(24)),
        (Some("winspool.drv"), Some(22)),
        (Some("WINSPOOL.DRV"), Some(22)),
        (Some("winspool"), None),
        (Some("api-ms-win-crt-string-l1-1-0.dll"), Some(1)),
        (Some("IMAGEHLP.DLL"), Some(7)),
        (Some("SHFolder"), Some(16)),
        (Some("nosuch.dll"), None),
        (Some("nosuch"), None),
        (Some(""), None),
        (Some("."), None),
        (Some("kernel32.dll.dll"), None),
        (Some("kernel3\u{0131}.dll"), None),
        (Some("dwmapi.dll"), Some(38)),    // decision 0122: dwmapi.dll is virtual module N = 39 (index 38); old: None (one of the lacking names)
        (Some("api-ms-win-core-libraryloader-l1-1-0.dll"), Some(39)), // decision 0238: a contract of the schema (host kernelbase.dll, N = 40); old: None (an API set name that was not a module of the table)
        // decision 0081: N = 27..30
        (Some("api-ms-win-core-synch-l1-2-0.dll"), Some(26)),
        (Some("API-MS-WIN-CORE-SYNCH-L1-2-0"), Some(26)),
        (Some("api-ms-win-core-fibers-l1-1-1"), Some(27)),
        (Some("api-ms-win-core-localization-l1-2-1.dll"), Some(28)),
        (Some("api-ms-win-appmodel-runtime-l1-1-2"), Some(29)),
        (Some("api-ms-win-core-synch-l1-2-1.dll"), Some(39)), // decision 0238: not a name of the table (only l1-2-0 is), so the schema redirects it to its host kernelbase.dll (N = 40, index 39); old: None
        // decision 0088: N = 31 (avrt.dll) and N = 32 (powrprof.dll); the guest spells them "avrt" and "PowrProf.dll"
        (Some("avrt"), Some(30)),
        (Some("AVRT.DLL"), Some(30)),
        (Some("Avrt.dll"), Some(30)),
        (Some("powrprof"), Some(31)),
        (Some("PowrProf.dll"), Some(31)),
        (Some("POWRPROF"), Some(31)),
        (Some("avrt2.dll"), None),
        (Some("avrt.dll.dll"), None),
        (Some("powrprof2"), None),
        // decision 0088 leaves the other ten names and the nine lacking names alone
        (Some("psapi"), None),
        (Some("ws2_32.dll"), None),
        (Some("dwmapi"), Some(38)),        // decision 0122 (old: None)
        (Some("DWMAPI.DLL"), Some(38)),
        // decision 0099: N = 35 (shcore.dll), after the deferred modules N = 33, 34 (decision 0096)
        (Some("shcore.dll"), Some(34)),
        (Some("SHCore"), Some(34)),
        (Some("Shcore.dll"), Some(34)),
        (Some("shcore2.dll"), None),
    ]
}

#[test]
fn get_module_handle_name_rules() {
    let rows = name_rows();
    let mut body = String::new();
    for (n, _) in &rows {
        body += &gmh_line(*n);
    }
    body += &gmh_line(None);
    let src = format!("{PRE}int main(void)\n{{\n{body}    return 0;\n}}\n");
    let exe = clang("gmh-rules", &src, &[]);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let got: Vec<&str> = out.lines().collect();
    assert_eq!(got.len(), rows.len() + 1);
    for (i, (name, want)) in rows.iter().enumerate() {
        // Success leaves the last error untouched (5, set before the call); failure sets 126.
        let expect = match want {
            Some(ix) => format!("{:x} 5", handle_of(*ix)),
            None => "0 126".to_string(),
        };
        assert_eq!(got[i], expect, "{name:?}");
    }
    // NULL: the handle of the lifted DLL (ImageBase 0x400000), last error untouched.
    assert_eq!(got[rows.len()], "400000 5");
}

// ---- 2a. GetModuleHandleA (decision 0091) ----

/// A C string literal with every byte that is not printable ASCII (and the quote and backslash) written as an octal escape.
fn c_ansi(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        if (0x20..0x7f).contains(&b) && b != b'"' && b != b'\\' && b != b'?' {
            s.push(b as char);
        } else {
            s += &format!("\\{:03o}", b);
        }
    }
    s.push('"');
    s
}

/// Like `gmh_line`, for GetModuleHandleA with the bytes of `name` (None: NULL).
fn gmh_a_line(name: Option<&[u8]>) -> String {
    let arg = match name {
        None => "0".to_string(),
        Some(b) => format!("(uint64_t)(uintptr_t){}", c_ansi(b)),
    };
    format!("    fle_set_last_error(5); R[1] = {arg}; R[0] = 99; fle_imp_GetModuleHandleA(); {{ uint64_t h = R[0]; fle_imp_GetLastError(); printf(\"%llx %llu\\n\", (unsigned long long)h, (unsigned long long)R[0]); }}\n")
}

/// Every row of the GetModuleHandleW name test whose name is ASCII is asked of both functions in one program: the answers are the same
/// (handle or NULL and 126, last error untouched on success) and are the expected ones. NULL is 0x400000 for both.
#[test]
fn get_module_handle_a_answers_like_w_on_every_ascii_row() {
    let rows: Vec<(Option<&str>, Option<u64>)> = name_rows().into_iter().filter(|(n, _)| n.map_or(true, |s| s.is_ascii())).collect();
    assert!(rows.len() >= 40, "{}", rows.len());
    let mut body = String::new();
    for (n, _) in &rows {
        body += &gmh_line(*n);
        body += &gmh_a_line(n.map(|s| s.as_bytes()));
    }
    body += &gmh_line(None);
    body += &gmh_a_line(None);
    let src = format!("{PRE}int main(void)\n{{\n{body}    return 0;\n}}\n");
    let exe = clang("gmha-rules", &src, &[]);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let got: Vec<&str> = out.lines().collect();
    assert_eq!(got.len(), 2 * (rows.len() + 1));
    for (i, (name, want)) in rows.iter().enumerate() {
        let expect = match want {
            Some(ix) => format!("{:x} 5", handle_of(*ix)),
            None => "0 126".to_string(),
        };
        assert_eq!(got[2 * i], expect, "W {name:?}");
        assert_eq!(got[2 * i + 1], expect, "A {name:?}");
    }
    assert_eq!(got[2 * rows.len()], "400000 5");
    assert_eq!(got[2 * rows.len() + 1], "400000 5");
}

/// The case, extension and trailing-point rules, on the names that the guest and Learn use, for A alone (not a copy of the W rows: the
/// expected handles are computed from the position in the table).
#[test]
fn get_module_handle_a_name_rules() {
    let rows: [(&[u8], Option<u64>); 14] = [
        (b"api-ms-win-core-synch-l1-2-0.dll", Some(26)), // the argument at 0x4899fed of Blend2D (docs/investigations/blend2d-getmodulehandlea.md)
        (b"API-MS-WIN-CORE-SYNCH-L1-2-0.DLL", Some(26)),
        (b"api-ms-win-core-synch-l1-2-0", Some(26)),
        (b"Kernel32", Some(8)),
        (b"KERNEL32.", None),
        (b"kernel32.dll.", Some(8)),
        (b"ntdll", Some(24)),
        (b"", None),
        (b".", None),
        (b"nosuch", None),
        (b"dwmapi", Some(38)),             // decision 0122 (old: None)
        (b"api-ms-win-core-synch-l1-2-1.dll", Some(39)), // decision 0238: the schema redirects the name that the table lacks to kernelbase.dll (N = 40); old: None
        (b"avrt", Some(30)),
        (b"PowrProf.dll", Some(31)),
    ];
    let mut body = String::new();
    for (n, _) in &rows {
        body += &gmh_a_line(Some(n));
    }
    let src = format!("{PRE}int main(void)\n{{\n{body}    return 0;\n}}\n");
    let exe = clang("gmha-names", &src, &[]);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let got: Vec<&str> = out.lines().collect();
    assert_eq!(got.len(), rows.len());
    for (i, (name, want)) in rows.iter().enumerate() {
        let expect = match want {
            Some(ix) => format!("{:x} 5", handle_of(*ix)),
            None => "0 126".to_string(),
        };
        assert_eq!(got[i], expect, "{:?}", String::from_utf8_lossy(name));
    }
}

/// A byte above 0x7F stops GetModuleHandleA by name (the page does not define the ANSI code page); the same name as a UTF-16 unit is "not
/// found" for GetModuleHandleW (126). The path, the 256-character limit and the stop happen before the high byte is looked at, as in W.
#[test]
fn get_module_handle_a_stops_where_the_page_is_silent_and_w_does_not_stop_on_a_high_unit() {
    use std::os::unix::process::ExitStatusExt;
    // (label, C expression of the lpModuleName pointer, text that the stop message holds)
    let long = format!("(const char[]){{ {} 0 }}", "97, ".repeat(399));
    let rows: Vec<(&str, String, &str)> = vec![
        ("high-0x80", c_ansi(b"\x80"), "a byte above 0x7F (0x80)"),
        ("high-0xe9", c_ansi(b"caf\xe9.dll"), "a byte above 0x7F (0xe9)"),
        ("high-0xff", c_ansi(b"\xff"), "a byte above 0x7F (0xff)"),
        ("high-after-kernel32", c_ansi(b"kernel32\xe9"), "a byte above 0x7F (0xe9)"),
        ("path-backslash", c_ansi(b"c:\\windows\\kernel32.dll"), "GetModuleHandleA with a path is not supported"),
        ("path-slash", c_ansi(b"dir/kernel32.dll"), "GetModuleHandleA with a path is not supported"),
        ("path-colon", c_ansi(b"c:kernel32.dll"), "GetModuleHandleA with a path is not supported"),
        ("long-name", long, "GetModuleHandleA name longer than 256 characters is not supported"),
    ];
    for (label, arg, say) in rows {
        let src = format!(
            "{PRE}int main(void)\n{{\n    R[1] = (uint64_t)(uintptr_t){arg}; fle_imp_GetModuleHandleA();\n    printf(\"returned %llu\\n\", (unsigned long long)R[0]);\n    return 0;\n}}\n"
        );
        let exe = clang(&format!("gmha-stop-{label}"), &src, &[]);
        let run = Command::new(&exe).output().unwrap();
        let err = String::from_utf8_lossy(&run.stderr).to_string();
        assert!(!run.status.success(), "{label}: expected a stop, stdout: {}", String::from_utf8_lossy(&run.stdout));
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{label}");
        assert_eq!(run.status.signal(), Some(6), "{label}");
        assert!(err.starts_with("fle: GetModuleHandleA") && err.contains(say), "{label}: {err}");
    }
    // The same characters as UTF-16 units: W answers "not found" (126) and does not stop; a stop in W needs a path or a long name.
    let wide_high = gmh_line(Some("caf\u{00e9}.dll")) + &gmh_line(Some("\u{0080}")) + &gmh_line(Some("kernel32\u{00e9}"));
    let src = format!("{PRE}int main(void)\n{{\n{wide_high}    return 0;\n}}\n");
    let exe = clang("gmh-wide-high", &src, &[]);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout).lines().collect::<Vec<_>>(), ["0 126", "0 126", "0 126"]);
}

/// The rows of `fle_resp_table` in the generated C, in order: (name, pseudo-VA).
fn resp_rows(c: &str) -> Vec<(String, u64)> {
    let start = c.find("const fle_resp_entry fle_resp_table[] = {").unwrap();
    let end = c[start..].find("};").unwrap() + start;
    c[start..end]
        .lines()
        .filter(|l| l.trim_start().starts_with("{ \""))
        .map(|l| {
            let name = l.split('"').nth(3).unwrap().to_string();
            let va = u64::from_str_radix(l.split("UINT64_C(0x").nth(1).unwrap().split(')').next().unwrap(), 16).unwrap();
            (name, va)
        })
        .collect()
}

/// The generator: a `jmp [IAT]` thunk of kernel32!GetModuleHandleA calls the runtime and has a row in the response table, appended after
/// the rows that existed (so that no existing pseudo-VA moves); GetProcAddress finds it in kernel32 by exact case, and the pseudo-VA, called
/// through fle_dispatch, answers like GetModuleHandleA.
///
/// Decision 0092 (and every later decision that appends rows) made this row no longer the last one, so the test no longer says it is.
/// It asserts what stays true when rows are appended: the row exists once, at position n (1-based) its pseudo-VA is
/// 0x7ff900000000 + 0x10 * n, every row before it keeps its position and VA (VA = base + 0x10 * (index + 1) for the whole table, which
/// is what keeps an old VA from moving), and the rows that were there before it (GetModuleHandleW 0xc0, GetProcAddress 0xd0) are where
/// they were. Where the last row is, and how many rows follow this one, is not pinned.
#[test]
fn get_module_handle_a_has_a_response_row_after_the_existing_ones_and_is_found_by_get_proc_address() {
    let (st, c) = gen(&["GetModuleHandleA", "GetProcAddress"]);
    assert_eq!(st.handled["jmp_imp:GetModuleHandleA"], 1);
    assert!(c.contains("fle_imp_GetModuleHandleA(); return;"));
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    for (i, (name, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7ff9_0000_0000u64 + 0x10 * (i as u64 + 1), "row {i} {name}: a pseudo-VA is base + 0x10 * (position), so appending never moves one");
    }
    let at = |n: &str| rows.iter().position(|(name, _)| name == n).unwrap_or_else(|| panic!("no row {n}"));
    let a = at("GetModuleHandleA");
    let n = a as u64 + 1; // the position of the row (1-based); its VA is base + 0x10 * n
    assert!(c.contains(&format!("{{ \"kernel32.dll\", \"GetModuleHandleA\", UINT64_C({:#x}), fle_imp_GetModuleHandleA }}", 0x7ff9_0000_0000u64 + 0x10 * n)));
    // the rows asserted by the first test (GetModuleHandleW 0xc0, GetProcAddress 0xd0) are where they were, and GetModuleHandleA comes after them
    assert!(c.contains("{ \"kernel32.dll\", \"GetModuleHandleW\", UINT64_C(0x7ff9000000c0), fle_imp_GetModuleHandleW }"));
    assert!(c.contains("{ \"kernel32.dll\", \"GetProcAddress\", UINT64_C(0x7ff9000000d0), fle_imp_GetProcAddress }"));
    assert!(at("GetModuleHandleW") < a && at("GetProcAddress") < a && at("GetModuleHandleExW") < a);
    assert_eq!(c.matches("\"GetModuleHandleA\"").count(), 1);

    let (rt, dir) = dirs();
    let tag = etxt_common::uniq("gen-gmha");
    let src = dir.join(format!("{tag}.c"));
    std::fs::write(&src, &c).unwrap();
    let main = "#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_dispatch.h\"\n#include \"fle_modules.h\"\n#include \"fle_imports.h\"\n\n\
int main(void)\n{\n\
    static const uint16_t k[] = { 'k','e','r','n','e','l','3','2', 0 };\n\
    uint64_t k32, va;\n\
    R[1] = (uint64_t)(uintptr_t)k; fle_imp_GetModuleHandleW(); k32 = R[0];\n\
    R[1] = k32; R[2] = (uint64_t)(uintptr_t)\"GetModuleHandleA\"; fle_imp_GetProcAddress(); va = R[0];\n\
    if (va == 0) { printf(\"FAIL not found\\n\"); return 1; }\n\
    R[1] = (uint64_t)(uintptr_t)\"API-MS-WIN-CORE-SYNCH-L1-2-0.DLL\"; fle_dispatch(va);\n\
    printf(\"dispatch %llx\\n\", (unsigned long long)R[0]);\n\
    R[1] = 0; fle_dispatch(va);\n\
    printf(\"null %llx\\n\", (unsigned long long)R[0]);\n\
    R[1] = k32; R[2] = (uint64_t)(uintptr_t)\"getmodulehandlea\"; fle_set_last_error(5); fle_imp_GetProcAddress();\n\
    printf(\"lower %llx\\n\", (unsigned long long)R[0]);\n\
    fle_imp_GetLastError(); printf(\"le %llu\\n\", (unsigned long long)R[0]);\n\
    return 0;\n}\n";
    let mainc = dir.join(format!("{tag}-main.c"));
    std::fs::write(&mainc, main).unwrap();
    let exe = dir.join(&tag);
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt);
    for p in runtime_sources(&rt) {
        cmd.arg(p);
    }
    let cc = cmd.arg(&src).arg(&mainc).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    assert!(run.status.success(), "{out}\n{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(out.lines().collect::<Vec<_>>(), [format!("dispatch {:x}", handle_of(26)), "null 400000".to_string(), "lower 0".to_string(), "le 127".to_string()]);
}

#[test]
fn module_handles_follow_the_formula_and_are_distinct() {
    let src = format!(
        "{PRE}int main(void)\n{{\n    uint64_t n = fle_module_count(), i, j;\n    printf(\"count %llu\\n\", (unsigned long long)n);\n\
    for (i = 0; i < n; i++) {{\n        const char *s = fle_module_name(i);\n        printf(\"%llu %s %llx\\n\", (unsigned long long)i, s, (unsigned long long)fle_module_handle(i));\n\
        for (j = i + 1; j < n; j++) if (fle_module_handle(i) == fle_module_handle(j) || !strcmp(s, fle_module_name(j))) {{ printf(\"DUP\\n\"); return 1; }}\n    }}\n\
    printf(\"over %p\\n\", (const void *)fle_module_name(n));\n    return 0;\n}}\n"
    );
    let exe = clang("module-table", &src, &[]);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success());
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = out.lines().collect();
    // 24 PE import DLLs, ntdll.dll (decision 0015), msctf.dll (decision 0055), the 4 API set names (decision 0081), avrt.dll and powrprof.dll (decision 0088),
    // the two deferred modules blend2d.dll and quickfontcache_x64.dll (decision 0096), shcore.dll (decision 0099), rpcrt4.dll (decision 0118),
    // wtsapi32.dll and uxtheme.dll (decision 0119; old: count 36, the end of the table after rpcrt4.dll), dwmapi.dll (decision 0122; old: count 38), kernelbase.dll (decision 0238; old: count 39).
    assert_eq!(lines[0], "count 41");   // decision 0238: kernelbase.dll and combase.dll (old: 39)
    for (i, l) in lines[1..42].iter().enumerate() {
        let f: Vec<&str> = l.split(' ').collect();
        assert_eq!(f[0], i.to_string());
        assert_eq!(f[2], format!("{:x}", handle_of(i as u64)));
        assert_eq!(f[1], f[1].to_ascii_lowercase());
    }
    assert_eq!(lines[31].split(' ').nth(1), Some("avrt.dll"));
    assert_eq!(lines[32].split(' ').nth(1), Some("powrprof.dll"));
    assert_eq!(lines[33].split(' ').nth(1), Some("blend2d.dll"));
    assert_eq!(lines[34].split(' ').nth(1), Some("quickfontcache_x64.dll"));
    assert_eq!(lines[35].split(' ').nth(1), Some("shcore.dll"));
    assert_eq!(lines[36].split(' ').nth(1), Some("rpcrt4.dll"));
    assert_eq!(lines[37].split(' ').nth(1), Some("wtsapi32.dll"));
    assert_eq!(lines[38].split(' ').nth(1), Some("uxtheme.dll"));
    assert_eq!(lines[39].split(' ').nth(1), Some("dwmapi.dll"));
    assert_eq!(lines[40].split(' ').nth(1), Some("kernelbase.dll"));
    assert_eq!(lines[41].split(' ').nth(1), Some("combase.dll"));
    assert!(lines[42].contains("nil") || lines[42].ends_with(" (nil)") || lines[42].ends_with(" 0x0") || lines[42].ends_with(" 0"));
}

fn real_dll() -> Option<Vec<u8>> {
    let p = std::env::var("FLENGINE_DLL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"));
    std::fs::read(p).ok()
}

/// The 24 DLLs of the PE import table all have a handle (skipped when the DLL is absent).
#[test]
fn every_pe_import_dll_is_a_module() {
    let Some(data) = real_dll() else { return };
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let mut dlls: Vec<String> = pe.imports().unwrap().iter().map(|i| i.dll.clone()).collect();
    dlls.sort_by_key(|d| d.to_ascii_lowercase());
    dlls.dedup_by_key(|d| d.to_ascii_lowercase());
    assert_eq!(dlls.len(), 24, "{dlls:?}");
    let mut body = String::new();
    for d in &dlls {
        // Mixed case as the PE spells it, and upper case.
        body += &gmh_line(Some(d));
        body += &gmh_line(Some(&d.to_ascii_uppercase()));
    }
    let src = format!("{PRE}int main(void)\n{{\n{body}    return 0;\n}}\n");
    let exe = clang("gmh-pe-dlls", &src, &[]);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success());
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let mut seen = std::collections::BTreeSet::new();
    for (i, l) in out.lines().enumerate() {
        assert!(!l.starts_with("0 "), "{} not found", dlls[i / 2]);
        assert!(l.ends_with(" 5"));
        seen.insert(l.to_string());
    }
    assert_eq!(seen.len(), 24, "handles must differ per DLL");
}

// ---- 3. GetProcAddress ----

/// A table of the test's own (not the generator's), with one fn that records which entry ran.
const TABLE_C: &str = "static int ran;\n\
static void fn_a(void) { ran = 1; R[0] = 111; }\n\
static void fn_b(void) { ran = 2; R[0] = 222; }\n\
const fle_resp_entry fle_resp_table[] = {\n\
    { \"kernel32.dll\", \"GetLastError\", UINT64_C(0x7ff900000010), fn_a },\n\
    { \"kernel32.dll\", \"GetVersion\", UINT64_C(0x7ff900000020), fn_b },\n\
    { \"user32.dll\", \"GetVersion\", UINT64_C(0x7ff900000030), fn_b },\n\
};\n\
const uint64_t fle_resp_count = 3;\n\
const fle_fn_entry fle_dispatch_table[] = {\n\
    { UINT64_C(0x7ff900000010), fn_a },\n\
    { UINT64_C(0x7ff900000020), fn_b },\n\
    { UINT64_C(0x7ff900000030), fn_b },\n\
};\n\
const uint64_t fle_dispatch_count = 3;\n";

fn gpa_line(module: &str, proc_name: &str) -> String {
    format!(
        "    R[1] = {mod}; R[2] = (uint64_t)(uintptr_t)\"{proc_name}\"; fle_set_last_error(5); R[0] = 99; fle_imp_GetProcAddress();\n    {{ uint64_t a = R[0]; fle_imp_GetLastError(); printf(\"%llx %llu\\n\", (unsigned long long)a, (unsigned long long)R[0]); }}\n",
        mod = module
    )
}

fn h(i: u64) -> String {
    format!("UINT64_C({:#x})", handle_of(i))
}

#[test]
fn get_proc_address_found_and_not_found() {
    let k = h(8); // kernel32.dll
    let u = h(18); // user32.dll
    let rows: Vec<(String, &str, &str)> = vec![
        (k.clone(), "GetLastError", "7ff900000010 5"),
        (k.clone(), "GetVersion", "7ff900000020 5"),
        (u.clone(), "GetVersion", "7ff900000030 5"),
        (h(24), "GetVersion", "0 127"),            // ntdll has no such row
        (k.clone(), "getlasterror", "0 127"),      // exact case
        (k.clone(), "GETLASTERROR", "0 127"),
        (k.clone(), "GetLastErro", "0 127"),
        (k.clone(), "GetLastErrorX", "0 127"),
        (k.clone(), "GetLogicalProcessorInformation", "0 127"), // not in the table: not claimed to exist
        (u.clone(), "GetLastError", "0 127"),      // same name, other module
        (h(1), "GetLastError", "0 127"),
        (k.clone(), "", "0 127"),
    ];
    let mut body = String::new();
    for (m, n, _) in &rows {
        body += &gpa_line(m, n);
    }
    let src = format!("{PRE}{TABLE_C}int main(void)\n{{\n{body}    return 0;\n}}\n");
    let exe = clang("gpa-rules", &src, &[]);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let got: Vec<&str> = out.lines().collect();
    assert_eq!(got.len(), rows.len());
    for (i, (_, n, want)) in rows.iter().enumerate() {
        assert_eq!(got[i], *want, "{n:?}");
    }
}

#[test]
fn get_proc_address_result_is_called_through_dispatch() {
    let src = format!(
        "{PRE}{TABLE_C}int main(void)\n{{\n\
    R[1] = {k}; R[2] = (uint64_t)(uintptr_t)\"GetVersion\"; fle_imp_GetProcAddress();\n\
    {{ uint64_t va = R[0]; R[0] = 0; fle_dispatch(va); printf(\"%d %llu\\n\", ran, (unsigned long long)R[0]); }}\n\
    R[1] = {k}; R[2] = (uint64_t)(uintptr_t)\"GetLastError\"; fle_imp_GetProcAddress();\n\
    {{ uint64_t va = R[0]; R[0] = 0; fle_dispatch(va); printf(\"%d %llu\\n\", ran, (unsigned long long)R[0]); }}\n\
    return 0;\n}}\n",
        k = h(8)
    );
    let exe = clang("gpa-dispatch", &src, &[]);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout).lines().collect::<Vec<_>>(), ["2 222", "1 111"]);
}

#[test]
fn get_proc_address_without_a_table_finds_nothing() {
    // No fle_resp_table is linked (weak reference): every lookup is "not found", not a crash.
    let src = format!(
        "{PRE}int main(void)\n{{\n{}    return 0;\n}}\n",
        gpa_line(&h(8), "GetLastError")
    );
    let exe = clang("gpa-no-table", &src, &[]);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "0 127");
}

// ---- 4. Undefined cases stop the run ----

#[test]
fn undefined_cases_abort() {
    use std::os::unix::process::ExitStatusExt;
    let k = h(8);
    let rows: Vec<(&str, String)> = vec![
        ("ordinal-1", format!("R[1] = {k}; R[2] = 1; fle_imp_GetProcAddress();")),
        ("ordinal-ffff", format!("R[1] = {k}; R[2] = 0xffff; fle_imp_GetProcAddress();")),
        ("null-name", format!("R[1] = {k}; R[2] = 0; fle_imp_GetProcAddress();")),
        // decision 0233 (load1): hModule NULL and the handle 0x400000 are the process image (FLEngine_x64.dll); a name it does not export is "not found" (NULL / 127, the next test), an export of its own still stops
        // (old rows "exe-handle" and "null-module" used the name "GetLastError", which the image does not export, and expected a stop)
        ("exe-handle", "R[1] = 0x400000; R[2] = (uint64_t)(uintptr_t)\"CreateFruityInstance\"; fle_imp_GetProcAddress();".to_string()),
        ("null-module", "R[1] = 0; R[2] = (uint64_t)(uintptr_t)\"dbkFCallWrapperAddr\"; fle_imp_GetProcAddress();".to_string()),
        ("null-module-ordinal", "R[1] = 0; R[2] = 3; fle_imp_GetProcAddress();".to_string()),
        ("bad-handle", "R[1] = 0x12345678; R[2] = (uint64_t)(uintptr_t)\"GetLastError\"; fle_imp_GetProcAddress();".to_string()),
        ("handle-not-aligned", format!("R[1] = {k} + 1; R[2] = (uint64_t)(uintptr_t)\"GetLastError\"; fle_imp_GetProcAddress();")),
        // decision 0238: the table has 40 modules, so the first handle past it is h(40) (old: h(39) with 39 modules after decision 0122, h(38) with 38)
        ("handle-past-table", format!("R[1] = {}; R[2] = (uint64_t)(uintptr_t)\"GetLastError\"; fle_imp_GetProcAddress();", h(41))),
        ("long-proc-name", format!("static char big[400]; memset(big, 'a', 399); R[1] = {k}; R[2] = (uint64_t)(uintptr_t)big; fle_imp_GetProcAddress();")),
        ("path-backslash", format!("R[1] = (uint64_t)(uintptr_t){}; fle_imp_GetModuleHandleW();", c_wide("c:\\windows\\kernel32.dll"))),
        ("path-slash", format!("R[1] = (uint64_t)(uintptr_t){}; fle_imp_GetModuleHandleW();", c_wide("dir/kernel32.dll"))),
        ("long-module-name", "static uint16_t big[400]; { int i; for (i = 0; i < 399; i++) big[i] = 'a'; } R[1] = (uint64_t)(uintptr_t)big; fle_imp_GetModuleHandleW();".to_string()),
    ];
    for (name, body) in rows {
        let src = format!("{PRE}{TABLE_C}int main(void)\n{{\n    {body}\n    printf(\"returned %llu\\n\", (unsigned long long)R[0]);\n    return 0;\n}}\n");
        let exe = clang(&format!("abort-{name}"), &src, &[]);
        let run = Command::new(&exe).output().unwrap();
        assert!(!run.status.success(), "{name}: expected abort, stdout: {}", String::from_utf8_lossy(&run.stdout));
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{name}");
        assert!(String::from_utf8_lossy(&run.stderr).contains("fle:"), "{name}: {}", String::from_utf8_lossy(&run.stderr));
        assert_eq!(run.status.signal(), Some(6), "{name}");
    }
}

#[test]
fn a_name_the_process_image_does_not_export_is_not_found() {
    // Decision 0233 (load1, PROVISIONAL; old: a stop). hModule NULL, or the handle of the process image (0x400000), is the image of the process, here FLEngine_x64.dll whose export directory has three names; a name that it lacks fails:
    // GetProcAddress page "If the function fails, the return value is NULL"; Wine 10.0 (UNOFFICIAL) get_proc_address line 72 + LdrGetProcedureAddress -> STATUS_PROCEDURE_NOT_FOUND -> ERROR_PROC_NOT_FOUND 127 (tests/load.rs has the full list).
    for (name, h) in [("null", "0"), ("exe", "0x400000")] {
        let body = format!("fle_set_last_error(5); R[1] = {h}; R[2] = (uint64_t)(uintptr_t)\"GetLastError\"; fle_imp_GetProcAddress(); printf(\"%llu \", (unsigned long long)R[0]); fle_imp_GetLastError(); printf(\"%llu\\n\", (unsigned long long)R[0]);");
        let src = format!("{PRE}{TABLE_C}int main(void)\n{{\n    {body}\n    return 0;\n}}\n");
        let exe = clang(&format!("proc-image-{name}"), &src, &[]);
        let run = Command::new(&exe).output().unwrap();
        assert!(run.status.success(), "{name}: {}", String::from_utf8_lossy(&run.stderr));
        assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "0 127", "{name}");
    }
}

/// With the real DLL: the generator emits the response table (skipped when the DLL is absent).
#[test]
fn real_dll_emits_response_table() {
    let Some(data) = real_dll() else { return };
    let mut out = Vec::new();
    let st = generate(&data, &mut out).unwrap();
    assert!(st.resp_entries >= 13, "{}", st.resp_entries);
    assert!(st.handled["jmp_imp:GetModuleHandleW"] >= 1);
    assert!(st.handled["jmp_imp:GetProcAddress"] >= 1);
    let c = String::from_utf8(out).unwrap();
    assert!(c.contains(&format!("const uint64_t fle_resp_count = {};", st.resp_entries)));
}
