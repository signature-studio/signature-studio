/* Self-test of lift/harness/authenticode_claims.h (decision 0117). Links the virtual file system / registry / environment of the runtime (the claims table is in
 * fle_vfs.c) and nothing of the lifted code, so it builds and runs in seconds. Writes only into the directory that is its first argument (which must exist). The claims
 * table is a process-wide singleton of at most 8 entries and only grows, so the checks run in one order: the refusals first (each must leave the table empty), then the
 * accepted files (each adds claims that stay). Stdout and stderr of the code under test are captured into files and read back.
 *
 *   cd lift && clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I runtime -I harness \
 *       runtime/fle_vfs.c runtime/fle_reg.c runtime/fle_env.c runtime/fle_imports.c runtime/fle_modules.c runtime/fle_string.c \
 *       harness/seeds/authenticode_claims_selftest.c -o $OUT/authenticode_claims_selftest && $OUT/authenticode_claims_selftest <existing empty dir>
 *
 * Prints "ok <name>" or "FAIL <name>" per check and "<n> failure(s)" last; the exit code is the number of failures (0 = all passed). Expected values are written from the
 * format description in authenticode_claims.h, not read back from the code under test. */
#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "authenticode_claims.h"

uint64_t R[16];

static int failures;
static char dir[900];

static void check(int cond, const char *name)
{
    printf("%s %s\n", cond ? "ok  " : "FAIL", name);
    if (!cond) failures++;
}

static void write_file(const char *name, const void *data, size_t n)
{
    char path[1024];
    FILE *f;
    snprintf(path, sizeof path, "%s/%s", dir, name);
    f = fopen(path, "wb");
    if (!f || fwrite(data, 1, n, f) != n) {
        fprintf(stderr, "selftest: cannot write %s\n", path);
        exit(99);
    }
    fclose(f);
}

static char *read_file(const char *name)
{
    char path[1024];
    FILE *f;
    char *buf;
    size_t n;
    snprintf(path, sizeof path, "%s/%s", dir, name);
    f = fopen(path, "rb");
    if (!f) return strdup("");
    buf = malloc(1 << 20);
    n = fread(buf, 1, (1 << 20) - 1, f);
    buf[n] = 0;
    fclose(f);
    return buf;
}

static void path_of(char *out, size_t cap, const char *name)
{
    snprintf(out, cap, "%s/%s", dir, name);
}

/* runs auth_claims_from_env(dll) with FLE_AUTH_CLAIMS = <dir>/<list> (or unset when list is NULL) and returns its result; the output of the call is in out/err */
static char *last_out, *last_err;
static int run(const char *list, const char *dll)
{
    char lp[1024];
    int rc, so, se;
    fflush(stdout);
    fflush(stderr);
    so = dup(1);
    se = dup(2);
    {
        char p1[1024], p2[1024];
        int f1, f2;
        path_of(p1, sizeof p1, "captured.out");
        path_of(p2, sizeof p2, "captured.err");
        f1 = open(p1, O_WRONLY | O_CREAT | O_TRUNC, 0644);
        f2 = open(p2, O_WRONLY | O_CREAT | O_TRUNC, 0644);
        dup2(f1, 1);
        dup2(f2, 2);
        close(f1);
        close(f2);
    }
    if (list) {
        path_of(lp, sizeof lp, list);
        setenv(AUTH_CLAIMS_ENV, lp, 1);
    } else {
        unsetenv(AUTH_CLAIMS_ENV);
    }
    {
        char dp[1024];
        path_of(dp, sizeof dp, dll);
        rc = auth_claims_from_env(dp);
    }
    fflush(stdout);
    fflush(stderr);
    dup2(so, 1);
    dup2(se, 2);
    close(so);
    close(se);
    free(last_out);
    free(last_err);
    last_out = read_file("captured.out");
    last_err = read_file("captured.err");
    return rc;
}

static const char *UNV = "chain,revocation,time";
static const char ISS[] = "3069310b300906035504061302555331173015060355040a130e44696769436572742c20496e632e3141";
static char dll_sha[65];

/* a row; fields are given as strings so that a test can break exactly one of them */
static void row(char *out, size_t cap, const char *file, const char *sha, const char *a, const char *b, const char *cn, const char *unv, const char *issuer, const char *serial)
{
    snprintf(out, cap, "%s\t%s\tsha256:00\t%s\t%s\t%s\tDigiCert Test CA\t2026-07-28T00:00:00Z\t2027-10-21T23:59:59Z\trfc3161@2026-09-29T14:31:53Z\t%s\t%s\t%s\n", file, sha, a, b, cn, unv, issuer, serial);
}

static void refused(const char *name, const char *text, const char *must_say)
{
    char label[200];
    int rc;
    write_file("list.tsv", text, strlen(text));
    rc = run("list.tsv", "dll.bin");
    snprintf(label, sizeof label, "refused: %s", name);
    check(rc == -1 && fle_trust_claim_count() == 0 && strstr(last_err, must_say) != NULL && strstr(last_out, "authenticode claims:") == NULL, label);
    if (rc != -1 || strstr(last_err, must_say) == NULL) fprintf(stderr, "  rc %d err: %s\n", rc, last_err);
}

int main(int argc, char **argv)
{
    char r1[2048], r2[2048], text[8192], hex[65];
    static const char dll_bytes[] = "not a real DLL: the harness only hashes the file it loads";
    int rc, i;

    if (argc < 2) {
        fprintf(stderr, "usage: %s <existing empty dir>\n", argv[0]);
        return 99;
    }
    snprintf(dir, sizeof dir, "%s", argv[1]);
    (void)vfs_seed_from_env; /* vfs_seed.h (for its SHA-256) defines it; this test does not seed files */
    write_file("dll.bin", dll_bytes, sizeof dll_bytes - 1);
    vs_sha256_hex((const uint8_t *)dll_bytes, sizeof dll_bytes - 1, dll_sha);
    vs_sha256_hex((const uint8_t *)"abc", 3, hex);
    check(strcmp(hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad") == 0, "the SHA-256 used for the attestation is FIPS 180-4's (abc)");
    check(strlen(dll_sha) == 64, "sha of the fake DLL is 64 hex digits");

    /* ---- off by default ---- */
    check(run(NULL, "dll.bin") == 0 && last_out[0] == 0 && last_err[0] == 0 && fle_trust_claim_count() == 0, "FLE_AUTH_CLAIMS unset: returns 0, prints nothing, registers nothing");
    setenv(AUTH_CLAIMS_ENV, "", 1);
    check(auth_claims_from_env("dll.bin") == 0 && fle_trust_claim_count() == 0, "FLE_AUTH_CLAIMS empty: returns 0, registers nothing");
    unsetenv(AUTH_CLAIMS_ENV);

    /* ---- refusals: one broken thing each; nothing may be registered ---- */
    rc = run("missing.tsv", "dll.bin");
    check(rc == -1 && strstr(last_err, "cannot open the claims file") && fle_trust_claim_count() == 0, "refused: a file that does not exist");
    refused("only comments", "# nothing\n\n# more\n", "holds no row");
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
    {
        char r[2048];
        snprintf(r, sizeof r, "%s", r1);
        *strrchr(r, '\t') = 0;   /* 12 fields */
        refused("12 fields", r, "expected 13 TAB-separated fields, found 12");
        snprintf(r, sizeof r, "%.*s\textra\n", (int)(strlen(r1) - 1), r1);
        refused("14 fields", r, "expected 13 TAB-separated fields, found 14");
        snprintf(r, sizeof r, "%.*s\textra\textra\n", (int)(strlen(r1) - 1), r1);
        refused("15 fields", r, "more than 13 TAB-separated fields");
    }
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "ok", "Image Line N\xc3\x89", UNV, ISS, "0afc");
    refused("a character outside printable ASCII", r1, "outside printable ASCII");
    row(r1, sizeof r1, "dir\\x.exe", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
    refused("a backslash in the file name", r1, "field 1 (file)");
    row(r1, sizeof r1, "dir/x.exe", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
    refused("a slash in the file name", r1, "field 1 (file)");
    row(r1, sizeof r1, "", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
    refused("an empty file name", r1, "field 1 (file)");
    row(r1, sizeof r1, "x.exe", "abcd", "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
    refused("a short file hash", r1, "field 2 (file_sha256) is not 64 hex digits");
    {
        char up[65];
        for (i = 0; i < 64; i++) up[i] = (dll_sha[i] >= 'a' && dll_sha[i] <= 'f') ? (char)(dll_sha[i] - 32) : dll_sha[i];
        up[0] = 'A';
        up[64] = 0;
        row(r1, sizeof r1, "x.exe", up, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
        refused("an upper-case file hash", r1, "field 2 (file_sha256) is not 64 lower-case hex digits");
    }
    row(r1, sizeof r1, "x.exe", dll_sha, "OK", "ok", "Image Line NV", UNV, ISS, "0afc");
    refused("a_digest 'OK'", r1, "field 4 (a_digest) is neither ok nor ng");
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "", "Image Line NV", UNV, ISS, "0afc");
    refused("b_signature empty", r1, "field 5 (b_signature) is neither ok nor ng");
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "ok", "Image Line NV", "chain,revocation", ISS, "0afc");
    refused("unverified lists less than the tool does not check", r1, "a table must say what was not checked");
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "ok", "Image Line NV", "", ISS, "0afc");
    refused("unverified empty", r1, "a table must say what was not checked");
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "ok", "Image Line NV", "time,chain,revocation", ISS, "0afc");
    refused("unverified in another order", r1, "a table must say what was not checked");
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "ok", "Image Line NV", UNV, "3069310", "0afc");
    refused("issuer hex with an odd number of digits", r1, "field 12 (signer_issuer_der)");
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "ok", "Image Line NV", UNV, "30693A", "0afc");
    refused("issuer hex in upper case", r1, "field 12 (signer_issuer_der)");
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0xafc");
    refused("serial that is not hex", r1, "field 13 (signer_serial)");
    row(r1, sizeof r1, "FL64.exe", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
    row(r2, sizeof r2, "fl64.EXE", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
    snprintf(text, sizeof text, "%s%s", r1, r2);
    refused("the same file name twice (without regard to case)", text, "is listed twice (first at line 1)");
    row(r1, sizeof r1, "x.exe", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
    {
        char crlf[4096];
        snprintf(crlf, sizeof crlf, "%.*s\r\n", (int)(strlen(r1) - 1), r1);
        refused("CRLF line endings", crlf, "NUL or a carriage return");
    }
    text[0] = 0;
    for (i = 0; i < 9; i++) {
        char nm[32];
        snprintf(nm, sizeof nm, "f%d.dll", i);
        row(r1, sizeof r1, nm, dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
        strcat(text, r1);
    }
    refused("nine rows (the table holds eight)", text, "more than 8 rows");
    /* attestation: the row for the file name of FLE_VIRTUAL_DLL_PATH is compared with the DLL the harness loads */
    {
        char other[65];
        memcpy(other, dll_sha, 65);
        other[0] = other[0] == '0' ? '1' : '0';
        row(r1, sizeof r1, "FLEngine_x64.dll", other, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
        refused("a row for the engine DLL made from another file than the one loaded", r1, "was made from a file with SHA-256");
        row(r1, sizeof r1, "flengine_x64.DLL", other, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
        refused("the same, with the name in another case", r1, "was made from a file with SHA-256");
    }
    row(r1, sizeof r1, "FLEngine_x64.dll", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc");
    write_file("list.tsv", r1, strlen(r1));
    rc = run("list.tsv", "no-such-dll.bin");
    check(rc == -1 && strstr(last_err, "cannot read the DLL the harness loads") && fle_trust_claim_count() == 0, "refused: the DLL the harness loads cannot be read to attest its row");

    /* ---- accepted: the engine DLL (attested) and the host exe (not attested), in one file ---- */
    row(r1, sizeof r1, "FLEngine_x64.dll", dll_sha, "ok", "ok", "Image Line NV", UNV, ISS, "0afc3aad2fa7d5b5e9c267ed4a67eb12");
    row(r2, sizeof r2, "FL64.exe", "4c839f24da13885fef7190d81c792fe2a2e0248dcfce9cb5f400a48ae0520c65", "ok", "ok", "Image Line NV", UNV, ISS, "0afc3aad2fa7d5b5e9c267ed4a67eb12");
    snprintf(text, sizeof text, "# a comment\n\n%s# another\n%s", r1, r2);
    write_file("good.tsv", text, strlen(text));
    rc = run("good.tsv", "dll.bin");
    check(rc == 0, "accepted: a good file registers and returns 0");
    check(fle_trust_claim_count() == 2, "two claims are registered");
    check(strstr(last_out, "authenticode claims: C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll <- row 'FLEngine_x64.dll' established=yes") != NULL,
          "the guest path is the directory of FLE_VIRTUAL_DLL_PATH plus the file name (engine)");
    check(strstr(last_out, "authenticode claims: C:\\Program Files\\Image-Line\\FL Studio 21\\FL64.exe <- row 'FL64.exe' established=yes") != NULL, "the guest path of the host exe");
    check(strstr(last_out, "attested: the SHA-256 equals that of the DLL the harness loaded") != NULL, "the engine row is reported as attested");
    check(strstr(last_out, "NOT attested: the harness holds no copy of this file") != NULL, "the exe row is reported as NOT attested");
    check(strstr(last_out, "not verified (by the tool or here): chain,revocation,time. done, 2 claim(s) registered") != NULL, "the closing line names what was not verified");
    check(last_err[0] == 0, "nothing on stderr");
    check(strstr(last_out, "signer CN 'Image Line NV' issuer CN 'DigiCert Test CA' valid 2026-07-28T00:00:00Z .. 2027-10-21T23:59:59Z timestamp rfc3161@2026-09-29T14:31:53Z") != NULL,
          "the line shows the signer, the issuer, the validity and the timestamp as read (never judged)");
    /* the runtime answers from them: WinVerifyTrust on both paths returns 0 (a WINTRUST_DATA of the shape the guest builds) */
    {
        static uint64_t stk[16];
        uint8_t wvt[88], fi[32];
        uint8_t guid[16] = { 0x6b, 0xc5, 0xaa, 0x00, 0x44, 0xcd, 0xd0, 0x11, 0x8c, 0xc2, 0x00, 0xc0, 0x4f, 0xc2, 0x95, 0xee };
        const char *paths[2] = { "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll", "C:\\Program Files\\Image-Line\\FL Studio 21\\FL64.exe" };
        int k;
        for (k = 0; k < 2; k++) {
            uint16_t w[128];
            uint32_t v;
            uint64_t p;
            size_t n = strlen(paths[k]), j;
            for (j = 0; j <= n; j++) w[j] = (uint8_t)paths[k][j];
            memset(wvt, 0, sizeof wvt);
            memset(fi, 0, sizeof fi);
            v = 32; memcpy(fi, &v, 4);
            p = (uint64_t)(uintptr_t)w; memcpy(fi + 8, &p, 8);
            v = 88; memcpy(wvt, &v, 4);
            v = 2; memcpy(wvt + 24, &v, 4);
            v = 1; memcpy(wvt + 32, &v, 4);
            p = (uint64_t)(uintptr_t)fi; memcpy(wvt + 40, &p, 8);
            memset(stk, 0, sizeof stk);
            R[4] = (uint64_t)(uintptr_t)stk;
            R[1] = 0; R[2] = (uint64_t)(uintptr_t)guid; R[8] = (uint64_t)(uintptr_t)wvt; R[9] = 0; R[0] = 0xBAD;
            fle_imp_WinVerifyTrust();
            check(R[0] == 0, k == 0 ? "the runtime answers WinVerifyTrust 0 for the engine path registered by the harness" : "the runtime answers WinVerifyTrust 0 for the exe path registered by the harness");
        }
    }
    /* a second registration of the same file names: the runtime refuses the duplicate path (the table is filled once per process) */
    rc = run("good.tsv", "dll.bin");
    check(rc == -1 && strstr(last_err, "the runtime refused the claim for") != NULL && fle_trust_claim_count() == 2, "registering the same paths again is refused by the runtime and adds nothing");

    /* ---- rows that do not hold are registered as not established, with the reason ---- */
    row(r1, sizeof r1, "BadDigest.dll", "1111111111111111111111111111111111111111111111111111111111111111", "ng", "ok", "Image Line NV", UNV, ISS, "0afc");
    row(r2, sizeof r2, "BadSig.dll", "2222222222222222222222222222222222222222222222222222222222222222", "ok", "ng", "Image Line NV", UNV, ISS, "0afc");
    snprintf(text, sizeof text, "%s%s", r1, r2);
    {
        char r3[2048];
        row(r3, sizeof r3, "NoName.dll", "3333333333333333333333333333333333333333333333333333333333333333", "ok", "ok", "-", UNV, ISS, "0afc");
        strcat(text, r3);
    }
    write_file("notheld.tsv", text, strlen(text));
    rc = run("notheld.tsv", "dll.bin");
    check(rc == 0 && fle_trust_claim_count() == 5, "rows that do not hold are registered (3 more claims)");
    check(strstr(last_out, "row 'BadDigest.dll' established=NO (a_digest is ng") != NULL, "a_digest ng: not established, with the reason");
    check(strstr(last_out, "row 'BadSig.dll' established=NO (b_signature is ng") != NULL, "b_signature ng: not established, with the reason");
    check(strstr(last_out, "row 'NoName.dll' established=NO (a_digest and b_signature are ok, but the signer CN, issuer or serial number is missing") != NULL,
          "ok/ok without a signer CN is not established either");
    check(strstr(last_out, "established=yes") == NULL, "and none of them is reported as established");

    printf("%d failure(s)\n", failures);
    return failures;
}
