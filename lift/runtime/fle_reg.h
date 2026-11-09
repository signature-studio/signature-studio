/* In-memory virtual registry and the advapi32 answers (docs/decisions/0018-virtual-fs-reg-env.md). The host
 * registry is never read: the five predefined roots exist and are empty, and only what the guest creates is
 * visible. The return value of every function is an LSTATUS (0 = ERROR_SUCCESS, else a system error code),
 * set in R[0]; the guest return address stays on the stack for the caller's ret (see fle_imports.h). The
 * error numbers and helpers are in fle_env.h; the handle regions are in fle_vfs.h. */
#ifndef FLE_REG_H
#define FLE_REG_H

#include <stdint.h>

#include "fle_vfs.h"

/* Predefined key handles. The Microsoft pages read for this work (Predefined Keys, RegOpenKeyExW, ...) name
 * the keys but do not give their numeric values; these values are the winnt.h constants from memory and are
 * not confirmed by those pages. FLEngine itself passes 0x80000001 and 0x80000002 (objdump of f_419... at
 * 0x419a12 and 0x419a40), which agrees with HKEY_CURRENT_USER and HKEY_LOCAL_MACHINE. */
#define FLE_HKEY_CLASSES_ROOT   UINT64_C(0xFFFFFFFF80000000)
#define FLE_HKEY_CURRENT_USER   UINT64_C(0xFFFFFFFF80000001)
#define FLE_HKEY_LOCAL_MACHINE  UINT64_C(0xFFFFFFFF80000002)
#define FLE_HKEY_USERS          UINT64_C(0xFFFFFFFF80000003)
#define FLE_HKEY_CURRENT_CONFIG UINT64_C(0xFFFFFFFF80000005)

/* Value types (MSDN: Registry Value Types gives the names; the numbers are from winnt.h, unconfirmed by the
 * page). */
#define FLE_REG_NONE      0u
#define FLE_REG_SZ        1u
#define FLE_REG_EXPAND_SZ 2u
#define FLE_REG_BINARY    3u
#define FLE_REG_DWORD     4u
#define FLE_REG_DWORD_BE  5u
#define FLE_REG_LINK      6u
#define FLE_REG_MULTI_SZ  7u
#define FLE_REG_QWORD     11u

#define FLE_KEY_QUERY_VALUE        0x1u
#define FLE_KEY_SET_VALUE          0x2u
#define FLE_KEY_CREATE_SUB_KEY     0x4u
#define FLE_KEY_ENUMERATE_SUB_KEYS 0x8u

void fle_imp_RegOpenKeyExW(void);
void fle_imp_RegOpenKeyExA(void);
void fle_imp_RegQueryValueExW(void);
void fle_imp_RegCloseKey(void);
void fle_imp_RegCreateKeyExW(void);
void fle_imp_RegSetValueExW(void);
void fle_imp_RegEnumKeyExW(void);
void fle_imp_RegEnumValueW(void);
void fle_imp_RegDeleteKeyW(void);
void fle_imp_RegDeleteValueW(void);
void fle_imp_RegQueryInfoKeyW(void);
void fle_imp_RegFlushKey(void);

#endif
