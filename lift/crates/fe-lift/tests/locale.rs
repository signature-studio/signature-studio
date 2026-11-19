//! Tests for the emulated locale (docs/decisions/0016-locale.md): the kernel32 NLS answers in runtime/fle_locale.c.
//!
//! 1. Generator: the thunks of three locale imports call the runtime.
//! 2. Runtime, against the emulated locale en-US (0x0409, ACP 1252, OEM 437). Expected values are written in Rust
//!    from the MSDN rules (cited in docs/investigations/import-locale-spec.md), independent of the C code: the
//!    UTF-8/UTF-16 expectations come from Rust's own `str` encoders, the case mapping from `to_ascii_*`, the
//!    comparison from `ref_cmp` below, the sizes and error codes from the pages. The code page 1252 table of
//!    MultiByteToWideChar (decision 0084) is read from runtime/test/cp1252_expect.txt, which
//!    runtime/test/gen_cp1252_expect.py wrote from the Microsoft data file, not from the C array.
//! 3. Cases the pages leave open, or that need another locale's data, stop the run (SIGABRT).
//! 4. Decision 0087 (provisional): GetStringTypeW(CT_CTYPE1), LCMapStringW (LOWERCASE/UPPERCASE) and WideCharToMultiByte(1252) for the
//!    range D (284 units). The expected values are read from runtime/test/{c1_wine_10_0_enus,c1_ucd16_upper_lower,case_ms8_d,
//!    wctable_1252_expect}.txt (written by the gen_*.py scripts there from the Wine record, the UCD, the Windows 8 upper case table and
//!    the code page data file), not from the C arrays. The Wine C1 values are unofficial and cannot be regenerated.
//!
//! The host is Linux; the Windows NLS cannot run here, so nothing is compared with a real Windows.

mod etxt_common;

use fe_lift::generate;
use std::collections::BTreeMap;
use std::os::unix::process::ExitStatusExt;
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
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-locale/test");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    (rt, dir)
}

fn clang(name: &str, src: &str) -> PathBuf {
    let (rt, dir) = dirs();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-I"]).arg(&rt);
    for f in ["fle_locale.c", "fle_imports.c"] {
        cmd.arg(rt.join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

// ---- 1. Generator ----

#[test]
fn thunks_call_runtime() {
    let names = ["SetThreadLocale", "GetThreadLocale", "GetACP"];
    let img = build(&names);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    for n in names {
        assert_eq!(st.handled[format!("jmp_imp:{n}").as_str()], 1, "{n}");
        assert!(c.contains(&format!("fle_imp_{n}(); return;")), "{n}");
    }
    // None of the three is GetProcAddress, so no response table is emitted.
    assert_eq!(st.resp_entries, 0);
}

#[test]
fn all_thirteen_locale_imports_are_in_the_table() {
    let (_, dir) = dirs();
    let all = [
        "SetThreadLocale", "GetThreadLocale", "IsValidLocale", "GetUserDefaultUILanguage", "GetSystemDefaultUILanguage",
        "GetACP", "IsDBCSLeadByteEx", "GetCPInfo", "GetLocaleInfoW", "LCMapStringW", "CompareStringW",
        "MultiByteToWideChar", "WideCharToMultiByte",
    ];
    for chunk in all.chunks(3) {
        let img = build(chunk);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        for n in chunk {
            assert_eq!(st.handled[format!("jmp_imp:{n}").as_str()], 1, "{n}");
        }
    }
    let _ = dir;
}

// ---- 2. Runtime ----

const PRE: &str = "#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_imports.h\"\n#include \"fle_locale.h\"\n\n\
uint64_t R[16];\n\
static uint64_t st[16];\n\
#define P(...) ((uint64_t)(uintptr_t)(__VA_ARGS__))\n\
static void setup(uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7, uint64_t a8)\n\
{ R[1] = a1; R[2] = a2; R[8] = a3; R[9] = a4; st[5] = a5; st[6] = a6; st[7] = a7; st[8] = a8; R[4] = P(st); fle_set_last_error(0xAAAA); R[0] = 0xDEAD; }\n\
static void out(void) { uint64_t r = R[0]; fle_imp_GetLastError(); printf(\"%llx %llx\\n\", (unsigned long long)r, (unsigned long long)R[0]); }\n\
static void dumpw(const uint16_t *p, int n) { int i; printf(\"W\"); for (i = 0; i < n; i++) printf(\" %x\", p[i]); printf(\"\\n\"); }\n\
static void dumpb(const uint8_t *p, int n) { int i; printf(\"B\"); for (i = 0; i < n; i++) printf(\" %x\", p[i]); printf(\"\\n\"); }\n\
static void fillw(uint16_t *p, int n) { int i; for (i = 0; i < n; i++) p[i] = 0x5555; }\n\
static void fillb(uint8_t *p, int n) { int i; for (i = 0; i < n; i++) p[i] = 0x55; }\n\
#define RUN(fn, a1, a2, a3, a4, a5, a6, a7, a8) do { setup(a1, a2, a3, a4, a5, a6, a7, a8); fn(); } while (0)\n\
static uint16_t wbuf[128];\nstatic uint8_t bbuf[128];\n\
static uint16_t dbw1[320], dbw2[320], dbw3[320];\nstatic uint8_t dbb1[320];\n";

const E_NONE: u32 = 0xAAAA; // last error left as the harness set it
const E_PARAM: u32 = 87;
const E_BUF: u32 = 122;
const E_FLAGS: u32 = 1004;
const E_NOTRANS: u32 = 1113;

struct Prog {
    body: String,
    want: Vec<String>,
}

impl Prog {
    fn new() -> Self {
        Prog { body: String::new(), want: Vec::new() }
    }
    /// A call; `args` are C expressions (a1, a2, a3, a4, a5...), padded with 0. `pre` and `post` are C statements.
    fn call(&mut self, pre: &str, f: &str, args: &[&str], post: &str, want: &[String]) {
        let mut a: Vec<&str> = args.to_vec();
        while a.len() < 8 {
            a.push("0");
        }
        self.body += &format!("    {pre} RUN(fle_imp_{f}, {}); out(); {post}\n", a.join(", "));
        self.want.extend_from_slice(want);
    }
    fn ret(&mut self, f: &str, args: &[&str], r: u64, e: u32) {
        self.call("", f, args, "", &[line(r, e)]);
    }
    fn run(&self, name: &str) {
        let src = format!("{PRE}int main(void)\n{{\n{}    return 0;\n}}\n", self.body);
        let exe = clang(name, &src);
        let run = Command::new(&exe).output().unwrap();
        assert!(run.status.success(), "{name}: {}", String::from_utf8_lossy(&run.stderr));
        let out = String::from_utf8_lossy(&run.stdout).to_string();
        let got: Vec<&str> = out.lines().collect();
        assert_eq!(got.len(), self.want.len(), "{name}: {out}");
        for (i, (g, w)) in got.iter().zip(&self.want).enumerate() {
            assert_eq!(g, w, "{name}: line {i}");
        }
    }
}

fn line(r: u64, e: u32) -> String {
    format!("{r:x} {e:x}")
}
fn wl(units: &[u16]) -> String {
    let mut s = String::from("W");
    for u in units {
        s += &format!(" {u:x}");
    }
    s
}
fn bl(bytes: &[u8]) -> String {
    let mut s = String::from("B");
    for u in bytes {
        s += &format!(" {u:x}");
    }
    s
}
fn c_w(units: &[u16]) -> String {
    let v: Vec<String> = units.iter().map(|u| u.to_string()).collect();
    format!("P((const uint16_t[]){{ {} }})", v.join(", "))
}
fn c_b(bytes: &[u8]) -> String {
    let v: Vec<String> = bytes.iter().map(|u| u.to_string()).collect();
    format!("P((const uint8_t[]){{ {} }})", v.join(", "))
}
fn w_nul(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

#[test]
fn thread_locale_and_validity() {
    let mut p = Prog::new();
    // A thread starts with the user locale, here en-US (GetThreadLocale Remarks).
    p.ret("GetThreadLocale", &[], 0x409, E_NONE);
    // LOCALE_USER_DEFAULT (0x400) and LOCALE_SYSTEM_DEFAULT (0x800) are predefined values (MSDN): they resolve to the
    // emulated locale; the call succeeds.
    p.ret("SetThreadLocale", &["0x400"], 1, E_NONE);
    p.ret("GetThreadLocale", &[], 0x409, E_NONE);
    // German (0x0407) is a real locale but the environment does not have it: failure, thread locale unchanged.
    p.ret("SetThreadLocale", &["0x407"], 0, E_PARAM);
    p.ret("GetThreadLocale", &[], 0x409, E_NONE);
    p.ret("SetThreadLocale", &["0"], 0, E_PARAM);
    // LOCALE_INVARIANT is 0x007f (MSDN): accepted, and then GetThreadLocale returns it.
    p.ret("SetThreadLocale", &["0x7f"], 1, E_NONE);
    p.ret("GetThreadLocale", &[], 0x7f, E_NONE);
    p.ret("SetThreadLocale", &["0x409"], 1, E_NONE);
    p.ret("GetThreadLocale", &[], 0x409, E_NONE);
    // IsValidLocale: LCID_INSTALLED (1) and LCID_SUPPORTED (2).
    for fl in ["1", "2"] {
        p.ret("IsValidLocale", &["0x409", fl], 1, E_NONE);
        p.ret("IsValidLocale", &["0x400", fl], 1, E_NONE);
        p.ret("IsValidLocale", &["0x800", fl], 1, E_NONE);
        p.ret("IsValidLocale", &["0x7f", fl], 1, E_NONE);
        p.ret("IsValidLocale", &["0x407", fl], 0, E_NONE);
        p.ret("IsValidLocale", &["0x411", fl], 0, E_NONE);
        p.ret("IsValidLocale", &["0", fl], 0, E_NONE);
        p.ret("IsValidLocale", &["0x10409", fl], 0, E_NONE);
    }
    // UI languages: the LANGID of en-US.
    p.ret("GetUserDefaultUILanguage", &[], 0x409, E_NONE);
    p.ret("GetSystemDefaultUILanguage", &[], 0x409, E_NONE);
    p.ret("GetACP", &[], 1252, E_NONE);
    // IsDBCSLeadByteEx validates lead bytes only in 932, 936, 949, 950, 1361 (MSDN); elsewhere 0.
    for cp in ["0", "1", "3", "1252", "437", "65001", "1251"] {
        for b in ["0x41", "0x81", "0xE0", "0xFF"] {
            p.ret("IsDBCSLeadByteEx", &[cp, b], 0, E_NONE);
        }
    }
    p.run("locale-thread");
}

#[test]
fn thread_locale_is_per_thread_value() {
    // The thread locale is thread-local; a new thread starts with the user locale again (MSDN: GetThreadLocale and
    // SetThreadLocale Remarks: "When a thread is created, it uses the user locale"; the page also says a new thread
    // inherits the creating thread's locale, which the two pages do not agree on: the runtime follows the first).
    let src = format!(
        "#include <pthread.h>\n{PRE}static void *th(void *a) {{ (void)a; fle_imp_GetThreadLocale(); printf(\"t %llx\\n\", (unsigned long long)R[0]); return 0; }}\n\
int main(void)\n{{\n    pthread_t t;\n    RUN(fle_imp_SetThreadLocale, 0x7f, 0, 0, 0, 0, 0, 0, 0);\n    fle_imp_GetThreadLocale(); printf(\"m %llx\\n\", (unsigned long long)R[0]);\n    pthread_create(&t, 0, th, 0); pthread_join(t, 0);\n    return 0;\n}}\n"
    );
    let (rt, dir) = dirs();
    let tag = etxt_common::uniq("locale-thread-local");
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    std::fs::write(&c, src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"])
        .arg(&rt)
        .arg(rt.join("fle_locale.c"))
        .arg(rt.join("fle_imports.c"))
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .unwrap();
    assert!(cc.status.success(), "{}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().unwrap();
    // R[] is shared and the harness's last-error owner is the main thread: the second thread's GetThreadLocale
    // does not touch the last error, so only the locale value is checked.
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout).lines().collect::<Vec<_>>(), ["m 7f", "t 409"]);
}

#[test]
fn get_cp_info() {
    let mut p = Prog::new();
    // CPINFO: MaxCharSize (UINT), DefaultChar[2], LeadByte[12] = 20 bytes; single-byte code pages: 1, '?', no lead bytes.
    let mut want = vec![0u8; 20];
    want[0] = 1;
    want[4] = b'?';
    for cp in ["1252", "437", "0", "1", "3"] {
        p.call("fillb(bbuf, 32);", "GetCPInfo", &[cp, "P(bbuf)"], "dumpb(bbuf, 24);", &[line(1, E_NONE), {
            let mut v = want.clone();
            v.extend_from_slice(&[0x55; 4]);
            bl(&v)
        }]);
    }
    p.ret("GetCPInfo", &["1252", "0"], 0, E_PARAM);
    p.run("locale-cpinfo");
}

#[test]
fn get_locale_info() {
    let mut p = Prog::new();
    // (LCType, text) from the MSDN constant pages for English (United States).
    let rows: [(u32, &str); 11] = [
        (0x1, "0409"),
        (0x3, "ENU"),
        (0x59, "en"),
        (0x67, "eng"),
        (0x5a, "US"),
        (0x68, "USA"),
        (0x5c, "en-US"),
        (0x14, "$"),
        (0x1004, "1252"),
        (0xb, "437"),
        // LOCALE_IOPTIONALCALENDAR: not on the MSDN page; unofficial source (Wine en-US data), decision 0041.
        (0x100b, "0"),
    ];
    for lcid in ["0x409", "0x400", "0x800"] {
        for &(ty, text) in &rows {
            let u = w_nul(text);
            let n = u.len() as u64;
            let t = format!("{ty:#x}");
            p.ret("GetLocaleInfoW", &[lcid, &t, "0", "0"], n, E_NONE);
            let mut buf = vec![0x5555u16; 8];
            buf[..u.len()].copy_from_slice(&u);
            p.call("fillw(wbuf, 16);", "GetLocaleInfoW", &[lcid, &t, "P(wbuf)", "8"], "dumpw(wbuf, 8);", &[line(n, E_NONE), wl(&buf)]);
            // one unit too small, and a NULL buffer with a size
            p.ret("GetLocaleInfoW", &[lcid, &t, "P(wbuf)", &format!("{}", n - 1)], 0, E_BUF);
            p.ret("GetLocaleInfoW", &[lcid, &t, "0", "8"], 0, E_BUF);
            // negative size
            p.ret("GetLocaleInfoW", &[lcid, &t, "P(wbuf)", "-1"], 0, E_PARAM);
        }
    }
    // LOCALE_NOUSEROVERRIDE may be combined with any constant.
    p.call("fillw(wbuf, 16);", "GetLocaleInfoW", &["0x409", "0x8000005c", "P(wbuf)", "8"], "dumpw(wbuf, 6);", &[line(6, E_NONE), wl(&w_nul("en-US"))]);
    // LOCALE_RETURN_NUMBER with an integer type: a DWORD (2 characters), the return value is 2.
    for (ty, v) in [(0x1004u32, 1252u32), (0xb, 437), (0x100b, 0)] {
        let t = format!("{:#x}", ty | 0x2000_0000);
        p.call("fillw(wbuf, 16);", "GetLocaleInfoW", &["0x409", &t, "P(wbuf)", "2"], "dumpw(wbuf, 3);", &[line(2, E_NONE), wl(&[(v & 0xFFFF) as u16, (v >> 16) as u16, 0x5555])]);
        p.ret("GetLocaleInfoW", &["0x409", &t, "P(wbuf)", "1"], 0, E_BUF);
    }
    p.run("locale-info");
}

// The 0042 en-US values (docs/decisions/0042-locale-enus-values.md): Wine en-US data and CLDR agree on them; not
// on the MSDN pages, provisional. Written here in Rust, apart from the C table: the day and month names are listed
// once as full names, and the abbreviations are their first three letters (a property of these 38 names, checked
// against Wine's separate abbreviation table in the decision), and the numbers come from the decision's table.
#[test]
fn get_locale_info_enus_0042() {
    let days = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    let months = [
        "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December",
    ];
    let mut rows: Vec<(u32, String)> = vec![
        (0x2, "English (United States)".into()),
        (0xe, ".".into()),
        (0xf, ",".into()),
        (0x19, "2".into()),
        (0x1b, "0".into()),
        (0x1d, "/".into()),
        (0x1e, ":".into()),
        (0x23, "0".into()),
        (0x25, "0".into()),
        (0x28, "AM".into()),
        (0x29, "PM".into()),
        (0x1005, "0".into()),
        (0x1009, "1".into()),
    ];
    for (i, d) in days.iter().enumerate() {
        rows.push((0x2a + i as u32, d.to_string()));
        rows.push((0x31 + i as u32, d[..3].to_string()));
    }
    for (i, m) in months.iter().enumerate() {
        rows.push((0x38 + i as u32, m.to_string()));
        rows.push((0x44 + i as u32, m[..3].to_string()));
    }
    assert_eq!(rows.len(), 13 + 14 + 24);
    let mut p = Prog::new();
    for lcid in ["0x409", "0x400", "0x800"] {
        for (ty, text) in &rows {
            let u = w_nul(text);
            let n = u.len();
            let t = format!("{ty:#x}");
            let cap = n + 3;
            p.ret("GetLocaleInfoW", &[lcid, &t, "0", "0"], n as u64, E_NONE);
            let mut buf = vec![0x5555u16; cap];
            buf[..n].copy_from_slice(&u);
            p.call(
                &format!("fillw(wbuf, {});", cap + 2),
                "GetLocaleInfoW",
                &[lcid, &t, "P(wbuf)", &format!("{cap}")],
                &format!("dumpw(wbuf, {cap});"),
                &[line(n as u64, E_NONE), wl(&buf)],
            );
            p.ret("GetLocaleInfoW", &[lcid, &t, "P(wbuf)", &format!("{}", n - 1)], 0, E_BUF);
            p.ret("GetLocaleInfoW", &[lcid, &t, "0", &format!("{cap}")], 0, E_BUF);
            p.ret("GetLocaleInfoW", &[lcid, &t, "P(wbuf)", "-1"], 0, E_PARAM);
        }
    }
    // LOCALE_NOUSEROVERRIDE (accepted) on a name; LOCALE_USE_CP_ACP is ignored.
    p.call("fillw(wbuf, 16);", "GetLocaleInfoW", &["0x409", "0x80000037", "P(wbuf)", "8"], "dumpw(wbuf, 5);", &[line(4, E_NONE), wl(&[0x53, 0x75, 0x6e, 0, 0x5555])]);
    // LOCALE_RETURN_NUMBER on the integer types: a DWORD (2 characters), return value 2; one character is too small.
    for (ty, v) in [(0x19u32, 2u32), (0x1b, 0), (0x23, 0), (0x25, 0), (0x1005, 0), (0x1009, 1)] {
        let t = format!("{:#x}", ty | 0x2000_0000);
        p.call("fillw(wbuf, 16);", "GetLocaleInfoW", &["0x409", &t, "P(wbuf)", "2"], "dumpw(wbuf, 3);", &[line(2, E_NONE), wl(&[(v & 0xFFFF) as u16, (v >> 16) as u16, 0x5555])]);
        p.ret("GetLocaleInfoW", &["0x409", &t, "P(wbuf)", "1"], 0, E_BUF);
    }
    p.run("locale-info-0042");
}

// The 0048 en-US values (docs/decisions/0048-locale-six.md): provisional, not checked on real Windows. Five strings
// (NLS notation, plain U+0020 before "tt") and the integer INEGCURR (0).
#[test]
fn get_locale_info_enus_0048() {
    let rows: [(u32, &str); 6] = [
        (0xc, ","),
        (0x1f, "M/d/yyyy"),
        (0x20, "dddd, MMMM d, yyyy"),
        (0x1003, "h:mm:ss tt"),
        (0x79, "h:mm tt"),
        (0x1c, "0"),
    ];
    let mut p = Prog::new();
    for lcid in ["0x409", "0x400", "0x800"] {
        for &(ty, text) in &rows {
            let u = w_nul(text);
            let n = u.len();
            let t = format!("{ty:#x}");
            let cap = n + 3;
            p.ret("GetLocaleInfoW", &[lcid, &t, "0", "0"], n as u64, E_NONE);
            let mut buf = vec![0x5555u16; cap];
            buf[..n].copy_from_slice(&u);
            p.call(
                &format!("fillw(wbuf, {});", cap + 2),
                "GetLocaleInfoW",
                &[lcid, &t, "P(wbuf)", &format!("{cap}")],
                &format!("dumpw(wbuf, {cap});"),
                &[line(n as u64, E_NONE), wl(&buf)],
            );
            p.ret("GetLocaleInfoW", &[lcid, &t, "P(wbuf)", &format!("{}", n - 1)], 0, E_BUF);
            p.ret("GetLocaleInfoW", &[lcid, &t, "0", &format!("{cap}")], 0, E_BUF);
            p.ret("GetLocaleInfoW", &[lcid, &t, "P(wbuf)", "-1"], 0, E_PARAM);
        }
    }
    // LOCALE_NOUSEROVERRIDE (accepted) on a format string.
    let u = w_nul("h:mm tt");
    let mut buf = u.clone();
    buf.push(0x5555);
    p.call("fillw(wbuf, 16);", "GetLocaleInfoW", &["0x409", "0x80000079", "P(wbuf)", "16"], &format!("dumpw(wbuf, {});", buf.len()), &[line(u.len() as u64, E_NONE), wl(&buf)]);
    // LOCALE_RETURN_NUMBER on the integer type INEGCURR: a DWORD (2 characters), return value 2; one character is too small.
    p.call("fillw(wbuf, 16);", "GetLocaleInfoW", &["0x409", "0x2000001c", "P(wbuf)", "2"], "dumpw(wbuf, 3);", &[line(2, E_NONE), wl(&[0, 0, 0x5555])]);
    p.ret("GetLocaleInfoW", &["0x409", "0x2000001c", "P(wbuf)", "1"], 0, E_BUF);
    p.run("locale-info-0048");
}

fn utf8_sample() -> String {
    "h\u{e9}llo \u{20ac} \u{1F600}!".to_string()
}

#[test]
fn multi_byte_to_wide_char() {
    let mut p = Prog::new();
    let s = utf8_sample();
    let mut sb = s.as_bytes().to_vec();
    let w: Vec<u16> = s.encode_utf16().collect();
    let wn = w_nul(&s);
    sb.push(0);
    let src = c_b(&sb);
    let (lw, lwn) = (w.len() as u64, wn.len() as u64);
    // -1: NUL-terminated; the NUL is converted and counted.
    p.ret("MultiByteToWideChar", &["65001", "0", &src, "-1", "0", "0"], lwn, E_NONE);
    p.call("fillw(wbuf, 64);", "MultiByteToWideChar", &["65001", "0", &src, "-1", "P(wbuf)", "64"], &format!("dumpw(wbuf, {});", wn.len() + 1), &[line(lwn, E_NONE), {
        let mut v = wn.clone();
        v.push(0x5555);
        wl(&v)
    }]);
    // exact size works, one less fails with ERROR_INSUFFICIENT_BUFFER
    p.ret("MultiByteToWideChar", &["65001", "0", &src, "-1", "P(wbuf)", &lwn.to_string()], lwn, E_NONE);
    p.ret("MultiByteToWideChar", &["65001", "0", &src, "-1", "P(wbuf)", &(lwn - 1).to_string()], 0, E_BUF);
    // explicit byte count without the NUL: no NUL is written and not counted
    let nb = (sb.len() - 1).to_string();
    p.call("fillw(wbuf, 64);", "MultiByteToWideChar", &["65001", "0", &src, &nb, "P(wbuf)", "64"], &format!("dumpw(wbuf, {});", w.len() + 1), &[line(lw, E_NONE), {
        let mut v = w.clone();
        v.push(0x5555);
        wl(&v)
    }]);
    p.ret("MultiByteToWideChar", &["65001", "0", &src, &nb, "0", "0"], lw, E_NONE);
    // flags: only 0 and MB_ERR_INVALID_CHARS (8) for UTF-8
    p.ret("MultiByteToWideChar", &["65001", "8", &src, "-1", "0", "0"], lwn, E_NONE);
    for f in ["1", "2", "4", "0x10"] {
        p.ret("MultiByteToWideChar", &["65001", f, &src, "-1", "0", "0"], 0, E_FLAGS);
    }
    // cbMultiByte 0 fails; the same pointer for source and buffer fails
    p.ret("MultiByteToWideChar", &["65001", "0", &src, "0", "0", "0"], 0, E_PARAM);
    p.ret("MultiByteToWideChar", &["65001", "0", "P(wbuf)", "-1", "P(wbuf)", "4"], 0, E_PARAM);
    p.ret("MultiByteToWideChar", &["65001", "0", "0", "-1", "0", "0"], 0, E_PARAM);
    // Ill-formed UTF-8 with MB_ERR_INVALID_CHARS fails with ERROR_NO_UNICODE_TRANSLATION.
    let bad: [&[u8]; 8] = [
        &[0x80, 0],                   // lone continuation byte
        &[0xC0, 0x80, 0],             // overlong NUL
        &[0xC1, 0xBF, 0],             // overlong
        &[0xE0, 0x80, 0x80, 0],       // overlong
        &[0xED, 0xA0, 0x80, 0],       // surrogate U+D800
        &[0xF4, 0x90, 0x80, 0x80, 0], // above U+10FFFF
        &[0xE2, 0x82, 0],             // truncated
        &[0xFF, 0],
    ];
    for b in bad {
        assert!(std::str::from_utf8(&b[..b.len() - 1]).is_err());
        p.ret("MultiByteToWideChar", &["65001", "8", &c_b(b), "-1", "0", "0"], 0, E_NOTRANS);
    }
    // Every scalar value class round-trips with the widths the UTF-8 rules give.
    for ch in ['\u{7f}', '\u{80}', '\u{7ff}', '\u{800}', '\u{d7ff}', '\u{e000}', '\u{ffff}', '\u{10000}', '\u{10ffff}'] {
        let t = format!("{ch}");
        let mut tb = t.as_bytes().to_vec();
        tb.push(0);
        let expect: Vec<u16> = w_nul(&t);
        p.call("fillw(wbuf, 16);", "MultiByteToWideChar", &["65001", "8", &c_b(&tb), "-1", "P(wbuf)", "16"], &format!("dumpw(wbuf, {});", expect.len()), &[line(expect.len() as u64, E_NONE), wl(&expect)]);
    }
    // ASCII in the environment's ANSI (CP_ACP 0, 1252, CP_THREAD_ACP 3) and OEM (CP_OEMCP 1, 437) code pages.
    let asc = c_b(b"Hi!\0");
    for cp in ["0", "1", "3", "1252", "437"] {
        p.call("fillw(wbuf, 8);", "MultiByteToWideChar", &[cp, "0", &asc, "-1", "P(wbuf)", "8"], "dumpw(wbuf, 5);", &[line(4, E_NONE), wl(&[0x48, 0x69, 0x21, 0, 0x5555])]);
        p.ret("MultiByteToWideChar", &[cp, "0", &asc, "3", "0", "0"], 3, E_NONE);
    }
    // negative output size
    p.ret("MultiByteToWideChar", &["65001", "0", &src, "-1", "P(wbuf)", "-1"], 0, E_PARAM);
    p.run("locale-mbtwc");
}

// ---- code page 1252, bytes 0x80 and above (decision 0084) ----

/// byte -> UTF-16 unit, from the file gen_cp1252_expect.py wrote (256 rows, no zip needed here).
fn cp1252_expect() -> Vec<u16> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime/test/cp1252_expect.txt");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut t: Vec<Option<u16>> = vec![None; 256];
    for l in text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (b, u) = l.split_once(' ').unwrap();
        let b = usize::from_str_radix(b.trim_start_matches("0x"), 16).unwrap();
        let u = u16::from_str_radix(u.trim_start_matches("0x"), 16).unwrap();
        assert!(t[b].replace(u).is_none(), "byte {b:#x} twice");
    }
    t.into_iter().map(|u| u.expect("a byte without a row")).collect()
}

const FLAGS_1252: [&str; 4] = ["0", "1", "8", "9"]; // 0, MB_PRECOMPOSED, MB_ERR_INVALID_CHARS, both

#[test]
fn mbtwc_cp1252_every_byte_one_by_one() {
    let exp = cp1252_expect();
    let mut p = Prog::new();
    for fl in FLAGS_1252 {
        for b in 0..=255u8 {
            // one byte, an explicit count of 1: one unit, nothing after it is touched
            p.call(
                "fillw(wbuf, 4);",
                "MultiByteToWideChar",
                &["1252", fl, &c_b(&[b]), "1", "P(wbuf)", "4"],
                "dumpw(wbuf, 2);",
                &[line(1, E_NONE), wl(&[exp[b as usize], 0x5555])],
            );
        }
    }
    p.run("locale-mbtwc-1252-bytes");
}

#[test]
fn mbtwc_cp1252_all_256_bytes_at_once_and_the_size_query() {
    let exp = cp1252_expect();
    let all: Vec<u8> = (0..=255u8).collect();
    let src = c_b(&all);
    let mut want_w = exp.clone();
    want_w.push(0x5555);
    // 255 bytes (1..=255) and a NUL: -1 counts up to the terminator, which is converted and counted: 256 units
    let mut nul_term: Vec<u8> = (1..=255u8).collect();
    nul_term.push(0);
    let mut exp_nul: Vec<u16> = exp[1..].to_vec();
    exp_nul.push(0);
    exp_nul.push(0x5555);
    let mut p = Prog::new();
    let mut first = true;
    for fl in FLAGS_1252 {
        let decl = if first { "static uint16_t big[300]; fillw(big, 300);" } else { "fillw(big, 300);" };
        first = false;
        p.call(decl, "MultiByteToWideChar", &["1252", fl, &src, "256", "P(big)", "300"], "dumpw(big, 257);", &[line(256, E_NONE), wl(&{
            let mut v = exp.clone();
            v.push(0x5555);
            v
        })]);
        // the size query (cchWideChar 0, no output): 256, for an explicit count and for -1
        p.ret("MultiByteToWideChar", &["1252", fl, &src, "256", "0", "0"], 256, E_NONE);
        p.ret("MultiByteToWideChar", &["1252", fl, &c_b(&nul_term), "-1", "0", "0"], 256, E_NONE);
        // exactly 256 units fit; 255 do not (ERROR_INSUFFICIENT_BUFFER)
        p.call("fillw(big, 300);", "MultiByteToWideChar", &["1252", fl, &src, "256", "P(big)", "256"], "dumpw(big, 257);", &[line(256, E_NONE), wl(&want_w)]);
        p.ret("MultiByteToWideChar", &["1252", fl, &src, "256", "P(big)", "255"], 0, E_BUF);
        p.call("fillw(big, 300);", "MultiByteToWideChar", &["1252", fl, &c_b(&nul_term), "-1", "P(big)", "300"], "dumpw(big, 257);", &[line(256, E_NONE), wl(&exp_nul)]);
    }
    p.run("locale-mbtwc-1252-all");
}

#[test]
fn mbtwc_cp_acp_and_thread_acp_are_1252() {
    // CP_ACP (0) resolves to the emulated ANSI code page (1252); CP_THREAD_ACP (3) too, for the emulated thread locale.
    let exp = cp1252_expect();
    let all: Vec<u8> = (0..=255u8).collect();
    let src = c_b(&all);
    let mut p = Prog::new();
    let mut first = true;
    for cp in ["0", "3"] {
        for fl in ["0", "9"] {
            let decl = if first { "static uint16_t big[300]; fillw(big, 300);" } else { "fillw(big, 300);" };
            first = false;
            p.call(decl, "MultiByteToWideChar", &[cp, fl, &src, "256", "P(big)", "300"], "dumpw(big, 257);", &[line(256, E_NONE), wl(&{
                let mut v = exp.clone();
                v.push(0x5555);
                v
            })]);
            p.ret("MultiByteToWideChar", &[cp, fl, &src, "256", "0", "0"], 256, E_NONE);
        }
    }
    for b in [0x80u8, 0x81, 0x8d, 0x8e, 0x8f, 0x90, 0x9d, 0x9e, 0x9f, 0xa0, 0xff] {
        p.call("fillw(wbuf, 4);", "MultiByteToWideChar", &["0", "0", &c_b(&[b]), "1", "P(wbuf)", "4"], "dumpw(wbuf, 2);", &[line(1, E_NONE), wl(&[exp[b as usize], 0x5555])]);
    }
    p.run("locale-mbtwc-acp");
}

/// A call that must stop the run by name: SIGABRT, "fle:" and `needle` on stderr, nothing returned.
fn assert_named_stop(name: &str, body: &str, needle: &str) {
    let src = format!("{PRE}int main(void)\n{{\n    setup(0, 0, 0, 0, 0, 0, 0, 0);\n    {body}\n    printf(\"returned %llx\\n\", (unsigned long long)R[0]);\n    return 0;\n}}\n");
    let exe = clang(&format!("abort-{name}"), &src);
    let run = Command::new(&exe).output().unwrap();
    let (out, err) = (String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
    assert!(!run.status.success(), "{name}: expected abort, stdout: {out}");
    assert!(!out.contains("returned"), "{name}");
    assert!(err.contains("fle: MultiByteToWideChar: ") && err.contains(needle), "{name}: {err}");
    assert_eq!(run.status.signal(), Some(6), "{name}");
}

#[test]
fn mbtwc_cp1252_composite_and_glyph_flags_still_stop_on_a_high_byte() {
    let all: Vec<u8> = (0..=255u8).collect();
    let high = c_b(&[0x80, 0]);
    let whole = c_b(&all);
    let needle = "MB_COMPOSITE or MB_USEGLYPHCHARS";
    // 2 = MB_COMPOSITE, 4 = MB_USEGLYPHCHARS (and combinations that keep one of them and are not the invalid 3 and 7)
    for fl in [2, 4, 5, 6, 0xa, 0xc] {
        assert_named_stop(&format!("mbtwc-1252-flags-{fl:x}-high"), &format!("RUN(fle_imp_MultiByteToWideChar, 1252, {fl}, {high}, -1, 0, 0, 0, 0);"), needle);
    }
    for fl in [2, 4] {
        assert_named_stop(&format!("mbtwc-1252-flags-{fl:x}-all256"), &format!("RUN(fle_imp_MultiByteToWideChar, 1252, {fl}, {whole}, 256, 0, 0, 0, 0);"), needle);
        assert_named_stop(&format!("mbtwc-acp-flags-{fl:x}-high"), &format!("RUN(fle_imp_MultiByteToWideChar, 0, {fl}, {high}, -1, 0, 0, 0, 0);"), needle);
    }
}

#[test]
fn mbtwc_cp437_high_byte_still_stops() {
    let all: Vec<u8> = (0..=255u8).collect();
    let needle = "byte of 0x80 or above in a single-byte code page";
    // 437 and CP_OEMCP (1) are not covered by decision 0084; the table stays unadopted, for every flag that is accepted
    for fl in [0, 1, 8, 9] {
        assert_named_stop(&format!("mbtwc-437-flags-{fl:x}-0x80"), &format!("RUN(fle_imp_MultiByteToWideChar, 437, {fl}, {}, -1, 0, 0, 0, 0);", c_b(&[0x80, 0])), needle);
    }
    assert_named_stop("mbtwc-437-0xff", &format!("RUN(fle_imp_MultiByteToWideChar, 437, 0, {}, 1, 0, 0, 0, 0);", c_b(&[0xFF])), needle);
    assert_named_stop("mbtwc-oemcp-0x80", &format!("RUN(fle_imp_MultiByteToWideChar, 1, 0, {}, -1, 0, 0, 0, 0);", c_b(&[0x80, 0])), needle);
    assert_named_stop("mbtwc-437-size-query-256", &format!("RUN(fle_imp_MultiByteToWideChar, 437, 1, {}, 256, 0, 0, 0, 0);", c_b(&all)), needle);
}

#[test]
fn wide_char_to_multi_byte() {
    let mut p = Prog::new();
    let s = utf8_sample();
    let mut sb = s.as_bytes().to_vec();
    sb.push(0);
    let wn = w_nul(&s);
    let src = c_w(&wn);
    let (nb, nbn) = ((sb.len() - 1) as u64, sb.len() as u64);
    p.ret("WideCharToMultiByte", &["65001", "0", &src, "-1", "0", "0", "0", "0"], nbn, E_NONE);
    p.call("fillb(bbuf, 64);", "WideCharToMultiByte", &["65001", "0", &src, "-1", "P(bbuf)", "64", "0", "0"], &format!("dumpb(bbuf, {});", sb.len() + 1), &[line(nbn, E_NONE), {
        let mut v = sb.clone();
        v.push(0x55);
        bl(&v)
    }]);
    p.ret("WideCharToMultiByte", &["65001", "0", &src, "-1", "P(bbuf)", &nbn.to_string(), "0", "0"], nbn, E_NONE);
    p.ret("WideCharToMultiByte", &["65001", "0", &src, "-1", "P(bbuf)", &(nbn - 1).to_string(), "0", "0"], 0, E_BUF);
    // explicit count without NUL
    let cw = (wn.len() - 1).to_string();
    p.call("fillb(bbuf, 64);", "WideCharToMultiByte", &["65001", "0", &src, &cw, "P(bbuf)", "64", "0", "0"], &format!("dumpb(bbuf, {});", sb.len()), &[line(nb, E_NONE), {
        let mut v = sb[..sb.len() - 1].to_vec();
        v.push(0x55);
        bl(&v)
    }]);
    // flags: 0 or WC_ERR_INVALID_CHARS (0x80) for UTF-8
    p.ret("WideCharToMultiByte", &["65001", "0x80", &src, "-1", "0", "0", "0", "0"], nbn, E_NONE);
    for f in ["1", "0x400", "0x200", "0x40"] {
        p.ret("WideCharToMultiByte", &["65001", f, &src, "-1", "0", "0", "0", "0"], 0, E_FLAGS);
    }
    // lpDefaultChar and lpUsedDefaultChar must be NULL for UTF-8
    p.ret("WideCharToMultiByte", &["65001", "0", &src, "-1", "0", "0", "P(\"?\")", "0"], 0, E_PARAM);
    p.ret("WideCharToMultiByte", &["65001", "0", &src, "-1", "0", "0", "0", "P(&bbuf)"], 0, E_PARAM);
    // lone surrogates: WC_ERR_INVALID_CHARS fails; otherwise U+FFFD
    for lone in [vec![0xD800u16, 0x61, 0], vec![0xDC00, 0x61, 0], vec![0x61, 0xD800, 0]] {
        p.ret("WideCharToMultiByte", &["65001", "0x80", &c_w(&lone), "-1", "0", "0", "0", "0"], 0, E_NOTRANS);
        let expect: Vec<u8> = String::from_utf16_lossy(&lone).into_bytes();
        p.call("fillb(bbuf, 16);", "WideCharToMultiByte", &["65001", "0", &c_w(&lone), "-1", "P(bbuf)", "16", "0", "0"], &format!("dumpb(bbuf, {});", expect.len()), &[line(expect.len() as u64, E_NONE), bl(&expect)]);
    }
    p.ret("WideCharToMultiByte", &["65001", "0", &src, "0", "0", "0", "0", "0"], 0, E_PARAM);
    p.ret("WideCharToMultiByte", &["65001", "0", "0", "-1", "0", "0", "0", "0"], 0, E_PARAM);
    // ASCII in the environment's code pages; lpUsedDefaultChar is set to FALSE.
    let asc = c_w(&[0x48, 0x69, 0x21, 0]);
    for cp in ["0", "1", "3", "1252", "437"] {
        p.call("fillb(bbuf, 8); { static uint32_t u; u = 1; setup(0, 0, 0, 0, 0, 0, 0, 0);", "WideCharToMultiByte", &[cp, "0", &asc, "-1", "P(bbuf)", "8", "0", "P(&u)"], "printf(\"U %u\\n\", u); dumpb(bbuf, 5); }", &[line(4, E_NONE), "U 0".into(), bl(&[0x48, 0x69, 0x21, 0, 0x55])]);
    }
    p.run("locale-wctmb");
}

#[test]
fn lc_map_string() {
    let mut p = Prog::new();
    let s = "Hello, World! i I 123 {}[]@`";
    let wn = w_nul(s);
    let src = c_w(&wn);
    let n = wn.len() as u64;
    let up: Vec<u16> = w_nul(&s.to_ascii_uppercase());
    let lo: Vec<u16> = w_nul(&s.to_ascii_lowercase());
    for lcid in ["0x409", "0x400", "0x800", "0x7f"] {
        for (flag, want) in [("0x200", &up), ("0x100", &lo), ("0x01000200", &up), ("0x01000100", &lo)] {
            // negative length: NUL-terminated, the NUL is mapped and counted
            p.ret("LCMapStringW", &[lcid, flag, &src, "-1", "0", "0"], n, E_NONE);
            p.call("fillw(wbuf, 64);", "LCMapStringW", &[lcid, flag, &src, "-1", "P(wbuf)", "64"], &format!("dumpw(wbuf, {});", wn.len() + 1), &[line(n, E_NONE), {
                let mut v = want.clone();
                v.push(0x5555);
                wl(&v)
            }]);
            p.ret("LCMapStringW", &[lcid, flag, &src, "-1", "P(wbuf)", &(n - 1).to_string()], 0, E_BUF);
            p.ret("LCMapStringW", &[lcid, flag, &src, "-1", "P(wbuf)", &n.to_string()], n, E_NONE);
            // explicit length without the NUL: nothing is added
            let m = (wn.len() - 1).to_string();
            p.call("fillw(wbuf, 64);", "LCMapStringW", &[lcid, flag, &src, &m, "P(wbuf)", "64"], &format!("dumpw(wbuf, {});", wn.len()), &[line(n - 1, E_NONE), {
                let mut v = want[..want.len() - 1].to_vec();
                v.push(0x5555);
                wl(&v)
            }]);
        }
    }
    // bit not defined by the pages, LCMAP_LINGUISTIC_CASING alone: ERROR_INVALID_FLAGS
    p.ret("LCMapStringW", &["0x409", "0x40000200", &src, "-1", "0", "0"], 0, E_FLAGS);
    p.ret("LCMapStringW", &["0x409", "0x01000000", &src, "-1", "0", "0"], 0, E_FLAGS);
    // NULL source, size 0, negative output size
    p.ret("LCMapStringW", &["0x409", "0x200", "0", "-1", "0", "0"], 0, E_PARAM);
    p.ret("LCMapStringW", &["0x409", "0x200", &src, "0", "0", "0"], 0, E_PARAM);
    p.ret("LCMapStringW", &["0x409", "0x200", &src, "-1", "P(wbuf)", "-1"], 0, E_PARAM);
    p.run("locale-lcmap");
}

/// CompareString order for ASCII letters and digits, as the MSDN rules give it for this subset: equal after
/// optional ASCII folding is CSTR_EQUAL; otherwise the first position of a differing (folded) unit decides, within
/// letters or within digits; None where the pages do not decide.
fn ref_cmp(a: &str, b: &str, ignore_case: bool) -> Option<u64> {
    let f = |c: char| if ignore_case { c.to_ascii_lowercase() } else { c };
    if a.chars().map(f).eq(b.chars().map(f)) {
        return Some(2);
    }
    for (x, y) in a.chars().zip(b.chars()) {
        let (x, y) = (x.to_ascii_lowercase(), y.to_ascii_lowercase());
        if x == y {
            continue;
        }
        if x.is_ascii_alphabetic() && y.is_ascii_alphabetic() || x.is_ascii_digit() && y.is_ascii_digit() {
            return Some(if x < y { 1 } else { 3 });
        }
        return None;
    }
    None
}

#[test]
fn compare_string() {
    let mut p = Prog::new();
    let pairs = [
        ("abc", "abc"), ("ABC", "abc"), ("abc", "ABC"), ("abc", "abd"), ("b", "a"), ("a", "b"), ("Abc", "abd"),
        ("abd", "Abc"), ("x1", "x2"), ("a9", "a10"), ("a10", "a9"), ("Zebra", "apple"), ("apple", "Zebra"), ("", ""),
        ("1", "2"), ("B", "a"), ("a", "B"),
    ];
    for lcid in ["0x409", "0x400", "0x800"] {
        for ic in [false, true] {
            for (a, b) in pairs {
                // A case-only difference without NORM_IGNORECASE is not decided by the pages (the runtime stops;
                // see `cmp-case-only` in undefined_cases_abort).
                let Some(want) = ref_cmp(a, b, ic) else {
                    assert!(!ic && a.eq_ignore_ascii_case(b), "{a} {b}");
                    continue;
                };
                let (wa, wb) = (c_w(&w_nul(a)), c_w(&w_nul(b)));
                let flags = if ic { "1" } else { "0" };
                p.ret("CompareStringW", &[lcid, flags, &wa, "-1", &wb, "-1"], want, E_NONE);
                // explicit lengths without the NUL give the same answer
                let (la, lb) = (a.len().to_string(), b.len().to_string());
                p.ret("CompareStringW", &[lcid, flags, &wa, &la, &wb, &lb], want, E_NONE);
            }
        }
    }
    // embedded NUL with explicit lengths: equal as written
    let e = c_w(&[0x61, 0, 0x62]);
    p.ret("CompareStringW", &["0x409", "0", &e, "3", &e, "3"], 2, E_NONE);
    // a flag bit the page does not define, NULL strings
    let a = c_w(&w_nul("a"));
    p.ret("CompareStringW", &["0x409", "0x40", &a, "-1", &a, "-1"], 0, E_FLAGS);
    p.ret("CompareStringW", &["0x409", "0", "0", "-1", &a, "-1"], 0, E_PARAM);
    p.ret("CompareStringW", &["0x409", "0", &a, "-1", "0", "-1"], 0, E_PARAM);
    p.run("locale-compare");
}

/// Decision 0126 (wall 72, S-cmp-prefix, a choice without a source): the guest compares ".FLMS" with ".FLM". The page CompareStringEx gives no rule for two strings of which one is a prefix of the other; the shorter one
/// is the smaller (a lexicographic order puts a prefix before the strings it begins): CSTR_LESS_THAN (1) when string 1 is the shorter, CSTR_GREATER_THAN (3) when it is the longer, CSTR_EQUAL (2) when the strings
/// are equal. Old: these pairs stopped (rows "cmp-prefix" and "symbol-empty" of the abort tests). A case-only difference of strings of the same length without NORM_IGNORECASE still stops (cmp-case-only).
/// Also wall 62 / 68 (S-cmp-equal-prefix, S-cmp-equal-hyphen): a common prefix that holds a space, a hyphen or an apostrophe does not decide the order; the first difference after it does.
#[test]
fn compare_string_one_a_prefix_of_the_other_is_the_shorter_first() {
    let mut p = Prog::new();
    let rows = [
        ("ab", "abc", 1u64), ("abc", "ab", 3), (".FLM", ".FLMS", 1), (".FLMS", ".FLM", 3), ("_", "", 3), ("", "_", 1), ("a", "ab", 1), ("ab", "a", 3),
        // a space, a hyphen and an apostrophe in a common prefix (walls 62, 68): decided by the first difference after it
        ("a b", "a c", 1), ("a c", "a b", 3), ("a-b", "a-c", 1), ("a-c", "a-b", 3), ("a'b", "a'c", 1), ("Test message processing", "Test program expiration", 1),
        ("Test program expiration", "Test message processing", 3),
    ];
    for lcid in ["0x409", "0x400", "0x800"] {
        for flags in ["0", "1"] {
            for (a, b, want) in rows {
                let (wa, wb) = (c_w(&w_nul(a)), c_w(&w_nul(b)));
                p.ret("CompareStringW", &[lcid, flags, &wa, "-1", &wb, "-1"], want, E_NONE);
            }
        }
    }
    p.run("locale-compare-prefix");
}

/// Decision 0072: word sort puts punctuation and other nonalphanumeric characters (except the hyphen and the
/// apostrophe) before alphanumeric characters ("Handling Sorting in Your Applications"). Decided only where the
/// first units (position 0) differ and one is a printable ASCII symbol and the other an ASCII letter or digit.
#[test]
fn compare_string_symbol_before_alnum() {
    let mut p = Prog::new();
    // (string1, string2, expected). The first two rows are the pair of unit 144 (proc 0x65c540).
    let rows = [
        ("_x64.dll", "ppv2.dll", 1u64), ("ppv2.dll", "_x64.dll", 3), ("_x64.dll", "ache.dll", 1),
        ("ache.dll", "_x64.dll", 3), ("_", "a", 1), ("a", "_", 3), (".", "a", 1), ("a", ".", 3), ("_", "0", 1),
        ("9", "_", 3), ("!", "Z", 1), ("Z", "!", 3), ("{", "z", 1), ("~", "A", 1), ("_", "ab", 1), ("ab", "_", 3),
        ("_ab", "c", 1), ("c", "_ab", 3),
    ];
    for lcid in ["0x409", "0x400", "0x800"] {
        for flags in ["0", "1"] {
            for (a, b, want) in rows {
                let (wa, wb) = (c_w(&w_nul(a)), c_w(&w_nul(b)));
                p.ret("CompareStringW", &[lcid, flags, &wa, "-1", &wb, "-1"], want, E_NONE);
                let (la, lb) = (a.len().to_string(), b.len().to_string());
                p.ret("CompareStringW", &[lcid, flags, &wa, &la, &wb, &lb], want, E_NONE);
            }
        }
    }
    p.run("locale-compare-symbol");
}

/// Decision 0129, walls 7, 8, 13 (old: these eight pairs stopped, see compare_string_symbol_outside_rule_aborts). Expected values are written from the rule, not read from the runtime. Learn "Handling Sorting in Your
/// Applications": punctuation and other nonalphanumeric characters (the space included) come before alphanumerics (word sort), at the first difference wherever it is (S-cmp-symbol-any-position); two different symbols
/// by code point (S-cmp-symbol-vs-symbol, a CHOICE WITHOUT A SOURCE); the space is a symbol (S-cmp-space-as-symbol). 1 = CSTR_LESS_THAN, 3 = CSTR_GREATER_THAN.
#[test]
fn compare_string_symbol_at_any_position_two_symbols_and_the_space() {
    let mut p = Prog::new();
    let rows = [
        ("a_b", "ab", 1u64), ("ab", "a_b", 3), ("a.b", "ac", 1), ("Ab_", "aBc", 1),   // '_' / '.' against a letter at position 1 or 2: the symbol is the smaller
        ("_", ".", 3), (".", "_", 1),                                                   // '_' (0x5f) against '.' (0x2e): by code point
        (" ", "a", 1), ("a", " ", 3), (" ", "_", 1), ("_", " ", 3), ("Default.tpr", "Default (macOS).tpr", 3), // the space is the smallest symbol; '.' (0x2e) after ' ' (0x20)
    ];
    for flags in ["0", "1"] {
        for (a, b, want) in rows {
            let (wa, wb) = (c_w(&w_nul(a)), c_w(&w_nul(b)));
            p.ret("CompareStringW", &["0x409", flags, &wa, "-1", &wb, "-1"], want, E_NONE);
        }
    }
    p.run("locale-compare-symbol-any-position");
}

/// Decision 0229 (ref1, PROVISIONAL; S-ref1-cmp-hyphen-default). Old: the pairs of this test stopped (decisions 0072 and 0129 listed the hyphen and the apostrophe against a letter or a digit as "not on the pages").
/// The page Learn "Handling Sorting in Your Applications" (https://learn.microsoft.com/en-us/windows/win32/intl/handling-sorting-in-your-applications, read 2026-11-04) says of the default "word sort" (the one of
/// CompareString and CompareStringEx without SORT_STRINGSORT): "all punctuation marks and other nonalphanumeric characters, except for the hyphen and the apostrophe, come before any alphanumeric character. The hyphen
/// and the apostrophe are treated differently from the other nonalphanumeric characters to ensure that words such as "coop" and "co-op" stay together in a sorted list.", and lists the word sort of nine words in this
/// order: billet, bills, bill's, cannot, cant, can't, con, coop, co-op. The expected values are written from that list and from the reading that the runtime states (a hyphen or apostrophe does not take part in the
/// comparison of the other characters; of two strings that are equal without them the one with more of them is the greater): every adjacent pair of the list, the pairs of the list in both orders, and the
/// hyphen / apostrophe against a letter or a digit. The page does not quote that reading, only the list and the sentence: the values for a pair that the list does not contain ("-" against "a", "-x" against "a")
/// follow the reading. 1 = CSTR_LESS_THAN, 3 = CSTR_GREATER_THAN. The code-point order of a unit above U+007F against ASCII has no source and stays behind FLE_CMP_EXTEND (see compare_string_symbol_outside_rule_aborts).
#[test]
fn compare_string_hyphen_and_apostrophe_follow_the_word_sort() {
    let mut p = Prog::new();
    let list = ["billet", "bills", "bill's", "cannot", "cant", "can't", "con", "coop", "co-op"];
    let mut rows: Vec<(String, String, u64)> = Vec::new();
    for i in 0..list.len() {
        for j in 0..list.len() {
            if i != j {
                rows.push((list[i].to_string(), list[j].to_string(), if i < j { 1 } else { 3 }));
            }
        }
    }
    for (a, b, want) in [("-", "a", 1u64), ("a", "-", 3), ("-", "0", 1), ("0", "-", 3), ("-x", "a", 3), ("'", "a", 1), ("a", "'", 3), ("'", "0", 1)] {
        rows.push((a.to_string(), b.to_string(), want));
    }
    for flags in ["0", "1"] {
        for (a, b, want) in &rows {
            let (wa, wb) = (c_w(&w_nul(a)), c_w(&w_nul(b)));
            p.ret("CompareStringW", &["0x409", flags, &wa, "-1", &wb, "-1"], *want, E_NONE);
        }
    }
    p.run("locale-compare-hyphen-apostrophe-word-sort");
}

/// Decision 0284 (B; decision 0191, wall ia2): the sorting weights of Microsoft's published Sorting Weight Table (Windows10-SortingWeightTable.txt, SHA-256 81fcfa1e...) are in every build. The expected values are
/// written from the table rows (Script Member, Alphabetic Weight; the primary weight is SM then AW): U+00E9 is 14/33 ("e Acute"), "a" is 14/2, so U+00E9 is greater than "a" (3); U+2022 "Bullet" is 11/39, a symbol
/// (SM 11 < 14), so it is less than "a" (1). 1 = CSTR_LESS_THAN, 3 = CSTR_GREATER_THAN. Old: these two pairs stopped (compare_string_symbol_outside_rule_aborts).
#[test]
fn compare_string_nonascii_units_by_the_sorting_weight_table() {
    let mut p = Prog::new();
    for (a, b, want) in [(0xe9u16, 'a' as u16, 3u64), ('a' as u16, 0xe9, 1), (0x2022, 'a' as u16, 1), ('a' as u16, 0x2022, 3)] {
        p.ret("CompareStringW", &["0x409", "0", &c_w(&[a, 0]), "-1", &c_w(&[b, 0]), "-1"], want, E_NONE);
    }
    p.run("locale-compare-nonascii-by-table");
}

/// Decision 0072: what the rule above does not cover still stops: a symbol after position 0, a symbol against a
/// symbol, the hyphen and the apostrophe, a space, a symbol against an empty string, a non-ASCII character, a
/// flag other than 0 and NORM_IGNORECASE, and another locale.
#[test]
fn compare_string_symbol_outside_rule_aborts() {
    let w = |s: &str| c_w(&w_nul(s));
    let rows: Vec<(&str, String)> = vec![
        // decision 0129 (old: the rows after-pos0-mid, after-pos0-mid-rev, after-pos0-dot, after-case-diff, after-case-diff-ic, symbol-symbol, symbol-symbol-rev and space-letter stopped here): walls 7, 8 and 13
        // (S-cmp-symbol-any-position, S-cmp-symbol-vs-symbol, S-cmp-space-as-symbol) answer them; see compare_string_symbol_at_any_position_two_symbols_and_the_space below.
        // decision 0229 (ref1, S-ref1-cmp-hyphen-default): old: the rows hyphen-letter, letter-hyphen, hyphen-digit, hyphen-letter-ic, apostrophe-letter and letter-apostrophe stopped here (decisions 0072, 0129: the
        // hyphen and the apostrophe against a letter or a digit were not on the pages). The word sort's treatment of the two characters is on the Learn page "Handling Sorting in Your Applications" and is the default
        // now; see compare_string_hyphen_and_apostrophe_follow_the_word_sort below.
        // decision 0126 (wall 72, S-cmp-prefix): old: ("symbol-empty", "_" against "") stopped here; a string that is a prefix of the other (the empty string is a prefix of every string) now gives
        // CSTR_LESS_THAN / CSTR_GREATER_THAN by length: see compare_string_one_a_prefix_of_the_other_is_the_shorter_first below.
        // decision 0126 (wall 72, S-cmp-prefix): old: ("symbol-prefix", "_" against "_a") stopped; "_" is a prefix of "_a" now, the shorter string is the smaller (tests/locale.rs compare_string_one_a_prefix_...)
        // decision 0284 (B): old: the rows nonascii-letter (U+00E9 against "a") and nonascii-symbol-letter (U+2022 against "a") stopped here; the wall ia2 weights (Microsoft's Sorting Weight Table) answer them in every build now,
        // see compare_string_nonascii_units_by_the_sorting_weight_table below.
        ("flags-ignorenonspace", format!("RUN(fle_imp_CompareStringW, 0x409, 2, {}, -1, {}, -1, 0, 0);", w("_"), w("a"))),
        ("flags-stringsort", format!("RUN(fle_imp_CompareStringW, 0x409, 0x1000, {}, -1, {}, -1, 0, 0);", w("_"), w("a"))),
        ("locale-german", format!("RUN(fle_imp_CompareStringW, 0x407, 0, {}, -1, {}, -1, 0, 0);", w("_"), w("a"))),
        ("locale-invariant", format!("RUN(fle_imp_CompareStringW, 0x7f, 0, {}, -1, {}, -1, 0, 0);", w("_"), w("a"))),
    ];
    for (name, body) in rows {
        let src = format!("{PRE}int main(void)\n{{\n    setup(0, 0, 0, 0, 0, 0, 0, 0);\n    {body}\n    printf(\"returned %llx\\n\", (unsigned long long)R[0]);\n    return 0;\n}}\n");
        let exe = clang(&format!("abort-sym-{name}"), &src);
        let run = Command::new(&exe).output().unwrap();
        assert!(!run.status.success(), "{name}: expected abort, stdout: {}", String::from_utf8_lossy(&run.stdout));
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{name}");
        assert!(String::from_utf8_lossy(&run.stderr).contains("fle:"), "{name}: {}", String::from_utf8_lossy(&run.stderr));
        assert_eq!(run.status.signal(), Some(6), "{name}");
    }
}

// ---- 4. Decision 0087 (provisional): GetStringTypeW(CT_CTYPE1), LCMapStringW(0x100/0x200), WideCharToMultiByte(1252) for the range D ----

/// The rows of a data file in runtime/test (comment lines and blank lines dropped), split at white space.
fn rt_test_rows(name: &str) -> Vec<Vec<String>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime/test").join(name);
    std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split_whitespace().map(|s| s.to_string()).collect())
        .collect()
}
fn hx(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

/// unit -> (upper, lower) for the 284 units of D (case_ms8_d.txt: MS8 upper case table, lower case = its inverse).
fn d_case() -> BTreeMap<u16, (u16, u16)> {
    rt_test_rows("case_ms8_d.txt").iter().map(|r| (hx(&r[0]) as u16, (hx(&r[1]) as u16, hx(&r[2]) as u16))).collect()
}
/// unit -> C1 value, Wine 10.0 en-US (unofficial; c1_wine_10_0_enus.txt).
fn c1_wine() -> BTreeMap<u16, u16> {
    rt_test_rows("c1_wine_10_0_enus.txt").iter().map(|r| (hx(&r[0]) as u16, hx(&r[1]) as u16)).collect()
}
/// unit -> (UPPER, LOWER) from the UCD 16.0.0 (c1_ucd16_upper_lower.txt).
fn c1_ucd() -> BTreeMap<u16, (u16, u16)> {
    rt_test_rows("c1_ucd16_upper_lower.txt").iter().map(|r| (hx(&r[0]) as u16, (hx(&r[1]) as u16, hx(&r[2]) as u16))).collect()
}
struct Wct {
    default: u8,
    rows: BTreeMap<u16, (u8, bool)>, // unit -> (byte, exact inverse of MBTABLE)
}
fn wctable() -> Wct {
    let mut w = Wct { default: 0, rows: BTreeMap::new() };
    for r in rt_test_rows("wctable_1252_expect.txt") {
        if r[0] == "DEFAULTCHAR" {
            w.default = hx(&r[1]) as u8;
            continue;
        }
        assert!(w.rows.insert(hx(&r[0]) as u16, (hx(&r[1]) as u8, r[2] == "1")).is_none(), "unit {} twice", r[0]);
    }
    w
}
/// The 256 units ATTACH passes to GetStringTypeW and LCMapStringW: bytes 0x20, 0x01..0xFF decoded with MBTABLE
/// (investigation dsp-ippv2-ctype-locale-data.md 3.1).
fn attach_input() -> Vec<u16> {
    let e = cp1252_expect();
    let mut v = vec![0x20u16];
    v.extend_from_slice(&e[1..]);
    v
}
/// WideCharToMultiByte(1252) of one unit of D: (byte, default character used). A row converts; a best-fit row only without
/// WC_NO_BEST_FIT_CHARS (0x400); otherwise the default character.
fn wct_expect(w: &Wct, u: u16, flags: u32, defb: u8) -> (u8, bool) {
    match w.rows.get(&u) {
        Some(&(b, exact)) if exact || flags == 0 => (b, false),
        _ => (defb, true),
    }
}

#[test]
fn range_d_data_files_agree_with_each_other_and_with_the_attach_input() {
    let (d, wine, ucd, w, exp) = (d_case(), c1_wine(), c1_ucd(), wctable(), cp1252_expect());
    // D = U+0000..U+00FF + 28 units above (the 27 of ATTACH and U+0191)
    assert_eq!(d.len(), 284);
    assert!((0..=255u16).all(|u| d.contains_key(&u)));
    let high: Vec<u16> = d.keys().copied().filter(|&u| u > 0xFF).collect();
    assert_eq!(
        high,
        [0x152, 0x153, 0x160, 0x161, 0x178, 0x17d, 0x17e, 0x191, 0x192, 0x2c6, 0x2dc, 0x2013, 0x2014, 0x2018, 0x2019, 0x201a, 0x201c, 0x201d, 0x201e, 0x2020, 0x2021, 0x2022, 0x2026, 0x2030, 0x2039, 0x203a, 0x20ac, 0x2122]
    );
    // ATTACH's 256 units: 255 distinct (U+0020 twice), 228 at or below U+00FF, 27 above, no U+0000, no U+0191, all in D
    let att = attach_input();
    assert_eq!(att.len(), 256);
    let mut dist = att.clone();
    dist.sort();
    dist.dedup();
    assert_eq!(dist.len(), 255);
    assert_eq!(dist.iter().filter(|&&u| u <= 0xFF).count(), 228);
    assert_eq!(dist.iter().filter(|&&u| u > 0xFF).count(), 27);
    assert!(att.iter().all(|u| d.contains_key(u)) && !att.contains(&0) && !att.contains(&0x191));
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-ctype/tables/in-units.txt");
    if let Ok(t) = std::fs::read_to_string(&p) {
        let from_investigation: Vec<u16> = t.lines().map(|l| hx(l.split_whitespace().nth(1).unwrap()) as u16).collect();
        assert_eq!(from_investigation, att, "the measured ATTACH input of the investigation");
    } else {
        eprintln!("note: {} not present (gitignored work file); the measured ATTACH input is not compared", p.display());
    }
    // closure: upper and lower of every unit of D are in D; the 61 MS8 rows are inside D; U+00B5, U+00DF, U+00AA, U+00BA do not change
    for (&u, &(up, lo)) in &d {
        assert!(d.contains_key(&up) && d.contains_key(&lo), "U+{u:04X}");
        if up != u {
            assert_eq!(d[&up].1, u, "the lower case map is the inverse of the upper case map: U+{u:04X}");
        }
    }
    let all: Vec<(u16, u16, u16)> = d.iter().map(|(&u, &(up, lo))| (u, up, lo)).collect();
    assert_eq!(all.iter().filter(|&&(_, up, lo)| up != lo).count(), 61 + 61, "61 units change by upper, 61 by lower, none by both");
    assert_eq!(all.iter().filter(|&&(u, up, _)| up != u).count(), 61);
    assert_eq!(all.iter().filter(|&&(u, _, lo)| lo != u).count(), 61);
    for u in [0xB5u16, 0xDF, 0xAA, 0xBA] {
        assert_eq!(d[&u], (u, u), "U+{u:04X}");
    }
    assert_eq!((d[&0xFF].0, d[&0x192].0, d[&0x191].1, d[&0x178].1), (0x178, 0x191, 0x192, 0xFF));
    // C1: Wine has 283 units (D without U+0191, which nobody measured); UPPER and LOWER equal the UCD's
    assert_eq!(wine.len(), 283);
    assert!(!wine.contains_key(&0x191) && wine.keys().all(|u| d.contains_key(u)));
    assert!(wine.keys().eq(ucd.keys()));
    for (u, v) in &wine {
        assert_eq!(((v & 1), (v >> 1) & 1), ucd[u], "UPPER/LOWER of U+{u:04X}: Wine {v:#06x}");
        assert_eq!(v & 3 == 3, false, "U+{u:04X} is both UPPER and LOWER");
        assert_eq!(v & 0x200, 0x200, "DEFINED on every unit (Wine)");
    }
    // WCTABLE: 698 rows, default char 0x3f, 256 exact rows = the inverse of MBTABLE, 257 rows inside D, 441 outside
    assert_eq!((w.rows.len(), w.default), (698, 0x3f));
    assert_eq!(w.rows.values().filter(|r| r.1).count(), 256);
    for b in 0..256usize {
        assert_eq!(w.rows[&exp[b]], (b as u8, true), "byte {b:#x} -> U+{:04X} and back", exp[b]);
    }
    assert_eq!(w.rows.keys().copied().filter(|u| d.contains_key(u)).count(), 257);
    assert_eq!(w.rows.keys().copied().filter(|u| !d.contains_key(u)).count(), 441);
    let best_fit_in_d: Vec<(u16, u8)> = w.rows.iter().map(|(&u, &(b, exact))| (u, b, exact)).filter(|&(u, _, exact)| d.contains_key(&u) && !exact).map(|(u, b, _)| (u, b)).collect();
    assert_eq!(best_fit_in_d, [(0x191, 0x83)]);
    let no_row: Vec<u16> = d.keys().copied().filter(|u| !w.rows.contains_key(u)).collect();
    assert_eq!(no_row.len(), 27);
    assert!(no_row.iter().all(|&u| (0x80..0xA0).contains(&u) && ![0x81, 0x8d, 0x8f, 0x90, 0x9d].contains(&u)));
}

/// C source that makes a stop observable and survivable: the program defines its own `abort` (the runtime's calls bind to it at link time)
/// that jumps back, and stderr goes to a memory buffer, so one program can try many calls and print what each did. (glibc's own abort
/// cannot be survived twice: it advances a static stage and the second call removes the handler.) That the stops are real SIGABRTs is
/// tested separately, in `decision_0087_stops_are_real_aborts`.
const PRE_TRY: &str = r#"#include <setjmp.h>
static sigjmp_buf jb;
static char emsg[1024];
void abort(void) { siglongjmp(jb, 1); }
#define TRY_ABORT(stmt, aborted) do { FILE *sv_ = stderr; FILE *f_; memset(emsg, 0, sizeof emsg); f_ = fmemopen(emsg, sizeof emsg - 1, "w"); \
    aborted = 0; stderr = f_; if (sigsetjmp(jb, 1) == 0) { stmt; } else { aborted = 1; } fflush(f_); stderr = sv_; fclose(f_); } while (0)
#define EXPECT_ABORT(label, needle, stmt) do { volatile int ab_; TRY_ABORT(stmt, ab_); printf("%s %d %d\n", label, ab_, strstr(emsg, needle) != NULL); } while (0)
"#;

fn try_prog(name: &str, body: &str) -> Vec<String> {
    let src = format!("{PRE}{PRE_TRY}int main(void)\n{{\n{body}    return 0;\n}}\n");
    let exe = clang(name, &src);
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{name}: {:?} {}", run.status, String::from_utf8_lossy(&run.stderr));
    String::from_utf8_lossy(&run.stdout).lines().map(|s| s.to_string()).collect()
}

/// Calls that must stop by name ("<label> 1 1": aborted, and the stderr text has the needle), and plain lines.
struct Stops {
    body: String,
    want: Vec<String>,
}
impl Stops {
    fn new() -> Self {
        Stops { body: String::new(), want: Vec::new() }
    }
    fn abort(&mut self, label: &str, needle: &str, stmt: &str) {
        self.body += &format!("    EXPECT_ABORT(\"{label}\", \"{needle}\", {stmt});\n");
        self.want.push(format!("{label} 1 1"));
    }
    fn plain(&mut self, c: &str, want: &str) {
        self.body += &format!("    {c}\n");
        self.want.push(want.to_string());
    }
    fn run(&self, name: &str) {
        let got = try_prog(name, &self.body);
        assert_eq!(got, self.want, "{name}");
    }
}

#[test]
fn gstw_c1_every_unit_of_d_one_by_one() {
    let wine = c1_wine();
    let mut p = Prog::new();
    for (&u, &v) in &wine {
        // one unit, an explicit count of 1: one word is written, the next one is not touched; success sets no error
        p.call("fillw(wbuf, 4);", "GetStringTypeW", &["1", &c_w(&[u]), "1", "P(wbuf)"], "dumpw(wbuf, 2);", &[line(1, E_NONE), wl(&[v, 0x5555])]);
    }
    p.run("locale-0087-gstw-each");
}

#[test]
fn gstw_c1_all_at_once_the_attach_input_and_the_page_rules() {
    let wine = c1_wine();
    let ucd = c1_ucd();
    let mut p = Prog::new();
    // all 283 units at once (cchSrc 283)
    let units: Vec<u16> = wine.keys().copied().collect();
    let mut want: Vec<u16> = units.iter().map(|u| wine[u]).collect();
    want.push(0x5555);
    p.call("fillw(dbw2, 320);", "GetStringTypeW", &["1", &c_w(&units), "283", "P(dbw2)"], "dumpw(dbw2, 284);", &[line(1, E_NONE), wl(&want)]);
    // UPPER and LOWER of the runtime's answer equal the UCD 16.0.0 (Lu, Lowercase), from the UCD file and not from the Wine file
    let bits: Vec<String> = units.iter().map(|u| format!("{:x}", ucd[u].0 | (ucd[u].1 << 1))).collect();
    p.call(
        "fillw(dbw2, 320);",
        "GetStringTypeW",
        &["1", &c_w(&units), "283", "P(dbw2)"],
        "{ int i; printf(\"BITS\"); for (i = 0; i < 283; i++) printf(\" %x\", dbw2[i] & 3); printf(\"\\n\"); }",
        &[line(1, E_NONE), format!("BITS {}", bits.join(" "))],
    );
    // the exact input of ATTACH (cchSrc 256, dwInfoType 1): no stop, the Wine values, nothing after the 256th word
    let att = attach_input();
    let mut want: Vec<u16> = att.iter().map(|u| wine[u]).collect();
    want.push(0x5555);
    p.call("fillw(dbw2, 320);", "GetStringTypeW", &["1", &c_w(&att), "256", "P(dbw2)"], "dumpw(dbw2, 257);", &[line(1, E_NONE), wl(&want)]);
    // a negative cchSrc: NUL-terminated, the NUL is typed too (the page); any negative value
    let s = [0x41u16, 0x20, 0x31, 0xe9, 0];
    let mut want: Vec<u16> = s.iter().map(|u| wine[u]).collect();
    want.push(0x5555);
    for cch in ["-1", "-2", "-100"] {
        p.call("fillw(dbw2, 16);", "GetStringTypeW", &["1", &c_w(&s), cch, "P(dbw2)"], "dumpw(dbw2, 6);", &[line(1, E_NONE), wl(&want)]);
    }
    // an explicit count without the NUL: exactly that many words (the page: "exactly the specified number")
    let mut want: Vec<u16> = s[..4].iter().map(|u| wine[u]).collect();
    want.push(0x5555);
    p.call("fillw(dbw2, 16);", "GetStringTypeW", &["1", &c_w(&s), "4", "P(dbw2)"], "dumpw(dbw2, 5);", &[line(1, E_NONE), wl(&want)]);
    // an explicit count that includes the NUL types it; an embedded NUL with an explicit count is one character
    let mut want: Vec<u16> = s.iter().map(|u| wine[u]).collect();
    want.push(0x5555);
    p.call("fillw(dbw2, 16);", "GetStringTypeW", &["1", &c_w(&s), "5", "P(dbw2)"], "dumpw(dbw2, 6);", &[line(1, E_NONE), wl(&want)]);
    let e = [0x61u16, 0, 0x62];
    let mut want: Vec<u16> = e.iter().map(|u| wine[u]).collect();
    want.push(0x5555);
    p.call("fillw(dbw2, 16);", "GetStringTypeW", &["1", &c_w(&e), "3", "P(dbw2)"], "dumpw(dbw2, 4);", &[line(1, E_NONE), wl(&want)]);
    // lpSrcStr and lpCharType the same pointer: ERROR_INVALID_PARAMETER, nothing else
    p.ret("GetStringTypeW", &["1", "P(dbw2)", "4", "P(dbw2)"], 0, E_PARAM);
    p.ret("GetStringTypeW", &["1", "P(dbw2)", "-1", "P(dbw2)"], 0, E_PARAM);
    p.run("locale-0087-gstw-all");
}

#[test]
fn gstw_stops_by_name_outside_the_decision_and_writes_nothing() {
    let wine = c1_wine();
    let mut s = Stops::new();
    // every dwInfoType other than 1: CT_CTYPE2 (2), CT_CTYPE3 (4), and the values the page does not list
    for ty in ["2", "4", "0", "3", "5", "6", "7", "8", "0x100", "0x10001", "0xffffffff"] {
        s.abort(&format!("gstw-type-{ty}"), "GetStringTypeW: dwInfoType other than CT_CTYPE1", &format!("RUN(fle_imp_GetStringTypeW, {ty}, {}, 1, P(wbuf), 0, 0, 0, 0)", c_w(&[0x61, 0])));
    }
    // inputs the page gives no rule for: a NULL pointer, cchSrc 0
    let a = c_w(&[0x61, 0]);
    let needle = "GetStringTypeW: NULL lpSrcStr or lpCharType, or cchSrc 0";
    s.abort("gstw-null-src", needle, "RUN(fle_imp_GetStringTypeW, 1, 0, 1, P(wbuf), 0, 0, 0, 0)");
    s.abort("gstw-null-out", needle, &format!("RUN(fle_imp_GetStringTypeW, 1, {a}, 1, 0, 0, 0, 0, 0)"));
    s.abort("gstw-cch-0", needle, &format!("RUN(fle_imp_GetStringTypeW, 1, {a}, 0, P(wbuf), 0, 0, 0, 0)"));
    // U+0191 is in D but nobody measured its C1 value: no value is made up
    s.abort(
        "gstw-u0191",
        "GetStringTypeW: U+0191 is in the range D of decision 0087 but has no C1 value",
        &format!("RUN(fle_imp_GetStringTypeW, 1, {}, 1, P(wbuf), 0, 0, 0, 0)", c_w(&[0x191])),
    );
    // a string whose later unit is the problem stops before any word is written
    s.abort("gstw-late-outside", "GetStringTypeW: U+4E2D is outside the range D of decision 0087", &format!("RUN(fle_imp_GetStringTypeW, 1, {}, 3, P(wbuf), 0, 0, 0, 0)", c_w(&[0x41, 0x4e2d, 0x42, 0])));
    s.abort("gstw-late-outside-nul", "GetStringTypeW: U+0100 is outside the range D of decision 0087", &format!("RUN(fle_imp_GetStringTypeW, 1, {}, -1, P(wbuf), 0, 0, 0, 0)", c_w(&[0x41, 0x42, 0x100, 0])));
    s.run("locale-0087-gstw-stops");
    // every one of the 65,536 units, in the middle of "a?b": outside D (or U+0191) stops with the unit named, nothing written; D answers
    let sweep = r#"
    {
        unsigned u; int ab = 0, msg_out = 0, msg_none = 0, unchanged = 0, ok = 0, k;
        char want[160];
        for (u = 0; u < 0x10000; u++) {
            uint16_t s3[3] = { 0x61, (uint16_t)u, 0x62 };
            volatile int a;
            fillw(wbuf, 8);
            TRY_ABORT(RUN(fle_imp_GetStringTypeW, 1, P(s3), 3, P(wbuf), 0, 0, 0, 0), a);
            if (a) {
                ab++;
                for (k = 0; k < 8 && wbuf[k] == 0x5555; k++) {}
                if (k == 8) unchanged++;
                snprintf(want, sizeof want, "GetStringTypeW: U+%04X is outside the range D of decision 0087", u);
                if (strstr(emsg, want)) msg_out++;
                else if (u == 0x191 && strstr(emsg, "GetStringTypeW: U+0191 is in the range D of decision 0087 but has no C1 value")) msg_none++;
            } else if (R[0] == 1 && wbuf[0] != 0x5555 && wbuf[1] != 0x5555 && wbuf[2] != 0x5555 && wbuf[3] == 0x5555) ok++;
        }
        printf("gstw %d %d %d %d %d\n", ab, msg_out, msg_none, unchanged, ok);
    }
"#;
    let got = try_prog("locale-0087-gstw-sweep", sweep);
    let (outside, with_value) = (65536 - 284, wine.len());
    assert_eq!(got, [format!("gstw {} {} 1 {} {}", outside + 1, outside, outside + 1, with_value)]);
}

#[test]
fn lcmap_range_d_every_unit_for_locale_zero_and_the_defaults() {
    let d = d_case();
    let units: Vec<u16> = d.keys().copied().collect();
    let upper: Vec<u16> = units.iter().map(|u| d[u].0).collect();
    let lower: Vec<u16> = units.iter().map(|u| d[u].1).collect();
    let src = c_w(&units);
    // for a negative length the source must end at its first NUL: D has U+0000 as its first unit, so the NUL-terminated
    // source is the 283 other units and a NUL (the mapped string is then 284 characters with the NUL, 283 + 1)
    let mut nul_units: Vec<u16> = units[1..].to_vec();
    nul_units.push(0);
    let src_nul = c_w(&nul_units);
    let mut p = Prog::new();
    for lcid in ["0", "0x409", "0x400", "0x800", "0x7f"] {
        for (flag, want) in [("0x200", &upper), ("0x100", &lower)] {
            // size query: the number of characters, nothing is used
            p.ret("LCMapStringW", &[lcid, flag, &src, "284", "0", "0"], 284, E_NONE);
            // the whole range at once; nothing after it is touched
            let mut w = want.clone();
            w.push(0x5555);
            p.call("fillw(dbw2, 320);", "LCMapStringW", &[lcid, flag, &src, "284", "P(dbw2)", "300"], "dumpw(dbw2, 285);", &[line(284, E_NONE), wl(&w)]);
            // exactly enough fits, one less is ERROR_INSUFFICIENT_BUFFER and the buffer keeps its content
            let mut w = want.clone();
            w.push(0x5555);
            p.call("fillw(dbw2, 320);", "LCMapStringW", &[lcid, flag, &src, "284", "P(dbw2)", "284"], "dumpw(dbw2, 285);", &[line(284, E_NONE), wl(&w)]);
            p.call("fillw(dbw3, 320);", "LCMapStringW", &[lcid, flag, &src, "284", "P(dbw3)", "283"], "dumpw(dbw3, 8);", &[line(0, E_BUF), wl(&[0x5555; 8])]);
            p.call("fillw(dbw3, 320);", "LCMapStringW", &[lcid, flag, &src, "284", "0", "283"], "", &[line(0, E_BUF)]);
            // a negative length: NUL-terminated, the NUL is mapped and counted (283 units + the NUL = 284)
            let mut w = want[1..].to_vec();
            w.push(0);
            w.push(0x5555);
            p.ret("LCMapStringW", &[lcid, flag, &src_nul, "-1", "0", "0"], 284, E_NONE);
            p.call("fillw(dbw2, 320);", "LCMapStringW", &[lcid, flag, &src_nul, "-1", "P(dbw2)", "300"], "dumpw(dbw2, 285);", &[line(284, E_NONE), wl(&w)]);
            p.ret("LCMapStringW", &[lcid, flag, &src_nul, "-5", "P(dbw2)", "283"], 0, E_BUF);
            p.ret("LCMapStringW", &[lcid, flag, &src_nul, "-1", "P(dbw2)", "284"], 284, E_NONE);
            // the page: the source may not be NULL or of size 0, the destination size may not be negative
            p.ret("LCMapStringW", &[lcid, flag, "0", "-1", "0", "0"], 0, E_PARAM);
            p.ret("LCMapStringW", &[lcid, flag, &src, "0", "0", "0"], 0, E_PARAM);
            p.ret("LCMapStringW", &[lcid, flag, &src, "-1", "P(dbw2)", "-1"], 0, E_PARAM);
        }
    }
    // one unit at a time, for Locale 0 and en-US
    for lcid in ["0", "0x409"] {
        for (flag, which) in [("0x200", 0usize), ("0x100", 1)] {
            for &u in &units {
                let m = if which == 0 { d[&u].0 } else { d[&u].1 };
                p.call("fillw(wbuf, 4);", "LCMapStringW", &[lcid, flag, &c_w(&[u]), "1", "P(wbuf)", "4"], "dumpw(wbuf, 2);", &[line(1, E_NONE), wl(&[m, 0x5555])]);
            }
        }
    }
    // the page: "the destination string can use the same buffer as the source string"
    p.call(
        "{ static uint16_t ip[8] = { 233, 65, 8364, 402, 255, 0, 0, 0 };",
        "LCMapStringW",
        &["0", "0x200", "P(ip)", "5", "P(ip)", "5"],
        "dumpw(ip, 5); }",
        &[line(5, E_NONE), wl(&[0xc9, 0x41, 0x20ac, 0x191, 0x178])],
    );
    p.call(
        "{ static uint16_t ip[8] = { 201, 65, 8364, 401, 376, 0, 0, 0 };",
        "LCMapStringW",
        &["0", "0x100", "P(ip)", "5", "P(ip)", "5"],
        "dumpw(ip, 5); }",
        &[line(5, E_NONE), wl(&[0xe9, 0x61, 0x20ac, 0x192, 0xff])],
    );
    // the thread locale was not changed by Locale 0
    p.ret("GetThreadLocale", &[], 0x409, E_NONE);
    p.run("locale-0087-lcmap-d");
}

#[test]
fn lcmap_u00b5_is_unchanged_and_the_ascii_rows_still_work_with_locale_zero() {
    // MS8: U+00B5 and U+00DF are not in the table (not upper-cased); the UCD would map U+00B5 to U+039C
    let mut p = Prog::new();
    for (u, up, lo) in [(0xb5u16, 0xb5u16, 0xb5u16), (0xdf, 0xdf, 0xdf), (0xaa, 0xaa, 0xaa), (0xba, 0xba, 0xba), (0x178, 0x178, 0xff), (0xff, 0x178, 0xff)] {
        p.call("fillw(wbuf, 4);", "LCMapStringW", &["0", "0x200", &c_w(&[u]), "1", "P(wbuf)", "4"], "dumpw(wbuf, 2);", &[line(1, E_NONE), wl(&[up, 0x5555])]);
        p.call("fillw(wbuf, 4);", "LCMapStringW", &["0", "0x100", &c_w(&[u]), "1", "P(wbuf)", "4"], "dumpw(wbuf, 2);", &[line(1, E_NONE), wl(&[lo, 0x5555])]);
    }
    let s = "Hello, World! i I 123 {}[]@`";
    let wn = w_nul(s);
    let up: Vec<u16> = w_nul(&s.to_ascii_uppercase());
    let lo: Vec<u16> = w_nul(&s.to_ascii_lowercase());
    p.call("fillw(wbuf, 64);", "LCMapStringW", &["0", "0x200", &c_w(&wn), "-1", "P(wbuf)", "64"], &format!("dumpw(wbuf, {});", wn.len()), &[line(wn.len() as u64, E_NONE), wl(&up)]);
    p.call("fillw(wbuf, 64);", "LCMapStringW", &["0", "0x100", &c_w(&wn), "-1", "P(wbuf)", "64"], &format!("dumpw(wbuf, {});", wn.len()), &[line(wn.len() as u64, E_NONE), wl(&lo)]);
    p.run("locale-0087-lcmap-misc");
}

#[test]
fn lcmap_stops_locale_zero_for_other_flags_and_everything_else_stops_as_before() {
    let a = c_w(&w_nul("a"));
    let e_acute = c_w(&[0xe9, 0]);
    let mut s = Stops::new();
    // Locale 0 is accepted only for dwMapFlags exactly 0x100 or 0x200: with LCMAP_LINGUISTIC_CASING and every other flag it stops as before
    for fl in ["0x01000100", "0x01000200", "0x300", "0x400", "0", "1", "0x101", "0x201", "0x40000200", "0x01000000", "0x100000", "0x400000", "0x8000100"] {
        s.abort(&format!("lcmap0-{fl}"), "LCMapStringW: locale other than the emulated one", &format!("RUN(fle_imp_LCMapStringW, 0, {fl}, {a}, -1, 0, 0, 0, 0)"));
    }
    // other locales stop for the exact flags too
    for lcid in ["0x407", "1", "2", "0x10409", "0x7e", "0x80", "0x1000", "0xffffffff", "0x400000"] {
        for fl in ["0x100", "0x200"] {
            s.abort(&format!("lcmap-{lcid}-{fl}"), "LCMapStringW: locale other than the emulated one", &format!("RUN(fle_imp_LCMapStringW, {lcid}, {fl}, {a}, -1, 0, 0, 0, 0)"));
        }
    }
    // LCMAP_LINGUISTIC_CASING: the earlier behaviour, ASCII only: a unit of 0x80 or above stops, with Locale 0x409
    for fl in ["0x01000100", "0x01000200"] {
        s.abort(&format!("lcmap-ling-{fl}-e-acute"), "LCMapStringW: character of 0x80 or above", &format!("RUN(fle_imp_LCMapStringW, 0x409, {fl}, {e_acute}, -1, 0, 0, 0, 0)"));
        s.abort(&format!("lcmap-ling-{fl}-e-acute-buf"), "LCMapStringW: character of 0x80 or above", &format!("RUN(fle_imp_LCMapStringW, 0x409, {fl}, {e_acute}, -1, P(wbuf), 8, 0, 0)"));
    }
    // flags the page does not define: ERROR_INVALID_FLAGS with a known locale, a stop with Locale 0 (unchanged)
    s.plain("RUN(fle_imp_LCMapStringW, 0x409, 0x40000200, P((const uint16_t[]){ 97, 0 }), -1, 0, 0, 0, 0); out();", "0 3ec");
    s.abort("lcmap0-undefined-flag", "LCMapStringW: locale other than the emulated one", &format!("RUN(fle_imp_LCMapStringW, 0, 0x40000200, {a}, -1, 0, 0, 0, 0)"));
    // Locale 0 is not accepted by the other functions: they are not changed
    s.abort("gli-0", "GetLocaleInfoW: locale other than the emulated one", "RUN(fle_imp_GetLocaleInfoW, 0, 1, P(wbuf), 8, 0, 0, 0, 0)");
    s.abort("gli-0-return-number", "GetLocaleInfoW: locale other than the emulated one", "RUN(fle_imp_GetLocaleInfoW, 0, 0x20001004, P(wbuf), 8, 0, 0, 0, 0)");
    for fl in ["0", "1"] {
        s.abort(&format!("cmp-0-{fl}"), "CompareStringW: locale other than the emulated one", &format!("RUN(fle_imp_CompareStringW, 0, {fl}, {a}, -1, {a}, -1, 0, 0)"));
    }
    s.plain("RUN(fle_imp_SetThreadLocale, 0, 0, 0, 0, 0, 0, 0, 0); out();", "0 57");
    for fl in ["1", "2"] {
        s.plain(&format!("RUN(fle_imp_IsValidLocale, 0, {fl}, 0, 0, 0, 0, 0, 0); out();"), "0 aaaa");
    }
    // a successful Locale 0 call leaves the thread locale alone
    s.plain("RUN(fle_imp_LCMapStringW, 0, 0x200, P((const uint16_t[]){ 97, 0 }), -1, 0, 0, 0, 0); out();", "2 aaaa");
    s.plain("RUN(fle_imp_GetThreadLocale, 0, 0, 0, 0, 0, 0, 0, 0); out();", "409 aaaa");
    s.run("locale-0087-lcmap-stops");
    // every one of the 65,536 units in the middle of "A?B", both flags, size query and conversion, Locale 0: outside D stops, the unit named
    let sweep = r#"
    {
        unsigned u; int fi, ci, ab = 0, msg_out = 0, unchanged = 0, ok = 0, k;
        char want[160];
        for (fi = 0; fi < 2; fi++) for (ci = 0; ci < 2; ci++) for (u = 0; u < 0x10000; u++) {
            uint16_t s3[3] = { 0x41, (uint16_t)u, 0x42 };
            volatile int a;
            fillw(wbuf, 8);
            TRY_ABORT(RUN(fle_imp_LCMapStringW, 0, fi ? 0x200 : 0x100, P(s3), 3, P(wbuf), ci ? 8 : 0, 0, 0), a);
            if (a) {
                ab++;
                for (k = 0; k < 8 && wbuf[k] == 0x5555; k++) {}
                if (k == 8) unchanged++;
                snprintf(want, sizeof want, "LCMapStringW: U+%04X is outside the range D of decision 0087", u);
                if (strstr(emsg, want)) msg_out++;
            } else if (R[0] == 3) ok++;
        }
        printf("lcmap %d %d %d %d\n", ab, msg_out, unchanged, ok);
    }
"#;
    let got = try_prog("locale-0087-lcmap-sweep", sweep);
    // decision 0284 (B): wall ia2-W25's private-use units U+E000..U+F8FF (0x1900 = 6,400 units) are mapped by decision 0191 in every build (old: only with the ia2 marker row, which the production imports always had)
    let pua = 0xF8FF - 0xE000 + 1;
    let outside = 4 * (65536 - 284 - pua);
    assert_eq!(got, [format!("lcmap {outside} {outside} {outside} {}", 4 * (284 + pua))]);
}

#[test]
fn wctmb_1252_every_unit_of_d_flags_0_and_no_best_fit_default_and_used_chars() {
    let w = wctable();
    let d = d_case();
    let mut p = Prog::new();
    for &u in d.keys() {
        for flags in [0u32, 0x400] {
            let fl = flags.to_string();
            // lpDefaultChar and lpUsedDefaultChar NULL: no default checking, the CPINFO default 0x3f stands in
            let (b, _) = wct_expect(&w, u, flags, w.default);
            p.call("fillb(bbuf, 4);", "WideCharToMultiByte", &["1252", &fl, &c_w(&[u]), "1", "P(bbuf)", "4", "0", "0"], "dumpb(bbuf, 2);", &[line(1, E_NONE), bl(&[b, 0x55])]);
            // a given default character '#' and lpUsedDefaultChar: the flag is TRUE when the default character was used
            let (b2, used) = wct_expect(&w, u, flags, b'#');
            p.call(
                "fillb(bbuf, 4); { static uint32_t us; us = 7;",
                "WideCharToMultiByte",
                &["1252", &fl, &c_w(&[u]), "1", "P(bbuf)", "4", "P(\"#\")", "P(&us)"],
                "printf(\"U %u\\n\", us); dumpb(bbuf, 2); }",
                &[line(1, E_NONE), format!("U {}", used as u32), bl(&[b2, 0x55])],
            );
        }
    }
    // the whole range at once, 284 units, the 27 units without a row use the default character
    let units: Vec<u16> = d.keys().copied().collect();
    for flags in [0u32, 0x400] {
        let fl = flags.to_string();
        let exp: Vec<u8> = units.iter().map(|&u| wct_expect(&w, u, flags, w.default).0).collect();
        let mut want = exp.clone();
        want.push(0x55);
        p.call("fillb(dbb1, 320);", "WideCharToMultiByte", &["1252", &fl, &c_w(&units), "284", "P(dbb1)", "300", "0", "0"], "dumpb(dbb1, 285);", &[line(284, E_NONE), bl(&want)]);
        p.ret("WideCharToMultiByte", &["1252", &fl, &c_w(&units), "284", "0", "0", "0", "0"], 284, E_NONE);
        // CP_ACP (0) and CP_THREAD_ACP (3) are 1252 for the emulated locale
        for cp in ["0", "3"] {
            p.call("fillb(dbb1, 320);", "WideCharToMultiByte", &[cp, &fl, &c_w(&units), "284", "P(dbb1)", "300", "0", "0"], "dumpb(dbb1, 285);", &[line(284, E_NONE), bl(&want)]);
        }
    }
    p.run("locale-0087-wctmb-d");
}

#[test]
fn wctmb_1252_the_attach_chain_gstw_lcmap_lower_wctmb_lcmap_upper_wctmb() {
    // the calls of f_37d674c in the order of the investigation 3.1, 3.2 with its arguments: no stop, the table values are checked
    let (d, wine, w) = (d_case(), c1_wine(), wctable());
    let att = attach_input();
    let lower: Vec<u16> = att.iter().map(|u| d[u].1).collect();
    let upper: Vec<u16> = att.iter().map(|u| d[u].0).collect();
    let lower_bytes: Vec<u8> = lower.iter().map(|&u| wct_expect(&w, u, 0, w.default).0).collect();
    let upper_bytes: Vec<u8> = upper.iter().map(|&u| wct_expect(&w, u, 0, w.default).0).collect();
    // the one best-fit unit of the whole chain: U+0192 -> U+0191 -> 0x83, at the position of byte 0x83
    assert_eq!((att[0x83], upper[0x83], upper_bytes[0x83]), (0x192, 0x191, 0x83));
    assert_eq!(upper.iter().filter(|&&u| !w.rows[&u].1).count(), 1);
    let mut p = Prog::new();
    let c1: Vec<u16> = att.iter().map(|u| wine[u]).collect();
    p.call("fillw(dbw1, 320);", "GetStringTypeW", &["1", &c_w(&att), "256", "P(dbw1)"], "dumpw(dbw1, 256);", &[line(1, E_NONE), wl(&c1)]);
    for (flag, mapped, bytes) in [("0x100", &lower, &lower_bytes), ("0x200", &upper, &upper_bytes)] {
        // MultiByteToWideChar of ATTACH is not repeated here; LCMapStringW: Locale 0, a size query and a conversion, then WideCharToMultiByte(1252,
        // dwFlags 0, 256 units, 256 bytes, no default char, no used flag)
        p.ret("LCMapStringW", &["0", flag, &c_w(&att), "256", "0", "0"], 256, E_NONE);
        p.call("fillw(dbw2, 320);", "LCMapStringW", &["0", flag, &c_w(&att), "256", "P(dbw2)", "256"], "dumpw(dbw2, 256);", &[line(256, E_NONE), wl(mapped)]);
        p.call("fillb(dbb1, 320);", "WideCharToMultiByte", &["1252", "0", "P(dbw2)", "256", "P(dbb1)", "256", "0", "0"], "dumpb(dbb1, 256);", &[line(256, E_NONE), bl(bytes)]);
    }
    p.run("locale-0087-attach-chain");
}

#[test]
fn wctmb_1252_size_buffer_nul_and_error_rules() {
    // units: 'A', U+00E9, U+20AC, U+0080 (no row: the default character), U+0191 (a best-fit row: 0x83 without WC_NO_BEST_FIT_CHARS), NUL
    let s = [0x41u16, 0xe9, 0x20ac, 0x80, 0x191, 0];
    let src = c_w(&s);
    let mut p = Prog::new();
    for cp in ["1252", "0", "3"] {
        // size query: the number of bytes including the NUL for -1; lpUsedDefaultChar is written (TRUE: U+0080 used the default character)
        p.call("{ static uint32_t us; us = 7;", "WideCharToMultiByte", &[cp, "0", &src, "-1", "0", "0", "0", "P(&us)"], "printf(\"U %u\\n\", us); }", &[line(6, E_NONE), "U 1".into()]);
        p.ret("WideCharToMultiByte", &[cp, "0", &src, "-1", "0", "0", "0", "0"], 6, E_NONE);
        p.ret("WideCharToMultiByte", &[cp, "0", &src, "5", "0", "0", "0", "0"], 5, E_NONE);
        // the result with lpDefaultChar NULL (0x3f) and with '#'; a best-fit conversion is not "default used"
        p.call("fillb(bbuf, 16);", "WideCharToMultiByte", &[cp, "0", &src, "-1", "P(bbuf)", "16", "0", "0"], "dumpb(bbuf, 7);", &[line(6, E_NONE), bl(&[0x41, 0xe9, 0x80, 0x3f, 0x83, 0, 0x55])]);
        p.call("fillb(bbuf, 16);", "WideCharToMultiByte", &[cp, "0", &src, "-1", "P(bbuf)", "16", "P(\"#\")", "0"], "dumpb(bbuf, 7);", &[line(6, E_NONE), bl(&[0x41, 0xe9, 0x80, 0x23, 0x83, 0, 0x55])]);
        p.call("fillb(bbuf, 16);", "WideCharToMultiByte", &[cp, "0x400", &src, "-1", "P(bbuf)", "16", "0", "0"], "dumpb(bbuf, 7);", &[line(6, E_NONE), bl(&[0x41, 0xe9, 0x80, 0x3f, 0x3f, 0, 0x55])]);
    }
    // Learn's table of lpDefaultChar x lpUsedDefaultChar on "A", U+0080, "B"
    let t = c_w(&[0x41, 0x80, 0x42]);
    p.call("fillb(bbuf, 8);", "WideCharToMultiByte", &["1252", "0", &t, "3", "P(bbuf)", "8", "0", "0"], "dumpb(bbuf, 4);", &[line(3, E_NONE), bl(&[0x41, 0x3f, 0x42, 0x55])]);
    p.call("fillb(bbuf, 8);", "WideCharToMultiByte", &["1252", "0", &t, "3", "P(bbuf)", "8", "P(\"#\")", "0"], "dumpb(bbuf, 4);", &[line(3, E_NONE), bl(&[0x41, 0x23, 0x42, 0x55])]);
    p.call("fillb(bbuf, 8); { static uint32_t us; us = 7;", "WideCharToMultiByte", &["1252", "0", &t, "3", "P(bbuf)", "8", "0", "P(&us)"], "printf(\"U %u\\n\", us); dumpb(bbuf, 4); }", &[line(3, E_NONE), "U 1".into(), bl(&[0x41, 0x3f, 0x42, 0x55])]);
    p.call("fillb(bbuf, 8); { static uint32_t us; us = 7;", "WideCharToMultiByte", &["1252", "0", &t, "3", "P(bbuf)", "8", "P(\"#\")", "P(&us)"], "printf(\"U %u\\n\", us); dumpb(bbuf, 4); }", &[line(3, E_NONE), "U 1".into(), bl(&[0x41, 0x23, 0x42, 0x55])]);
    // a string that is all representable: lpUsedDefaultChar is FALSE (a stale TRUE is cleared)
    let ab = c_w(&[0x41, 0xe9, 0x20ac, 0x42]);
    p.call("fillb(bbuf, 8); { static uint32_t us; us = 7;", "WideCharToMultiByte", &["1252", "0", &ab, "4", "P(bbuf)", "8", "P(\"#\")", "P(&us)"], "printf(\"U %u\\n\", us); dumpb(bbuf, 5); }", &[line(4, E_NONE), "U 0".into(), bl(&[0x41, 0xe9, 0x80, 0x42, 0x55])]);
    // buffer sizes: exactly enough, one too small with -1 (nothing written), too small with an explicit count (the first cbMultiByte bytes are
    // written: the page), a size of 0 is the query, a negative size is invalid
    p.ret("WideCharToMultiByte", &["1252", "0", &src, "-1", "P(bbuf)", "6", "0", "0"], 6, E_NONE);
    p.call("fillb(bbuf, 16);", "WideCharToMultiByte", &["1252", "0", &src, "-1", "P(bbuf)", "5", "0", "0"], "dumpb(bbuf, 7);", &[line(0, E_BUF), bl(&[0x55; 7])]);
    p.call("fillb(bbuf, 16);", "WideCharToMultiByte", &["1252", "0", &src, "5", "P(bbuf)", "3", "0", "0"], "dumpb(bbuf, 6);", &[line(0, E_BUF), bl(&[0x41, 0xe9, 0x80, 0x55, 0x55, 0x55])]);
    p.ret("WideCharToMultiByte", &["1252", "0", &src, "5", "0", "3", "0", "0"], 0, E_BUF);
    p.ret("WideCharToMultiByte", &["1252", "0", &src, "5", "P(bbuf)", "-1", "0", "0"], 0, E_PARAM);
    // the page: cchWideChar 0 fails, NULL source, the same pointer for both strings: ERROR_INVALID_PARAMETER
    p.ret("WideCharToMultiByte", &["1252", "0", &src, "0", "P(bbuf)", "8", "0", "0"], 0, E_PARAM);
    p.ret("WideCharToMultiByte", &["1252", "0", "0", "-1", "P(bbuf)", "8", "0", "0"], 0, E_PARAM);
    p.ret("WideCharToMultiByte", &["1252", "0", "P(bbuf)", "2", "P(bbuf)", "8", "0", "0"], 0, E_PARAM);
    // 437 and CP_OEMCP: the ASCII rows still work, with lpUsedDefaultChar FALSE (the existing behaviour)
    let asc = c_w(&[0x48, 0x69, 0]);
    for cp in ["437", "1"] {
        p.call("fillb(bbuf, 8); { static uint32_t us; us = 7;", "WideCharToMultiByte", &[cp, "0", &asc, "-1", "P(bbuf)", "8", "0", "P(&us)"], "printf(\"U %u\\n\", us); dumpb(bbuf, 4); }", &[line(3, E_NONE), "U 0".into(), bl(&[0x48, 0x69, 0, 0x55])]);
    }
    p.run("locale-0087-wctmb-rules");
}

#[test]
fn wctmb_stops_outside_d_other_flags_and_other_code_pages() {
    let a = c_w(&w_nul("a"));
    let e_acute = c_w(&[0xe9, 0]);
    let mut s = Stops::new();
    // 1252: every dwFlags other than 0 and WC_NO_BEST_FIT_CHARS stops (WC_COMPOSITECHECK 0x200 with its options, 0x80, 0x40, 0x1, ...)
    for fl in ["1", "2", "0x40", "0x80", "0x200", "0x220", "0x240", "0x280", "0x800", "0x401", "0x420", "0x4000", "0x8000"] {
        s.abort(&format!("wctmb-1252-flags-{fl}"), "WideCharToMultiByte: dwFlags other than 0 and WC_NO_BEST_FIT_CHARS", &format!("RUN(fle_imp_WideCharToMultiByte, 1252, {fl}, {a}, -1, 0, 0, 0, 0)"));
    }
    // cp437, CP_OEMCP and other code pages: a unit of 0x80 or above stops as before (even a unit that is in D)
    for (cp, label) in [("437", "437"), ("1", "oemcp")] {
        s.abort(&format!("wctmb-{label}-e-acute"), "WideCharToMultiByte: UTF-16 unit of 0x80 or above for a single-byte code page", &format!("RUN(fle_imp_WideCharToMultiByte, {cp}, 0, {e_acute}, -1, 0, 0, 0, 0)"));
        s.abort(&format!("wctmb-{label}-euro"), "WideCharToMultiByte: UTF-16 unit of 0x80 or above for a single-byte code page", &format!("RUN(fle_imp_WideCharToMultiByte, {cp}, 0, {}, -1, P(bbuf), 8, 0, 0)", c_w(&[0x20ac, 0])));
    }
    for cp in ["1251", "932", "850", "28591", "2", "42"] {
        s.abort(&format!("wctmb-cp-{cp}"), "WideCharToMultiByte: ", &format!("RUN(fle_imp_WideCharToMultiByte, {cp}, 0, {a}, -1, 0, 0, 0, 0)"));
    }
    // cchWideChar below -1 stops (the page does not define it)
    s.abort("wctmb-1252-cw-minus2", "WideCharToMultiByte: cchWideChar below -1", &format!("RUN(fle_imp_WideCharToMultiByte, 1252, 0, {a}, -2, 0, 0, 0, 0)"));
    // outside D: a unit that has a WCTABLE row stops all the same (fullwidth A U+FF21, U+00A0.. are in D; U+2190 is not), nothing is written
    for (label, u) in [("fullwidth-a", 0xff21u16), ("arrow", 0x2190), ("cjk", 0x4e2d), ("u0100", 0x100), ("surrogate", 0xd800), ("ffff", 0xffff)] {
        s.abort(&format!("wctmb-1252-{label}"), &format!("WideCharToMultiByte: U+{u:04X} is outside the range D of decision 0087"), &format!("RUN(fle_imp_WideCharToMultiByte, 1252, 0, {}, 3, P(bbuf), 8, 0, 0)", c_w(&[0x41, u, 0x42])));
        s.abort(&format!("wctmb-1252-{label}-size"), &format!("WideCharToMultiByte: U+{u:04X} is outside the range D of decision 0087"), &format!("RUN(fle_imp_WideCharToMultiByte, 1252, 0x400, {}, -1, 0, 0, 0, 0)", c_w(&[0x41, u, 0])));
    }
    s.run("locale-0087-wctmb-stops");
    // the 441 rows of WCTABLE outside D and every other unit outside D: all of the 65,536 units, size query and conversion, both flags
    let w = wctable();
    let d = d_case();
    let outside_rows = w.rows.keys().copied().filter(|u| !d.contains_key(u)).count();
    assert_eq!(outside_rows, 441);
    let sweep = r#"
    {
        unsigned u; int fi, ci, ab = 0, msg_out = 0, unchanged = 0, ok = 0, k;
        char want[160];
        for (fi = 0; fi < 2; fi++) for (ci = 0; ci < 2; ci++) for (u = 0; u < 0x10000; u++) {
            uint16_t s3[3] = { 0x61, (uint16_t)u, 0x62 };
            volatile int a;
            fillb(bbuf, 8);
            TRY_ABORT(RUN(fle_imp_WideCharToMultiByte, 1252, fi ? 0x400 : 0, P(s3), 3, P(bbuf), ci ? 8 : 0, 0, 0), a);
            if (a) {
                ab++;
                for (k = 0; k < 8 && bbuf[k] == 0x55; k++) {}
                if (k == 8) unchanged++;
                snprintf(want, sizeof want, "WideCharToMultiByte: U+%04X is outside the range D of decision 0087", u);
                if (strstr(emsg, want)) msg_out++;
            } else if (R[0] == 3) ok++;
        }
        printf("wctmb %d %d %d %d\n", ab, msg_out, unchanged, ok);
    }
"#;
    let got = try_prog("locale-0087-wctmb-sweep", sweep);
    let outside = 4 * (65536 - 284);
    assert_eq!(got, [format!("wctmb {outside} {outside} {outside} {}", 4 * 284)]);
}

#[test]
fn decision_0087_stops_are_real_aborts() {
    // The survivable stops above run against a test-local `abort`; here the real one: SIGABRT, "fle: <api>: U+XXXX is outside the range D
    // of decision 0087" on stderr, no value returned.
    let w = |u: u16| c_w(&[0x41, u, 0]);
    let rows: Vec<(&str, String, &str)> = vec![
        ("gstw-outside", format!("RUN(fle_imp_GetStringTypeW, 1, {}, -1, P(wbuf), 0, 0, 0, 0);", w(0x4e2d)), "fle: GetStringTypeW: U+4E2D is outside the range D of decision 0087"),
        ("gstw-ctype2", format!("RUN(fle_imp_GetStringTypeW, 2, {}, -1, P(wbuf), 0, 0, 0, 0);", w(0x42)), "fle: GetStringTypeW: dwInfoType other than CT_CTYPE1"),
        ("gstw-ctype3", format!("RUN(fle_imp_GetStringTypeW, 4, {}, -1, P(wbuf), 0, 0, 0, 0);", w(0x42)), "fle: GetStringTypeW: dwInfoType other than CT_CTYPE1"),
        ("lcmap-outside", format!("RUN(fle_imp_LCMapStringW, 0, 0x200, {}, -1, 0, 0, 0, 0);", w(0x4e2d)), "fle: LCMapStringW: U+4E2D is outside the range D of decision 0087"),
        ("wctmb-outside", format!("RUN(fle_imp_WideCharToMultiByte, 1252, 0, {}, -1, 0, 0, 0, 0);", w(0x4e2d)), "fle: WideCharToMultiByte: U+4E2D is outside the range D of decision 0087"),
    ];
    for (name, body, needle) in rows {
        let src = format!("{PRE}int main(void)\n{{\n    setup(0, 0, 0, 0, 0, 0, 0, 0);\n    {body}\n    printf(\"returned %llx\\n\", (unsigned long long)R[0]);\n    return 0;\n}}\n");
        let exe = clang(&format!("abort87-{name}"), &src);
        let run = Command::new(&exe).output().unwrap();
        let (out, err) = (String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
        assert!(!run.status.success() && !out.contains("returned"), "{name}: {out}");
        assert!(err.contains(needle), "{name}: {err}");
        assert_eq!(run.status.signal(), Some(6), "{name}");
    }
}

// ---- 3. Undefined cases stop the run ----

#[test]
fn undefined_cases_abort() {
    let a = c_w(&w_nul("a"));
    let ab = c_w(&w_nul("ab"));
    let abc = c_w(&w_nul("abc"));
    let rows: Vec<(&str, String)> = vec![
        ("gli-german", "RUN(fle_imp_GetLocaleInfoW, 0x407, 1, P(wbuf), 8, 0, 0, 0, 0);".into()),
        // 0x1f (SSHORTDATE) was this row until decision 0048 answered it; 0x1001 (LOCALE_SENGLISHLANGUAGENAME) is not answered.
        ("gli-lctype-unknown", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x1001, P(wbuf), 8, 0, 0, 0, 0);".into()),
        // Neighbour of LOCALE_IOPTIONALCALENDAR (0x100b) that is still not answered: IFIRSTDAYOFWEEK. (0x1009,
        // LOCALE_ICALENDARTYPE, was a row here until decision 0042 answered it; see get_locale_info_enus_0042.)
        ("gli-lctype-ifirstdayofweek", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x100c, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-optcal-german", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x100b, P(wbuf), 8, 0, 0, 0, 0);".into()),
        // 0048 (was: six rows "gli-0042-slist" ... that expected abort): the six are answered for en-US now (see
        // get_locale_info_enus_0048); they stay stopped for other locales, and as a string with RETURN_NUMBER for the
        // string types.
        ("gli-0048-german-slist", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0xc, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-0048-german-inegcurr", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x1c, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-0048-german-sshortdate", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x1f, P(wbuf), 80, 0, 0, 0, 0);".into()),
        ("gli-0048-german-slongdate", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x20, P(wbuf), 80, 0, 0, 0, 0);".into()),
        ("gli-0048-german-stimeformat", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x1003, P(wbuf), 80, 0, 0, 0, 0);".into()),
        ("gli-0048-german-sshorttime", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x79, P(wbuf), 80, 0, 0, 0, 0);".into()),
        ("gli-0048-return-number-slist", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x2000000c, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-0048-return-number-sshortdate", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x2000001f, P(wbuf), 80, 0, 0, 0, 0);".into()),
        ("gli-0048-return-number-inegcurr-size0", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x2000001c, 0, 0, 0, 0, 0, 0);".into()),
        // 0042: just outside the answered ranges (SMONTHNAME13 0x100e, the unused 0x50, SABBREVMONTHNAME13 0x100f).
        ("gli-0042-smonthname13", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x100e, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-0042-after-abbrevmonth12", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x50, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-0042-sabbrevmonthname13", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x100f, P(wbuf), 8, 0, 0, 0, 0);".into()),
        // 0042: the new values exist for en-US only.
        ("gli-0042-german-sabbrevday7", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x37, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-0042-german-sdayname1", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x2a, P(wbuf), 16, 0, 0, 0, 0);".into()),
        ("gli-0042-german-smonthname1", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x38, P(wbuf), 16, 0, 0, 0, 0);".into()),
        ("gli-0042-german-s1159", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x28, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-0042-german-icalendartype", "RUN(fle_imp_GetLocaleInfoW, 0x407, 0x1009, P(wbuf), 8, 0, 0, 0, 0);".into()),
        // 0042: LOCALE_RETURN_NUMBER on a string type is still not defined here.
        ("gli-0042-return-number-s1159", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x20000028, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-0042-return-number-sabbrevday7", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x20000037, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-return-number-string", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x2000005c, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("gli-return-number-size0", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x20001004, 0, 0, 0, 0, 0, 0);".into()),
        ("gli-genitive", "RUN(fle_imp_GetLocaleInfoW, 0x409, 0x10000038, P(wbuf), 8, 0, 0, 0, 0);".into()),
        ("cpinfo-932", "RUN(fle_imp_GetCPInfo, 932, P(bbuf), 0, 0, 0, 0, 0, 0);".into()),
        ("cpinfo-macintosh", "RUN(fle_imp_GetCPInfo, 2, P(bbuf), 0, 0, 0, 0, 0, 0);".into()),
        ("dbcs-932", "RUN(fle_imp_IsDBCSLeadByteEx, 932, 0x81, 0, 0, 0, 0, 0, 0);".into()),
        ("dbcs-936", "RUN(fle_imp_IsDBCSLeadByteEx, 936, 0x81, 0, 0, 0, 0, 0, 0);".into()),
        ("valid-locale-0x39", "RUN(fle_imp_IsValidLocale, 0x409, 0x39, 0, 0, 0, 0, 0, 0);".into()),
        ("valid-locale-flags0", "RUN(fle_imp_IsValidLocale, 0x409, 0, 0, 0, 0, 0, 0, 0);".into()),
        ("mbtwc-cp1251", format!("RUN(fle_imp_MultiByteToWideChar, 1251, 0, {}, -1, 0, 0, 0, 0);", c_b(b"a\0"))),
        // decision 0084 answers 1252 (the row here was mbtwc-high-byte-1252, expected abort); 437 is not covered and still stops.
        ("mbtwc-high-byte-437", format!("RUN(fle_imp_MultiByteToWideChar, 437, 0, {}, -1, 0, 0, 0, 0);", c_b(&[0x80, 0]))),
        ("mbtwc-bad-utf8-no-flag", format!("RUN(fle_imp_MultiByteToWideChar, 65001, 0, {}, -1, 0, 0, 0, 0);", c_b(&[0xFF, 0]))),
        ("mbtwc-cb-below-minus1", format!("RUN(fle_imp_MultiByteToWideChar, 65001, 0, {}, -2, 0, 0, 0, 0);", c_b(b"a\0"))),
        ("wctmb-cp932", format!("RUN(fle_imp_WideCharToMultiByte, 932, 0, {a}, -1, 0, 0, 0, 0);")),
        // decision 0087 converts U+00E9 for 1252 (the row here was wctmb-non-ascii-1252, expected abort); 437 is not covered and still
        // stops, and so does a unit outside the range D for 1252 (U+4E2D).
        ("wctmb-non-ascii-437", format!("RUN(fle_imp_WideCharToMultiByte, 437, 0, {}, -1, 0, 0, 0, 0);", c_w(&[0xe9, 0]))),
        ("wctmb-outside-d-1252", format!("RUN(fle_imp_WideCharToMultiByte, 1252, 0, {}, -1, 0, 0, 0, 0);", c_w(&[0x4e2d, 0]))),
        ("wctmb-flags-1252", format!("RUN(fle_imp_WideCharToMultiByte, 1252, 0x200, {a}, -1, 0, 0, 0, 0);")),
        ("lcmap-german", format!("RUN(fle_imp_LCMapStringW, 0x407, 0x200, {a}, -1, 0, 0, 0, 0);")),
        ("lcmap-sortkey", format!("RUN(fle_imp_LCMapStringW, 0x409, 0x400, {a}, -1, 0, 0, 0, 0);")),
        ("lcmap-no-case-flag", format!("RUN(fle_imp_LCMapStringW, 0x409, 0, {a}, -1, 0, 0, 0, 0);")),
        ("lcmap-halfwidth", format!("RUN(fle_imp_LCMapStringW, 0x409, 0x400000, {a}, -1, 0, 0, 0, 0);")),
        ("lcmap-titlecase", format!("RUN(fle_imp_LCMapStringW, 0x409, 0x300, {a}, -1, 0, 0, 0, 0);")),
        // decision 0087 maps U+00E9 for the exact flags 0x100/0x200 (the row here was lcmap-non-ascii, 0x200, expected abort); with
        // LCMAP_LINGUISTIC_CASING the older ASCII-only behaviour is kept and still stops.
        ("lcmap-non-ascii-linguistic", format!("RUN(fle_imp_LCMapStringW, 0x409, 0x01000200, {}, -1, 0, 0, 0, 0);", c_w(&[0xe9, 0]))),
        ("lcmap-non-ascii-linguistic-lower", format!("RUN(fle_imp_LCMapStringW, 0x409, 0x01000100, {}, -1, 0, 0, 0, 0);", c_w(&[0xc9, 0]))),
        ("cmp-german", format!("RUN(fle_imp_CompareStringW, 0x407, 0, {a}, -1, {a}, -1, 0, 0);")),
        ("cmp-invariant", format!("RUN(fle_imp_CompareStringW, 0x7f, 0, {a}, -1, {a}, -1, 0, 0);")),
        // decision 0126 (wall 72, S-cmp-prefix): old: ("cmp-prefix", "ab" against "abc") stopped here; the shorter string is the smaller now (compare_string_one_a_prefix_of_the_other_is_the_shorter_first below)
        ("cmp-case-only", format!("RUN(fle_imp_CompareStringW, 0x409, 0, {}, -1, {a}, -1, 0, 0);", c_w(&w_nul("A")))),
        // decision 0171 (wall ia1-W11, S-ia1-cmp-digit-letter): old: ("cmp-letter-digit", "a" against "1") stopped here; a digit sorts before a letter now (tests/wall_r2.rs checks both orders)
        // decision 0229 (ref1, S-ref1-cmp-hyphen-default): old: ("cmp-punct", the string {a} against "-") stopped here; the hyphen follows the word sort now (compare_string_hyphen_and_apostrophe_follow_the_word_sort)
        ("cmp-ignorenonspace", format!("RUN(fle_imp_CompareStringW, 0x409, 2, {a}, -1, {a}, -1, 0, 0);")),
    ];
    for (name, body) in rows {
        let src = format!("{PRE}int main(void)\n{{\n    setup(0, 0, 0, 0, 0, 0, 0, 0);\n    {body}\n    printf(\"returned %llx\\n\", (unsigned long long)R[0]);\n    return 0;\n}}\n");
        let exe = clang(&format!("abort-{name}"), &src);
        let run = Command::new(&exe).output().unwrap();
        assert!(!run.status.success(), "{name}: expected abort, stdout: {}", String::from_utf8_lossy(&run.stdout));
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{name}");
        assert!(String::from_utf8_lossy(&run.stderr).contains("fle:"), "{name}: {}", String::from_utf8_lossy(&run.stderr));
        assert_eq!(run.status.signal(), Some(6), "{name}");
    }
}
