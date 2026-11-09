//! Tests for the emulated time source, the time zone and the kernel32 time/date answers in runtime/fle_time.c
//! (docs/decisions/0020-process-and-time.md; sources and what they do not say: docs/investigations/import-proctime-spec.md).
//!
//! Expected values come from rules written HERE, not read back from the C code: the calendar is Howard Hinnant's
//! days_from_civil / civil_from_days (proleptic Gregorian), the FILETIME epoch is 1601-01-01 (FILETIME page), the
//! clock rules are those of decision 0017 (start value, Sleep adds ms * 10000 ticks, each read adds 10 ticks), the
//! SYSTEMTIME ranges are those of the SYSTEMTIME page. A value the pages do not give is marked "assumption" or
//! "inference". The host is Linux: nothing is compared with a real Windows. No host clock or time zone is read.
//!
//! The C sources come from runtime/ when fle_time.c is there, otherwise from out-proctime/work/ (development).

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if runtime_dir().join("fle_time.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-proctime/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-proctime/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd
        .arg("-I")
        .arg(src_dir())
        .arg("-I")
        .arg(runtime_dir())
        .arg(src)
        .arg("-o")
        .arg(obj)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-time"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_time.c", true), ("fle_sys.c", false), ("fle_imports.c", false), ("fle_env.c", false)] {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
            v.push(o);
        }
        v
    })
}

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_time.h"

uint64_t R[16];
static uint64_t stk[32];

/* a[0..3] go to rcx, rdx, r8, r9; a[4..11] are arguments 5..12 at [rsp + 0x28 + 8 * k]. */
static uint64_t callv(void (*f)(void), const uint64_t *a)
{
    int i;
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    for (i = 4; i < 12; i++) stk[i + 1] = a[i];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define CALL(f, ...) callv((void (*)(void))(f), (uint64_t[12]){ __VA_ARGS__ })
#define A(x) ((uint64_t)(uintptr_t)(x))
#define W(s) A(u##s)
#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static char asc_buf[8][1024];
static int asc_i;
static const char *AS(const uint16_t *s)
{
    char *o = asc_buf[asc_i++ & 7];
    size_t n = 0;
    while (s[n] && n < 1000) { o[n] = (char)s[n]; n++; }
    o[n] = 0;
    return o;
}
static uint64_t FT(const uint32_t *b) { return ((uint64_t)b[1] << 32) | b[0]; }
static void PST(const uint16_t *s) { printf("%u %u %u %u %u %u %u %u\n", s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]); }
"#;

fn run(name: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(&format!("time-{name}"));
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}", o.signal, o.stderr);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn assert_aborts(prefix: &str, setup: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{label}: no fle: message: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

// ---- The independent model ----

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

const TICKS_PER_SEC: u64 = 10_000_000;
const TICKS_PER_MS: u64 = 10_000;

/// FILETIME of a UTC date and time (FILETIME page: 100 ns intervals since 1601-01-01).
fn ft_of(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64, ms: i64) -> u64 {
    let days = days_from_civil(y, mo, d) - days_from_civil(1601, 1, 1);
    assert!(days >= 0);
    ((days * 86400 + h * 3600 + mi * 60 + s) as u64) * TICKS_PER_SEC + (ms as u64) * TICKS_PER_MS
}

/// SYSTEMTIME members (year, month, day of week, day, hour, minute, second, milliseconds) of a FILETIME.
fn st_of(ft: u64) -> [u16; 8] {
    let secs = ft / TICKS_PER_SEC;
    let ms = (ft % TICKS_PER_SEC) / TICKS_PER_MS;
    let days = (secs / 86400) as i64 + days_from_civil(1601, 1, 1);
    let rem = secs % 86400;
    let (y, m, d) = civil_from_days(days);
    let dow = (days + 4).rem_euclid(7); // 1970-01-01 (day 0) was a Thursday = 4, Sunday = 0
    [y as u16, m as u16, dow as u16, d as u16, (rem / 3600) as u16, ((rem % 3600) / 60) as u16, (rem % 60) as u16, ms as u16]
}

/// A real calendar date: it survives a round trip through the day count. Ranges of the SYSTEMTIME page.
fn real_date(y: i64, m: i64, d: i64) -> bool {
    (1601..=30827).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d) && civil_from_days(days_from_civil(y, m, d)) == (y, m, d)
}

fn st_line(s: [u16; 8]) -> String {
    s.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ")
}

/// The first clock value a fresh process reads: the clock of decision 0017 starts at 3,600,000 ms = 36,000,000,000 ticks.
const CLOCK_START_TICKS: u64 = 3_600_000 * TICKS_PER_MS;
/// Tick 0 is 2020-12-31T23:00:00Z, so that the first read is 2021-01-01T00:00:00Z, the start of the file clock.
const BOOT_FT: u64 = 132_539_292_000_000_000;

#[test]
fn model_is_pinned_by_known_dates() {
    // Hand-known anchors, so the model itself is checked before it is used as the oracle.
    assert_eq!(ft_of(1601, 1, 1, 0, 0, 0, 0), 0);
    assert_eq!(ft_of(1970, 1, 1, 0, 0, 0, 0), 116_444_736_000_000_000); // the well-known Unix epoch FILETIME
    assert_eq!(ft_of(2021, 1, 1, 0, 0, 0, 0), 132_539_328_000_000_000); // FLE_VFS_CLOCK_START (fle_vfs.h)
    assert_eq!(BOOT_FT, ft_of(2021, 1, 1, 0, 0, 0, 0) - 3600 * TICKS_PER_SEC);
    assert_eq!(BOOT_FT + CLOCK_START_TICKS, ft_of(2021, 1, 1, 0, 0, 0, 0));
    assert_eq!(st_of(ft_of(2024, 2, 29, 13, 4, 5, 678)), [2024, 2, 4, 29, 13, 4, 5, 678]); // a Thursday
    assert_eq!(st_of(0), [1601, 1, 1, 1, 0, 0, 0, 0]); // a Monday
    assert_eq!(st_of(ft_of(2000, 1, 1, 0, 0, 0, 0))[2], 6); // a Saturday
    assert!(real_date(2000, 2, 29) && real_date(2024, 2, 29) && real_date(1600 + 4, 2, 29));
    assert!(!real_date(1900, 2, 29) && !real_date(2100, 2, 29) && !real_date(2023, 2, 29) && !real_date(2021, 4, 31));
}

// ---- 1. Table, header ----

const A4: [&str; 9] = [
    "DosDateTimeToFileTime",
    "FileTimeToDosDateTime",
    "FileTimeToLocalFileTime",
    "FileTimeToSystemTime",
    "GetLocalTime",
    "GetTimeZoneInformation",
    "LocalFileTimeToFileTime",
    "SystemTimeToTzSpecificLocalTime",
    "GetDateFormatW",
];
/// Not thunks of the closure (0 call sites) but names the program holds or may ask GetProcAddress for.
const EXTRA: [&str; 4] = ["GetSystemTime", "GetSystemTimeAsFileTime", "GetTickCount64", "SystemTimeToFileTime"];

#[test]
fn rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_time.h")).unwrap();
    for n in A4.iter().chain(EXTRA.iter()) {
        let row = format!("(\"kernel32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_time.h");
    }
    assert!(h.contains("UINT64_C(132539292000000000)"), "boot FILETIME constant");
}

#[test]
fn real_dll_handles_the_rows() {
    let p = manifest().join("../../../binaries/win64/FLEngine_x64.dll");
    let Ok(data) = std::fs::read(p) else { return };
    let Ok(seeds_txt) = std::fs::read_to_string(manifest().join("../../out-vfs/seeds-896.txt")) else { return };
    let seeds: Vec<u64> =
        seeds_txt.lines().filter(|l| !l.trim().is_empty()).map(|l| u64::from_str_radix(l.trim().trim_start_matches("0x"), 16).unwrap()).collect();
    assert_eq!(seeds.len(), 896);
    let mut out = Vec::new();
    let st = fe_lift::generate_with_seeds(&data, &seeds, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    for n in A4 {
        assert!(st.handled.get(format!("jmp_imp:{n}").as_str()).copied().unwrap_or(0) >= 1, "{n} is a handled thunk");
        assert!(c.contains(&format!("fle_imp_{n}(); return;")), "{n}: the thunk calls the runtime");
    }
    for n in A4.iter().chain(EXTRA.iter()) {
        assert!(c.contains(&format!("{{ \"kernel32.dll\", \"{n}\", ")), "{n} in fle_resp_table");
    }
}

// ---- 2. FILETIME <-> SYSTEMTIME ----

fn lcg(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *seed >> 1
}

#[test]
fn filetime_to_systemtime_every_day_of_a_400_year_cycle() {
    // 1601-01-01 .. 2000-12-31, one probe per day at 23:59:59.999 (the last millisecond of the day). 146,097 days.
    let l = lines(
        "ft2st-dense",
        r#"
    uint32_t b[2]; uint16_t s[8]; uint32_t o[2];
    uint64_t d, bad = 0, r;
    for (d = 0; d < 146097; d++) {
        uint64_t ft = d * 864000000000ull + 863999990000ull;
        b[0] = (uint32_t)ft; b[1] = (uint32_t)(ft >> 32);
        r = CALL(fle_imp_FileTimeToSystemTime, A(b), A(s));
        printf("%llu ", U(r)); PST(s);
        /* SystemTimeToFileTime ignores wDayOfWeek: poison it */
        s[2] = 9;
        r = CALL(fle_imp_SystemTimeToFileTime, A(s), A(o));
        if (r != 1 || FT(o) != ft) bad++;
    }
    printf("roundtrip_bad %llu\n", U(bad));
"#,
    );
    assert_eq!(l.len(), 146097 + 1);
    for d in 0..146097u64 {
        let ft = d * 864_000_000_000 + 863_999_990_000;
        assert_eq!(l[d as usize], format!("1 {}", st_line(st_of(ft))), "day {d}");
    }
    assert_eq!(l[146097], "roundtrip_bad 0");
}

fn boundary_filetimes() -> Vec<u64> {
    let mut v = vec![0u64, 1, 9_999, 10_000, 9_999_999, 10_000_000, 10_000_001];
    // leap-year rules: divisible by 4, not by 100 unless by 400
    for (y, m, d, h, mi, s, ms) in [
        (1604, 2, 29, 0, 0, 0, 0),
        (1700, 2, 28, 23, 59, 59, 999),
        (1700, 3, 1, 0, 0, 0, 0),
        (1900, 2, 28, 23, 59, 59, 999),
        (1900, 3, 1, 0, 0, 0, 0),
        (2000, 2, 29, 12, 0, 0, 0),
        (2000, 3, 1, 0, 0, 0, 0),
        (2023, 2, 28, 23, 59, 59, 999),
        (2023, 3, 1, 0, 0, 0, 0),
        (2024, 2, 28, 23, 59, 59, 999),
        (2024, 2, 29, 0, 0, 0, 0),
        (2024, 2, 29, 23, 59, 59, 999),
        (2024, 3, 1, 0, 0, 0, 0),
        (2100, 2, 28, 23, 59, 59, 999),
        (2100, 3, 1, 0, 0, 0, 0),
        (2400, 2, 29, 6, 7, 8, 9),
        (2400, 12, 31, 23, 59, 59, 999),
        (1970, 1, 1, 0, 0, 0, 0),
        (2038, 1, 19, 3, 14, 7, 0),
        (2106, 2, 7, 6, 28, 15, 0),
        (2107, 12, 31, 23, 59, 58, 0),
        (2108, 1, 1, 0, 0, 0, 0),
        (9999, 12, 31, 23, 59, 59, 999),
        (10000, 1, 1, 0, 0, 0, 0),
        (30827, 2, 28, 23, 59, 59, 999),
        (30827, 12, 31, 23, 59, 59, 999),
    ] {
        v.push(ft_of(y, m, d, h, mi, s, ms));
    }
    let limit = ft_of(30828, 1, 1, 0, 0, 0, 0);
    let mut seed = 12345u64;
    for _ in 0..3000 {
        v.push(lcg(&mut seed) % limit);
    }
    v.push(limit - 1);
    v
}

#[test]
fn filetime_to_systemtime_boundaries_and_random_values() {
    let cases = boundary_filetimes();
    let list = cases.iter().map(|v| format!("{v}ull")).collect::<Vec<_>>().join(", ");
    let body = format!(
        r#"
    static const uint64_t c[] = {{ {list} }};
    uint32_t b[2]; uint16_t s[8]; uint32_t o[2];
    size_t i, bad = 0;
    for (i = 0; i < sizeof c / sizeof c[0]; i++) {{
        uint64_t r;
        b[0] = (uint32_t)c[i]; b[1] = (uint32_t)(c[i] >> 32);
        r = CALL(fle_imp_FileTimeToSystemTime, A(b), A(s));
        printf("%llu ", U(r)); PST(s);
        r = CALL(fle_imp_SystemTimeToFileTime, A(s), A(o));
        if (r != 1 || FT(o) != c[i]) bad++;
    }}
    printf("roundtrip_bad %zu\n", bad);
"#
    );
    // the round trip is exact only when the value is a whole number of milliseconds: SYSTEMTIME has no finer member
    let l = lines("ft2st-bounds", &body);
    assert_eq!(l.len(), cases.len() + 1);
    let mut inexact = 0;
    for (i, &ft) in cases.iter().enumerate() {
        assert_eq!(l[i], format!("1 {}", st_line(st_of(ft))), "case {i}: ft {ft}");
        if ft % TICKS_PER_MS != 0 {
            inexact += 1;
        }
    }
    assert_eq!(l[cases.len()], format!("roundtrip_bad {inexact}"));
}

#[test]
fn filetime_to_systemtime_failure_and_stop() {
    // Page: the value "must be less than 0x8000000000000000. Otherwise, the function fails." The error code is not
    // named (assumption: ERROR_INVALID_PARAMETER, 87).
    let l = lines(
        "ft2st-fail",
        r#"
    uint32_t b[2]; uint16_t s[8];
    uint64_t r;
    memset(s, 0xAB, sizeof s);
    b[0] = 0; b[1] = 0x80000000u; SETLE(0);
    r = CALL(fle_imp_FileTimeToSystemTime, A(b), A(s)); printf("%llu %u %x\n", U(r), LE(), s[0]);
    b[0] = 0xFFFFFFFFu; b[1] = 0xFFFFFFFFu;
    r = CALL(fle_imp_FileTimeToSystemTime, A(b), A(s)); printf("%llu %u\n", U(r), LE());
"#,
    );
    assert_eq!(l, ["0 87 abab", "0 87"]);
    let over = ft_of(30828, 1, 1, 0, 0, 0, 0);
    // the first FILETIME whose year is above the documented wYear maximum (30827): the page defines no result: stop
    assert_aborts(
        "ft2st-abort",
        "    uint32_t b[2]; uint16_t s[8];",
        &[
            ("year-30828", &format!("b[0] = (uint32_t){over}ull; b[1] = (uint32_t)({over}ull >> 32); CALL(fle_imp_FileTimeToSystemTime, A(b), A(s));")),
            ("null-in", "CALL(fle_imp_FileTimeToSystemTime, 0, A(s));"),
            ("null-out", "CALL(fle_imp_FileTimeToSystemTime, A(b), 0);"),
        ],
    );
}

#[test]
fn systemtime_to_filetime_validity() {
    // Invalid dates fail (the page: "A False return value can indicate that the passed SYSTEMTIME structure represents an
    // invalid date", with the leap year given as the example). Ranges are those of the SYSTEMTIME page. The error code
    // is not named (assumption 87). wDayOfWeek is ignored (page).
    let mut cases: Vec<[u16; 8]> = Vec::new();
    let base = [2021u16, 6, 9, 15, 12, 30, 45, 500];
    cases.push(base);
    for (y, m, d) in [
        (1601u16, 1u16, 1u16),
        (1600, 12, 31),
        (1601, 2, 29),
        (1604, 2, 29),
        (1900, 2, 29),
        (2000, 2, 29),
        (2023, 2, 29),
        (2024, 2, 29),
        (2024, 2, 30),
        (2100, 2, 29),
        (2400, 2, 29),
        (2021, 4, 30),
        (2021, 4, 31),
        (2021, 0, 1),
        (2021, 13, 1),
        (2021, 1, 0),
        (2021, 1, 32),
        (30827, 12, 31),
        (30828, 1, 1),
        (0, 1, 1),
        (65535, 12, 31),
    ] {
        cases.push([y, m, 3, d, 0, 0, 0, 0]);
    }
    for (h, mi, s, ms) in [(23u16, 59u16, 59u16, 999u16), (24, 0, 0, 0), (0, 60, 0, 0), (0, 0, 60, 0), (0, 0, 0, 1000), (0, 0, 0, 65535), (65535, 0, 0, 0)] {
        cases.push([2021, 1, 1, 1, h, mi, s, ms]);
    }
    let list = cases.iter().map(|c| format!("{{ {} }}", c.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))).collect::<Vec<_>>().join(", ");
    let body = format!(
        r#"
    static const uint16_t c[][8] = {{ {list} }};
    uint32_t o[2];
    size_t i;
    for (i = 0; i < sizeof c / sizeof c[0]; i++) {{
        uint64_t r;
        o[0] = o[1] = 0xCDCDCDCDu; SETLE(0);
        r = CALL(fle_imp_SystemTimeToFileTime, A(c[i]), A(o));
        if (r) printf("1 %llu\n", U(FT(o))); else printf("0 %u %x\n", LE(), o[0]);
    }}
"#
    );
    let l = lines("st2ft", &body);
    assert_eq!(l.len(), cases.len());
    for (i, c) in cases.iter().enumerate() {
        let (y, m, d) = (c[0] as i64, c[1] as i64, c[3] as i64);
        let ok = real_date(y, m, d) && c[4] < 24 && c[5] < 60 && c[6] < 60 && c[7] < 1000;
        let want = if ok {
            format!("1 {}", ft_of(y, m, d, c[4] as i64, c[5] as i64, c[6] as i64, c[7] as i64))
        } else {
            "0 87 cdcdcdcd".to_string()
        };
        assert_eq!(l[i], want, "case {i}: {c:?}");
    }
    // the cases above are not all accepted and not all rejected
    assert!(l.iter().any(|s| s.starts_with("1 ")) && l.iter().any(|s| s.starts_with("0 ")));
}

#[test]
fn systemtime_to_filetime_stops_on_null() {
    assert_aborts(
        "st2ft-abort",
        "    uint32_t o[2]; uint16_t s[8] = { 2021, 1, 0, 1, 0, 0, 0, 0 };",
        &[("null-in", "CALL(fle_imp_SystemTimeToFileTime, 0, A(o));"), ("null-out", "CALL(fle_imp_SystemTimeToFileTime, A(s), 0);")],
    );
}

// ---- 3. The one time source ----

#[derive(Clone, Copy)]
enum Op {
    Sys,
    Local,
    Ft,
    Tick64,
    Tick,
    Qpc,
    Sleep(u32),
}

/// Decision 0017 plus 0020: ticks of 100 ns; a read returns the clock and then adds 10 ticks; Sleep adds ms * 10000.
/// UTC time = BOOT_FT + ticks. Time zone UTC: local time equals UTC.
fn clock_model(ops: &[Op]) -> Vec<String> {
    let mut c = CLOCK_START_TICKS;
    let mut v = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        match *op {
            Op::Sys | Op::Local => {
                v.push(format!("{i} {}", st_line(st_of(BOOT_FT + c))));
                c += 10;
            }
            Op::Ft => {
                v.push(format!("{i} {}", BOOT_FT + c));
                c += 10;
            }
            Op::Tick64 => {
                v.push(format!("{i} {}", c / TICKS_PER_MS));
                c += 10;
            }
            Op::Tick => {
                v.push(format!("{i} {}", (c / TICKS_PER_MS) as u32));
                c += 10;
            }
            Op::Qpc => {
                v.push(format!("{i} {c}"));
                c += 10;
            }
            Op::Sleep(ms) => c += ms as u64 * TICKS_PER_MS,
        }
    }
    v
}

fn clock_program(ops: &[Op]) -> String {
    let mut b = String::from("    uint32_t f[2]; uint16_t s[8]; uint64_t q;\n");
    for (i, op) in ops.iter().enumerate() {
        match *op {
            Op::Sys => b.push_str(&format!("    CALL(fle_imp_GetSystemTime, A(s)); printf(\"{i} \"); PST(s);\n")),
            Op::Local => b.push_str(&format!("    CALL(fle_imp_GetLocalTime, A(s)); printf(\"{i} \"); PST(s);\n")),
            Op::Ft => b.push_str(&format!("    CALL(fle_imp_GetSystemTimeAsFileTime, A(f)); printf(\"{i} %llu\\n\", U(FT(f)));\n")),
            Op::Tick64 => b.push_str(&format!("    printf(\"{i} %llu\\n\", U(CALL(fle_imp_GetTickCount64, 0)));\n")),
            Op::Tick => b.push_str(&format!("    printf(\"{i} %u\\n\", (uint32_t)CALL(fle_imp_GetTickCount, 0));\n")),
            Op::Qpc => b.push_str(&format!("    CALL(fle_imp_QueryPerformanceCounter, A(&q)); printf(\"{i} %llu\\n\", U(q));\n")),
            Op::Sleep(ms) => b.push_str(&format!("    CALL(fle_imp_Sleep, {ms}u);\n")),
        }
    }
    b
}

#[test]
fn every_time_api_reads_one_clock() {
    use Op::*;
    let ops = [
        Sys, Local, Ft, Tick64, Tick, Qpc, Sys, Sleep(1), Sys, Sleep(1500), Local, Ft, Sleep(86_400_000), Sys, Tick64, Sleep(59_999), Sys, Sleep(0xFFFF_FFFE), Sys, Tick64, Tick, Ft, Qpc,
    ];
    let l = lines("clock", &clock_program(&ops));
    assert_eq!(l, clock_model(&ops));
    // anchors written by hand: the first read of a fresh process is 2021-01-01T00:00:00.000Z, a Friday (the op index
    // 0 is printed first), and the sleeps total 1 + 1500 + 86,400,000 + 59,999 + 4,294,967,294 ms, a little over
    // 50 d 17 h, so the last reads are on 2021-02-20 (Jan 1 + 31 days = Feb 1, + 19 days)
    assert_eq!(l[0], "0 2021 1 5 1 0 0 0 0");
    assert_eq!(l[3], "3 3600000", "GetTickCount64 reads the clock of GetTickCount: 3,600,000 ms at the start");
    let after = st_of(ft_of(2021, 1, 1, 0, 0, 0, 0) + (86_400_000u64 + 1 + 1500 + 59_999 + 4_294_967_294 + 1) * TICKS_PER_MS);
    assert_eq!([after[0], after[1], after[3]], [2021, 2, 20], "the clock moved only by the sleeps and the 1 us per read: {after:?}");
}

#[test]
fn time_does_not_depend_on_the_host() {
    // The same program twice: identical output. (A host clock would differ between the runs.)
    use Op::*;
    let ops = [Sys, Ft, Tick64, Sleep(10), Local, Qpc];
    let a = lines("clock-det-a", &clock_program(&ops));
    let b = lines("clock-det-b", &clock_program(&ops));
    assert_eq!(a, b);
    assert_eq!(a, clock_model(&ops));
}

#[test]
fn a_loop_that_waits_for_the_time_to_change_ends() {
    // GetTickCount64 advances the clock by 1 us per read: waiting for the next millisecond ends after 1000 reads.
    let l = lines(
        "clock-poll",
        r#"
    uint64_t first = CALL(fle_imp_GetTickCount64, 0), n = 1;
    while (CALL(fle_imp_GetTickCount64, 0) == first) n++;
    printf("%llu\n", U(n));
"#,
    );
    // first read at tick T (a whole ms); the clock reads T, T+10, ... ; the ms changes at T+10000 = the 1001st read
    assert_eq!(l, ["1000"]);
}

// ---- 4. Time zone ----

fn hex_bytes(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn time_zone_information_is_utc() {
    // TIME_ZONE_INFORMATION (page): Bias LONG; StandardName[32] WCHAR; StandardDate SYSTEMTIME; StandardBias LONG;
    // DaylightName[32]; DaylightDate; DaylightBias: 4 + 64 + 16 + 4 + 64 + 16 + 4 = 172 bytes.
    // No DST: wMonth 0 in both dates; return value TIME_ZONE_ID_UNKNOWN (0). The environment's zone is UTC: bias 0.
    let l = lines(
        "tzi",
        r#"
    uint8_t b[256]; size_t i; uint64_t r;
    memset(b, 0xAB, sizeof b);
    r = CALL(fle_imp_GetTimeZoneInformation, A(b));
    printf("%llu\n", U(r));
    for (i = 0; i < 172; i++) printf("%02x", b[i]);
    printf("\n");
    for (i = 172; i < 256; i++) if (b[i] != 0xAB) printf("overrun %zu\n", i);
"#,
    );
    let mut want = vec![0u8; 172];
    for (i, c) in "UTC".encode_utf16().enumerate() {
        want[4 + 2 * i..6 + 2 * i].copy_from_slice(&c.to_le_bytes());
    }
    assert_eq!(l, ["0".to_string(), hex_bytes(&want)]);
}

#[test]
fn file_time_to_local_and_back_with_a_utc_zone() {
    // With bias 0 local time equals UTC (UTC = local time + bias).
    let cases = boundary_filetimes();
    let list = cases.iter().map(|v| format!("{v}ull")).collect::<Vec<_>>().join(", ");
    let body = format!(
        r#"
    static const uint64_t c[] = {{ {list} }};
    uint32_t b[2], o[2], p[2];
    size_t i, bad = 0;
    for (i = 0; i < sizeof c / sizeof c[0]; i++) {{
        uint64_t r1, r2;
        b[0] = (uint32_t)c[i]; b[1] = (uint32_t)(c[i] >> 32);
        r1 = CALL(fle_imp_FileTimeToLocalFileTime, A(b), A(o));
        r2 = CALL(fle_imp_LocalFileTimeToFileTime, A(o), A(p));
        if (r1 != 1 || r2 != 1 || FT(o) != c[i] || FT(p) != c[i]) bad++;
    }}
    printf("bad %zu\n", bad);
    b[0] = 0; b[1] = 0x80000000u; SETLE(0);
    printf("high %llu %u\n", U(CALL(fle_imp_FileTimeToLocalFileTime, A(b), A(o))), LE());
    SETLE(0);
    printf("high2 %llu %u\n", U(CALL(fle_imp_LocalFileTimeToFileTime, A(b), A(o))), LE());
"#
    );
    // a FILETIME of 0x8000... or more is outside the range both conversions can represent (assumption: 87)
    assert_eq!(lines("localft", &body), ["bad 0", "high 0 87", "high2 0 87"]);
}

#[test]
fn file_time_to_local_stops_when_output_is_input() {
    assert_aborts(
        "localft-abort",
        "    uint32_t b[2] = { 1, 0 };",
        &[
            ("to-local-same", "CALL(fle_imp_FileTimeToLocalFileTime, A(b), A(b));"),
            ("to-utc-same", "CALL(fle_imp_LocalFileTimeToFileTime, A(b), A(b));"),
            ("to-local-null-in", "CALL(fle_imp_FileTimeToLocalFileTime, 0, A(b));"),
            ("to-utc-null-out", "CALL(fle_imp_LocalFileTimeToFileTime, A(b), 0);"),
        ],
    );
}

#[test]
fn system_time_to_tz_specific_local_time() {
    // local = UTC - bias (UTC = local + bias). Zone NULL: the current zone (UTC here). A zone given without DST dates
    // (wMonth 0 in both) uses Bias only. The result is computed on the FILETIME, so it crosses day, month, leap day and
    // year boundaries; a result below 1601-01-01 fails (assumption: 87).
    struct C {
        bias: Option<i32>,
        utc: (i64, i64, i64, i64, i64, i64, i64),
    }
    let cases = [
        C { bias: None, utc: (2024, 2, 29, 12, 0, 0, 0) },
        C { bias: Some(0), utc: (2021, 1, 1, 0, 0, 0, 0) },
        C { bias: Some(-60), utc: (2024, 2, 28, 23, 30, 0, 0) },   // UTC+1: 00:30 on the leap day
        C { bias: Some(-60), utc: (2023, 2, 28, 23, 30, 0, 0) },   // no leap day: 1 March
        C { bias: Some(-60), utc: (1999, 12, 31, 23, 59, 59, 999) }, // into the next year
        C { bias: Some(300), utc: (2024, 3, 1, 2, 0, 0, 0) },      // UTC-5: back over the leap day
        C { bias: Some(300), utc: (2023, 3, 1, 2, 0, 0, 0) },
        C { bias: Some(300), utc: (2021, 1, 1, 0, 0, 0, 0) },      // back over the year
        C { bias: Some(-840), utc: (2100, 2, 28, 12, 0, 0, 0) },   // UTC+14, 2100 is not a leap year
        C { bias: Some(720), utc: (1601, 1, 1, 12, 0, 0, 0) },     // exactly at the epoch
        C { bias: Some(721), utc: (1601, 1, 1, 12, 0, 0, 0) },     // one minute before the epoch: fails
        C { bias: Some(60), utc: (1601, 1, 1, 0, 0, 0, 0) },       // before the epoch: fails
    ];
    let mut body = String::new();
    for (i, c) in cases.iter().enumerate() {
        let u = c.utc;
        body.push_str(&format!("    {{ uint16_t u[8] = {{ {}, {}, 0, {}, {}, {}, {}, {} }}, l[8]; uint8_t tz[172]; uint64_t r; memset(tz, 0, sizeof tz); memset(l, 0xAB, sizeof l); SETLE(0);\n", u.0, u.1, u.2, u.3, u.4, u.5, u.6));
        match c.bias {
            None => body.push_str("      r = CALL(fle_imp_SystemTimeToTzSpecificLocalTime, 0, A(u), A(l));\n"),
            Some(b) => body.push_str(&format!("      *(int32_t *)tz = {b}; r = CALL(fle_imp_SystemTimeToTzSpecificLocalTime, A(tz), A(u), A(l));\n")),
        }
        body.push_str(&format!("      printf(\"{i} %llu \", U(r)); if (r) PST(l); else printf(\"%u %x\\n\", LE(), l[0]); }}\n"));
    }
    let l = lines("tzspecific", &body);
    for (i, c) in cases.iter().enumerate() {
        let u = c.utc;
        let utc_ft = ft_of(u.0, u.1, u.2, u.3, u.4, u.5, u.6);
        let bias = c.bias.unwrap_or(0) as i64;
        let local = utc_ft as i64 - bias * 60 * TICKS_PER_SEC as i64;
        let want = if local < 0 { format!("{i} 0 87 abab") } else { format!("{i} 1 {}", st_line(st_of(local as u64))) };
        assert_eq!(l[i], want, "case {i}");
    }
    assert_eq!(l[0], "0 1 2024 2 4 29 12 0 0 0");
    assert_eq!(l[2], "2 1 2024 2 4 29 0 30 0 0");
    assert_eq!(l[9], "9 1 1601 1 1 1 0 0 0 0");
    assert!(l[10].starts_with("10 0 87") && l[11].starts_with("11 0 87"));
}

#[test]
fn system_time_to_tz_specific_stops_on_dst_rules_and_nulls() {
    // The TIME_ZONE_INFORMATION page defines transition dates (wMonth, wDay, wDayOfWeek, wYear); applying them is not
    // implemented, so a zone that has them stops the run rather than being treated as a fixed bias.
    assert_aborts(
        "tzspecific-abort",
        "    uint16_t u[8] = { 2021, 7, 0, 1, 0, 0, 0, 0 }, l[8]; uint8_t tz[172]; memset(tz, 0, sizeof tz);",
        &[
            ("dst-daylight-date", "*(uint16_t *)(tz + 152 + 2) = 3; CALL(fle_imp_SystemTimeToTzSpecificLocalTime, A(tz), A(u), A(l));"),
            ("dst-standard-date", "*(uint16_t *)(tz + 68 + 2) = 11; CALL(fle_imp_SystemTimeToTzSpecificLocalTime, A(tz), A(u), A(l));"),
            ("null-universal", "CALL(fle_imp_SystemTimeToTzSpecificLocalTime, 0, 0, A(l));"),
            ("null-local", "CALL(fle_imp_SystemTimeToTzSpecificLocalTime, 0, A(u), 0);"),
        ],
    );
}

#[test]
fn time_zone_and_current_time_stop_on_null() {
    assert_aborts(
        "null-out",
        "",
        &[
            ("gettimezone", "CALL(fle_imp_GetTimeZoneInformation, 0);"),
            ("getsystemtime", "CALL(fle_imp_GetSystemTime, 0);"),
            ("getlocaltime", "CALL(fle_imp_GetLocalTime, 0);"),
            ("getsystemtimeasfiletime", "CALL(fle_imp_GetSystemTimeAsFileTime, 0);"),
        ],
    );
}

// ---- 5. MS-DOS date and time ----

/// Page: date bits 0-4 day, 5-8 month, 9-15 year - 1980; time bits 0-4 second / 2, 5-10 minute, 11-15 hour.
fn dos_decode(date: u32, time: u32) -> (i64, i64, i64, i64, i64, i64) {
    ((1980 + (date >> 9)) as i64, ((date >> 5) & 15) as i64, (date & 31) as i64, (time >> 11) as i64, ((time >> 5) & 63) as i64, ((time & 31) * 2) as i64)
}

#[test]
fn dos_date_time_to_file_time_sweeps_every_date_value() {
    // All 65,536 date values against five time values (three valid and two with an impossible field). The result is
    // UTC-based, no zone is applied (MS-DOS Date and Time page). Invalid fields fail; the page does not list the cases or
    // the error code (inference: impossible fields fail; assumption: 87).
    let times: [(u32, u32, u32); 5] = [(0, 0, 0), (23, 59, 29), (12, 30, 15), (23, 60, 29), (0, 0, 30)];
    let traw: Vec<u32> = times.iter().map(|&(h, m, s2)| (h << 11) | (m << 5) | s2).collect();
    let tl = traw.iter().map(|v| format!("{v}u")).collect::<Vec<_>>().join(", ");
    let body = format!(
        r#"
    static const uint32_t t[] = {{ {tl} }};
    uint32_t d, k; uint32_t o[2];
    for (d = 0; d < 65536; d++) for (k = 0; k < 5; k++) {{
        uint64_t r;
        o[0] = o[1] = 0xCDCDCDCDu; SETLE(0);
        r = CALL(fle_imp_DosDateTimeToFileTime, d, t[k], A(o));
        if (r) printf("1 %llu\n", U(FT(o))); else printf("0 %u %x\n", LE(), o[0]);
    }}
"#
    );
    let l = lines("dos2ft", &body);
    assert_eq!(l.len(), 65536 * 5);
    let mut ok = 0;
    for d in 0..65536u32 {
        for (k, &t) in traw.iter().enumerate() {
            let (y, mo, dd, h, mi, s) = dos_decode(d, t);
            let valid = real_date(y, mo, dd) && h < 24 && mi < 60 && s < 60;
            let want = if valid {
                ok += 1;
                format!("1 {}", ft_of(y, mo, dd, h, mi, s, 0))
            } else {
                "0 87 cdcdcdcd".to_string()
            };
            assert_eq!(l[d as usize * 5 + k], want, "date {d:#06x} time {t:#06x}");
        }
    }
    // 128 years x 365.25 days x 3 valid times, counted by the model (a sanity bound, not an oracle)
    assert!(ok > 128 * 365 * 3 && ok < 128 * 366 * 3 + 1, "{ok}");
    // hand-written anchors: 1980-01-01 00:00:00 is date 0x21, time 0
    assert_eq!(l[0x21 * 5], format!("1 {}", ft_of(1980, 1, 1, 0, 0, 0, 0)));
}

#[test]
fn file_time_to_dos_date_time_sweeps_the_range() {
    // Every day from 1979-12-25 to 2108-01-07, at 00:00:00 and at 23:59:59.9999999. The page: only 1980-01-01 to
    // 2107-12-31 can be represented; outside it the call fails (error code not named: assumption 87). Odd seconds:
    // the page does not say; the model truncates (second / 2) (assumption).
    let first = days_from_civil(1979, 12, 25) - days_from_civil(1601, 1, 1);
    let last = days_from_civil(2108, 1, 7) - days_from_civil(1601, 1, 1);
    let n = (last - first + 1) as u64;
    let body = format!(
        r#"
    uint64_t d; uint32_t b[2]; uint16_t date, time; int k;
    for (d = 0; d < {n}ull; d++) for (k = 0; k < 2; k++) {{
        uint64_t ft = ({first}ull + d) * 864000000000ull + (k ? 863999999999ull : 0ull), r;
        b[0] = (uint32_t)ft; b[1] = (uint32_t)(ft >> 32); date = time = 0xFFFF; SETLE(0);
        r = CALL(fle_imp_FileTimeToDosDateTime, A(b), A(&date), A(&time));
        if (r) printf("1 %u %u\n", date, time); else printf("0 %u %x %x\n", LE(), date, time);
    }}
"#
    );
    let l = lines("ft2dos", &body);
    assert_eq!(l.len(), (n * 2) as usize);
    for d in 0..n {
        for k in 0..2u64 {
            let ft = (first as u64 + d) * 864_000_000_000 + if k == 1 { 863_999_999_999 } else { 0 };
            let s = st_of(ft);
            let want = if (1980..=2107).contains(&s[0]) {
                let date = ((s[0] as u32 - 1980) << 9) | ((s[1] as u32) << 5) | s[3] as u32;
                let time = ((s[4] as u32) << 11) | ((s[5] as u32) << 5) | (s[6] as u32 / 2);
                format!("1 {date} {time}")
            } else {
                "0 87 ffff ffff".to_string()
            };
            assert_eq!(l[(d * 2 + k) as usize], want, "day offset {d} k {k}: {s:?}");
        }
    }
    // anchors written by hand: the first and last representable instants
    let i1980 = (days_from_civil(1980, 1, 1) - days_from_civil(1601, 1, 1) - first) as usize * 2;
    assert_eq!(l[i1980], "1 33 0");
    assert_eq!(l[i1980 - 1], "0 87 ffff ffff", "1979-12-31 23:59:59.9999999");
    let i2107 = (days_from_civil(2107, 12, 31) - days_from_civil(1601, 1, 1) - first) as usize * 2;
    assert_eq!(l[i2107 + 1], format!("1 {} {}", (127 << 9) | (12 << 5) | 31, (23 << 11) | (59 << 5) | 29));
    assert_eq!(l[i2107 + 2], "0 87 ffff ffff", "2108-01-01");
}

#[test]
fn dos_conversions_stop_on_null() {
    assert_aborts(
        "dos-abort",
        "    uint32_t b[2] = { 0, 0 }; uint16_t d, t;",
        &[
            ("dos2ft-null-out", "CALL(fle_imp_DosDateTimeToFileTime, 0x21, 0, 0);"),
            ("ft2dos-null-in", "CALL(fle_imp_FileTimeToDosDateTime, 0, A(&d), A(&t));"),
            ("ft2dos-null-date", "CALL(fle_imp_FileTimeToDosDateTime, A(b), 0, A(&t));"),
            ("ft2dos-null-time", "CALL(fle_imp_FileTimeToDosDateTime, A(b), A(&d), 0);"),
        ],
    );
}

// ---- 6. GetDateFormatW ----

#[test]
fn get_date_format_pictures() {
    // Day/month/year pictures of "Day, Month, Year, and Era Format Pictures": d, dd, M, MM, yy (last two digits, leading
    // zero), yyyy (full year), yyyyy behaves like yyyy; text in single quotes is copied, '' is one quote. The time
    // members and wDayOfWeek of lpDate are ignored/corrected (GetDateFormatEx page). The return value counts the NUL.
    struct C {
        fmt: &'static str,
        st: [u16; 8],
        want: &'static str,
    }
    let cases = [
        C { fmt: "yyyy-MM-dd", st: [2024, 2, 0, 29, 0, 0, 0, 0], want: "2024-02-29" },
        C { fmt: "d/M/yy", st: [2005, 3, 6, 7, 0, 0, 0, 0], want: "7/3/05" },
        C { fmt: "dd'.'MM'.'yyyy", st: [2005, 3, 6, 7, 0, 0, 0, 0], want: "07.03.2005" },
        C { fmt: "'abc''bar' yyyyy", st: [2024, 1, 1, 1, 0, 0, 0, 0], want: "abc'bar 2024" },
        C { fmt: "MM/dd/yyyy", st: [1601, 1, 1, 1, 0, 0, 0, 0], want: "01/01/1601" },
        C { fmt: "yy", st: [2000, 1, 1, 1, 0, 0, 0, 0], want: "00" },
        C { fmt: "yy", st: [1999, 12, 1, 31, 0, 0, 0, 0], want: "99" },
        C { fmt: "yyyy", st: [9999, 12, 5, 31, 23, 59, 59, 999], want: "9999" },
        C { fmt: "yyyy", st: [1601, 1, 1, 1, 0, 0, 0, 0], want: "1601" }, // years below 1601 are invalid (SYSTEMTIME page)
        C { fmt: "d M", st: [2024, 12, 4, 25, 99, 99, 99, 9999], want: "25 12" },
        C { fmt: "", st: [2024, 12, 4, 25, 0, 0, 0, 0], want: "" },
        C { fmt: "(dd)[MM]{yy}", st: [2021, 5, 1, 9, 0, 0, 0, 0], want: "(09)[05]{21}" },
    ];
    let mut body = String::from("    static uint16_t o[64]; uint64_t r;\n");
    for (i, c) in cases.iter().enumerate() {
        let st = c.st.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
        body.push_str(&format!(
            "    {{ uint16_t d[8] = {{ {st} }}; memset(o, 0xEE, sizeof o); r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"{}\"), A(o), 64); printf(\"{i} %llu [%s]\\n\", U(r), AS(o)); }}\n",
            c.fmt
        ));
    }
    let l = lines("datefmt", &body);
    for (i, c) in cases.iter().enumerate() {
        assert_eq!(l[i], format!("{i} {} [{}]", c.want.len() + 1, c.want), "{}", c.fmt);
    }
}

#[test]
fn get_date_format_sizes_locales_and_current_date() {
    let l = lines(
        "datefmt-size",
        r#"
    static uint16_t o[64];
    uint16_t d[8] = { 2024, 2, 0, 29, 0, 0, 0, 0 };
    uint64_t r;
    /* cchDate 0: the size needed, with the NUL; the buffer is not used */
    memset(o, 0xEE, sizeof o);
    r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W("yyyy-MM-dd"), 0, 0); printf("probe %llu\n", U(r));
    r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W("yyyy-MM-dd"), A(o), 0); printf("probe2 %llu %x\n", U(r), o[0]);
    /* too small: 0 and ERROR_INSUFFICIENT_BUFFER (122) */
    SETLE(0); r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W("yyyy-MM-dd"), A(o), 10); printf("small %llu %u\n", U(r), LE());
    SETLE(0); r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W("yyyy-MM-dd"), A(o), 11); printf("fit %llu %u %s\n", U(r), LE(), AS(o));
    /* NULL buffer with room claimed */
    SETLE(0); r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W("yyyy"), 0, 10); printf("nullbuf %llu %u\n", U(r), LE());
    /* the locale ids this environment has: en-US, user default, system default, invariant */
    r = CALL(fle_imp_GetDateFormatW, 0x0400, 0, A(d), W("yyyy"), A(o), 64); printf("user %llu %s\n", U(r), AS(o));
    r = CALL(fle_imp_GetDateFormatW, 0x0800, 0, A(d), W("yyyy"), A(o), 64); printf("sys %llu %s\n", U(r), AS(o));
    r = CALL(fle_imp_GetDateFormatW, 0x007F, 0, A(d), W("yyyy"), A(o), 64); printf("inv %llu %s\n", U(r), AS(o));
    /* invalid date members: fail with ERROR_INVALID_PARAMETER (87) (page); the time members are not checked */
    { uint16_t bad[8] = { 2023, 2, 0, 29, 0, 0, 0, 0 };
      SETLE(0); r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(bad), W("yyyy"), A(o), 64); printf("feb29-2023 %llu %u\n", U(r), LE());
      bad[0] = 2024; bad[1] = 13; bad[3] = 1;
      SETLE(0); r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(bad), W("yyyy"), A(o), 64); printf("month13 %llu %u\n", U(r), LE());
      bad[1] = 1; bad[3] = 0;
      SETLE(0); r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(bad), W("yyyy"), A(o), 64); printf("day0 %llu %u\n", U(r), LE());
      bad[0] = 1600; bad[3] = 1;
      SETLE(0); r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(bad), W("yyyy"), A(o), 64); printf("y1600 %llu %u\n", U(r), LE()); }
    /* lpDate NULL: the current local date; a fresh process reads 2021-01-01 first */
    r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, 0, W("yyyy-MM-dd"), A(o), 64); printf("now %llu %s\n", U(r), AS(o));
    CALL(fle_imp_Sleep, 86400000u * 30u);
    CALL(fle_imp_Sleep, 86400000u * 29u);
    r = CALL(fle_imp_GetDateFormatW, 0x0409, 0, 0, W("yyyy-MM-dd"), A(o), 64); printf("later %llu %s\n", U(r), AS(o));
"#,
    );
    assert_eq!(
        l,
        [
            "probe 11",
            "probe2 11 eeee",
            "small 0 122",
            "fit 11 0 2024-02-29",
            "nullbuf 0 122",
            "user 5 2024",
            "sys 5 2024",
            "inv 5 2024",
            "feb29-2023 0 87",
            "month13 0 87",
            "day0 0 87",
            "y1600 0 87",
            "now 11 2021-01-01",
            // 59 days after 2021-01-01 (31 January days + 28 February days) is 2021-03-01; the 59 days plus the
            // few microseconds of clock reads do not reach the next day
            "later 11 2021-03-01",
        ]
    );
}

#[test]
fn get_date_format_stops_on_what_the_pages_do_not_give() {
    assert_aborts(
        "datefmt-abort",
        "    static uint16_t o[64]; uint16_t d[8] = { 2024, 2, 0, 29, 0, 0, 0, 0 };",
        &[
            ("ddd", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"ddd\"), A(o), 64);"),
            ("dddd", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"dddd\"), A(o), 64);"),
            ("MMM", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"MMM\"), A(o), 64);"),
            ("MMMM", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"MMMM\"), A(o), 64);"),
            ("era-g", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"g\"), A(o), 64);"),
            ("era-gg", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"gg\"), A(o), 64);"),
            ("y-single", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"y\"), A(o), 64);"),
            ("y-triple", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"yyy\"), A(o), 64);"),
            ("y-six", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"yyyyyy\"), A(o), 64);"),
            ("unquoted-letter", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"yyyy x\"), A(o), 64);"),
            ("null-format", "CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), 0, A(o), 64);"),
            ("flags-with-picture", "CALL(fle_imp_GetDateFormatW, 0x0409, 1, A(d), W(\"yyyy\"), A(o), 64);"),
            ("other-locale-de", "CALL(fle_imp_GetDateFormatW, 0x0407, 0, A(d), W(\"yyyy\"), A(o), 64);"),
            ("year-above-9999", "d[0] = 10000; CALL(fle_imp_GetDateFormatW, 0x0409, 0, A(d), W(\"yyyy\"), A(o), 64);"),
        ],
    );
}

// ---- 7. No host state ----

fn strip_comments(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut o = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}

#[test]
fn source_uses_no_host_clock_time_zone_or_environment() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_time.c")).unwrap());
    for bad in ["<time.h>", "clock_gettime", "gettimeofday", "localtime", "gmtime", "mktime", "tzset", "timezone", "getenv", "environ", "fopen", "getpid", "<unistd.h>", "sys/time.h"] {
        assert!(!code.contains(bad), "fle_time.c must not use {bad}");
    }
    // the one source of time is the virtual clock of fle_sys.c
    assert!(code.contains("fle_imp_QueryPerformanceCounter"));
}
