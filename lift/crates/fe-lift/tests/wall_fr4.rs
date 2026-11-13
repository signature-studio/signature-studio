//! Tests of decision 0117 (docs/decisions/0117-winverifytrust-and-walls.md): wintrust!WinVerifyTrust and the crypt32 calls behind the engine's signer-name check,
//! answered from a table of CLAIMS that the harness registers (harness/authenticode_claims.h, harness/seeds/authenticode-claims.tsv, written by
//! tools/authenticode_check.py); kernel32!GetUserPreferredUILanguages; and the wall behind them (the engine's CPU check, which the answer of CPUID decides).
//!
//! Expected values are written from the Microsoft Learn pages named in the decision (WinVerifyTrust, WINTRUST_DATA, WINTRUST_FILE_INFO, CryptQueryObject,
//! CryptMsgGetParam, CMSG_SIGNER_INFO, CERT_INFO, CERT_CONTEXT, CertFindCertificateInStore, CertGetNameStringW, CertFreeCertificateContext, CertCloseStore,
//! CryptMsgClose, GetUserPreferredUILanguages), from the format description of harness/authenticode_claims.h, and from decisions 0015, 0018, 0116; not read back from
//! the C code. EVERY ANSWER IS PROVISIONAL: nothing is compared with a real Windows. WinVerifyTrust answers 0 only for a path that has an ESTABLISHED claim, and a claim
//! says that the digest and the signature value matched the embedded signer certificate; the certificate chain, revocation and time were not verified.
//!
//! 1. Generator: the new rows are the last rows, in the order they were reached; the thunks of wintrust.dll and crypt32.dll are answered.
//! 2. Runtime, through a driver linked with every runtime source and the generated C of a synthetic PE (the table of claims is filled by the driver).
//! 3. The harness: the self-test of authenticode_claims.h, the rules of a4-multi.sh for AUTH_CLAIMS, the guarantee that a build without FLE_FRUITY contains no claims
//!    registration.
//! 4. The bytes of the real FLEngine_x64.dll that the decision rests on (skipped when the DLL is absent): the calls of CreateFruityInstance, the CPU check and its cause.

mod etxt_common;
mod rows_0132;
mod rows_0139;
mod rows_0140;
mod rows_0144;
mod rows_0147;
mod rows_0267;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn harness_dir() -> PathBuf {
    manifest().join("../../harness")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR4/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// A synthetic PE that imports (dll, name) pairs, one thunk each (the same shape as tests/wall_fr3.rs)
// ---------------------------------------------------------------------------------------------

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

fn build_imports(imps: &[(&str, &str)]) -> Vec<u8> {
    assert!(imps.len() <= 6);
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
    w32(&mut b, o + 124, 20 * (imps.len() as u32 + 1));

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
    for (k, (dll, name)) in imps.iter().enumerate() {
        let k32 = k as u32;
        let (oft, ft, dll_rva, hn_rva) = (0x2100 + 0x10 * k32, 0x2200 + 0x10 * k32, 0x2300 + 0x20 * k32, 0x2500 + 0x40 * k32);
        w32(&mut b, d + 20 * k, oft);
        w32(&mut b, d + 20 * k + 12, dll_rva);
        w32(&mut b, d + 20 * k + 16, ft);
        put(&mut b, dll_rva, format!("{dll}\0").as_bytes());
        put(&mut b, hn_rva, &[&[0u8, 0u8][..], name.as_bytes(), &[0u8]].concat());
        w64(&mut b, data_at(oft), hn_rva as u64);
        w64(&mut b, data_at(ft), hn_rva as u64);
    }

    let t = TEXT_RAW;
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    for k in 0..imps.len() as u32 {
        let at = 0x1000 + 5 * k;
        code.push((at, [&[0xe8][..], &rel(at + 5, 0x1040 + 0x10 * k)].concat()));
        code.push((0x1040 + 0x10 * k, [&[0xff, 0x25][..], &rel(0x1046 + 0x10 * k, 0x2200 + 0x10 * k)].concat()));
    }
    code.push((0x1000 + 5 * imps.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn gen(imps: &[(&str, &str)]) -> (fe_lift::Stats, String) {
    let img = build_imports(imps);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

const GUEST_IMPORTS: [(&str, &str); 2] = [("kernel32.dll", "GetProcAddress"), ("kernel32.dll", "LoadLibraryW")];

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

fn resp_rows(c: &str) -> Vec<(String, String, u64)> {
    let start = c.find("const fle_resp_entry fle_resp_table[] = {").expect("no response table");
    let end = start + c[start..].find("};").unwrap();
    c[start..end]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let l = l.strip_prefix("{ \"")?;
            let (dll, rest) = l.split_once("\", \"")?;
            let (name, rest) = rest.split_once("\", UINT64_C(0x")?;
            let (va, _) = rest.split_once(')')?;
            Some((dll.to_string(), name.to_string(), u64::from_str_radix(va, 16).unwrap()))
        })
        .collect()
}

const NEW_ROWS: [(&str, &str); 9] = [
    ("wintrust.dll", "WinVerifyTrust"),
    ("crypt32.dll", "CryptQueryObject"),
    ("crypt32.dll", "CryptMsgGetParam"),
    ("crypt32.dll", "CertFindCertificateInStore"),
    ("crypt32.dll", "CertGetNameStringW"),
    ("crypt32.dll", "CertFreeCertificateContext"),
    ("crypt32.dll", "CertCloseStore"),
    ("crypt32.dll", "CryptMsgClose"),
    ("kernel32.dll", "GetUserPreferredUILanguages"),
];

#[test]
fn the_nine_new_rows_are_the_last_nine_in_the_order_they_were_reached_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let n = rows.len();
    // decision 0015: append only. The rows of decision 0116 (the last ones before this decision) keep their places; the nine new rows follow them, and the rows of decision 0118
    // (tests/wall_fr5.rs) were appended after those: LATER_ROWS is how many there are now (1 until decision 0119, which appended 8 more: tests/wall_fr6.rs; decision 0122 appended 15 more:
    // tests/wall_fr8.rs; decision 0123 appended 73 more: tests/wall_fr9.rs; decision 0125 appended 2 more: tests/wall_fr10.rs; old value 9 + 15 + 73).
    // decision 0126 appended 33 more (tests/wall_fr11.rs; old value 9 + 15 + 73 + 2: it ended at decision 0125's rows).
    const LATER_ROWS: usize = 9 + 15 + 73 + 2 + 33 + 4 + rows_0132::ROWS_0132.len() + 10 + rows_0139::ROWS_0139.len() + rows_0140::ROWS_0140.len() + rows_0144::ROWS_0144.len() + rows_0147::ROWS_0147.len() + rows_0267::ROWS_0267.len(); // decision 0267: + the row of tests/rows_0267/mod.rs after that of decision 0147; decision 0147: + the row of tests/rows_0147/mod.rs after those of decision 0144; decision 0136: + the 10 rows of tests/wall_fr14.rs, appended after those of decision 0132; decision 0139: + tests/rows_0139/mod.rs after those
    // decision 0129 (old: ... + 33): + 4 rows (tests/wall_fr12.rs); decision 0132: + tests/rows_0132/mod.rs
    assert_eq!((rows[n - LATER_ROWS - 10].0.as_str(), rows[n - LATER_ROWS - 10].1.as_str()), ("shell32.dll", "SHGetPathFromIDListW"));
    for (k, (dll, name)) in NEW_ROWS.iter().enumerate() {
        assert_eq!((rows[n - LATER_ROWS - 9 + k].0.as_str(), rows[n - LATER_ROWS - 9 + k].1.as_str()), (*dll, *name), "row {}", n - LATER_ROWS - 9 + k);
    }
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}");
    }
    let at = |dll: &str, name: &str| rows.iter().position(|(d, nm, _)| (d.as_str(), nm.as_str()) == (dll, name)).unwrap_or_else(|| panic!("no row {dll}!{name}"));
    // the rows of decision 0116 keep their places relative to the 0112 rows (6 rows between GetComputerNameExW and GetComputerNameExA)
    assert_eq!(at("kernel32.dll", "GetComputerNameExA"), at("kernel32.dll", "GetComputerNameExW") + 6);
    assert_eq!(at("shell32.dll", "SHGetPathFromIDListW"), at("kernel32.dll", "GetComputerNameExA") + 2);
    assert_eq!(rows.iter().filter(|(d, _, _)| d == "wintrust.dll").count(), 1);
    assert_eq!(rows.iter().filter(|(d, _, _)| d == "crypt32.dll").count(), 7);
    for (_, name) in NEW_ROWS {
        assert!(c.contains(&format!("void fle_imp_{name}(void);")), "prototype of {name}");
    }
}

#[test]
fn the_wintrust_and_crypt32_thunks_are_answered_under_their_own_dll_and_no_other() {
    // FLEngine imports wintrust.dll!WinVerifyTrust (thunk 0x4256d0) and crypt32.dll!CryptQueryObject ... CryptMsgClose (thunks 0x1375030..0x13750e0). The DLL name is
    // compared without case; another DLL's function of the same name is another API.
    let (st, c) = gen(&[
        ("WINTRUST.dll", "WinVerifyTrust"),
        ("Crypt32.DLL", "CertGetNameStringW"),
        ("user32.dll", "WinVerifyTrust"),
        ("crypt32.dll", "CertGetNameStringA"),
        ("kernel32.dll", "GetProcAddress"),
    ]);
    assert_eq!(st.handled["jmp_imp:WinVerifyTrust"], 1);
    assert_eq!(st.handled["jmp_imp:CertGetNameStringW"], 1);
    assert_eq!(c.matches("fle_imp_WinVerifyTrust(); return;").count(), 1, "only the wintrust thunk is answered");
    assert_eq!(c.matches("fle_imp_CertGetNameStringW(); return;").count(), 1);
    for k in [2u64, 3] {
        let va = IB + 0x1040 + 0x10 * k;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "thunk {k} (user32! / the ANSI form) stays unsupported");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. Runtime
// ---------------------------------------------------------------------------------------------

const DRIVER: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_imports.h"
#include "fle_load.h"
#include "fle_locale.h"
#include "fle_env.h"
#include "fle_vfs.h"
#include "fle_uipref.h"

static uint64_t stk[32];
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
#define M(n) (0x7FF800000000ull + 0x10000ull * (n))
#define N_KERNEL32 9

static void prep(uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, const uint64_t *more, int nmore)
{
    int i;
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    for (i = 0; i < nmore; i++) stk[5 + i] = more[i];
    R[4] = A(stk);
    R[1] = a1; R[2] = a2; R[8] = a3; R[9] = a4;
    R[0] = 0xBADBADBADull;
    fle_set_last_error(5);
}
static uint64_t le(void) { uint64_t r = R[0], e; fle_imp_GetLastError(); e = R[0]; R[0] = r; return e; }

static const char *FLE_PATH = "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll";
static const char *EXE_PATH = "C:\\Program Files\\Image-Line\\FL Studio 21\\FL64.exe";
static const uint8_t ISSUER[] = { 0x30,0x0f,0x31,0x0d,0x30,0x0b,0x06,0x03,0x55,0x04,0x03,0x0c,0x04,'T','e','s','t' };
static const uint8_t SERIAL_BE[] = { 0x0a,0xfc,0x3a,0xad,0x2f,0xa7,0xd5,0xb5,0xe9,0xc2,0x67,0xed,0x4a,0x67,0xeb,0x12 };

static uint16_t *wide(const char *s)
{
    size_t n = strlen(s), i;
    uint16_t *w = calloc(n + 1, 2);
    for (i = 0; i < n; i++) w[i] = (uint8_t)s[i];
    return w;
}

static void add_claim(const char *path, const char *row, int established, const char *why, const char *cn)
{
    fle_trust_claim c;
    memset(&c, 0, sizeof c);
    c.guest_path = path; c.row = row; c.established = established; c.why = why; c.signer_cn = cn;
    c.issuer_der = ISSUER; c.issuer_len = sizeof ISSUER; c.serial_be = SERIAL_BE; c.serial_len = sizeof SERIAL_BE;
    if (fle_trust_claim_add(&c) != 0) { printf("claim refused\n"); exit(3); }
}

/* claim sets: none | good (the DLL, established) | both (the DLL and the exe, established) | bad (the DLL, not established) | mixed (DLL established, exe not) */
static void setup(const char *kind)
{
    if (!strcmp(kind, "none")) return;
    if (!strcmp(kind, "bad")) { add_claim(FLE_PATH, "FLEngine_x64.dll", 0, "b_signature is ng (the signature does not verify)", "Image Line NV"); return; }
    add_claim(FLE_PATH, "FLEngine_x64.dll", 1, "", "Image Line NV");
    if (!strcmp(kind, "both")) add_claim(EXE_PATH, "FL64.exe", 1, "", "Image Line NV");
    if (!strcmp(kind, "mixed")) add_claim(EXE_PATH, "FL64.exe", 0, "a_digest is ng", "Image Line NV");
}

/* WINTRUST_DATA (88 bytes) and WINTRUST_FILE_INFO (32 bytes) in host memory, which is the guest memory of the native harness */
static uint8_t wvt[88], fi[32];
static uint8_t guid_ok[16] = { 0x6b,0xc5,0xaa,0x00, 0x44,0xcd, 0xd0,0x11, 0x8c,0xc2,0x00,0xc0,0x4f,0xc2,0x95,0xee };
static uint8_t guid_bad[16] = { 0x6b,0xc5,0xaa,0x00, 0x44,0xcd, 0xd0,0x11, 0x8c,0xc2,0x00,0xc0,0x4f,0xc2,0x95,0xef };
static void put32(uint8_t *b, int o, uint32_t v) { memcpy(b + o, &v, 4); }
static void put64(uint8_t *b, int o, uint64_t v) { memcpy(b + o, &v, 8); }
static uint64_t get64(const uint8_t *b, int o) { uint64_t v; memcpy(&v, b + o, 8); return v; }

static void build_wvt(uint16_t *path, uint32_t state_action, uint64_t state_h)
{
    memset(wvt, 0, sizeof wvt); memset(fi, 0, sizeof fi);
    put32(fi, 0, 32); put64(fi, 8, A(path));
    put32(wvt, 0, 88); put32(wvt, 24, 2); put32(wvt, 28, 0); put32(wvt, 32, 1);
    put64(wvt, 40, A(fi)); put32(wvt, 48, state_action); put64(wvt, 56, state_h);
}

static uint64_t call_wvt(const uint8_t *guid, const uint8_t *data)
{
    prep(0, guid ? A(guid) : 0, data ? A(data) : 0, 0, NULL, 0);
    fle_imp_WinVerifyTrust();
    return R[0];
}

static uint16_t *the_path;

/* mutations of a good request, by name */
static void mutate(const char *m, const uint8_t **guid, const uint8_t **data)
{
    if (!strcmp(m, "none")) return;
    if (!strcmp(m, "badguid")) *guid = guid_bad;
    else if (!strcmp(m, "nullguid")) *guid = NULL;
    else if (!strcmp(m, "nulldata")) *data = NULL;
    else if (!strcmp(m, "cbstruct")) put32(wvt, 0, 80);
    else if (!strcmp(m, "callback")) put64(wvt, 8, 0x1234);
    else if (!strcmp(m, "sipclient")) put64(wvt, 16, 0x1234);
    else if (!strcmp(m, "uichoice0")) put32(wvt, 24, 0);
    else if (!strcmp(m, "uichoice5")) put32(wvt, 24, 5);
    else if (!strcmp(m, "uichoice1")) put32(wvt, 24, 1);
    else if (!strcmp(m, "revoke")) put32(wvt, 28, 1);
    else if (!strcmp(m, "catalog")) put32(wvt, 32, 2);
    else if (!strcmp(m, "nullfile")) put64(wvt, 40, 0);
    else if (!strcmp(m, "url")) put64(wvt, 64, 0x1234);
    else if (!strcmp(m, "provflags_none")) put32(wvt, 72, 0x10);
    else if (!strcmp(m, "provflags_chain")) put32(wvt, 72, 0x40);
    else if (!strcmp(m, "uicontext")) put32(wvt, 76, 1);
    else if (!strcmp(m, "filecb")) put32(fi, 0, 24);
    else if (!strcmp(m, "nullpath")) put64(fi, 8, 0);
    else if (!strcmp(m, "hfile")) put64(fi, 16, 0x44);
    else if (!strcmp(m, "known")) put64(fi, 24, 0x44);
    else { printf("no mutation %s\n", m); exit(3); }
}

static void show_wvt(const char *what)
{
    printf("%s ret %llu state %s%llx le %llu live %u objs %u\n", what, U(R[0]), get64(wvt, 56) ? "0x" : "", U(get64(wvt, 56)), U(le()), fle_trust_states_live(), fle_trust_objects_live());
}

static uint64_t gpa(uint64_t m, const char *n)
{
    uint64_t r;
    prep(m, A(n), 0, 0, NULL, 0); fle_imp_GetProcAddress();
    r = R[0];
    printf("%s %llu\n", r ? "found" : "missing", U(le()));
    return r;
}

/* crypt32 state */
static uint64_t h_store, h_msg, ctx;
static uint8_t signer_info[1024];
static uint32_t cb_info;
static uint8_t cert_info[0x60];
static uint32_t enc, ctype, ftype;

static uint64_t cqo(const char *path_name, uint32_t objtype, uint32_t content, uint32_t format, uint32_t flags)
{
    uint16_t *w = path_name ? wide(path_name) : NULL;
    uint64_t more[7];
    more[0] = flags; more[1] = A(&enc); more[2] = A(&ctype); more[3] = A(&ftype); more[4] = A(&h_store); more[5] = A(&h_msg); more[6] = 0;
    prep(objtype, A(w), content, format, more, 7);
    fle_imp_CryptQueryObject();
    return R[0];
}

static uint64_t cmgp(uint64_t msg, uint32_t type, uint32_t idx, uint8_t *buf, uint32_t *pcb)
{
    uint64_t more[1];
    more[0] = A(pcb);
    prep(msg, type, idx, A(buf), more, 1);
    fle_imp_CryptMsgGetParam();
    return R[0];
}

static void build_cert_info(const uint8_t *issuer, uint32_t ilen, const uint8_t *serial_le, uint32_t slen)
{
    memset(cert_info, 0, sizeof cert_info);
    put32(cert_info, 8, slen); put64(cert_info, 16, A(serial_le));
    put32(cert_info, 0x30, ilen); put64(cert_info, 0x38, A(issuer));
}

static uint64_t cfind(uint64_t store, uint32_t enc_t, uint32_t flags, uint32_t ftype_, const void *para, uint64_t prev)
{
    uint64_t more[2];
    more[0] = A(para); more[1] = prev;
    prep(store, enc_t, flags, ftype_, more, 2);
    fle_imp_CertFindCertificateInStore();
    return R[0];
}

static uint64_t cname(uint64_t c, uint32_t type, uint32_t flags, uint64_t para, uint16_t *buf, uint32_t cch)
{
    uint64_t more[2];
    more[0] = A(buf); more[1] = cch;
    prep(c, type, flags, para, more, 2);
    fle_imp_CertGetNameStringW();
    return R[0];
}

static void one(void (*f)(void), uint64_t a1, uint64_t a2)
{
    prep(a1, a2, 0, 0, NULL, 0);
    f();
}

static void flow_to_context(void)
{
    uint8_t ser_le[sizeof SERIAL_BE];
    size_t i;
    uint64_t r = cqo(FLE_PATH, 1, 0x400, 2, 0);
    if (!r) { printf("cqo failed\n"); exit(3); }
    cb_info = 0;
    cmgp(h_msg, 6, 0, NULL, &cb_info);
    cmgp(h_msg, 6, 0, signer_info, &cb_info);
    for (i = 0; i < sizeof SERIAL_BE; i++) ser_le[i] = SERIAL_BE[sizeof SERIAL_BE - 1 - i];
    build_cert_info(ISSUER, sizeof ISSUER, ser_le, sizeof ser_le);
    ctx = cfind(h_store, 0x10001, 0, 0xb0000, cert_info, 0);
}

int main(int argc, char **argv)
{
    const char *s = argc > 1 ? argv[1] : "";
    const char *kind = argc > 2 ? argv[2] : "none";
    size_t i;

    if (!strcmp(s, "lookup")) {
        gpa(M(N_KERNEL32), "GetUserPreferredUILanguages");
        gpa(M(N_KERNEL32), "GetUserPreferredUILanguagesW");
        gpa(M(N_KERNEL32), "getuserpreferreduilanguages");
        return 0;
    }
    setvbuf(stdout, NULL, _IONBF, 0);   /* the driver aborts on purpose: what it printed before must not be lost in a buffer */
    if (strncmp(s, "uipref", 6) != 0 && strcmp(s, "claim-add") != 0) setup(kind);
    if (!strcmp(s, "wvt")) {                 /* kind path state_action mutation */
        const uint8_t *guid = guid_ok, *data = wvt;
        uint32_t sa = (uint32_t)strtoul(argv[4], 0, 0);
        the_path = wide(argv[3]);
        build_wvt(the_path, sa, 0);
        mutate(argv[5], &guid, &data);
        call_wvt(guid, data);
        show_wvt("wvt");
    } else if (!strcmp(s, "wvt-flow")) {     /* VERIFY (state attached, 0), CLOSE (0), the same state again stops (see wvt-close-twice) */
        the_path = wide(FLE_PATH);
        build_wvt(the_path, 1, 0);
        call_wvt(guid_ok, wvt); show_wvt("verify");
        build_wvt(the_path, 1, 0);
        call_wvt(guid_ok, wvt); show_wvt("verify2");
        { uint64_t second = get64(wvt, 56); build_wvt(the_path, 2, second); call_wvt(guid_ok, wvt); show_wvt("close2"); }
        { uint64_t st = get64(wvt, 56); printf("(second state was %#llx)\n", U(st)); }
        printf("states live %u\n", fle_trust_states_live());
    } else if (!strcmp(s, "wvt-close-twice")) {
        uint64_t st;
        the_path = wide(FLE_PATH);
        build_wvt(the_path, 1, 0); call_wvt(guid_ok, wvt); st = get64(wvt, 56);
        build_wvt(the_path, 2, st); call_wvt(guid_ok, wvt); printf("first close ret %llu\n", U(R[0]));
        build_wvt(the_path, 2, st); call_wvt(guid_ok, wvt); printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "wvt-close-foreign")) {  /* CLOSE with a handle nobody issued (argv[3]) */
        the_path = wide(FLE_PATH);
        build_wvt(the_path, 2, strtoull(argv[3], 0, 0)); call_wvt(guid_ok, wvt); printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "wvt-verify-with-state")) {  /* VERIFY with hWVTStateData already set */
        the_path = wide(FLE_PATH);
        build_wvt(the_path, 1, 0x7FFA90000004ull); call_wvt(guid_ok, wvt); printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "wvt-sigsettings")) {  /* pSignatureSettings (offset 80) not NULL */
        the_path = wide(FLE_PATH);
        build_wvt(the_path, 1, 0); put64(wvt, 80, 0x1234); call_wvt(guid_ok, wvt); printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "cqo")) {          /* kind path objtype content format flags */
        uint64_t r = cqo(argv[3], (uint32_t)strtoul(argv[4], 0, 0), (uint32_t)strtoul(argv[5], 0, 0), (uint32_t)strtoul(argv[6], 0, 0), (uint32_t)strtoul(argv[7], 0, 0));
        printf("cqo ret %llu\n", U(r));
    } else if (!strcmp(s, "crypt-flow")) {
        uint16_t nm[64];
        uint32_t need = 0;
        uint64_t r;
        r = cqo(FLE_PATH, 1, 0x400, 2, 0);
        printf("cqo ret %llu enc %x content %u format %u store %s msg %s le %llu\n", U(r), enc, ctype, ftype, h_store >= 0x7FFA90000000ull ? "in-region" : "OUT", h_msg >= 0x7FFA90000000ull ? "in-region" : "OUT", U(le()));
        printf("distinct %d live objs %u\n", h_store != h_msg, fle_trust_objects_live());
        r = cmgp(h_msg, 6, 0, NULL, &need);
        printf("size query ret %llu need %u\n", U(r), need);
        cb_info = need - 1;
        r = cmgp(h_msg, 6, 0, signer_info, &cb_info);
        printf("small buffer ret %llu le %llu cb %u\n", U(r), U(le()), cb_info);
        cb_info = need + 100;
        memset(signer_info, 0xEE, sizeof signer_info);
        r = cmgp(h_msg, 6, 0, signer_info, &cb_info);
        printf("fill ret %llu cb %u le %llu\n", U(r), cb_info, U(le()));
        printf("version %u issuer cb %u serial cb %u\n", (unsigned)(signer_info[0] | signer_info[1] << 8), (unsigned)(signer_info[8] | signer_info[9] << 8), (unsigned)(signer_info[24] | signer_info[25] << 8));
        {
            uint64_t pi = get64(signer_info, 16), ps = get64(signer_info, 32);
            printf("issuer bytes equal %d serial little-endian %d inside buffer %d\n", memcmp((void *)(uintptr_t)pi, ISSUER, sizeof ISSUER) == 0,
                   ps != 0 && ((uint8_t *)(uintptr_t)ps)[0] == 0x12 && ((uint8_t *)(uintptr_t)ps)[15] == 0x0a, pi >= A(signer_info) && ps + 16 <= A(signer_info) + need);
            printf("rest zero %d beyond-fill untouched %d\n", signer_info[40] == 0 && signer_info[88] == 0 && signer_info[104] == 0 && signer_info[120] == 0, signer_info[need] == 0xEE);
        }
        /* the guest copies the Issuer blob (+8) and the SerialNumber blob (+0x18) of the CMSG_SIGNER_INFO into a CERT_INFO (SerialNumber at +8, Issuer at +0x30) */
        memset(cert_info, 0, sizeof cert_info);
        memcpy(cert_info + 0x30, signer_info + 8, 16);
        memcpy(cert_info + 8, signer_info + 24, 16);
        ctx = cfind(h_store, 0x10001, 0, 0xb0000, cert_info, 0);
        printf("find ctx %s\n", ctx ? "non-NULL" : "NULL");
        printf("ctx enc %llx cert-encoded %llu cb %llu info %llu store-ok %d\n", U(fle_ld(ctx, 4)), U(fle_ld(ctx + 8, 8)), U(fle_ld(ctx + 16, 4)), U(fle_ld(ctx + 24, 8)), fle_ld(ctx + 32, 8) == h_store);
        r = cname(ctx, 4, 0, 0, NULL, 0);
        printf("name size ret %llu\n", U(r));
        for (i = 0; i < 64; i++) nm[i] = 0xAAAA;
        r = cname(ctx, 4, 0, 0, nm, 14);
        printf("name ret %llu [", U(r)); for (i = 0; nm[i] != 0 && i < 64; i++) putchar((int)nm[i]); printf("] nul %x next %04x\n", nm[13], nm[14]);
        r = cname(ctx, 4, 0, 0, nm, 100);
        printf("name big ret %llu\n", U(r));
        r = cname(ctx, 4, 0, 0, nm, 0);
        printf("name cch0 ret %llu\n", U(r));
        one(fle_imp_CertFreeCertificateContext, ctx, 0); printf("free ctx ret %llu live %u\n", U(R[0]), fle_trust_objects_live());
        one(fle_imp_CertFreeCertificateContext, 0, 0); printf("free NULL ret %llu\n", U(R[0]));
        one(fle_imp_CertCloseStore, h_store, 0); printf("close store ret %llu live %u\n", U(R[0]), fle_trust_objects_live());
        one(fle_imp_CryptMsgClose, h_msg, 0); printf("close msg ret %llu live %u\n", U(R[0]), fle_trust_objects_live());
    } else if (!strcmp(s, "find-miss")) {    /* argv[3]: issuer | serial | both | none (what differs) */
        uint8_t ser_le[sizeof SERIAL_BE], iss[sizeof ISSUER];
        uint64_t r;
        cqo(FLE_PATH, 1, 0x400, 2, 0);
        for (i = 0; i < sizeof SERIAL_BE; i++) ser_le[i] = SERIAL_BE[sizeof SERIAL_BE - 1 - i];
        memcpy(iss, ISSUER, sizeof iss);
        if (!strcmp(argv[3], "issuer") || !strcmp(argv[3], "both")) iss[sizeof iss - 1] ^= 1;
        if (!strcmp(argv[3], "serial") || !strcmp(argv[3], "both")) ser_le[0] ^= 1;
        if (!strcmp(argv[3], "serial-high-zero")) { /* extra most-significant zero bytes are the same number (CertCompareIntegerBlob) */ }
        build_cert_info(iss, sizeof iss, ser_le, sizeof ser_le);
        r = cfind(h_store, 0x10001, 0, 0xb0000, cert_info, 0);
        printf("find ret %llx le %llx\n", U(r), U(le()));
    } else if (!strcmp(s, "find-padded")) {  /* the serial with two extra most-significant zero bytes still matches */
        uint8_t ser_le[sizeof SERIAL_BE + 2];
        uint64_t r;
        cqo(FLE_PATH, 1, 0x400, 2, 0);
        for (i = 0; i < sizeof SERIAL_BE; i++) ser_le[i] = SERIAL_BE[sizeof SERIAL_BE - 1 - i];
        ser_le[sizeof SERIAL_BE] = 0; ser_le[sizeof SERIAL_BE + 1] = 0;
        build_cert_info(ISSUER, sizeof ISSUER, ser_le, sizeof ser_le);
        r = cfind(h_store, 0x10001, 0, 0xb0000, cert_info, 0);
        printf("find %s\n", r ? "matched" : "missed");
    } else if (!strcmp(s, "find-raw")) {     /* argv[3]: enc flags type prev store-kind */
        uint8_t ser_le[sizeof SERIAL_BE];
        cqo(FLE_PATH, 1, 0x400, 2, 0);
        for (i = 0; i < sizeof SERIAL_BE; i++) ser_le[i] = SERIAL_BE[sizeof SERIAL_BE - 1 - i];
        build_cert_info(ISSUER, sizeof ISSUER, ser_le, sizeof ser_le);
        cfind(!strcmp(argv[7], "msg") ? h_msg : h_store, (uint32_t)strtoul(argv[3], 0, 0), (uint32_t)strtoul(argv[4], 0, 0), (uint32_t)strtoul(argv[5], 0, 0), !strcmp(argv[8], "null") ? NULL : cert_info, strtoull(argv[6], 0, 0));
        printf("returned %llx\n", U(R[0]));
    } else if (!strcmp(s, "getparam-raw")) { /* argv[3]: type index nullcb */
        uint32_t need = 0;
        cqo(FLE_PATH, 1, 0x400, 2, 0);
        cmgp(h_msg, (uint32_t)strtoul(argv[3], 0, 0), (uint32_t)strtoul(argv[4], 0, 0), NULL, !strcmp(argv[5], "nullcb") ? NULL : &need);
        printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "name-raw")) {     /* argv[3]: type flags para cch */
        uint16_t nm[64];
        flow_to_context();
        cname(ctx, (uint32_t)strtoul(argv[3], 0, 0), (uint32_t)strtoul(argv[4], 0, 0), strtoull(argv[5], 0, 0), nm, (uint32_t)strtoul(argv[6], 0, 0));
        printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "double")) {       /* argv[3]: store | msg | ctx: closed twice */
        flow_to_context();
        if (!strcmp(argv[3], "store")) { one(fle_imp_CertCloseStore, h_store, 0); one(fle_imp_CertCloseStore, h_store, 0); }
        else if (!strcmp(argv[3], "msg")) { one(fle_imp_CryptMsgClose, h_msg, 0); one(fle_imp_CryptMsgClose, h_msg, 0); }
        else { one(fle_imp_CertFreeCertificateContext, ctx, 0); one(fle_imp_CertFreeCertificateContext, ctx, 0); }
        printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "foreign")) {      /* argv[3]: store-as-msg | msg-as-store | stack-ctx | wvt-state-as-msg | closeflags */
        flow_to_context();
        if (!strcmp(argv[3], "store-as-msg")) one(fle_imp_CryptMsgClose, h_store, 0);
        else if (!strcmp(argv[3], "msg-as-store")) one(fle_imp_CertCloseStore, h_msg, 0);
        else if (!strcmp(argv[3], "stack-ctx")) { static uint8_t fake[40]; one(fle_imp_CertFreeCertificateContext, A(fake), 0); }
        else if (!strcmp(argv[3], "closeflags")) one(fle_imp_CertCloseStore, h_store, 4);
        else if (!strcmp(argv[3], "null-store")) one(fle_imp_CertCloseStore, 0, 0);
        printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "uipref")) {       /* argv[2]: flags, argv[3]: mode (size | fill | small | nullbuf-size | nullnum | nullcch) */
        uint32_t flags = (uint32_t)strtoul(argv[2], 0, 0), num = 0xFFFF, cch = 0;
        uint16_t buf[32];
        uint64_t more[1];
        int mode_fill = !strcmp(argv[3], "fill"), small = !strcmp(argv[3], "small"), nobuf = !strcmp(argv[3], "nullbuf-size");
        for (i = 0; i < 32; i++) buf[i] = 0xAAAA;
        if (mode_fill) cch = 32; else if (small) cch = 3;
        more[0] = 0;
        prep(flags, !strcmp(argv[3], "nullnum") ? 0 : A(&num), (mode_fill || small) ? A(buf) : 0, !strcmp(argv[3], "nullcch") ? 0 : A(&cch), more, 0);
        if (nobuf) cch = 0;
        fle_imp_GetUserPreferredUILanguages();
        printf("ret %llu le %llu num %u cch %u\n", U(R[0]), U(le()), num, cch);
        if (mode_fill && R[0]) {
            printf("buf:"); for (i = 0; i < cch; i++) printf(" %04x", buf[i]); printf(" next %04x\n", buf[cch]);
        }
    } else if (!strcmp(s, "uipref-nonnull-buffer-size0")) {  /* a buffer but a size of 0: the page's "incorrectly set" case is a NULL buffer with a non-zero size */
        uint32_t num = 0, cch = 0;
        uint16_t buf[8];
        prep(8, A(&num), A(buf), A(&cch), NULL, 0);
        fle_imp_GetUserPreferredUILanguages();
        printf("ret %llu le %llu num %u cch %u\n", U(R[0]), U(le()), num, cch);
    } else if (!strcmp(s, "uipref-nullbuf-nonzero")) {
        uint32_t num = 0, cch = 7;
        prep(8, A(&num), 0, A(&cch), NULL, 0);
        fle_imp_GetUserPreferredUILanguages();
        printf("ret %llu le %llu num %u cch %u\n", U(R[0]), U(le()), num, cch);
    } else if (!strcmp(s, "uipref-dispatch")) {
        uint64_t va = gpa(M(N_KERNEL32), "GetUserPreferredUILanguages");
        uint32_t num = 0, cch = 0;
        uint16_t buf[16];
        prep(8, A(&num), 0, A(&cch), NULL, 0);
        fle_dispatch(va);
        printf("size ret %llu num %u cch %u\n", U(R[0]), num, cch);
        prep(8, A(&num), A(buf), A(&cch), NULL, 0);
        fle_dispatch(va);
        printf("fill ret %llu [", U(R[0])); for (i = 0; buf[i]; i++) putchar((int)buf[i]); printf("]\n");
    } else if (!strcmp(s, "claims")) {
        printf("claims %zu states %u objs %u\n", fle_trust_claim_count(), fle_trust_states_live(), fle_trust_objects_live());
    } else if (!strcmp(s, "claim-add")) {    /* argv[2..]: the refusal rules of fle_trust_claim_add */
        fle_trust_claim c;
        int r;
        memset(&c, 0, sizeof c);
        c.guest_path = FLE_PATH; c.row = "r"; c.established = 1; c.why = ""; c.signer_cn = "Image Line NV";
        c.issuer_der = ISSUER; c.issuer_len = sizeof ISSUER; c.serial_be = SERIAL_BE; c.serial_len = sizeof SERIAL_BE;
        r = fle_trust_claim_add(&c); printf("first %d\n", r);
        c.guest_path = "c:\\program files\\image-line\\fl studio 21\\flengine_x64.dll"; r = fle_trust_claim_add(&c); printf("same path other case %d\n", r);
        c.guest_path = "C:\\other.dll"; c.signer_cn = ""; r = fle_trust_claim_add(&c); printf("established without CN %d\n", r);
        c.signer_cn = "Image Line NV"; c.issuer_len = 0; r = fle_trust_claim_add(&c); printf("established without issuer %d\n", r);
        c.issuer_len = sizeof ISSUER; c.serial_len = 0; r = fle_trust_claim_add(&c); printf("established without serial %d\n", r);
        c.serial_len = sizeof SERIAL_BE; c.signer_cn = "caf\xc3\xa9"; r = fle_trust_claim_add(&c); printf("non-ASCII CN %d\n", r);
        c.signer_cn = "Image Line NV"; c.guest_path = ""; r = fle_trust_claim_add(&c); printf("empty path %d\n", r);
        c.guest_path = "C:\\x.dll"; c.established = 0; c.signer_cn = ""; c.issuer_len = 0; c.serial_len = 0; c.issuer_der = NULL; c.serial_be = NULL; r = fle_trust_claim_add(&c); printf("not established, nothing else %d\n", r);
        c.guest_path = NULL; r = fle_trust_claim_add(&c); printf("NULL path %d\n", r);
        r = fle_trust_claim_add(NULL); printf("NULL claim %d\n", r);
        printf("claims %zu\n", fle_trust_claim_count());
        for (i = 0; i < 12; i++) { char p[40]; snprintf(p, sizeof p, "C:\\f%zu.dll", i); c.guest_path = p; r = fle_trust_claim_add(&c); if (r) { printf("full at %zu\n", i); break; } }
    } else {
        printf("no scenario %s\n", s);
        return 3;
    }
    return 0;
}
"#;

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let dir = work_dir();
        let tag = etxt_common::per_process("wall-fr4-driver");
        let (_, c) = gen(&GUEST_IMPORTS);
        let gen_c = dir.join(format!("{tag}-gen.c"));
        let main_c = dir.join(format!("{tag}-main.c"));
        std::fs::write(&gen_c, &c).unwrap();
        std::fs::write(&main_c, DRIVER).unwrap();
        let rt = rt_dir();
        let mut srcs: Vec<PathBuf> = std::fs::read_dir(&rt)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("fle_") && n.ends_with(".c")
            })
            .collect();
        srcs.sort();
        let exe = dir.join(&tag);
        let mut cmd = Command::new("clang");
        cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt);
        for p in &srcs {
            cmd.arg(p);
        }
        let cc = cmd.arg(&gen_c).arg(&main_c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        exe
    })
}

fn run(args: &[&str]) -> Out {
    let r = Command::new(driver()).args(args).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn lines(args: &[&str]) -> Vec<String> {
    let o = run(args);
    assert!(o.ok, "{args:?}: did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn aborts(args: &[&str], must_say: &str) -> Out {
    let o = run(args);
    assert!(!o.ok, "{args:?}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{args:?}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains("fle:"), "{args:?}: no fle: message: {}", o.stderr);
    assert!(o.stderr.contains(must_say), "{args:?}: message lacks {must_say:?}: {}", o.stderr);
    assert!(!o.stdout.contains("returned"), "{args:?}: returned instead of stopping");
    o
}

const DLL_PATH: &str = "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll";
const EXE_PATH: &str = "C:\\Program Files\\Image-Line\\FL Studio 21\\FL64.exe";

#[test]
fn an_empty_table_stops_every_call_by_name_as_the_unanswered_import_did() {
    // a build whose harness registers nothing (every build but the FRUITY one with FLE_AUTH_CLAIMS): no call is answered, whatever the arguments
    aborts(&["wvt", "none", DLL_PATH, "1", "none"], "WinVerifyTrust: no file is vouched for: the table of authenticode claims is empty");
    aborts(&["cqo", "none", DLL_PATH, "1", "0x400", "2", "0"], "CryptQueryObject: no file is vouched for: the table of authenticode claims is empty");
    // the crypt32 calls that take a handle stop because no handle was issued
    aborts(&["foreign", "none", "closeflags"], "CryptQueryObject: no file is vouched for");
}

#[test]
fn a_claim_for_the_path_answers_zero_attaches_a_state_and_the_close_releases_it() {
    // Learn (WINTRUST_DATA): VERIFY "The hWVTStateData member will receive a handle to the state data. This handle must be freed by specifying
    // WTD_STATEACTION_CLOSE in a subsequent call." (CLOSE is answered 0: the page gives no value; hWVTStateData is left as the caller has it). The driver sets the last
    // error to 5 before each call: WinVerifyTrust does not touch it.
    let got = lines(&["wvt", "good", DLL_PATH, "1", "none"]);
    assert_eq!(got.len(), 1);
    assert!(got[0].starts_with("wvt ret 0 state 0x7ffa9000"), "{}", got[0]);
    assert!(got[0].ends_with(" le 5 live 1 objs 0"), "{}", got[0]);
    // the state handle is a token in the region 0x7FFA90000000.., a multiple of 4
    let st = u64::from_str_radix(got[0].split("state 0x").nth(1).unwrap().split(' ').next().unwrap(), 16).unwrap();
    assert!(st >= 0x7FFA_9000_0000 && st % 4 == 0, "{st:#x}");
    // IGNORE (0): the one-shot verification, no state attached
    let got = lines(&["wvt", "good", DLL_PATH, "0", "none"]);
    assert_eq!(got, ["wvt ret 0 state 0 le 5 live 0 objs 0"]);
}

#[test]
fn verify_then_close_in_the_order_the_guest_does_it_and_two_states_are_two_handles() {
    let got = lines(&["wvt-flow", "good"]);
    assert!(got[0].starts_with("verify ret 0 state 0x7ffa9000") && got[0].ends_with("live 1 objs 0"), "{got:?}");
    assert!(got[1].starts_with("verify2 ret 0 state 0x7ffa9000") && got[1].ends_with("live 2 objs 0"), "{got:?}");
    let s1 = got[0].split("state ").nth(1).unwrap().split(' ').next().unwrap();
    let s2 = got[1].split("state ").nth(1).unwrap().split(' ').next().unwrap();
    assert_ne!(s1, s2, "tokens are never reused");
    // closing the second leaves the first live
    assert!(got[2].starts_with("close2 ret 0 state ") && got[2].ends_with("live 1 objs 0"), "{got:?}");
    assert_eq!(got[4], "states live 1");
}

#[test]
fn a_second_close_a_foreign_state_and_a_verify_with_a_state_already_attached_stop_by_name() {
    aborts(&["wvt-close-twice", "good"], "the WinVerifyTrust state handle 0x7ffa9");
    let o = aborts(&["wvt-close-twice", "good"], "was already closed or freed");
    assert!(o.stdout.contains("first close ret 0"), "the first close is answered: {}", o.stdout);
    aborts(&["wvt-close-foreign", "good", "0x7FFA90000004"], "was not issued by this runtime");
    aborts(&["wvt-close-foreign", "good", "0"], "was not issued by this runtime");
    aborts(&["wvt-verify-with-state", "good"], "hWVTStateData is not NULL");
}

#[test]
fn the_path_is_compared_without_case_and_only_established_claims_are_answered() {
    assert_eq!(lines(&["wvt", "good", &DLL_PATH.to_ascii_uppercase(), "0", "none"]), ["wvt ret 0 state 0 le 5 live 0 objs 0"]);
    assert_eq!(lines(&["wvt", "good", &DLL_PATH.to_ascii_lowercase(), "0", "none"]), ["wvt ret 0 state 0 le 5 live 0 objs 0"]);
    // the host exe has its own claim
    assert_eq!(lines(&["wvt", "both", EXE_PATH, "0", "none"]), ["wvt ret 0 state 0 le 5 live 0 objs 0"]);
    // a path that is not in the table: nothing is known about it (no guess from the directory or the extension)
    aborts(&["wvt", "good", EXE_PATH, "1", "none"], "is not in the table of authenticode claims (1 claim(s)");
    aborts(&["wvt", "good", "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll.bak", "1", "none"], "is not in the table of authenticode claims");
    aborts(&["wvt", "good", "FLEngine_x64.dll", "1", "none"], "is not in the table of authenticode claims");
    aborts(&["wvt", "good", "C:\\Program Files\\Image-Line\\FL Studio 21\\\\FLEngine_x64.dll", "1", "none"], "is not in the table of authenticode claims");
    // a row that exists but does not hold (the tool found the digest or the signature wrong) is not answered, and says why
    aborts(&["wvt", "bad", DLL_PATH, "1", "none"], "has the row 'FLEngine_x64.dll' for C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll, but the row is not established: b_signature is ng");
    aborts(&["cqo", "bad", DLL_PATH, "1", "0x400", "2", "0"], "but the row is not established");
    aborts(&["wvt", "mixed", EXE_PATH, "1", "none"], "has the row 'FL64.exe' for C:\\Program Files\\Image-Line\\FL Studio 21\\FL64.exe, but the row is not established: a_digest is ng");
    // while the established claim of the same table still answers
    assert_eq!(lines(&["wvt", "mixed", DLL_PATH, "0", "none"]), ["wvt ret 0 state 0 le 5 live 0 objs 0"]);
}

#[test]
fn a_request_that_asks_for_more_than_the_claim_holds_or_that_the_page_leaves_open_stops_by_name() {
    // the request is the one of the guest (uichoice 2, no revocation, flags 0, file, state action 1); each mutation changes one thing
    let cases: [(&str, &str); 19] = [
        ("badguid", "pgActionID is not WINTRUST_ACTION_GENERIC_VERIFY_V2"),
        ("nullguid", "pgActionID is NULL"),
        ("nulldata", "pWVTData is NULL"),
        ("cbstruct", "WINTRUST_DATA.cbStruct is 0x50, not 0x58"),
        ("callback", "pPolicyCallbackData is not NULL"),
        ("sipclient", "pSIPClientData is not NULL"),
        ("uichoice0", "dwUIChoice is not one of the values 1..4"),
        ("uichoice5", "dwUIChoice is not one of the values 1..4"),
        ("revoke", "fdwRevocationChecks asks for revocation checking"),
        ("catalog", "dwUnionChoice is not WTD_CHOICE_FILE"),
        ("nullfile", "pFile is NULL for WTD_CHOICE_FILE"),
        ("url", "pwszURLReference is not NULL"),
        ("provflags_chain", "dwProvFlags is neither 0 nor WTD_REVOCATION_CHECK_NONE"),
        ("filecb", "WINTRUST_FILE_INFO.cbStruct is not 0x20"),
        ("nullpath", "pcwszFilePath is NULL"),
        ("hfile", "WINTRUST_FILE_INFO.hFile is not NULL"),
        ("known", "WINTRUST_FILE_INFO.pgKnownSubject is not NULL"),
        ("uichoice1", ""),
        ("provflags_none", ""),
    ];
    for (m, why) in cases {
        if why.is_empty() {
            // allowed variations: any of the four UI choices, and WTD_REVOCATION_CHECK_NONE (0x10) in dwProvFlags: still answered
            assert_eq!(lines(&["wvt", "good", DLL_PATH, "0", m]), ["wvt ret 0 state 0 le 5 live 0 objs 0"], "{m}");
            continue;
        }
        aborts(&["wvt", "good", DLL_PATH, "1", m], why);
    }
    aborts(&["wvt-sigsettings", "good"], "pSignatureSettings is not NULL");
    // state actions 3 and 4 are the catalog caching actions: not modelled
    aborts(&["wvt", "good", DLL_PATH, "3", "none"], "dwStateAction is not WTD_STATEACTION_IGNORE, VERIFY or CLOSE");
    aborts(&["wvt", "good", DLL_PATH, "4", "none"], "dwStateAction is not WTD_STATEACTION_IGNORE, VERIFY or CLOSE");
}

#[test]
fn the_crypt32_flow_of_the_guest_gives_the_signer_name_of_the_claim_and_every_handle_is_released() {
    let got = lines(&["crypt-flow", "good"]);
    // Learn: CryptQueryObject writes the encoding (X509 | PKCS7 = 0x10001), content type PKCS7_SIGNED_EMBED (10) and format BINARY (1) of wincrypt.h, a store and a message handle
    assert_eq!(got[0], "cqo ret 1 enc 10001 content 10 format 1 store in-region msg in-region le 5");
    assert_eq!(got[1], "distinct 1 live objs 2");
    // CMSG_SIGNER_INFO: 136 bytes + the issuer Name (17 bytes, padded to 24) + the serial (16 bytes) = 136 + 24 + 16 = 176
    assert_eq!(got[2], "size query ret 1 need 176");
    assert_eq!(got[3], "small buffer ret 0 le 234 cb 176", "ERROR_MORE_DATA and the size needed");
    assert_eq!(got[4], "fill ret 1 cb 176 le 5", "a success leaves the last error alone (the driver sets 5 before each call)");
    assert_eq!(got[5], "version 1 issuer cb 17 serial cb 16");
    assert_eq!(got[6], "issuer bytes equal 1 serial little-endian 1 inside buffer 1");
    assert_eq!(got[7], "rest zero 1 beyond-fill untouched 1");
    assert_eq!(got[8], "find ctx non-NULL");
    assert_eq!(got[9], "ctx enc 10001 cert-encoded 0 cb 0 info 0 store-ok 1");
    // CertGetNameStringW(CERT_NAME_SIMPLE_DISPLAY_TYPE): "Image Line NV" is 13 characters; the count includes the NUL; the buffer is written to its NUL only
    assert_eq!(got[10], "name size ret 14");
    assert_eq!(got[11], "name ret 14 [Image Line NV] nul 0 next aaaa");
    assert_eq!(got[12], "name big ret 14");
    assert_eq!(got[13], "name cch0 ret 14");
    assert_eq!(got[14], "free ctx ret 1 live 2");
    assert_eq!(got[15], "free NULL ret 1");
    assert_eq!(got[16], "close store ret 1 live 1");
    assert_eq!(got[17], "close msg ret 1 live 0");
}

#[test]
fn a_certificate_with_another_issuer_or_serial_is_not_found_and_the_serial_compares_as_a_number() {
    // Learn (CertFindCertificateInStore): no match gives NULL (the page lists CRYPT_E_NOT_FOUND, 0x80092004 in winerror.h)
    for what in ["issuer", "serial", "both"] {
        assert_eq!(lines(&["find-miss", "good", what]), ["find ret 0 le 80092004"], "{what}");
    }
    // the same number with extra most-significant zero bytes is the same serial (the page: leading 0x00 bytes are removed)
    assert_eq!(lines(&["find-padded", "good"]), ["find matched"]);
}

#[test]
fn the_crypt32_arguments_the_pages_leave_open_stop_by_name() {
    fn q<'a>(extra: &[&'a str]) -> Vec<&'a str> {
        let mut v = vec!["cqo", "good", DLL_PATH];
        v.extend_from_slice(extra);
        v
    }
    aborts(&q(&["2", "0x400", "2", "0"]), "dwObjectType is not CERT_QUERY_OBJECT_FILE");
    aborts(&q(&["1", "0x7ff", "2", "0"]), "dwExpectedContentTypeFlags is not CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED only");
    aborts(&q(&["1", "0x400", "0xe", "0"]), "dwExpectedFormatTypeFlags is not CERT_QUERY_FORMAT_FLAG_BINARY only");
    aborts(&q(&["1", "0x400", "2", "1"]), "dwFlags is not zero");
    aborts(&["cqo", "good", "C:\\Windows\\System32\\kernel32.dll", "1", "0x400", "2", "0"], "is not in the table of authenticode claims");
    // handles
    aborts(&["foreign", "good", "store-as-msg"], "the message handle 0x7ffa9");
    aborts(&["foreign", "good", "msg-as-store"], "the certificate store handle 0x7ffa9");
    aborts(&["foreign", "good", "stack-ctx"], "the certificate context");
    aborts(&["foreign", "good", "closeflags"], "dwFlags is not zero: CERT_CLOSE_STORE_CHECK_FLAG");
    aborts(&["foreign", "good", "null-store"], "was not issued by this runtime");
    for w in ["store", "msg", "ctx"] {
        aborts(&["double", "good", w], "was already closed or freed");
    }
    // CryptMsgGetParam: CMSG_SIGNER_INFO_PARAM (6) index 0 only; a NULL size pointer is not defined
    aborts(&["getparam-raw", "good", "7", "0", "cb"], "dwParamType is not CMSG_SIGNER_INFO_PARAM");
    aborts(&["getparam-raw", "good", "6", "1", "cb"], "dwIndex is not 0");
    aborts(&["getparam-raw", "good", "6", "0", "nullcb"], "pcbData is NULL");
    // CertFindCertificateInStore: CERT_FIND_SUBJECT_CERT (0xB0000) only, X509 | PKCS7, flags 0, no previous context, a CERT_INFO
    aborts(&["find-raw", "good", "1", "0", "0xb0000", "0", "store", "cert"], "dwCertEncodingType is not X509_ASN_ENCODING | PKCS_7_ASN_ENCODING");
    aborts(&["find-raw", "good", "0x10001", "1", "0xb0000", "0", "store", "cert"], "dwFindFlags is not zero");
    aborts(&["find-raw", "good", "0x10001", "0", "0x10000", "0", "store", "cert"], "dwFindType is not CERT_FIND_SUBJECT_CERT");
    aborts(&["find-raw", "good", "0x10001", "0", "0xb0000", "0x1234", "store", "cert"], "pPrevCertContext is not NULL");
    aborts(&["find-raw", "good", "0x10001", "0", "0xb0000", "0", "store", "null"], "pvFindPara is NULL");
    aborts(&["find-raw", "good", "0x10001", "0", "0xb0000", "0", "msg", "cert"], "the certificate store handle");
    // CertGetNameStringW: the simple display name only, flags 0, no type parameter, a buffer that holds the name
    aborts(&["name-raw", "good", "5", "0", "0", "64"], "dwType is not CERT_NAME_SIMPLE_DISPLAY_TYPE");
    aborts(&["name-raw", "good", "4", "1", "0", "64"], "dwFlags is not zero");
    aborts(&["name-raw", "good", "4", "0", "0x10", "64"], "pvTypePara is not NULL");
    aborts(&["name-raw", "good", "4", "0", "0", "5"], "cchNameString is smaller than the name");
}

#[test]
fn the_claim_table_refuses_what_the_runtime_cannot_answer_from() {
    let got = lines(&["claim-add"]);
    assert_eq!(
        got,
        [
            "first 0",
            "same path other case -1",
            "established without CN -1",
            "established without issuer -1",
            "established without serial -1",
            "non-ASCII CN -1",
            "empty path -1",
            "not established, nothing else 0",
            "NULL path -1",
            "NULL claim -1",
            "claims 2",
            "full at 6",
        ]
    );
    assert_eq!(lines(&["claims", "none"]), ["claims 0 states 0 objs 0"]);
}

#[test]
fn get_user_preferred_ui_languages_answers_the_emulated_language_with_the_buffer_rules_of_the_page() {
    // Learn: "The required size includes the two null characters": "en-US" (5) + NUL + NUL = 7; MUI_LANGUAGE_ID: "en-US will be returned as "0409"": 4 + NUL + NUL = 6.
    // The default format is MUI_LANGUAGE_NAME: flags 0 and flags 8 are the same. The last error is not touched on success (the driver sets 5).
    assert_eq!(lines(&["uipref", "8", "size"]), ["ret 1 le 5 num 1 cch 7"]);
    assert_eq!(lines(&["uipref", "0", "size"]), ["ret 1 le 5 num 1 cch 7"]);
    assert_eq!(lines(&["uipref", "4", "size"]), ["ret 1 le 5 num 1 cch 6"]);
    let got = lines(&["uipref", "8", "fill"]);
    assert_eq!(got, ["ret 1 le 5 num 1 cch 7", "buf: 0065 006e 002d 0055 0053 0000 0000 next aaaa"]);
    let got = lines(&["uipref", "4", "fill"]);
    assert_eq!(got, ["ret 1 le 5 num 1 cch 6", "buf: 0030 0034 0030 0039 0000 0000 next aaaa"]);
    // a buffer that is too small, or a NULL buffer with a non-zero size, fails with ERROR_INSUFFICIENT_BUFFER (122) ("not large enough, or it was incorrectly set to NULL")
    assert_eq!(lines(&["uipref", "8", "small"]), ["ret 0 le 122 num 65535 cch 3"]);
    assert_eq!(lines(&["uipref-nullbuf-nonzero"]), ["ret 0 le 122 num 0 cch 7"]);
    // a buffer with a size of 0 is too small, too
    assert_eq!(lines(&["uipref-nonnull-buffer-size0"]), ["ret 0 le 122 num 0 cch 0"]);
}

#[test]
fn get_user_preferred_ui_languages_is_found_through_get_proc_address_under_kernel32_and_works_through_the_pseudo_va() {
    assert_eq!(lines(&["lookup"]), ["found 5", "missing 127", "missing 127"]);
    assert_eq!(lines(&["uipref-dispatch"]), ["found 5", "size ret 1 num 1 cch 7", "fill ret 1 [en-US]"]);
}

#[test]
fn get_user_preferred_ui_languages_stops_for_the_flags_and_pointers_the_page_does_not_define() {
    let name = "GetUserPreferredUILanguages";
    aborts(&["uipref", "0x10", "size"], &format!("{name}: dwFlags holds a bit other than MUI_LANGUAGE_ID and MUI_LANGUAGE_NAME"));
    aborts(&["uipref", "0x20", "size"], &format!("{name}: dwFlags holds a bit other than"));
    aborts(&["uipref", "0x30", "size"], &format!("{name}: dwFlags holds a bit other than"));
    aborts(&["uipref", "0x2", "size"], &format!("{name}: dwFlags holds a bit other than"));
    aborts(&["uipref", "12", "size"], &format!("{name}: MUI_LANGUAGE_ID with MUI_LANGUAGE_NAME"));
    aborts(&["uipref", "8", "nullnum"], &format!("{name}: NULL pulNumLanguages or pcchLanguagesBuffer"));
    aborts(&["uipref", "8", "nullcch"], &format!("{name}: NULL pulNumLanguages or pcchLanguagesBuffer"));
}

// ---------------------------------------------------------------------------------------------
// 3. The harness
// ---------------------------------------------------------------------------------------------

fn compile_and_run_claims_selftest() -> (bool, String) {
    let dir = work_dir();
    let tag = etxt_common::uniq("auth-claims-selftest");
    let exe = dir.join(&tag);
    let rt = rt_dir();
    let h = harness_dir();
    let mut cmd = Command::new("clang");
    cmd.args(["-std=gnu11", "-O0", "-Wall", "-Wextra", "-Werror", "-fPIE", "-I"]).arg(&rt).arg("-I").arg(&h);
    for f in ["fle_vfs.c", "fle_reg.c", "fle_env.c", "fle_imports.c", "fle_modules.c", "fle_string.c"] {
        cmd.arg(rt.join(f));
    }
    let cc = cmd.arg(h.join("seeds/authenticode_claims_selftest.c")).arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "authenticode_claims_selftest does not compile warning-free: {}", String::from_utf8_lossy(&cc.stderr));
    let work = dir.join(format!("{tag}-dir"));
    std::fs::create_dir_all(&work).unwrap();
    let r = Command::new(&exe).arg(&work).output().expect("run failed");
    (r.status.success(), String::from_utf8_lossy(&r.stdout).to_string())
}

#[test]
fn the_authenticode_claims_self_test_passes_with_every_check_and_no_failure() {
    // the self-test exits with the number of failures; link set = the registry/file/environment sources and nothing of the lifted code
    let (ok, out) = compile_and_run_claims_selftest();
    assert!(ok, "the self-test failed:\n{out}");
    assert!(!out.contains("FAIL"), "{out}");
    assert!(out.lines().filter(|l| l.starts_with("ok")).count() >= 40, "fewer checks than expected:\n{out}");
    assert!(out.trim_end().ends_with("0 failure(s)"), "{out}");
}

fn script_out(extra_env: &[(&str, &str)], claims: &str) -> (i32, String) {
    // a4-multi.sh refuses a bad AUTH_CLAIMS before it builds anything: give it a readable module file (never lifted here) and a throw-away OUT
    let dir = work_dir();
    let out = dir.join(etxt_common::uniq("a4m-out"));
    let module = format!("m.dll:{}:0x2000000", manifest().join("Cargo.toml").display());
    let mut cmd = Command::new("bash");
    cmd.arg(manifest().join("../../../archive/harness-attic/a4-multi.sh")).arg(&module).env("OUT", &out).env("FLE", manifest().join("Cargo.toml")).env("FEBIN", "/nonexistent").env("LIFT", manifest().join("../..").canonicalize().unwrap()).env("TMPDIR", dir.join("tmp")).env_remove("FLE_CALL_FRUITY");
    if !claims.is_empty() {
        cmd.env("AUTH_CLAIMS", claims);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let r = cmd.output().expect("bash failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stderr).to_string())
}

#[test]
fn a4_multi_refuses_auth_claims_without_the_fruity_build_or_with_a_bad_file() {
    let t = harness_dir().join("seeds/authenticode-claims.tsv");
    let t = t.canonicalize().unwrap();
    let s = t.to_str().unwrap();
    let (code, err) = script_out(&[], s);
    assert_eq!(code, 2);
    assert!(err.contains("AUTH_CLAIMS is set without FLE_CALL_FRUITY=1"), "{err}");
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "0")], s);
    assert_eq!(code, 2);
    assert!(err.contains("AUTH_CLAIMS is set without FLE_CALL_FRUITY=1"), "{err}");
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "1")], "relative/claims.tsv");
    assert_eq!(code, 2);
    assert!(err.contains("AUTH_CLAIMS must be an absolute path"), "{err}");
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "1")], "/nonexistent/claims.tsv");
    assert_eq!(code, 2);
    assert!(err.contains("cannot read AUTH_CLAIMS file"), "{err}");
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "1")], &format!("{s} x"));
    assert_eq!(code, 2);
    assert!(err.contains("AUTH_CLAIMS holds a quote, a space"), "{err}");
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "1")], &format!("{s}:/other.tsv"));
    assert_eq!(code, 2);
    assert!(err.contains("AUTH_CLAIMS holds a quote, a space"), "{err}");
}

#[test]
fn a_caller_fle_auth_claims_does_not_vouch_for_a_run_that_did_not_ask_for_it() {
    // the script unsets FLE_AUTH_CLAIMS unless AUTH_CLAIMS asks for it, as it does for FLE_REG_SEED (decision 0116) and FLE_VFS_SEED (decision 0110)
    let t = std::fs::read_to_string(manifest().join("../../../archive/harness-attic/a4-multi.sh")).unwrap();
    assert!(t.contains("unset FLE_AUTH_CLAIMS\nAUTH_CLAIMS=${AUTH_CLAIMS:-}"), "a4-multi.sh must unset the caller's FLE_AUTH_CLAIMS before it reads AUTH_CLAIMS");
    assert!(t.contains("export FLE_AUTH_CLAIMS=\"$AUTH_CLAIMS\""));
    // and the 0116 rule for the registry seed is still in place beside it
    assert!(t.contains("unset FLE_REG_SEED\nREG_SEED=${REG_SEED:-}"));
}

fn preprocess(defines: &[&str]) -> String {
    let dir = work_dir();
    let mut cmd = Command::new("clang");
    cmd.args(["-E", "-P", "-std=gnu11", "-I"]).arg(rt_dir()).arg("-I").arg(harness_dir());
    for d in defines {
        cmd.arg(d);
    }
    let r = cmd.arg(harness_dir().join("two/main_two.c")).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(r.status.success(), "clang -E failed: {}", String::from_utf8_lossy(&r.stderr));
    String::from_utf8_lossy(&r.stdout).to_string()
}

#[test]
fn a_build_without_fle_fruity_contains_no_claims_registration_and_the_fruity_build_calls_it_once_with_the_dll_path() {
    // the native binary of the default build must stay what it was: the harness main is compiled without -DFLE_FRUITY there
    let off = preprocess(&[]);
    // (fle_vfs.h, which the harness includes anyway, declares fle_trust_claim_add: a declaration, not a call; nothing of the harness header is present)
    assert!(!off.contains("auth_claims") && !off.contains("ac_run") && !off.contains("FLE_AUTH_CLAIMS") && !off.contains("fle_trust_claim_add(&"), "the default build must not contain the claims registration");
    assert!(off.contains("vfs_seed_from_env"), "sanity: the preprocessed text is main_two.c");
    let on = preprocess(&["-DFLE_FRUITY=1"]);
    assert_eq!(on.matches("auth_claims_from_env(path) != 0").count(), 1, "one call, checked, with the path of the DLL the harness loads");
    assert!(on.contains("FLE_AUTH_CLAIMS"));
    // the call comes after the registry seeding and before the fruity precheck, i.e. before the guest runs
    let (v, r, a, f) = (on.find("vfs_seed_from_env() != 0").unwrap(), on.find("reg_seed_from_env() != 0").unwrap(), on.find("auth_claims_from_env(path) != 0").unwrap(), on.find("fruity_precheck() != 0").unwrap());
    assert!(v < r && r < a && a < f, "order: vfs seed {v}, reg seed {r}, claims {a}, fruity precheck {f}");
}

#[test]
fn the_runtime_has_no_new_source_file_and_the_claims_table_is_empty_unless_the_harness_fills_it() {
    // decision 0117: no new .c/.h in runtime/ (tests/modules.rs links every fle_*.c); the registration lives in fle_vfs.c
    let mut names: Vec<String> = std::fs::read_dir(rt_dir()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".c") || n.ends_with(".h")).collect();
    names.sort();
    for n in ["fle_vfs.c", "fle_vfs.h", "fle_uipref.c", "fle_uipref.h"] {
        assert!(names.contains(&n.to_string()), "{n}");
    }
    // decision 0183: fle_crypt.c (MD5-only CryptoAPI hashing, no signature or trust call) is the one named exception; its content is checked instead of its name
    assert!(!names.iter().any(|n| n != "fle_crypt.c" && n != "fle_crypt.h" && (n.contains("trust") || n.contains("crypt") || n.contains("auth"))), "{names:?}");
    if names.contains(&"fle_crypt.c".to_string()) {
        let c = std::fs::read_to_string(rt_dir().join("fle_crypt.c")).unwrap();
        for w in ["WinVerifyTrust", "CertVerify", "CryptVerify", "CryptSign", "CryptMsg", "trust_claim", "Authenticode"] {
            assert!(!c.contains(w), "fle_crypt.c must not make a trust verdict: {w}");
        }
    }
    // the table is a static array with no initial entry; only fle_trust_claim_add fills it
    let vfs = std::fs::read_to_string(rt_dir().join("fle_vfs.c")).unwrap();
    assert!(vfs.contains("static trust_claim trust_claims[TRUST_MAX_CLAIMS];\nstatic size_t trust_nclaims;"));
    assert_eq!(vfs.matches("trust_nclaims++").count(), 1);
    // and nothing outside the harness header calls the registration
    let mut callers = 0;
    for dir in [rt_dir(), harness_dir(), harness_dir().join("two")] {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().map(|x| x == "c" || x == "h").unwrap_or(false) && p.file_name().unwrap() != "fle_vfs.c" && p.file_name().unwrap() != "fle_vfs.h" {
                if std::fs::read_to_string(&p).unwrap().contains("fle_trust_claim_add") {
                    callers += 1;
                    assert_eq!(p.file_name().unwrap(), "authenticode_claims.h");
                }
            }
        }
    }
    assert_eq!(callers, 1);
}

#[test]
fn the_committed_claims_file_has_the_columns_and_the_honest_header_the_harness_requires() {
    let t = std::fs::read_to_string(harness_dir().join("seeds/authenticode-claims.tsv")).unwrap();
    let hdr: Vec<&str> = t.lines().take_while(|l| l.starts_with('#')).collect();
    assert!(hdr[0].contains("PROVISIONAL") && hdr[0].contains("do not edit by hand"));
    assert!(hdr.iter().any(|l| l.contains("It does NOT mean the file is trusted")));
    let rows: Vec<Vec<&str>> = t.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).map(|l| l.split('\t').collect()).collect();
    assert_eq!(rows.len(), 2);
    for r in &rows {
        assert_eq!(r.len(), 13);
        assert_eq!(r[10], "chain,revocation,time");
        assert_eq!((r[3], r[4]), ("ok", "ok"));
        assert_eq!(r[5], "Image Line NV");
        assert_eq!(r[1].len(), 64);
        assert!(r[2].starts_with("sha256:"));
    }
    assert_eq!((rows[0][0], rows[1][0]), ("FLEngine_x64.dll", "FL64.exe"));
    assert_eq!(rows[0][1], "b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d");
}

// ---------------------------------------------------------------------------------------------
// 4. The real DLL
// ---------------------------------------------------------------------------------------------

fn real_dll() -> Option<Vec<u8>> {
    let p = manifest().join("../../../binaries/win64/FLEngine_x64.dll");
    std::fs::read(p).ok()
}

fn rd32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn rd16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}

fn va_to_off(dll: &[u8], va: u64) -> Option<usize> {
    let lf = rd32(dll, 0x3c) as usize;
    let nsec = rd16(dll, lf + 6) as usize;
    let optsz = rd16(dll, lf + 20) as usize;
    let rva = va.checked_sub(0x40_0000)? as u32;
    for k in 0..nsec {
        let s = lf + 24 + optsz + 40 * k;
        let vsz = rd32(dll, s + 8);
        let vrva = rd32(dll, s + 12);
        let raw = rd32(dll, s + 20);
        if rva >= vrva && rva < vrva + vsz {
            return Some((rva - vrva + raw) as usize);
        }
    }
    None
}

fn bytes_at(dll: &[u8], va: u64, n: usize) -> Vec<u8> {
    let o = va_to_off(dll, va).unwrap_or_else(|| panic!("va {va:#x} not in a section"));
    dll[o..o + n].to_vec()
}

fn cstr_at(dll: &[u8], rva: u32) -> String {
    let off = va_to_off(dll, 0x40_0000 + rva as u64).unwrap();
    let end = off + dll[off..].iter().position(|&b| b == 0).unwrap();
    String::from_utf8_lossy(&dll[off..end]).to_string()
}

/// The import (dll, name) whose IAT slot is at `slot_va`.
fn import_of_slot(dll: &[u8], slot_va: u64) -> Option<(String, String)> {
    let lf = rd32(dll, 0x3c) as usize;
    let dir = rd32(dll, lf + 24 + 112 + 8) as u64; // data directory 1: import table RVA
    let mut d = va_to_off(dll, 0x40_0000 + dir)?;
    loop {
        let (oft, name, ft) = (rd32(dll, d), rd32(dll, d + 12), rd32(dll, d + 16));
        if name == 0 {
            return None;
        }
        let thunks = if oft != 0 { oft } else { ft };
        let mut i = 0u64;
        loop {
            let to = va_to_off(dll, 0x40_0000 + thunks as u64 + 8 * i)?;
            let ent = u64::from_le_bytes(dll[to..to + 8].try_into().unwrap());
            if ent == 0 {
                break;
            }
            if 0x40_0000 + ft as u64 + 8 * i == slot_va && ent >> 63 == 0 {
                return Some((cstr_at(dll, name).to_ascii_lowercase(), cstr_at(dll, ent as u32 + 2)));
            }
            i += 1;
        }
        d += 20;
    }
}

/// The import that the `jmp [rip+disp]` thunk at `va` goes through.
fn thunk_import(dll: &[u8], va: u64) -> (String, String) {
    let b = bytes_at(dll, va, 6);
    assert_eq!(&b[..2], &[0xff, 0x25], "thunk {va:#x} is not jmp [rip+disp]");
    let disp = i32::from_le_bytes(b[2..6].try_into().unwrap()) as i64;
    import_of_slot(dll, (va as i64 + 6 + disp) as u64).unwrap_or_else(|| panic!("no import at the slot of thunk {va:#x}"))
}

#[test]
fn the_eight_thunks_of_the_signature_check_are_the_imports_the_table_answers() {
    let Some(dll) = real_dll() else { return };
    let want = [
        (0x4256d0u64, "wintrust.dll", "WinVerifyTrust"),
        (0x1375030, "crypt32.dll", "CryptQueryObject"),
        (0x1375040, "crypt32.dll", "CryptMsgGetParam"),
        (0x1375050, "crypt32.dll", "CertCloseStore"),
        (0x1375060, "crypt32.dll", "CryptMsgClose"),
        (0x1375070, "crypt32.dll", "CertFindCertificateInStore"),
        (0x1375080, "crypt32.dll", "CertGetNameStringW"),
        (0x1375090, "crypt32.dll", "CertFreeCertificateContext"),
    ];
    for (va, d, n) in want {
        assert_eq!(thunk_import(&dll, va), (d.to_string(), n.to_string()), "thunk {va:#x}");
        assert!(NEW_ROWS.contains(&(d, n)), "{d}!{n} has a row");
    }
    // 0x1375bce: call WinVerifyTrust (the first, StateAction 1) then test eax,eax; sete al (0x1375bd3); the second call (StateAction 2) at 0x1375bf8; the DLL's own path
    // comes from GetModuleFileNameW(the module handle) at 0x1375ac9 and the host exe's from GetModuleFileNameW(NULL) (xor ecx,ecx at 0x1375f0f; call at 0x1375f1e)
    assert_eq!(bytes_at(&dll, 0x1375bd3, 4), [0x85, 0xc0, 0x0f, 0x94]);
    assert_eq!(bytes_at(&dll, 0x1375f0f, 2), [0x33, 0xc9]);
    assert_eq!(bytes_at(&dll, 0x1375f18, 6), [0x41, 0xb8, 0x04, 0x01, 0x00, 0x00]);
    assert_eq!(thunk_import(&dll, 0x4235a0), ("kernel32.dll".to_string(), "GetModuleFileNameW".to_string()));
    // the engine compares the name with the UTF-16 string "Image Line NV" at 0x1376724 (the CertGetNameStringW result), the type argument is 4 (mov edx,4 at 0x1375e40..)
    let name: Vec<u8> = "Image Line NV\0".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    assert_eq!(bytes_at(&dll, 0x1376724, name.len()), name);
    // CryptQueryObject(1, path, 0x400, 2, ...): mov ecx,1 (0x1375c52); mov r8d,0x400 (0x1375c5a); mov r9d,2 (0x1375c60)
    assert_eq!(bytes_at(&dll, 0x1375c52, 5), [0xb9, 0x01, 0x00, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x1375c5a, 6), [0x41, 0xb8, 0x00, 0x04, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x1375c60, 6), [0x41, 0xb9, 0x02, 0x00, 0x00, 0x00]);
    // CMSG_SIGNER_INFO_PARAM 6 (mov edx,6 at 0x1375cc7), CERT_FIND_SUBJECT_CERT 0xb0000 and the encoding 0x10001 (0x1375da0, 0x1375da8)
    assert_eq!(bytes_at(&dll, 0x1375cc7, 5), [0xba, 0x06, 0x00, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x1375da0, 5), [0xba, 0x01, 0x00, 0x01, 0x00]);
    assert_eq!(bytes_at(&dll, 0x1375da8, 6), [0x41, 0xb9, 0x00, 0x00, 0x0b, 0x00]);
}

#[test]
fn the_delay_loaded_ui_language_call_is_the_one_the_page_describes_and_the_engine_gives_the_message_box_a_cpu_error() {
    let Some(dll) = real_dll() else { return };
    // the name that the delay-load helper asks kernel32 for (0x1976f7a in the delay-import name table), used by f_9df190 with MUI_LANGUAGE_NAME (mov ecx,8 at 0x9df1b0)
    let name = "GetUserPreferredUILanguages\0";
    assert_eq!(bytes_at(&dll, 0x1976f7a, name.len()), name.as_bytes());
    assert_eq!(bytes_at(&dll, 0x9df1b0, 5), [0xb9, 0x08, 0x00, 0x00, 0x00]);
    // WALL 6 (the CPU check; decision 0117 stops here): f_13755b0 reads a dword at +0x12 of the CPU-information record (pointer slot 0x15e4c78 -> 0x15f32e0) and, when bit
    // 0x10 is clear, builds "This computer's CPU won't run %s" with "FL Studio" and shows it with the message-box helper f_818d30 (flags 0x10), which asks
    // user32!GetActiveWindow (thunk 0x424b60) first
    assert_eq!(bytes_at(&dll, 0x1375631, 10), [0x8b, 0x40, 0x12, 0xa9, 0x10, 0x00, 0x00, 0x00, 0x75, 0x63]);
    let msg: Vec<u8> = "This computer's CPU won't run %s\0".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    assert_eq!(bytes_at(&dll, 0x1375860, msg.len()), msg);
    assert_eq!(bytes_at(&dll, 0x137568a, 5), [0xba, 0x10, 0x00, 0x00, 0x00]);
    assert_eq!(thunk_import(&dll, 0x424b60), ("user32.dll".to_string(), "GetActiveWindow".to_string()));
    assert_eq!(bytes_at(&dll, 0x86c8c6, 5)[0], 0xe8, "f_86c890 calls the GetActiveWindow thunk when its owner window is 0");
    // the record is written by the CPU-information unit (f_639a20): the dword is filled from CPUID leaf 1 EDX, and bit 0x10 is set by `test rax,0x8000; je; or [rec+0x12],0x10`
    // (0x639b26: mov rax,[rbp+0x38]; 0x639b2a: test rax,0x8000; 0x639b32: or dword [rip+..],0x10): EDX bit 15, CMOV
    assert_eq!(bytes_at(&dll, 0x639b26, 4), [0x48, 0x8b, 0x45, 0x38]);
    assert_eq!(bytes_at(&dll, 0x639b2a, 6), [0x48, 0xa9, 0x00, 0x80, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x639b30, 2), [0x74, 0x07]);
    assert_eq!(bytes_at(&dll, 0x639b32, 2), [0x83, 0x0d]);
    assert_eq!(bytes_at(&dll, 0x639b38, 1), [0x10]);
    // the other EDX bits that unit reads: 0 (FPU) -> 1, 4 (TSC) -> 2, 8 (CX8) -> 4, 11 (SEP) -> 8, 24 (FXSR) -> 0x40
    assert_eq!(bytes_at(&dll, 0x639ab8, 8), [0x48, 0x8b, 0x45, 0x38, 0x48, 0xa9, 0x01, 0x00]);
}

#[test]
fn the_emulated_cpu_claims_the_x86_64_baseline_so_the_engines_cpu_check_passes() {
    // Decision 0118 (fle_cpuid.c; before it, decision 0009 answered SSE | SSE2 only and wall 6 of decision 0117 was the engine refusing to run). The engine's check
    // needs EDX bit 15 (CMOV): `test rax,0x8000` in f_639a20 sets flag 0x10, which f_13755b0 requires. This test pins the CAUSE and its repair together.
    let t = std::fs::read_to_string(rt_dir().join("fle_cpuid.c")).unwrap();
    assert!(!t.contains("#define FLE_CPUID_LEAF1_EDX ((UINT64_C(1) << 25) | (UINT64_C(1) << 26))"), "the SSE|SSE2-only answer of decision 0009 is gone");
    assert!(t.contains("#define FLE_CPUID_EDX_CMOV  (UINT64_C(1) << 15)"));
    let edx: u64 = (1 << 0) | (1 << 8) | (1 << 15) | (1 << 23) | (1 << 24) | (1 << 25) | (1 << 26);
    assert_eq!(edx, 0x0780_8101);
    assert_ne!(edx & (1 << 15), 0, "CMOV bit: the engine's flag 0x10 is set");
    assert_eq!(edx & ((1 << 4) | (1 << 11)), 0, "TSC and SEP are not in the baseline");
}
