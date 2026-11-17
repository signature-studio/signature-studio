/* A minimal CryptoAPI (advapi32) for hashing: MD5 only (decision 0183, PROVISIONAL, PM's range 2026-11-03). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9) and the stack
 * ([rsp+0x28] is the fifth), the Win32 return value in R[0]; the guest return address stays on the stack. Reached only through rows (harness/seeds/wrp1-vst3-imports.txt): a build without the rows stops at the import thunk.
 * What is implemented (Learn: CryptAcquireContext, CryptCreateHash, CryptHashData, CryptGetHashParam, CryptDestroyHash, CryptReleaseContext; RFC 1321 for the algorithm):
 *   CryptAcquireContextA(phProv, szContainer = NULL, szProvider = NULL, dwProvType = PROV_RSA_FULL | PROV_RSA_AES, dwFlags = CRYPT_VERIFYCONTEXT)  -> a provider handle
 *   CryptCreateHash(hProv, CALG_MD5, 0, 0, phHash), CryptHashData(hHash, pbData, cb, 0), CryptGetHashParam(hHash, HP_ALGID | HP_HASHSIZE | HP_HASHVAL, ...), CryptDestroyHash(hHash), CryptReleaseContext(hProv, 0)
 * Everything else (another algorithm, a keyed hash, a named container, other flags) stops by name. Nothing here knows about licences: the function hashes what the caller gives it. With FLE_CRYPT_TRACE set, the data of every
 * CryptHashData is printed on stderr (length, up to 96 bytes in hex and as text) so that the caller's input can be identified. */
#ifndef FLE_CRYPT_H
#define FLE_CRYPT_H

#include <stddef.h>
#include <stdint.h>

#define FLE_CRYPT_HANDLE_BASE UINT64_C(0x7FFAD0000000)
#define FLE_CRYPT_REGION_SIZE UINT64_C(0x10000000)

void fle_imp_CryptAcquireContextA(void);
void fle_imp_CryptReleaseContext(void);
void fle_imp_CryptCreateHash(void);
void fle_imp_CryptHashData(void);
void fle_imp_CryptGetHashParam(void);
void fle_imp_CryptDestroyHash(void);

/* MD5 of a buffer (RFC 1321), 16 bytes in out. For the tests. */
void fle_md5(const uint8_t *data, size_t n, uint8_t out[16]);
/* live provider handles / live hash handles (for tests) */
unsigned fle_crypt_live_providers(void);
unsigned fle_crypt_live_hashes(void);

#endif
