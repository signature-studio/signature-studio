/* The emulated Windows that the lift runtime shows to the guest, and the version
 * APIs that read it (docs/decisions/0014-emulated-os.md).
 *
 * The OS values live in one place: fle_emulated_os in fle_osver.c. Changing the
 * emulated OS means editing that definition and nothing else. */
#ifndef FLE_OSVER_H
#define FLE_OSVER_H

#include <stdint.h>

typedef struct {
    uint32_t major;        /* dwMajorVersion */
    uint32_t minor;        /* dwMinorVersion */
    uint32_t build;        /* dwBuildNumber */
    uint32_t platform_id;  /* dwPlatformId */
    uint16_t sp_major;     /* wServicePackMajor */
    uint16_t sp_minor;     /* wServicePackMinor */
    uint16_t suite_mask;   /* wSuiteMask */
    uint8_t product_type;  /* wProductType */
    /* szCSDVersion is the empty string for every OS this runtime can emulate. */
} fle_osver_t;

extern const fle_osver_t fle_emulated_os;

/* Structure layout (MSDN: OSVERSIONINFOW, OSVERSIONINFOEXW). The page lists the
 * members and their types; the offsets below follow from those types (DWORD 4,
 * WCHAR 2, WORD 2, BYTE 1, no padding is needed). The EXW size 0x11c is the value
 * FLEngine itself stores in dwOSVersionInfoSize (f_409b30, 0x409b56). */
#define FLE_OSVI_SIZE_OFF       0u
#define FLE_OSVI_MAJOR_OFF      4u
#define FLE_OSVI_MINOR_OFF      8u
#define FLE_OSVI_BUILD_OFF      12u
#define FLE_OSVI_PLATFORM_OFF   16u
#define FLE_OSVI_CSD_OFF        20u
#define FLE_OSVI_CSD_BYTES      256u    /* WCHAR szCSDVersion[128] */
#define FLE_OSVERSIONINFOW_SIZE 276u    /* 20 + 256 */
#define FLE_OSVIEX_SPMAJOR_OFF  276u
#define FLE_OSVIEX_SPMINOR_OFF  278u
#define FLE_OSVIEX_SUITE_OFF    280u
#define FLE_OSVIEX_PRODTYPE_OFF 282u
#define FLE_OSVIEX_RESERVED_OFF 283u
#define FLE_OSVERSIONINFOEXW_SIZE 284u  /* 0x11c */

/* OSVERSIONINFOA / OSVERSIONINFOEXA (MSDN: OSVERSIONINFOA, OSVERSIONINFOEXA; decision 0102 W4): the same members with CHAR
 * szCSDVersion[128]; the first five members have the offsets above, the extra members of the EX form follow the 128 bytes. */
#define FLE_OSVERSIONINFOA_SIZE  148u   /* 20 + 128 = 0x94 */
#define FLE_OSVIEXA_SPMAJOR_OFF  148u
#define FLE_OSVIEXA_SPMINOR_OFF  150u
#define FLE_OSVIEXA_SUITE_OFF    152u
#define FLE_OSVIEXA_PRODTYPE_OFF 154u
#define FLE_OSVERSIONINFOEXA_SIZE 156u  /* 0x9c */

/* dwTypeMask and TypeMask flags (MSDN: VerifyVersionInfoW, VerSetConditionMask). */
#define FLE_VER_MINORVERSION     0x01u
#define FLE_VER_MAJORVERSION     0x02u
#define FLE_VER_BUILDNUMBER      0x04u
#define FLE_VER_PLATFORMID       0x08u
#define FLE_VER_SERVICEPACKMINOR 0x10u
#define FLE_VER_SERVICEPACKMAJOR 0x20u
#define FLE_VER_SUITENAME        0x40u
#define FLE_VER_PRODUCT_TYPE     0x80u

/* Conditions (MSDN: VerSetConditionMask, Condition). */
#define FLE_VER_EQUAL         1u
#define FLE_VER_GREATER       2u
#define FLE_VER_GREATER_EQUAL 3u
#define FLE_VER_LESS          4u
#define FLE_VER_LESS_EQUAL    5u
#define FLE_VER_AND           6u
#define FLE_VER_OR            7u

#define FLE_ERROR_OLD_WIN_VERSION 1150u

/* VerifyVersionInfoW against an arbitrary OS (the import uses fle_emulated_os).
 * info points at an OSVERSIONINFOEXW. Returns 1 when the OS satisfies the
 * requirements, 0 when not. Cases the MSDN pages do not define stop the run. */
int fle_osver_check(const fle_osver_t *os, const uint8_t *info, uint32_t type_mask,
                    uint64_t condition_mask);

/* GetVersion's packed DWORD and GetVersionExW's fill, for an arbitrary OS. The second
 * returns 1 on success, 0 when dwOSVersionInfoSize is not one of the two sizes. */
uint32_t fle_osver_getversion(const fle_osver_t *os);
int fle_osver_get_version_ex(const fle_osver_t *os, uint8_t *info);

/* GetVersionExA's fill (decision 0102 W4), same contract as fle_osver_get_version_ex for the two ANSI sizes. */
int fle_osver_get_version_ex_a(const fle_osver_t *os, uint8_t *info);

/* kernel32!VerifyVersionInfoW: BOOL (LPOSVERSIONINFOEXW, DWORD, DWORDLONG). */
void fle_imp_VerifyVersionInfoW(void);

/* kernel32!GetVersion: DWORD GetVersion(void). */
void fle_imp_GetVersion(void);

/* kernel32!GetVersionExW: BOOL GetVersionExW(LPOSVERSIONINFOW). */
void fle_imp_GetVersionExW(void);

/* kernel32!GetVersionExA: BOOL GetVersionExA(LPOSVERSIONINFOA) (decision 0102 W4). */
void fle_imp_GetVersionExA(void);

#endif
