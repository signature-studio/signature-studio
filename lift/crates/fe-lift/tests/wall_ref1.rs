//! Tests of decision 0229 (docs/0229-reference-project-open.md, PROVISIONAL, ref1): the msacm32 model of runtime/fle_acm.c (an MPEG Layer-3 decoder behind the Audio Compression Manager calls that the engine's MP3 sample loader
//! makes), the SendMessageA text-message list of runtime/fle_win.c, and the rows of harness/seeds/ref1-imports.txt. No product file is needed.
//! The numbers asserted are those of the pages (return codes by name, with the SDK header values read in mingw-w64's msacm.h / mmsyscom.h); the expectations of the conversion are written by hand from the frame
//! arithmetic (a 128 kbit/s, 44.1 kHz Layer-3 frame is 144 * 128000 / 44100 = 417 bytes and decodes to 1152 samples per channel).

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn lift_dir() -> PathBuf {
    manifest().join("../..")
}
fn rt() -> PathBuf {
    lift_dir().join("runtime")
}
fn read(rel: &str) -> String {
    std::fs::read_to_string(lift_dir().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}
fn rows(text: &str) -> Vec<(String, String, String)> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(f.len(), 3, "bad row: {l}");
            (f[0].to_string(), f[1].to_string(), f[2].to_string())
        })
        .collect()
}
fn runtime_text() -> String {
    let mut all = String::new();
    for e in std::fs::read_dir(rt()).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "c").unwrap_or(false) {
            all.push_str(&std::fs::read_to_string(&p).unwrap());
            all.push('\n');
        }
    }
    all
}
fn work() -> PathBuf {
    let d = std::env::temp_dir().join("opencode").join("wall_ref1_tests");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

#[test]
fn rows_name_runtime_functions_and_are_new() {
    let ref1 = rows(&read("harness/seeds/ref1-imports.txt"));
    assert_eq!(ref1.len(), 14, "13 msacm32 rows and SendMessageA");
    let rt = runtime_text();
    let table = read("crates/fe-lift/src/imports_table.rs");
    let mut earlier: HashSet<(String, String)> = HashSet::new();
    for f in ["ship2", "dec1", "crash1", "api2", "act1", "sweep1", "win2"] {
        earlier.extend(rows(&read(&format!("harness/seeds/{f}-imports.txt"))).into_iter().map(|(d, a, _)| (d.to_lowercase(), a.to_lowercase())));
    }
    let mut seen = HashSet::new();
    for (dll, api, func) in &ref1 {
        assert!(func.starts_with("fle_imp_"), "{api}: {func}");
        assert!(dll.ends_with(".dll"));
        assert!(rt.contains(&format!("\nvoid {func}(void)")), "{func} is not defined in runtime/*.c");
        assert!(!table.contains(&format!("\"{api}\", \"{func}\"")), "{api} already has a row in imports_table.rs");
        assert!(!earlier.contains(&(dll.to_lowercase(), api.to_lowercase())), "{api} already has a row in an earlier layer");
        assert!(seen.insert(api.clone()), "{api} twice");
        let h = read("runtime/fle_acm.h") + &read("runtime/fle_win.h");
        assert!(h.contains(&format!("void {func}(void);")), "{func} is not declared in fle_acm.h / fle_win.h");
    }
    // the msacm32 rows are the thirteen functions of the engine's import table that are not acmMetrics (decision 0098)
    let acm: Vec<&str> = ref1.iter().filter(|r| r.0 == "msacm32.dll").map(|r| r.1.as_str()).collect();
    assert_eq!(
        acm,
        ["acmDriverEnum", "acmDriverOpen", "acmDriverClose", "acmDriverDetailsA", "acmFormatChooseW", "acmFormatSuggest", "acmStreamOpen", "acmStreamClose", "acmStreamSize", "acmStreamPrepareHeader", "acmStreamUnprepareHeader", "acmStreamConvert", "acmStreamMessage"]
    );
}

/// compiles fle_acm.c with a fake guest (guest addresses are host pointers) and runs `main_body`
fn run(name: &str, main_body: &str) -> (i32, Vec<String>) {
    let dir = work();
    let u = format!("{name}-{}", std::process::id());
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    let src = format!(
        r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_acm.h"
uint64_t R[16];
#define U(x) ((unsigned long long)(x))
void fle_stop(const char *api, const char *why) {{ printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }}
void fle_set_last_error(uint32_t c) {{ (void)c; }}
static int cb_calls; static uint64_t cb_hadid, cb_inst, cb_support; static uint64_t cb_ret = 1;
uint64_t fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9) {{ (void)va; (void)r9; cb_calls++; cb_hadid = rcx; cb_inst = rdx; cb_support = r8; return cb_ret; }}
static uint64_t P(const void *p) {{ return (uint64_t)(uintptr_t)p; }}
static uint64_t stack_args[16];
#define SETARGS(a,b,c,d) (R[1]=(a),R[2]=(b),R[8]=(c),R[9]=(d))
/* WAVEFORMATEX at p: tag ch rate avg align bits cb */
static void wfx(void *p, unsigned tag, unsigned ch, unsigned rate, unsigned avg, unsigned align, unsigned bits, unsigned cb) {{
    uint8_t *b = p; memcpy(b, &(uint16_t){{tag}}, 2); memcpy(b+2, &(uint16_t){{ch}}, 2); memcpy(b+4, &rate, 4); memcpy(b+8, &avg, 4); memcpy(b+12, &(uint16_t){{align}}, 2); memcpy(b+14, &(uint16_t){{bits}}, 2); memcpy(b+16, &(uint16_t){{cb}}, 2); }}
/* MPEGLAYER3WAVEFORMAT: wfx(0x55, ch, rate, bytes/s, 1, 0, 12) + wID 1, fdwFlags 0, nBlockSize 417, nFramesPerBlock 1, nCodecDelay 0 */
static void mp3fmt(uint8_t *p, unsigned ch, unsigned rate) {{ memset(p, 0, 32); wfx(p, 0x55, ch, rate, 16000, 1, 0, 12); p[18] = 1; p[24] = 0xA1; p[25] = 0x01; p[26] = 1; }}
static void pcmfmt(uint8_t *p, unsigned ch, unsigned rate) {{ memset(p, 0, 32); wfx(p, 1, ch, rate, rate * ch * 2, ch * 2, 16, 0); }}
/* the stack is the call's: acmStreamOpen has eight arguments (R[1], R[2], R[8], R[9], then fle_arg(5..8)) */
int main(void)
{{
    setvbuf(stdout, NULL, _IONBF, 0);
    R[4] = P(stack_args) - 0x28;   /* fle_arg(n >= 5) reads [rsp + 0x28 + 8 * (n - 5)] (fle_env.h): stack_args[0] is argument 5 */
{main_body}
    return 0;
}}
"#
    );
    std::fs::write(&c, src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pie"])
        .arg("-I")
        .arg(rt())
        .arg(&c)
        .arg(rt().join("fle_acm.c"))
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect())
}

#[test]
fn driver_enumeration_details_open_and_close_follow_the_pages() {
    let (rc, l) = run(
        "drv",
        r#"
    uint64_t had = 0, fn = 0x1000;
    uint8_t det[1024];
    /* acmDriverEnum: flags other than DISABLED / NOLOCAL = MMSYSERR_INVALFLAG (10); a NULL callback = MMSYSERR_INVALPARAM (11); otherwise the callback gets (hadid, instance, fdwSupport) and the call returns 0 */
    SETARGS(fn, 7, 0x1, 0); fle_imp_acm_DriverEnum(); printf("enum badflag %llu calls %d\n", U(R[0]), cb_calls);
    SETARGS(0, 7, 0x40000000, 0); fle_imp_acm_DriverEnum(); printf("enum nullfn %llu calls %d\n", U(R[0]), cb_calls);
    SETARGS(fn, 7, 0x40000000, 0); fle_imp_acm_DriverEnum(); printf("enum ok %llu calls %d inst %llu support %llu\n", U(R[0]), cb_calls, U(cb_inst), U(cb_support));
    uint64_t hadid = cb_hadid;
    /* acmDriverOpen: flags must be zero, phad not NULL, hadid must be a driver id */
    SETARGS(P(&had), hadid, 1, 0); fle_imp_acm_DriverOpen(); printf("open badflag %llu\n", U(R[0]));
    SETARGS(0, hadid, 0, 0); fle_imp_acm_DriverOpen(); printf("open nullphad %llu\n", U(R[0]));
    SETARGS(P(&had), 0x1234, 0, 0); fle_imp_acm_DriverOpen(); printf("open badid %llu\n", U(R[0]));
    SETARGS(P(&had), hadid, 0, 0); fle_imp_acm_DriverOpen(); printf("open ok %llu had nonzero %d\n", U(R[0]), had != 0);
    /* acmDriverDetailsA: cbStruct is the requested size and returns the size written; the fccType is 'audc' */
    memset(det, 0xEE, sizeof det); *(uint32_t *)det = 928;
    SETARGS(hadid, P(det), 0, 0); fle_imp_acm_DriverDetailsA(); printf("details %llu cb %u fcc %.4s mid %u pid %u support %u tags %u short %s\n", U(R[0]), *(uint32_t *)det, det + 4, *(uint16_t *)(det + 12), *(uint16_t *)(det + 14), *(uint32_t *)(det + 24), *(uint32_t *)(det + 28), det + 48);
    memset(det, 0xEE, sizeof det); *(uint32_t *)det = 20;
    SETARGS(hadid, P(det), 0, 0); fle_imp_acm_DriverDetailsA(); printf("details short cb %u byte20 %02x\n", *(uint32_t *)det, det[20]);
    SETARGS(0x99, P(det), 0, 0); fle_imp_acm_DriverDetailsA(); printf("details badhandle %llu\n", U(R[0]));
    SETARGS(had, 1, 0, 0); fle_imp_acm_DriverClose(); printf("close badflag %llu\n", U(R[0]));
    SETARGS(had, 0, 0, 0); fle_imp_acm_DriverClose(); printf("close ok %llu\n", U(R[0]));
    SETARGS(had, 0, 0, 0); fle_imp_acm_DriverClose(); printf("close again %llu\n", U(R[0]));
"#,
    );
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l[0], "enum badflag 10 calls 0");
    assert_eq!(l[1], "enum nullfn 11 calls 0");
    assert_eq!(l[2], "enum ok 0 calls 1 inst 7 support 1");
    assert_eq!(l[3], "open badflag 10");
    assert_eq!(l[4], "open nullphad 11");
    assert_eq!(l[5], "open badid 5");
    assert_eq!(l[6], "open ok 0 had nonzero 1");
    assert_eq!(l[7], "details 0 cb 928 fcc audc mid 172 pid 9 support 1 tags 2 short MPEG Layer-3");
    assert_eq!(l[8], "details short cb 20 byte20 ee");
    assert_eq!(l[9], "details badhandle 5");
    assert_eq!(l[10], "close badflag 10");
    assert_eq!(l[11], "close ok 0");
    assert_eq!(l[12], "close again 5");
}

#[test]
fn format_suggest_and_stream_open_accept_only_the_mp3_to_pcm16_conversion() {
    let (rc, l) = run(
        "fmt",
        r#"
    uint8_t src[32], dst[32], pcm[32], bad[32], mono[32];
    uint64_t has = 0;
    mp3fmt(src, 2, 44100); mp3fmt(mono, 1, 22050); pcmfmt(pcm, 2, 44100); pcmfmt(bad, 2, 44100); bad[14] = 8;
    /* acmFormatSuggest(had, src, dst, cbdst, flags): a PCM16 destination with the source's channels and rate; the flagged members of dst are valid and must be satisfiable */
    memset(dst, 0xEE, sizeof dst); SETARGS(0, P(src), P(dst), 18); stack_args[0] = 0; fle_imp_acm_FormatSuggest();
    printf("suggest %llu tag %u ch %u rate %u avg %u align %u bits %u cb %u\n", U(R[0]), *(uint16_t *)dst, *(uint16_t *)(dst + 2), *(uint32_t *)(dst + 4), *(uint32_t *)(dst + 8), *(uint16_t *)(dst + 12), *(uint16_t *)(dst + 14), *(uint16_t *)(dst + 16));
    memset(dst, 0, sizeof dst); *(uint16_t *)(dst + 2) = 1;
    SETARGS(0, P(src), P(dst), 18); stack_args[0] = 0x20000; fle_imp_acm_FormatSuggest(); printf("suggest nchannels=1 wanted: %llu\n", U(R[0]));
    SETARGS(0, P(pcm), P(dst), 18); stack_args[0] = 0; fle_imp_acm_FormatSuggest(); printf("suggest from pcm: %llu\n", U(R[0]));
    memset(dst, 0, sizeof dst); *(uint32_t *)(dst + 4) = 22050;
    SETARGS(0, P(src), P(dst), 18); stack_args[0] = 0x40000; fle_imp_acm_FormatSuggest(); printf("suggest rate=22050 wanted: %llu\n", U(R[0]));
    memset(dst, 0, sizeof dst); *(uint32_t *)(dst + 4) = 44100; *(uint16_t *)(dst + 2) = 2; *(uint16_t *)dst = 1; *(uint16_t *)(dst + 14) = 16;
    SETARGS(0, P(src), P(dst), 18); stack_args[0] = 0x40000 | 0x20000 | 0x10000 | 0x80000; fle_imp_acm_FormatSuggest(); printf("suggest all four members as the driver gives them: %llu\n", U(R[0]));
    *(uint16_t *)dst = 0x55; SETARGS(0, P(src), P(dst), 18); stack_args[0] = 0x10000; fle_imp_acm_FormatSuggest(); printf("suggest tag=0x55 wanted: %llu\n", U(R[0]));
    *(uint16_t *)dst = 1; *(uint16_t *)(dst + 14) = 8; SETARGS(0, P(src), P(dst), 18); stack_args[0] = 0x80000; fle_imp_acm_FormatSuggest(); printf("suggest 8 bits wanted: %llu\n", U(R[0]));
    SETARGS(0, P(src), P(dst), 18); stack_args[0] = 0x1000000; fle_imp_acm_FormatSuggest(); printf("suggest badflag: %llu\n", U(R[0]));
    SETARGS(0x77, P(src), P(dst), 18); stack_args[0] = 0; fle_imp_acm_FormatSuggest(); printf("suggest badhandle: %llu\n", U(R[0]));
    SETARGS(0, 0, P(dst), 18); stack_args[0] = 0; fle_imp_acm_FormatSuggest(); printf("suggest null src: %llu\n", U(R[0]));
    /* acmStreamOpen(phas, had, src, dst, filter, callback, instance, flags): the arguments 5..8 are on the stack */
#define OPEN(ph, had_, s, d, f, fl) (SETARGS((ph), (had_), (s), (d)), stack_args[0] = (f), stack_args[1] = 0, stack_args[2] = 0, stack_args[3] = (fl), fle_imp_acm_StreamOpen(), R[0])
    printf("open pcm->pcm %llu\n", U(OPEN(P(&has), 0, P(pcm), P(pcm), 0, 4)));
    printf("open mp3->pcm8 %llu\n", U(OPEN(P(&has), 0, P(src), P(bad), 0, 4)));
    printf("open filter %llu\n", U(OPEN(P(&has), 0, P(src), P(pcm), 5, 4)));
    printf("open async %llu\n", U(OPEN(P(&has), 0, P(src), P(pcm), 0, 2)));
    printf("open badflag %llu\n", U(OPEN(P(&has), 0, P(src), P(pcm), 0, 0x80000000)));
    printf("open null phas %llu\n", U(OPEN(0, 0, P(src), P(pcm), 0, 4)));
    printf("open query %llu has %llu\n", U(OPEN(0, 0, P(src), P(pcm), 0, 1)), U(has));
    printf("open query mismatch %llu\n", U(OPEN(0, 0, P(src), P(bad), 0, 1)));
    printf("open badhad %llu\n", U(OPEN(P(&has), 0x55, P(src), P(pcm), 0, 4)));
    printf("open mono 22.05k vs stereo 44.1k dst %llu\n", U(OPEN(P(&has), 0, P(mono), P(pcm), 0, 4)));
    printf("open ok %llu has nonzero %d\n", U(OPEN(P(&has), 0, P(src), P(pcm), 0, 4)), has != 0);
    SETARGS(has, 1, 0, 0); fle_imp_acm_StreamClose(); printf("close badflag %llu\n", U(R[0]));
    SETARGS(has, 0, 0, 0); fle_imp_acm_StreamClose(); printf("close %llu\n", U(R[0]));
    SETARGS(has, 0, 0, 0); fle_imp_acm_StreamClose(); printf("close again %llu\n", U(R[0]));
"#,
    );
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l[0], "suggest 0 tag 1 ch 2 rate 44100 avg 176400 align 4 bits 16 cb 0");
    assert_eq!(l[1], "suggest nchannels=1 wanted: 512");
    assert_eq!(l[2], "suggest from pcm: 512");
    assert_eq!(l[3], "suggest rate=22050 wanted: 512");
    assert_eq!(l[4], "suggest all four members as the driver gives them: 0");
    assert_eq!(l[5], "suggest tag=0x55 wanted: 512");
    assert_eq!(l[6], "suggest 8 bits wanted: 512");
    let l: Vec<String> = l.iter().enumerate().filter(|(i, _)| !(3..=6).contains(i)).map(|(_, x)| x.clone()).collect();
    assert_eq!(l[3], "suggest badflag: 10");
    assert_eq!(l[4], "suggest badhandle: 5");
    assert_eq!(l[5], "suggest null src: 11");
    assert_eq!(l[6], "open pcm->pcm 512");
    assert_eq!(l[7], "open mp3->pcm8 512");
    assert_eq!(l[8], "open filter 512");
    assert_eq!(l[9], "open async 512");
    assert_eq!(l[10], "open badflag 10");
    assert_eq!(l[11], "open null phas 11");
    assert_eq!(l[12], "open query 0 has 0");
    assert_eq!(l[13], "open query mismatch 512");
    assert_eq!(l[14], "open badhad 5");
    assert_eq!(l[15], "open mono 22.05k vs stereo 44.1k dst 512");
    assert_eq!(l[16], "open ok 0 has nonzero 1");
    assert_eq!(l[17], "close badflag 10");
    assert_eq!(l[18], "close 0");
    assert_eq!(l[19], "close again 5");
}

#[test]
fn a_conversion_decodes_whole_frames_reports_the_bytes_used_and_follows_the_header_states() {
    // 417-byte frames of 128 kbit/s, 44.1 kHz, stereo (header FF FB 90 00, no padding) with a zero body: each decodes to 1152 samples per channel of silence (the bit reservoir is empty, so minimp3 returns no samples and the
    // runtime gives a frame of silence for it: S-ref1-acm-reservoir-silence, one frame in = one frame out)
    let (rc, l) = run(
        "conv",
        r#"
    uint8_t src[32], pcm[32];
    static uint8_t in[417 * 10 + 100], out[4608 * 12];
    uint64_t has = 0;
    uint64_t hdr[16];   /* ACMSTREAMHEADER, 104 bytes */
    uint32_t sz = 0;
    int f;
    mp3fmt(src, 2, 44100); pcmfmt(pcm, 2, 44100);
    for (f = 0; f < 11; f++) { in[417 * f] = 0xFF; in[417 * f + 1] = 0xFB; in[417 * f + 2] = 0x90; in[417 * f + 3] = 0x00; }   /* the eleventh is a partial frame (100 bytes of 417) */
    memset(out, 0x55, sizeof out);
    SETARGS(P(&has), 0, P(src), P(pcm)); stack_args[0] = 0; stack_args[1] = 0; stack_args[2] = 0; stack_args[3] = 4; fle_imp_acm_StreamOpen();
    /* acmStreamSize: ACM_STREAMSIZEF_SOURCE (0): the recommended destination size for a source size; DESTINATION (1): the source size for a destination size; zero input is invalid; the output is never zero on success */
    SETARGS(has, 4170, P(&sz), 0); fle_imp_acm_StreamSize(); printf("size src %llu -> %u (a multiple of 4608: %d)\n", U(R[0]), sz, sz % 4608 == 0);
    SETARGS(has, 4608 * 10, P(&sz), 1); fle_imp_acm_StreamSize(); printf("size dst %llu -> %u (nonzero %d)\n", U(R[0]), sz, sz != 0);
    SETARGS(has, 100, P(&sz), 1); sz = 77; fle_imp_acm_StreamSize(); printf("size dst too small %llu -> %u\n", U(R[0]), sz);
    SETARGS(has, 0, P(&sz), 0); fle_imp_acm_StreamSize(); printf("size zero %llu\n", U(R[0]));
    SETARGS(has, 100, P(&sz), 0x10); fle_imp_acm_StreamSize(); printf("size badflag %llu\n", U(R[0]));
    memset(hdr, 0, sizeof hdr); ((uint32_t *)hdr)[0] = 104;
    *(uint64_t *)((uint8_t *)hdr + 16) = P(in); *(uint32_t *)((uint8_t *)hdr + 24) = sizeof in;
    *(uint64_t *)((uint8_t *)hdr + 40) = P(out); *(uint32_t *)((uint8_t *)hdr + 48) = 4608 * 12;
    /* a header that was not prepared: ACMERR_UNPREPARED (514) */
    SETARGS(has, P(hdr), 0, 0); fle_imp_acm_StreamConvert(); printf("convert unprepared %llu\n", U(R[0]));
    SETARGS(has, P(hdr), 1, 0); fle_imp_acm_StreamPrepareHeader(); printf("prepare badflag %llu\n", U(R[0]));
    SETARGS(has, P(hdr), 0, 0); fle_imp_acm_StreamPrepareHeader(); printf("prepare %llu status %x\n", U(R[0]), ((uint32_t *)hdr)[1]);
    SETARGS(has, P(hdr), 0, 0); fle_imp_acm_StreamPrepareHeader(); printf("prepare again %llu\n", U(R[0]));
    /* the last 100 bytes are the start of an eleventh frame: a partial frame is not consumed */
    SETARGS(has, P(hdr), 0x10, 0); fle_imp_acm_StreamConvert();
    printf("convert %llu src used %u dst used %u status %x first sample bytes %02x %02x last %02x\n", U(R[0]), ((uint32_t *)hdr)[7], ((uint32_t *)hdr)[13], ((uint32_t *)hdr)[1], out[0], out[1], out[4608 * 10 - 1]);
    printf("beyond the used bytes untouched: %02x\n", out[4608 * 10]);
    /* a destination of 3 frames: three frames are converted, the used source is 3 * 417 */
    *(uint32_t *)((uint8_t *)hdr + 48) = 4608 * 3; *(uint32_t *)((uint8_t *)hdr + 24) = 417 * 10;
    SETARGS(has, P(hdr), 0x14, 0); fle_imp_acm_StreamConvert();
    printf("convert small dst %llu src used %u dst used %u\n", U(R[0]), ((uint32_t *)hdr)[7], ((uint32_t *)hdr)[13]);
    SETARGS(has, P(hdr), 0x80, 0); fle_imp_acm_StreamConvert(); printf("convert badflag %llu\n", U(R[0]));
    SETARGS(has, P(hdr), 0, 0); fle_imp_acm_StreamUnprepareHeader(); printf("unprepare %llu status %x\n", U(R[0]), ((uint32_t *)hdr)[1]);
    SETARGS(has, P(hdr), 0, 0); fle_imp_acm_StreamUnprepareHeader(); printf("unprepare again %llu\n", U(R[0]));
    /* framing: every frame body is filled with fake headers (FF FB 90 00 at the offsets 3, 7 ... 415) and frames alternate between 417 and 418 bytes (the padding bit, header FF FB 92 00): a frame length that is off by
     * one or two bytes makes the loop meet a fake header and emit an extra frame */
    {
        static uint8_t in2[(417 + 418) * 5 + 4], out2[4608 * 16];
        uint64_t has2 = 0, h2[16];
        size_t pos = 0;
        int k, nframes = 0;
        for (k = 0; k < 10; k++) {
            size_t len = (k & 1) ? 418 : 417, o;
            for (o = 0; o < len; o++) in2[pos + o] = (o >= 3 && (o & 3) == 3) ? 0xFF : (o >= 4 && (o & 3) == 0) ? 0xFB : (o >= 5 && (o & 3) == 1) ? 0x90 : 0x00;
            in2[pos] = 0xFF; in2[pos + 1] = 0xFB; in2[pos + 2] = (k & 1) ? 0x92 : 0x90; in2[pos + 3] = 0x00;
            pos += len; nframes++;
        }
        mp3fmt(src, 2, 44100); pcmfmt(pcm, 2, 44100);
        SETARGS(P(&has2), 0, P(src), P(pcm)); stack_args[0] = 0; stack_args[1] = 0; stack_args[2] = 0; stack_args[3] = 4; fle_imp_acm_StreamOpen();
        memset(h2, 0, sizeof h2); ((uint32_t *)h2)[0] = 104;
        *(uint64_t *)((uint8_t *)h2 + 16) = P(in2); *(uint32_t *)((uint8_t *)h2 + 24) = (uint32_t)pos;
        *(uint64_t *)((uint8_t *)h2 + 40) = P(out2); *(uint32_t *)((uint8_t *)h2 + 48) = sizeof out2;
        SETARGS(has2, P(h2), 0, 0); fle_imp_acm_StreamPrepareHeader();
        SETARGS(has2, P(h2), 0x10, 0); fle_imp_acm_StreamConvert();
        printf("framing: %d frames of 417/418 bytes (%zu bytes): src used %u dst used %u frames out %u\n", nframes, pos, ((uint32_t *)h2)[7], ((uint32_t *)h2)[13], ((uint32_t *)h2)[13] / 4608);
    }
    /* a partial last frame is not consumed, whatever its length is short by: two 417-byte frames and a padded frame (418 bytes, header FF FB 92 00) of which one byte is missing; and the same with a plain frame short by one */
    {
        static uint8_t in3[417 * 2 + 417], out3[4608 * 8];
        uint64_t h3[16];
        int k;
        memset(in3, 0, sizeof in3);
        for (k = 0; k < 3; k++) { in3[417 * k] = 0xFF; in3[417 * k + 1] = 0xFB; in3[417 * k + 2] = (k == 2) ? 0x92 : 0x90; }
        memset(h3, 0, sizeof h3); ((uint32_t *)h3)[0] = 104;
        *(uint64_t *)((uint8_t *)h3 + 16) = P(in3); *(uint32_t *)((uint8_t *)h3 + 24) = sizeof in3;
        *(uint64_t *)((uint8_t *)h3 + 40) = P(out3); *(uint32_t *)((uint8_t *)h3 + 48) = sizeof out3;
        SETARGS(has, P(h3), 0, 0); fle_imp_acm_StreamPrepareHeader();
        SETARGS(has, P(h3), 0x10, 0); fle_imp_acm_StreamConvert();
        printf("partial padded frame: src used %u frames out %u\n", ((uint32_t *)h3)[7], ((uint32_t *)h3)[13] / 4608);
        in3[417 * 2 + 2] = 0x90; *(uint32_t *)((uint8_t *)h3 + 24) = 417 * 2 + 416;
        SETARGS(has, P(h3), 0x10, 0); fle_imp_acm_StreamConvert();
        printf("partial plain frame: src used %u frames out %u\n", ((uint32_t *)h3)[7], ((uint32_t *)h3)[13] / 4608);
    }
    SETARGS(has, 1, 2, 3); fle_imp_acm_StreamMessage();
"#,
    );
    assert_eq!(rc, 3, "the last call (acmStreamMessage) stops by name: {l:?}");
    assert!(l[0].starts_with("size src 0 -> ") && l[0].ends_with("(a multiple of 4608: 1)"), "{}", l[0]);
    assert!(l[1].starts_with("size dst 0 -> ") && l[1].ends_with("(nonzero 1)"), "{}", l[1]);
    assert_eq!(l[2], "size dst too small 512 -> 0");
    assert_eq!(l[3], "size zero 11");
    assert_eq!(l[4], "size badflag 10");
    assert_eq!(l[5], "convert unprepared 514");
    assert_eq!(l[6], "prepare badflag 10");
    assert_eq!(l[7], "prepare 0 status 20000");
    assert_eq!(l[8], "prepare again 0");
    assert_eq!(l[9], "convert 0 src used 4170 dst used 46080 status 30000 first sample bytes 00 00 last 00");
    assert_eq!(l[10], "beyond the used bytes untouched: 55");
    assert_eq!(l[11], "convert small dst 0 src used 1251 dst used 13824");
    assert_eq!(l[12], "convert badflag 10");
    assert_eq!(l[13], "unprepare 0 status 0");
    assert_eq!(l[14], "unprepare again 514");
    assert_eq!(l[15], "framing: 10 frames of 417/418 bytes (4175 bytes): src used 4175 dst used 46080 frames out 10");
    assert_eq!(l[16], "partial padded frame: src used 834 frames out 2");
    assert_eq!(l[17], "partial plain frame: src used 834 frames out 2");
    assert!(l[18].starts_with("STOP acmStreamMessage: "), "{}", l[18]);
}

#[test]
fn the_sendmessagea_text_messages_stop_by_name_and_the_others_are_the_w_message() {
    let w = read("runtime/fle_win.c");
    let a = w.find("void fle_imp_SendMessageA(void)").expect("fle_imp_SendMessageA");
    let body = &w[a..a + 1800];
    // WM_CREATE 1, WM_SETTEXT 0x0C, WM_GETTEXT 0x0D, WM_GETTEXTLENGTH 0x0E, WM_SETTINGCHANGE 0x1A, WM_DEVMODECHANGE 0x1B, WM_CHARTOITEM 0x2F, WM_NCCREATE 0x81, WM_CHAR 0x102, WM_DEADCHAR 0x103, WM_SYSCHAR 0x106,
    // WM_SYSDEADCHAR 0x107, WM_MENUCHAR 0x120, WM_MDICREATE 0x220, WM_IME_CHAR 0x286
    for m in ["0x0001", "0x000C", "0x000D", "0x000E", "0x001A", "0x001B", "0x002F", "0x0081", "0x0102", "0x0103", "0x0106", "0x0107", "0x0120", "0x0220", "0x0286"] {
        assert!(body.contains(&format!("case {m}:")), "message {m}");
    }
    assert!(body.contains("fle_imp_SendMessageW();"), "the other messages are sent as the W form sends them");
}

#[test]
fn a_frame_whose_rate_or_channel_mode_differs_from_the_stream_stops_by_name() {
    // the pages define no conversion between sample rates or channel counts for this driver: no answer is made up
    for (hdr2, hdr3, why) in [("0x94", "0x00", "sample rate"), ("0x90", "0xC0", "channel mode")] {
        let (rc, l) = run(
            "mismatch",
            &format!(
                r#"
    uint8_t src[32], pcm[32];
    static uint8_t in[417], out[4608 * 2];
    uint64_t has = 0, h[16];
    mp3fmt(src, 2, 44100); pcmfmt(pcm, 2, 44100);
    in[0] = 0xFF; in[1] = 0xFB; in[2] = {hdr2}; in[3] = {hdr3};
    SETARGS(P(&has), 0, P(src), P(pcm)); stack_args[0] = 0; stack_args[1] = 0; stack_args[2] = 0; stack_args[3] = 4; fle_imp_acm_StreamOpen();
    memset(h, 0, sizeof h); ((uint32_t *)h)[0] = 104;
    *(uint64_t *)((uint8_t *)h + 16) = P(in); *(uint32_t *)((uint8_t *)h + 24) = sizeof in;
    *(uint64_t *)((uint8_t *)h + 40) = P(out); *(uint32_t *)((uint8_t *)h + 48) = sizeof out;
    SETARGS(has, P(h), 0, 0); fle_imp_acm_StreamPrepareHeader();
    SETARGS(has, P(h), 0, 0); fle_imp_acm_StreamConvert();
    printf("not reached\n");
"#
            ),
        );
        assert_eq!(rc, 3, "{l:?}");
        assert!(l[0].starts_with("STOP acmStreamConvert: ") && l[0].contains(why), "{l:?}");
    }
}
