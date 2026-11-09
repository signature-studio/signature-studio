//! Tests of decision 0213 (docs/0213-activation.md, PROVISIONAL), section 5: runtime/fle_act1_net.c, the WinINet calls of FLEngine's HTTP unit (f_64d520) answered as "the server cannot be reached" with the failure
//! that the Microsoft pages document (Learn: InternetConnectW, HttpOpenRequestW, HttpSendRequestExW, WinINet Error Codes: ERROR_INTERNET_CANNOT_CONNECT 12029, ERROR_INTERNET_NAME_NOT_RESOLVED 12007,
//! ERROR_INTERNET_INCORRECT_HANDLE_STATE 12019), instead of a stop. The C programs call the `fle_imp_*` functions the way the lifted code reaches them: arguments in R[1], R[2], R[8], R[9], then the guest stack.
//! EVERY ANSWER IS PROVISIONAL (nothing is compared with a real Windows). FLE_ACT1_RT_DIR (a full copy of runtime/) replaces runtime/ for the mutation runs.
//!
//! What is pinned: (1) the guest's sequence (InternetOpenW, InternetConnectW(HTTP), HttpOpenRequestW, HttpAddRequestHeadersW, HttpSendRequestExW, InternetWriteFile, HttpEndRequestW, HttpQueryInfoA,
//! InternetReadFile, InternetCloseHandle x3) never succeeds at a transfer and never stops; (2) the stderr record never holds a query value, a header value or a body byte; (3) the forms the guest has not been seen to use
//! still stop by name; (4) a build without fle_act1_net.c behaves as before (InternetCloseHandle of an unknown handle stops); (5) the error of the failed send is 12029.

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_ACT1_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-act1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

const TOP: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_wininet.h"
#include "fle_act1_net.h"
uint64_t R[16];
static uint64_t stk[256];
static uint64_t cN(void (*f)(void), uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7, uint64_t a8)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u; stk[5] = a5; stk[6] = a6; stk[7] = a7; stk[8] = a8;
    R[1] = a1; R[2] = a2; R[8] = a3; R[9] = a4;
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
uint64_t fle_module_count(void) { return 40; }
const char *fle_module_name(uint64_t i) { return i == 0 ? "kernel32.dll" : i == 20 ? "wininet.dll" : "other.dll"; }
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
static void w(uint16_t *d, const char *s) { while ((*d++ = (uint8_t)*s++)) {} }
static uint16_t agent[16], host[40], verb[8], obj[200], obj2[80], hdrs[400];
"#;

fn run(name: &str, files: &[&str], defs: &[&str], body: &str) -> Out {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    std::fs::write(&c, format!("{TOP}\nint main(void)\n{{\n    setvbuf(stdout, NULL, _IONBF, 0);\n{body}\n    return 0;\n}}\n")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pie"]).args(defs).arg("-I").arg(rt()).arg(&c);
    for f in files {
        cmd.arg(rt().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_file(&c);
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

const FILES: [&str; 4] = ["fle_act1_net.c", "fle_wininet.c", "fle_imports.c", "fle_env.c"];

/// The guest's own sequence of f_64d520 (read from the disassembly, docs/investigations/act1/logs/dis-64d520.txt): the POST helper opens a session, connects (HTTP, port 443), opens the request with
/// flags 0x4c00000 (NO_CACHE_WRITE | KEEP_CONNECTION | SECURE), adds headers, HttpSendRequestExW(req, NULL, NULL, 0, 0), writes the body, ends, queries, reads and closes.
const SEQ: &str = r#"
    uint64_t s, c, r, ok;
    uint32_t written = 77, got = 77;
    char buf[8];
    w(agent, "FL Studio"); w(host, "support.image-line.com"); w(verb, "GET");
    w(obj, "/member/regfile_unlock.php?username=SECRETUSER&password=SECRETPASS&checksum=ABCDEF&flags=1&id=MACHINE&v=26.1&dn=PC");
    w(hdrs, "Host: support.image-line.com\r\nUser-Agent: FL Studio\r\nAuthorization: Bearer SECRETTOKEN\r\nContent-Length: 12\r\n");
    fle_set_last_error(0x1234);
    s = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0, 0, 0, 0);
    c = cN(fle_imp_InternetConnectW, s, A(host), 443, 0, 0, 3, 0, 0);
    printf("connect: handle %d, in the region %d, last error untouched %u\n", c != 0, c > 0x7FFAB1000000ull && c % 4 == 0, LE());
    r = cN(fle_imp_HttpOpenRequestW, c, A(verb), A(obj), 0, 0, 0, 0x4c00000, 0);
    printf("open request: handle %d distinct %d last error untouched %u\n", r != 0, r != c, LE());
    ok = cN(fle_imp_HttpAddRequestHeadersW, r, A(hdrs), 0xFFFFFFFFu, 0x20000000u, 0, 0, 0, 0);
    printf("add headers: %llu\n", U(ok));
    ok = cN(fle_imp_HttpSendRequestExW, r, 0, 0, 0, 0, 0, 0, 0);
    printf("send: %llu le %u\n", U(ok), LE());
    ok = cN(fle_imp_InternetWriteFile, r, A("abcdefgh"), 8, A(&written), 0, 0, 0, 0);
    printf("write: %llu le %u written %u\n", U(ok), LE(), written);
    ok = cN(fle_imp_HttpEndRequestW, r, 0, 0, 0, 0, 0, 0, 0);
    printf("end: %llu le %u\n", U(ok), LE());
    ok = cN(fle_imp_HttpQueryInfoA, r, 0x20000013u, A(buf), A(&got), 0, 0, 0, 0);
    printf("query status: %llu le %u\n", U(ok), LE());
    ok = cN(fle_imp_InternetReadFile, r, A(buf), 8, A(&got), 0, 0, 0, 0);
    printf("read: %llu le %u got %u\n", U(ok), LE(), got);
    printf("close request %llu, connect %llu, session %llu\n", U(cN(fle_imp_InternetCloseHandle, r, 0, 0, 0, 0, 0, 0, 0)), U(cN(fle_imp_InternetCloseHandle, c, 0, 0, 0, 0, 0, 0, 0)), U(cN(fle_imp_InternetCloseHandle, s, 0, 0, 0, 0, 0, 0, 0)));
"#;

#[test]
fn the_guests_request_sequence_fails_at_the_send_with_cannot_connect_and_never_succeeds_at_a_transfer() {
    let o = run("act1-seq", &FILES, &[], SEQ);
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(
        l,
        [
            "connect: handle 1, in the region 1, last error untouched 4660",
            "open request: handle 1 distinct 1 last error untouched 4660",
            "add headers: 1",
            "send: 0 le 12029",
            "write: 0 le 12019 written 0",
            "end: 0 le 12019",
            "query status: 0 le 12019",
            "read: 0 le 12019 got 0",
            "close request 1, connect 1, session 1"
        ]
    );
}

#[test]
fn the_record_on_stderr_holds_names_and_lengths_but_no_query_value_header_value_or_body_byte() {
    let o = run("act1-redact", &FILES, &[], SEQ);
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    for secret in ["SECRETUSER", "SECRETPASS", "SECRETTOKEN", "ABCDEF", "MACHINE", "abcdefgh", "Bearer"] {
        assert!(!o.stderr.contains(secret), "the record leaks {secret:?}:\n{}", o.stderr);
    }
    let recs: Vec<&str> = o.stderr.lines().filter(|l| l.starts_with("fle: act1-net\t")).collect();
    assert!(recs.len() >= 9, "records: {recs:?}");
    assert!(recs.iter().any(|l| l.contains("InternetConnectW") && l.contains("host=support.image-line.com") && l.contains("port=443")), "{recs:?}");
    assert!(recs.iter().any(|l| l.contains("HttpOpenRequestW") && l.contains("object=/member/regfile_unlock.php?username=<10>&password=<10>&checksum=<6>&flags=<1>&id=<7>&v=<4>&dn=<2>") && l.contains("verb=GET")), "{recs:?}");
    assert!(recs.iter().any(|l| l.contains("HttpAddRequestHeadersW") && l.contains("header-names=Host,User-Agent,Authorization,Content-Length")), "{recs:?}");
    assert!(recs.iter().any(|l| l.contains("HttpSendRequestExW") && l.contains("-> FALSE, last error 12029") && l.contains("NOT SENT")), "{recs:?}");
    assert!(recs.iter().any(|l| l.contains("InternetWriteFile") && l.contains("bytes=8")), "{recs:?}");
}

#[test]
fn last_response_info_with_nothing_stored_is_true_with_error_zero_and_an_empty_text() {
    let o = run(
        "act1-lastresp",
        &FILES,
        &[],
        r#"
    uint32_t err = 99, len = 20;
    uint16_t text[20];
    text[0] = 'x';
    printf("ret %llu err %u len %u first %u\n", U(cN(fle_imp_InternetGetLastResponseInfoW, A(&err), A(text), A(&len), 0, 0, 0, 0, 0)), err, len, text[0]);
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    assert_eq!(o.stdout.trim(), "ret 1 err 0 len 0 first 0");
}

fn stops(name: &str, body: &str, words: &[&str]) {
    let pre = r#"
    uint64_t s, c, r;
    w(agent, "FL Studio"); w(host, "h.example"); w(verb, "POST"); w(obj, "/x?a=1"); w(obj2, "/y");
    s = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0, 0, 0, 0);
    c = cN(fle_imp_InternetConnectW, s, A(host), 443, 0, 0, 3, 0, 0);
    r = cN(fle_imp_HttpOpenRequestW, c, A(verb), A(obj), 0, 0, 0, 0, 0);
"#;
    let o = run(name, &FILES, &[], &format!("{pre}{body}"));
    assert!(!o.ok, "{name}: did not stop: {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{name}: {}", o.stderr);
    let msg = o.stderr.lines().find(|l| l.starts_with("fle: ") && !l.starts_with("fle: act1-net")).unwrap_or_else(|| panic!("{name}: no message: {}", o.stderr));
    for wd in words {
        assert!(msg.contains(wd), "{name}: message lacks {wd:?}: {msg}");
    }
}

#[test]
fn the_forms_the_guest_has_not_been_seen_to_use_still_stop_by_name() {
    stops("act1-ftp", "cN(fle_imp_InternetConnectW, s, A(host), 21, 0, 0, 1, 0, 0);", &["InternetConnectW", "dwService"]);
    stops("act1-user", "cN(fle_imp_InternetConnectW, s, A(host), 21, A(agent), 0, 3, 0, 0);", &["InternetConnectW", "user name or password"]);
    stops("act1-badsess", "cN(fle_imp_InternetConnectW, 0x1234, A(host), 443, 0, 0, 3, 0, 0);", &["InternetConnectW", "hInternet"]);
    stops("act1-reqonsession", "cN(fle_imp_HttpOpenRequestW, s, A(verb), A(obj), 0, 0, 0, 0, 0);", &["HttpOpenRequestW", "hConnect"]);
    stops("act1-sendonconnect", "cN(fle_imp_HttpSendRequestExW, c, 0, 0, 0, 0, 0, 0, 0);", &["HttpSendRequestExW", "hRequest"]);
    stops("act1-sendbuffers", "cN(fle_imp_HttpSendRequestExW, r, 1, 0, 0, 0, 0, 0, 0);", &["HttpSendRequestExW", "lpBuffersIn"]);
    stops("act1-twosends", "cN(fle_imp_HttpSendRequestExW, r, 0, 0, 0, 0, 0, 0, 0); cN(fle_imp_HttpSendRequestExW, r, 0, 0, 0, 0, 0, 0, 0);", &["HttpSendRequestExW", "already sent"]);
    stops("act1-modifier", "w(hdrs, \"A: b\\r\\n\"); cN(fle_imp_HttpAddRequestHeadersW, r, A(hdrs), 0xFFFFFFFFu, 0x40000000u, 0, 0, 0, 0);", &["HttpAddRequestHeadersW", "dwModifiers"]);
    stops("act1-objnull", "cN(fle_imp_HttpOpenRequestW, c, A(verb), 0, 0, 0, 0, 0, 0);", &["HttpOpenRequestW", "lpszObjectName"]);
}

#[test]
fn a_null_verb_is_get_and_the_header_length_may_be_explicit() {
    let o = run(
        "act1-nullverb",
        &FILES,
        &[],
        r#"
    uint64_t s, c, r;
    w(agent, "FL Studio"); w(host, "h.example"); w(obj, "/p");
    w(hdrs, "Accept: */*\r\nX-Two: 2\r\n");
    s = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0, 0, 0, 0);
    c = cN(fle_imp_InternetConnectW, s, A(host), 0, 0, 0, 3, 0, 0);
    r = cN(fle_imp_HttpOpenRequestW, c, 0, A(obj), 0, 0, 0, 0, 0);
    printf("add: %llu\n", U(cN(fle_imp_HttpAddRequestHeadersW, r, A(hdrs), 11, 0x20000000u, 0, 0, 0, 0)));
    printf("send: %llu le %u\n", U(cN(fle_imp_HttpSendRequestExW, r, 0, 0, 0, 0, 0, 0, 0)), LE());
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    assert_eq!(o.stdout.lines().collect::<Vec<_>>(), ["add: 1", "send: 0 le 12029"]);
    assert!(o.stderr.contains("verb=GET"), "{}", o.stderr);
    assert!(o.stderr.contains("header-names=Accept"), "{}", o.stderr);
    assert!(!o.stderr.contains("X-Two"), "only the first 11 characters were added: {}", o.stderr);
}

#[test]
fn without_the_act1_file_closing_an_unknown_handle_stops_as_before() {
    let o = run("act1-absent", &["fle_wininet.c", "fle_imports.c", "fle_env.c"], &[], "    cN(fle_imp_InternetCloseHandle, 0x7FFAB1000004ull, 0, 0, 0, 0, 0, 0, 0);");
    assert!(!o.ok, "{}", o.stdout);
    assert_eq!(o.signal, Some(6), "{}", o.stderr);
    assert!(o.stderr.contains("InternetCloseHandle") && o.stderr.contains("not a live session"), "{}", o.stderr);
}

#[test]
fn the_open_url_record_hides_every_query_value_including_the_ampersand_form_of_the_news_url() {
    // decision 0132's answer (NULL, 12007) is unchanged; with fle_act1_net.c linked one record is written (a weak call from fle_wininet.c)
    let o = run(
        "act1-openurl",
        &FILES,
        &[],
        r#"
    uint64_t s;
    static uint16_t url[200];
    w(agent, "FL Studio");
    w(url, "https://support.image-line.com/redirect/downloadnewsrss&flsdata=SECRETDATA&id=SECRETID");
    s = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0, 0, 0, 0);
    printf("open url: %llu le %u\n", U(cN(fle_imp_InternetOpenUrlW, s, A(url), 0, 0, 0x80000000u, 0, 0, 0)), LE());
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    assert_eq!(o.stdout.trim(), "open url: 0 le 12007");
    assert!(!o.stderr.contains("SECRETDATA") && !o.stderr.contains("SECRETID"), "{}", o.stderr);
    let rec = o.stderr.lines().find(|l| l.starts_with("fle: act1-net\t") && l.contains("InternetOpenUrlW")).unwrap_or_else(|| panic!("no record: {}", o.stderr));
    assert!(rec.contains("scheme=https") && rec.contains("host=support.image-line.com") && rec.contains("object=/redirect/downloadnewsrss?flsdata=<10>&id=<8>"), "{rec}");
}

#[test]
fn a_polling_loop_does_not_run_out_of_handles() {
    // the guest may retry a request many times (an unlock that waits for the website): 600 rounds x (connect + request) = 1200 handles, all closed again
    let o = run(
        "act1-many",
        &FILES,
        &[],
        r#"
    uint64_t s, c, r;
    int i, bad = 0;
    w(agent, "FL Studio"); w(host, "h.example"); w(verb, "GET"); w(obj, "/poll?a=1");
    s = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0, 0, 0, 0);
    for (i = 0; i < 600; i++) {
        c = cN(fle_imp_InternetConnectW, s, A(host), 443, 0, 0, 3, 0, 0);
        r = cN(fle_imp_HttpOpenRequestW, c, A(verb), A(obj), 0, 0, 0, 0, 0);
        if (c == 0 || r == 0 || c == r) bad++;
        if (cN(fle_imp_HttpSendRequestExW, r, 0, 0, 0, 0, 0, 0, 0) != 0) bad++;
        if (cN(fle_imp_InternetCloseHandle, r, 0, 0, 0, 0, 0, 0, 0) != 1) bad++;
        if (cN(fle_imp_InternetCloseHandle, c, 0, 0, 0, 0, 0, 0, 0) != 1) bad++;
    }
    printf("rounds 600 bad %d\n", bad);
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    assert_eq!(o.stdout.trim(), "rounds 600 bad 0");
}
