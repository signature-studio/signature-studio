//! Runtime side of decision 0103 (provisional), the part that is not a duplicate of decision 0102 (tests/wall_b.rs) after the merge: what the
//! answers added while quickfontcache_x64.dll (a real module), its freetype_x64.dll and the guest's unit 158 are run, and that no other lane
//! gave. Expected values are written here from the Microsoft Learn pages named in lift/runtime/*.c (and, for the CSIDL and FE_ numbers, from
//! mingw-w64's headers), not read back from the C code; the host is Linux and nothing is compared with a real Windows. Values that are
//! inventions of the decision (SPI orientation) are tested as what they are: the decision's choice, with the property that must hold.
//!
//! 1. Handles that have no path, and resources (LoadStringW, FindResourceW, LoadResource, SizeofResource, LockResource) of a real module's
//!    image (the implementation is decision 0104's in fle_res.c, which decision 0103 had also written; the tests are 0103's).
//! 2. SystemParametersInfoW: SPI_GETFONTSMOOTHINGCONTRAST (also tests/wall_b.rs), ...TYPE and ...ORIENTATION (only here).
//! 3. SHGetFolderPathW(CSIDL_FONTS).
//!
//! Not here, because tests/wall_b.rs has the same API with the implementation that stayed in the merge: GetVersionExA, GetStartupInfoA,
//! SetHandleCount, HeapCreate/HeapSetInformation/HeapSize, EncodePointer/DecodePointer, GetModuleFileNameA (decision 0102), and the path of a
//! real module (tests/realmod.rs, decision 0104).

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    manifest().join("../../runtime")
}
fn harness() -> PathBuf {
    manifest().join("../../harness")
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-wallQ-test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

/// Every runtime/fle_*.c, compiled once per process.
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work().join(etxt_common::per_process("rt"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut srcs: Vec<PathBuf> = std::fs::read_dir(rt())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("fle_") && n.ends_with(".c")
            })
            .collect();
        srcs.sort();
        let mut v = Vec::new();
        for s in srcs {
            let o = dir.join(s.file_name().unwrap().to_string_lossy().replace(".c", ".o"));
            let cc = Command::new("clang")
                .args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-c", "-I"])
                .arg(rt())
                .arg(&s)
                .arg("-o")
                .arg(&o)
                .env("TMPDIR", work().join("tmp"))
                .output()
                .expect("clang not found");
            assert!(cc.status.success(), "clang failed for {}: {}", s.display(), String::from_utf8_lossy(&cc.stderr));
            v.push(o);
        }
        v
    })
}

const PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_spi.h"
#include "fle_proc.h"
#include "fle_vfs.h"
#include "fle_res.h"
#include "fle_osver.h"
#include "fle_modules.h"
#include "fle_realmod.h"

uint64_t R[16];
fle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };
fle_xmm X[16];
const fle_fn_entry fle_dispatch_table[] = { { 0, 0 } };
const uint64_t fle_dispatch_count = 0;
/* two real modules, as the harness would link them (fle_realmod.h): the weak empty table of fle_realmod.c is overridden */
#ifdef WQ_COPY_ONLY
/* the resource driver maps only FLEngine and the copy: every table entry must be a mapped image (the harness maps them all) */
const fle_realmod_entry fle_realmod_table[] = { { "copy.dll", UINT64_C(0x2000000) } };
const uint64_t fle_realmod_count = 1;
#else
const fle_realmod_entry fle_realmod_table[] = { { "quickfontcache_x64.dll", UINT64_C(0x1d00000) }, { "copy.dll", UINT64_C(0x2000000) } };
const uint64_t fle_realmod_count = 2;
#endif
static uint64_t stk[64];
#define U(x) ((unsigned long long)(x))
#define A(p) ((uint64_t)(uintptr_t)(p))
/* R[4] points at the return address; the arguments after the fourth are at R[4] + 0x28, ... */
#define ENTER() do { memset(stk, 0, sizeof stk); R[4] = (uint64_t)(uintptr_t)&stk[16]; stk[16] = 0xdead0000ull; } while (0)
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
"#;

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn compile(name: &str, extra_src: &[PathBuf], body: &str, with_image: bool) -> PathBuf {
    let dir = work();
    let tag = etxt_common::uniq(name);
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    let main = if with_image {
        format!(
            "int main(int argc, char **argv)\n{{\n    harness_image a, b;\n    char err[256];\n    (void)argc;\n    \
             if (image_load(argv[1], &a, err, sizeof err) != 0) {{ fprintf(stderr, \"load a: %s\\n\", err); return 3; }}\n    \
             if (image_load(argv[2], &b, err, sizeof err) != 0) {{ fprintf(stderr, \"load b: %s\\n\", err); return 3; }}\n    ENTER();\n{body}\n    return 0;\n}}\n"
        )
    } else {
        format!("int main(void)\n{{\n    ENTER();\n{body}\n    return 0;\n}}\n")
    };
    let inc = if with_image { "#include \"boot.h\"\n" } else { "" };
    std::fs::write(&c, format!("{PRE}\n{inc}{main}")).unwrap();
    let mut cmd = Command::new("clang");
    // a driver that maps FLEngine at 0x400000 must itself be position independent (a -no-pie executable is linked at 0x400000)
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE"]).args(if with_image { ["-DWQ_COPY_ONLY"] } else { ["-no-pie"] }).arg("-I").arg(rt()).arg("-I").arg(harness());
    for o in objs() {
        cmd.arg(o);
    }
    for s in extra_src {
        cmd.arg(s);
    }
    let cc = cmd.arg(&c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn finish(exe: &PathBuf, args: &[&PathBuf]) -> Out {
    let r = Command::new(exe).args(args).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn run(name: &str, body: &str) -> Out {
    finish(&compile(name, &[], body, false), &[])
}

fn lines(o: &Out) -> Vec<String> {
    assert!(o.ok, "signal {:?}: {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn stops(name: &str, api: &str, must_say: &str, body: &str) {
    let o = run(name, &format!("{body}\n    printf(\"returned %llu\\n\", U(R[0]));"));
    assert!(!o.ok && !o.stdout.contains("returned"), "{name}: returned: {}", o.stdout);
    assert!(o.stderr.contains(&format!("fle: {api}")), "{name}: no 'fle: {api}': {}", o.stderr);
    assert!(o.stderr.contains(must_say), "{name}: message lacks {must_say:?}: {}", o.stderr);
    assert_eq!(o.signal, Some(6), "{name}: not SIGABRT");
}

// ---- 1. Handles without a path; resources of a real module's image ---------------

#[test]
fn get_module_file_name_of_other_handles_still_stops() {
    // a virtual module handle (kernel32 is N = 9) and a value that is nobody's handle (the base of a real module has a path since decision 0104)
    for (label, h) in [("virtual", "0x7FF800090000ull"), ("none", "0x1e00000")] {
        for api in ["GetModuleFileNameW", "GetModuleFileNameA"] {
            stops(
                &format!("gmfn-{label}-{api}"),
                api,
                "only the lifted DLL has a path",
                &format!("static char b[64]; R[1] = {h}; R[2] = A(b); R[8] = 64; fle_imp_{api}();"),
            );
        }
    }
}

fn real_dll() -> Option<PathBuf> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => manifest().join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    match std::fs::metadata(&path) {
        Ok(_) => Some(path),
        Err(e) if explicit.is_some() => panic!("FLENGINE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

/// FLEngine_x64.dll at 0x400000 and a copy of it rebased to 0x2000000 (`fe_pe::rebase`), which the driver's table calls "copy.dll":
/// the resources of the copy are the resources of the original, so the answers for the real module's handle must be the answers for the
/// lifted DLL's (the strings are the ones the independent parser of tests/res.rs gave).
#[test]
fn resources_of_a_real_modules_image_are_read_from_that_image() {
    let Some(dll) = real_dll() else { return };
    let data = std::fs::read(&dll).unwrap();
    let (copy, _) = fe_pe::rebase::rebase(&data, 0x200_0000).unwrap();
    let copy_path = work().join(etxt_common::uniq("copy-2000000.dll"));
    std::fs::write(&copy_path, &copy).unwrap();
    let exe = compile(
        "res-real",
        &[harness().join("image.c")],
        r#"
    static uint16_t buf[64]; static uint16_t buf2[64];
    uint64_t r, hr, hg, p, sz;
    const uint64_t HC = 0x2000000;
    int i, same = 1;
    (void)a; (void)b;
    /* LoadStringW: ID 64896 is "Set Size Exceeded." (18 characters) in the lifted DLL; the same in the copy */
    memset(buf, 0xAA, sizeof buf);
    ENTER(); R[1] = HC; R[2] = 64896; R[8] = A(buf); R[9] = 64; fle_imp_LoadStringW(); r = R[0];
    printf("copy %llu", U(r));
    for (i = 0; i < 19; i++) printf(" %04x", buf[i]);
    printf("\n");
    memset(buf2, 0xAA, sizeof buf2);
    ENTER(); R[1] = 0x400000; R[2] = 64896; R[8] = A(buf2); R[9] = 64; fle_imp_LoadStringW();
    for (i = 0; i < 64; i++) if (buf[i] != buf2[i]) same = 0;
    printf("same %d\n", same);
    /* FindResourceW / LoadResource / SizeofResource / LockResource on RT_VERSION name 1 (728 bytes) */
    ENTER(); R[1] = HC; R[2] = 1; R[8] = 16; fle_imp_FindResourceW(); hr = R[0];
    ENTER(); R[1] = HC; R[2] = hr; fle_imp_LoadResource(); hg = R[0];
    ENTER(); R[1] = hg; fle_imp_LockResource(); p = R[0];
    ENTER(); R[1] = HC; R[2] = hr; fle_imp_SizeofResource(); sz = R[0];
    printf("res found %d inside_copy %d same %d size %llu first %02x %02x\n", hr != 0, hg >= HC && hg < HC + 0x18c0000, hg == p, U(sz),
           *(const uint8_t *)(uintptr_t)p, *(const uint8_t *)(uintptr_t)(p + 1));
"#,
        true,
    );
    let o = finish(&exe, &[&dll, &copy_path]);
    let l = lines(&o);
    let want: Vec<String> = "Set Size Exceeded.".encode_utf16().map(|c| format!("{c:04x}")).collect();
    assert_eq!(l[0], format!("copy 18 {} 0000", want.join(" ")));
    assert_eq!(l[1], "same 1");
    assert_eq!(l[2], "res found 1 inside_copy 1 same 1 size 728 first d8 02");
}

#[test]
fn resources_of_a_module_that_is_not_the_lifted_dll_or_a_real_module_still_stop() {
    stops("res-none", "LoadStringW", "is not a module handle of this runtime", "static uint16_t b[8]; R[1] = 0x1e00000; R[2] = 1; R[8] = A(b); R[9] = 8; fle_imp_LoadStringW();");
    stops(
        "res-virtual",
        "LoadStringW",
        "has no resources in this runtime",
        "static uint16_t b[8]; R[1] = 0x7FF800090000ull; R[2] = 1; R[8] = A(b); R[9] = 8; fle_imp_LoadStringW();",
    );
}

// ---- 2. SystemParametersInfoW ------------------------------------------------------------------------

#[test]
fn spi_font_smoothing_contrast_type_and_orientation() {
    let o = run(
        "spi-fs",
        r#"
    static uint32_t u[4];
    memset(u, 0xAA, sizeof u); fle_set_last_error(55);
    R[1] = 0x200c; R[2] = 0; R[8] = A(u); R[9] = 0; fle_imp_SystemParametersInfoW(); printf("contrast %llu %u le %u next %08x\n", U(R[0]), u[0], LE(), u[1]);
    R[1] = 0x200a; R[2] = 0; R[8] = A(&u[2]); R[9] = 0; fle_imp_SystemParametersInfoW(); printf("type %llu %u\n", U(R[0]), u[2]);
    R[1] = 0x2012; R[2] = 0; R[8] = A(&u[3]); R[9] = 0; fle_imp_SystemParametersInfoW(); printf("orientation %llu %u\n", U(R[0]), u[3]);
"#,
    );
    let l = lines(&o);
    // Learn: contrast "Valid contrast values are from 1000 to 2200. The default value is 1400."
    assert_eq!(l[0], "contrast 1 1400 le 55 next aaaaaaaa");
    // FE_FONTSMOOTHINGSTANDARD (0x0001, the default the page names for the SET action); FE_FONTSMOOTHINGORIENTATIONRGB (0x0001): a
    // choice of decision 0103, no default on the page
    assert_eq!(l[1], "type 1 1");
    assert_eq!(l[2], "orientation 1 1");
    for (n, a) in [("contrast", "0x200c"), ("type", "0x200a"), ("orientation", "0x2012")] {
        stops(&format!("spi-{n}-null"), "SystemParametersInfoW", "pvParam NULL", &format!("R[1] = {a}; R[2] = 0; R[8] = 0; R[9] = 0; fle_imp_SystemParametersInfoW();"));
        stops(&format!("spi-{n}-ui"), "SystemParametersInfoW", "uiParam not zero", &format!("static uint32_t x; R[1] = {a}; R[2] = 1; R[8] = A(&x); R[9] = 0; fle_imp_SystemParametersInfoW();"));
        stops(&format!("spi-{n}-ini"), "SystemParametersInfoW", "fWinIni not zero", &format!("static uint32_t x; R[1] = {a}; R[2] = 0; R[8] = A(&x); R[9] = 1; fle_imp_SystemParametersInfoW();"));
    }
    // the neighbours are still not answered: the SET actions and the GET actions without a default
    stops("spi-set-type", "SystemParametersInfoW", "0x200b", "static uint32_t x; R[1] = 0x200b; R[2] = 0; R[8] = A(&x); R[9] = 0; fle_imp_SystemParametersInfoW();");
    stops("spi-getfontsmoothing", "SystemParametersInfoW", "0x4a", "static uint32_t x; R[1] = 0x4a; R[2] = 0; R[8] = A(&x); R[9] = 0; fle_imp_SystemParametersInfoW();");
}

// ---- 3. SHGetFolderPathW(CSIDL_FONTS) ------------------------------------------------------------------

#[test]
fn sh_get_folder_path_w_answers_the_virtual_font_folder() {
    let o = run(
        "shgfp",
        r#"
    static uint16_t p[300]; int i;
    memset(p, 0xAA, sizeof p);
    ENTER(); stk[21] = A(p);                 /* the fifth argument: R[4] + 0x28 */
    R[1] = 0; R[2] = 0x14; R[8] = 0; R[9] = 0; fle_imp_SHGetFolderPathW();
    printf("hr %llu ", U(R[0]));
    for (i = 0; p[i]; i++) putchar((int)p[i]);
    printf(" nul %u next %04x\n", p[i], p[i + 1]);
    ENTER(); stk[21] = A(p);                 /* SHGFP_TYPE_DEFAULT gives the same path */
    memset(p, 0xAA, sizeof p);
    R[1] = 0; R[2] = 0x14; R[8] = 0; R[9] = 1; fle_imp_SHGetFolderPathW();
    printf("default hr %llu len ", U(R[0])); for (i = 0; p[i]; i++) {} printf("%d\n", i);
"#,
    );
    let l = lines(&o);
    // Learn, CSIDL_FONTS: "A typical path is C:\Windows\Fonts"; the returned path has no trailing backslash. 0x14 is shlobj.h's value.
    assert_eq!(l[0], "hr 0 C:\\Windows\\Fonts nul 0 next aaaa");
    assert_eq!(l[1], "default hr 0 len 16");
    let pre = "ENTER(); stk[21] = A(p);";
    // decision 0126 (wall 59, tests/wall_fr11.rs): old: csidl 0x24 stopped ("csidl 0x24 is not answered"); CSIDL_WINDOWS is answered; 0x25 (CSIDL_SYSTEM) has no answer yet and stops with the new message
    stops("shgfp-csidl", "SHGetFolderPathW", "csidl 0x25 is not answered", &format!("static uint16_t p[300]; {pre} R[1] = 0; R[2] = 0x25; R[8] = 0; R[9] = 0; fle_imp_SHGetFolderPathW();"));
    stops("shgfp-flag-create", "SHGetFolderPathW", "csidl 0x8014", &format!("static uint16_t p[300]; {pre} R[1] = 0; R[2] = 0x8014; R[8] = 0; R[9] = 0; fle_imp_SHGetFolderPathW();"));
    stops("shgfp-token", "SHGetFolderPathW", "hToken", &format!("static uint16_t p[300]; {pre} R[1] = 0; R[2] = 0x14; R[8] = 5; R[9] = 0; fle_imp_SHGetFolderPathW();"));
    stops("shgfp-flags", "SHGetFolderPathW", "dwFlags", &format!("static uint16_t p[300]; {pre} R[1] = 0; R[2] = 0x14; R[8] = 0; R[9] = 2; fle_imp_SHGetFolderPathW();"));
    stops("shgfp-null", "SHGetFolderPathW", "pszPath is NULL", "ENTER(); stk[21] = 0; R[1] = 0; R[2] = 0x14; R[8] = 0; R[9] = 0; fle_imp_SHGetFolderPathW();");
}
