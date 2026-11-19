//! Tests for the first batch of wall responses (docs/decisions/0098-wall-responses-batch1.md; census walls of decision 0097, dsp
//! configuration): msacm32!acmMetrics, shell32!SHGetMalloc (and the IMalloc object it returns), user32!EnumDisplaySettingsW,
//! user32!GetClipCursor, gdi32!EnumFontFamiliesExW, version!GetFileVersionInfoSizeW, SystemParametersInfoW(SPI_GETWORKAREA) and
//! GetSystemMetrics(SM_DIGITIZER).
//!
//! Expected values are written here from the decision (the Microsoft Learn pages named in it, the choices of the decision and the earlier
//! decisions 0053, 0057, 0058, 0065, 0066, 0018), not read back from the C code. EVERY ANSWER IS PROVISIONAL (decision 0098): nothing is
//! compared with a real Windows (the host is Linux), and no display, file, registry, locale, font, audio device, clock, thread or process of
//! the host is read.
//!
//! One C driver (the scenarios below) is linked with the runtime files these answers live in; the guest's memory is the driver's memory.
//! The driver's SIGABRT handler prints "ABORT calls <n> BUF <hex>" so that a stop shows that no callback was made and nothing was written.

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallr/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const DRIVER: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_callback.h"
#include "fle_modules.h"
#include "fle_gdc.h"
#include "fle_gdi.h"
#include "fle_monitor.h"
#include "fle_cursor.h"
#include "fle_spi.h"
#include "fle_sysmetrics.h"
#include "fle_winmm.h"
#include "fle_ole.h"
#include "fle_vfs.h"

uint64_t R[16];
static uint64_t stk[0x2000] __attribute__((aligned(4096)));
#define E0 ((uint64_t)(uintptr_t)stk + 0x8000 + 8)
#define TOP ((uint64_t)(uintptr_t)stk + sizeof stk)
#define RET0 UINT64_C(0xdead0000)
#define PATTERN UINT64_C(0xA5A5A5A5A5A5A5A5)
#define U(x) ((unsigned long long)(x))
#define VA_CB 0x2000
#define VA_SH UINT64_C(0x7ff900001000)

static unsigned g_calls;
static uint64_t g_a[4], g_cb_sp, g_retv;
static uint8_t g_elf[352], g_ntm[100];
static const uint8_t *g_watch;
static size_t g_watch_n;

static void cb_font(void)
{
    g_calls++;
    g_a[0] = R[1]; g_a[1] = R[2]; g_a[2] = R[8]; g_a[3] = R[9];
    g_cb_sp = R[4];
    memcpy(g_elf, (void *)(uintptr_t)R[1], 348);
    memcpy(g_ntm, (void *)(uintptr_t)R[2], 100);
    R[0] = g_retv;
    R[1] = R[2] = R[8] = R[9] = 0;
}

const fle_fn_entry fle_dispatch_table[] = {
    { VA_CB, cb_font },
    { VA_SH, fle_imp_SHGetMalloc },
    { VA_SH + 0x10, fle_imp_IMalloc_Release },
    { VA_SH + 0x20, fle_imp_IMalloc_Free },
    { VA_SH + 0x30, fle_imp_IMalloc_DidAlloc },
    { VA_SH + 0x40, fle_imp_IMalloc_unimplemented },
};
const uint64_t fle_dispatch_count = 6;
const fle_resp_entry fle_resp_table[] = {
    { "shell32.dll", "SHGetMalloc", VA_SH, fle_imp_SHGetMalloc },
    { "shell32.dll", "IMalloc::Release", VA_SH + 0x10, fle_imp_IMalloc_Release },
    { "shell32.dll", "IMalloc::Free", VA_SH + 0x20, fle_imp_IMalloc_Free },
    { "shell32.dll", "IMalloc::DidAlloc", VA_SH + 0x30, fle_imp_IMalloc_DidAlloc },
    { "shell32.dll", "IMalloc::unimplemented", VA_SH + 0x40, fle_imp_IMalloc_unimplemented },
};
const uint64_t fle_resp_count = 5;

static void hex(const uint8_t *p, size_t n) { size_t i; for (i = 0; i < n; i++) printf("%02x", p[i]); }
static void on_abort(int s)
{
    char out[64];
    int k;
    (void)s;
    k = snprintf(out, sizeof out, "ABORT calls %u BUF ", g_calls);
    (void)!write(1, out, (size_t)k);
    if (g_watch) {
        size_t i;
        for (i = 0; i < g_watch_n; i++) {
            char h[2] = { "0123456789abcdef"[g_watch[i] >> 4], "0123456789abcdef"[g_watch[i] & 15] };
            (void)!write(1, h, 2);
        }
    }
    (void)!write(1, "\n", 1);
    _exit(134);
}
static uint64_t u(const char *s) { return strtoull(s, 0, 0); }
static void fill(void) { size_t i; for (i = 0; i < sizeof stk / 8; i++) stk[i] = PATTERN; }
static uint16_t *W(const char *s) { size_t n = strlen(s), i; uint16_t *w = calloc(n + 1, 2); for (i = 0; i < n; i++) w[i] = (uint8_t)s[i]; return w; }
static uint64_t le(void) { fle_imp_GetLastError(); return R[0]; }
static void dispatch(uint64_t va, uint64_t a1, uint64_t a2) { R[1] = a1; R[2] = a2; R[4] = E0; fle_dispatch(va); }

int main(int argc, char **argv)
{
    const char *t = argc > 1 ? argv[1] : "";
    int i;
    setvbuf(stdout, NULL, _IONBF, 0);
    signal(SIGABRT, on_abort);
    fill();
    R[4] = E0;
    fle_st(E0, 8, RET0);
    fle_set_last_error(0x1234);

    if (!strcmp(t, "acm")) {                 /* hao metric buf|null */
        uint32_t b[3] = { 0xAAAAAAAAu, 0xBBBBBBBBu, 0xCCCCCCCCu };
        g_watch = (const uint8_t *)b; g_watch_n = sizeof b;
        R[1] = u(argv[2]); R[2] = u(argv[3]); R[8] = !strcmp(argv[4], "buf") ? (uint64_t)(uintptr_t)&b[1] : 0;
        R[0] = 0x99;
        fle_imp_acmMetrics();
        printf("ret %llx buf %x %x %x le %llx\n", U(R[0]), b[0], b[1], b[2], U(le()));
    } else if (!strcmp(t, "spiwa")) {        /* action uiParam fWinIni buf|null */
        uint32_t b[6] = { 0x11111111u, 0xAAAAAAAAu, 0xAAAAAAAAu, 0xAAAAAAAAu, 0xAAAAAAAAu, 0x22222222u };
        g_watch = (const uint8_t *)b; g_watch_n = sizeof b;
        R[1] = u(argv[2]); R[2] = u(argv[3]); R[9] = u(argv[4]); R[8] = !strcmp(argv[5], "buf") ? (uint64_t)(uintptr_t)&b[1] : 0;
        fle_imp_SystemParametersInfoW();
        printf("ret %llx rect %d %d %d %d guards %x %x le %llx\n", U(R[0]), (int)b[1], (int)b[2], (int)b[3], (int)b[4], b[0], b[5], U(le()));
    } else if (!strcmp(t, "spi-vs-monitor")) {
        uint8_t mi[0x28]; uint32_t r[4];
        memset(mi, 0xAA, sizeof mi); fle_st((uint64_t)(uintptr_t)mi, 4, 0x28);
        R[1] = FLE_MONITOR_HANDLE; R[2] = (uint64_t)(uintptr_t)mi; fle_imp_GetMonitorInfoW();
        printf("monitor-ret %llx work %d %d %d %d\n", U(R[0]), (int)fle_ld((uint64_t)(uintptr_t)mi + 20, 4) , (int)fle_ld((uint64_t)(uintptr_t)mi + 24, 4), (int)fle_ld((uint64_t)(uintptr_t)mi + 28, 4), (int)fle_ld((uint64_t)(uintptr_t)mi + 32, 4));
        R[1] = 0x30; R[2] = 0; R[8] = (uint64_t)(uintptr_t)r; R[9] = 0; fle_imp_SystemParametersInfoW();
        printf("spi-ret %llx work %d %d %d %d\n", U(R[0]), (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
    } else if (!strcmp(t, "sm")) {           /* index */
        R[1] = u(argv[2]); R[0] = 0x99;
        fle_imp_GetSystemMetrics();
        printf("ret %d le %llx\n", (int)R[0], U(le()));
    } else if (!strcmp(t, "eds")) {          /* name(null|x) mode dmSize dmDriverExtra dm(buf|null) */
        uint8_t b[0xE8 + 16];
        uint64_t dm = (uint64_t)(uintptr_t)b + 8;
        uint16_t dummy[2] = { 'x', 0 };
        memset(b, 0xAB, sizeof b);
        g_watch = b; g_watch_n = sizeof b;
        fle_st(dm + 68, 2, u(argv[4])); fle_st(dm + 70, 2, u(argv[5]));
        R[1] = !strcmp(argv[2], "null") ? 0 : (uint64_t)(uintptr_t)dummy; R[2] = u(argv[3]); R[8] = !strcmp(argv[6], "buf") ? dm : 0;
        fle_imp_EnumDisplaySettingsW();
        printf("ret %llx le %llx\n", U(R[0]), U(le()));
        hex(b, sizeof b); printf("\n");
    } else if (!strcmp(t, "clip")) {         /* buf|null */
        uint32_t b[6] = { 0x11111111u, 0xAAAAAAAAu, 0xAAAAAAAAu, 0xAAAAAAAAu, 0xAAAAAAAAu, 0x22222222u };
        g_watch = (const uint8_t *)b; g_watch_n = sizeof b;
        R[1] = !strcmp(argv[2], "buf") ? (uint64_t)(uintptr_t)&b[1] : 0;
        fle_imp_GetClipCursor();
        printf("ret %llx rect %d %d %d %d guards %x %x le %llx\n", U(R[0]), (int)b[1], (int)b[2], (int)b[3], (int)b[4], b[0], b[5], U(le()));
    } else if (!strcmp(t, "fvs")) {          /* path buf|null [mkdir=P] [mkfile=P] */
        uint32_t h = 0x55555555u;
        g_watch = (const uint8_t *)&h; g_watch_n = 4;
        for (i = 4; i < argc; i++) {
            if (!strncmp(argv[i], "mkdir=", 6)) {
                R[1] = (uint64_t)(uintptr_t)W(argv[i] + 6); R[2] = 0; R[4] = E0; fle_imp_CreateDirectoryW();
                if (R[0] != 1) { printf("mkdir failed\n"); return 2; }
            } else if (!strncmp(argv[i], "mkfile=", 7)) {
                uint64_t hf;
                R[1] = (uint64_t)(uintptr_t)W(argv[i] + 7); R[2] = 0x40000000; R[8] = 0; R[9] = 0; R[4] = E0;
                fle_st(E0 + 0x28, 8, 2); fle_st(E0 + 0x30, 8, 0x80); fle_st(E0 + 0x38, 8, 0);
                fle_imp_CreateFileW();
                hf = R[0];
                if (hf == FLE_INVALID_HANDLE_VALUE) { printf("mkfile failed\n"); return 2; }
                R[1] = hf; R[4] = E0; fle_imp_CloseHandle();
            }
        }
        fill(); R[4] = E0; fle_st(E0, 8, RET0);
        fle_set_last_error(0x1234);
        R[1] = !strcmp(argv[2], "null") ? 0 : (uint64_t)(uintptr_t)W(argv[2]);
        R[2] = !strcmp(argv[3], "buf") ? (uint64_t)(uintptr_t)&h : 0;
        R[0] = 0x99;
        fle_imp_GetFileVersionInfoSizeW();
        printf("ret %llx handle %x le %llx\n", U(R[0]), h, U(le()));
    } else if (!strcmp(t, "sh-basic")) {
        uint64_t p = 0, *o, *vt;
        const char *names[] = { "unimplemented", "Release", "Free", "DidAlloc" };
        R[1] = (uint64_t)(uintptr_t)&p; R[4] = E0;
        fle_imp_SHGetMalloc();
        printf("ret %llx set %d refs %u le %llx\n", U(R[0]), (int)(p != 0), fle_ole_shell_malloc_refs(), U(le()));
        o = (uint64_t *)(uintptr_t)p; vt = (uint64_t *)(uintptr_t)o[0];
        for (i = 0; i < 9; i++) {
            int k = vt[i] == VA_SH + 0x10 ? 1 : vt[i] == VA_SH + 0x20 ? 2 : vt[i] == VA_SH + 0x30 ? 3 : vt[i] == VA_SH + 0x40 ? 0 : -1;
            printf("slot %d %s\n", i, k >= 0 ? names[k] : "?");
        }
    } else if (!strcmp(t, "sh-ops")) {
        uint64_t p = 0, *o, *vt, blk, local = 0;
        R[1] = (uint64_t)(uintptr_t)&p; R[4] = E0; fle_imp_SHGetMalloc();
        o = (uint64_t *)(uintptr_t)p; vt = (uint64_t *)(uintptr_t)o[0];
        R[1] = 24; R[4] = E0; fle_imp_CoTaskMemAlloc(); blk = R[0];
        printf("live %llu\n", U(fle_ole_task_live()));
        dispatch(vt[7], p, blk); printf("didalloc-live %llx\n", U(R[0]));
        dispatch(vt[7], p, 0); printf("didalloc-null %llx\n", U(R[0]));
        dispatch(vt[7], p, (uint64_t)(uintptr_t)&local); printf("didalloc-stack %llx\n", U(R[0]));
        dispatch(vt[5], p, 0); printf("free-null live %llu\n", U(fle_ole_task_live()));
        dispatch(vt[5], p, blk); printf("free-live live %llu\n", U(fle_ole_task_live()));
        dispatch(vt[7], p, blk); printf("didalloc-freed %llx\n", U(R[0]));
        dispatch(vt[2], p, 0); printf("release %llx refs %u\n", U(R[0]), fle_ole_shell_malloc_refs());
        printf("le %llx\n", U(le()));
    } else if (!strcmp(t, "sh-twice")) {
        uint64_t p1 = 0, p2 = 0, *o, *vt;
        R[1] = (uint64_t)(uintptr_t)&p1; R[4] = E0; fle_imp_SHGetMalloc();
        R[1] = (uint64_t)(uintptr_t)&p2; R[4] = E0; fle_imp_SHGetMalloc();
        printf("same %d refs %u\n", (int)(p1 == p2), fle_ole_shell_malloc_refs());
        o = (uint64_t *)(uintptr_t)p1; vt = (uint64_t *)(uintptr_t)o[0];
        dispatch(vt[2], p1, 0); printf("release %llx\n", U(R[0]));
        dispatch(vt[2], p1, 0); printf("release %llx\n", U(R[0]));
        fflush(stdout);
        dispatch(vt[2], p1, 0); printf("not reached\n");
    } else if (!strcmp(t, "sh-stop")) {      /* what */
        uint64_t p = 0, *o, *vt, local = 0;
        const char *w = argv[2];
        if (!strcmp(w, "release-before-get")) { dispatch(VA_SH + 0x10, 0x1000, 0); return 0; }
        if (!strcmp(w, "null-pp")) { R[1] = 0; R[4] = E0; fle_imp_SHGetMalloc(); return 0; }
        R[1] = (uint64_t)(uintptr_t)&p; R[4] = E0; fle_imp_SHGetMalloc();
        o = (uint64_t *)(uintptr_t)p; vt = (uint64_t *)(uintptr_t)o[0];
        if (!strcmp(w, "free-stray")) dispatch(vt[5], p, (uint64_t)(uintptr_t)&local);
        else if (!strcmp(w, "release-wrong-this")) dispatch(vt[2], p + 8, 0);
        else if (!strncmp(w, "slot", 4)) dispatch(vt[atoi(w + 4)], p, 0);
        printf("not reached\n");
    } else if (!strcmp(t, "efe")) {          /* dc(live|null|released|junk) charset face|- pitch proc(1|0) flags retv */
        uint64_t hdc = 0, sp0 = E0, a, bad = 0;
        uint8_t lf[92];
        uint16_t *wf;
        memset(lf, 0, sizeof lf);
        if (!strcmp(argv[2], "live") || !strcmp(argv[2], "released")) {
            R[1] = 0; R[4] = E0; fle_imp_GetDC(); hdc = R[0];
            if (!strcmp(argv[2], "released")) { R[1] = 0; R[2] = hdc; R[4] = E0; fle_imp_ReleaseDC(); }
        } else if (!strcmp(argv[2], "junk")) hdc = 0x12345678;
        lf[23] = (uint8_t)u(argv[3]);
        if (strcmp(argv[4], "-")) { wf = W(argv[4]); memcpy(lf + 28, wf, 2 * strlen(argv[4])); }
        lf[27] = (uint8_t)u(argv[5]);
        fill(); R[4] = sp0; fle_st(sp0, 8, RET0);
        g_watch = lf; g_watch_n = sizeof lf;
        for (i = 3; i < 16; i++) if (i != 4) R[i] = 0x5000 + (uint64_t)i;
        R[1] = hdc; R[2] = (uint64_t)(uintptr_t)lf; R[8] = !strcmp(argv[6], "1") ? VA_CB : 0; R[9] = UINT64_C(0x1122334455667788);
        fle_st(sp0 + 0x28, 8, u(argv[7]));
        g_retv = u(argv[8]);
        fle_set_last_error(0x1234);
        fle_imp_EnumFontFamiliesExW();
        printf("ret %llx calls %u rsp-kept %d api-ret %llx\n", U(R[0]), g_calls, (int)(R[4] == sp0), U(fle_ld(sp0, 8)));
        for (a = sp0 + 0x30; a + 8 <= TOP; a += 8) if (fle_ld(a, 8) != PATTERN) bad++;
        printf("above-changed %llu\n", U(bad));
        printf("args-r8-r9 %llx %llx\n", U(g_a[2]), U(g_a[3]));
        printf("elf-aligned %d ntm-after-elf %d struct-below-api-frame %d cb-sp-below-structs %d\n", (int)(g_a[0] % 16 == 0), (int)(g_a[1] == g_a[0] + 0x160),
               (int)(g_a[1] + 100 <= sp0), (int)(g_cb_sp + 8 + 32 <= g_a[0]));
        printf("elf "); hex(g_elf, 348); printf("\n");
        printf("ntm "); hex(g_ntm, 100); printf("\n");
        for (i = 3; i < 16; i++) if ((i == 3 || (i >= 5 && i <= 7) || i >= 12) && R[i] != 0x5000 + (uint64_t)i) printf("changed R[%d]\n", i);   /* rbx rbp rsi rdi r12-r15 */
        printf("le %llx\n", U(le()));
    } else {
        printf("unknown %s\n", t);
        return 3;
    }
    return 0;
}
"#;

const RUNTIME_FILES: [&str; 15] = [
    "fle_winmm.c",
    "fle_spi.c",
    "fle_sysmetrics.c",
    "fle_monitor.c",
    "fle_callback.c",
    "fle_cursor.c",
    "fle_vfs.c",
    "fle_ole.c",
    "fle_gdi.c",
    "fle_raster.c", // decision 0126: fle_gdi.c calls the pixel model (fle_r_*, fle_raster_*)
    "fle_gdc.c",
    "fle_imports.c",
    "fle_env.c",
    "fle_reg.c",
    "fle_host.c",
];

fn driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let dir = work_dir();
        let tag = etxt_common::per_process("wallr-driver");
        let rt = runtime_dir();
        let main_c = dir.join(format!("{tag}-main.c"));
        std::fs::write(&main_c, DRIVER).unwrap();
        let exe = dir.join(&tag);
        let mut cmd = Command::new("clang");
        cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"]).arg(&rt);
        for f in RUNTIME_FILES {
            cmd.arg(rt.join(f));
        }
        let cc = cmd.arg("-o").arg(&exe).arg(&main_c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        exe
    })
}

struct Out {
    stdout: String,
    stderr: String,
    code: Option<i32>,
    signal: Option<i32>,
}

fn run(args: &[&str]) -> Out {
    let r = Command::new(driver()).args(args).env("TMPDIR", work_dir().join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        code: r.status.code(),
        signal: r.status.signal(),
    }
}

/// The lines of a run that ended normally.
fn lines(args: &[&str]) -> Vec<String> {
    let o = run(args);
    assert!(o.code == Some(0), "{args:?}: exit {:?} signal {:?}: {}\n{}", o.code, o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// A stop: the driver's SIGABRT handler ran (exit 134), `fle: <needle>` is in stderr, `calls` callbacks were made and the watched buffer
/// holds `buf` (hex) at the end. Returns the stdout lines printed before the stop.
fn stops(args: &[&str], needle: &str, calls: u32, buf: &str) -> Vec<String> {
    let o = run(args);
    assert_eq!(o.code, Some(134), "{args:?}: not a stop: exit {:?} signal {:?}: {}\n{}", o.code, o.signal, o.stderr, o.stdout);
    assert!(o.stderr.contains("fle: ") && o.stderr.contains(needle), "{args:?}: no {needle:?} in {}", o.stderr);
    let abort = o.stdout.lines().find(|l| l.starts_with("ABORT ")).unwrap_or_else(|| panic!("{args:?}: no ABORT line: {}", o.stdout));
    assert_eq!(abort, format!("ABORT calls {calls} BUF {buf}"), "{args:?}");
    o.stdout.lines().filter(|l| !l.starts_with("ABORT ")).map(|s| s.to_string()).collect()
}

fn le_hex(words: &[u32]) -> String {
    words.iter().map(|w| w.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>()).collect()
}

fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(runtime_dir().join(file)).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
    if let Some(r) = rest.strip_prefix("0x") {
        let tok: String = r.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        return u64::from_str_radix(&tok, 16).unwrap();
    }
    rest.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

// ---- 1. acmMetrics (unit 170) ----

/// Learn (acmMetrics): ACM_METRIC_MAX_SIZE_FORMAT "Returned value is the size of the largest WAVEFORMATEX structure"; the pMetric buffer is a DWORD;
/// "Returns zero if successful". WAVEFORMATEX = WORD, WORD, DWORD, DWORD, WORD, WORD, WORD = 2 + 2 + 4 + 4 + 2 + 2 + 2 = 18 (Learn). 50 is the number
/// of the Windows SDK header (mingw-w64 msacm.h), and what the guest passes (`mov edx, 0x32` at 0x672610, 0x6726e2, 0x67405d).
const ACM_MAX_SIZE_FORMAT: u64 = 50;
const WAVEFORMATEX: u32 = 2 + 2 + 4 + 4 + 2 + 2 + 2;

#[test]
fn acm_metrics_max_size_format_is_the_size_of_a_waveformatex() {
    // the DWORD is written, its neighbours are not; 0 (MMSYSERR_NOERROR) is returned; the last error is left alone
    let l = lines(&["acm", "0", &ACM_MAX_SIZE_FORMAT.to_string(), "buf"]);
    assert_eq!(l, [format!("ret 0 buf aaaaaaaa {WAVEFORMATEX:x} cccccccc le 1234")]);
    assert_eq!(WAVEFORMATEX, 18);
}

#[test]
fn acm_metrics_reads_uint_arguments_as_32_bits_but_hao_as_a_whole_handle() {
    // uMetric is a UINT: garbage in the upper half of rdx does not matter
    let l = lines(&["acm", "0", "0xFFFFFFFF00000032", "buf"]);
    assert_eq!(l, ["ret 0 buf aaaaaaaa 12 cccccccc le 1234"]);
    // hao is a handle (64 bits): a non-NULL value in the upper half is not NULL
    stops(&["acm", "0x100000000", "50", "buf"], "acmMetrics: hao is not NULL", 0, &le_hex(&[0xAAAA_AAAA, 0xBBBB_BBBB, 0xCCCC_CCCC]));
}

#[test]
fn acm_metrics_other_forms_stop_by_name_and_write_nothing() {
    let untouched = le_hex(&[0xAAAA_AAAA, 0xBBBB_BBBB, 0xCCCC_CCCC]);
    stops(&["acm", "0", "49", "buf"], "acmMetrics: uMetric is not ACM_METRIC_MAX_SIZE_FORMAT (50)", 0, &untouched);
    stops(&["acm", "0", "0", "buf"], "acmMetrics: uMetric is not ACM_METRIC_MAX_SIZE_FORMAT (50)", 0, &untouched);
    stops(&["acm", "0x1234", "50", "buf"], "acmMetrics: hao is not NULL", 0, &untouched);
    stops(&["acm", "0", "50", "null"], "acmMetrics: pMetric is NULL", 0, &untouched);
}

// ---- 2. SystemParametersInfoW(SPI_GETWORKAREA) (unit 275) ----

/// Learn (SystemParametersInfoW, SPI_GETWORKAREA 0x0030): "The pvParam parameter must point to a RECT structure that receives the coordinates of the work
/// area"; uiParam and fWinIni are "zero if not otherwise indicated". Decision 0065 wrote the condition: the same four numbers as rcWork of GetMonitorInfoW
/// = the monitor rectangle (0, 0, 1920, 1080) of decisions 0053 and 0057.
#[test]
fn spi_getworkarea_is_the_work_area_of_the_one_monitor() {
    let l = lines(&["spiwa", "0x30", "0", "0", "buf"]);
    assert_eq!(l, ["ret 1 rect 0 0 1920 1080 guards 11111111 22222222 le 1234"]);
}

#[test]
fn spi_getworkarea_reads_the_uint_arguments_as_32_bits() {
    let l = lines(&["spiwa", "0xAAAAAAAA00000030", "0xFFFFFFFF00000000", "0x1234567800000000", "buf"]);
    assert_eq!(l, ["ret 1 rect 0 0 1920 1080 guards 11111111 22222222 le 1234"]);
}

#[test]
fn spi_getworkarea_is_the_same_rectangle_as_rcwork_of_getmonitorinfow() {
    // the condition of decision 0065: change one, change both
    let l = lines(&["spi-vs-monitor"]);
    assert_eq!(l, ["monitor-ret 1 work 0 0 1920 1080", "spi-ret 1 work 0 0 1920 1080"]);
}

#[test]
fn spi_getworkarea_stops_for_the_forms_the_page_does_not_define() {
    let untouched = le_hex(&[0x1111_1111, 0xAAAA_AAAA, 0xAAAA_AAAA, 0xAAAA_AAAA, 0xAAAA_AAAA, 0x2222_2222]);
    stops(&["spiwa", "0x30", "0", "0", "null"], "SPI_GETWORKAREA with pvParam NULL", 0, &untouched);
    stops(&["spiwa", "0x30", "1", "0", "buf"], "SPI_GETWORKAREA with uiParam not zero", 0, &untouched);
    stops(&["spiwa", "0x30", "0", "3", "buf"], "SPI_GETWORKAREA with fWinIni not zero", 0, &untouched);
    // its neighbours are not answered by this decision
    stops(&["spiwa", "0x2F", "0", "0", "buf"], "uiAction 0x2f is not answered", 0, &untouched);
    stops(&["spiwa", "0x31", "0", "0", "buf"], "uiAction 0x31 is not answered", 0, &untouched);
    stops(&["spiwa", "0x48", "0", "0", "buf"], "SPI_GETANIMATION", 0, &untouched);
}

// ---- 3. GetSystemMetrics(SM_DIGITIZER) (unit 276) ----

/// Learn (GetSystemMetrics): SM_DIGITIZER 94: "The return value is a bitmask that specifies the type of digitizer input supported by the device" and 0
/// "otherwise". This environment has a mouse (decision 0057) and no touch or pen digitizer: no NID_* bit.
#[test]
fn sm_digitizer_is_zero_and_the_last_error_is_left_alone() {
    assert_eq!(lines(&["sm", "94"]), ["ret 0 le 1234"]);
    // index is an int: the upper half of rcx does not count
    assert_eq!(lines(&["sm", "0xFFFFFFFF0000005E"]), ["ret 0 le 1234"]);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_IDX_DIGITIZER"), 94);
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_DIGITIZER"), 0);
}

#[test]
fn the_other_metrics_of_the_same_guest_function_are_answered_by_decision_0107() {
    // f_80e360 asks 94 (answered by decision 0098), then 86, 0x2003 and 0x2004, and f_80e3f0 asks 95. Decision 0098 did not decide them (they
    // stopped by name here); decision 0107 (PROVISIONAL) answers them: 0, 1, 0 and 0. The reasons and the other checks are in tests/wall_r2.rs.
    for (idx, v) in [("86", 0), ("95", 0), ("0x2003", 1), ("0x2004", 0)] {
        assert_eq!(lines(&["sm", idx]), [format!("ret {v} le 1234")], "{idx}");
    }
}

// ---- 4. EnumDisplaySettingsW (unit 246) ----

/// DEVMODEW (Learn): WCHAR dmDeviceName[32] = 64 bytes, then dmSpecVersion, dmDriverVersion, dmSize, dmDriverExtra (WORD x 4) -> dmSize at 68,
/// dmDriverExtra at 70, dmFields (DWORD) at 72, a 16-byte union at 76, five shorts (92..102), WCHAR dmFormName[32] (102..166), dmLogPixels (WORD)
/// at 166, dmBitsPerPel at 168, dmPelsWidth at 172, dmPelsHeight at 176, dmDisplayFlags at 180, dmDisplayFrequency at 184 and five more DWORDs
/// ... dmPanningHeight at 216: 220 = 0xDC bytes. (The sums are ours; the guest writes dmSize 0xDC at buffer + 0x44 and reads the frequency at
/// buffer + 0xB8 = 184.) The DM_ bits are the Windows SDK header's (mingw-w64 wingdi.h).
const DM_FIELDS: u32 = 0x0004_0000 | 0x0008_0000 | 0x0010_0000 | 0x0020_0000 | 0x0040_0000;

/// The 232 + 16 bytes of the "eds" scenario after an answered call: 8 guard bytes of 0xAB, the 220 bytes of the structure, 20 guard bytes of 0xAB
/// (the buffer is 0xE8 + 16 bytes long, the structure starts at 8).
fn eds_expected(extra: u16) -> String {
    let mut dm = vec![0u8; 220];
    dm[68..70].copy_from_slice(&0xDCu16.to_le_bytes());
    dm[70..72].copy_from_slice(&extra.to_le_bytes());
    dm[72..76].copy_from_slice(&DM_FIELDS.to_le_bytes());
    dm[168..172].copy_from_slice(&32u32.to_le_bytes());
    dm[172..176].copy_from_slice(&1920u32.to_le_bytes());
    dm[176..180].copy_from_slice(&1080u32.to_le_bytes());
    dm[180..184].copy_from_slice(&0u32.to_le_bytes());
    dm[184..188].copy_from_slice(&60u32.to_le_bytes());
    let mut all = vec![0xABu8; 8];
    all.extend(dm);
    all.extend(vec![0xABu8; 0xE8 + 16 - 8 - 220]);
    all.iter().map(|b| format!("{b:02x}")).collect()
}

fn eds_untouched(extra: u16, size: u16) -> String {
    let mut all = vec![0xABu8; 0xE8 + 16];
    all[8 + 68..8 + 70].copy_from_slice(&size.to_le_bytes());
    all[8 + 70..8 + 72].copy_from_slice(&extra.to_le_bytes());
    all.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn enum_display_settings_current_settings_is_the_virtual_display() {
    let l = lines(&["eds", "null", "0xFFFFFFFF", "0xdc", "0", "buf"]);
    assert_eq!(l[0], "ret 1 le 1234");
    assert_eq!(l[1], eds_expected(0));
    assert_eq!(DM_FIELDS, 0x007C_0000);
}

#[test]
fn enum_display_settings_keeps_the_callers_inputs_and_zeroes_the_rest() {
    // dmDriverExtra is the caller's input and is kept; every other member that is not one of the five is 0
    let l = lines(&["eds", "null", "0xFFFFFFFF", "0xdc", "16", "buf"]);
    assert_eq!(l[0], "ret 1 le 1234");
    assert_eq!(l[1], eds_expected(16));
    // iModeNum is a DWORD (the upper half of rdx does not count)
    let l = lines(&["eds", "null", "0xABCDEF01FFFFFFFF", "0xdc", "0", "buf"]);
    assert_eq!(l[1], eds_expected(0));
}

#[test]
fn enum_display_settings_numbers_follow_the_virtual_display_of_decision_0053() {
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_HORZRES"), 1920);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_VERTRES"), 1080);
    assert_eq!(def_const("fle_gdc.h", "FLE_GDC_BITSPIXEL"), 32);
    assert_eq!(def_const("fle_monitor.h", "FLE_DISPLAYSETTINGS_FREQUENCY"), 60);
    assert_eq!(def_const("fle_monitor.h", "FLE_ENUM_CURRENT_SETTINGS"), 0xFFFF_FFFF);
    assert_eq!(def_const("fle_monitor.h", "FLE_DEVMODEW_SIZE"), 220);
    for (n, v) in [("SIZE", 68), ("DRIVEREXTRA", 70), ("FIELDS", 72), ("BITSPERPEL", 168), ("PELSWIDTH", 172), ("PELSHEIGHT", 176), ("DISPLAYFLAGS", 180), ("FREQUENCY", 184)] {
        assert_eq!(def_const("fle_monitor.h", &format!("FLE_DEVMODEW_OFF_{n}")), v, "{n}");
    }
}

#[test]
fn enum_display_settings_stops_for_the_forms_the_page_does_not_define_and_writes_nothing() {
    let sz = eds_untouched(0, 0xDC);
    stops(&["eds", "x", "0xFFFFFFFF", "0xdc", "0", "buf"], "EnumDisplaySettingsW: lpszDeviceName is not NULL", 0, &sz);
    stops(&["eds", "null", "0", "0xdc", "0", "buf"], "iModeNum 0x0 is not ENUM_CURRENT_SETTINGS", 0, &sz);
    stops(&["eds", "null", "1", "0xdc", "0", "buf"], "iModeNum 0x1 is not ENUM_CURRENT_SETTINGS", 0, &sz);
    stops(&["eds", "null", "0xFFFFFFFE", "0xdc", "0", "buf"], "iModeNum 0xfffffffe is not ENUM_CURRENT_SETTINGS", 0, &sz);
    stops(&["eds", "null", "0xFFFFFFFF", "0xdc", "0", "null"], "lpDevMode is NULL", 0, &sz);
    stops(&["eds", "null", "0xFFFFFFFF", "0", "0", "buf"], "dmSize 0x0 is not sizeof(DEVMODEW) 0xdc", 0, &eds_untouched(0, 0));
    stops(&["eds", "null", "0xFFFFFFFF", "0xd8", "0", "buf"], "dmSize 0xd8 is not sizeof(DEVMODEW) 0xdc", 0, &eds_untouched(0, 0xd8));
    stops(&["eds", "null", "0xFFFFFFFF", "0x9c", "0", "buf"], "dmSize 0x9c is not sizeof(DEVMODEW) 0xdc", 0, &eds_untouched(0, 0x9c));
}

// ---- 5. GetClipCursor (unit 272) ----

/// Learn (GetClipCursor): "The structure receives the dimensions of the screen if the cursor is not confined to a rectangle"; nonzero on success.
/// Nothing confines the cursor here (ClipCursor is not answered and not imported), so the RECT is the screen of decisions 0053 and 0057.
#[test]
fn get_clip_cursor_is_the_screen_when_nothing_confines_the_cursor() {
    assert_eq!(lines(&["clip", "buf"]), ["ret 1 rect 0 0 1920 1080 guards 11111111 22222222 le 1234"]);
    // two calls give the same answer (no state)
    assert_eq!(lines(&["clip", "buf"]), lines(&["clip", "buf"]));
}

#[test]
fn get_clip_cursor_null_rect_stops_by_name() {
    let untouched = le_hex(&[0x1111_1111, 0xAAAA_AAAA, 0xAAAA_AAAA, 0xAAAA_AAAA, 0xAAAA_AAAA, 0x2222_2222]);
    stops(&["clip", "null"], "GetClipCursor: lpRect is NULL", 0, &untouched);
}

// ---- 6. GetFileVersionInfoSizeW (unit 880) ----

/// Learn (GetFileVersionInfoSizeW): lpdwHandle "A pointer to a variable that the function sets to zero"; "If the function fails, the return value is zero.
/// To get extended error information, call GetLastError." The virtual file system holds only what the guest creates (decision 0018): the guest's own
/// DLL path (GetModuleFileNameW) is not a file there, so there is no version information: 0, and the file system's error for the path.
const OWN_DLL: &str = "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll";

#[test]
fn version_info_size_of_the_guests_own_dll_path_needs_the_resource_reader_and_a_path_next_to_it_is_not_found() {
    // decision 0129 (old test name: ..._is_zero_with_path_not_found; old answers: "ret 0 handle 0 le 3" and "ret 0 handle 55555555 le 3" for OWN_DLL): the guest's own path is the lifted DLL's RT_VERSION resource now
    // (PM's decision A, S-version-info-real-resource), read by fle_res.c from the image mapped at 0x400000. This driver links no fle_res.c (the weak reference of fle_vfs.c is NULL) and maps no image, so the call
    // stops by name instead of answering 0; the answer with a mapped image (size, *lpdwHandle = 0) is tests/wall_fr12.rs.
    let o = stops(&["fvs", OWN_DLL, "buf"], "GetFileVersionInfoSizeW: the path is the engine's own and no resource reader is linked", 0, "00000000");
    assert!(o.is_empty());
    // the same path under another directory, and the path with another file name, are not the engine: as before (C:\Program Files does not exist in the virtual file system: ERROR_PATH_NOT_FOUND 3, as
    // GetFileAttributesW gives; the handle variable is set to 0; lpdwHandle is optional)
    assert_eq!(lines(&["fvs", "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x32.dll", "buf"]), ["ret 0 handle 0 le 3"]);
    assert_eq!(lines(&["fvs", "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x32.dll", "null"]), ["ret 0 handle 55555555 le 3"]);
}

#[test]
fn version_info_size_of_a_missing_file_in_an_existing_directory_is_file_not_found() {
    assert_eq!(lines(&["fvs", "C:\\nofile.dll", "buf"]), ["ret 0 handle 0 le 2"]);
    assert_eq!(lines(&["fvs", "c:/nofile.dll", "buf"]), ["ret 0 handle 0 le 2"]);
    assert_eq!(lines(&["fvs", "C:\\d\\nofile.dll", "buf", "mkdir=C:\\d"]), ["ret 0 handle 0 le 2"]);
    assert_eq!(lines(&["fvs", "C:\\x\\y.dll", "buf", "mkdir=C:\\d"]), ["ret 0 handle 0 le 3"]);
}

#[test]
fn version_info_size_stops_for_a_path_that_exists_in_the_virtual_file_system() {
    // no version resource of a virtual file is modelled: the file the guest created, and a directory
    let a = stops(&["fvs", "C:\\d\\f.dll", "buf", "mkdir=C:\\d", "mkfile=C:\\d\\f.dll"], "GetFileVersionInfoSizeW: the path exists in the virtual file system", 0, "00000000");
    assert!(a.is_empty());
    stops(&["fvs", "C:\\d", "buf", "mkdir=C:\\d"], "the path exists in the virtual file system", 0, "00000000");
    stops(&["fvs", "C:\\", "buf"], "the path exists in the virtual file system", 0, "00000000");
}

#[test]
fn version_info_size_stops_for_the_forms_the_page_leaves_open() {
    // the handle variable is written (0) before the stop in every case where the pointer is given
    stops(&["fvs", "null", "buf"], "GetFileVersionInfoSizeW: lptstrFilename is NULL", 0, "55555555");
    stops(&["fvs", "kernel32.dll", "buf"], "a file name with no directory part", 0, "00000000");
    stops(&["fvs", "", "buf"], "a file name with no directory part", 0, "00000000");
    // a name with a directory part is resolved by the file system rules of decision 0018 (here: C:\ + y.dll, missing)
    assert_eq!(lines(&["fvs", "x\\..\\y.dll", "null"]), ["ret 0 handle 55555555 le 2"]);
}

// ---- 7. SHGetMalloc and the IMalloc object (unit 235) ----

#[test]
fn sh_get_malloc_returns_s_ok_and_an_object_whose_vtable_holds_the_response_rows() {
    let l = lines(&["sh-basic"]);
    assert_eq!(l[0], "ret 0 set 1 refs 1 le 1234");
    // IMalloc order (objidl.h; the Learn pages list the methods alphabetically): QueryInterface, AddRef, Release, Alloc, Realloc, Free, GetSize,
    // DidAlloc, HeapMinimize. The guest's call offsets 0x10, 0x28, 0x38 are slots 2, 5 and 7.
    let want = ["unimplemented", "unimplemented", "Release", "unimplemented", "unimplemented", "Free", "unimplemented", "DidAlloc", "unimplemented"];
    for (i, w) in want.iter().enumerate() {
        assert_eq!(l[1 + i], format!("slot {i} {w}"));
    }
}

#[test]
fn imalloc_free_and_didalloc_follow_the_pages_over_the_task_allocator() {
    let l = lines(&["sh-ops"]);
    assert_eq!(
        l,
        [
            "live 1",
            "didalloc-live 1",
            "didalloc-null ffffffff",
            "didalloc-stack 0",
            "free-null live 1",
            "free-live live 0",
            "didalloc-freed 0",
            "release 0 refs 0",
            "le 1234",
        ]
    );
}

#[test]
fn every_sh_get_malloc_counts_one_reference_and_release_answers_the_new_count() {
    let o = run(&["sh-twice"]);
    assert_eq!(o.code, Some(134), "{}", o.stderr);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(&l[..3], ["same 1 refs 2", "release 1", "release 0"]);
    assert!(l[3].starts_with("ABORT calls 0"), "{l:?}");
    assert!(o.stderr.contains("fle: IMalloc::Release: no reference outstanding"), "{}", o.stderr);
}

#[test]
fn the_imalloc_object_stops_by_name_for_what_the_guest_does_not_use() {
    stops(&["sh-stop", "null-pp"], "SHGetMalloc: ppMalloc is NULL", 0, "");
    stops(&["sh-stop", "free-stray"], "IMalloc::Free: pointer is not a live task-allocator block", 0, "");
    stops(&["sh-stop", "release-wrong-this"], "IMalloc::Release: the object is not the one SHGetMalloc returned", 0, "");
    stops(&["sh-stop", "release-before-get"], "IMalloc::Release: the object is not the one SHGetMalloc returned", 0, "");
    for slot in [0, 1, 3, 4, 6, 8] {
        stops(&["sh-stop", &format!("slot{slot}")], "IMalloc: a method other than Release, Free and DidAlloc was called", 0, "");
    }
}

// ---- 8. EnumFontFamiliesExW (unit 283) ----

/// ENUMLOGFONTEXW (Learn): LOGFONTW (92 bytes: 5 LONG, 8 BYTE, WCHAR[32]) + elfFullName[64] + elfStyle[32] + elfScript[32] = 92 + 128 + 64 + 64 = 348.
/// NEWTEXTMETRICEXW = NEWTEXTMETRICW (11 LONG, 4 WCHAR, 5 BYTE, 3 pad, DWORD, 3 UINT = 76) + FONTSIGNATURE (6 DWORD = 24) = 100.
/// The one font of this environment is the system font of decision 0066: "Segoe UI", weight 400 (FW_NORMAL).
fn expected_elf() -> String {
    let mut b = vec![0u8; 348];
    b[16..20].copy_from_slice(&400u32.to_le_bytes());
    for (i, c) in "Segoe UI".chars().enumerate() {
        b[28 + 2 * i] = c as u8;
        b[92 + 2 * i] = c as u8;
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}


fn efe_ok(dc: &str, charset: &str, face: &str, retv: &str) -> Vec<String> {
    lines(&["efe", dc, charset, face, "0", "1", "0", retv])
}

#[test]
fn enum_font_families_for_all_families_calls_the_callback_once_with_the_system_font() {
    let l = efe_ok("live", "1", "-", "1");
    // the callback returned 1, one call, rsp kept, the API's return address and the stack above it untouched
    assert_eq!(l[0], "ret 1 calls 1 rsp-kept 1 api-ret dead0000");
    assert_eq!(l[1], "above-changed 0");
    // FontType = TRUETYPE_FONTTYPE (4, the SDK header's value), lParam passed through unchanged (all 64 bits)
    assert_eq!(l[2], "args-r8-r9 4 1122334455667788");
    assert_eq!(l[3], "elf-aligned 1 ntm-after-elf 1 struct-below-api-frame 1 cb-sp-below-structs 1");
    assert_eq!(l[4], format!("elf {}", expected_elf()));
    assert_eq!(l[5], format!("ntm {}", "0".repeat(200)));
    assert_eq!(l.len(), 7);
    assert_eq!(l[6], "le 1234");
}

#[test]
fn enum_font_families_for_the_family_by_name_ignores_ascii_case() {
    for face in ["Segoe UI", "segoe ui", "SEGOE UI"] {
        let l = efe_ok("live", "1", face, "1");
        assert_eq!(l[0], "ret 1 calls 1 rsp-kept 1 api-ret dead0000", "{face}");
        assert_eq!(l[4], format!("elf {}", expected_elf()), "{face}");
    }
}

#[test]
fn enum_font_families_returns_the_last_value_the_callback_returned_as_an_int() {
    assert_eq!(efe_ok("live", "1", "-", "0")[0], "ret 0 calls 1 rsp-kept 1 api-ret dead0000");
    assert_eq!(efe_ok("live", "1", "-", "7")[0], "ret 7 calls 1 rsp-kept 1 api-ret dead0000");
    // EnumFontFamiliesExW returns int: only the low 32 bits of rax count
    assert_eq!(efe_ok("live", "1", "-", "0xFFFFFFFF00000001")[0], "ret 1 calls 1 rsp-kept 1 api-ret dead0000");
    assert_eq!(efe_ok("live", "1", "-", "0xFFFFFFFF")[0], "ret ffffffff calls 1 rsp-kept 1 api-ret dead0000");
}

#[test]
fn enum_font_families_stops_for_the_forms_the_page_leaves_open_and_calls_nothing() {
    let lf = |charset: u8, face: &str, pitch: u8| -> String {
        let mut b = vec![0u8; 92];
        b[23] = charset;
        b[27] = pitch;
        for (i, c) in face.chars().enumerate() {
            b[28 + 2 * i] = c as u8;
        }
        b.iter().map(|x| format!("{x:02x}")).collect()
    };
    let go = |dc: &str, charset: &str, face: &str, pitch: &str, proc: &str, flags: &str, needle: &str, buf: String| {
        stops(&["efe", dc, charset, face, pitch, proc, flags, "1"], needle, 0, &buf);
    };
    go("null", "1", "-", "0", "1", "0", "hdc is not a live virtual screen DC", lf(1, "", 0));
    go("released", "1", "-", "0", "1", "0", "hdc is not a live virtual screen DC", lf(1, "", 0));
    go("junk", "1", "-", "0", "1", "0", "hdc is not a live virtual screen DC", lf(1, "", 0));
    go("live", "1", "-", "0", "0", "0", "lpProc is NULL", lf(1, "", 0));
    go("live", "1", "-", "0", "1", "1", "dwFlags is not zero", lf(1, "", 0));
    go("live", "1", "-", "2", "1", "0", "lfPitchAndFamily is not zero", lf(1, "", 2));
    go("live", "0", "-", "0", "1", "0", "lfCharSet is not DEFAULT_CHARSET", lf(0, "", 0));
    go("live", "238", "-", "0", "1", "0", "lfCharSet is not DEFAULT_CHARSET", lf(238, "", 0));
    go("live", "1", "Arial", "0", "1", "0", "lfFaceName is neither empty nor the face of the system font", lf(1, "Arial", 0));
    go("live", "1", "Segoe U", "0", "1", "0", "lfFaceName is neither empty nor the face of the system font", lf(1, "Segoe U", 0));
    go("live", "1", "Segoe UIX", "0", "1", "0", "lfFaceName is neither empty nor the face of the system font", lf(1, "Segoe UIX", 0));
}

// ---- 9. The response table (generator) ----

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

/// A PE that imports each `(dll, name)` through its own import descriptor; entry 0x1000 calls the thunks at 0x1040 + 0x10 * k.
fn build(imports: &[(&str, &str)]) -> Vec<u8> {
    assert!(imports.len() <= 7);
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
    w32(&mut b, o + 124, 20 * (imports.len() as u32 + 1));

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
    for (k, (dll, name)) in imports.iter().enumerate() {
        let k32 = k as u32;
        let (int, iat, dllname, hn) = (0x2100 + 0x10 * k32, 0x2300 + 0x10 * k32, 0x2500 + 0x20 * k32, 0x2600 + 0x40 * k32);
        w32(&mut b, d + 20 * k, int);
        w32(&mut b, d + 20 * k + 12, dllname);
        w32(&mut b, d + 20 * k + 16, iat);
        let mut dn = dll.as_bytes().to_vec();
        dn.push(0);
        put(&mut b, dllname, &dn);
        let mut h = vec![0u8, 0u8];
        h.extend_from_slice(name.as_bytes());
        h.push(0);
        put(&mut b, hn, &h);
        w64(&mut b, data_at(int), hn as u64);
        w64(&mut b, data_at(iat), hn as u64);
    }

    let t = TEXT_RAW;
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    for k in 0..imports.len() {
        let k32 = k as u32;
        let at = 0x1000 + 5 * k32;
        code.push((at, [&[0xe8][..], &rel(at + 5, 0x1040 + 0x10 * k32)].concat()));
        code.push((0x1040 + 0x10 * k32, [&[0xff, 0x25][..], &rel(0x1046 + 0x10 * k32, 0x2300 + 0x10 * k32)].concat()));
    }
    code.push((0x1000 + 5 * imports.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

/// The six imports of the batch (the same DLL and names as the guest's import table) and GetProcAddress, so that the response table is emitted.
const BATCH: [(&str, &str); 6] = [
    ("msacm32.dll", "acmMetrics"),
    ("shell32.dll", "SHGetMalloc"),
    ("user32.dll", "EnumDisplaySettingsW"),
    ("user32.dll", "GetClipCursor"),
    ("gdi32.dll", "EnumFontFamiliesExW"),
    ("version.dll", "GetFileVersionInfoSizeW"),
];

fn gen(imports: &[(&str, &str)]) -> (fe_lift::Stats, String) {
    let img = build(imports);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

fn resp_rows(c: &str) -> Vec<(String, String, u64)> {
    let start = c.find("const fle_resp_entry fle_resp_table[] = {").unwrap();
    let end = c[start..].find("};").unwrap() + start;
    c[start..end]
        .lines()
        .filter(|l| l.trim_start().starts_with("{ \""))
        .map(|l| {
            let dll = l.split('"').nth(1).unwrap().to_string();
            let name = l.split('"').nth(3).unwrap().to_string();
            let va = u64::from_str_radix(l.split("UINT64_C(0x").nth(1).unwrap().split(')').next().unwrap(), 16).unwrap();
            (dll, name, va)
        })
        .collect()
}

#[test]
fn the_six_thunks_call_the_runtime_and_the_rows_come_after_the_existing_ones() {
    let mut imports = BATCH.to_vec();
    imports.push(("kernel32.dll", "GetProcAddress"));
    let (st, c) = gen(&imports);
    for (_, name) in BATCH {
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{name}");
        assert!(c.contains(&format!("fle_imp_{name}(); return;")), "{name}");
    }
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    for (i, (_, name, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7ff9_0000_0000u64 + 0x10 * (i as u64 + 1), "row {i} {name}: VA = base + 0x10 * (position), so appending never moves one");
    }
    let at = |n: &str| rows.iter().position(|(_, name, _)| name == n).unwrap_or_else(|| panic!("no row {n}"));
    // the rows that existed before this batch keep their place (decision 0015): the last one before it is SleepConditionVariableSRW (decision 0092)
    let last_old = at("SleepConditionVariableSRW");
    assert!(at("GetModuleHandleA") < last_old);
    let new_rows = [
        ("msacm32.dll", "acmMetrics"),
        ("shell32.dll", "SHGetMalloc"),
        ("shell32.dll", "IMalloc::Release"),
        ("shell32.dll", "IMalloc::Free"),
        ("shell32.dll", "IMalloc::DidAlloc"),
        ("shell32.dll", "IMalloc::unimplemented"),
        ("user32.dll", "EnumDisplaySettingsW"),
        ("user32.dll", "GetClipCursor"),
        ("gdi32.dll", "EnumFontFamiliesExW"),
        ("version.dll", "GetFileVersionInfoSizeW"),
    ];
    for (dll, name) in new_rows {
        assert!(at(name) > last_old, "{name} is appended after the existing rows");
        assert_eq!(rows.iter().filter(|(_, n, _)| n == name).count(), 1, "{name} once");
        assert_eq!(rows[at(name)].0, dll, "{name}");
        let f = format!("fle_imp_{}", name.replace("::", "_"));
        let va = rows[at(name)].2;
        assert!(c.contains(&format!("{{ \"{dll}\", \"{name}\", UINT64_C({va:#x}), {f} }}")), "{name}");
        assert!(c.contains(&format!("{{ UINT64_C({va:#x}), {f} }}")), "{name} is in the dispatch table");
    }
    // the names after the batch are in this order, so the doc of the decision can name the pseudo-VAs by position
    let pos: Vec<usize> = new_rows.iter().map(|(_, n)| at(n)).collect();
    assert!(pos.windows(2).all(|w| w[0] < w[1]), "{pos:?}");
}

#[test]
fn the_vtable_rows_reach_the_runtime_through_the_generated_tables() {
    // links the generated C of the synthetic PE with every runtime file (the response table names fle_imp_* of any of them): the object that
    // SHGetMalloc returns holds pseudo-VAs that fle_dispatch routes to the IMalloc methods, with the table the generator wrote
    let mut imports = BATCH.to_vec();
    imports.push(("kernel32.dll", "GetProcAddress"));
    let (_, c) = gen(&imports);
    let rt = runtime_dir();
    let dir = work_dir();
    let tag = etxt_common::uniq("gen-wallr");
    let src = dir.join(format!("{tag}.c"));
    std::fs::write(&src, &c).unwrap();
    let main_c = dir.join(format!("{tag}-main.c"));
    let main = r#"
#include <stdio.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_ole.h"
static uint64_t stk[64];
int main(void)
{
    uint64_t p = 0, *o, *vt, blk;
    setvbuf(stdout, NULL, _IONBF, 0);
    stk[0] = 0xdead0000u;
    R[4] = (uint64_t)(uintptr_t)(stk + 8);
    R[1] = (uint64_t)(uintptr_t)&p;
    fle_imp_SHGetMalloc();
    o = (uint64_t *)(uintptr_t)p; vt = (uint64_t *)(uintptr_t)o[0];
    R[1] = 24; fle_imp_CoTaskMemAlloc(); blk = R[0];
    R[1] = p; R[2] = blk; fle_dispatch(vt[7]); printf("didalloc %llu\n", (unsigned long long)R[0]);
    R[1] = p; R[2] = blk; fle_dispatch(vt[5]); printf("live %llu\n", (unsigned long long)fle_ole_task_live());
    R[1] = p; fle_dispatch(vt[2]); printf("release %llu\n", (unsigned long long)R[0]);
    R[1] = p; R[2] = 0; fle_dispatch(vt[3]);
    printf("not reached\n");
    return 0;
}
"#;
    std::fs::write(&main_c, main).unwrap();
    let exe = dir.join(&tag);
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"]).arg(&rt);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&rt)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with("fle_") && n.ends_with(".c")
        })
        .collect();
    files.sort();
    for f in &files {
        cmd.arg(f);
    }
    // decision 0139: fle_dsound.c mixes with powf (volume and pan in hundredths of a dB), so every program that links all the runtime files needs -lm
    let cc = cmd.arg("-o").arg(&exe).arg(&src).arg(&main_c).arg("-lm").env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().unwrap();
    let out = String::from_utf8_lossy(&r.stdout).to_string();
    let err = String::from_utf8_lossy(&r.stderr).to_string();
    assert_eq!(r.status.signal(), Some(6), "{err}");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["didalloc 1", "live 0", "release 0"]);
    assert!(err.contains("fle: IMalloc: a method other than Release, Free and DidAlloc was called"), "{err}");
}

#[test]
fn the_rows_are_declared_in_the_headers_and_the_table_names_each_function_once() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    for (dll, api, f, h) in [
        ("msacm32.dll", "acmMetrics", "fle_imp_acmMetrics", "fle_winmm.h"),
        ("shell32.dll", "SHGetMalloc", "fle_imp_SHGetMalloc", "fle_ole.h"),
        ("shell32.dll", "IMalloc::Release", "fle_imp_IMalloc_Release", "fle_ole.h"),
        ("shell32.dll", "IMalloc::Free", "fle_imp_IMalloc_Free", "fle_ole.h"),
        ("shell32.dll", "IMalloc::DidAlloc", "fle_imp_IMalloc_DidAlloc", "fle_ole.h"),
        ("shell32.dll", "IMalloc::unimplemented", "fle_imp_IMalloc_unimplemented", "fle_ole.h"),
        ("user32.dll", "EnumDisplaySettingsW", "fle_imp_EnumDisplaySettingsW", "fle_monitor.h"),
        ("user32.dll", "GetClipCursor", "fle_imp_GetClipCursor", "fle_cursor.h"),
        ("gdi32.dll", "EnumFontFamiliesExW", "fle_imp_EnumFontFamiliesExW", "fle_gdi.h"),
        ("version.dll", "GetFileVersionInfoSizeW", "fle_imp_GetFileVersionInfoSizeW", "fle_vfs.h"),
    ] {
        let row = format!("(\"{dll}\", \"{api}\", \"{f}\", \"jmp_imp:{api}\")");
        assert_eq!(table.matches(&row).count(), 1, "exactly one row for {api}");
        let hh = std::fs::read_to_string(runtime_dir().join(h)).unwrap();
        assert!(hh.contains(&format!("void {f}(void)")), "{f} in {h}");
    }
    // the rows of this batch are the last ones that existed when it was written: nothing of an earlier batch moved (a row is only appended)
    let first = table.find("SleepConditionVariableSRW\", \"fle_imp_SleepConditionVariableSRW").unwrap();
    for api in ["acmMetrics", "SHGetMalloc", "EnumDisplaySettingsW", "GetClipCursor", "EnumFontFamiliesExW", "GetFileVersionInfoSizeW"] {
        assert!(table.find(&format!("\"{api}\"")).unwrap() > first, "{api}");
    }
}

#[test]
fn the_new_runtime_code_reads_nothing_of_the_host() {
    // every function of this batch is a pure function of its arguments and the virtual tables; none may reach for the host's display, files,
    // registry, locale, fonts, audio, clock, threads or processes (comments are stripped: they say what is NOT read)
    let strip = |s: &str| -> String {
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
            } else {
                o.push(b[i]);
                i += 1;
            }
        }
        o
    };
    for (file, from) in [
        ("fle_winmm.c", "void fle_imp_acmMetrics"),
        ("fle_monitor.c", "void fle_imp_EnumDisplaySettingsW"),
        ("fle_cursor.c", "void fle_imp_GetClipCursor"),
        ("fle_gdi.c", "void fle_imp_EnumFontFamiliesExW"),
        ("fle_vfs.c", "void fle_imp_GetFileVersionInfoSizeW"),
        ("fle_ole.c", "uint32_t fle_ole_shell_malloc_refs"),
    ] {
        let t = std::fs::read_to_string(runtime_dir().join(file)).unwrap();
        let code = strip(&t[t.find(from).unwrap_or_else(|| panic!("{from} in {file}"))..]);
        for bad in ["fopen", "open(", "getenv", "time(", "clock", "gettimeofday", "getpid", "uname", "dlopen", "XOpen", "setlocale", "popen", "system(", "socket"] {
            assert!(!code.contains(bad), "{file}: {bad}");
        }
    }
    let t = std::fs::read_to_string(runtime_dir().join("fle_spi.c")).unwrap();
    let code = strip(&t[t.find("if (action == FLE_SPI_GETWORKAREA)").unwrap()..t.find("if (action == FLE_SPI_GETICONTITLELOGFONT)").unwrap()]);
    assert!(!code.contains("getenv") && !code.contains("fopen"));
}
