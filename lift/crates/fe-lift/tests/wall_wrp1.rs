//! Tests of decision 0183 (docs/0183-fruity-wrapper.md, PROVISIONAL): the runtime pieces that the Fruity Wrapper needed inside the engine.
//!
//!  1. `fle_realmod_find_path` (runtime/fle_realmod.c): a module requested by a PATH is a real module when the link's path table has an entry whose suffix is the tail of the path (ASCII case and '/' folded; the
//!     character before the tail is a separator or there is none). The expectations are the rules written in fle_realmod.h (Learn LoadLibraryW: "If the string specifies a full path, the function searches only that path").
//!  2. Job objects (runtime/fle_wall_wrp1.c): CreateJobObjectW (unnamed, named, "Name comparison is case-sensitive", ERROR_ALREADY_EXISTS for an existing name, "destroyed when its last handle has been closed"),
//!     SetInformationJobObject (class 9 records the limits; an invalid handle fails with ERROR_INVALID_HANDLE; other classes stop by name). Expected values from the Learn pages named there.
//!
//! The C programs call the `fle_imp_*` functions the way the lifted code reaches them: arguments in R[1], R[2], R[8], R[9], the result in R[0]. EVERY ANSWER IS PROVISIONAL (nothing is compared with a real Windows). FLE_WRP1_RT_DIR (a full copy of runtime/) replaces runtime/ for the mutation runs.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_WRP1_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-wrp1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn run(name: &str, files: &[&str], src: &str) -> (i32, Vec<String>) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Wno-unused-function", "-Wno-unused-parameter", "-fPIE", "-pie"]).arg("-I").arg(rt()).arg(&c);
    for f in files {
        cmd.arg(rt().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect())
}

const REALMOD_PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_realmod.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
uint64_t fle_call_guest(uint64_t va, uint64_t a, uint64_t b, uint64_t c, uint64_t d) { (void)va; (void)a; (void)b; (void)c; (void)d; return 0; }
uint64_t fle_module_count(void) { return 0; }
const char *fle_module_name(uint64_t i) { (void)i; return ""; }
int fle_module_is_deferred(uint64_t i) { (void)i; return 0; }
const fle_realmod_entry fle_realmod_table[] = { { "dsp_ippv2_x64.dll", UINT64_C(0x2000000) }, { "fruity wrapper_x64.dll", UINT64_C(0x7000000) }, { "fruity wrapper effects_x64.dll", UINT64_C(0x9000000) }, { "serum2.vst3", UINT64_C(0x8000000) } };
const uint64_t fle_realmod_count = 4;
const fle_realmod_path_entry fle_realmod_path_table[] = {
    { "plugins\\fruity\\generators\\fruity wrapper\\fruity wrapper_x64.dll", "fruity wrapper_x64.dll" },
    { "plugins\\fruity\\effects\\fruity wrapper\\fruity wrapper_x64.dll", "fruity wrapper effects_x64.dll" },
    { "serum2.vst3\\contents\\x86_64-win\\serum2.vst3", "serum2.vst3" } };
const uint64_t fle_realmod_path_count = 3;
static uint16_t *W(const char *s) { size_t n = strlen(s), i; uint16_t *d = malloc((n + 1) * 2); for (i = 0; i <= n; i++) d[i] = (unsigned char)s[i]; return d; }
static int find(const char *s) { uint16_t *w = W(s); int r = fle_realmod_find_path(w, strlen(s)); free(w); return r; }
"#;

#[test]
fn a_path_is_matched_by_its_tail_with_case_and_slashes_folded() {
    let (rc, out) = run(
        "path_tail",
        &["fle_realmod.c"],
        &format!(
            r#"{REALMOD_PRE}
int main(void) {{
    printf("gen %d\n", find("C:\\Program Files\\Image-Line\\FL Studio 21\\Plugins\\Fruity\\Generators\\Fruity Wrapper\\Fruity Wrapper_x64.dll"));
    printf("eff %d\n", find("C:\\Program Files\\Image-Line\\FL Studio 21\\Plugins\\Fruity\\Effects\\Fruity Wrapper\\Fruity Wrapper_x64.dll"));
    printf("slash %d\n", find("c:/program files/image-line/fl studio 21/plugins/fruity/generators/fruity wrapper/fruity wrapper_x64.dll"));
    printf("whole %d\n", find("Plugins\\Fruity\\Generators\\Fruity Wrapper\\Fruity Wrapper_x64.dll"));
    printf("vst3 %d\n", find("C:\\Program Files\\Common Files\\VST3\\Serum2.vst3\\Contents\\x86_64-win\\Serum2.vst3"));
    return 0;
}}"#
        ),
    );
    assert_eq!(rc, 0, "{out:?}");
    assert_eq!(out, ["gen 1", "eff 2", "slash 1", "whole 1", "vst3 3"]);
}

#[test]
fn a_tail_inside_a_component_a_foreign_directory_or_a_non_ascii_unit_does_not_match() {
    let (rc, out) = run(
        "path_no_match",
        &["fle_realmod.c"],
        &format!(
            r#"{REALMOD_PRE}
int main(void) {{
    uint16_t u[8] = {{ 0x30aa, 'a', 0 }};
    printf("mid %d\n", find("C:\\xplugins\\fruity\\generators\\fruity wrapper\\fruity wrapper_x64.dll"));
    printf("other %d\n", find("C:\\Program Files\\Plugins\\Fruity\\Generators\\Other\\Fruity Wrapper_x64.dll"));
    printf("name %d\n", find("Fruity Wrapper_x64.dll"));
    printf("empty %d\n", find(""));
    printf("kana %d\n", fle_realmod_find_path(u, 2));
    printf("bare %d\n", fle_realmod_find_path(0, 0));
    {{ const char *t = "serum2.vst3\\contents\\x86_64-win\\serum2.vst3"; uint16_t *w = W(t); w[0] = 0x0173; printf("trunc %d\n", fle_realmod_find_path(w, strlen(t))); }}
    return 0;
}}"#
        ),
    );
    assert_eq!(rc, 0, "{out:?}");
    assert_eq!(out, ["mid -1", "other -1", "name -1", "empty -1", "kana -1", "bare -1", "trunc -1"]);
}

const JOB_PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_vfs.h"
#include "fle_wall_wrp1.h"
uint64_t R[16];
#define U(x) ((unsigned long long)(x))
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
static uint32_t last_error;
void fle_set_last_error(uint32_t c) { last_error = c; }
static int (*close_hook)(uint64_t);
void fle_vfs_add_close_hook(int (*h)(uint64_t)) { close_hook = h; }
size_t fle_w_len(const uint16_t *s) { size_t n = 0; while (s[n]) n++; return n; }
uint16_t *fle_w_read_alloc(const char *api, uint64_t g, size_t max, size_t *len) { const uint16_t *s = (const uint16_t *)(uintptr_t)g; size_t n = fle_w_len(s); uint16_t *d; if (n > max) fle_stop(api, "too long"); d = malloc((n + 1) * 2); memcpy(d, s, (n + 1) * 2); *len = n; return d; }
static uint16_t *W(const char *s) { size_t n = strlen(s), i; uint16_t *d = malloc((n + 1) * 2); for (i = 0; i <= n; i++) d[i] = (unsigned char)s[i]; return d; }
static uint64_t P(const void *p) { return (uint64_t)(uintptr_t)p; }
static uint64_t create(uint64_t attrs, const char *name) { uint16_t *w = name ? W(name) : 0; last_error = 0; R[1] = attrs; R[2] = w ? P(w) : 0; fle_imp_CreateJobObjectW(); return R[0]; }
static uint64_t setinfo(uint64_t h, uint32_t cls, uint64_t info, uint32_t cb) { R[1] = h; R[2] = cls; R[8] = info; R[9] = cb; fle_imp_SetInformationJobObject(); return R[0]; }
static uint64_t closeh(uint64_t h) { return close_hook(h) ? R[0] : 99; }
"#;

#[test]
fn unnamed_jobs_get_distinct_handles_in_the_job_region_and_close_with_the_last_handle() {
    let (rc, out) = run(
        "job_unnamed",
        &["fle_wall_wrp1.c"],
        &format!(
            r#"{JOB_PRE}
int main(void) {{
    uint64_t a = create(0, 0), b = create(0, 0);
    printf("in-region %d %d\n", a > FLE_JOB_HANDLE_BASE && a < FLE_JOB_HANDLE_BASE + FLE_JOB_REGION_SIZE, b > FLE_JOB_HANDLE_BASE);
    printf("distinct %d\n", a != b);
    printf("last-error-untouched %u\n", last_error);
    printf("live %u\n", fle_job_live());
    printf("close-a %llu live %u\n", U(closeh(a)), fle_job_live());
    printf("close-a-again %llu err %u\n", U(closeh(a)), last_error);
    printf("close-b %llu live %u\n", U(closeh(b)), fle_job_live());
    return 0;
}}"#
        ),
    );
    assert_eq!(rc, 0, "{out:?}");
    assert_eq!(out, ["in-region 1 1", "distinct 1", "last-error-untouched 0", "live 2", "close-a 1 live 1", "close-a-again 0 err 6", "close-b 1 live 0"]);
}

#[test]
fn a_named_job_that_exists_gives_a_second_handle_and_error_already_exists_and_names_are_case_sensitive() {
    let (rc, out) = run(
        "job_named",
        &["fle_wall_wrp1.c"],
        &format!(
            r#"{JOB_PRE}
int main(void) {{
    uint64_t a = create(0, "C:/Program Files/FL64.exe");
    uint32_t e1 = last_error;
    uint64_t b = create(0, "C:/Program Files/FL64.exe");
    uint32_t e2 = last_error;
    uint64_t c = create(0, "c:/program files/fl64.exe");
    printf("first err %u\n", e1);
    printf("second err %u distinct-handle %d\n", e2, a != b);
    printf("case-sensitive live %u\n", fle_job_live());
    (void)c;
    printf("close-a live %llu %u\n", U(closeh(a)), fle_job_live());
    printf("close-b live %llu %u\n", U(closeh(b)), fle_job_live());
    return 0;
}}"#
        ),
    );
    assert_eq!(rc, 0, "{out:?}");
    assert_eq!(out, ["first err 0", "second err 183 distinct-handle 1", "case-sensitive live 2", "close-a live 1 2", "close-b live 1 1"]);
}

#[test]
fn set_information_job_object_records_class_9_and_fails_on_a_bad_handle() {
    let (rc, out) = run(
        "job_setinfo",
        &["fle_wall_wrp1.c"],
        &format!(
            r#"{JOB_PRE}
int main(void) {{
    uint8_t info[0x6c]; uint64_t h = create(0, 0);
    memset(info, 0, sizeof info); info[0x10] = 0x00; info[0x11] = 0x20;
    printf("ok %llu\n", U(setinfo(h, 9, P(info), 0x6c)));
    printf("null %llu err %u\n", U(setinfo(0, 9, P(info), 0x6c)), last_error);
    closeh(h);
    printf("closed %llu err %u\n", U(setinfo(h, 9, P(info), 0x6c)), last_error);
    return 0;
}}"#
        ),
    );
    assert_eq!(rc, 0, "{out:?}");
    assert_eq!(out, ["ok 1", "null 0 err 6", "closed 0 err 6"]);
}

#[test]
fn security_attributes_other_classes_long_names_and_foreign_handles_stop_by_name() {
    for (name, body, what) in [
        ("job_attrs", "create(0x1000, 0);", "STOP CreateJobObjectW: lpJobAttributes is not NULL"),
        ("job_class", "uint8_t info[0x40] = {0}; uint64_t h = create(0, 0); setinfo(h, 2, P(info), 0x40);", "STOP SetInformationJobObject: only JobObjectExtendedLimitInformation (9) is modelled"),
        ("job_short", "uint8_t info[8] = {0}; uint64_t h = create(0, 0); setinfo(h, 9, P(info), 8);", "STOP SetInformationJobObject: the information buffer does not reach LimitFlags"),
        ("job_foreign", "setinfo(0x1234, 9, 0, 0);", "STOP SetInformationJobObject: handle was not issued as a job object by this runtime"),
        ("job_long", "char n[300]; memset(n, 'a', 261); n[261] = 0; create(0, n);", "STOP CreateJobObjectW: a name longer than MAX_PATH"),
    ] {
        let (rc, out) = run(name, &["fle_wall_wrp1.c"], &format!("{JOB_PRE}\nint main(void) {{ {body} return 0; }}"));
        assert_eq!(rc, 3, "{name}: {out:?}");
        assert!(out[0].starts_with(what), "{name}: {out:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// The MD5-only CryptoAPI (runtime/fle_crypt.c). The expected digests are the seven test vectors of RFC 1321 appendix A.5; the API behaviour is Learn's (CryptAcquireContext, CryptCreateHash, CryptHashData, CryptGetHashParam).

const CRYPT_PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_crypt.h"
uint64_t R[16];
#define U(x) ((unsigned long long)(x))
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
static uint32_t last_error;
void fle_set_last_error(uint32_t c) { last_error = c; }
/* a guest stack: [rsp] = return address, [rsp+0x28] the fifth argument */
static uint64_t stk[16];
static uint64_t P(const void *p) { return (uint64_t)(uintptr_t)p; }
static void setup(void) { R[4] = P(stk); stk[0] = 0x1234; }
static uint64_t acquire(uint64_t *ph, uint32_t type, uint32_t flags) { setup(); last_error = 0; stk[5] = flags; R[1] = P(ph); R[2] = 0; R[8] = 0; R[9] = type; fle_imp_CryptAcquireContextA(); return R[0]; }
static uint64_t create(uint64_t prov, uint32_t alg, uint64_t *ph) { setup(); last_error = 0; stk[5] = P(ph); R[1] = prov; R[2] = alg; R[8] = 0; R[9] = 0; fle_imp_CryptCreateHash(); return R[0]; }
static uint64_t hashdata(uint64_t h, const void *d, uint32_t n) { setup(); last_error = 0; R[1] = h; R[2] = P(d); R[8] = n; R[9] = 0; fle_imp_CryptHashData(); return R[0]; }
static uint64_t getparam(uint64_t h, uint32_t prm, void *buf, uint32_t *len) { setup(); last_error = 0; stk[5] = 0; R[1] = h; R[2] = prm; R[8] = P(buf); R[9] = P(len); fle_imp_CryptGetHashParam(); return R[0]; }
static uint64_t destroy(uint64_t h) { setup(); last_error = 0; R[1] = h; fle_imp_CryptDestroyHash(); return R[0]; }
static uint64_t release(uint64_t p) { setup(); last_error = 0; R[1] = p; R[2] = 0; fle_imp_CryptReleaseContext(); return R[0]; }
static void hex(const uint8_t *d, int n, char *o) { int i; for (i = 0; i < n; i++) sprintf(o + 2 * i, "%02x", d[i]); }
static void md5_via_api(const char *s, char *out)
{
    uint64_t prov = 0, h = 0; uint8_t v[16]; uint32_t len = 16;
    acquire(&prov, 1, 0xF0000000u); create(prov, 0x8003, &h); hashdata(h, s, (uint32_t)strlen(s)); getparam(h, 2, v, &len); hex(v, 16, out); destroy(h); release(prov);
}
"#;

#[test]
fn md5_through_the_cryptoapi_gives_the_seven_rfc_1321_vectors() {
    let (rc, out) = run(
        "crypt_rfc",
        &["fle_crypt.c"],
        &format!(
            r#"{CRYPT_PRE}
int main(void) {{
    static const char *in[7] = {{ "", "a", "abc", "message digest", "abcdefghijklmnopqrstuvwxyz",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        "12345678901234567890123456789012345678901234567890123456789012345678901234567890" }};
    int i;
    for (i = 0; i < 7; i++) {{ char o[40]; md5_via_api(in[i], o); printf("%s\n", o); }}
    printf("live %u %u\n", fle_crypt_live_providers(), fle_crypt_live_hashes());
    return 0;
}}"#
        ),
    );
    assert_eq!(rc, 0, "{out:?}");
    assert_eq!(
        out,
        [
            "d41d8cd98f00b204e9800998ecf8427e",
            "0cc175b9c0f1b6a831c399e269772661",
            "900150983cd24fb0d6963f7d28e17f72",
            "f96b697d7cb7938d525a2f31aaf161d0",
            "c3fcd3d76192e4007dfb496cca67e13b",
            "d174ab98d277d9f5a5611c2c9f419d9f",
            "57edf4a22be3c955ac49da2e2107b67a",
            "live 0 0"
        ]
    );
}

#[test]
fn hashing_in_pieces_and_across_the_block_boundaries_equals_hashing_at_once() {
    // 55, 56, 63, 64, 65, 119, 120 bytes straddle the padding cases of RFC 1321 section 3.1; the digest of the whole is the digest of the pieces
    let (rc, out) = run(
        "crypt_pieces",
        &["fle_crypt.c"],
        &format!(
            r#"{CRYPT_PRE}
int main(void) {{
    static const int sizes[] = {{ 55, 56, 57, 63, 64, 65, 119, 120, 121, 1000 }};
    unsigned k;
    for (k = 0; k < sizeof sizes / sizeof sizes[0]; k++) {{
        char buf[1100]; uint8_t whole[16], piece[16]; uint64_t prov = 0, h = 0; uint32_t len = 16; int n = sizes[k], i;
        for (i = 0; i < n; i++) buf[i] = (char)('A' + (i * 7) % 26);
        fle_md5((const uint8_t *)buf, (size_t)n, whole);
        acquire(&prov, 24, 0xF0000000u); create(prov, 0x8003, &h);
        for (i = 0; i < n; i += 13) hashdata(h, buf + i, (uint32_t)(n - i < 13 ? n - i : 13));
        getparam(h, 2, piece, &len);
        {{ char a[40], b[40]; hex(whole, 16, a); hex(piece, 16, b); printf("%d %s\n", n, strcmp(a, b) == 0 ? "same" : "DIFFERENT"); }}
    }}
    return 0;
}}"#
        ),
    );
    assert_eq!(rc, 0, "{out:?}");
    assert_eq!(out, ["55 same", "56 same", "57 same", "63 same", "64 same", "65 same", "119 same", "120 same", "121 same", "1000 same"]);
}

#[test]
fn hash_params_size_buffers_and_states_follow_the_learn_page() {
    let (rc, out) = run(
        "crypt_params",
        &["fle_crypt.c"],
        &format!(
            r#"{CRYPT_PRE}
int main(void) {{
    uint64_t prov = 0, h = 0; uint32_t len, dw = 0; uint8_t v[16];
    acquire(&prov, 1, 0xF0000000u); create(prov, 0x8003, &h);
    hashdata(h, "abc", 3);
    len = 0; printf("size-query %llu len %u\n", U(getparam(h, 2, 0, &len)), len);
    len = 4; printf("small %llu len %u err %u\n", U(getparam(h, 2, v, &len)), len, last_error);
    len = 4; printf("hashsize %llu %u len %u\n", U(getparam(h, 4, &dw, &len)), dw, len);
    len = 4; printf("algid %llu %#x len %u\n", U(getparam(h, 1, &dw, &len)), dw, len);
    len = 16; getparam(h, 2, v, &len);
    printf("add-after-value %llu err %#x\n", U(hashdata(h, "d", 1)), last_error);
    len = 16; getparam(h, 2, v, &len);
    {{ char o[40]; hex(v, 16, o); printf("again %s\n", o); }}
    printf("destroy %llu\n", U(destroy(h)));
    printf("destroy-again %llu err %#x\n", U(destroy(h)), last_error);
    printf("hash-on-released %llu err %#x\n", U((release(prov), create(prov, 0x8003, &h))), last_error);
    printf("release-again %llu err %#x\n", U(release(prov)), last_error);
    return 0;
}}"#
        ),
    );
    assert_eq!(rc, 0, "{out:?}");
    assert_eq!(
        out,
        [
            "size-query 1 len 16",
            "small 0 len 16 err 234",
            "hashsize 1 16 len 4",
            "algid 1 0x8003 len 4",
            "add-after-value 0 err 0x8009000c",
            "again 900150983cd24fb0d6963f7d28e17f72",
            "destroy 1",
            "destroy-again 0 err 0x80090002",
            "hash-on-released 0 err 0x80090001",
            "release-again 0 err 0x80090001"
        ]
    );
}

#[test]
fn anything_outside_md5_with_verifycontext_stops_by_name() {
    for (name, body, what) in [
        ("crypt_alg", "uint64_t prov = 0, h = 0; acquire(&prov, 1, 0xF0000000u); create(prov, 0x8004, &h);", "STOP CryptCreateHash: Algid is not CALG_MD5"),
        ("crypt_flags", "uint64_t prov = 0; acquire(&prov, 1, 0);", "STOP CryptAcquireContextA: dwFlags is not exactly CRYPT_VERIFYCONTEXT"),
        ("crypt_type", "uint64_t prov = 0; acquire(&prov, 3, 0xF0000000u);", "STOP CryptAcquireContextA: dwProvType is not PROV_RSA_FULL"),
        ("crypt_param", "uint64_t prov = 0, h = 0; uint32_t len = 16; uint8_t v[16]; acquire(&prov, 1, 0xF0000000u); create(prov, 0x8003, &h); getparam(h, 7, v, &len);", "STOP CryptGetHashParam: dwParam is none of"),
        ("crypt_foreign", "destroy(0x1234);", "STOP CryptDestroyHash: handle was not issued"),
    ] {
        let (rc, out) = run(name, &["fle_crypt.c"], &format!("{CRYPT_PRE}\nint main(void) {{ {body} return 0; }}"));
        assert_eq!(rc, 3, "{name}: {out:?}");
        assert!(out[0].starts_with(what), "{name}: {out:?}");
    }
}
