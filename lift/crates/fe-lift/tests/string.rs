//! U-G のテスト（文字列命令と CLD、STD）。
//!
//! 各命令について、ホストの x86_64 CPU で同じバイト列（`.byte` で埋め込む）を
//! 実行した結果と、生成 C（`lift_body` の出力）を clang でコンパイルして実行した
//! 結果を比べる。比べるのは、RAX、RCX、RSI と RDI（オフセット）、メモリ 1024 byte、
//! 六つのステータスフラグ、DF。
//!
//! - rep なし・rep / repe / repne 付きを含む。
//! - count 0、DF=1、RDI を RSI と重ねる場合、境界値、乱数。
//! - RAX の上位ビットを残す（8 bit、16 bit の書き込み規則）。

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;

struct Case {
    name: &'static str,
    bytes: &'static [u8],
    /// Host function name suffix in the generated C file.
    /// Host and generated functions are named by the index.
    host_asm: &'static str,
}

fn c(name: &'static str, bytes: &'static [u8], host_asm: &'static str) -> Case {
    Case { name, bytes, host_asm }
}

fn cases() -> Vec<Case> {
    vec![
        c("movsb", &[0xa4], ".byte 0xa4"),
        c("movsw", &[0x66, 0xa5], ".byte 0x66, 0xa5"),
        c("movsd", &[0xa5], ".byte 0xa5"),
        c("movsq", &[0x48, 0xa5], ".byte 0x48, 0xa5"),
        c("rep movsb", &[0xf3, 0xa4], ".byte 0xf3, 0xa4"),
        c("rep movsw", &[0xf3, 0x66, 0xa5], ".byte 0xf3, 0x66, 0xa5"),
        c("rep movsd", &[0xf3, 0xa5], ".byte 0xf3, 0xa5"),
        c("rep movsq", &[0xf3, 0x48, 0xa5], ".byte 0xf3, 0x48, 0xa5"),
        c("stosb", &[0xaa], ".byte 0xaa"),
        c("stosw", &[0x66, 0xab], ".byte 0x66, 0xab"),
        c("stosd", &[0xab], ".byte 0xab"),
        c("stosq", &[0x48, 0xab], ".byte 0x48, 0xab"),
        c("rep stosb", &[0xf3, 0xaa], ".byte 0xf3, 0xaa"),
        c("rep stosw", &[0xf3, 0x66, 0xab], ".byte 0xf3, 0x66, 0xab"),
        c("rep stosd", &[0xf3, 0xab], ".byte 0xf3, 0xab"),
        c("rep stosq", &[0xf3, 0x48, 0xab], ".byte 0xf3, 0x48, 0xab"),
        c("lodsb", &[0xac], ".byte 0xac"),
        c("lodsd", &[0xad], ".byte 0xad"),
        c("lodsq", &[0x48, 0xad], ".byte 0x48, 0xad"),
        c("scasb", &[0xae], ".byte 0xae"),
        c("scasd", &[0xaf], ".byte 0xaf"),
        c("repe scasb", &[0xf3, 0xae], ".byte 0xf3, 0xae"),
        c("repe scasd", &[0xf3, 0xaf], ".byte 0xf3, 0xaf"),
        c("repne scasb", &[0xf2, 0xae], ".byte 0xf2, 0xae"),
        c("repne scasd", &[0xf2, 0xaf], ".byte 0xf2, 0xaf"),
        c("cld", &[0xfc], ".byte 0xfc"),
        c("std", &[0xfd], ".byte 0xfd"),
    ]
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Test outputs go under lift/out-string/test (not /tmp).
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-string/test");
    std::fs::create_dir_all(&d).expect("create test output dir");
    d
}

/// The generated function for case `k`. Writes RAX, RCX, RSI and RDI offsets,
/// and the flags (six status flags and DF, in EFLAGS bit positions).
fn gen_fn(k: usize, body: &str) -> String {
    format!(
        "static void gen_{k}(uint8_t *M, uint64_t a, uint64_t cnt, unsigned df, uint64_t init, uint64_t doff, uint64_t *out) {{\n\
         \x20   R[0] = a; R[1] = cnt; R[6] = (uint64_t)(uintptr_t)(M + SRC); R[7] = (uint64_t)(uintptr_t)(M + doff);\n\
         \x20   FL.cf = (uint8_t)(init & 1); FL.pf = (uint8_t)((init >> 2) & 1); FL.af = (uint8_t)((init >> 4) & 1);\n\
         \x20   FL.zf = (uint8_t)((init >> 6) & 1); FL.sf = (uint8_t)((init >> 7) & 1); FL.of = (uint8_t)((init >> 11) & 1);\n\
         \x20   fle_df = (uint8_t)(df & 1);\n\
         \x20   {body}\n\
         \x20   out[0] = R[0]; out[1] = R[1];\n\
         \x20   out[2] = R[6] - (uint64_t)(uintptr_t)M; out[3] = R[7] - (uint64_t)(uintptr_t)M;\n\
         \x20   out[4] = (FL.cf ? 0x001u : 0) | (FL.pf ? 0x004u : 0) | (FL.af ? 0x010u : 0) | (FL.zf ? 0x040u : 0)\n\
         \x20       | (FL.sf ? 0x080u : 0) | (FL.of ? 0x800u : 0) | (fle_df ? 0x400u : 0);\n\
         }}\n\n"
    )
}

fn host_fn(k: usize, asm: &str) -> String {
    format!(
        "static void host_{k}(uint8_t *M, uint64_t a, uint64_t cnt, unsigned df, uint64_t init, uint64_t doff, uint64_t *out) {{\n\
         \x20   uint64_t ra = a, rc = cnt, rs = (uint64_t)(uintptr_t)(M + SRC), rd = (uint64_t)(uintptr_t)(M + doff), fl = 0;\n\
         \x20   uint64_t ef = (init & 0x8d5u) | 2 | (df ? 0x400u : 0);\n\
         \x20   __asm__ volatile(\"pushq %[i]\\n\\tpopfq\\n\\t{asm}\\n\\tpushfq\\n\\tpopq %[f]\\n\\tcld\"\n\
         \x20       : \"+a\"(ra), \"+c\"(rc), \"+S\"(rs), \"+D\"(rd), [f] \"=r\"(fl)\n\
         \x20       : [i] \"r\"(ef)\n\
         \x20       : \"cc\", \"memory\");\n\
         \x20   out[0] = ra; out[1] = rc;\n\
         \x20   out[2] = rs - (uint64_t)(uintptr_t)M; out[3] = rd - (uint64_t)(uintptr_t)M;\n\
         \x20   out[4] = fl & (0x8d5u | 0x400u);\n\
         }}\n\n"
    )
}

/// Test driver. Host and generated code run on two copies of the same memory.
const MAIN_C: &str = r#"
#define BUF 1024
#define DST0 640
#define DSTO 392

static uint64_t rng = 0x9e3779b97f4a7c15ULL;
static uint64_t xr(void) {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

static const uint8_t ALPHA[4] = {0x00, 0x41, 0xff, 0x01};

static void fill(uint8_t *M, unsigned mode) {
    unsigned i;
    for (i = 0; i < BUF; i++) {
        M[i] = mode ? ALPHA[xr() & 3] : (uint8_t)xr();
    }
}

static void report(unsigned k, uint64_t a, uint64_t cnt, unsigned df, uint64_t init, uint64_t doff, const uint64_t *ho, const uint64_t *go) {
    fprintf(stderr, "FAIL %s a=%#llx cnt=%llu df=%u init=%#llx doff=%llu host=%#llx/%llu/%llu/%llu/%#llx gen=%#llx/%llu/%llu/%llu/%#llx\n",
            CASES[k].name, (unsigned long long)a, (unsigned long long)cnt, df, (unsigned long long)init, (unsigned long long)doff,
            (unsigned long long)ho[0], (unsigned long long)ho[1], (unsigned long long)ho[2], (unsigned long long)ho[3], (unsigned long long)ho[4],
            (unsigned long long)go[0], (unsigned long long)go[1], (unsigned long long)go[2], (unsigned long long)go[3], (unsigned long long)go[4]);
}

int main(void) {
    static const uint64_t CNT[] = {0, 1, 2, 3, 5, 8, 16};
    static const uint64_t ACC[] = {0x0123456789abcdefULL, 0x41, 0, 0xff, 0x8000000080000000ULL};
    static const uint64_t INIT[] = {0, 0x8d5, 0x8c0};
    static uint8_t H[BUF], G[BUF];
    uint64_t ho[5], go[5];
    unsigned k, mode, d, df, ia, ii, ic, r;
    long checks = 0, fails = 0;

    for (k = 0; k < NCASES; k++) {
        for (mode = 0; mode < 2; mode++) {
            for (d = 0; d < 2; d++) {
                uint64_t doff = d ? DSTO : DST0;
                for (ic = 0; ic < sizeof CNT / sizeof CNT[0]; ic++) {
                    for (df = 0; df < 2; df++) {
                        for (ia = 0; ia < sizeof ACC / sizeof ACC[0]; ia++) {
                            for (ii = 0; ii < sizeof INIT / sizeof INIT[0]; ii++) {
                                fill(H, mode);
                                for (r = 0; r < BUF; r++) G[r] = H[r];
                                HF[k](H, ACC[ia], CNT[ic], df, INIT[ii], doff, ho);
                                GF[k](G, ACC[ia], CNT[ic], df, INIT[ii], doff, go);
                                checks++;
                                if (memcmp(H, G, BUF) || memcmp(ho, go, sizeof ho)) {
                                    fails++;
                                    if (fails <= 30) report(k, ACC[ia], CNT[ic], df, INIT[ii], doff, ho, go);
                                }
                            }
                        }
                    }
                }
            }
        }
        for (r = 0; r < 300; r++) {
            uint64_t a = xr(), cnt = xr() % 17, init = xr() & 0x8d5u;
            uint64_t doff = (xr() & 1) ? DSTO : DST0;
            unsigned df2 = (unsigned)(xr() & 1);
            mode = (unsigned)(xr() & 1);
            fill(H, mode);
            for (ii = 0; ii < BUF; ii++) G[ii] = H[ii];
            HF[k](H, a, cnt, df2, init, doff, ho);
            GF[k](G, a, cnt, df2, init, doff, go);
            checks++;
            if (memcmp(H, G, BUF) || memcmp(ho, go, sizeof ho)) {
                fails++;
                if (fails <= 30) report(k, a, cnt, df2, init, doff, ho, go);
            }
        }
    }
    printf("cases %u\nchecks %ld\nfailures %ld\n", (unsigned)NCASES, checks, fails);
    return fails > 0 ? 1 : 0;
}
"#;

#[test]
fn string_forms_match_host() {
    let cs = cases();
    let mut s = String::new();
    s.push_str("#include \"fle_runtime.h\"\n#include \"fle_string.h\"\n#include <stdio.h>\n#include <string.h>\n\n");
    s.push_str("uint64_t R[16];\nfle_eflags FL;\n\n#define SRC 384\n\n");
    for (k, c) in cs.iter().enumerate() {
        let body = lift_body(c.bytes, 0x1000).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        s.push_str(&gen_fn(k, &body));
        s.push_str(&host_fn(k, c.host_asm));
    }
    let hs: Vec<String> = (0..cs.len()).map(|k| format!("host_{k}")).collect();
    let gs: Vec<String> = (0..cs.len()).map(|k| format!("gen_{k}")).collect();
    s.push_str("typedef void (*hfn)(uint8_t *, uint64_t, uint64_t, unsigned, uint64_t, uint64_t, uint64_t *);\n");
    s.push_str(&format!("static const hfn HF[] = {{ {} }};\n", hs.join(", ")));
    s.push_str(&format!("static const hfn GF[] = {{ {} }};\n", gs.join(", ")));
    s.push_str("static const struct { const char *name; } CASES[] = {\n");
    for c in &cs {
        s.push_str(&format!("    {{\"{}\"}},\n", c.name));
    }
    s.push_str("};\n");
    s.push_str(&format!("#define NCASES {}\n", cs.len()));
    s.push_str(MAIN_C);

    let tag = etxt_common::uniq("string_check");
    let src = out_dir().join(format!("{tag}.c"));
    std::fs::write(&src, &s).expect("write generated test source");
    let bin = out_dir().join(&tag);
    let runtime = crate_dir().join("../../runtime");
    let st = Command::new("clang")
        .args(["-O0", "-mno-red-zone", "-w", "-I"])
        .arg(&runtime)
        .arg("-o")
        .arg(&bin)
        .arg(&src)
        .arg(runtime.join("fle_string.c"))
        .status()
        .expect("run clang (is clang installed?)");
    assert!(st.success(), "clang failed to build {}", src.display());
    let out = Command::new(&bin).output().expect("run test binary");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(out.status.success(), "string_check reported failures:\n{text}\n{err}");
}

#[test]
fn string_forms_lift_without_fallback() {
    for c in cases() {
        let body = lift_body(c.bytes, 0x1000).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert!(!body.contains("fle_unsupported"), "{}: {body}", c.name);
    }
}

#[test]
fn repne_on_movs_is_rejected() {
    // repne movsb: not modelled, so it must stop the lift.
    assert!(lift_body(&[0xf2, 0xa4], 0x1000).is_err());
}

#[test]
fn repne_on_stos_and_lods_is_rejected() {
    assert!(lift_body(&[0xf2, 0xaa], 0x1000).is_err());
    assert!(lift_body(&[0xf3, 0xac], 0x1000).is_err());
}

#[test]
fn address_size_prefix_is_rejected() {
    // addr32 movsb: esi and edi are 32-bit
    assert!(lift_body(&[0x67, 0xa4], 0x1000).is_err());
}

#[test]
fn sse_movsd_is_not_a_string_op() {
    // movsd xmm0, xmm1 (F2 0F 10 C1) keeps its SSE lift.
    let body = lift_body(&[0xf2, 0x0f, 0x10, 0xc1], 0x1000).expect("lift movsd xmm");
    assert!(!body.contains("fle_df"), "body: {body}");
}
