#include <stdint.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_osver.h"

/* The one place that holds the emulated OS (docs/decisions/0014-emulated-os.md):
 * Windows 10 22H2, workstation, no service pack, empty szCSDVersion, no suites.
 * Everything below reads this object; nothing else in the runtime hard-codes an OS. */
const fle_osver_t fle_emulated_os = {
    .major = 10,
    .minor = 0,
    .build = 19045,
    .platform_id = 2, /* VER_PLATFORM_WIN32_NT */
    .sp_major = 0,
    .sp_minor = 0,
    .suite_mask = 0,
    .product_type = 1, /* VER_NT_WORKSTATION */
};

static void fle_osver_stop(const char *api, const char *why)
{
    fprintf(stderr, "fle: %s: %s (not defined by the MSDN page; not implemented)\n", api, why);
    abort();
}

static uint32_t rd32(const uint8_t *p) { uint32_t v; memcpy(&v, p, 4); return v; }
static uint16_t rd16(const uint8_t *p) { uint16_t v; memcpy(&v, p, 2); return v; }
static void wr32(uint8_t *p, uint32_t v) { memcpy(p, &v, 4); }
static void wr16(uint8_t *p, uint16_t v) { memcpy(p, &v, 2); }

/* One comparison: "current <cond> required" (MSDN: VerSetConditionMask, Condition).
 * Conditions other than 1..5 are not defined for these members. */
static int fle_ver_cmp(uint32_t cur, uint32_t req, uint32_t cond)
{
    switch (cond) {
    case FLE_VER_EQUAL:         return cur == req;
    case FLE_VER_GREATER:       return cur > req;
    case FLE_VER_GREATER_EQUAL: return cur >= req;
    case FLE_VER_LESS:          return cur < req;
    case FLE_VER_LESS_EQUAL:    return cur <= req;
    default:
        fle_osver_stop("VerifyVersionInfoW", "comparison condition outside 1..5");
        return 0;
    }
}

/* Position of a member's condition in dwlConditionMask: 3 bits at 3 * k, k = bit index
 * of the TypeMask flag. The MSDN pages do not give this layout; it is the one that
 * fle_imp_VerSetConditionMask writes (decision 0011, from Wine and mingw-w64). */
static uint32_t fle_cond_of(uint64_t mask, uint32_t flag)
{
    unsigned k = 0;
    while (!((flag >> k) & 1u)) k++;
    return (uint32_t)((mask >> (3u * k)) & 7u);
}

/* VerifyVersionInfoW (MSDN: VerifyVersionInfoW, Parameters, Return value, Remarks).
 *
 * - Every member named in type_mask must satisfy its condition (Remarks: "returns a
 *   nonzero value only if all specified tests succeed"). Members whose bit is not set
 *   are ignored (Parameters, lpVersionInformation).
 * - Major, minor, service pack major and service pack minor are tested in that order
 *   (Remarks). The test continues while the values are equal and stops at the first
 *   member whose values differ. A member that does not meet its condition fails the
 *   whole test; a member that differs and meets its condition ends the test with
 *   success (Remarks: ">= 5.1 SP1" succeeds on 6.0 and on 5.2, fails on 5.0 SP2).
 *   Build number, platform id, suite mask and product type are compared on their own.
 * - Not defined by the pages, so the run stops: type_mask 0 or with bits outside
 *   0xFF, a NULL structure, dwOSVersionInfoSize other than 0 and sizeof(OSVERSIONINFOEXW)
 *   (0 is let through by decision 0040, a provisional decision from unofficial sources;
 *   members are read only when their dwTypeMask bit is set, so a size-0 call reads
 *   nothing else), a condition outside 1..5 (suite: outside 6..7), an empty suite mask, and two
 *   adjacent tested hierarchy members whose conditions differ when the higher one is
 *   not VER_EQUAL (Remarks say the higher condition "supersedes" the lower and that the
 *   function "will change the request", without defining the resulting test).
 * - Not enforced: the Parameters table says a major version test "must" also test minor
 *   and both service pack members. The page states no consequence for leaving one out,
 *   and says members without their bit are ignored, so the omitted member is skipped.
 *   FLEngine's own call (0x409ba3, type_mask 0x23) leaves out service pack minor. */
int fle_osver_check(const fle_osver_t *os, const uint8_t *info, uint32_t type_mask,
                    uint64_t condition_mask)
{
    static const uint32_t hier[4] = {
        FLE_VER_MAJORVERSION, FLE_VER_MINORVERSION,
        FLE_VER_SERVICEPACKMAJOR, FLE_VER_SERVICEPACKMINOR,
    };
    uint32_t cur[4], req[4];
    uint32_t cond[4] = { 0, 0, 0, 0 };
    int i, prev = -1;

    if (type_mask == 0) fle_osver_stop("VerifyVersionInfoW", "dwTypeMask is 0");
    if (type_mask & ~0xFFu) fle_osver_stop("VerifyVersionInfoW", "dwTypeMask has bits outside 0x01..0x80");
    if (info == NULL) fle_osver_stop("VerifyVersionInfoW", "lpVersionInformation is NULL");
    /* Decision 0040 (provisional, unofficial sources, not checked on a real Windows):
     * a size of 0 is not checked and the comparison goes on. Any other size that is
     * not sizeof(OSVERSIONINFOEXW) still stops the run. */
    {
        uint32_t size = rd32(info + FLE_OSVI_SIZE_OFF);
        if (size != 0 && size != FLE_OSVERSIONINFOEXW_SIZE)
            fle_osver_stop("VerifyVersionInfoW", "dwOSVersionInfoSize is neither 0 nor sizeof(OSVERSIONINFOEXW)");
    }

    if (type_mask & FLE_VER_PRODUCT_TYPE) {
        uint32_t c = fle_cond_of(condition_mask, FLE_VER_PRODUCT_TYPE);
        if (!fle_ver_cmp(os->product_type, info[FLE_OSVIEX_PRODTYPE_OFF], c)) return 0;
    }
    if (type_mask & FLE_VER_SUITENAME) {
        uint32_t c = fle_cond_of(condition_mask, FLE_VER_SUITENAME);
        uint16_t want = rd16(info + FLE_OSVIEX_SUITE_OFF);
        if (want == 0) fle_osver_stop("VerifyVersionInfoW", "VER_SUITENAME with an empty wSuiteMask");
        if (c == FLE_VER_AND) {
            if ((want & os->suite_mask) != want) return 0;
        } else if (c == FLE_VER_OR) {
            if ((want & os->suite_mask) == 0) return 0;
        } else {
            fle_osver_stop("VerifyVersionInfoW", "VER_SUITENAME condition is not VER_AND or VER_OR");
        }
    }
    if (type_mask & FLE_VER_PLATFORMID) {
        uint32_t c = fle_cond_of(condition_mask, FLE_VER_PLATFORMID);
        if (!fle_ver_cmp(os->platform_id, rd32(info + FLE_OSVI_PLATFORM_OFF), c)) return 0;
    }
    if (type_mask & FLE_VER_BUILDNUMBER) {
        uint32_t c = fle_cond_of(condition_mask, FLE_VER_BUILDNUMBER);
        if (!fle_ver_cmp(os->build, rd32(info + FLE_OSVI_BUILD_OFF), c)) return 0;
    }

    cur[0] = os->major;    req[0] = (type_mask & hier[0]) ? rd32(info + FLE_OSVI_MAJOR_OFF) : 0;
    cur[1] = os->minor;    req[1] = (type_mask & hier[1]) ? rd32(info + FLE_OSVI_MINOR_OFF) : 0;
    cur[2] = os->sp_major; req[2] = (type_mask & hier[2]) ? rd16(info + FLE_OSVIEX_SPMAJOR_OFF) : 0;
    cur[3] = os->sp_minor; req[3] = (type_mask & hier[3]) ? rd16(info + FLE_OSVIEX_SPMINOR_OFF) : 0;

    for (i = 0; i < 4; i++) {
        if (!(type_mask & hier[i])) continue;
        cond[i] = fle_cond_of(condition_mask, hier[i]);
        if (cond[i] < FLE_VER_EQUAL || cond[i] > FLE_VER_LESS_EQUAL)
            fle_osver_stop("VerifyVersionInfoW", "comparison condition outside 1..5");
        if (prev >= 0 && cond[prev] != FLE_VER_EQUAL && cond[i] != cond[prev])
            fle_osver_stop("VerifyVersionInfoW", "different conditions on adjacent version members");
        prev = i;
    }

    for (i = 0; i < 4; i++) {
        if (!(type_mask & hier[i])) continue;
        if (!fle_ver_cmp(cur[i], req[i], cond[i])) return 0;
        if (cur[i] != req[i]) return 1;
    }
    return 1;
}

/* kernel32!VerifyVersionInfoW: lpVersionInformation in rcx (R[1]), dwTypeMask in edx
 * (R[2]), dwlConditionMask in r8 (R[8]). A satisfied request returns nonzero (1). An
 * unsatisfied one returns 0 and sets the last error to ERROR_OLD_WIN_VERSION (1150)
 * (MSDN: Return value). The last error is left alone on success (the page says
 * nothing about it). */
void fle_imp_VerifyVersionInfoW(void)
{
    const uint8_t *info = (const uint8_t *)fle_addr_opt(R[1], 1, 0);
    uint32_t type_mask = (uint32_t)R[2];
    uint64_t cmask = R[8];

    if (fle_osver_check(&fle_emulated_os, info, type_mask, cmask)) {
        R[0] = 1;
        return;
    }
    fle_set_last_error(FLE_ERROR_OLD_WIN_VERSION);
    R[0] = 0;
}

/* GetVersion value (MSDN: GetVersion, Return value and Examples). The low word holds the
 * major version in its low byte and the minor version in its high byte. The Examples code
 * reads the build number from the high word when the value is below 0x80000000, so bit 31
 * is 0 and the high word is the build number. The Return value text also has the sentence
 * "The high-order bit is zero, the next 7 bits represent the build number, and the
 * low-order byte is 5", which does not agree with that code and is not used here.
 * A major, minor (above 255) or build (above 0x7FFF) that does not fit stops the run. */
uint32_t fle_osver_getversion(const fle_osver_t *os)
{
    if (os->major > 0xFFu || os->minor > 0xFFu || os->build > 0x7FFFu)
        fle_osver_stop("GetVersion", "version does not fit the packed DWORD");
    return (os->build << 16) | (os->minor << 8) | os->major;
}

void fle_imp_GetVersion(void)
{
    R[0] = fle_osver_getversion(&fle_emulated_os);
}

/* GetVersionExW (MSDN: GetVersionExW, Parameters and Return value). dwOSVersionInfoSize
 * tells which structure is passed: sizeof(OSVERSIONINFOW) or sizeof(OSVERSIONINFOEXW).
 * Any other size fails with a zero return (Return value). The last-error code is not
 * named by the page; ERROR_INVALID_PARAMETER (87) is a harness choice. A success writes
 * the members both structures share, and for the EXW structure the four extra members.
 * szCSDVersion gets the empty string. dwOSVersionInfoSize and wReserved are left as
 * they were. A NULL pointer is not defined by the page and stops the run. */
int fle_osver_get_version_ex(const fle_osver_t *os, uint8_t *info)
{
    uint32_t size;

    if (info == NULL) fle_osver_stop("GetVersionExW", "lpVersionInformation is NULL");
    size = rd32(info + FLE_OSVI_SIZE_OFF);
    if (size != FLE_OSVERSIONINFOW_SIZE && size != FLE_OSVERSIONINFOEXW_SIZE) return 0;

    wr32(info + FLE_OSVI_MAJOR_OFF, os->major);
    wr32(info + FLE_OSVI_MINOR_OFF, os->minor);
    wr32(info + FLE_OSVI_BUILD_OFF, os->build);
    wr32(info + FLE_OSVI_PLATFORM_OFF, os->platform_id);
    memset(info + FLE_OSVI_CSD_OFF, 0, FLE_OSVI_CSD_BYTES);
    if (size == FLE_OSVERSIONINFOEXW_SIZE) {
        wr16(info + FLE_OSVIEX_SPMAJOR_OFF, os->sp_major);
        wr16(info + FLE_OSVIEX_SPMINOR_OFF, os->sp_minor);
        wr16(info + FLE_OSVIEX_SUITE_OFF, os->suite_mask);
        info[FLE_OSVIEX_PRODTYPE_OFF] = os->product_type;
    }
    return 1;
}

/* GetVersionExA (MSDN: GetVersionExA; decision 0102 W4): the page is the same text as GetVersionExW with OSVERSIONINFOA /
 * OSVERSIONINFOEXA, whose only difference is the CHAR szCSDVersion[128]; the values, the size rule (any other
 * dwOSVersionInfoSize fails with a zero return and 87 as the harness's choice, as for W) and the empty szCSDVersion are those of W. */
int fle_osver_get_version_ex_a(const fle_osver_t *os, uint8_t *info)
{
    uint32_t size;

    if (info == NULL) fle_osver_stop("GetVersionExA", "lpVersionInformation is NULL");
    size = rd32(info + FLE_OSVI_SIZE_OFF);
    if (size != FLE_OSVERSIONINFOA_SIZE && size != FLE_OSVERSIONINFOEXA_SIZE) return 0;

    wr32(info + FLE_OSVI_MAJOR_OFF, os->major);
    wr32(info + FLE_OSVI_MINOR_OFF, os->minor);
    wr32(info + FLE_OSVI_BUILD_OFF, os->build);
    wr32(info + FLE_OSVI_PLATFORM_OFF, os->platform_id);
    memset(info + FLE_OSVI_CSD_OFF, 0, 128u);
    if (size == FLE_OSVERSIONINFOEXA_SIZE) {
        wr16(info + FLE_OSVIEXA_SPMAJOR_OFF, os->sp_major);
        wr16(info + FLE_OSVIEXA_SPMINOR_OFF, os->sp_minor);
        wr16(info + FLE_OSVIEXA_SUITE_OFF, os->suite_mask);
        info[FLE_OSVIEXA_PRODTYPE_OFF] = os->product_type;
    }
    return 1;
}

void fle_imp_GetVersionExA(void)
{
    uint8_t *info = (uint8_t *)fle_addr_opt(R[1], 1, 1);

    if (fle_osver_get_version_ex_a(&fle_emulated_os, info)) {
        R[0] = 1;
        return;
    }
    fle_set_last_error(87u);
    R[0] = 0;
}

void fle_imp_GetVersionExW(void)
{
    uint8_t *info = (uint8_t *)fle_addr_opt(R[1], 1, 1);

    if (fle_osver_get_version_ex(&fle_emulated_os, info)) {
        R[0] = 1;
        return;
    }
    fle_set_last_error(87u);
    R[0] = 0;
}
