//! U-A のテスト。フラグを更新する命令について、生成 C（`lift_body` の出力）を
//! clang でコンパイルして実行し、ホストの x86_64 CPU（`pushfq`）と比べる。
//! 比較するフラグは `docs/milestones/M1-remaining-work.md` 2.3 節の表で定義される
//! ものだけ。未定義のフラグは比較しない。
//!
//! - `flags_check.c`: `fle_flags.h` のヘルパを、ホストと表の境界値で検査する。
//! - `lifted_check.c`（このテストが生成）: 生成 C の命令本体を、ホストと同じ
//!   入力で実行して比べる。

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;

struct Case {
    name: String,
    w: u32,
    /// Index of `host_alu` in host_ref.c.
    host_op: u32,
    /// 0: compare R[0]. 1: compare R[0] and R[2] (mul, 16/32/64). 2: mul 8-bit (AX).
    /// 3: shift, the flag mask depends on the count.
    kind: u32,
    /// Bits: CF 1, PF 2, AF 4, ZF 8, SF 16, OF 32.
    mask: u32,
    bytes: Vec<u8>,
}

fn prefix(w: u32) -> Vec<u8> {
    match w {
        16 => vec![0x66],
        64 => vec![0x48],
        _ => vec![],
    }
}

/// Encodings use rax as the destination and rbx as the source.
fn cases() -> Vec<Case> {
    let mut v = Vec::new();
    // (name, host op, opcode for r/m8, r8). The wide form is opcode + 1.
    let bin: [(&str, u32, u8); 9] = [
        ("add", 0, 0x00),
        ("adc", 1, 0x10),
        ("sub", 2, 0x28),
        ("sbb", 3, 0x18),
        ("and", 4, 0x20),
        ("or", 5, 0x08),
        ("xor", 6, 0x30),
        ("cmp", 7, 0x38),
        ("test", 8, 0x84),
    ];
    for &(name, op, opc) in &bin {
        for &w in &[8u32, 16, 32, 64] {
            let mut b = prefix(w);
            b.push(if w == 8 { opc } else { opc + 1 });
            b.push(0xd8); // r/m = rax, reg = rbx
            let mask = if matches!(name, "and" | "or" | "xor" | "test") { 0x3b } else { 0x3f };
            v.push(Case { name: format!("{name}{w}"), w, host_op: op, kind: 0, mask, bytes: b });
        }
    }
    // (name, host op, opcode r/m8, opcode r/m, /n)
    let un: [(&str, u32, u8, u8, u8); 4] = [
        ("inc", 9, 0xfe, 0xff, 0),
        ("dec", 10, 0xfe, 0xff, 1),
        ("neg", 11, 0xf6, 0xf7, 3),
        ("not", 12, 0xf6, 0xf7, 2),
    ];
    for &(name, op, o8, ow, n) in &un {
        for &w in &[8u32, 16, 32, 64] {
            let mut b = prefix(w);
            b.push(if w == 8 { o8 } else { ow });
            b.push(0xc0 | (n << 3));
            // inc and dec keep CF. not changes no flags.
            let mask = match name {
                "inc" | "dec" => 0x3e,
                _ => 0x3f,
            };
            v.push(Case { name: format!("{name}{w}"), w, host_op: op, kind: 0, mask, bytes: b });
        }
    }
    // (name, host op, /n). Count in CL.
    let sh: [(&str, u32, u8); 3] = [("shl", 13, 4), ("shr", 14, 5), ("sar", 15, 7)];
    for &(name, op, n) in &sh {
        for &w in &[8u32, 16, 32, 64] {
            let mut b = prefix(w);
            b.push(if w == 8 { 0xd2 } else { 0xd3 });
            b.push(0xc0 | (n << 3));
            v.push(Case { name: format!("{name}{w}"), w, host_op: op, kind: 3, mask: 0, bytes: b });
        }
    }
    for &w in &[16u32, 32, 64] {
        let mut b = prefix(w);
        b.extend_from_slice(&[0x0f, 0xaf, 0xc3]); // imul rax, rbx
        v.push(Case { name: format!("imul2_{w}"), w, host_op: 16, kind: 0, mask: 0x21, bytes: b });
    }
    for &w in &[8u32, 16, 32, 64] {
        let mut b = prefix(w);
        b.push(if w == 8 { 0xf6 } else { 0xf7 });
        b.push(0xe3); // mul rbx (/4)
        let kind = if w == 8 { 2 } else { 1 };
        v.push(Case { name: format!("mul{w}"), w, host_op: 17, kind, mask: 0x21, bytes: b });
    }
    v
}



fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Test outputs go under lift/out-uA (not /tmp).
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-uA/test");
    std::fs::create_dir_all(&d).expect("create test output dir");
    d
}

fn build_and_run(src: &PathBuf, name: &str) -> (bool, String, String) {
    let bin = out_dir().join(etxt_common::uniq(name));
    let st = Command::new("clang")
        .args(["-O0", "-mno-red-zone", "-w", "-I"])
        .arg(crate_dir().join("../../runtime"))
        .arg("-o")
        .arg(&bin)
        .arg(src)
        .status()
        .expect("run clang (is clang installed?)");
    assert!(st.success(), "clang failed to build {}", src.display());
    let out = Command::new(&bin).output().expect("run test binary");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// C source with one function per case. Each runs the lifted body with the
/// given inputs and returns R[0], R[2] and the six flags packed as in `mask`.
fn gen_c(cs: &[Case]) -> Result<String, String> {
    let tests = crate_dir().join("tests");
    let mut s = String::new();
    s.push_str("#include \"fle_runtime.h\"\n");
    s.push_str(&format!("#include \"{}\"\n\n", tests.join("host_ref.c").display()));
    s.push_str("uint64_t R[16];\nfle_eflags FL;\n\n");
    for (k, c) in cs.iter().enumerate() {
        let body = lift_body(&c.bytes, 0x1000).map_err(|e| format!("{}: {e}", c.name))?;
        s.push_str(&format!(
            "static void t{k}(uint64_t a, uint64_t b, unsigned cfin, uint64_t cnt, uint64_t *out) {{\n\
             \x20   R[0] = a; R[1] = cnt; R[2] = 0; R[3] = b;\n\
             \x20   FL.cf = (uint8_t)(cfin & 1); FL.pf = FL.af = FL.zf = FL.sf = FL.of = 0;\n\
             \x20   {body}\n\
             \x20   out[0] = R[0]; out[1] = R[2];\n\
             \x20   out[2] = (FL.cf ? 1u : 0u) | (FL.pf ? 2u : 0u) | (FL.af ? 4u : 0u) | (FL.zf ? 8u : 0u) | (FL.sf ? 16u : 0u) | (FL.of ? 32u : 0u);\n\
             }}\n"
        ));
    }
    let fns: Vec<String> = (0..cs.len()).map(|k| format!("t{k}")).collect();
    s.push_str(&format!(
        "static void (*const FN[])(uint64_t, uint64_t, unsigned, uint64_t, uint64_t *) = {{ {} }};\n",
        fns.join(", ")
    ));
    s.push_str("static const struct { const char *name; unsigned w, op, kind, mask; } CASES[] = {\n");
    for c in cs {
        s.push_str(&format!("    {{\"{}\", {}, {}, {}, {}}},\n", c.name, c.w, c.host_op, c.kind, c.mask));
    }
    s.push_str("};\n");
    s.push_str(&format!("#define NCASES {}\n", cs.len()));
    s.push_str(MAIN_C);
    Ok(s)
}

const MAIN_C: &str = r#"
#define NB 20
#define NR 300

static uint64_t rng = 0x9e3779b97f4a7c15ULL;
static uint64_t xr(void) {
    rng ^= rng << 13;
    rng ^= rng >> 7;
    rng ^= rng << 17;
    return rng;
}

static uint64_t bval(unsigned w, unsigned i) {
    uint64_t m = fle_mask(w), s = UINT64_C(1) << (w - 1);
    static const uint64_t base[] = {0, 1, 2, 0x0f, 0x55, 0xaa, 0x7f, 0x80, 0xfe, 0xff};
    switch (i) {
    case 0: return 0;
    case 1: return 1;
    case 2: return m;
    case 3: return m - 1;
    case 4: return s;
    case 5: return s - 1;
    case 6: return (s + 1) & m;
    case 7: return 2;
    case 8: return 0x5555555555555555ULL & m;
    case 9: return 0xaaaaaaaaaaaaaaaaULL & m;
    default: return base[(i - 10) % 10] & m;
    }
}

static const uint64_t CNTS[] = {0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 255};
static long checks, fails;

static void check(unsigned k, uint64_t a, uint64_t b, unsigned cf, uint64_t cnt) {
    unsigned w = CASES[k].w, kind = CASES[k].kind, mask = CASES[k].mask;
    uint64_t m = fle_mask(w), out[3];
    uint64_t cc = 0;
    host_out o;
    unsigned c, hf, hp;
    if (kind == 3) {
        cc = cnt;
        c = (unsigned)(cnt & (w == 64 ? 0x3f : 0x1f));
        if (c > w && c != 0) {
            return;
        }
        mask = c == 0 ? 0x3f : (c == 1 ? 0x3b : 0x1b);
    }
    FN[k](a, b, cf, cc, out);
    o = host_alu(CASES[k].op, w, a, b, cf, cc);
    hf = (unsigned)o.flags;
    hp = ((hf & 0x001) ? 1u : 0u) | ((hf & 0x004) ? 2u : 0u) | ((hf & 0x010) ? 4u : 0u) |
         ((hf & 0x040) ? 8u : 0u) | ((hf & 0x080) ? 16u : 0u) | ((hf & 0x800) ? 32u : 0u);
    checks++;
    if (kind == 2) {
        if ((out[0] ^ ((o.hi << 8) | o.res)) & 0xffff) {
            goto bad;
        }
    } else if (kind == 1) {
        if (((out[0] ^ o.res) & m) || ((out[1] ^ o.hi) & m)) {
            goto bad;
        }
    } else if ((out[0] ^ o.res) & m) {
        goto bad;
    }
    if (((unsigned)out[2] ^ hp) & mask) {
        goto bad;
    }
    return;
bad:
    fails++;
    if (fails <= 30) {
        fprintf(stderr, "FAIL %s a=%#llx b=%#llx cf=%u cnt=%llu gen=%#llx/%#llx/%#x host=%#llx/%#llx/%#x mask=%#x\n",
                CASES[k].name, (unsigned long long)a, (unsigned long long)b, cf, (unsigned long long)cc,
                (unsigned long long)out[0], (unsigned long long)out[1], (unsigned)out[2],
                (unsigned long long)o.res, (unsigned long long)o.hi, hp, mask);
    }
}

int main(void) {
    unsigned k, i, j, cf, r;
    for (k = 0; k < NCASES; k++) {
        unsigned w = CASES[k].w;
        uint64_t m = fle_mask(w);
        for (cf = 0; cf < 2; cf++) {
            if (CASES[k].kind == 3) {
                for (i = 0; i < NB; i++) {
                    for (j = 0; j < sizeof CNTS / sizeof CNTS[0]; j++) {
                        check(k, bval(w, i), 0, cf, CNTS[j]);
                    }
                }
                for (r = 0; r < NR; r++) {
                    check(k, xr() & m, 0, cf, xr() & 0xff);
                }
            } else {
                for (i = 0; i < NB; i++) {
                    for (j = 0; j < NB; j++) {
                        check(k, bval(w, i), bval(w, j), cf, 0);
                    }
                }
                for (r = 0; r < NR; r++) {
                    check(k, xr() & m, xr() & m, cf, 0);
                }
            }
        }
    }
    printf("cases %u\nchecks %ld\nfailures %ld\n", (unsigned)NCASES, checks, fails);
    return fails > 0 ? 1 : 0;
}
"#;

#[test]
fn lift_body_add_emits_flag_helper() {
    // add al, bl
    let body = lift_body(&[0x00, 0xd8], 0x1000).expect("lift add al,bl");
    assert!(body.contains("fle_op_add(8"), "body: {body}");
    assert!(!body.contains("unmodelled"), "body: {body}");
}

#[test]
fn lift_body_rejects_control_transfer() {
    // ret
    assert!(lift_body(&[0xc3], 0x1000).is_err());
}

#[test]
fn flag_helpers_match_host_and_table() {
    let src = crate_dir().join("tests/flags_check.c");
    let (ok, text, stderr) = build_and_run(&src, "flags_check");
    assert!(ok, "flags_check reported failures:\n{text}\n{stderr}");
    assert!(text.contains("failures 0"), "unexpected output:\n{text}");
}

/// setcc and cmovcc for all 16 condition codes. The generated bodies run with
/// every combination of the five flags they read (CF, PF, ZF, SF, OF), and the
/// result is compared with the SDM Jcc condition table.
#[test]
fn lifted_setcc_cmov_match_sdm_formula() {
    // Generated setcc and cmovcc bodies, run for every flag combination of the
    // five flags they read, compared with `cc_formula`.
    let mut src = String::new();
    src.push_str("#include \"fle_runtime.h\"\n\nuint64_t R[16];\nfle_eflags FL;\n\n");
    for cc in 0u8..16 {
        let set_body = lift_body(&[0x0f, 0x90 | cc, 0xc3], 0x1000).expect("setcc");
        let cmov_body = lift_body(&[0x48, 0x0f, 0x40 | cc, 0xc3], 0x1000).expect("cmovcc");
        src.push_str(&format!("static void s{cc}(void) {{ {set_body} }}\n"));
        src.push_str(&format!("static void m{cc}(void) {{ {cmov_body} }}\n"));
    }
    src.push_str("static void (*const S[16])(void) = {");
    for cc in 0u8..16 {
        src.push_str(&format!(" s{cc},"));
    }
    src.push_str(" };\nstatic void (*const M[16])(void) = {");
    for cc in 0u8..16 {
        src.push_str(&format!(" m{cc},"));
    }
    src.push_str(" };\n");
    src.push_str(
        "int main(void) {\n    long checks = 0, fails = 0;\n    unsigned b, cc;\n    for (b = 0; b < 32; b++) {\n        \
         unsigned cf = b & 1, pf = (b >> 1) & 1, zf = (b >> 2) & 1, sf = (b >> 3) & 1, of = (b >> 4) & 1;\n        \
         for (cc = 0; cc < 16; cc++) {\n            unsigned want;\n            \
         switch (cc) {\n            \
         case 0: want = of; break;\n            case 1: want = !of; break;\n            \
         case 2: want = cf; break;\n            case 3: want = !cf; break;\n            \
         case 4: want = zf; break;\n            case 5: want = !zf; break;\n            \
         case 6: want = cf || zf; break;\n            case 7: want = !cf && !zf; break;\n            \
         case 8: want = sf; break;\n            case 9: want = !sf; break;\n            \
         case 10: want = pf; break;\n            case 11: want = !pf; break;\n            \
         case 12: want = sf != of; break;\n            case 13: want = sf == of; break;\n            \
         case 14: want = zf || (sf != of); break;\n            default: want = !zf && (sf == of); break;\n            }\n            \
         FL.cf = cf; FL.pf = pf; FL.af = 0; FL.zf = zf; FL.sf = sf; FL.of = of;\n            \
         R[3] = 0x2200; R[0] = 0x11;\n            S[cc]();\n            checks++;\n            \
         if ((R[3] & 0xff) != (want ? 1u : 0u)) { fails++; if (fails < 20) fprintf(stderr, \"setcc %u b=%u got=%llu\\n\", cc, b, (unsigned long long)(R[3] & 0xff)); }\n            \
         FL.cf = cf; FL.pf = pf; FL.af = 0; FL.zf = zf; FL.sf = sf; FL.of = of;\n            \
         R[0] = 0x1111; R[3] = 0x2222;\n            M[cc]();\n            checks++;\n            \
         if (R[0] != (want ? 0x2222u : 0x1111u)) { fails++; if (fails < 20) fprintf(stderr, \"cmov %u b=%u got=%llu\\n\", cc, b, (unsigned long long)R[0]); }\n        }\n    }\n    \
         printf(\"checks %ld\\nfailures %ld\\n\", checks, fails);\n    return fails > 0;\n}\n",
    );
    let src_path = out_dir().join(format!("{}.c", etxt_common::uniq("setcc_cmov_check")));
    std::fs::write(&src_path, &src).expect("write setcc_cmov_check.c");
    let (ok, text, stderr) = build_and_run(&src_path, "setcc_cmov_check");
    assert!(ok, "setcc/cmov disagree with the SDM formula:\n{text}\n{stderr}");
    assert!(text.contains("failures 0"), "unexpected output:\n{text}");
    assert!(text.contains("checks 1024"), "unexpected output:\n{text}");
}

#[test]
fn lifted_code_matches_host_flags() {
    let cs = cases();
    let c = gen_c(&cs).expect("generate C for lifted cases");
    let src = out_dir().join(format!("{}.c", etxt_common::uniq("lifted_check")));
    std::fs::write(&src, c).expect("write lifted_check.c");
    let (ok, text, stderr) = build_and_run(&src, "lifted_check");
    assert!(ok, "lifted code disagrees with the host:\n{text}\n{stderr}");
    assert!(text.contains("failures 0"), "unexpected output:\n{text}");
    assert!(text.contains(&format!("cases {}", cs.len())), "unexpected output:\n{text}");
}
