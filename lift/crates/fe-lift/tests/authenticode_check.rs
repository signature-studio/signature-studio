//! Tests of tools/authenticode_check.py (decision 0117, docs/decisions/0117-winverifytrust-and-walls.md): the offline Authenticode check that stands behind the
//! claims table of the WinVerifyTrust answer.
//!
//! The expected values are NOT read back from the tool. Files with a known signature state are made by tests/auth117/mk_signed_pe.py, which has its own DER writer,
//! its own Authenticode hash (a byte mask over the file) and its own RSA signing, and which flips exactly one thing after signing; the real FLEngine_x64.dll is
//! checked against the values that an independent run (OpenSSL's certificate printer for subject/issuer/validity/serial, the `cryptography` package for the RSA
//! verification, pefile for the header offsets) gave when the decision was written (docs/decisions/0117, section "Checked"). The tool's `ok` means: the digest of the
//! file equals the signed digest AND the SignerInfo signature verifies with the embedded signer certificate; it does not mean the certificate chain, revocation or
//! time were verified, and the tests below pin that too.
//!
//! The real FLEngine_x64.dll is read from binaries/win64 (skipped when it is absent). FL64.exe is not in the repository: the tests that need it read the path in
//! the environment variable FL64_EXE (extracted from the installer with tools/extract_nsis2.py) and are skipped without it.

use std::path::{Path, PathBuf};
use std::process::Command;

mod etxt_common;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn tool() -> PathBuf {
    manifest().join("../../../tools/authenticode_check.py")
}
fn mk() -> PathBuf {
    manifest().join("tests/auth117/mk_signed_pe.py")
}
fn mutate() -> PathBuf {
    manifest().join("tests/auth117/mutate_real.py")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-auth117/test");
    std::fs::create_dir_all(&d).unwrap();
    d
}
fn real_dll() -> Option<PathBuf> {
    let p = manifest().join("../../../binaries/win64/FLEngine_x64.dll");
    p.exists().then_some(p)
}
fn fl64() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var_os("FL64_EXE")?);
    p.exists().then_some(p)
}

fn py(args: &[&std::ffi::OsStr]) -> (i32, String, String) {
    let r = Command::new("python3").args(args).env("PYTHONDONTWRITEBYTECODE", "1").output().expect("python3 not found");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).to_string(), String::from_utf8_lossy(&r.stderr).to_string())
}

fn s(p: &Path) -> &std::ffi::OsStr {
    p.as_os_str()
}

/// Makes a signed (or tampered) file with mk_signed_pe.py; returns its path.
fn make(tag: &str, opts: &[&str]) -> PathBuf {
    let out = work_dir().join(format!("{}.exe", etxt_common::uniq(tag)));
    let m = mk();
    let mut a: Vec<&std::ffi::OsStr> = vec![s(&m), s(&out)];
    for o in opts {
        a.push(std::ffi::OsStr::new(o));
    }
    let (code, o, e) = py(&a);
    assert_eq!(code, 0, "mk_signed_pe.py {opts:?}: {o} {e}");
    out
}

struct Row {
    cols: Vec<String>,
    exit: i32,
    stdout: String,
    stderr: String,
    tsv: String,
    report: String,
}

/// Runs the tool on `files`, with --tsv and --report; returns the first data row.
fn check(files: &[&Path]) -> Row {
    let dir = work_dir();
    let tag = etxt_common::uniq("chk");
    let tsv = dir.join(format!("{tag}.tsv"));
    let rep = dir.join(format!("{tag}.md"));
    let t = tool();
    let mut a: Vec<&std::ffi::OsStr> = vec![s(&t), std::ffi::OsStr::new("--tsv"), s(&tsv), std::ffi::OsStr::new("--report"), s(&rep)];
    for f in files {
        a.push(s(f));
    }
    let (exit, stdout, stderr) = py(&a);
    let tsv_text = std::fs::read_to_string(&tsv).unwrap_or_default();
    let cols = tsv_text.lines().find(|l| !l.starts_with('#') && !l.is_empty()).map(|l| l.split('\t').map(|x| x.to_string()).collect()).unwrap_or_default();
    Row { cols, exit, stdout, stderr, tsv: tsv_text, report: std::fs::read_to_string(&rep).unwrap_or_default() }
}

// columns: 0 file, 1 file_sha256, 2 authenticode_digest, 3 a_digest, 4 b_signature, 5 signer_cn, 6 issuer_cn, 7 not_before, 8 not_after, 9 timestamp, 10 unverified, 11 issuer_der, 12 serial
const A: usize = 3;
const B: usize = 4;

fn sha256_hex(p: &Path) -> String {
    let out = Command::new("sha256sum").arg(p).output().expect("sha256sum");
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().unwrap().to_string()
}

#[test]
fn a_correctly_signed_file_passes_both_checks_and_the_digest_is_the_independent_one() {
    let f = make("good", &[]);
    let r = check(&[&f]);
    assert_eq!(r.exit, 0, "{}{}", r.stdout, r.stderr);
    assert_eq!(r.cols.len(), 13, "{:?}", r.cols);
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ok", "ok"));
    assert_eq!(r.cols[0], f.file_name().unwrap().to_str().unwrap());
    assert_eq!(r.cols[1], sha256_hex(&f), "column 2 is the SHA-256 of the whole file");
    // the digest value: mk_signed_pe.py hashes the file with its own byte mask; recompute it here with a third formulation (sha256sum over the three kept ranges)
    let data = std::fs::read(&f).unwrap();
    let lf = u32::from_le_bytes(data[0x3c..0x40].try_into().unwrap()) as usize;
    let opt = lf + 24;
    let (cks, dir) = (opt + 64, opt + 112 + 32);
    let (coff, csz) = (u32::from_le_bytes(data[dir..dir + 4].try_into().unwrap()) as usize, u32::from_le_bytes(data[dir + 4..dir + 8].try_into().unwrap()) as usize);
    assert_eq!(coff + csz, data.len(), "the certificate table is the tail");
    let kept: Vec<u8> = [&data[..cks], &data[cks + 4..dir], &data[dir + 8..coff]].concat();
    let kp = work_dir().join(format!("{}.kept", etxt_common::uniq("kept")));
    std::fs::write(&kp, &kept).unwrap();
    assert_eq!(r.cols[2], format!("sha256:{}", sha256_hex(&kp)));
    assert_eq!(r.cols[5], "Test Signer Ltd");
    assert_eq!(r.cols[6], "Test Root");
    assert_eq!((r.cols[7].as_str(), r.cols[8].as_str()), ("2026-07-28T00:00:00Z", "2027-10-21T23:59:59Z"));
    assert_eq!(r.cols[9], "none", "no timestamp in this file");
    assert_eq!(r.cols[10], "chain,revocation,time");
    // the serial number column is the unsigned big-endian hex of the certificate's serial (0x0102..0f10), the issuer column the DER of the issuer Name
    assert_eq!(r.cols[12], "0102030405060708090a0b0c0d0e0f10");
    assert!(r.cols[11].starts_with("30") && r.cols[11].len() % 2 == 0);
    assert!(r.report.contains("Not checked: chain, revocation, time"), "{}", r.report);
}

#[test]
fn a_sha1_signature_with_a_counter_signature_is_ok_and_the_timestamp_is_reported_but_not_judged() {
    let f = make("sha1ts", &["--hash", "sha1", "--timestamp"]);
    let r = check(&[&f]);
    assert_eq!(r.exit, 0, "{}{}", r.stdout, r.stderr);
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ok", "ok"));
    assert!(r.cols[2].starts_with("sha1:") && r.cols[2].len() == "sha1:".len() + 40, "{}", r.cols[2]);
    assert_eq!(r.cols[9], "countersignature@2026-09-29T14:31:53Z");
}

#[test]
fn one_changed_byte_of_the_code_fails_the_digest_check_and_not_the_signature_check() {
    let f = make("text", &["--tamper", "text"]);
    let r = check(&[&f]);
    assert_eq!(r.exit, 1, "a row that does not hold makes the exit code 1");
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ng", "ok"), "{:?}", r.cols);
    assert!(r.report.contains("**(a) digest: ng**") && r.report.contains("differs from the signed one"), "{}", r.report);
}

#[test]
fn one_changed_byte_of_the_signature_value_fails_the_signature_check_and_not_the_digest_check() {
    let f = make("sig", &["--tamper", "signature"]);
    let r = check(&[&f]);
    assert_eq!(r.exit, 1);
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ok", "ng"), "{:?}", r.cols);
    assert!(r.report.contains("RSA signature over the signed attributes"), "{}", r.report);
}

#[test]
fn a_wrong_message_digest_attribute_a_wrong_content_type_a_missing_signer_and_a_mismatched_algorithm_fail_the_signature_check_each_for_its_own_reason() {
    let cases: [(&str, &str); 4] = [
        ("messagedigest", "messageDigest differs from the sha256 of the SpcIndirectDataContent content octets"),
        ("contenttype", "contentType is missing or is not SpcIndirectDataContent"),
        ("signer", "is not among the 1 embedded certificates"),
        ("sigalg", "signatureAlgorithm names sha1 but digestAlgorithm is sha256"),
    ];
    for (tamper, why) in cases {
        let f = make(tamper, &["--tamper", tamper]);
        let r = check(&[&f]);
        assert_eq!(r.exit, 1, "{tamper}");
        assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ok", "ng"), "{tamper}: {:?}", r.cols);
        assert!(r.report.contains(why), "{tamper}: report lacks {why:?}:\n{}", r.report);
    }
}

#[test]
fn an_unsigned_file_is_a_row_that_does_not_hold_and_says_why() {
    let f = make("unsigned", &["--unsigned"]);
    let r = check(&[&f]);
    assert_eq!(r.exit, 1);
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ng", "ng"));
    assert_eq!((r.cols[5].as_str(), r.cols[6].as_str(), r.cols[2].as_str()), ("-", "-", "-"));
    assert!(r.report.contains("the file has no certificate table (not signed)"), "{}", r.report);
}

#[test]
fn the_tool_names_what_it_does_not_check_in_every_row_and_in_the_file_header() {
    let f = make("hdr", &[]);
    let r = check(&[&f]);
    let hdr: Vec<&str> = r.tsv.lines().take_while(|l| l.starts_with('#')).collect();
    assert!(hdr.iter().any(|l| l.contains("unverified = chain,revocation,time")), "{hdr:?}");
    assert!(hdr.iter().any(|l| l.contains("does NOT mean the file is trusted")), "{hdr:?}");
    // the column header line
    assert_eq!(hdr.last().unwrap().trim_start_matches("# "), "file\tfile_sha256\tauthenticode_digest\ta_digest\tb_signature\tsigner_cn\tissuer_cn\tnot_before\tnot_after\ttimestamp\tunverified\tsigner_issuer_der\tsigner_serial");
    // a bad row keeps the same column in its own row
    let g = make("hdr2", &["--tamper", "signature"]);
    assert_eq!(check(&[&g]).cols[10], "chain,revocation,time");
}

#[test]
fn the_tool_does_not_modify_its_input_and_writes_only_the_two_named_files() {
    // a directory of its own: the other tests write into the shared work directory at the same time
    let dir = work_dir().join(etxt_common::uniq("ro-dir"));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("signed.exe");
    let m = mk();
    let (code, o, e) = py(&[s(&m), s(&f)]);
    assert_eq!(code, 0, "{o} {e}");
    let before = sha256_hex(&f);
    let (tsv, rep) = (dir.join("out.tsv"), dir.join("out.md"));
    let t = tool();
    let (exit, _, err) = py(&[s(&t), std::ffi::OsStr::new("--tsv"), s(&tsv), std::ffi::OsStr::new("--report"), s(&rep), s(&f)]);
    assert_eq!(exit, 0, "{err}");
    assert_eq!(sha256_hex(&f), before);
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
    names.sort();
    assert_eq!(names, ["out.md", "out.tsv", "signed.exe"], "exactly the --tsv and --report files are new");
    // without --tsv and --report it writes nothing at all
    let (exit, out, _) = py(&[s(&t), s(&f)]);
    assert_eq!(exit, 0);
    assert!(out.contains("signed.exe"));
    let names2: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
    assert_eq!(names2.len(), 3);
}

#[test]
fn the_command_line_errors_exit_2_and_write_nothing() {
    let t = tool();
    assert_eq!(py(&[s(&t)]).0, 2, "no file");
    assert_eq!(py(&[s(&t), std::ffi::OsStr::new("--frobnicate"), std::ffi::OsStr::new("x")]).0, 2, "unknown option");
    assert_eq!(py(&[s(&t), std::ffi::OsStr::new("/nonexistent/file.exe")]).0, 2, "unreadable file");
    assert_eq!(py(&[s(&t), std::ffi::OsStr::new("--tsv")]).0, 2, "--tsv without a name");
    let not_pe = work_dir().join(format!("{}.bin", etxt_common::uniq("notpe")));
    std::fs::write(&not_pe, b"this is not a PE file at all").unwrap();
    let r = check(&[&not_pe]);
    assert_eq!(r.exit, 1);
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ng", "ng"));
    assert!(r.report.contains("not a PE file"), "{}", r.report);
}

// ---------------------------------------------------------------------------------------------
// The real files
// ---------------------------------------------------------------------------------------------

const FLE_SHA: &str = "b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d";

#[test]
fn the_real_flengine_passes_both_checks_with_the_values_of_the_independent_run() {
    let Some(dll) = real_dll() else { return };
    let r = check(&[&dll]);
    assert_eq!(r.exit, 0, "{}{}", r.stdout, r.stderr);
    assert_eq!(r.cols[0], "FLEngine_x64.dll");
    assert_eq!(r.cols[1], FLE_SHA, "the file under test is the one of decision 0017");
    // PE digest: computed by the tool, by an independent pefile + hashlib run (digest equal: True) and stored in the signature
    assert_eq!(r.cols[2], "sha256:16a77cf976070f9d356a3f575451239a1452a839e45cafc95a9c2217f9ddd5d0");
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ok", "ok"));
    // `openssl pkcs7 -print_certs -text` of the embedded certificates: Subject C=BE, L=Gent, O=Image Line NV, CN=Image Line NV; Issuer C=US, O=DigiCert, Inc.,
    // CN=DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1; Not Before Jul 28 00:00:00 2026 GMT, Not After Oct 21 23:59:59 2027 GMT; serial 0a:fc:3a:ad:...:eb:12
    assert_eq!(r.cols[5], "Image Line NV");
    assert_eq!(r.cols[6], "DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1");
    assert_eq!((r.cols[7].as_str(), r.cols[8].as_str()), ("2026-07-28T00:00:00Z", "2027-10-21T23:59:59Z"));
    assert_eq!(r.cols[12], "0afc3aad2fa7d5b5e9c267ed4a67eb12");
    assert!(r.cols[9].starts_with("rfc3161@2026-09-29T14:31:5"), "{}", r.cols[9]);
    assert_eq!(r.cols[10], "chain,revocation,time");
}

fn mutate_real(src: &Path, what: &str) -> PathBuf {
    let out = work_dir().join(format!("{}.dll", etxt_common::uniq(&format!("real-{what}"))));
    let m = mutate();
    let (code, o, e) = py(&[s(&m), s(src), s(&out), std::ffi::OsStr::new(what)]);
    assert_eq!(code, 0, "mutate_real.py {what}: {o} {e}");
    out
}

#[test]
fn a_real_flengine_with_one_byte_of_code_changed_fails_the_digest_and_one_byte_of_the_signature_fails_the_signature() {
    let Some(dll) = real_dll() else { return };
    let t = mutate_real(&dll, "text");
    let r = check(&[&t]);
    assert_eq!(r.exit, 1);
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ng", "ok"), "{:?}", r.cols);
    std::fs::remove_file(&t).unwrap();
    let g = mutate_real(&dll, "signature");
    let r = check(&[&g]);
    assert_eq!(r.exit, 1);
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ok", "ng"), "{:?}", r.cols);
    std::fs::remove_file(&g).unwrap();
}

#[test]
fn the_limits_are_real_a_changed_certificate_or_timestamp_byte_still_gives_ok_ok_because_neither_chain_nor_time_is_checked() {
    // This pins what "ok" does NOT mean (decision 0117): the certificate's own signature (the chain) and the timestamp are not verified.
    let Some(dll) = real_dll() else { return };
    let c = mutate_real(&dll, "certificate");
    let r = check(&[&c]);
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ok", "ok"), "{:?}", r.cols);
    assert_ne!(r.cols[5], "Image Line NV", "the signer name is read from the (changed) certificate: {}", r.cols[5]);
    assert_eq!(r.exit, 0);
    std::fs::remove_file(&c).unwrap();
    let t = mutate_real(&dll, "timestamp");
    let r = check(&[&t]);
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ok", "ok"), "{:?}", r.cols);
    std::fs::remove_file(&t).unwrap();
}

#[test]
fn the_real_fl64_exe_passes_both_checks_and_has_the_same_signer_as_the_engine() {
    let Some(exe) = fl64() else { return };
    let r = check(&[&exe]);
    assert_eq!(r.exit, 0, "{}{}", r.stdout, r.stderr);
    assert_eq!(r.cols[0], "FL64.exe");
    assert_eq!(r.cols[1], "4c839f24da13885fef7190d81c792fe2a2e0248dcfce9cb5f400a48ae0520c65");
    assert_eq!(r.cols[2], "sha256:2853174a524a4a9dadc8205b64ee02726a610673e50681913d2a744e0948070c");
    assert_eq!((r.cols[A].as_str(), r.cols[B].as_str()), ("ok", "ok"));
    assert_eq!((r.cols[5].as_str(), r.cols[12].as_str()), ("Image Line NV", "0afc3aad2fa7d5b5e9c267ed4a67eb12"));
}

#[test]
fn the_committed_claims_table_is_what_the_tool_writes_for_the_real_files() {
    let Some(dll) = real_dll() else { return };
    let committed = std::fs::read_to_string(manifest().join("../../harness/seeds/authenticode-claims.tsv")).unwrap();
    let rows: Vec<&str> = committed.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).collect();
    assert_eq!(rows.len(), 2, "FLEngine_x64.dll and FL64.exe");
    let r = check(&[&dll]);
    let mine: Vec<&str> = r.tsv.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).collect();
    assert_eq!(rows[0], mine[0], "the FLEngine row is exactly what the tool writes now");
    if let Some(exe) = fl64() {
        let both = check(&[&dll, &exe]);
        assert_eq!(both.tsv, committed, "the whole committed file is exactly what the tool writes for the two real files");
    }
    assert!(rows[1].starts_with("FL64.exe\t"));
}
