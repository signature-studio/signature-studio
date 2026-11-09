/* Authenticode claims of the harness (decision 0117, PROVISIONAL). Header-only helper of lift/harness/two/main_two.c, compiled only into a build with
 * FLE_FRUITY (build_two_native.sh FRUITY=1), like reg_seed.h (decision 0116). It reads the table that tools/authenticode_check.py wrote
 * (lift/harness/seeds/authenticode-claims.tsv) and registers each row with the runtime (fle_trust_claim_add, runtime/fle_vfs.c) under the GUEST path of the file.
 * The runtime answers wintrust!WinVerifyTrust and the crypt32 calls behind the engine's signer-name check only for a registered path with an ESTABLISHED claim;
 * every other path, and every run whose harness registers nothing, stops by name (decision 0117).
 *
 * Off by default. FLE_AUTH_CLAIMS unset or empty: auth_claims_from_env() returns 0 at once, prints nothing and registers nothing, so the run is the one of a
 * harness without this file (the table is empty and WinVerifyTrust stops by name).
 *
 * FLE_AUTH_CLAIMS=<tsv file>. One row per line, 13 TAB-separated fields (the first 11 are the columns of the decision, the last two carry what the runtime needs
 * from the signer certificate; every field was read from the real file by the tool):
 *    file TAB file_sha256 TAB authenticode_digest TAB a_digest TAB b_signature TAB signer_cn TAB issuer_cn TAB not_before TAB not_after TAB timestamp TAB
 *    unverified TAB signer_issuer_der TAB signer_serial
 * Empty lines and lines that start with '#' are ignored.
 *   file                  the file name only (no directory), printable ASCII without \ / : * ? " < > |. Its GUEST path is the directory of FLE_VIRTUAL_DLL_PATH
 *                         (runtime/fle_env.h: the install directory the PM chose in decision 0018) + this name.
 *   file_sha256           64 lower-case hex digits: the SHA-256 of the file the tool checked.
 *   authenticode_digest   <algorithm>:<hex> or "-".
 *   a_digest, b_signature `ok` or `ng`. A row is ESTABLISHED only when both are `ok` and the signer CN, the issuer Name and the serial number are present.
 *   signer_cn, issuer_cn  printable ASCII, or "-". not_before, not_after, timestamp: free text without TAB, "-" for none (reported, never evaluated).
 *   unverified            must be exactly `chain,revocation,time`: a table that claims more than the tool checked is refused.
 *   signer_issuer_der     the signer certificate's issuer Name, DER, hex (even number of digits), or "-". signer_serial: unsigned big-endian hex, or "-".
 * The whole file is read and checked first. One bad line, an unreadable file or the same file name twice stops the harness by name with nothing registered.
 * ATTESTATION: the row whose file name is that of FLE_VIRTUAL_DLL_PATH is compared with the DLL the harness itself loads (SHA-256 of the file named by the harness's
 * first argument); a difference stops the harness. Every other row (the host exe FL64.exe) cannot be compared: the harness holds no copy of that file. The line it
 * prints says so. Success prints "authenticode claims:" lines on stdout; any failure prints "authenticode-claims: ..." on stderr and auth_claims_from_env() returns -1
 * (the caller exits 2 before the guest runs). */
#ifndef FLE_HARNESS_AUTH_CLAIMS_H
#define FLE_HARNESS_AUTH_CLAIMS_H

#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_vfs.h"
#include "vfs_seed.h"

#define AUTH_CLAIMS_ENV "FLE_AUTH_CLAIMS"
#define AC_FIELDS 13
#define AC_LINE_MAX 8192
#define AC_FILE_MAX (1u << 20)
#define AC_ROWS_MAX 8
#define AC_UNVERIFIED "chain,revocation,time"

typedef struct {
    char file[128], sha[65], digest[200], cn[200], icn[200], when[2][64], tstamp[200];
    int established;
    char why[200];
    uint8_t issuer[1024], serial[64];
    size_t issuer_len, serial_len;
    unsigned line;
} ac_row;

static int ac_fail(const char *list, unsigned line, const char *fmt, ...) __attribute__((format(printf, 3, 4)));
static int ac_fail(const char *list, unsigned line, const char *fmt, ...)
{
    va_list ap;
    if (line) fprintf(stderr, "authenticode-claims: %s:%u: ", list, line);
    else fprintf(stderr, "authenticode-claims: %s: ", list);
    va_start(ap, fmt);
    vfprintf(stderr, fmt, ap);
    va_end(ap);
    fputc('\n', stderr);
    return -1;
}

static int ac_hexval(char c)
{
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}

/* Lower-case hex only, an even number of digits; returns the byte count or -1. */
static long ac_unhex(const char *s, uint8_t *out, size_t cap)
{
    size_t n = strlen(s), i;
    if (n % 2 != 0 || n / 2 > cap) return -1;
    for (i = 0; i < n; i += 2) {
        int h = ac_hexval(s[i]), l = ac_hexval(s[i + 1]);
        if (h < 0 || l < 0) return -1;
        out[i / 2] = (uint8_t)(h * 16 + l);
    }
    return (long)(n / 2);
}

static int ac_printable(const char *s)
{
    for (; *s; s++)
        if ((unsigned char)*s < 0x20 || (unsigned char)*s > 0x7E) return 0;
    return 1;
}

static int ac_ieq(const char *a, const char *b)
{
    for (; *a && *b; a++, b++) {
        char x = *a, y = *b;
        if (x >= 'A' && x <= 'Z') x = (char)(x + 32);
        if (y >= 'A' && y <= 'Z') y = (char)(y + 32);
        if (x != y) return 0;
    }
    return *a == *b;
}

static const char *ac_basename_of_dll(void)
{
    const char *p = FLE_VIRTUAL_DLL_PATH, *s = strrchr(p, '\\');
    return s ? s + 1 : p;
}

static void ac_guest_path(const char *file, char *out, size_t cap)
{
    const char *p = FLE_VIRTUAL_DLL_PATH, *s = strrchr(p, '\\');
    size_t dir = s ? (size_t)(s - p) + 1 : 0;
    snprintf(out, cap, "%.*s%s", (int)dir, p, file);
}

static int ac_parse_row(const char *list, unsigned line, char *text, ac_row *r)
{
    char *f[AC_FIELDS + 1];
    int n = 0;
    char *p = text;
    long k;
    memset(r, 0, sizeof *r);
    r->line = line;
    f[n++] = p;
    for (; *p; p++)
        if (*p == '\t') {
            if (n > AC_FIELDS) return ac_fail(list, line, "more than %d TAB-separated fields", AC_FIELDS);
            *p = 0;
            f[n++] = p + 1;
        }
    if (n != AC_FIELDS) return ac_fail(list, line, "expected %d TAB-separated fields, found %d", AC_FIELDS, n);
    for (k = 0; k < n; k++)
        if (!ac_printable(f[k])) return ac_fail(list, line, "field %ld holds a character outside printable ASCII", k + 1);
    if (f[0][0] == 0 || strlen(f[0]) >= sizeof r->file || strpbrk(f[0], "\\/:*?\"<>|")) return ac_fail(list, line, "field 1 (file) is empty, too long, or holds one of \\ / : * ? \" < > |");
    snprintf(r->file, sizeof r->file, "%s", f[0]);
    if (strlen(f[1]) != 64) return ac_fail(list, line, "field 2 (file_sha256) is not 64 hex digits");
    for (k = 0; k < 64; k++)
        if (ac_hexval(f[1][k]) < 0) return ac_fail(list, line, "field 2 (file_sha256) is not 64 lower-case hex digits");
    snprintf(r->sha, sizeof r->sha, "%s", f[1]);
    if (strlen(f[2]) >= sizeof r->digest) return ac_fail(list, line, "field 3 (authenticode_digest) is too long");
    snprintf(r->digest, sizeof r->digest, "%s", f[2]);
    if (strcmp(f[3], "ok") && strcmp(f[3], "ng")) return ac_fail(list, line, "field 4 (a_digest) is neither ok nor ng");
    if (strcmp(f[4], "ok") && strcmp(f[4], "ng")) return ac_fail(list, line, "field 5 (b_signature) is neither ok nor ng");
    if (strlen(f[5]) >= sizeof r->cn || strlen(f[6]) >= sizeof r->icn) return ac_fail(list, line, "field 6 or 7 (signer_cn, issuer_cn) is too long");
    snprintf(r->cn, sizeof r->cn, "%s", strcmp(f[5], "-") ? f[5] : "");
    snprintf(r->icn, sizeof r->icn, "%s", strcmp(f[6], "-") ? f[6] : "");
    if (strlen(f[7]) >= sizeof r->when[0] || strlen(f[8]) >= sizeof r->when[1]) return ac_fail(list, line, "field 8 or 9 (not_before, not_after) is too long");
    snprintf(r->when[0], sizeof r->when[0], "%s", f[7]);
    snprintf(r->when[1], sizeof r->when[1], "%s", f[8]);
    if (strlen(f[9]) >= sizeof r->tstamp) return ac_fail(list, line, "field 10 (timestamp) is too long");
    snprintf(r->tstamp, sizeof r->tstamp, "%s", f[9]);
    if (strcmp(f[10], AC_UNVERIFIED)) return ac_fail(list, line, "field 11 (unverified) is '%s', not '%s': a table must say what was not checked", f[10], AC_UNVERIFIED);
    if (strcmp(f[11], "-")) {
        k = ac_unhex(f[11], r->issuer, sizeof r->issuer);
        if (k <= 0) return ac_fail(list, line, "field 12 (signer_issuer_der) is not lower-case hex of 1..%zu bytes", sizeof r->issuer);
        r->issuer_len = (size_t)k;
    }
    if (strcmp(f[12], "-")) {
        k = ac_unhex(f[12], r->serial, sizeof r->serial);
        if (k <= 0) return ac_fail(list, line, "field 13 (signer_serial) is not lower-case hex of 1..%zu bytes", sizeof r->serial);
        r->serial_len = (size_t)k;
    }
    if (strcmp(f[3], "ok")) snprintf(r->why, sizeof r->why, "a_digest is ng (the digest of the file does not equal the signed digest, or the file could not be hashed)");
    else if (strcmp(f[4], "ok")) snprintf(r->why, sizeof r->why, "b_signature is ng (the signature does not verify with the embedded signer certificate)");
    else if (!r->cn[0] || !r->issuer_len || !r->serial_len) snprintf(r->why, sizeof r->why, "a_digest and b_signature are ok, but the signer CN, issuer or serial number is missing");
    else r->established = 1;
    return 0;
}

static int ac_read_file(const char *list, char **out, size_t *len)
{
    FILE *f = fopen(list, "rb");
    char *buf;
    size_t n;
    if (!f) return ac_fail(list, 0, "cannot open the claims file");
    buf = malloc(AC_FILE_MAX + 1);
    if (!buf) {
        fclose(f);
        return ac_fail(list, 0, "out of memory");
    }
    n = fread(buf, 1, AC_FILE_MAX + 1, f);
    if (ferror(f) || n > AC_FILE_MAX) {
        free(buf);
        fclose(f);
        return ac_fail(list, 0, "cannot read the claims file, or it is larger than %u bytes", AC_FILE_MAX);
    }
    fclose(f);
    buf[n] = 0;
    *out = buf;
    *len = n;
    return 0;
}

/* SHA-256 (lower-case hex) of the file at `path`, or -1. */
static int ac_file_sha(const char *path, char hex[65])
{
    FILE *f = fopen(path, "rb");
    uint8_t *buf = NULL;
    size_t cap = 0, n = 0, got;
    if (!f) return -1;
    for (;;) {
        if (n == cap) {
            uint8_t *nb;
            cap = cap ? cap * 2 : (1u << 20);
            nb = realloc(buf, cap);
            if (!nb) {
                free(buf);
                fclose(f);
                return -1;
            }
            buf = nb;
        }
        got = fread(buf + n, 1, cap - n, f);
        n += got;
        if (got == 0) break;
    }
    if (ferror(f)) {
        free(buf);
        fclose(f);
        return -1;
    }
    fclose(f);
    vs_sha256_hex(buf ? buf : (const uint8_t *)"", n, hex);
    free(buf);
    return 0;
}

static int ac_run(const char *list, const char *fle_path)
{
    char *text = NULL, *p, *eol;
    size_t len, nrows = 0, i;
    unsigned line = 0;
    static ac_row rows[AC_ROWS_MAX];
    char gp[300], hex[65];
    int attested[AC_ROWS_MAX] = { 0 };
    if (ac_read_file(list, &text, &len) != 0) return -1;
    if (memchr(text, 0, len) || memchr(text, '\r', len)) {
        free(text);
        return ac_fail(list, 0, "the file holds a NUL or a carriage return (the format is LF-terminated lines)");
    }
    for (p = text; *p; p = eol ? eol + 1 : p + strlen(p)) {
        char *ln;
        eol = strchr(p, '\n');
        if (eol) *eol = 0;
        line++;
        ln = p;
        if (strlen(ln) >= AC_LINE_MAX) {
            free(text);
            return ac_fail(list, line, "line longer than %d characters", AC_LINE_MAX);
        }
        if (ln[0] == 0 || ln[0] == '#') {
            if (!eol) break;
            continue;
        }
        if (nrows >= AC_ROWS_MAX) {
            free(text);
            return ac_fail(list, line, "more than %d rows", AC_ROWS_MAX);
        }
        if (ac_parse_row(list, line, ln, &rows[nrows]) != 0) {
            free(text);
            return -1;
        }
        for (i = 0; i < nrows; i++)
            if (ac_ieq(rows[i].file, rows[nrows].file)) {
                free(text);
                return ac_fail(list, line, "the file '%s' is listed twice (first at line %u)", rows[nrows].file, rows[i].line);
            }
        nrows++;
        if (!eol) break;
    }
    free(text);
    if (nrows == 0) return ac_fail(list, 0, "the claims file holds no row");
    for (i = 0; i < nrows; i++) {
        if (!ac_ieq(rows[i].file, ac_basename_of_dll())) continue;
        if (ac_file_sha(fle_path, hex) != 0) return ac_fail(list, rows[i].line, "cannot read the DLL the harness loads (%s) to attest the row '%s'", fle_path, rows[i].file);
        if (strcmp(hex, rows[i].sha) != 0)
            return ac_fail(list, rows[i].line, "the row '%s' was made from a file with SHA-256 %s, but the DLL the harness loads (%s) has %s; nothing was registered", rows[i].file,
                           rows[i].sha, fle_path, hex);
        attested[i] = 1;
    }
    printf("authenticode claims: list %s, %zu row(s)\n", list, nrows);
    for (i = 0; i < nrows; i++) {
        fle_trust_claim c;
        ac_guest_path(rows[i].file, gp, sizeof gp);
        memset(&c, 0, sizeof c);
        c.guest_path = gp;
        c.row = rows[i].file;
        c.established = rows[i].established;
        c.why = rows[i].why;
        c.signer_cn = rows[i].cn;
        c.issuer_der = rows[i].issuer;
        c.issuer_len = rows[i].issuer_len;
        c.serial_be = rows[i].serial;
        c.serial_len = rows[i].serial_len;
        if (fle_trust_claim_add(&c) != 0) return ac_fail(list, rows[i].line, "the runtime refused the claim for %s", gp);
        printf("authenticode claims: %s <- row '%s' established=%s%s%s signer CN '%s' issuer CN '%s' valid %s .. %s timestamp %s file sha256 %s digest %s; %s\n", gp, rows[i].file,
               rows[i].established ? "yes" : "NO", rows[i].established ? "" : " (", rows[i].established ? "" : rows[i].why, rows[i].cn, rows[i].icn, rows[i].when[0], rows[i].when[1],
               rows[i].tstamp, rows[i].sha, rows[i].digest,
               attested[i] ? "attested: the SHA-256 equals that of the DLL the harness loaded" : "NOT attested: the harness holds no copy of this file");
    }
    printf("authenticode claims: not verified (by the tool or here): %s. done, %zu claim(s) registered\n", AC_UNVERIFIED, fle_trust_claim_count());
    return 0;
}

static int auth_claims_from_env(const char *fle_path)
{
    const char *env = getenv(AUTH_CLAIMS_ENV);
    if (!env || !env[0]) return 0;
    return ac_run(env, fle_path);
}

#endif /* FLE_HARNESS_AUTH_CLAIMS_H */
