//! Decision 0302 (mt3, PROVISIONAL): dwmapi!DwmGetCompositionTimingInfo (runtime/fle_dwm.c). The expected values are written here from the Microsoft Learn pages DwmGetCompositionTimingInfo and DWM_TIMING_INFO
//! (docs/investigations/mt3-0302/learn/) and from the clock arithmetic of DwmFlush (fle_thread.c: vblank k at ceil(k * 10^7 / hz) ticks since the virtual boot; QueryPerformanceCounter is the tick count, FLE_QPF_HZ = 10^7), not read
//! back from the C code. The member list is Learn's, in Learn's order; the offsets are computed here with 1-byte packing, which the real caller's cbSize (QuickFontCache: 0x124) confirms. The choices the pages leave open are the
//! S-dwm-ti-* of the decision. EVERY ANSWER IS PROVISIONAL: nothing is compared with a real Windows.

mod etxt_common;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    manifest().join("../../runtime")
}
fn seeds() -> PathBuf {
    manifest().join("../../harness/seeds")
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("dwm_timing");
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// DWM_TIMING_INFO (Learn), in the page's order: (member, size in bytes). UNSIGNED_RATIO = two UINT32; QPC_TIME, DWM_FRAME_COUNT and ULONGLONG are 8 bytes ("defined in Dwmapi.h as ULONGLONG"); UINT and UINT32 are 4.
const MEMBERS: [(&str, usize); 40] = [
    ("cbSize", 4), ("rateRefresh", 8), ("qpcRefreshPeriod", 8), ("rateCompose", 8), ("qpcVBlank", 8), ("cRefresh", 8), ("cDXRefresh", 4), ("qpcCompose", 8), ("cFrame", 8), ("cDXPresent", 4),
    ("cRefreshFrame", 8), ("cFrameSubmitted", 8), ("cDXPresentSubmitted", 4), ("cFrameConfirmed", 8), ("cDXPresentConfirmed", 4), ("cRefreshConfirmed", 8), ("cDXRefreshConfirmed", 4), ("cFramesLate", 8),
    ("cFramesOutstanding", 4), ("cFrameDisplayed", 8), ("qpcFrameDisplayed", 8), ("cRefreshFrameDisplayed", 8), ("cFrameComplete", 8), ("qpcFrameComplete", 8), ("cFramePending", 8), ("qpcFramePending", 8),
    ("cFramesDisplayed", 8), ("cFramesComplete", 8), ("cFramesPending", 8), ("cFramesAvailable", 8), ("cFramesDropped", 8), ("cFramesMissed", 8), ("cRefreshNextDisplayed", 8), ("cRefreshNextPresented", 8),
    ("cRefreshesDisplayed", 8), ("cRefreshesPresented", 8), ("cRefreshStarted", 8), ("cPixelsReceived", 8), ("cPixelsDrawn", 8), ("cBuffersEmpty", 8),
];

fn offsets() -> (HashMap<&'static str, (usize, usize)>, usize) {
    let (mut m, mut at) = (HashMap::new(), 0usize);
    for (n, s) in MEMBERS {
        m.insert(n, (at, s));
        at += s;
    }
    (m, at)
}

const PROG: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_win.h"
#include "fle_dwm.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); exit(3); }
int fle_win_get(uint64_t hwnd, fle_win_view *v) { (void)hwnd; (void)v; return 0; }
void fle_win_set_frame_margins(uint64_t hwnd, const int32_t m[4]) { (void)hwnd; (void)m; }
static uint64_t clock_now;
uint64_t fle_sys_clock_ticks(void) { return clock_now; }
void fle_sys_clock_advance_to(uint64_t t) { if (t > clock_now) clock_now = t; }
static uint32_t hz = 60;
uint32_t fle_display_hz(void) { return hz; }

static unsigned char buf[512];
static void call(uint64_t hwnd, int null_ptr, uint32_t cb)
{
    memset(buf, 0xAA, sizeof buf);
    memcpy(buf, &cb, 4);
    R[0] = 0xdead; R[1] = hwnd; R[2] = null_ptr ? 0 : (uint64_t)(uintptr_t)buf;
    fle_imp_DwmGetCompositionTimingInfo();
}
static void dump(const char *tag)
{
    printf("%s hr %llx clock %llu bytes ", tag, (unsigned long long)R[0], (unsigned long long)clock_now);
    for (int i = 0; i < 300; i++) printf("%02x", buf[i]);
    printf("\n");
}
/* usage: exe <hz> <mode> <args...>
 *   plain <now> <cb>              one call at tick <now> with cbSize <cb>
 *   twice <now1> <now2>           a call at <now1>, then at <now2>; the second is printed
 *   hwnd <h>                      a call with hwnd = <h> at tick 1000
 *   null                          pTimingInfo = NULL */
int main(int argc, char **argv)
{
    hz = (uint32_t)strtoul(argv[1], 0, 10);
    if (!strcmp(argv[2], "plain")) { clock_now = strtoull(argv[3], 0, 10); call(0, 0, (uint32_t)strtoul(argv[4], 0, 0)); dump("r"); }
    else if (!strcmp(argv[2], "twice")) { clock_now = strtoull(argv[3], 0, 10); call(0, 0, 0x124); clock_now = strtoull(argv[4], 0, 10); call(0, 0, 0x124); dump("r"); }
    else if (!strcmp(argv[2], "hwnd")) { clock_now = 1000; call(strtoull(argv[3], 0, 0), 0, 0x124); dump("r"); }
    else if (!strcmp(argv[2], "null")) { clock_now = 1000; call(0, 1, 0x124); dump("r"); }
    return 0;
}
"#;

struct Out {
    rc: i32,
    hr: u32,
    clock: u64,
    bytes: Vec<u8>,
    first: String,
}

fn exe() -> PathBuf {
    static ONCE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = work().join(etxt_common::per_process("build"));
        std::fs::create_dir_all(&dir).unwrap();
        let c = dir.join("prog.c");
        std::fs::write(&c, PROG).unwrap();
        let e = dir.join("prog");
        let cc = Command::new("clang")
            .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-pie", "-I"])
            .arg(rt())
            .arg(&c)
            .arg(rt().join("fle_dwm.c"))
            .arg("-o")
            .arg(&e)
            .output()
            .expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        e
    })
    .clone()
}

fn run(args: &[&str]) -> Out {
    let r = Command::new(exe()).args(args).output().expect("run failed");
    let text = String::from_utf8_lossy(&r.stdout).to_string();
    let first = text.lines().next().unwrap_or("").to_string();
    let (mut hr, mut clock, mut bytes) = (0u32, 0u64, Vec::new());
    if let Some(rest) = first.strip_prefix("r hr ") {
        let f: Vec<&str> = rest.split_whitespace().collect();
        hr = u32::from_str_radix(f[0], 16).unwrap();
        clock = f[2].parse().unwrap();
        bytes = (0..f[4].len() / 2).map(|i| u8::from_str_radix(&f[4][2 * i..2 * i + 2], 16).unwrap()).collect();
    }
    Out { rc: r.status.code().unwrap_or(-1), hr, clock, bytes, first }
}

fn member(o: &Out, name: &str) -> u64 {
    let (m, _) = offsets();
    let (at, sz) = m[name];
    let mut v = 0u64;
    for i in (0..sz).rev() {
        v = (v << 8) | o.bytes[at + i] as u64;
    }
    v
}

/// What the contract gives for a display of `hz` hertz at virtual tick `now`, `d` vblanks after the previous call (0 for the first): (member, value). Written from the pages and the clock arithmetic of DwmFlush.
fn expected(hz: u64, now: u64, d: u64) -> Vec<(&'static str, u64)> {
    const QPF: u64 = 10_000_000;
    let k = now * hz / QPF; // the last vblank at or before now: ceil(k * QPF / hz) <= now
    let vb = (k * QPF).div_ceil(hz);
    let period = (QPF + hz / 2) / hz;
    let ratio = hz | (1u64 << 32); // UNSIGNED_RATIO { uiNumerator = hz, uiDenominator = 1 }
    vec![
        ("cbSize", 0x124), ("rateRefresh", ratio), ("qpcRefreshPeriod", period), ("rateCompose", ratio), ("qpcVBlank", vb), ("cRefresh", k), ("cDXRefresh", k & 0xffff_ffff), ("qpcCompose", vb), ("cFrame", k),
        ("cDXPresent", 0), ("cRefreshFrame", k), ("cFrameSubmitted", k), ("cDXPresentSubmitted", 0), ("cFrameConfirmed", k), ("cDXPresentConfirmed", 0), ("cRefreshConfirmed", k), ("cDXRefreshConfirmed", k & 0xffff_ffff),
        ("cFramesLate", 0), ("cFramesOutstanding", 0), ("cFrameDisplayed", k), ("qpcFrameDisplayed", vb), ("cRefreshFrameDisplayed", k), ("cFrameComplete", k), ("qpcFrameComplete", vb), ("cFramePending", k),
        ("qpcFramePending", vb), ("cFramesDisplayed", d), ("cFramesComplete", d), ("cFramesPending", 0), ("cFramesAvailable", 0), ("cFramesDropped", 0), ("cFramesMissed", 0), ("cRefreshNextDisplayed", k + 1),
        ("cRefreshNextPresented", k + 1), ("cRefreshesDisplayed", k), ("cRefreshesPresented", 0), ("cRefreshStarted", 0), ("cPixelsReceived", 0), ("cPixelsDrawn", 0), ("cBuffersEmpty", 0),
    ]
}

#[test]
fn the_structure_is_292_bytes_with_1_byte_packing_which_is_the_cbsize_the_real_caller_passes() {
    // QuickFontCache (f_1e857b0, decision 0299 section 2) sets cbSize = 0x124 = 292. The members of DWM_TIMING_INFO, 40 of them, add up to exactly that with 1-byte packing (the SDK header includes pshpack1.h);
    // with natural alignment (8-byte members at multiples of 8) the structure would be 320 = 0x140 bytes, which no caller passes. rateRefresh (UNSIGNED_RATIO) is at 4, qpcRefreshPeriod at 12 (not 16).
    let (m, total) = offsets();
    assert_eq!(MEMBERS.len(), 40);
    assert_eq!(total, 0x124);
    assert_eq!(m["rateRefresh"], (4, 8));
    assert_eq!(m["qpcRefreshPeriod"], (12, 8));
    assert_eq!(m["cBuffersEmpty"], (284, 8));
    let mut nat = 0usize;
    for (_, s) in MEMBERS {
        nat = nat.div_ceil(s.min(8)) * s.min(8) + s;
    }
    assert_eq!(nat.div_ceil(8) * 8, 0x140, "natural alignment would give 320 bytes");
}

#[test]
fn every_member_has_the_value_the_contract_and_the_virtual_clock_give_at_60_75_and_120_hz() {
    // 60 Hz: tick 12_345_678 (1.2345678 s) is in frame k = floor(12_345_678 * 60 / 10^7) = 74; vblank 74 is at ceil(74 * 10^7 / 60) = 12_333_334 ticks; the period is round(10^7 / 60) = 166_667 ticks.
    for (hz, now) in [(60u64, 12_345_678u64), (75, 12_345_678), (120, 12_345_678), (60, 0), (60, 166_666), (60, 166_667), (120, 83_333), (120, 83_334), (60, 10_000_000)] {
        let o = run(&[&hz.to_string(), "plain", &now.to_string(), "0x124"]);
        assert_eq!((o.rc, o.hr), (0, 0), "{hz} Hz at {now}: {}", o.first);
        assert_eq!(o.bytes.len(), 300);
        for (name, want) in expected(hz, now, 0) {
            assert_eq!(member(&o, name), want, "{hz} Hz at tick {now}: {name}");
        }
        // nothing is written past the structure (the 0xAA fill of the buffer survives)
        assert!(o.bytes[0x124..300].iter().all(|b| *b == 0xAA), "{hz} Hz at {now}: bytes past 0x124");
        // reading the clock moves it on by FLE_CLOCK_READ_TICKS = 10 (decision 0017)
        assert_eq!(o.clock, now + 10, "{hz} Hz at {now}: the clock after the call");
    }
    let o = run(&["60", "plain", "12345678", "0x124"]);
    assert_eq!((member(&o, "cRefresh"), member(&o, "qpcVBlank"), member(&o, "qpcRefreshPeriod")), (74, 12_333_334, 166_667));
}

#[test]
fn the_vblank_is_the_one_dwmflush_waits_for_so_the_two_agree_at_every_boundary() {
    // DwmFlush returns at the tick of vblank k (fle_thread.c: ceil(k * 10^7 / hz)); a call at that tick must say cRefresh = k and qpcVBlank = that tick, and one tick earlier cRefresh = k - 1 (the previous vblank).
    for hz in [60u64, 75, 120, 144] {
        for k in [1u64, 2, 59, 60, 61, 119, 1000] {
            let t = (k * 10_000_000).div_ceil(hz);
            let at = run(&[&hz.to_string(), "plain", &t.to_string(), "0x124"]);
            assert_eq!((member(&at, "cRefresh"), member(&at, "qpcVBlank")), (k, t), "{hz} Hz, vblank {k} at {t}");
            let before = run(&[&hz.to_string(), "plain", &(t - 1).to_string(), "0x124"]);
            assert_eq!(member(&before, "cRefresh"), k - 1, "{hz} Hz, one tick before vblank {k}");
            assert!(member(&before, "qpcVBlank") < t, "{hz} Hz, one tick before vblank {k}");
        }
    }
}

#[test]
fn the_counters_that_the_page_says_are_valid_after_a_second_call_are_the_change_since_the_previous_call() {
    // Learn DWM_TIMING_INFO: cFramesDisplayed "The number of unique frames displayed. This value is valid only after a second call to DwmGetCompositionTimingInfo", cFramesAvailable / cFramesDropped likewise. 60 Hz: the first
    // call at tick 1_000_000 is in frame 6, the second at tick 5_000_000 in frame 30: 24 frames displayed and completed in between, none available or dropped.
    let o = run(&["60", "twice", "1000000", "5000000"]);
    assert_eq!((o.rc, o.hr), (0, 0), "{}", o.first);
    for (name, want) in expected(60, 5_000_000, 24) {
        assert_eq!(member(&o, name), want, "second call: {name}");
    }
    let one = run(&["60", "plain", "5000000", "0x124"]);
    assert_eq!((member(&one, "cFramesDisplayed"), member(&one, "cFramesComplete")), (0, 0), "the first call");
}

#[test]
fn hwnd_not_null_is_e_invalidarg_a_wrong_cbsize_is_e_invalidarg_and_touches_nothing_and_a_null_pointer_stops_by_name() {
    // Learn: "Starting with Windows 8.1, this parameter must be set to NULL. If this parameter is not set to NULL, DwmGetCompositionTimingInfo returns E_INVALIDARG" (0x80070057).
    let o = run(&["60", "hwnd", "0x7F990004"]);
    assert_eq!((o.rc, o.hr), (0, 0x8007_0057), "{}", o.first);
    assert!(o.bytes[4..300].iter().all(|b| *b == 0xAA), "hwnd != NULL: the structure is not written");
    // S-dwm-ti-cbsize: the page only says cbSize "must be set"; any value but 0x124 is E_INVALIDARG and the structure is left as it was
    for cb in ["0", "12", "0x120", "0x128", "0x140", "0xAAAAAAAA"] {
        let o = run(&["60", "plain", "1000", cb]);
        assert_eq!((o.rc, o.hr), (0, 0x8007_0057), "cbSize {cb}: {}", o.first);
        assert!(o.bytes[4..300].iter().all(|b| *b == 0xAA), "cbSize {cb}: the structure is not written");
    }
    // the page gives no behaviour for a NULL pointer: the call stops by name (as DwmIsCompositionEnabled)
    let o = run(&["60", "null"]);
    assert_eq!(o.rc, 3);
    assert!(o.first.starts_with("STOP DwmGetCompositionTimingInfo: pTimingInfo is NULL"), "{}", o.first);
}

#[test]
fn the_import_row_is_one_appended_row_after_the_rows_of_the_earlier_decisions_and_the_runtime_defines_it() {
    // decision 0015: append only; the product's rows are ship5-imports.txt (decision 0230) and, from decision 0302, ship6-imports.txt = ship5 (unchanged, first) + dwmti-imports.txt (last, this row). ci/test-hybrid.sh pins the
    // concatenation; here the row itself.
    let rows = |f: &str| -> Vec<String> {
        std::fs::read_to_string(seeds().join(f)).unwrap().lines().map(|l| l.split('#').next().unwrap().trim().to_string()).filter(|l| !l.is_empty()).collect()
    };
    let d = rows("dwmti-imports.txt");
    assert_eq!(d, ["dwmapi.dll DwmGetCompositionTimingInfo fle_imp_DwmGetCompositionTimingInfo"]);
    let (s5, s6) = (rows("ship5-imports.txt"), rows("ship6-imports.txt"));
    // the later decisions (0319: ScrollWindowEx, ScrollDC, ImageList drag, the processor count) append their rows after this one (append only, decision 0015): the dwmti row is the first row after ship5's
    assert!(s6.len() >= s5.len() + 1);
    assert_eq!(s6[..s5.len()], s5[..], "the rows of ship5 are first and unchanged");
    assert_eq!(s6[s5.len()], d[0], "the new row is the first after ship5's");
    assert!(!s5.iter().any(|r| r.contains("DwmGetCompositionTimingInfo")), "no earlier file has the row");
    let c = std::fs::read_to_string(rt().join("fle_dwm.c")).unwrap();
    let h = std::fs::read_to_string(rt().join("fle_dwm.h")).unwrap();
    assert!(c.contains("void fle_imp_DwmGetCompositionTimingInfo(void)\n{"));
    assert!(h.contains("void fle_imp_DwmGetCompositionTimingInfo(void);"));
}
