/* A minimal CryptoAPI for hashing: MD5 only (decision 0183, PROVISIONAL). See fle_crypt.h. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_crypt.h"

#define PROV_RSA_FULL 1u        /* Learn: CryptAcquireContext, the provider types table */
#define PROV_RSA_AES 24u
#define CRYPT_VERIFYCONTEXT 0xF0000000u   /* Learn: CryptAcquireContext, dwFlags */
#define CALG_MD5 0x8003u        /* Learn: ALG_ID (wincrypt.h value) */
#define HP_ALGID 0x0001u        /* Learn: CryptGetHashParam, dwParam */
#define HP_HASHVAL 0x0002u
#define HP_HASHSIZE 0x0004u
#define ERROR_MORE_DATA 234u
#define NTE_BAD_UID 0x80090001u    /* wincrypt/winerror.h; the pages name the symbols */
#define NTE_BAD_HASH 0x80090002u
#define NTE_BAD_HASH_STATE 0x8009000Cu   /* "NTE_BAD_HASH_STATE": data added after the hash value was read */
#define MD5_LEN 16u

/* ---- MD5 (RFC 1321). T[i] = floor(2^32 * abs(sin(i + 1))), the table of section 3.4 ---- */
static const uint32_t T[64] = { 0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1, 0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391 };
static const uint8_t S[64] = { 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
                               4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21 };

typedef struct {
    uint32_t a, b, c, d;
    uint64_t len;       /* bytes hashed so far */
    uint8_t buf[64];
    size_t used;
} md5_ctx;

static uint32_t rol(uint32_t x, unsigned n) { return (x << n) | (x >> (32u - n)); }

static void md5_block(md5_ctx *c, const uint8_t *p)
{
    uint32_t M[16], A = c->a, B = c->b, C = c->c, D = c->d;
    unsigned i;
    for (i = 0; i < 16; i++) M[i] = (uint32_t)p[4 * i] | ((uint32_t)p[4 * i + 1] << 8) | ((uint32_t)p[4 * i + 2] << 16) | ((uint32_t)p[4 * i + 3] << 24);
    for (i = 0; i < 64; i++) {
        uint32_t f, tmp;
        unsigned g;
        if (i < 16) { f = (B & C) | (~B & D); g = i; }
        else if (i < 32) { f = (D & B) | (~D & C); g = (5 * i + 1) & 15u; }
        else if (i < 48) { f = B ^ C ^ D; g = (3 * i + 5) & 15u; }
        else { f = C ^ (B | ~D); g = (7 * i) & 15u; }
        tmp = D; D = C; C = B;
        B = B + rol(A + f + T[i] + M[g], S[i]);
        A = tmp;
    }
    c->a += A; c->b += B; c->c += C; c->d += D;
}

static void md5_init(md5_ctx *c) { c->a = 0x67452301u; c->b = 0xefcdab89u; c->c = 0x98badcfeu; c->d = 0x10325476u; c->len = 0; c->used = 0; }

static void md5_update(md5_ctx *c, const uint8_t *p, size_t n)
{
    c->len += n;
    while (n > 0) {
        size_t k = 64 - c->used;
        if (k > n) k = n;
        memcpy(c->buf + c->used, p, k);
        c->used += k; p += k; n -= k;
        if (c->used == 64) { md5_block(c, c->buf); c->used = 0; }
    }
}

static void md5_final(md5_ctx *c, uint8_t out[16])
{
    uint64_t bits = c->len * 8u;
    uint8_t pad[72] = { 0x80 };
    size_t padlen = (c->used < 56) ? 56 - c->used : 120 - c->used;
    unsigned i;
    for (i = 0; i < 8; i++) pad[padlen + i] = (uint8_t)(bits >> (8 * i));
    md5_update(c, pad, padlen + 8);
    for (i = 0; i < 4; i++) { out[i] = (uint8_t)(c->a >> (8 * i)); out[4 + i] = (uint8_t)(c->b >> (8 * i)); out[8 + i] = (uint8_t)(c->c >> (8 * i)); out[12 + i] = (uint8_t)(c->d >> (8 * i)); }
}

void fle_md5(const uint8_t *data, size_t n, uint8_t out[16]) { md5_ctx c; md5_init(&c); md5_update(&c, data, n); md5_final(&c, out); }

/* ---- handles: one table for providers and hashes, handle = FLE_CRYPT_HANDLE_BASE + 4 * (index + 1), never reused ---- */
typedef struct {
    int kind;           /* 0 free/destroyed, 1 provider, 2 hash */
    md5_ctx ctx;
    int finished;       /* the hash value was read: no more data (Learn, CryptHashData remarks) */
    uint8_t value[16];
} cr_ent;

static cr_ent *ents;
static uint64_t ents_n, ents_cap;
static unsigned live_prov, live_hash;

unsigned fle_crypt_live_providers(void) { return live_prov; }
unsigned fle_crypt_live_hashes(void) { return live_hash; }

static uint64_t new_ent(const char *api, int kind)
{
    if (ents_n == ents_cap) {
        uint64_t nc = ents_cap ? ents_cap * 2 : 16;
        cr_ent *n = realloc(ents, nc * sizeof *n);
        if (!n) fle_stop(api, "out of memory");
        ents = n;
        ents_cap = nc;
    }
    if (ents_n + 2 >= FLE_CRYPT_REGION_SIZE / 4u) fle_stop(api, "crypt handle region exhausted");
    memset(&ents[ents_n], 0, sizeof ents[ents_n]);
    ents[ents_n].kind = kind;
    ents_n++;
    return FLE_CRYPT_HANDLE_BASE + 4u * ents_n;
}

/* the live entry of a kind for a handle value; NULL when the value is 0, destroyed or of another kind (the caller fails); a value in no region of this file stops */
static cr_ent *get_ent(const char *api, uint64_t h, int kind)
{
    uint64_t off, i;
    if (h == 0) return NULL;
    if (h <= FLE_CRYPT_HANDLE_BASE || h >= FLE_CRYPT_HANDLE_BASE + FLE_CRYPT_REGION_SIZE || ((h - FLE_CRYPT_HANDLE_BASE) % 4u)) fle_stop(api, "handle was not issued by this runtime's CryptoAPI");
    off = h - FLE_CRYPT_HANDLE_BASE;
    i = off / 4u - 1u;
    if (i >= ents_n) fle_stop(api, "handle was not issued by this runtime's CryptoAPI");
    return ents[i].kind == kind ? &ents[i] : NULL;
}

/* kernel32 last error for a CryptoAPI failure (the pages: "For extended error information, call GetLastError", with the NTE_* / ERROR_* names) */
static void fail(uint32_t code) { fle_set_last_error(code); R[0] = 0; }

/* advapi32!CryptAcquireContextA(HCRYPTPROV *phProv rcx, LPCSTR szContainer rdx, LPCSTR szProvider r8, DWORD dwProvType r9d, DWORD dwFlags [rsp+0x28]): BOOL.
 * Learn: with CRYPT_VERIFYCONTEXT "the application has no access to persisted private keys" and szContainer "must be NULL"; PROV_RSA_FULL (1) is the default provider type, PROV_RSA_AES (24) the AES provider; a NULL szProvider
 * means the default provider of the type. Implemented: NULL container, NULL provider name, those two types, flags exactly CRYPT_VERIFYCONTEXT (0xF0000000). Everything else stops by name (S-wrp1-crypt-scope). */
void fle_imp_CryptAcquireContextA(void)
{
    static const char api[] = "CryptAcquireContextA";
    uint64_t pp = R[1];
    uint32_t type = (uint32_t)R[9], flags = (uint32_t)fle_arg(5);
    if (pp == 0) fle_stop(api, "phProv is NULL");
    if (R[2] != 0) fle_stop(api, "szContainer is not NULL (named key containers are not modelled)");
    if (R[8] != 0) fle_stop(api, "szProvider is not NULL (named providers are not modelled)");
    if (type != PROV_RSA_FULL && type != PROV_RSA_AES) fle_stop(api, "dwProvType is not PROV_RSA_FULL / PROV_RSA_AES");
    if (flags != CRYPT_VERIFYCONTEXT) fle_stop(api, "dwFlags is not exactly CRYPT_VERIFYCONTEXT (persisted containers are not modelled)");
    fle_st(pp, 8, new_ent(api, 1));
    live_prov++;
    R[0] = 1;
}

/* advapi32!CryptReleaseContext(HCRYPTPROV hProv rcx, DWORD dwFlags edx): BOOL. Learn: dwFlags "is reserved for future use and must be zero"; the handle is released. A released or foreign provider: FALSE + NTE_BAD_UID. */
void fle_imp_CryptReleaseContext(void)
{
    static const char api[] = "CryptReleaseContext";
    cr_ent *e;
    if ((uint32_t)R[2] != 0) fle_stop(api, "dwFlags is not 0");
    e = get_ent(api, R[1], 1);
    if (!e) { fail(NTE_BAD_UID); return; }
    e->kind = 0;
    live_prov--;
    R[0] = 1;
}

/* advapi32!CryptCreateHash(HCRYPTPROV hProv rcx, ALG_ID Algid edx, HCRYPTKEY hKey r8, DWORD dwFlags r9d, HCRYPTHASH *phHash [rsp+0x28]): BOOL. Learn: hKey "must be zero" for a nonkeyed algorithm such as MD5, dwFlags "is reserved ... must be zero".
 * Only CALG_MD5 (0x8003) is implemented; another ALG_ID stops by name (S-wrp1-crypt-scope). A bad provider handle: FALSE + NTE_BAD_UID. */
void fle_imp_CryptCreateHash(void)
{
    static const char api[] = "CryptCreateHash";
    uint64_t pph = fle_arg(5);
    cr_ent *e;
    uint64_t h;
    if (pph == 0) fle_stop(api, "phHash is NULL");
    if ((uint32_t)R[2] != CALG_MD5) fle_stop(api, "Algid is not CALG_MD5 (only MD5 is implemented)");
    if (R[8] != 0) fle_stop(api, "hKey is not 0 (keyed hashes are not implemented)");
    if ((uint32_t)R[9] != 0) fle_stop(api, "dwFlags is not 0");
    e = get_ent(api, R[1], 1);
    if (!e) { fail(NTE_BAD_UID); return; }
    h = new_ent(api, 2);
    md5_init(&ents[(h - FLE_CRYPT_HANDLE_BASE) / 4u - 1u].ctx);
    fle_st(pph, 8, h);
    live_hash++;
    R[0] = 1;
}

/* advapi32!CryptHashData(HCRYPTHASH hHash rcx, const BYTE *pbData rdx, DWORD dwDataLen r8d, DWORD dwFlags r9d): BOOL. Learn: adds data to the hash; "Once CryptGetHashParam has been called to get the hash value, no more data can be added"
 * (NTE_BAD_HASH_STATE); dwFlags 0 (CRYPT_USERDATA stops). A bad hash handle: FALSE + NTE_BAD_HASH. With FLE_CRYPT_TRACE the data is printed. */
void fle_imp_CryptHashData(void)
{
    static const char api[] = "CryptHashData";
    cr_ent *e;
    uint64_t p = R[2];
    uint32_t n = (uint32_t)R[8];
    uint32_t i;
    uint8_t *tmp;
    if ((uint32_t)R[9] != 0) fle_stop(api, "dwFlags is not 0");
    e = get_ent(api, R[1], 2);
    if (!e) { fail(NTE_BAD_HASH); return; }
    if (e->finished) { fail(NTE_BAD_HASH_STATE); return; }
    if (n != 0 && p == 0) fle_stop(api, "pbData is NULL with a nonzero length");
    tmp = malloc(n ? n : 1);
    if (!tmp) fle_stop(api, "out of memory");
    for (i = 0; i < n; i++) tmp[i] = (uint8_t)fle_ld(p + i, 1);
    md5_update(&e->ctx, tmp, n);
    free(tmp);
    R[0] = 1;
}

/* advapi32!CryptGetHashParam(HCRYPTHASH hHash rcx, DWORD dwParam edx, BYTE *pbData r8, DWORD *pdwDataLen r9, DWORD dwFlags [rsp+0x28]): BOOL. Learn: HP_ALGID (the ALG_ID, a DWORD), HP_HASHSIZE (the hash length in bytes, a DWORD: 16 for MD5),
 * HP_HASHVAL (the hash value, 16 bytes; reading it finishes the hash). "If pbData is NULL, the required size is returned in *pdwDataLen"; a too small buffer sets ERROR_MORE_DATA and returns the required size in *pdwDataLen. dwFlags 0.
 * Other dwParam values stop by name. A bad hash handle: FALSE + NTE_BAD_HASH. */
void fle_imp_CryptGetHashParam(void)
{
    static const char api[] = "CryptGetHashParam";
    cr_ent *e;
    uint32_t param = (uint32_t)R[2], need, have;
    uint64_t pb = R[8], pl = R[9];
    uint8_t dw[4];
    const uint8_t *src;
    if ((uint32_t)fle_arg(5) != 0) fle_stop(api, "dwFlags is not 0");
    if (pl == 0) fle_stop(api, "pdwDataLen is NULL");
    e = get_ent(api, R[1], 2);
    if (!e) { fail(NTE_BAD_HASH); return; }
    if (param == HP_HASHVAL) { need = MD5_LEN; src = NULL; }
    else if (param == HP_HASHSIZE) { need = 4; dw[0] = MD5_LEN; dw[1] = dw[2] = dw[3] = 0; src = dw; }
    else if (param == HP_ALGID) { need = 4; dw[0] = CALG_MD5 & 0xff; dw[1] = CALG_MD5 >> 8; dw[2] = dw[3] = 0; src = dw; }
    else { fle_stop(api, "dwParam is none of HP_ALGID / HP_HASHSIZE / HP_HASHVAL"); return; }
    have = (uint32_t)fle_ld(pl, 4);
    if (pb == 0) { fle_st(pl, 4, need); R[0] = 1; return; }
    if (have < need) { fle_st(pl, 4, need); fail(ERROR_MORE_DATA); return; }
    if (param == HP_HASHVAL) {
        if (!e->finished) {
            md5_ctx c = e->ctx;
            md5_final(&c, e->value);
            e->finished = 1;
        }
        src = e->value;
    }
    {
        uint32_t i;
        for (i = 0; i < need; i++) fle_st(pb + i, 1, src[i]);
    }
    fle_st(pl, 4, need);
    R[0] = 1;
}

/* advapi32!CryptDestroyHash(HCRYPTHASH hHash rcx): BOOL. Learn: destroys the hash object; a bad handle: FALSE + NTE_BAD_HASH. */
void fle_imp_CryptDestroyHash(void)
{
    static const char api[] = "CryptDestroyHash";
    cr_ent *e = get_ent(api, R[1], 2);
    if (!e) { fail(NTE_BAD_HASH); return; }
    e->kind = 0;
    live_hash--;
    R[0] = 1;
}
