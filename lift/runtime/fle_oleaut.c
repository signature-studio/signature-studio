/* oleaut32 answers: BSTR, VARIANT, SAFEARRAY, GetErrorInfo (docs/decisions/0037-apis-oleaut-ole.md; the pages read are
 * listed in docs/investigations/apis-oleaut-ole-run.md). Single guest thread. There is no guest-function call-back
 * mechanism, so every input that would call IUnknown::AddRef/Release or IRecordInfo stops the run through fle_stop
 * (fle_env.h); so does every input the Microsoft pages leave open. Where a page says only "fails" or names no value,
 * the choice made here is marked "setting" or "inference". Nothing is read from the host. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_ptrmap.h"
#include "fle_oleaut.h"

static long owner_tid;

static void check_thread(const char *api)
{
    long tid = fle_host_tid();

    if (owner_tid == 0) owner_tid = tid;
    if (tid != owner_tid) fle_stop(api, "called from a second thread, not supported");
}

/* VARENUM (Microsoft Learn: VARENUM). */
#define VT_EMPTY    0u
#define VT_NULL     1u
#define VT_I2       2u
#define VT_I4       3u
#define VT_R4       4u
#define VT_R8       5u
#define VT_CY       6u
#define VT_DATE     7u
#define VT_BSTR     8u
#define VT_DISPATCH 9u
#define VT_ERROR    10u
#define VT_BOOL     11u
#define VT_VARIANT  12u
#define VT_UNKNOWN  13u
#define VT_DECIMAL  14u
#define VT_I1       16u
#define VT_UI1      17u
#define VT_UI2      18u
#define VT_UI4      19u
#define VT_I8       20u
#define VT_UI8      21u
#define VT_INT      22u
#define VT_UINT     23u
#define VT_RECORD   36u
#define VT_VECTOR   0x1000u
#define VT_ARRAY    0x2000u
#define VT_BYREF    0x4000u
#define VT_RESERVED 0x8000u
#define VT_TYPEMASK 0x0FFFu

/* VARIANT on x64, from the struct on the VARIANT page with natural alignment: vt (2) and three reserved words, then a
 * 16-byte union (the largest member is the 16-byte BRECORD / DECIMAL). */
#define VAR_VT    0u
#define VAR_VAL   8u
#define VAR_SIZE  24u

/* SAFEARRAY on x64, from the struct on the SAFEARRAY page with natural alignment: cDims (2), fFeatures (2),
 * cbElements (4), cLocks (4), pvData (8 at offset 16), rgsabound at 24, each SAFEARRAYBOUND = cElements (4), lLbound (4). */
#define SA_CDIMS   0u
#define SA_FEAT    2u
#define SA_CBELEM  4u
#define SA_CLOCKS  8u
#define SA_PVDATA  16u
#define SA_BOUNDS  24u
#define SA_BOUND_SIZE 8u

#define FADF_HAVEVARTYPE 0x0080u
#define FADF_BSTR        0x0100u
#define FADF_UNKNOWN     0x0200u
#define FADF_DISPATCH    0x0400u
#define FADF_VARIANT     0x0800u

/* The most bytes of element data one array may hold (a limit of the virtual machine; a larger request fails the way the
 * page allows: SafeArrayCreate returns NULL). */
#define FLE_SA_MAX_BYTES UINT64_C(0x40000000)

static fle_pmap bstrs;  /* live BSTR pointer -> character count */
static fle_pmap arrays; /* live SAFEARRAY descriptor -> element VARTYPE */

uint64_t fle_oleaut_bstr_live(void) { return bstrs.live; }
uint64_t fle_oleaut_array_live(void) { return arrays.live; }

/* ---- BSTR ----
 * BSTR page: a 4-byte count of the bytes of the data string (terminator not counted) sits right before the first
 * character, the pointer points at the first character, a NUL WCHAR ends the data. The block is a host calloc block; the
 * 4 bytes of prefix make the BSTR pointer 4 mod 16 (the pages give no alignment). */

/* New BSTR of `chars` characters copied from the guest address `src` (0 = no source: the characters are zero, a choice;
 * the page leaves them uninitialized). Returns 0 when the request is above the limit of the virtual machine (the page
 * allows NULL for insufficient memory). */
static uint64_t bstr_new(const char *api, uint64_t src, uint64_t chars)
{
    uint8_t *blk;
    uint32_t bytes;
    uint64_t p;

    if (chars > FLE_BSTR_MAX_CHARS) return 0;
    blk = calloc(1, 4u + (size_t)chars * 2u + 2u);
    if (!blk) return 0;
    bytes = (uint32_t)(chars * 2u);
    memcpy(blk, &bytes, 4);
    if (src != 0 && chars != 0) memcpy(blk + 4, (const void *)fle_addr(src, (unsigned)(chars * 2u), 0), (size_t)chars * 2u);
    p = (uint64_t)(uintptr_t)(blk + 4);
    fle_pm_put(&bstrs, api, p, chars);
    return p;
}

/* The character count of a live BSTR; a pointer that is not one stops the run (a stray pointer, a freed BSTR, a string
 * literal passed as a BSTR). */
static uint64_t bstr_len(const char *api, uint64_t p)
{
    uint64_t n;

    if (!fle_pm_get(&bstrs, p, &n)) fle_stop(api, "pointer is not a live BSTR (freed, never allocated by SysAllocStringLen/SysReAllocStringLen, or a plain string)");
    return n;
}

static void bstr_free(const char *api, uint64_t p)
{
    if (p == 0) return;
    (void)bstr_len(api, p);
    fle_pm_del(&bstrs, p);
    free((void *)(uintptr_t)(p - 4u));
}

/* A copy of a live BSTR (the pages: "a copy of the string is made"); NULL stays NULL (inference: a NULL BSTR is "treated
 * the same as" an empty one by convention, and either answer follows it). Host exhaustion stops. */
static uint64_t bstr_dup(const char *api, uint64_t p)
{
    uint64_t q;

    if (p == 0) return 0;
    q = bstr_new(api, p, bstr_len(api, p));
    if (q == 0) fle_stop(api, "out of memory while copying a BSTR");
    return q;
}

/* oleaut32!SysAllocStringLen: BSTR f(const OLECHAR *strIn rcx, UINT ui edx). Page: allocates ui + 1 characters, copies ui
 * characters from strIn (which may hold NULs and need not end in one; when not NULL it must be at least ui characters
 * long), puts a NUL after them; NULL when memory is insufficient. strIn NULL: nothing is copied and the page says
 * nothing about the contents; they are zero here (setting). ui 0 gives a valid empty BSTR (the page allocates "ui plus
 * one characters"). */
void fle_imp_SysAllocStringLen(void)
{
    static const char api[] = "SysAllocStringLen";

    check_thread(api);
    R[0] = bstr_new(api, R[1], (uint32_t)R[2]);
}

/* oleaut32!SysFreeString: void f(BSTR rcx). Page: NULL simply returns. A pointer that is not a live BSTR (freed twice,
 * from another allocator) is not defined by the page and stops the run. */
void fle_imp_SysFreeString(void)
{
    check_thread("SysFreeString");
    bstr_free("SysFreeString", R[1]);
}

/* oleaut32!SysReAllocStringLen: INT f(BSTR *pbstr rcx, const OLECHAR *psz rdx, unsigned int len r8d). Page: allocates a
 * new BSTR of len characters copied from psz (NULL: allocated, not initialized; zero here), frees the BSTR *pbstr
 * referred to, stores the new one in *pbstr, returns TRUE; FALSE when memory is insufficient. pbstr NULL is an access
 * violation on the page, so it stops. *pbstr NULL is the "null pointer is a valid BSTR" convention (String Manipulation
 * Functions page): nothing is freed. The copy is made before the old string is freed, so psz inside the old BSTR is
 * safe; a read past the old BSTR's terminator stops. On FALSE *pbstr is unchanged (inference: the page does not say). */
void fle_imp_SysReAllocStringLen(void)
{
    static const char api[] = "SysReAllocStringLen";
    uint64_t pb = R[1], psz = R[2], old, np;
    uint32_t len = (uint32_t)R[8];

    check_thread(api);
    if (pb == 0) fle_stop(api, "pbstr is NULL (the page: access violation)");
    old = fle_ld(pb, 8);
    if (old != 0) {
        uint64_t on = bstr_len(api, old);

        if (psz >= old && psz < old + 2u * (on + 1u) && psz + 2u * (uint64_t)len > old + 2u * (on + 1u))
            fle_stop(api, "psz is inside the old BSTR and len reads past its terminator");
    }
    np = bstr_new(api, psz, len);
    if (np == 0) {
        R[0] = 0;
        return;
    }
    bstr_free(api, old);
    fle_st(pb, 8, np);
    R[0] = 1;
}

/* ---- classification of a VARTYPE in a VARIANT ----
 * VARENUM page, "where used" table (VARIANT column): VT_BOOL VT_BSTR VT_CY VT_DATE VT_DECIMAL VT_DISPATCH VT_EMPTY
 * VT_ERROR VT_I1 VT_I2 VT_I4 VT_INT VT_NULL VT_R4 VT_R8 VT_RECORD VT_UI1 VT_UI2 VT_UI4 VT_UINT VT_UNKNOWN VT_VARIANT,
 * and the flags VT_ARRAY, VT_BYREF. VT_VECTOR and VT_RESERVED are not VARIANT flags; every other value is not a VARIANT
 * type. VT_I8 and VT_UI8 are not in that column but the VARIANT struct has llVal/ullVal: undecided, so they stop. */
enum { K_BAD = 1, K_PLAIN, K_BSTR, K_UNK, K_ARRAY, K_BYREF };

static int byref_base_ok(unsigned b)
{
    switch (b) {
    case VT_I2: case VT_I4: case VT_R4: case VT_R8: case VT_CY: case VT_DATE: case VT_BSTR: case VT_DISPATCH:
    case VT_ERROR: case VT_BOOL: case VT_VARIANT: case VT_UNKNOWN: case VT_DECIMAL: case VT_I1: case VT_UI1:
    case VT_UI2: case VT_UI4: case VT_INT: case VT_UINT:
        return 1;
    default:
        return 0;
    }
}

static int classify(const char *api, uint16_t vt)
{
    unsigned base = vt & VT_TYPEMASK, flags = vt & ~VT_TYPEMASK;

    if (flags & (VT_VECTOR | VT_RESERVED)) return K_BAD;
    if (base == VT_RECORD) fle_stop(api, "VT_RECORD needs IRecordInfo call-backs into guest code");
    if (base == VT_I8 || base == VT_UI8) fle_stop(api, "VT_I8/VT_UI8 in a VARIANT: the VARENUM page does not list them for VARIANT, the VARIANT page has llVal/ullVal");
    if (flags & VT_ARRAY) {
        /* safe-array element types (VARENUM table, "Safe array" column) */
        if (!byref_base_ok(base)) return K_BAD;
        return (flags & VT_BYREF) ? K_BYREF : K_ARRAY;
    }
    if (flags & VT_BYREF) {
        if (base == VT_EMPTY || base == VT_NULL) fle_stop(api, "VT_BYREF with VT_EMPTY/VT_NULL is not defined by the pages");
        return byref_base_ok(base) ? K_BYREF : K_BAD;
    }
    switch (base) {
    case VT_EMPTY: case VT_NULL: case VT_I2: case VT_I4: case VT_R4: case VT_R8: case VT_CY: case VT_DATE:
    case VT_ERROR: case VT_BOOL: case VT_DECIMAL: case VT_I1: case VT_UI1: case VT_UI2: case VT_UI4: case VT_INT:
    case VT_UINT:
        return K_PLAIN;
    case VT_BSTR:
        return K_BSTR;
    case VT_DISPATCH: case VT_UNKNOWN:
        return K_UNK;
    case VT_VARIANT:
        fle_stop(api, "VT_VARIANT without VT_BYREF: the pages only describe VT_VARIANT | VT_BYREF");
    default:
        return K_BAD;
    }
}

/* ---- SAFEARRAY element types ----
 * SafeArrayCreate page: neither VT_ARRAY nor VT_BYREF may be set; VT_EMPTY and VT_NULL are not valid base types (NULL
 * is returned); "all other types are legal". The VARENUM table's Safe array column gives the supported set. Sizes are
 * the sizes the VARENUM page names (2-byte integer, 4-byte integer, 4-byte real, 8-byte real, 16-byte DECIMAL, ...);
 * VT_BOOL is a 16-bit VARIANT_BOOL, VT_ERROR a 32-bit SCODE (MS-OAUT IDL: typedef LONG SCODE), VT_CY and VT_DATE 8
 * bytes (CY page: 8-byte integer; DATE: double-precision), the interface and BSTR types pointers, VT_VARIANT the
 * 24-byte VARIANT. fFeatures: the flag texts on the SAFEARRAY page give each flag's meaning, not which flags
 * SafeArrayCreate sets; FADF_BSTR/UNKNOWN/DISPATCH/VARIANT for those element types and FADF_HAVEVARTYPE for the others
 * is a setting. Returns 0 for a type the page makes invalid (the caller returns NULL). */
static uint32_t sa_elem(const char *api, uint16_t vt, uint16_t *feat)
{
    *feat = FADF_HAVEVARTYPE;
    if (vt & (VT_ARRAY | VT_BYREF)) return 0;
    switch (vt) {
    case VT_EMPTY: case VT_NULL: return 0;
    case VT_I1: case VT_UI1: return 1;
    case VT_I2: case VT_UI2: case VT_BOOL: return 2;
    case VT_I4: case VT_UI4: case VT_INT: case VT_UINT: case VT_R4: case VT_ERROR: return 4;
    case VT_R8: case VT_CY: case VT_DATE: return 8;
    case VT_DECIMAL: return 16;
    case VT_BSTR: *feat = FADF_BSTR; return 8;
    case VT_UNKNOWN: *feat = FADF_UNKNOWN; return 8;
    case VT_DISPATCH: *feat = FADF_DISPATCH; return 8;
    case VT_VARIANT: *feat = FADF_VARIANT; return VAR_SIZE;
    case VT_RECORD: fle_stop(api, "VT_RECORD arrays need IRecordInfo call-backs into guest code");
    case VT_I8: case VT_UI8: fle_stop(api, "VT_I8/VT_UI8 element type: the page calls all other types legal but the VARENUM table does not list them for safe arrays");
    default: fle_stop(api, "element VARTYPE is not in the VARENUM Safe array column and the page's \"all other types are legal\" does not say what it means");
    }
}

static uint64_t sa_count(uint64_t psa)
{
    uint32_t d = (uint32_t)fle_ld(psa + SA_CDIMS, 2), i;
    uint64_t n = 1;

    for (i = 0; i < d; i++) n *= (uint32_t)fle_ld(psa + SA_BOUNDS + SA_BOUND_SIZE * i, 4);
    return n;
}

/* A live descriptor's VARTYPE; NULL is the caller's E_INVALIDARG, a pointer that is not a descriptor made by
 * SafeArrayCreate here stops (it may be a guest-built descriptor, which the page's functions do not allow us to judge). */
static uint16_t sa_vt(const char *api, uint64_t psa)
{
    uint64_t v;

    if (!fle_pm_get(&arrays, psa, &v)) fle_stop(api, "psa is not a descriptor made by SafeArrayCreate (freed, guest-built, or stray)");
    return (uint16_t)v;
}

static uint32_t var_clear(const char *api, uint64_t p);
static uint32_t var_copy_val(const char *api, uint64_t dst, uint64_t src);

/* SafeArrayDestroy semantics (page): a locked array is DISP_E_ARRAYISLOCKED and stays; BSTR elements go through
 * SysFreeString, VARIANT elements through VariantClear; objects would be Released (call-back: stops if any is set). */
static uint32_t array_release(const char *api, uint64_t psa)
{
    uint16_t vt;
    uint64_t n, i, data;
    uint32_t es;

    if (psa == 0) fle_stop(api, "VT_ARRAY variant with a NULL parray is not defined");
    vt = sa_vt(api, psa);
    if ((uint32_t)fle_ld(psa + SA_CLOCKS, 4) != 0) return FLE_HR_DISP_E_ARRAYISLOCKED;
    n = sa_count(psa);
    es = (uint32_t)fle_ld(psa + SA_CBELEM, 4);
    data = fle_ld(psa + SA_PVDATA, 8);
    for (i = 0; i < n; i++) {
        uint64_t e = data + i * es;

        if (vt == VT_BSTR) {
            bstr_free(api, fle_ld(e, 8));
        } else if (vt == VT_UNKNOWN || vt == VT_DISPATCH) {
            if (fle_ld(e, 8) != 0) fle_stop(api, "releasing an object in a safe array needs IUnknown::Release (guest call-back)");
        } else if (vt == VT_VARIANT) {
            uint32_t hr = var_clear(api, e);

            if (hr != FLE_HR_S_OK) return hr;
        }
    }
    fle_pm_del(&arrays, psa);
    free((void *)(uintptr_t)data);
    free((void *)(uintptr_t)psa);
    return FLE_HR_S_OK;
}

/* SafeArrayCopy semantics (page): a new descriptor with the same shape; strings and variants are copied with the string
 * and variant functions; objects would be AddRef'd (call-back: stops if any is set). cLocks of the copy is 0 and
 * fFeatures is the source's (inference). */
static uint32_t array_copy(const char *api, uint64_t src, uint64_t *out)
{
    uint16_t vt;
    uint32_t dims, es;
    uint64_t n, i, dst, ddata, sdata;

    if (src == 0) fle_stop(api, "VT_ARRAY variant with a NULL parray is not defined");
    vt = sa_vt(api, src);
    dims = (uint32_t)fle_ld(src + SA_CDIMS, 2);
    es = (uint32_t)fle_ld(src + SA_CBELEM, 4);
    n = sa_count(src);
    dst = (uint64_t)(uintptr_t)calloc(1, SA_BOUNDS + (size_t)SA_BOUND_SIZE * dims);
    ddata = (uint64_t)(uintptr_t)calloc(1, (size_t)n * es + 1u);
    if (!dst || !ddata) fle_stop(api, "out of memory while copying a safe array");
    memcpy((void *)(uintptr_t)dst, (const void *)fle_addr(src, (unsigned)(SA_BOUNDS + (size_t)SA_BOUND_SIZE * dims), 0), SA_BOUNDS + (size_t)SA_BOUND_SIZE * dims);
    fle_st(dst + SA_CLOCKS, 4, 0);
    fle_st(dst + SA_PVDATA, 8, ddata);
    sdata = fle_ld(src + SA_PVDATA, 8);
    for (i = 0; i < n; i++) {
        uint64_t s = sdata + i * es, d = ddata + i * es;

        if (vt == VT_BSTR) {
            fle_st(d, 8, bstr_dup(api, fle_ld(s, 8)));
        } else if (vt == VT_UNKNOWN || vt == VT_DISPATCH) {
            if (fle_ld(s, 8) != 0) fle_stop(api, "copying an object in a safe array needs IUnknown::AddRef (guest call-back)");
        } else if (vt == VT_VARIANT) {
            if (var_copy_val(api, d, s) != FLE_HR_S_OK) fle_stop(api, "an element VARIANT of the array is invalid");
        } else {
            memcpy((void *)(uintptr_t)d, (const void *)fle_addr(s, es, 0), es);
        }
    }
    fle_pm_put(&arrays, api, dst, vt);
    *out = dst;
    return FLE_HR_S_OK;
}

/* ---- VARIANT ---- */

/* VariantClear semantics (page): the contents are released first (VT_BSTR: the string is freed; VT_ARRAY: the array is
 * freed, DISP_E_ARRAYISLOCKED when it is locked; VT_DISPATCH/VT_UNKNOWN: Release, a call-back, so a non-NULL pointer
 * stops; a NULL interface pointer has nothing to release, inference), VT_BYREF variants own nothing (page: for
 * object references; extended to the other by-reference types because VT_BYREF is "a void pointer for local use"), then
 * vt is set to VT_EMPTY. An invalid vt is DISP_E_BADVARTYPE and nothing changes. */
static uint32_t var_clear(const char *api, uint64_t p)
{
    uint16_t vt = (uint16_t)fle_ld(p + VAR_VT, 2);

    switch (classify(api, vt)) {
    case K_BAD:
        return FLE_HR_DISP_E_BADVARTYPE;
    case K_BSTR:
        bstr_free(api, fle_ld(p + VAR_VAL, 8));
        break;
    case K_UNK:
        if (fle_ld(p + VAR_VAL, 8) != 0) fle_stop(api, "VT_UNKNOWN/VT_DISPATCH with an object needs IUnknown::Release (guest call-back)");
        break;
    case K_ARRAY: {
        uint32_t hr = array_release(api, fle_ld(p + VAR_VAL, 8));

        if (hr != FLE_HR_S_OK) return hr;
        break;
    }
    default:
        break;
    }
    fle_st(p + VAR_VT, 2, VT_EMPTY);
    return FLE_HR_S_OK;
}

/* Exact copy of the 24 bytes (page: "an exact copy of the contents"), then the owned parts are made new: a BSTR is
 * copied, an array is copied; an object would be AddRef'd (call-back: a non-NULL pointer stops); a by-reference variant
 * is copied as the pointer (page: no reference count is changed for object references). `dst` is not cleared here. */
static uint32_t var_copy_val(const char *api, uint64_t dst, uint64_t src)
{
    switch (classify(api, (uint16_t)fle_ld(src + VAR_VT, 2))) {
    case K_BAD:
        return FLE_HR_DISP_E_BADVARTYPE;
    case K_BSTR: {
        uint64_t q = bstr_dup(api, fle_ld(src + VAR_VAL, 8));

        memmove((void *)fle_addr(dst, VAR_SIZE, 1), (const void *)fle_addr(src, VAR_SIZE, 0), VAR_SIZE);
        fle_st(dst + VAR_VAL, 8, q);
        return FLE_HR_S_OK;
    }
    case K_UNK:
        if (fle_ld(src + VAR_VAL, 8) != 0) fle_stop(api, "copying VT_UNKNOWN/VT_DISPATCH with an object needs IUnknown::AddRef (guest call-back)");
        break;
    case K_ARRAY: {
        uint64_t q = 0;
        uint32_t hr = array_copy(api, fle_ld(src + VAR_VAL, 8), &q);

        if (hr != FLE_HR_S_OK) return hr;
        memmove((void *)fle_addr(dst, VAR_SIZE, 1), (const void *)fle_addr(src, VAR_SIZE, 0), VAR_SIZE);
        fle_st(dst + VAR_VAL, 8, q);
        return FLE_HR_S_OK;
    }
    default:
        break;
    }
    memmove((void *)fle_addr(dst, VAR_SIZE, 1), (const void *)fle_addr(src, VAR_SIZE, 0), VAR_SIZE);
    return FLE_HR_S_OK;
}

/* The by-value size of a by-reference base type (the VARIANT page's pXxxVal members). */
static uint32_t deref_size(unsigned base)
{
    switch (base) {
    case VT_I1: case VT_UI1: return 1;
    case VT_I2: case VT_UI2: case VT_BOOL: return 2;
    case VT_I4: case VT_UI4: case VT_INT: case VT_UINT: case VT_R4: case VT_ERROR: return 4;
    case VT_R8: case VT_CY: case VT_DATE: return 8;
    default: return 0;
    }
}

/* VariantCopyInd semantics (page): a copy of the value, "performing the necessary indirection if the source is
 * VT_BYREF"; the destination is by value ((VT_BYREF | VT_I2) gives VT_I2). For (VT_BYREF | VT_VARIANT) the contained
 * variant is copied, and dereferenced too when it is by reference; the VARIANT page forbids a contained
 * VT_VARIANT | VT_BYREF, which stops. The unused bytes of the by-value result are zero (setting). */
static uint32_t var_deref_into(const char *api, uint64_t dst, uint64_t src, int depth)
{
    uint16_t vt = (uint16_t)fle_ld(src + VAR_VT, 2);
    uint64_t ptr;
    unsigned base = vt & VT_TYPEMASK;
    uint8_t tmp[VAR_SIZE];

    switch (classify(api, vt)) {
    case K_BAD:
        return FLE_HR_DISP_E_BADVARTYPE;
    case K_BYREF:
        break;
    default:
        return var_copy_val(api, dst, src);
    }
    ptr = fle_ld(src + VAR_VAL, 8);
    if (ptr == 0) fle_stop(api, "a by-reference VARIANT with a NULL pointer is not defined by the pages");
    memset(tmp, 0, sizeof tmp);
    if (vt & VT_ARRAY) {
        uint64_t q = 0;
        uint32_t hr = array_copy(api, fle_ld(ptr, 8), &q);

        if (hr != FLE_HR_S_OK) return hr;
        fle_st((uint64_t)(uintptr_t)tmp + VAR_VT, 2, VT_ARRAY | base);
        fle_st((uint64_t)(uintptr_t)tmp + VAR_VAL, 8, q);
    } else if (base == VT_VARIANT) {
        if (depth > 0) fle_stop(api, "a contained VT_VARIANT | VT_BYREF variant (the VARIANT page forbids it)");
        return var_deref_into(api, dst, ptr, depth + 1);
    } else if (base == VT_BSTR) {
        fle_st((uint64_t)(uintptr_t)tmp + VAR_VT, 2, VT_BSTR);
        fle_st((uint64_t)(uintptr_t)tmp + VAR_VAL, 8, bstr_dup(api, fle_ld(ptr, 8)));
    } else if (base == VT_DISPATCH || base == VT_UNKNOWN) {
        if (fle_ld(ptr, 8) != 0) fle_stop(api, "dereferencing VT_UNKNOWN/VT_DISPATCH with an object needs IUnknown::AddRef (guest call-back)");
        fle_st((uint64_t)(uintptr_t)tmp + VAR_VT, 2, base);
    } else if (base == VT_DECIMAL) {
        memcpy(tmp + 2, (const void *)fle_addr(ptr + 2, 14, 0), 14);
        fle_st((uint64_t)(uintptr_t)tmp + VAR_VT, 2, VT_DECIMAL);
    } else {
        uint32_t sz = deref_size(base);

        memcpy(tmp + VAR_VAL, (const void *)fle_addr(ptr, sz, 0), sz);
        fle_st((uint64_t)(uintptr_t)tmp + VAR_VT, 2, base);
    }
    memcpy((void *)fle_addr(dst, VAR_SIZE, 1), tmp, VAR_SIZE);
    return FLE_HR_S_OK;
}

/* oleaut32!VariantInit: void f(VARIANTARG *rcx). Page: sets the vt field to VT_EMPTY and does not interpret the current
 * contents; nothing else is written. NULL stops (the page names no behavior). */
void fle_imp_VariantInit(void)
{
    check_thread("VariantInit");
    if (R[1] == 0) fle_stop("VariantInit", "pvarg is NULL");
    fle_st(R[1] + VAR_VT, 2, VT_EMPTY);
}

/* oleaut32!VariantClear: HRESULT f(VARIANTARG *rcx). See var_clear. NULL is E_INVALIDARG (the page lists it as "one of
 * the arguments is not valid"). */
void fle_imp_VariantClear(void)
{
    check_thread("VariantClear");
    if (R[1] == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    R[0] = var_clear("VariantClear", R[1]);
}

/* oleaut32!VariantCopy: HRESULT f(VARIANTARG *dest rcx, const VARIANTARG *src rdx). Page: dest (a valid initialized
 * variant) is cleared as VariantClear does, then receives an exact copy of src. The order here: src is checked first
 * (DISP_E_BADVARTYPE, dest untouched), then dest is cleared (its error is returned and src is not copied), then the
 * copy is made. dest == src is not defined by the page and stops. NULL is E_INVALIDARG. */
void fle_imp_VariantCopy(void)
{
    static const char api[] = "VariantCopy";
    uint64_t dst = R[1], src = R[2];
    uint32_t hr;

    check_thread(api);
    if (dst == 0 || src == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    if (dst == src) fle_stop(api, "dest equals src (the page does not define it)");
    if (classify(api, (uint16_t)fle_ld(src + VAR_VT, 2)) == K_BAD) {
        R[0] = FLE_HR_DISP_E_BADVARTYPE;
        return;
    }
    hr = var_clear(api, dst);
    if (hr != FLE_HR_S_OK) {
        R[0] = hr;
        return;
    }
    R[0] = var_copy_val(api, dst, src);
}

/* oleaut32!VariantCopyInd: HRESULT f(VARIANT *dest rcx, const VARIANTARG *src rdx). See var_deref_into. dest is cleared
 * first as in VariantCopy ("frees any existing contents of pvarDest"); dest == src stops; NULL is E_INVALIDARG. */
void fle_imp_VariantCopyInd(void)
{
    static const char api[] = "VariantCopyInd";
    uint64_t dst = R[1], src = R[2];
    uint32_t hr;

    check_thread(api);
    if (dst == 0 || src == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    if (dst == src) fle_stop(api, "dest equals src (the page does not define it)");
    if (classify(api, (uint16_t)fle_ld(src + VAR_VT, 2)) == K_BAD) {
        R[0] = FLE_HR_DISP_E_BADVARTYPE;
        return;
    }
    hr = var_clear(api, dst);
    if (hr != FLE_HR_S_OK) {
        R[0] = hr;
        return;
    }
    R[0] = var_deref_into(api, dst, src, 0);
}

/* oleaut32!VariantChangeType: HRESULT f(VARIANTARG *dest rcx, const VARIANTARG *src rdx, USHORT wFlags r8w, VARTYPE vt
 * r9w). The page names the coercions only by example (numeric to string, string to numeric, BOOL to VT_I2 first, the
 * Value property of an object) and gives no table or rounding rule, and the flag values are not on it; those need
 * rules this runtime does not have, so they stop. What the page does fix: the result's vt is vt on S_OK; dest == src
 * converts in place; a by-reference source is read through; an invalid vt (source or target) is DISP_E_BADVARTYPE;
 * NULL is E_INVALIDARG. Implemented: wFlags 0 and a conversion to the type the value already has (inference: the
 * value is unchanged), including through a VT_BYREF source of that type. dest is treated as [out] (not cleared),
 * as the page marks it. */
void fle_imp_VariantChangeType(void)
{
    static const char api[] = "VariantChangeType";
    uint64_t dst = R[1], src = R[2];
    uint16_t flags = (uint16_t)R[8], vt = (uint16_t)R[9], svt;
    int sk;
    uint8_t tmp[VAR_SIZE];

    check_thread(api);
    if (dst == 0 || src == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    svt = (uint16_t)fle_ld(src + VAR_VT, 2);
    sk = classify(api, svt);
    if (sk == K_BAD || classify(api, vt) == K_BAD) {
        R[0] = FLE_HR_DISP_E_BADVARTYPE;
        return;
    }
    if (flags != 0) fle_stop(api, "wFlags is not 0 (the flag values are not on the page)");
    if (vt & (VT_BYREF | VT_ARRAY)) fle_stop(api, "a by-reference or array target type is not defined by the page");
    if (sk == K_ARRAY || (svt & VT_ARRAY)) fle_stop(api, "array sources are not defined by the page");
    if (sk == K_BYREF) {
        if ((svt & VT_TYPEMASK) != vt) fle_stop(api, "conversion through a by-reference source to another type needs coercion rules the page does not give");
        if (var_deref_into(api, (uint64_t)(uintptr_t)tmp, src, 0) != FLE_HR_S_OK) fle_stop(api, "dereferencing the source failed");
        memcpy((void *)fle_addr(dst, VAR_SIZE, 1), tmp, VAR_SIZE);
        R[0] = FLE_HR_S_OK;
        return;
    }
    if (svt != vt) fle_stop(api, "conversion between different types needs the coercion rules the page does not give");
    if (dst != src) R[0] = var_copy_val(api, dst, src);
    else R[0] = FLE_HR_S_OK;
}

/* ---- SAFEARRAY functions ---- */

/* oleaut32!SafeArrayCreate: SAFEARRAY *f(VARTYPE vt cx, UINT cDims edx, SAFEARRAYBOUND *rgsabound r8). Page: a new
 * descriptor with allocated and initialized data (zero here, inference: "initializes"), NULL if the array could not be
 * created; VT_ARRAY/VT_BYREF/VT_EMPTY/VT_NULL are not valid base types (NULL). Only cDims 1 is implemented: the pages are
 * not consistent about the order of several dimensions (the SAFEARRAY page puts the left-most bound in rgsabound[0] while
 * SafeArrayGetElement puts the right-most index in rgIndices[0], and the array functions page says the data is
 * column-major), so any other count stops. cElements 0 is not defined by the page and stops; a request above
 * FLE_SA_MAX_BYTES is NULL. */
void fle_imp_SafeArrayCreate(void)
{
    static const char api[] = "SafeArrayCreate";
    uint16_t vt = (uint16_t)R[1], feat;
    uint32_t dims = (uint32_t)R[2], es, cel;
    uint64_t bounds = R[8], total, psa, data;

    check_thread(api);
    es = sa_elem(api, vt, &feat);
    if (es == 0) {
        R[0] = 0;
        return;
    }
    if (dims != 1) fle_stop(api, "cDims is not 1: the order of several dimensions is not settled by the pages");
    if (bounds == 0) {
        R[0] = 0;
        return;
    }
    cel = (uint32_t)fle_ld(bounds, 4);
    if (cel == 0) fle_stop(api, "cElements 0 is not defined by the page");
    total = (uint64_t)cel * es;
    if (total > FLE_SA_MAX_BYTES) {
        R[0] = 0;
        return;
    }
    psa = (uint64_t)(uintptr_t)calloc(1, SA_BOUNDS + SA_BOUND_SIZE);
    data = (uint64_t)(uintptr_t)calloc(1, (size_t)total);
    if (!psa || !data) {
        free((void *)(uintptr_t)psa);
        free((void *)(uintptr_t)data);
        R[0] = 0;
        return;
    }
    fle_st(psa + SA_CDIMS, 2, 1);
    fle_st(psa + SA_FEAT, 2, feat);
    fle_st(psa + SA_CBELEM, 4, es);
    fle_st(psa + SA_PVDATA, 8, data);
    fle_st(psa + SA_BOUNDS, 4, cel);
    fle_st(psa + SA_BOUNDS + 4, 4, fle_ld(bounds + 4, 4));
    fle_pm_put(&arrays, api, psa, vt);
    R[0] = psa;
}

/* Common part of SafeArrayGetLBound/GetUBound: psa rcx, nDim edx, out r8. The first dimension is nDim 1 (the Microsoft
 * Learn samples that call these functions on one-dimensional arrays pass 1, e.g. the ADSI "Example Code for Reading
 * Attributes" and the Mobile Broadband "Working With SafeArrays" samples); nDim 0 or above cDims is DISP_E_BADINDEX
 * (page: "the specified index is out of bounds"). NULL psa or out is E_INVALIDARG. */
static void sa_bound(const char *api, int upper)
{
    uint64_t psa = R[1], out = R[8];
    uint32_t nd = (uint32_t)R[2];
    int64_t v;

    check_thread(api);
    if (psa == 0 || out == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    (void)sa_vt(api, psa);
    if (nd < 1u || nd > (uint32_t)fle_ld(psa + SA_CDIMS, 2)) {
        R[0] = FLE_HR_DISP_E_BADINDEX;
        return;
    }
    v = (int32_t)fle_ld(psa + SA_BOUNDS + SA_BOUND_SIZE * (nd - 1u) + 4, 4);
    if (upper) {
        v += (int64_t)(uint32_t)fle_ld(psa + SA_BOUNDS + SA_BOUND_SIZE * (nd - 1u), 4) - 1;
        if (v > INT32_MAX || v < INT32_MIN) {
            R[0] = FLE_HR_DISP_E_OVERFLOW;
            return;
        }
    }
    fle_st(out, 4, (uint32_t)(int32_t)v);
    R[0] = FLE_HR_S_OK;
}

/* oleaut32!SafeArrayGetLBound / SafeArrayGetUBound: HRESULT f(SAFEARRAY *psa, UINT nDim, LONG *). The upper bound is
 * lLbound + cElements - 1 and DISP_E_OVERFLOW when that leaves the 32-bit LONG (page: "overflow occurred while
 * computing the upper bound"). */
void fle_imp_SafeArrayGetLBound(void) { sa_bound("SafeArrayGetLBound", 0); }
void fle_imp_SafeArrayGetUBound(void) { sa_bound("SafeArrayGetUBound", 1); }

/* oleaut32!SafeArrayAccessData: HRESULT f(SAFEARRAY *psa rcx, void **ppvData rdx). Page: increments the lock count and
 * returns the data pointer. NULL arguments are E_INVALIDARG; a lock count at its maximum is E_UNEXPECTED ("the array
 * could not be locked"). */
void fle_imp_SafeArrayAccessData(void)
{
    static const char api[] = "SafeArrayAccessData";
    uint64_t psa = R[1], out = R[2];
    uint32_t cl;

    check_thread(api);
    if (psa == 0 || out == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    (void)sa_vt(api, psa);
    cl = (uint32_t)fle_ld(psa + SA_CLOCKS, 4);
    if (cl == UINT32_MAX) {
        R[0] = FLE_HR_E_UNEXPECTED;
        return;
    }
    fle_st(psa + SA_CLOCKS, 4, cl + 1u);
    fle_st(out, 8, fle_ld(psa + SA_PVDATA, 8));
    R[0] = FLE_HR_S_OK;
}

/* oleaut32!SafeArrayUnaccessData: HRESULT f(SAFEARRAY *psa rcx). Page: decrements the lock count. A count that is
 * already 0 is E_UNEXPECTED ("the array could not be unlocked"; the SAFEARRAY page names E_UNEXPECTED for an
 * inconsistent lock count). NULL is E_INVALIDARG. The pointer from SafeArrayAccessData stays usable (the page calls it
 * invalid; nothing is done to make it so). */
void fle_imp_SafeArrayUnaccessData(void)
{
    static const char api[] = "SafeArrayUnaccessData";
    uint64_t psa = R[1];
    uint32_t cl;

    check_thread(api);
    if (psa == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    (void)sa_vt(api, psa);
    cl = (uint32_t)fle_ld(psa + SA_CLOCKS, 4);
    if (cl == 0) {
        R[0] = FLE_HR_E_UNEXPECTED;
        return;
    }
    fle_st(psa + SA_CLOCKS, 4, cl - 1u);
    R[0] = FLE_HR_S_OK;
}

/* The element address for the indexes at rgIndices (one dimension): E_INVALIDARG for a NULL descriptor or vector,
 * DISP_E_BADINDEX when the index is outside [lLbound, lLbound + cElements - 1]. Returns the HRESULT; *elem is set on S_OK. */
static uint32_t sa_elem_addr(const char *api, uint64_t psa, uint64_t idx, uint64_t *elem)
{
    int64_t i, lb;
    uint64_t cel;

    if (psa == 0 || idx == 0) return FLE_HR_E_INVALIDARG;
    (void)sa_vt(api, psa);
    if ((uint32_t)fle_ld(psa + SA_CDIMS, 2) != 1u) fle_stop(api, "descriptor has more than one dimension");
    i = (int32_t)fle_ld(idx, 4);
    lb = (int32_t)fle_ld(psa + SA_BOUNDS + 4, 4);
    cel = (uint32_t)fle_ld(psa + SA_BOUNDS, 4);
    if (i < lb || (uint64_t)(i - lb) >= cel) return FLE_HR_DISP_E_BADINDEX;
    *elem = fle_ld(psa + SA_PVDATA, 8) + (uint64_t)(i - lb) * (uint32_t)fle_ld(psa + SA_CBELEM, 4);
    return FLE_HR_S_OK;
}

/* oleaut32!SafeArrayGetElement: HRESULT f(SAFEARRAY *psa rcx, LONG *rgIndices rdx, void *pv r8). Page: copies the element
 * to the caller's storage; "if the data element is a string, object, or variant, the function copies the element in the
 * correct way" (a new BSTR, a VARIANT copy; an object would be AddRef'd: a non-NULL pointer stops). pv is [out] and is
 * not cleared first (the page's sample VariantInit's it). NULL pv is E_INVALIDARG. */
void fle_imp_SafeArrayGetElement(void)
{
    static const char api[] = "SafeArrayGetElement";
    uint64_t psa = R[1], pv = R[8], e = 0;
    uint16_t vt;
    uint32_t hr, es;

    check_thread(api);
    hr = sa_elem_addr(api, psa, R[2], &e);
    if (hr == FLE_HR_S_OK && pv == 0) hr = FLE_HR_E_INVALIDARG;
    if (hr != FLE_HR_S_OK) {
        R[0] = hr;
        return;
    }
    vt = sa_vt(api, psa);
    es = (uint32_t)fle_ld(psa + SA_CBELEM, 4);
    if (vt == VT_BSTR) {
        fle_st(pv, 8, bstr_dup(api, fle_ld(e, 8)));
    } else if (vt == VT_UNKNOWN || vt == VT_DISPATCH) {
        if (fle_ld(e, 8) != 0) fle_stop(api, "copying an object out of a safe array needs IUnknown::AddRef (guest call-back)");
        fle_st(pv, 8, 0);
    } else if (vt == VT_VARIANT) {
        hr = var_copy_val(api, pv, e);
        if (hr != FLE_HR_S_OK) {
            R[0] = hr;
            return;
        }
    } else {
        memmove((void *)fle_addr(pv, es, 1), (const void *)fle_addr(e, es, 0), es);
    }
    R[0] = FLE_HR_S_OK;
}

/* oleaut32!SafeArrayPutElement: HRESULT f(SAFEARRAY *psa rcx, LONG *rgIndices rdx, void *pv r8). Page: stores the data;
 * for VT_BSTR/VT_DISPATCH/VT_UNKNOWN pv is the pointer itself ("no other level of indirection"); a string, object or
 * variant is copied, the existing element is cleared correctly, and a VT_DISPATCH/VT_UNKNOWN is AddRef'd (non-NULL stops;
 * so does an existing object that would be Released). A NULL pv is the NULL BSTR / NULL interface pointer for those
 * types and E_INVALIDARG for the others. */
void fle_imp_SafeArrayPutElement(void)
{
    static const char api[] = "SafeArrayPutElement";
    uint64_t psa = R[1], pv = R[8], e = 0;
    uint16_t vt;
    uint32_t hr, es;

    check_thread(api);
    hr = sa_elem_addr(api, psa, R[2], &e);
    if (hr != FLE_HR_S_OK) {
        R[0] = hr;
        return;
    }
    vt = sa_vt(api, psa);
    es = (uint32_t)fle_ld(psa + SA_CBELEM, 4);
    if (vt == VT_BSTR) {
        uint64_t q = bstr_dup(api, pv);

        bstr_free(api, fle_ld(e, 8));
        fle_st(e, 8, q);
    } else if (vt == VT_UNKNOWN || vt == VT_DISPATCH) {
        if (pv != 0) fle_stop(api, "storing an object in a safe array needs IUnknown::AddRef (guest call-back)");
        if (fle_ld(e, 8) != 0) fle_stop(api, "replacing an object in a safe array needs IUnknown::Release (guest call-back)");
    } else if (pv == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    } else if (vt == VT_VARIANT) {
        if (classify(api, (uint16_t)fle_ld(pv + VAR_VT, 2)) == K_BAD) {
            R[0] = FLE_HR_DISP_E_BADVARTYPE;
            return;
        }
        hr = var_clear(api, e);
        if (hr != FLE_HR_S_OK) {
            R[0] = hr;
            return;
        }
        hr = var_copy_val(api, e, pv);
        if (hr != FLE_HR_S_OK) {
            R[0] = hr;
            return;
        }
    } else {
        memmove((void *)fle_addr(e, es, 1), (const void *)fle_addr(pv, es, 0), es);
    }
    R[0] = FLE_HR_S_OK;
}

/* oleaut32!SafeArrayPtrOfIndex: HRESULT f(SAFEARRAY *psa rcx, LONG *rgIndices rdx, void **ppvData r8). Page: a pointer to
 * the element; the array should be locked (not checked: the page only warns). NULL arguments are E_INVALIDARG; an index
 * outside the bounds is DISP_E_BADINDEX. */
void fle_imp_SafeArrayPtrOfIndex(void)
{
    static const char api[] = "SafeArrayPtrOfIndex";
    uint64_t e = 0, out = R[8];
    uint32_t hr;

    check_thread(api);
    hr = sa_elem_addr(api, R[1], R[2], &e);
    if (hr == FLE_HR_S_OK && out == 0) hr = FLE_HR_E_INVALIDARG;
    if (hr == FLE_HR_S_OK) fle_st(out, 8, e);
    R[0] = hr;
}

/* oleaut32!SafeArrayGetElemsize: UINT f(SAFEARRAY *psa rcx). Page: the size of an element in bytes. The page names no
 * result for NULL, so NULL stops. */
void fle_imp_SafeArrayGetElemsize(void)
{
    static const char api[] = "SafeArrayGetElemsize";

    check_thread(api);
    if (R[1] == 0) fle_stop(api, "psa is NULL (the page names no result)");
    (void)sa_vt(api, R[1]);
    R[0] = (uint32_t)fle_ld(R[1] + SA_CBELEM, 4);
}

/* oleaut32!GetErrorInfo: HRESULT f(ULONG dwReserved ecx, IErrorInfo **pperrinfo rdx). Page: returns the IErrorInfo most
 * recently set by SetErrorInfo in the current logical thread, or S_FALSE when there is none. The virtual environment has
 * no COM object that could call SetErrorInfo (the closure does not import it), so there is never an error object: S_FALSE.
 * *pperrinfo is set to NULL (setting: the page does not say what an [out] pointer holds with S_FALSE; a caller that
 * tests only the result is unaffected). dwReserved must be zero on the page: other values, and a NULL pperrinfo, are
 * not defined and stop. */
void fle_imp_GetErrorInfo(void)
{
    static const char api[] = "GetErrorInfo";

    check_thread(api);
    if ((uint32_t)R[1] != 0) fle_stop(api, "dwReserved is not zero (the page: must be zero)");
    if (R[2] == 0) fle_stop(api, "pperrinfo is NULL");
    fle_st(R[2], 8, 0);
    R[0] = FLE_HR_S_FALSE;
}
