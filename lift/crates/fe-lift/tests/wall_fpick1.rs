//! Tests of decision 0193 (docs/0193-browser-file-bridge.md, PROVISIONAL): the browser branch of the file dialog model (lift/runtime/fle_filedlg.c) and the three volume functions
//! it uses (fle_vfs_make_dirs / fle_vfs_put_file / fle_vfs_file_state in fle_vfs.c).
//!
//! The "page" is a C stub of `fle_filedlg_host_pick` written in each test: it prints the JSON request it gets and returns the answer text a person's choice would produce. THE ANSWERS ARE THE TEST'S INPUT (a
//! simulation of the person at the page), not an output of the engine, and no browser is involved here. The real volume (fle_vfs.c) is linked, so the file the "page" hands over is really put on it and the engine-side
//! calls (GetFileAttributesW inside Show, CreateFileW / WriteFile / CloseHandle for the saved file) are the runtime's own. The vtable order, the GUIDs and the rows are those of wall_exp1.rs (written from the SDK headers).
//! FLE_FPICK1_RT_DIR (a full copy of runtime/) replaces runtime/ for the mutation runs.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_FPICK1_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-fpick1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const DIALOG_METHODS: [&str; 26] = [
    "Show", "SetFileTypes", "SetFileTypeIndex", "GetFileTypeIndex", "Advise", "Unadvise", "SetOptions", "GetOptions", "SetDefaultFolder", "SetFolder", "GetFolder", "GetCurrentSelection", "SetFileName",
    "GetFileName", "SetTitle", "SetOkButtonLabel", "SetFileNameLabel", "GetResult", "AddPlace", "SetDefaultExtension", "Close", "SetClientGuid", "ClearClientData", "SetFilter", "GetResults", "GetSelectedItems",
];
const SAVE_METHODS: [&str; 5] = ["SetSaveAsItem", "SetProperties", "SetCollectedProperties", "GetProperties", "ApplyProperties"];
const ITEM_METHODS: [&str; 5] = ["BindToHandler", "GetParent", "GetDisplayName", "GetAttributes", "Compare"];
const ARRAY_METHODS: [&str; 7] = ["BindToHandler", "GetPropertyStore", "GetPropertyDescriptionList", "GetAttributes", "GetCount", "GetItemAt", "EnumItems"];
const CUSTOM_METHODS: [&str; 27] = [
    "EnableOpenDropDown", "AddMenu", "AddPushButton", "AddComboBox", "AddRadioButtonList", "AddCheckButton", "AddEditBox", "AddSeparator", "AddText", "SetControlLabel", "GetControlState", "SetControlState",
    "GetEditBoxText", "SetEditBoxText", "GetCheckButtonState", "SetCheckButtonState", "AddControlItem", "RemoveControlItem", "RemoveAllControlItems", "GetControlItemState", "SetControlItemState",
    "GetSelectedControlItem", "SetSelectedControlItem", "StartVisualGroup", "EndVisualGroup", "MakeProminent", "SetControlItemText",
];

/// The names of the vtable rows in the order of the rows file.
fn row_names(save: bool) -> Vec<String> {
    let mut v: Vec<String> = ["QueryInterface", "AddRef", "Release"].iter().map(|m| format!("ShellObj::{m}")).collect();
    v.extend(DIALOG_METHODS.iter().map(|m| format!("IFileOpenDialog::{m}")));
    v.extend(ITEM_METHODS.iter().map(|m| format!("IShellItem::{m}")));
    v.extend(ARRAY_METHODS.iter().map(|m| format!("IShellItemArray::{m}")));
    v.extend(CUSTOM_METHODS.iter().map(|m| format!("IFileDialogCustomize::{m}")));
    if save {
        v.extend(SAVE_METHODS.iter().map(|m| format!("IFileSaveDialog::{m}")));
    }
    v
}



const GUIDS_C: &str = r#"
#define GUID(d1, d2, d3, a, b, c, d, e, f, g, h) { (d1) & 0xff, ((d1) >> 8) & 0xff, ((d1) >> 16) & 0xff, ((d1) >> 24) & 0xff, (d2) & 0xff, (d2) >> 8, (d3) & 0xff, (d3) >> 8, a, b, c, d, e, f, g, h }
static const uint8_t CLSID_FOD[16] = GUID(0xdc1c5a9cu, 0xe88a, 0x4dde, 0xa5, 0xa1, 0x60, 0xf8, 0x2a, 0x20, 0xae, 0xf7);
static const uint8_t CLSID_FSD[16] = GUID(0xc0b4e2f3u, 0xba21, 0x4773, 0x8d, 0xba, 0x33, 0x5e, 0xc9, 0x46, 0xeb, 0x8b);
static const uint8_t IID_Unknown[16] = GUID(0x00000000u, 0x0000, 0x0000, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46);
static const uint8_t IID_FOD[16] = GUID(0xd57c7288u, 0xd4ad, 0x4768, 0xbe, 0x02, 0x9d, 0x96, 0x95, 0x32, 0xd9, 0x60);
static const uint8_t IID_FSD[16] = GUID(0x84bccd23u, 0x5fde, 0x4cdb, 0xae, 0xa4, 0xaf, 0x64, 0xb8, 0x3d, 0x78, 0xab);
static const uint8_t IID_FD[16] = GUID(0x42f85136u, 0xdb7e, 0x439c, 0x85, 0xf1, 0xe4, 0x07, 0x5d, 0x13, 0x5f, 0xc8);
static const uint8_t IID_Modal[16] = GUID(0xb4db1657u, 0x70d7, 0x485e, 0x8e, 0x3e, 0x6f, 0xcb, 0x5a, 0x5c, 0x18, 0x02);
static const uint8_t IID_Item[16] = GUID(0x43826d1eu, 0xe718, 0x42ee, 0xbc, 0x55, 0xa1, 0xe2, 0x61, 0xc3, 0x7b, 0xfe);
static const uint8_t IID_Array[16] = GUID(0xb63ea76du, 0x1f85, 0x456f, 0xa1, 0x9c, 0x48, 0x15, 0x9e, 0xfa, 0x85, 0x8b);
static const uint8_t IID_Customize[16] = GUID(0xe6fdd21au, 0x163f, 0x4975, 0x9c, 0x8c, 0xa6, 0x9f, 0x1b, 0xa3, 0x70, 0x34);
static const uint8_t FOLDERID_Docs[16] = GUID(0xfdd39ad0u, 0x238f, 0x46af, 0xad, 0xb4, 0x6c, 0x85, 0x48, 0x03, 0x69, 0xc7);
static const uint8_t FOLDERID_Bogus[16] = GUID(0x12345678u, 0x1234, 0x1234, 1, 2, 3, 4, 5, 6, 7, 8);
/* methods through the vtable of an object: the VA in the slot -> the row name */


"#;

const PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_com.h"
#include "fle_modules.h"
#include "fle_vfs.h"
#include "fle_callback.h"
#include "fle_filedlg.h"
uint64_t R[16];
#define U(x) ((unsigned long long)(x))
static unsigned task_allocs;
void fle_imp_CoTaskMemAlloc(void) { task_allocs++; R[0] = (uint64_t)(uintptr_t)calloc(1, R[1]); }
uint64_t fle_call_guest(uint64_t va, uint64_t a, uint64_t b, uint64_t c, uint64_t d) { (void)a; (void)b; (void)c; (void)d; return va == 0x3000 ? 0 : 1; }
static uint16_t *W(const char *s) { return fle_w_from_ascii(s); }
static uint64_t P(const void *p) { return (uint64_t)(uintptr_t)p; }
static uint64_t call(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d) { R[0] = 0xdead; R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); return R[0]; }
static uint64_t stk7[32];
/* a call with up to 7 arguments the way the lifted code makes it: rcx rdx r8 r9 in R, the others at [rsp + 0x28 + 8 k] */
static uint64_t call7(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t g, uint64_t h) {
    memset(stk7, 0, sizeof stk7); stk7[0] = 0xdead0000u; stk7[5] = e; stk7[6] = g; stk7[7] = h; R[4] = P(stk7);
    R[0] = 0xdead; R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); return R[0];
}
void fle_imp_CoCreateInstance(void);
/* methods through the vtable of an object: the VA in the slot -> the row name */
static const char *va_name(uint64_t va) { uint64_t i; for (i = 0; i < fle_resp_count; i++) if (fle_resp_table[i].va == va) return fle_resp_table[i].name; return "?"; }
static uint64_t slot(uint64_t obj, unsigned k) { return fle_ld(fle_ld(obj, 8) + 8u * k, 8); }
/* call a method through the vtable the way the lifted code does: the VA is looked up in the response table and the function is called */
static uint64_t vcall(uint64_t obj, unsigned k, uint64_t b, uint64_t c, uint64_t d) {
    uint64_t va = slot(obj, k), i;
    for (i = 0; i < fle_resp_count; i++) if (fle_resp_table[i].va == va) { void (*f)(void) = fle_resp_table[i].fn; R[0] = 0xdead; R[1] = obj; R[2] = b; R[8] = c; R[9] = d; f(); return R[0]; }
    printf("no row for the VA in slot %u\n", k); exit(4);
}
static uint64_t cocreate(const uint8_t *clsid, uint64_t outer, const uint8_t *iid, uint64_t *out) { uint64_t stk[8] = { 0 }; stk[5] = P(out); R[4] = P(stk); return call(fle_imp_CoCreateInstance, P(clsid), outer, 1, P(iid)); }

void fle_imp_CoCreateInstance(void)
{
    uint64_t g = R[1], ppv = fle_arg(5); uint32_t hr;
    if (ppv == 0) fle_stop("CoCreateInstance", "ppv is NULL");
    fle_st(ppv, 8, 0);
    if (fle_filedlg_cocreate(g, R[2], (uint32_t)R[8], R[9], ppv, &hr)) { R[0] = hr; return; }
    R[0] = FLE_HR_REGDB_E_CLASSNOTREG;
}
static void show_path(const char *label, uint64_t wp) { const uint16_t *w = (const uint16_t *)(uintptr_t)wp; char b[300]; unsigned i; for (i = 0; w[i] && i < 299; i++) b[i] = (char)w[i]; b[i] = 0; printf("%s %s\n", label, b); }
static const uint8_t *bytes_of(const char *path, uint64_t *size) { uint16_t *w = W(path); const uint8_t *data = NULL; unsigned opens = 0; uint64_t wt = 0; *size = 0; if (!fle_vfs_file_state(w, size, &opens, &wt, &data)) return NULL; return data; }
"#;

fn rows_table() -> String {
    let names = row_names(true);
    let mut s = String::from("const fle_resp_entry fle_resp_table[] = {\n");
    for (k, n) in names.iter().enumerate() {
        let f = format!("fle_imp_{}", n.replace("::", "_"));
        s.push_str(&format!("    {{ \"shell32.dll\", \"{n}\", UINT64_C({:#x}), {f} }},\n", 0x7F00_0000u64 + 16 * (k as u64 + 1)));
    }
    s.push_str(&format!("}};\nconst uint64_t fle_resp_count = {};\n", names.len()));
    s
}

/// Compiles `PRE + rows + globals + main` with the real volume and fle_filedlg.c, runs it with `envs`, returns (exit code, stdout lines).
fn run_env(name: &str, globals: &str, main: &str, envs: &[(&str, &str)]) -> (i32, Vec<String>) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    let src = format!("{PRE}{GUIDS_C}{rows}\n{globals}\nint main(void)\n{{\n{main}\n    return 0;\n}}\n", rows = rows_table());
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Wno-unused-function", "-Wno-unused-parameter", "-fPIE", "-pie"]);
    cmd.arg("-I").arg(rt()).arg(&c);
    for f in ["fle_vfs.c", "fle_reg.c", "fle_env.c", "fle_imports.c", "fle_modules.c", "fle_string.c", "fle_filedlg.c"] {
        cmd.arg(rt().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let mut run = Command::new(&exe);
    for (k, v) in envs {
        run.env(k, v);
    }
    let r = run.output().expect("run failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect())
}
fn run(name: &str, globals: &str, main: &str) -> (i32, Vec<String>) {
    run_env(name, globals, main, &[])
}

const OPEN: &str = r#"
    uint64_t d = 0;
    uint64_t hr = cocreate(CLSID_FOD, 0, IID_FOD, &d);
    if (hr != 0 || d == 0) { printf("cocreate failed %#llx\n", U(hr)); return 5; }
"#;
const SAVE: &str = r#"
    uint64_t d = 0;
    uint64_t hr = cocreate(CLSID_FSD, 0, IID_FSD, &d);
    if (hr != 0 || d == 0) { printf("cocreate failed %#llx\n", U(hr)); return 5; }
"#;

// ---- the C text of the "page" stubs ----
/// A page that answers a person's choice of one file (7 bytes "FLPDATA") plus a sibling sample (3 bytes "abc") in a subfolder; it prints the request it was given.
const PAGE_OPEN: &str = r#"
static int page_calls;
const char *fle_filedlg_host_pick(const char *req)
{
    uint8_t *d1 = malloc(7), *d2 = malloc(3);
    char *a = malloc(1024);
    page_calls++;
    printf("REQUEST %s\n", req);
    memcpy(d1, "FLPDATA", 7);
    memcpy(d2, "abc", 3);
    snprintf(a, 1024, "OK\nR\t%llu\t7\tC:\\Users\\user\\Downloads\\Sampler.flp\nE\t%llu\t3\tC:\\Users\\user\\Downloads\\proj\\a.wav\n", U(P(d1)), U(P(d2)));
    return a;
}
"#;

/// A page that answers with the text of the C string `ANS` (set by the test before Show), and counts its calls.
const PAGE_TEXT: &str = r#"
static int page_calls;
static const char *ans;
const char *fle_filedlg_host_pick(const char *req)
{
    page_calls++;
    (void)req;
    return ans == NULL ? NULL : strdup(ans);
}
"#;

#[test]
fn the_browser_branch_is_always_compiled_and_the_page_and_the_volume_are_weak_references_on_native() {
    // decision 0243 (old: the branch existed only with -DFLE_FILEDLG_BROWSER): a native link list that supplies no page function leaves fle_filedlg_host_pick null and the harness environment answers; the three volume
    // functions of the branch are weak too, so that a link list without fle_vfs.c (the tests of the model alone) still links
    let dir = work();
    let o = dir.join(format!("{}.o", etxt_common::uniq("fpick1-obj")));
    let cc = Command::new("clang").args(["-c", "-O0", "-std=gnu11", "-w", "-fPIE"]).arg("-I").arg(rt()).arg(rt().join("fle_filedlg.c")).arg("-o").arg(&o).env("TMPDIR", dir.join("tmp")).output().unwrap();
    assert!(cc.status.success(), "{}", String::from_utf8_lossy(&cc.stderr));
    let nm = String::from_utf8_lossy(&Command::new("nm").arg(&o).output().unwrap().stdout).to_string();
    assert!(nm.lines().any(|l| l.trim_end().ends_with(" w fle_filedlg_host_pick")), "{nm}");
    assert!(nm.lines().any(|l| l.contains(" T fle_filedlg_saved_poll")), "{nm}");
    for f in ["fle_vfs_make_dirs", "fle_vfs_put_file", "fle_vfs_file_state"] {
        assert!(nm.lines().any(|l| l.trim_end().ends_with(&format!(" w {f}"))), "{f} must be a weak reference\n{nm}");
    }
}

#[test]
fn the_page_answer_puts_the_chosen_file_and_its_sibling_on_the_volume_and_show_returns_the_file() {
    let main = format!(
        r#"{OPEN}
    static const uint16_t title[] = {{ 'O', 'p', 'e', 'n', ' ', '"', 0x3042, '\\', 0 }};
    uint16_t *n1 = W("FL Studio song file (*.flp)"), *s1 = W("*.flp"), *n2 = W("Zip (*.zip)"), *s2 = W("*.zip");
    uint64_t specs[4] = {{ P(n1), P(s1), P(n2), P(s2) }};
    printf("settypes %#llx\n", U(vcall(d, 4, 2, P(specs), 0)));
    printf("settitle %#llx\n", U(vcall(d, 17, P(title), 0, 0)));
    printf("setindex %#llx\n", U(vcall(d, 5, 2, 0, 0)));
    uint64_t item = 0, name = 0;
    printf("show %#llx calls %d\n", U(vcall(d, 3, 0, 0, 0)), page_calls);
    printf("getresult %#llx\n", U(vcall(d, 20, P(&item), 0, 0)));
    printf("filesyspath %#llx\n", U(vcall(item, 5, 0x80058000u, P(&name), 0)));
    show_path("  result", name);
    uint64_t sz; const uint8_t *b = bytes_of("C:\\Users\\user\\Downloads\\Sampler.flp", &sz);
    printf("flp %llu %.7s\n", U(sz), b ? (const char *)b : "(none)");
    b = bytes_of("C:\\Users\\user\\Downloads\\proj\\a.wav", &sz);
    printf("sibling %llu %.3s\n", U(sz), b ? (const char *)b : "(none)");
    uint64_t item2 = 0, cnt = 0, arr = 0;
    printf("getresults %#llx\n", U(vcall(d, 27, P(&arr), 0, 0)));
    printf("count %#llx %llu\n", U(vcall(arr, 7, P(&cnt), 0, 0)), U(cnt));
    (void)item2;
"#
    );
    let (rc, l) = run("fpick1-open", PAGE_OPEN, &main);
    assert_eq!(rc, 0, "{l:?}");
    // the request: the JSON the C side writes (UTF-16 units above 0x7e as \uXXXX, quote and backslash escaped), the filters in the order the engine set them, the type index the engine set, save 0
    let req = l.iter().find(|x| x.starts_with("REQUEST ")).expect("the page was not asked");
    assert!(req.contains("\"save\":0"), "{req}");
    assert!(req.contains("\"options\":"), "{req}");
    assert!(req.contains("\"filterIndex\":2"), "{req}");
    assert!(req.contains("\"title\":\"Open \\\"\\u3042\\\\\""), "{req}");
    assert!(req.contains("\"filters\":[[\"FL Studio song file (*.flp)\",\"*.flp\"],[\"Zip (*.zip)\",\"*.zip\"]]"), "{req}");
    assert!(req.contains("\"fileName\":null") && req.contains("\"folder\":null"), "{req}");
    let rest: Vec<&String> = l.iter().filter(|x| !x.starts_with("REQUEST ")).collect();
    assert_eq!(
        rest,
        [
            "settypes 0", "settitle 0", "setindex 0", "show 0 calls 1", "getresult 0", "filesyspath 0", "  result C:\\Users\\user\\Downloads\\Sampler.flp", "flp 7 FLPDATA", "sibling 3 abc", "getresults 0", "count 0 1"
        ]
    );
}

#[test]
fn the_environment_of_the_native_harness_still_wins_and_the_page_is_not_asked() {
    let main = format!(
        r#"{OPEN}
    {{ uint8_t *x = malloc(4); memcpy(x, "wxyz", 4); printf("put %d\n", fle_vfs_put_file(W("C:\\Users\\user\\Documents\\env.flp"), x, 4)); }}
    uint64_t item = 0, name = 0;
    printf("show %#llx calls %d\n", U(vcall(d, 3, 0, 0, 0)), page_calls);
    printf("getresult %#llx\n", U(vcall(d, 20, P(&item), 0, 0)));
    vcall(item, 5, 0x80058000u, P(&name), 0);
    show_path("  result", name);
"#
    );
    let (rc, l) = run_env("fpick1-env", PAGE_TEXT, &main, &[("FLE_FILEDLG_PICK", "C:\\Users\\user\\Documents\\env.flp")]);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["put 0", "show 0 calls 0", "getresult 0", "  result C:\\Users\\user\\Documents\\env.flp"]);
    // the flagged build with the variable unset asks the page (a cancel answer: the user cancelled), the default build never does
    let (rc, l) = run("fpick1-env-unset", PAGE_TEXT, &format!("{OPEN}    ans = \"CANCEL\";\n    printf(\"show %#llx calls %d\\n\", U(vcall(d, 3, 0, 0, 0)), page_calls);\n"));
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["show 0x800704c7 calls 1"]);
}

#[test]
fn a_save_target_is_made_with_its_folder_and_reported_once_when_the_engine_has_finished_writing_it() {
    let main = format!(
        r#"{SAVE}
    ans = "OK\nS\t0\t0\tC:\\Users\\user\\Documents\\Out.wav";
    uint16_t *ext = W("wav");
    vcall(d, 22, P(ext), 0, 0);
    uint64_t item = 0, name = 0;
    printf("show %#llx calls %d\n", U(vcall(d, 3, 0, 0, 0)), page_calls);
    vcall(d, 20, P(&item), 0, 0);
    vcall(item, 5, 0x80058000u, P(&name), 0);
    show_path("  target", name);
    printf("folder attrs %#llx\n", U(call(fle_imp_GetFileAttributesW, P(W("C:\\Users\\user\\Documents")), 0, 0, 0)));
    const char *p;
    p = fle_filedlg_saved_poll(); printf("poll0 %s\n", p ? p : "(null)");
    /* the engine opens the target and writes 5 bytes: an open handle is never reported */
    uint64_t h = call7(fle_imp_CreateFileW, P(W("C:\\Users\\user\\Documents\\Out.wav")), 0xC0000000u, 0, 0, 2, 0x80, 0);
    uint32_t wr = 0;
    call7(fle_imp_WriteFile, h, P("hello"), 5, P(&wr), 0, 0, 0);
    p = fle_filedlg_saved_poll(); printf("poll-open1 %s\n", p ? p : "(null)");
    p = fle_filedlg_saved_poll(); printf("poll-open2 %s\n", p ? p : "(null)");
    call(fle_imp_CloseHandle, h, 0, 0, 0);
    p = fle_filedlg_saved_poll(); printf("poll-closed1 %s\n", p ? p : "(null)");
    p = fle_filedlg_saved_poll();
    if (p) {{ unsigned long long sz, at; char path[200]; if (sscanf(p, "%llu\t%llu\t%199[^\n]", &sz, &at, path) == 3) printf("poll-closed2 size %llu bytes %.5s path %s\n", sz, (const char *)(uintptr_t)at, path); else printf("poll-closed2 unparsed %s\n", p); }} else printf("poll-closed2 (null)\n");
    p = fle_filedlg_saved_poll(); printf("poll-again %s\n", p ? p : "(null)");
    /* written again (6 bytes): reported again after two quiet polls */
    h = call7(fle_imp_CreateFileW, P(W("C:\\Users\\user\\Documents\\Out.wav")), 0xC0000000u, 0, 0, 2, 0x80, 0);
    call7(fle_imp_WriteFile, h, P("hello!"), 6, P(&wr), 0, 0, 0);
    call(fle_imp_CloseHandle, h, 0, 0, 0);
    p = fle_filedlg_saved_poll(); printf("poll-rewrite1 %s\n", p ? "reported" : "(null)");
    p = fle_filedlg_saved_poll(); printf("poll-rewrite2 %s\n", p ? p : "(null)");
"#
    );
    let (rc, l) = run("fpick1-save", PAGE_TEXT, &main);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l[0], "show 0 calls 1");
    assert_eq!(l[1], "  target C:\\Users\\user\\Documents\\Out.wav");
    assert_eq!(l[2], "folder attrs 0x10");   // FILE_ATTRIBUTE_DIRECTORY: the model made the folder, as FOS_PATHMUSTEXIST needs
    assert_eq!(&l[3..7], ["poll0 (null)", "poll-open1 (null)", "poll-open2 (null)", "poll-closed1 (null)"]);
    assert_eq!(l[7], "poll-closed2 size 5 bytes hello path C:\\Users\\user\\Documents\\Out.wav");
    assert_eq!(&l[8..9], ["poll-again (null)"]);
    assert!(l[9].starts_with("poll-rewrite1") && l[9].ends_with("(null)"), "{l:?}");
    assert!(l[10].starts_with("poll-rewrite2 6\t") && l[10].ends_with("\tC:\\Users\\user\\Documents\\Out.wav"), "{l:?}");
}

#[test]
fn answers_the_model_does_not_accept_cancel_the_dialog_and_never_stop_the_run() {
    // every answer is a person's input or a page's fault, not the guest's: the dialog is cancelled (0x800704C7) and nothing is put on the volume
    let cases: [(&str, &str); 9] = [
        ("cancel", "CANCEL"),
        ("empty", "OK"),
        ("unknown-role", "OK\\nX\\t0\\t0\\tC:\\\\a.flp"),
        ("no-tab", "OK\\nR\\t0\\tC:\\\\a.flp"),
        ("invalid-path-device", "OK\\nR\\t0\\t0\\tC:\\\\Users\\\\con"),
        ("other-drive", "OK\\nR\\t0\\t0\\tD:\\\\a.flp"),
        ("relative", "OK\\nR\\t0\\t0\\ta.flp"),
        ("too-large", "OK\\nR\\t0\\t268435457\\tC:\\\\Users\\\\user\\\\big.flp"),
        ("save-bad-folder", "OK\\nS\\t0\\t0\\tC:\\\\Users\\\\con\\\\x.wav"),
    ];
    for (tag, a) in cases {
        let dlg = if tag.starts_with("save") { SAVE } else { OPEN };
        let main = format!(
            r#"{dlg}
    ans = "{a}";
    uint64_t item = 0xAAAA;
    printf("show %#llx calls %d\n", U(vcall(d, 3, 0, 0, 0)), page_calls);
    printf("getresult %#llx item %#llx\n", U(vcall(d, 20, P(&item), 0, 0)), U(item));
    uint64_t sz; printf("big %d x %d\n", bytes_of("C:\\Users\\user\\big.flp", &sz) != NULL, bytes_of("C:\\a.flp", &sz) != NULL);
"#
        );
        let (rc, l) = run(&format!("fpick1-bad-{tag}"), PAGE_TEXT, &main);
        assert_eq!(rc, 0, "{tag}: {l:?}");
        assert_eq!(l, ["show 0x800704c7 calls 1", "getresult 0x8000ffff item 0", "big 0 x 0"], "{tag}");
    }
}

#[test]
fn the_volume_functions_put_replace_refuse_and_look() {
    let main = r#"
    uint8_t *a = malloc(4); memcpy(a, "1234", 4);
    printf("put %d\n", fle_vfs_put_file(W("C:\\p\\q\\a.bin"), a, 4));
    uint64_t sz; const uint8_t *b = bytes_of("C:\\p\\q\\a.bin", &sz);
    printf("a %llu %.4s\n", U(sz), (const char *)b);
    uint8_t *a2 = malloc(2); memcpy(a2, "ab", 2);
    printf("replace %d\n", fle_vfs_put_file(W("C:\\p\\q\\A.BIN"), a2, 2));   /* names compare without case */
    b = bytes_of("C:\\p\\q\\a.bin", &sz);
    printf("a %llu %.2s\n", U(sz), (const char *)b);
    printf("empty %d\n", fle_vfs_put_file(W("C:\\p\\e.bin"), NULL, 0));
    b = bytes_of("C:\\p\\e.bin", &sz);
    printf("e %llu %d\n", U(sz), b == NULL);
    printf("onto-dir %d\n", fle_vfs_put_file(W("C:\\p\\q"), NULL, 0));
    printf("through-file %d\n", fle_vfs_put_file(W("C:\\p\\q\\a.bin\\x"), NULL, 0));
    printf("makedirs-through-file %d\n", fle_vfs_make_dirs(W("C:\\p\\q\\a.bin\\x")));
    printf("makedirs %d again %d\n", fle_vfs_make_dirs(W("C:\\m\\n\\o")), fle_vfs_make_dirs(W("C:\\m\\n\\o")));
    printf("dir attrs %#llx\n", U(call(fle_imp_GetFileAttributesW, P(W("C:\\m\\n\\o")), 0, 0, 0)));
    printf("too-large %d\n", fle_vfs_put_file(W("C:\\p\\big.bin"), NULL, (UINT64_C(256) << 20) + 1));
    printf("limit-ok %d\n", fle_vfs_put_file(W("C:\\p\\lim.bin"), calloc(1, 1), 1));
    {
        const char *bad[] = { "", "C:", "C:\\", "D:\\a", "a\\b", "\\a", "C:a", "C:\\a\\", "C:\\a\\\\b", "C:\\a\\..\\b", "C:\\a\\.\\b", "C:\\a\\con", "C:\\a\\b?", "C:\\a\\b*", "C:\\a\\b:s", "C:\\a\\b.", "C:\\a\\b ", "C:\\a/b" };
        unsigned i, nbad = 0;
        for (i = 0; i < sizeof bad / sizeof bad[0]; i++) { int rc = fle_vfs_put_file(W(bad[i]), NULL, 0); if (rc != 1) { printf("not refused: %s -> %d\n", bad[i], rc); nbad++; } }
        printf("bad paths not refused: %u\n", nbad);
    }
    {
        /* open handle: the target cannot be replaced (3) */
        uint64_t h = call7(fle_imp_CreateFileW, P(W("C:\\p\\q\\a.bin")), 0x80000000u, 1, 0, 3, 0x80, 0);
        uint8_t *z = malloc(1);
        printf("open-handle %d\n", fle_vfs_put_file(W("C:\\p\\q\\a.bin"), z, 1));
        call(fle_imp_CloseHandle, h, 0, 0, 0);
    }
    {
        char longp[400]; unsigned i = 0;
        strcpy(longp, "C:\\");
        for (i = 3; i < 258; i++) longp[i] = 'a';
        longp[258] = 0;                      /* one component of 255: valid, total 258 units */
        printf("len258 %d\n", fle_vfs_put_file(W(longp), NULL, 0));
        longp[258] = 'b'; longp[259] = 'b'; longp[260] = 0;   /* 258 + 2: the component is 257 */
        printf("len260 %d\n", fle_vfs_put_file(W(longp), NULL, 0));
    }
"#;
    let (rc, l) = run("fpick1-vfs", PAGE_TEXT, main);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(
        l,
        [
            "put 0", "a 4 1234", "replace 0", "a 2 ab", "empty 0", "e 0 1", "onto-dir 2", "through-file 5", "makedirs-through-file 5", "makedirs 0 again 0", "dir attrs 0x10", "too-large 4", "limit-ok 0", "bad paths not refused: 0",
            "open-handle 3", "len258 0", "len260 1"
        ]
    );
}
