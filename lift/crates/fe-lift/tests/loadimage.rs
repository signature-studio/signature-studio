//! Tests for the virtual LoadImageW of runtime/fle_loadimage.c: user32!LoadImageW(hInst, name, IMAGE_CURSOR, 32, 32, LR_SHARED)
//! (docs/decisions/0061-virtual-load-image.md; sources: docs/investigations/loadimage-caller.md).
//!
//! Expected values are written here from the decision (Microsoft Learn: LoadImageW, About Cursors, Resource File Formats, NEWHEADER,
//! RESDIR; the choices of the decision) and from the DLL read with fe-pe / raw bytes, not read back from the C code. The handle
//! values (0x7FCA0000 + 4 * (idc - 32512 + 1) for the standard cursors, 0x7FDA0000 + 4 * (id - 32762 + 1) for the DLL's cursors) are
//! PROVISIONAL choices (no page gives a value). The host is Linux: nothing is compared with a real Windows; no cursor, theme, mouse
//! setting, display, clock, thread or process is read.
//!
//! The C sources of fle_loadimage.c/.h, fle_cursor.c/.h and fle_res.c/.h come from runtime/. FLE_LI_SRC_DIR (a directory holding
//! any of these files) overrides the ones it holds: the mutation runs of the run record use it.

mod etxt_common;

use fe_lift::generate;
use fe_pe::{Pe, ResourceId};
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
fn override_dir() -> Option<PathBuf> {
    std::env::var("FLE_LI_SRC_DIR").ok().map(PathBuf::from)
}
/// The path of a runtime file: the override directory when it holds the file, runtime/ otherwise.
fn src(name: &str) -> PathBuf {
    if let Some(d) = override_dir() {
        if d.join(name).exists() {
            return d.join(name);
        }
    }
    runtime_dir().join(name)
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-nc/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(file: &str, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-fno-builtin", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    if let Some(d) = override_dir() {
        cmd.arg("-I").arg(d);
    }
    let cc = cmd
        .arg("-I")
        .arg(runtime_dir())
        .arg("-I")
        .arg(manifest().join("../../harness"))
        .arg(src(file))
        .arg("-o")
        .arg(obj)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {file}: {}", String::from_utf8_lossy(&cc.stderr));
}

/// fle_loadimage.c, fle_cursor.c and fle_res.c with warnings as errors; the files they link with: the loader of the harness (maps
/// the DLL at 0x400000), fle_env.c (fle_stop), fle_imports.c, fle_modules.c, fle_string.c, and the other two handle tables
/// (fle_gdi.c, fle_gdc.c: the regions and the "not issued by this table" stops are tested against each other).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-loadimage"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, strict) in [
            ("fle_loadimage.c", true),
            ("fle_cursor.c", true),
            ("fle_res.c", true),
            ("fle_gdi.c", false),
            ("fle_raster.c", false),
            ("fle_gdc.c", false),
            ("fle_imports.c", false),
            ("fle_env.c", false),
            ("fle_modules.c", false),
            ("fle_string.c", false),
        ] {
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(name, &o, strict);
            v.push(o);
        }
        let o = dir.join("image.o");
        let cc = Command::new("clang")
            .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-c", "-I"])
            .arg(manifest().join("../../harness"))
            .arg(manifest().join("../../harness/image.c"))
            .arg("-o")
            .arg(&o)
            .env("TMPDIR", work_dir().join("tmp"))
            .output()
            .expect("clang not found");
        assert!(cc.status.success(), "clang failed for image.c: {}", String::from_utf8_lossy(&cc.stderr));
        v.push(o);
        v
    })
}

pub struct Out {
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
#include <sys/resource.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_gdi.h"
#include "fle_cursor.h"
#include "fle_res.h"
#include "fle_loadimage.h"
#include "boot.h"

uint64_t R[16];
static uint64_t stk[32];

__attribute__((constructor)) static void nocore(void)
{
    struct rlimit r = { 0, 0 };
    setrlimit(RLIMIT_CORE, &r);
}

#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
/* a[0..5]: rcx, rdx, r8, r9, [rsp+0x28], [rsp+0x30] (whole 64-bit values) */
static uint64_t LI(const uint64_t *a)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    stk[5] = a[4];
    stk[6] = a[5];
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    fle_imp_LoadImageW();
    return R[0];
}
#define LIC(...) LI((uint64_t[6]){ __VA_ARGS__ })
"#;

fn compile_main(name: &str, body: &str) -> PathBuf {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let text = format!(
        "{PRELUDE}\nint main(int argc, char **argv)\n{{\n    harness_image img;\n    char err[256];\n    (void)argc;\n    \
         if (image_load(argv[1], &img, err, sizeof err) != 0) {{ fprintf(stderr, \"load: %s\\n\", err); return 3; }}\n{body}\n    return 0;\n}}\n"
    );
    std::fs::write(&c, text).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-fno-builtin", "-I"]);
    if let Some(d) = override_dir() {
        cmd.arg(d).arg("-I");
    }
    cmd.arg(runtime_dir()).arg("-I").arg(manifest().join("../../harness"));
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn exec(exe: &Path, args: &[&str]) -> Out {
    let r = Command::new(exe).args(args).env("TMPDIR", work_dir().join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn lines_on(name: &str, image: &Path, body: &str) -> Vec<String> {
    let exe = compile_main(name, body);
    let o = exec(&exe, &[image.to_str().unwrap()]);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn real_dll_path() -> Option<PathBuf> {
    let p = std::env::var("FLENGINE_DLL").map(PathBuf::from).unwrap_or_else(|_| manifest().join("../../../binaries/win64/FLEngine_x64.dll"));
    p.exists().then_some(p)
}
macro_rules! real {
    ($p:ident) => {
        let Some($p) = real_dll_path() else { return };
    };
}

/// One program for every call on one image: argv = image inst name type cx cy fuLoad (u64 each). Output "ret <hex>".
fn cli() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        compile_main(
            "loadimage-cli",
            r#"
    uint64_t a[6], r;
    int i;
    for (i = 0; i < 6; i++) a[i] = strtoull(argv[2 + i], 0, 0);
    r = LI(a);
    printf("ret %llx\n", U(r));
"#,
        )
    })
}

const IC: u64 = 2;
const SHARED: u64 = 0x8000;
const MODULE: u64 = 0x400000;

/// What LoadImageW(inst, name, type, cx, cy, fu) did on `image`: Ok(return value), or Err(stderr) when the run stopped (SIGABRT,
/// "fle: LoadImageW", nothing printed before the stop).
fn call_on(image: &Path, a: [u64; 6]) -> Result<u64, String> {
    let s: Vec<String> = a.iter().map(|v| format!("{v:#x}")).collect();
    let mut args = vec![image.to_str().unwrap()];
    args.extend(s.iter().map(|x| x.as_str()));
    let o = exec(cli(), &args);
    if o.ok {
        let t: Vec<&str> = o.stdout.trim().split(' ').collect();
        assert_eq!(t[0], "ret", "odd output {}", o.stdout);
        return Ok(u64::from_str_radix(t[1], 16).unwrap());
    }
    assert_eq!(o.signal, Some(6), "{a:x?}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains("fle: LoadImageW"), "{a:x?}: no 'fle: LoadImageW' message: {}", o.stderr);
    assert!(o.stdout.is_empty(), "{a:x?}: output before the stop: {}", o.stdout);
    Err(o.stderr)
}

/// The guest's form with only inst and name varied.
fn load(dll: &Path, inst: u64, name: u64) -> Result<u64, String> {
    call_on(dll, [inst, name, IC, 32, 32, SHARED])
}

/// An answer must be Ok(handle); a stop must say `decision 0061` and the needle.
fn stops(dll: &Path, a: [u64; 6], needle: &str) {
    let e = call_on(dll, a).expect_err(&format!("{a:x?} must stop"));
    assert!(e.contains("decision 0061") && e.contains(needle), "{a:x?}: {e}");
}

/// The value of `#define NAME <number>` (decimal, or UINT64_C(0x..), or 0x..) in a runtime header.
fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src(file)).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
    if let Some(r) = rest.strip_prefix("UINT64_C(") {
        let tok = r.split(')').next().unwrap();
        return u64::from_str_radix(tok.trim_start_matches("0x"), 16).unwrap();
    }
    if let Some(r) = rest.strip_prefix("0x") {
        let tok: String = r.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        return u64::from_str_radix(&tok, 16).unwrap();
    }
    rest.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

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

/// The 13 standard cursors the guest loads with hInst NULL (Learn About Cursors, written here from the page's table; IDC_HAND is
/// on the page too but the guest loads it from its module only, and it is not answered).
const STD13: [(u64, &str); 13] = [
    (32512, "IDC_ARROW"), (32513, "IDC_IBEAM"), (32514, "IDC_WAIT"), (32515, "IDC_CROSS"), (32516, "IDC_UPARROW"), (32642, "IDC_SIZENWSE"),
    (32643, "IDC_SIZENESW"), (32644, "IDC_SIZEWE"), (32645, "IDC_SIZENS"), (32646, "IDC_SIZEALL"), (32648, "IDC_NO"), (32650, "IDC_APPSTARTING"),
    (32651, "IDC_HELP"),
];
/// The rest of the page's two tables (IDC_HAND, IDC_PIN, IDC_PERSON, 32631, 32652..=32663): not answered.
const NOT_STD: [u64; 17] = [32649, 32671, 32672, 32631, 32652, 32653, 32654, 32655, 32656, 32657, 32658, 32659, 32660, 32661, 32662, 32663, 32641];
/// The decided handles (decision 0061 / 0059, PROVISIONAL).
fn std_handle(idc: u64) -> u64 {
    0x7FCA_0000 + 4 * (idc - 32512 + 1)
}
fn mod_handle(id: u64) -> u64 {
    0x7FDA_0000 + 4 * (id - 32762 + 1)
}

// ---- 1. Table, header, constants ----

#[test]
fn the_row_is_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src("fle_loadimage.h")).unwrap();
    let row = "(\"user32.dll\", \"LoadImageW\", \"fle_imp_LoadImageW\", \"jmp_imp:LoadImageW\")";
    assert_eq!(table.matches(row).count(), 1, "exactly one row in imports_table.rs");
    assert!(h.contains("void fle_imp_LoadImageW(void)"));
    // the nearby APIs are not answered by this decision (the investigation counts their call sites only)
    // ("SetCursor", "GetCursor", "DestroyCursor", "ShowCursor" and "GetIconInfo" were in this list until decision 0111: runtime/fle_gdi.c, tests/gdi_bulk.rs)
    for n in ["LoadImageA", "CopyImage", "CopyCursor", "LoadCursorA", "LoadCursorFromFileW"] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be answered by this decision");
    }
}

#[test]
fn learn_values_in_the_header() {
    // official: Learn LoadImageW: IMAGE_BITMAP, IMAGE_CURSOR, IMAGE_ICON; LR_SHARED 0x00008000 (written here from the page)
    assert_eq!(def_const("fle_loadimage.h", "FLE_IMAGE_BITMAP"), 0);
    assert_eq!(def_const("fle_loadimage.h", "FLE_IMAGE_ICON"), 1);
    assert_eq!(def_const("fle_loadimage.h", "FLE_IMAGE_CURSOR"), 2);
    assert_eq!(def_const("fle_loadimage.h", "FLE_LR_SHARED"), 0x8000);
    // the guest's immediates (0x865a39..0x865a54): r9d = 0x20, [rsp+0x20] = 0x20
    assert_eq!(def_const("fle_loadimage.h", "FLE_LI_SIZE"), 32);
    // official: About Cursors numbers of the twelve identifiers added to fle_cursor.h
    for (id, name) in STD13.iter().skip(1) {
        let short = name.trim_start_matches("IDC_");
        assert_eq!(def_const("fle_cursor.h", &format!("FLE_CURSOR_IDC_{short}")), *id, "{name}");
    }
    // the DLL ids the guest loads (decision 0061: read from the DLL's RT_GROUP_CURSOR directory)
    assert_eq!(def_const("fle_loadimage.h", "FLE_LI_MODULE_FIRST"), 32762);
    assert_eq!(def_const("fle_loadimage.h", "FLE_LI_MODULE_LAST"), 32767);
    // decision 0062: the id the DLL lacks and that gets NULL (IDC_HAND, About Cursors table; the guest's esi = -21 call)
    assert_eq!(def_const("fle_loadimage.h", "FLE_LI_MODULE_MISSING"), 32649);
}

#[test]
fn the_handle_regions_are_decided_below_2_gib_and_apart_from_the_other_tables() {
    let base = def_const("fle_loadimage.h", "FLE_LI_MODULE_HANDLE_BASE");
    let size = def_const("fle_loadimage.h", "FLE_LI_REGION_SIZE");
    assert_eq!(base, 0x7FDA_0000);
    assert_eq!(size, 0x10_0000);
    assert!(base + size <= 0x8000_0000, "Learn 'Interprocess communication': below 2^31 so truncation and sign extension agree");
    assert!(mod_handle(32762) > base && mod_handle(32767) < base + size);
    let (sb, ss) = (def_const("fle_cursor.h", "FLE_CURSOR_HANDLE_BASE"), def_const("fle_cursor.h", "FLE_CURSOR_REGION_SIZE"));
    let (gb, gs) = (def_const("fle_gdi.h", "FLE_GDI_HANDLE_BASE"), def_const("fle_gdi.h", "FLE_GDI_REGION_SIZE"));
    let (cb, cs) = (def_const("fle_gdc.h", "FLE_GDC_HANDLE_BASE"), def_const("fle_gdc.h", "FLE_GDC_REGION_SIZE"));
    for (n, b, s) in [("standard cursors", sb, ss), ("GDI objects", gb, gs), ("screen DCs", cb, cs)] {
        assert!(base >= b + s || base + size <= b, "overlaps the {n} region");
    }
    assert!(base > 0x400000 + 0x18c0000, "above the image");
}

// ---- 2. Answers ----

#[test]
fn the_thirteen_standard_cursors_are_answered_with_the_formula_handles() {
    real!(dll);
    let mut seen = Vec::new();
    for (id, name) in STD13 {
        let h = load(&dll, 0, id).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(h, std_handle(id), "{name}");
        seen.push(h);
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 13, "one handle per identifier");
    assert_eq!(std_handle(32512), 0x7FCA_0004);
}

#[test]
fn idc_arrow_has_the_same_handle_in_loadimagew_and_loadcursorw() {
    real!(dll);
    // decision 0061: Learn LR_SHARED "Shares the image handle if the image is loaded multiple times"; LoadCursor "always uses
    // LR_SHARED internally" -> one shared cursor object per identifier, so one handle (0x7FCA0004 of decision 0059)
    let l = lines_on(
        "li-same-as-lc",
        &dll,
        r#"
    uint64_t a = LIC(0, 32512, 2, 32, 32, 0x8000), c;
    R[1] = 0; R[2] = 32512; R[4] = (uint64_t)(uintptr_t)stk; stk[0] = 0xdead0000u;
    fle_imp_LoadCursorW();
    c = R[0];
    printf("%llx %llx %d\n", U(a), U(c), a == c);
"#,
    );
    assert_eq!(l, [format!("{:x} {:x} 1", std_handle(32512), std_handle(32512))]);
}

#[test]
fn the_dll_cursors_32762_to_32767_are_answered_with_the_formula_handles() {
    real!(dll);
    let mut seen = Vec::new();
    for id in 32762..=32767u64 {
        let h = load(&dll, MODULE, id).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(h, mod_handle(id), "{id}");
        seen.push(h);
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 6);
    assert_eq!(mod_handle(32762), 0x7FDA_0004);
    assert_eq!(mod_handle(32767), 0x7FDA_0018);
    // a module cursor is not a standard one
    for (id, _) in STD13 {
        assert!(!seen.contains(&std_handle(id)));
    }
}

#[test]
fn the_same_arguments_give_the_same_handle_on_every_call_and_nothing_else_matters() {
    real!(dll);
    let l = lines_on(
        "li-same",
        &dll,
        r#"
    uint64_t a = LIC(0, 32646, 2, 32, 32, 0x8000), b, m = LIC(0x400000, 32765, 2, 32, 32, 0x8000), i;
    uint64_t dc = 0;
    SETLE(0x1234);
    b = LIC(0, 32646, 2, 32, 32, 0x8000);
    for (i = 0; i < 1000; i++) {
        if (LIC(0, 32646, 2, 32, 32, 0x8000) != a || LIC(0x400000, 32765, 2, 32, 32, 0x8000) != m) { printf("changed at %llu\n", U(i)); return 1; }
    }
    (void)dc;
    printf("%llx %llx %llx\n", U(a), U(b), U(m));
    printf("last error %x\n", LE());
    printf("return address %llx\n", U(stk[0]));
"#,
    );
    assert_eq!(l, [format!("{:x} {:x} {:x}", std_handle(32646), std_handle(32646), mod_handle(32765)), "last error 1234".to_string(), "return address dead0000".to_string()]);
}

#[test]
fn only_the_low_32_bits_of_type_cx_cy_and_fuload_count() {
    real!(dll);
    // type UINT, cx int, cy int, fuLoad UINT: the upper half of a 32-bit argument is not defined by the x64 convention (Learn: x64
    // calling convention: "the callee can ignore the upper bits of the register"), so garbage there must not change the answer
    let g: u64 = 0xDEAD_BEEF_0000_0000;
    assert_eq!(call_on(&dll, [0, 32512, g | 2, g | 32, g | 32, g | 0x8000]), Ok(std_handle(32512)));
    assert_eq!(call_on(&dll, [MODULE, 32762, g | 2, g | 32, g | 32, g | 0x8000]), Ok(mod_handle(32762)));
    // the whole 64 bits of hInst and name do count
    stops(&dll, [1 << 32, 32512, IC, 32, 32, SHARED], "hInst");
    stops(&dll, [0, (1 << 32) | 32512, IC, 32, 32, SHARED], "pointer");
}

// ---- 3. Stops ----

#[test]
fn another_type_stops() {
    real!(dll);
    // Learn: IMAGE_BITMAP 0 and IMAGE_ICON 1 are valid values the guest does not use
    for t in [0u64, 1, 3, 4, 0xFFFF_FFFF] {
        stops(&dll, [0, 32512, t, 32, 32, SHARED], &format!("type {t} is not IMAGE_CURSOR"));
        stops(&dll, [MODULE, 32762, t, 32, 32, SHARED], &format!("type {t} is not IMAGE_CURSOR"));
    }
}

#[test]
fn another_size_stops() {
    real!(dll);
    for (cx, cy) in [(0u64, 0u64), (16, 16), (32, 31), (31, 32), (48, 48), (64, 64), (32, 0), (0, 32), (0xFFFF_FFE0, 32), (32, 0xFFFF_FFE0)] {
        let e = call_on(&dll, [0, 32512, IC, cx, cy, SHARED]).expect_err("size");
        assert!(e.contains("is not 32x32") && e.contains("decision 0061"), "{cx:#x}x{cy:#x}: {e}");
        let e = call_on(&dll, [MODULE, 32762, IC, cx, cy, SHARED]).expect_err("size");
        assert!(e.contains("is not 32x32"), "{cx:#x}x{cy:#x}: {e}");
    }
}

#[test]
fn another_fuload_stops() {
    real!(dll);
    // Learn flags: LR_DEFAULTSIZE 0x40, LR_LOADFROMFILE 0x10, LR_MONOCHROME 0x1, LR_VGACOLOR 0x80, LR_LOADTRANSPARENT 0x20; 0 = no LR_SHARED
    for fu in [0u64, 0x40, 0x10, 0x8000 | 0x40, 0x8000 | 0x10, 0x8000 | 1, 0x8000 | 0x80, 0x8000 | 0x20, 0x8000 | 0x2000, 0x1_8000, 0x8001_8000 & !0x8000] {
        let e = call_on(&dll, [0, 32512, IC, 32, 32, fu]).expect_err("fuLoad");
        assert!(e.contains("is not exactly LR_SHARED") && e.contains("decision 0061"), "{fu:#x}: {e}");
        let e = call_on(&dll, [MODULE, 32762, IC, 32, 32, fu]).expect_err("fuLoad");
        assert!(e.contains("is not exactly LR_SHARED"), "{fu:#x}: {e}");
    }
}

#[test]
fn a_string_name_stops() {
    real!(dll);
    // Learn IS_INTRESOURCE: all bits above the low 16 zero = identifier; otherwise a pointer to a string
    for name in [0x1_0000u64, 0x7FF7_0000_1000, 0x1_0000_0000 + 32512, 0x10000 + 32512, u64::MAX] {
        stops(&dll, [0, name, IC, 32, 32, SHARED], "name is a pointer to a string");
        stops(&dll, [MODULE, name, IC, 32, 32, SHARED], "name is a pointer to a string");
    }
}

#[test]
fn a_null_hinst_with_an_identifier_the_guest_does_not_load_stops() {
    real!(dll);
    let mut ids: Vec<u64> = NOT_STD.to_vec();
    // IDC_HAND is above; the DLL ids with NULL (they are not predefined images); the neighbours; the ends of the 16-bit range
    ids.extend([0, 1, 2, 32511, 32517, 32641, 32647, 32761, 32762, 32767, 32768, 0xFFFE, 0xFFFF]);
    for id in ids {
        let e = call_on(&dll, [0, id, IC, 32, 32, SHARED]).expect_err(&format!("{id}"));
        assert!(e.contains(&format!("MAKEINTRESOURCE({id}) is not one of the 13 standard cursors")) && e.contains("decision 0061"), "{id}: {e}");
    }
}

#[test]
fn a_hinst_that_is_neither_null_nor_the_dll_stops() {
    real!(dll);
    // 1: small; 0x7FF800010000: a virtual module handle (fle_modules.h); 0x1_0040_0000 / 0x4_0000_0000_0000: only upper bits set
    // around the DLL's handle (all 64 bits count); 0x401000: inside the image but not its handle; u64::MAX
    for inst in [1u64, 0x7FF8_0001_0000, 0x1_0040_0000, 0x4_0000_0040_0000, 0x401000, 0x3FFFFF, 0x400001, u64::MAX] {
        stops(&dll, [inst, 32762, IC, 32, 32, SHARED], "hInst is neither NULL nor the lifted DLL");
        stops(&dll, [inst, 32512, IC, 32, 32, SHARED], "hInst is neither NULL nor the lifted DLL");
    }
}

#[test]
fn the_dll_has_no_cursor_32649_and_loadimagew_returns_null_without_touching_the_last_error() {
    real!(dll);
    // the guest's esi = -21 call: hInst = the DLL, IDC_HAND = 32649; the DLL's RT_GROUP_CURSOR directory has no such id (see the
    // DLL-facts test). Decision 0062 (PROVISIONAL, PM): NULL, which Learn's LoadImageW page names as the failure value; the page
    // gives no GetLastError value, so none is invented and the last error keeps what it had (decision 0061 had stopped here).
    assert_eq!(load(&dll, MODULE, 32649), Ok(0));
    let l = lines_on(
        "li-32649-null",
        &dll,
        r#"
    uint64_t r;
    SETLE(0x1234);
    r = LIC(0x400000, 32649, 2, 32, 32, 0x8000);
    printf("ret %llx last error %x return address %llx\n", U(r), LE(), U(stk[0]));
    SETLE(0);
    r = LIC(0x400000, 32649, 2, 32, 32, 0x8000);
    printf("ret %llx last error %x\n", U(r), LE());
"#,
    );
    assert_eq!(l, ["ret 0 last error 1234 return address dead0000", "ret 0 last error 0"]);
}

#[test]
fn only_the_32649_form_of_the_guest_returns_null_every_other_shape_with_it_still_stops() {
    real!(dll);
    // 0062 changes the one form (type IMAGE_CURSOR, 32x32, fuLoad exactly LR_SHARED, integer name 32649, hInst the DLL); the checks
    // before it are unchanged: type, fuLoad, size, a string name, another hInst, and NULL hInst with 32649 (not a standard cursor)
    stops(&dll, [MODULE, 32649, 1, 32, 32, SHARED], "type 1 is not IMAGE_CURSOR");
    stops(&dll, [MODULE, 32649, IC, 32, 32, 0], "is not exactly LR_SHARED");
    stops(&dll, [MODULE, 32649, IC, 32, 32, SHARED | 0x40], "is not exactly LR_SHARED");
    stops(&dll, [MODULE, 32649, IC, 0, 0, SHARED | 0x40], "is not exactly LR_SHARED");
    stops(&dll, [MODULE, 32649, IC, 48, 48, SHARED], "is not 32x32");
    stops(&dll, [MODULE, 0x1_0000 | 32649, IC, 32, 32, SHARED], "name is a pointer");
    stops(&dll, [1, 32649, IC, 32, 32, SHARED], "hInst is neither NULL nor the lifted DLL");
    stops(&dll, [0, 32649, IC, 32, 32, SHARED], "MAKEINTRESOURCE(32649) is not one of the 13 standard cursors");
    // the neighbours of 32649 that the DLL does not have, the other ids it lacks, and 32761 (it has it; the guest does not load it)
    for id in [32648u64, 32650, 32651, 32512, 0, 1, 0xFFFF] {
        let e = load(&dll, MODULE, id).expect_err(&format!("{id}"));
        assert!(e.contains(&format!("the lifted DLL has no cursor MAKEINTRESOURCE({id})")) && e.contains("decision 0061"), "{id}: {e}");
        // the reason keeps saying why no failure is invented for these ids: the page names no GetLastError value
        assert!(e.contains("Learn gives no GetLastError value"), "{id}: {e}");
    }
    let e = load(&dll, MODULE, 32761).expect_err("32761");
    assert!(e.contains("is not one the guest loads"), "{e}");
}

#[test]
fn a_dll_that_does_have_a_32649_group_does_not_get_the_null_answer() {
    // the NULL answer is for the id the DLL lacks: a synthetic image that has the group 32649 (and its component) is not answered
    // by 0062's NULL; its id is not one the guest loads (32762..32767), so it still stops by that reason
    let p = write_image("li-has-32649", &cursor_tree_id(32649, &group_bytes(0, 2, 1, 5), true));
    let e = call_on(&p, [MODULE, 32649, IC, 32, 32, SHARED]).unwrap_err();
    assert!(e.contains("is not one the guest loads (32762..32767)") && e.contains("decision 0061"), "{e}");
    // and the same image lacks 32762: a different lacking id stops with the no-cursor reason, not NULL
    let e = call_on(&p, [MODULE, 32762, IC, 32, 32, SHARED]).unwrap_err();
    assert!(e.contains("the lifted DLL has no cursor MAKEINTRESOURCE(32762)"), "{e}");
}

#[test]
fn a_dll_cursor_the_guest_does_not_load_or_the_dll_does_not_have_stops() {
    real!(dll);
    // 32761 exists in the DLL but the guest's table has no such entry: not answered (it needs a decision of its own)
    let e = load(&dll, MODULE, 32761).expect_err("32761");
    assert!(e.contains("is not one the guest loads (32762..32767)") && e.contains("decision 0061"), "{e}");
    // the standard identifiers and small numbers are not RT_GROUP_CURSOR ids of the DLL (id 1 is an RT_CURSOR, not a group)
    for id in [0u64, 1, 2, 32512, 32646, 32768, 0xFFFF] {
        let e = load(&dll, MODULE, id).expect_err(&format!("{id}"));
        assert!(e.contains(&format!("the lifted DLL has no cursor MAKEINTRESOURCE({id})")), "{id}: {e}");
    }
}

#[test]
fn the_order_of_the_checks_is_type_fuload_size_name_hinst() {
    real!(dll);
    // each argument is wrong together with the later ones: the first check in this order is the one reported
    let e = call_on(&dll, [1, 0x10000, 0, 16, 16, 0]).unwrap_err();
    assert!(e.contains("type 0 is not IMAGE_CURSOR"), "{e}");
    let e = call_on(&dll, [1, 0x10000, IC, 16, 16, 0]).unwrap_err();
    assert!(e.contains("fuLoad 0 is not exactly LR_SHARED"), "{e}");
    let e = call_on(&dll, [1, 0x10000, IC, 16, 16, SHARED]).unwrap_err();
    assert!(e.contains("is not 32x32"), "{e}");
    let e = call_on(&dll, [1, 0x10000, IC, 32, 32, SHARED]).unwrap_err();
    assert!(e.contains("name is a pointer"), "{e}");
    let e = call_on(&dll, [1, 32512, IC, 32, 32, SHARED]).unwrap_err();
    assert!(e.contains("hInst is neither NULL nor the lifted DLL"), "{e}");
}

// ---- 4. Inspection functions and the other handle tables ----

#[test]
fn the_inspection_functions_agree_with_the_formulas() {
    real!(dll);
    let l = lines_on(
        "li-inspect",
        &dll,
        r#"
    uint64_t h = fle_cursor_image_handle_of(32646);
    printf("%llx %u\n", U(h), fle_cursor_image_idc_of(h));
    printf("%llx %llx %llx\n", U(fle_cursor_image_handle_of(32649)), U(fle_cursor_image_handle_of(0)), U(fle_cursor_image_handle_of(32511)));
    printf("%u %u %u\n", fle_cursor_image_idc_of(0), fle_cursor_image_idc_of(h + 4), fle_cursor_image_idc_of(h + 1));
    /* LoadCursorW's own inspection is unchanged: IDC_ARROW only */
    printf("%llx %llx %u %u\n", U(fle_cursor_handle_of(32646)), U(fle_cursor_handle_of(32512)), fle_cursor_idc_of(h), fle_cursor_idc_of(fle_cursor_handle_of(32512)));
    h = fle_loadimage_module_handle_of(32765);
    printf("%llx %u\n", U(h), fle_loadimage_module_id_of(h));
    printf("%llx %llx %llx %llx\n", U(fle_loadimage_module_handle_of(32761)), U(fle_loadimage_module_handle_of(32768)), U(fle_loadimage_module_handle_of(0)), U(fle_loadimage_module_handle_of(32649)));
    printf("%u %u %u %u\n", fle_loadimage_module_id_of(0), fle_loadimage_module_id_of(h + 4), fle_loadimage_module_id_of(h + 1), fle_loadimage_module_id_of(fle_cursor_handle_of(32512)));
"#,
    );
    let (a, b) = (std_handle(32646), std_handle(32512));
    assert_eq!(
        l,
        [
            format!("{a:x} 32646"),
            "0 0 0".to_string(),
            "0 0 0".to_string(),
            format!("0 {b:x} 0 32512"),
            format!("{:x} 32765", mod_handle(32765)),
            "0 0 0 0".to_string(),
            "0 32766 0 0".to_string(),
        ]
    );
}

#[test]
fn a_loadimagew_handle_is_not_a_gdi_object_or_a_dc() {
    real!(dll);
    // decision 0061: the cursor and DLL-cursor regions are their own; the GDI table (0054) and the DC table (0053) stop on them
    for (name, body, needle) in [
        ("li-delobj-std", "uint64_t h = LIC(0, 32512, 2, 32, 32, 0x8000); R[1] = h; R[4] = (uint64_t)(uintptr_t)stk; fle_imp_DeleteObject();", "fle: DeleteObject"),
        ("li-delobj-mod", "uint64_t h = LIC(0x400000, 32762, 2, 32, 32, 0x8000); R[1] = h; R[4] = (uint64_t)(uintptr_t)stk; fle_imp_DeleteObject();", "fle: DeleteObject"),
        ("li-relDC-mod", "uint64_t h = LIC(0x400000, 32762, 2, 32, 32, 0x8000); R[1] = 0; R[2] = h; R[4] = (uint64_t)(uintptr_t)stk; fle_imp_ReleaseDC();", "fle: ReleaseDC"),
    ] {
        let exe = compile_main(name, body);
        let o = exec(&exe, &[dll.to_str().unwrap()]);
        assert_eq!(o.signal, Some(6), "{name}: {}", o.stderr);
        assert!(o.stderr.contains(needle) && o.stderr.contains("not issued"), "{name}: {}", o.stderr);
    }
}

// ---- 5. Source checks ----

#[test]
fn source_uses_no_host_cursor_theme_mouse_display_clock_thread_process_or_file() {
    let code = strip_comments(&std::fs::read_to_string(src("fle_loadimage.c")).unwrap());
    for bad in [
        "getpid", "getenv", "environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "clock_gettime", "gettimeofday", "time(",
        "nanosleep", "sleep(", "pthread", "<sys/", "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "futex", "asm", "abort(", "X11", "DISPLAY", "XCursor",
        "xcursor", "gsettings", "xinput", "libinput", "dconf", "wayland", "ioctl", "emscripten", "SDL", "devicePixelRatio", "fle_host", "fle_reg", "fle_vfs",
        "fle_gdi", "fle_gdc", "fle_load.h", "fle_load(", "malloc", "realloc", "free(", "fle_set_last_error", "fle_st(", "fle_ld(", "LoadResource", "LockResource",
    ] {
        assert!(!code.contains(bad), "fle_loadimage.c must not use {bad}");
    }
    // stateless: only constants and functions are static (no mutable static variable)
    for line in code.lines().filter(|l| l.starts_with("static ")) {
        assert!(line.contains("const "), "fle_loadimage.c must be stateless (mutable static: {line})");
    }
    // the numbers are named in the headers
    for must in ["FLE_IMAGE_CURSOR", "FLE_LR_SHARED", "FLE_LI_SIZE", "FLE_EXE_HANDLE", "FLE_LI_MODULE_HANDLE_BASE", "fle_stop(", "fle_res_cursor_group_ok(", "fle_cursor_image_handle_of(", "FLE_LI_MODULE_MISSING"] {
        assert!(code.contains(must), "fle_loadimage.c must use {must}");
    }
    for bad in ["0x8000", "0x7FDA", "0x400000", "== 32", "!= 32", "32649", "32761"] {
        assert!(!code.contains(bad), "the number {bad} belongs in a header");
    }
    // `long` is not portable: fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_loadimage.c uses `long` near byte {at}");
        i = at + 4;
    }
}

#[test]
fn header_is_plain_c() {
    let h = strip_comments(&std::fs::read_to_string(src("fle_loadimage.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_loadimage.h must not contain {bad}");
    }
}

#[test]
fn the_resource_check_reads_only_the_dlls_own_directory_and_leaves_the_last_error() {
    let code = strip_comments(&std::fs::read_to_string(src("fle_res.c")).unwrap());
    // decision 0125: the body of the check moved into the static helper cursor_group_ok_key (old: it was in fle_res_cursor_group_ok_at itself); the function now ends where the first function of decision 0125 begins
    let end = code.find("int fle_cursor_load_named").expect("the function of decision 0125 follows");
    let at = code.find("static int cursor_group_ok_key").expect("helper");
    let f = &code[at..end];
    assert!(!f.contains("fle_set_last_error"), "the check must not touch the last error");
    assert!(f.contains("img_open(") && f.contains("find_leaf("), "it reads the mapped image's resource directory");
}

// ---- 6. Generator: the thunk calls the runtime ----

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

/// A PE that imports `dll`!`api` (slot 0x2200). Entry 0x1000: `call 0x1040; ret`; 0x1040 is the thunk `jmp [slot]`.
fn build(dll: &str, api: &str) -> Vec<u8> {
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
    let mut nm = dll.as_bytes().to_vec();
    nm.push(0);
    put(&mut b, 0x2300, &nm);
    let mut hn = vec![0u8, 0u8];
    hn.extend_from_slice(api.as_bytes());
    hn.push(0);
    put(&mut b, 0x2400, &hn);
    for (rva, v) in [(0x2100, 0x2400u64), (0x2200, 0x2400)] {
        w64(&mut b, data_at(rva), v);
    }
    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, [&[0xe8][..], &rel(0x1005, 0x1040)].concat()),
        (0x1005, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn the_thunk_calls_the_runtime() {
    let img = build("user32.dll", "LoadImageW");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_imp:LoadImageW"], 1);
    assert!(c.contains("fle_imp_LoadImageW(); return;"));
    let va = IB + 0x1040;
    assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{va:#x} still unsupported");
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
}

#[test]
fn the_dll_name_and_the_api_name_are_part_of_the_match() {
    for (dll, api) in [("other.dll", "LoadImageW"), ("user32.dll", "LoadImageA"), ("user32.dll", "CopyImage"), ("user32.dll", "LoadCursorFromFileW")] {
        let img = build(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert!(!st.handled.contains_key("jmp_imp:LoadImageW"), "{dll}!{api}");
        assert!(!c.contains("fle_imp_LoadImageW"), "{dll}!{api}");
        let va = IB + 0x1040;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
    }
}

// ---- 7. The DLL: what the investigation rests on ----

fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}
/// The section table read here, not through fe-pe: (name, virtual address, virtual size, raw offset, raw size).
fn sections(dll: &[u8]) -> Vec<(String, u32, u32, u32, u32)> {
    let lf = le32(dll, 0x3c) as usize;
    let nsec = le16(dll, lf + 6) as usize;
    let osz = le16(dll, lf + 20) as usize;
    (0..nsec)
        .map(|i| {
            let o = lf + 24 + osz + 40 * i;
            let name = String::from_utf8_lossy(&dll[o..o + 8]).trim_end_matches('\0').to_string();
            (name, le32(dll, o + 12), le32(dll, o + 8), le32(dll, o + 20), le32(dll, o + 16))
        })
        .collect()
}
fn bytes_at(dll: &[u8], va: u64, n: usize) -> Vec<u8> {
    let rva = (va - 0x400000) as u32;
    for (_, sva, vs, rp, rs) in sections(dll) {
        if rva >= sva && rva < sva + vs.max(rs) {
            let off = rp as usize + (rva - sva) as usize;
            return dll[off..off + n].to_vec();
        }
    }
    panic!("rva {rva:#x} is in no section");
}

/// (RT_GROUP_CURSOR integer ids, RT_CURSOR integer ids, the components each group lists) of the DLL, from fe-pe.
fn dll_cursors(data: &[u8]) -> (Vec<u32>, Vec<u32>, Vec<(u32, Vec<u32>)>) {
    let pe = Pe::parse(data).unwrap();
    let (mut groups, mut cursors, mut comps) = (Vec::new(), Vec::new(), Vec::new());
    for leaf in pe.resources().unwrap() {
        match (&leaf.type_id, &leaf.name) {
            (ResourceId::Id(12), ResourceId::Id(n)) => {
                groups.push(*n);
                let d = pe.resource_data(&leaf).unwrap();
                // NEWHEADER (Reserved, ResType, ResCount) then ResCount RESDIR of 14 bytes, the id last (Learn: NEWHEADER, RESDIR)
                assert_eq!((le16(d, 0), le16(d, 2)), (0, 2), "group {n}: NEWHEADER");
                let cnt = le16(d, 4) as usize;
                assert!(6 + 14 * cnt <= d.len());
                comps.push((*n, (0..cnt).map(|i| le16(d, 6 + 14 * i + 12) as u32).collect()));
            }
            (ResourceId::Id(1), ResourceId::Id(n)) => cursors.push(*n),
            _ => {}
        }
    }
    (groups, cursors, comps)
}

#[test]
fn the_dll_has_the_rt_group_cursors_32761_to_32767_and_not_32649() {
    real!(dll);
    let data = std::fs::read(&dll).unwrap();
    let (groups, cursors, comps) = dll_cursors(&data);
    assert_eq!(groups.iter().filter(|g| **g >= 32000).count(), 7, "numeric RT_GROUP_CURSOR ids: {groups:?}");
    for id in 32761..=32767 {
        assert!(groups.contains(&id), "{id}");
    }
    assert!(!groups.contains(&32649), "no RT_GROUP_CURSOR 32649 in the DLL");
    for id in [32512, 32513, 32646, 32650, 32651, 32514] {
        assert!(!groups.contains(&id), "{id}: the standard identifiers are not ids of the DLL");
    }
    // every component of the seven groups exists as an RT_CURSOR (one component each)
    for id in 32761..=32767 {
        let c = &comps.iter().find(|(g, _)| *g == id).unwrap().1;
        assert_eq!(c.len(), 1, "{id}");
        assert!(cursors.contains(&c[0]), "{id}: component {} is not an RT_CURSOR", c[0]);
    }
    // the other 50 RT_GROUP_CURSOR of the DLL are named (VT_*, EE*, ...), not numeric; RT_CURSOR: 60 (loadcursor-caller.md section 2)
    assert_eq!((groups.len(), cursors.len()), (7, 60));
}

#[test]
fn the_guests_21_calls_are_all_answered_and_the_module_idc_hand_is_null() {
    real!(dll);
    let data = std::fs::read(&dll).unwrap();
    // the call site and the loop (0x865a0d..0x865a71): the table at 0x1409038, 8 bytes each, esi = -22 ..= -2; hInst is the module
    // for esi in -17 ..= -12 and for -21 (0x865a19..0x865a2f), NULL otherwise
    assert_eq!(bytes_at(&data, 0x865a0d, 5), [0xbe, 0xea, 0xff, 0xff, 0xff]);
    assert_eq!(bytes_at(&data, 0x865a19, 12), [0x83, 0xfe, 0xef, 0x7c, 0x05, 0x83, 0xfe, 0xf4, 0x7e, 0x05, 0x83, 0xfe]);
    assert_eq!(bytes_at(&data, 0x865a23, 4), [0x83, 0xfe, 0xeb, 0x75]);
    let tab = bytes_at(&data, 0x1409038, 21 * 8);
    let mut handles = Vec::new();
    let mut nulls = Vec::new();
    for i in 0..21usize {
        let esi = -22 + i as i32;
        let id = u64::from_le_bytes(tab[8 * i..8 * i + 8].try_into().unwrap());
        let inst = if (-17..=-12).contains(&esi) || esi == -21 { MODULE } else { 0 };
        let h = call_on(&dll, [inst, id, IC, 32, 32, SHARED]).unwrap_or_else(|e| panic!("esi {esi} id {id}: {e}"));
        if h == 0 {
            nulls.push((esi, inst, id));
            continue;
        }
        handles.push((inst, id, h));
        let want = if inst == 0 { std_handle(id) } else { mod_handle(id) };
        assert_eq!(h, want, "esi {esi} id {id}");
    }
    // decision 0062: the only NULL is the esi = -21 call (hInst = the DLL, 32649: the DLL has no such cursor); 20 handles
    assert_eq!(nulls, [(-21, MODULE, 32649)]);
    assert_eq!(handles.len(), 20);
    // IDC_SIZEALL is in the table twice (esi -22 and -5): the same handle
    let sizeall: Vec<u64> = handles.iter().filter(|h| h.1 == 32646).map(|h| h.2).collect();
    assert_eq!(sizeall, [std_handle(32646), std_handle(32646)]);
    // 13 distinct standard handles minus nothing + 6 DLL handles = 19 distinct values over the 20 answers
    let mut v: Vec<u64> = handles.iter().map(|h| h.2).collect();
    v.sort();
    v.dedup();
    assert_eq!(v.len(), 19);
}

// ---- 8. A synthetic image: the NEWHEADER / RESDIR / component checks of the resource lookup ----

const S_RSRC_RVA: u32 = 0x1000;
const S_RSRC_RAW: usize = 0x400;
const S_RSRC_LEN: usize = 0x800;
const HIBIT: u32 = 0x8000_0000;

fn s_table(r: &mut [u8], off: usize, n_name: u16, n_id: u16, entries: &[(u32, u32)]) {
    w16(r, off + 12, n_name);
    w16(r, off + 14, n_id);
    for (i, (a, b)) in entries.iter().enumerate() {
        w32(r, off + 16 + 8 * i, *a);
        w32(r, off + 20 + 8 * i, *b);
    }
}
fn s_leaf(r: &mut [u8], off: usize, data_off: u32, size: u32) {
    w32(r, off, S_RSRC_RVA + data_off);
    w32(r, off + 4, size);
}

/// A PE32+ at ImageBase 0x400000 (size 0x3000) with a .rsrc section at RVA 0x1000; `fill` writes the tree.
fn build_rsrc(fill: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let mut b = vec![0u8; 0x1000];
    b.resize(S_RSRC_RAW + S_RSRC_LEN + 0x400, 0);
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 1);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w64(&mut b, o + 24, 0x40_0000);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 2);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 128, S_RSRC_RVA);
    w32(&mut b, o + 132, S_RSRC_LEN as u32);
    let s0 = o + 240;
    b[s0..s0 + 5].copy_from_slice(b".rsrc");
    w32(&mut b, s0 + 8, S_RSRC_LEN as u32);
    w32(&mut b, s0 + 12, S_RSRC_RVA);
    w32(&mut b, s0 + 16, S_RSRC_LEN as u32);
    w32(&mut b, s0 + 20, S_RSRC_RAW as u32);
    w32(&mut b, s0 + 36, 0x4000_0040);
    fill(&mut b[S_RSRC_RAW..S_RSRC_RAW + S_RSRC_LEN]);
    b
}

/// RT_GROUP_CURSOR (12) id 32762 with `group` bytes; RT_CURSOR (1) id 5 with 8 bytes when `with_cursor`.
fn cursor_tree(group: &[u8], with_cursor: bool) -> Vec<u8> {
    cursor_tree_id(32762, group, with_cursor)
}

/// The same with the RT_GROUP_CURSOR id `gid`.
fn cursor_tree_id(gid: u32, group: &[u8], with_cursor: bool) -> Vec<u8> {
    build_rsrc(|r| {
        let types: Vec<(u32, u32)> = if with_cursor { vec![(1, HIBIT | 0x100), (12, HIBIT | 0x140)] } else { vec![(12, HIBIT | 0x140)] };
        s_table(r, 0x000, 0, types.len() as u16, &types);
        if with_cursor {
            s_table(r, 0x100, 0, 1, &[(5, HIBIT | 0x120)]);
            s_table(r, 0x120, 0, 1, &[(0x409, 0x160)]);
            s_leaf(r, 0x160, 0x300, 8);
        }
        s_table(r, 0x140, 0, 1, &[(gid, HIBIT | 0x180)]);
        s_table(r, 0x180, 0, 1, &[(0x409, 0x1a0)]);
        s_leaf(r, 0x1a0, 0x400, group.len() as u32);
        r[0x400..0x400 + group.len()].copy_from_slice(group);
    })
}

/// NEWHEADER(reserved, restype, count) and `count` RESDIR of 14 bytes, the last WORD being the RT_CURSOR id `comp`.
fn group_bytes(reserved: u16, restype: u16, count: u16, comp: u16) -> Vec<u8> {
    let mut g = Vec::new();
    for v in [reserved, restype, count] {
        g.extend_from_slice(&v.to_le_bytes());
    }
    for _ in 0..count {
        g.extend_from_slice(&[32, 0, 64, 0, 1, 0, 1, 0, 0x34, 0x01, 0, 0]); // width, height, planes, bitcount, bytes (308)
        g.extend_from_slice(&comp.to_le_bytes());
    }
    g
}

fn write_image(name: &str, bytes: &[u8]) -> PathBuf {
    let p = work_dir().join(format!("{}.dll", etxt_common::per_process(name)));
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn synthetic_group_with_its_component_is_answered() {
    let p = write_image("li-good", &cursor_tree(&group_bytes(0, 2, 1, 5), true));
    assert_eq!(call_on(&p, [MODULE, 32762, IC, 32, 32, SHARED]), Ok(mod_handle(32762)));
}

#[test]
fn synthetic_group_whose_component_is_missing_stops_as_no_cursor() {
    // the group lists RT_CURSOR 6, the image has only RT_CURSOR 5
    let p = write_image("li-nocomp", &cursor_tree(&group_bytes(0, 2, 1, 6), true));
    let e = call_on(&p, [MODULE, 32762, IC, 32, 32, SHARED]).unwrap_err();
    assert!(e.contains("the lifted DLL has no cursor MAKEINTRESOURCE(32762)"), "{e}");
    // no RT_CURSOR type at all
    let p = write_image("li-notype", &cursor_tree(&group_bytes(0, 2, 1, 5), false));
    let e = call_on(&p, [MODULE, 32762, IC, 32, 32, SHARED]).unwrap_err();
    assert!(e.contains("the lifted DLL has no cursor MAKEINTRESOURCE(32762)"), "{e}");
}

#[test]
fn synthetic_group_with_two_components_needs_both() {
    let mut g = group_bytes(0, 2, 2, 5);
    let n = g.len();
    g[n - 2..].copy_from_slice(&7u16.to_le_bytes()); // second component RT_CURSOR 7: missing
    let p = write_image("li-two", &cursor_tree(&g, true));
    let e = call_on(&p, [MODULE, 32762, IC, 32, 32, SHARED]).unwrap_err();
    assert!(e.contains("the lifted DLL has no cursor MAKEINTRESOURCE(32762)"), "{e}");
}

#[test]
fn synthetic_malformed_groups_stop_by_name() {
    for (label, g, needle) in [
        ("short", vec![0u8, 0, 2, 0], "shorter than its NEWHEADER"),
        ("reserved", group_bytes(1, 2, 1, 5), "NEWHEADER is not (Reserved 0, ResType RES_CURSOR)"),
        ("icon", group_bytes(0, 1, 1, 5), "NEWHEADER is not (Reserved 0, ResType RES_CURSOR)"),
        ("zero", group_bytes(0, 2, 0, 5), "ResCount does not fit"),
        ("count", {
            let mut g = group_bytes(0, 2, 1, 5);
            g[4] = 2;
            g
        }, "ResCount does not fit"),
    ] {
        let p = write_image(&format!("li-bad-{label}"), &cursor_tree(&g, true));
        let e = call_on(&p, [MODULE, 32762, IC, 32, 32, SHARED]).unwrap_err();
        assert!(e.contains(needle), "{label}: {e}");
    }
}

#[test]
fn synthetic_image_without_a_resource_type_12_has_no_cursor() {
    let p = write_image("li-empty", &build_rsrc(|r| s_table(r, 0x000, 0, 0, &[])));
    let e = call_on(&p, [MODULE, 32762, IC, 32, 32, SHARED]).unwrap_err();
    assert!(e.contains("the lifted DLL has no cursor MAKEINTRESOURCE(32762)"), "{e}");
    // the NULL-hInst answers do not read the image at all
    assert_eq!(call_on(&p, [0, 32512, IC, 32, 32, SHARED]), Ok(std_handle(32512)));
}
