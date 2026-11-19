#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_realmod.h"
#include "fle_res.h"
#include "fle_loadimage.h"

/* Resource APIs (docs/decisions/0025-resource-apis.md). All of them read the PE resource directory (data
 * directory 2) of the image mapped at FLE_EXE_HANDLE, the lifted DLL itself, or (decisions 0102 W2 and 0104, provisional) of the
 * image of a real module (fle_realmod.h), selected by its handle (= base). Directory format: Microsoft Learn
 * "PE Format", section "The .rsrc Section". RT_STRING block format: "Resource Types", String Table Resources.
 *
 * Rules shared by the five stubs (each is spelled out where the Microsoft page is silent):
 *  - hModule NULL or FLE_EXE_HANDLE selects the lifted DLL (FindResourceW, LoadResource, SizeofResource: "NULL:
 *    the module used to create the current process", which is the lifted DLL, decision 0015). The handle (= base) of a
 *    real module selects that module's image (decisions 0102 W2, 0104). The handle of any other virtual module, or any
 *    other value, stops the run: no other module has resources here.
 *  - An argument that the page defines as "returned by an earlier call" (HRSRC, HGLOBAL) must be exactly what the
 *    earlier call returned for this module; anything else, including NULL, stops the run (the pages define no
 *    behaviour for it).
 *  - A name or type given as a string is an integer ID when the string is "#" and decimal digits (page); other
 *    strings starting with '#', and IDs above 0xFFFF written that way, stop the run. A plain string is compared
 *    with the directory name with ASCII case folding (decision: the page does not say).
 *  - Several language leaves under one (type, name): the choice is not given by the FindResourceW page, so the run
 *    stops. FLEngine_x64.dll has none (every name has exactly one leaf, measured by fe-pe's resource parser). */

#define RES_NAME_MAX  256u
#define RT_STRING_ID  6u
#define RES_ID_MAX    0xFFFFu
#define LOADSTR_ID_MAX 0xFFFFu

typedef struct {
    uint64_t base;       /* where the image is mapped: FLE_EXE_HANDLE, or the base of a real module (decision 0104) */
    uint64_t dir;        /* VA of the resource directory (0 when the image has none) */
    uint32_t dir_rva;
    uint32_t dir_size;
    uint32_t image_size;
} res_img;

typedef struct {
    int is_id;
    uint32_t id;
    uint16_t name[RES_NAME_MAX + 1];
    size_t nlen;
} res_key;

/* Returns the base of the image whose resources are meant: FLE_EXE_HANDLE, or the base of a real module (fle_realmod.h, decision 0104:
 * the handle of a real module is its base, and its resources are in the image mapped there). */
static uint64_t check_module(const char *api, uint64_t h, int null_ok)
{
    uint64_t i;
    if (h == FLE_EXE_HANDLE) return h;
    if (h == 0) {
        if (null_ok) return FLE_EXE_HANDLE;
        fle_stop(api, "hInstance is NULL (the page does not define it)");
    }
    if (FLE_REALMOD_PRESENT() && fle_realmod_find_handle(h) >= 0) return h;
    for (i = 0; i < fle_module_count(); i++) {
        if (fle_module_handle(i) == h) {
            fprintf(stderr, "fle: %s: module %s has no resources in this runtime (only the lifted DLL does)\n", api,
                    fle_module_name(i));
            abort();
        }
    }
    fprintf(stderr, "fle: %s: hModule %#llx is not a module handle of this runtime\n", api, (unsigned long long)h);
    abort();
}

static void img_open(const char *api, res_img *im, uint64_t b)
{
    uint64_t nt;
    uint32_t lfanew, nrva;

    im->base = b;
    if ((uint16_t)fle_ld(b, 2) != 0x5a4d) fle_stop(api, "no PE image is mapped at 0x400000 (no MZ signature)");
    lfanew = (uint32_t)fle_ld(b + 0x3c, 4);
    if (lfanew < 0x40 || lfanew > 0x1000) fle_stop(api, "e_lfanew of the mapped image is out of range");
    nt = b + lfanew;
    if ((uint32_t)fle_ld(nt, 4) != 0x4550) fle_stop(api, "no PE signature in the mapped image");
    if ((uint16_t)fle_ld(nt + 24, 2) != 0x20b) fle_stop(api, "the mapped image is not PE32+");
    if (fle_ld(nt + 24 + 24, 8) != b) fle_stop(api, "ImageBase of the mapped image is not the address it is mapped at");
    im->image_size = (uint32_t)fle_ld(nt + 24 + 56, 4);
    nrva = (uint32_t)fle_ld(nt + 24 + 108, 4);
    im->dir_rva = 0;
    im->dir_size = 0;
    im->dir = 0;
    if (nrva > 2) {
        im->dir_rva = (uint32_t)fle_ld(nt + 24 + 112 + 2 * 8, 4);
        im->dir_size = (uint32_t)fle_ld(nt + 24 + 112 + 2 * 8 + 4, 4);
    }
    if (im->dir_rva == 0) return;
    if ((uint64_t)im->dir_rva + im->dir_size > im->image_size) fle_stop(api, "resource directory lies outside the image");
    im->dir = b + im->dir_rva;
}

/* Bounds-checked reads inside the resource directory (offsets are relative to its start). */
static uint32_t d32(const char *api, const res_img *im, uint64_t off)
{
    if (off + 4 > im->dir_size) fle_stop(api, "malformed resource directory (offset outside the directory)");
    return (uint32_t)fle_ld(im->dir + off, 4);
}

static uint16_t d16(const char *api, const res_img *im, uint64_t off)
{
    if (off + 2 > im->dir_size) fle_stop(api, "malformed resource directory (offset outside the directory)");
    return (uint16_t)fle_ld(im->dir + off, 2);
}

/* Reads an LPCWSTR argument that may be MAKEINTRESOURCE(id): IS_INTRESOURCE is "the value above bit 15 is 0". */
static void key_from_guest(const char *api, uint64_t v, res_key *k)
{
    size_t i;
    uint32_t n = 0;

    k->nlen = 0;
    if ((v >> 16) == 0) {
        k->is_id = 1;
        k->id = (uint32_t)v;
        return;
    }
    k->is_id = 0;
    k->id = 0;
    k->nlen = fle_w_read(api, v, k->name, RES_NAME_MAX + 1);
    if (k->nlen > 0 && k->name[0] == '#') {
        if (k->nlen == 1) fle_stop(api, "\"#\" without digits as a resource name or type is not defined");
        for (i = 1; i < k->nlen; i++) {
            if (k->name[i] < '0' || k->name[i] > '9') fle_stop(api, "resource name or type \"#...\" is not a decimal number (not defined)");
            n = n * 10u + (uint32_t)(k->name[i] - '0');
            if (n > RES_ID_MAX) fle_stop(api, "resource name or type \"#<number>\" above 65535 is not defined");
        }
        k->is_id = 1;
        k->id = n;
    }
}

static int name_equal(const char *api, const res_img *im, uint32_t field, const res_key *k)
{
    uint32_t o = field & 0x7fffffffu;
    uint32_t n = d16(api, im, o), j;
    if (n != k->nlen) return 0;
    for (j = 0; j < n; j++) {
        if (fle_w_fold(d16(api, im, o + 2u + 2u * j)) != fle_w_fold(k->name[j])) return 0;
    }
    return 1;
}

/* Finds the entry for `k` in the directory table at `off`; ID entries follow the name entries. */
static int table_find(const char *api, const res_img *im, uint32_t off, const res_key *k, uint32_t *field)
{
    uint32_t nn = d16(api, im, off + 12), ni = d16(api, im, off + 14), i;

    if (k->is_id) {
        for (i = nn; i < nn + ni; i++) {
            uint32_t a = d32(api, im, off + 16u + 8u * i);
            if (a >> 31) fle_stop(api, "malformed resource directory (ID entry with the string flag)");
            if (a == k->id) {
                *field = d32(api, im, off + 20u + 8u * i);
                return 1;
            }
        }
    } else {
        for (i = 0; i < nn; i++) {
            uint32_t a = d32(api, im, off + 16u + 8u * i);
            if (!(a >> 31)) fle_stop(api, "malformed resource directory (name entry without the string flag)");
            if (name_equal(api, im, a, k)) {
                *field = d32(api, im, off + 20u + 8u * i);
                return 1;
            }
        }
    }
    return 0;
}

/* (type, name) -> data entry offset. Returns 0, or the error code FindResourceW sets (decision 0025). */
static uint32_t find_leaf(const char *api, const res_img *im, const res_key *type, const res_key *name, uint32_t *entry)
{
    uint32_t f, off, n;

    if (im->dir_rva == 0) return FLE_ERROR_RESOURCE_DATA_NOT_FOUND;
    if (!table_find(api, im, 0, type, &f)) return FLE_ERROR_RESOURCE_TYPE_NOT_FOUND;
    if (!(f >> 31)) fle_stop(api, "malformed resource directory (data entry at the type level)");
    if (!table_find(api, im, f & 0x7fffffffu, name, &f)) return FLE_ERROR_RESOURCE_NAME_NOT_FOUND;
    if (!(f >> 31)) fle_stop(api, "malformed resource directory (data entry at the name level)");
    off = f & 0x7fffffffu;
    n = (uint32_t)d16(api, im, off + 12) + (uint32_t)d16(api, im, off + 14);
    if (n == 0) return FLE_ERROR_RESOURCE_LANG_NOT_FOUND;
    if (n > 1) fle_stop(api, "several language leaves for one (type, name): the language choice is not defined");
    f = d32(api, im, off + 20);
    if (f >> 31) fle_stop(api, "malformed resource directory (subdirectory at the language level)");
    *entry = f;
    return 0;
}

/* Data entry (16 bytes at `entry`): data VA and size, checked against the image. */
static void leaf_data(const char *api, const res_img *im, uint32_t entry, uint64_t *va, uint32_t *size)
{
    uint32_t rva = d32(api, im, entry), sz = d32(api, im, entry + 4u);
    if ((uint64_t)rva + sz > im->image_size) fle_stop(api, "resource data lies outside the image");
    *va = im->base + rva;
    *size = sz;
}

#define WALK_ENTRY 0 /* match the VA of the data entry (an HRSRC) */
#define WALK_DATA  1 /* match the VA of the data (an HGLOBAL from LoadResource) */

/* Walks every leaf of the tree; returns the data entry offset whose HRSRC or data VA equals `value`, or
 * UINT32_MAX. */
static uint32_t walk(const char *api, const res_img *im, int mode, uint64_t value)
{
    uint32_t nt, i, j, k, nj, nk, ti, nf, lf;

    if (im->dir_rva == 0) return UINT32_MAX;
    nt = (uint32_t)d16(api, im, 12) + (uint32_t)d16(api, im, 14);
    for (i = 0; i < nt; i++) {
        ti = d32(api, im, 20u + 8u * i);
        if (!(ti >> 31)) fle_stop(api, "malformed resource directory (data entry at the type level)");
        ti &= 0x7fffffffu;
        nj = (uint32_t)d16(api, im, ti + 12) + (uint32_t)d16(api, im, ti + 14);
        for (j = 0; j < nj; j++) {
            nf = d32(api, im, ti + 20u + 8u * j);
            if (!(nf >> 31)) fle_stop(api, "malformed resource directory (data entry at the name level)");
            nf &= 0x7fffffffu;
            nk = (uint32_t)d16(api, im, nf + 12) + (uint32_t)d16(api, im, nf + 14);
            for (k = 0; k < nk; k++) {
                lf = d32(api, im, nf + 20u + 8u * k);
                if (lf >> 31) fle_stop(api, "malformed resource directory (subdirectory at the language level)");
                if (mode == WALK_ENTRY) {
                    if (im->dir + lf == value) return lf;
                } else {
                    if (im->base + d32(api, im, lf) == value) return lf;
                }
            }
        }
    }
    return UINT32_MAX;
}

/* kernel32!FindResourceW (Microsoft Learn: FindResourceW). hModule rcx, lpName rdx, lpType r8 (the name comes
 * before the type). Success returns the HRSRC, here the VA of the 16-byte Resource Data Entry inside the mapped
 * image (decision 0025: the page says only "a handle to the specified resource's information block"; this value
 * is a decision and is what LoadResource and SizeofResource validate). Failure returns NULL with the error code
 * from find_leaf (the page names no code). Success leaves the last error unchanged (the page does not say). */
void fle_imp_FindResourceW(void)
{
    static const char api[] = "FindResourceW";
    uint64_t h = R[1], pname = R[2], ptype = R[8];
    res_img im;
    res_key kt, kn;
    uint32_t entry = 0, err;

    img_open(api, &im, check_module(api, h, 1));
    key_from_guest(api, ptype, &kt);
    key_from_guest(api, pname, &kn);
    err = find_leaf(api, &im, &kt, &kn, &entry);
    if (err != 0) {
        fle_set_last_error(err);
        R[0] = 0;
        return;
    }
    R[0] = im.dir + entry;
}

/* kernel32!LoadResource (Microsoft Learn: LoadResource). hModule rcx, hResInfo rdx. Returns a handle to the data;
 * the page says it is not a real HGLOBAL and only LockResource and SizeofResource use it. Here it is the VA of
 * the first data byte in the mapped image (decision 0025). hResInfo must be an HRSRC from FindResourceW for this
 * module; NULL or any other value stops the run. */
void fle_imp_LoadResource(void)
{
    static const char api[] = "LoadResource";
    uint64_t h = R[1], hres = R[2], va;
    res_img im;
    uint32_t entry, size;

    img_open(api, &im, check_module(api, h, 1));
    if (hres == 0) fle_stop(api, "hResInfo is NULL (the page defines only a handle returned by FindResource)");
    entry = walk(api, &im, WALK_ENTRY, hres);
    if (entry == UINT32_MAX) fle_stop(api, "hResInfo is not a resource handle returned by FindResourceW for this module");
    leaf_data(api, &im, entry, &va, &size);
    R[0] = va;
}

/* kernel32!LockResource (Microsoft Learn: LockResource). hResData rcx. "Do not pass any value as a parameter
 * other than a successful return value from LoadResource": any other value, including NULL, stops the run. The
 * result is the same VA (the image is mapped, nothing is locked). */
void fle_imp_LockResource(void)
{
    static const char api[] = "LockResource";
    uint64_t hres = R[1];
    res_img im;
    int found, k;
    uint64_t b;

    /* LockResource gets no module: the data VA belongs to the image of the lifted DLL or to the one of a real module (decision 0104) */
    img_open(api, &im, FLE_EXE_HANDLE);
    if (hres == 0) fle_stop(api, "hResData is NULL (the page allows only a successful LoadResource result)");
    found = walk(api, &im, WALK_DATA, hres) != UINT32_MAX;
    for (k = 0; !found && FLE_REALMOD_PRESENT() && (b = fle_realmod_base_of(k)) != 0; k++) {
        img_open(api, &im, b);
        found = walk(api, &im, WALK_DATA, hres) != UINT32_MAX;
    }
    if (!found) fle_stop(api, "hResData is not a LoadResource result for the lifted DLL");
    R[0] = hres;
}

/* kernel32!SizeofResource (Microsoft Learn: SizeofResource). hModule rcx, hResInfo rdx. Returns the size field of
 * the Resource Data Entry. hResInfo as for LoadResource. The page names no failure for a valid handle. */
void fle_imp_SizeofResource(void)
{
    static const char api[] = "SizeofResource";
    uint64_t h = R[1], hres = R[2], va;
    res_img im;
    uint32_t entry, size;

    img_open(api, &im, check_module(api, h, 1));
    if (hres == 0) fle_stop(api, "hResInfo is NULL (the page defines only a handle returned by FindResource)");
    entry = walk(api, &im, WALK_ENTRY, hres);
    if (entry == UINT32_MAX) fle_stop(api, "hResInfo is not a resource handle returned by FindResourceW for this module");
    leaf_data(api, &im, entry, &va, &size);
    R[0] = size;
}

/* kernel32!FreeResource (Microsoft Learn: FreeResource, read 2026-10-31; decision 0123, PROVISIONAL, wall 26). BOOL f(HGLOBAL hResData rcx). The page: "This function is obsolete and is only supported for backward compatibility with 16-bit
 * Windows. For 32-bit Windows applications, it is not necessary to free the resources loaded using LoadResource. For modern versions of Windows this function always returns FALSE."; and: "If the function succeeds, the return
 * value is zero. If the function fails, the return value is nonzero, which indicates that the resource has not been freed". The two sentences are read together: the call returns FALSE (0) and frees nothing (the data is the
 * mapped image: nothing to free). The argument is not looked at (the page says it is "assumed" to be a LoadResource result; a value that is not one is not an error of the always-FALSE function). The last error is not touched. */
void fle_imp_FreeResource(void)
{
    R[0] = 0;
}

/* user32!LoadStringW (Microsoft Learn: LoadStringW). hInstance rcx, uID edx, lpBuffer r8, cchBufferMax r9d.
 *  - hInstance: the lifted DLL (0x400000). NULL is marked "optional" on the page but the page does not say what it
 *    selects, so it stops the run (decision 0025); other modules stop as in check_module.
 *  - The string is in the RT_STRING block number (uID >> 4) + 1, index uID & 15: a block is 16 strings, each a
 *    WORD length in UTF-16 units and the units, no terminating NUL; an unused ID has length 0 (Resource Types).
 *  - cchBufferMax > 0: copies min(length, cchBufferMax - 1) units, appends a NUL, returns the units copied (page).
 *  - cchBufferMax == 0: writes the 8-byte pointer to the string inside the image to *lpBuffer and returns the
 *    length (page); the string is not NUL-terminated there.
 *  - A missing string returns 0 (page). The page says "call GetLastError" but names no code: a missing block
 *    sets the code FindResourceW would set (ERROR_RESOURCE_TYPE_NOT_FOUND / ERROR_RESOURCE_NAME_NOT_FOUND), an
 *    unused ID in an existing block sets ERROR_RESOURCE_NAME_NOT_FOUND (decision 0025, not from a page). The
 *    buffer is left untouched on failure (the page does not say it is written).
 *  - A negative cchBufferMax, a NULL lpBuffer when it is used, and uID above 0xFFFF (STRINGTABLE IDs are 16-bit
 *    on the STRINGTABLE page) are not defined and stop the run. */
void fle_imp_LoadStringW(void)
{
    static const char api[] = "LoadStringW";
    uint64_t h = R[1], buf = R[8], data, p = 0;
    uint32_t uid = (uint32_t)R[2], entry = 0, size, err, idx, i, len;
    int32_t cch = (int32_t)(uint32_t)R[9];
    res_img im;
    res_key kt, kn;

    uint64_t base = check_module(api, h, 0);
    if (cch < 0) fle_stop(api, "cchBufferMax is negative (not defined)");
    if (uid > LOADSTR_ID_MAX) fle_stop(api, "uID above 0xFFFF (STRINGTABLE IDs are 16-bit; not defined)");
    if (buf == 0) fle_stop(api, "lpBuffer is NULL (it receives the string or a pointer, not defined as optional)");
    img_open(api, &im, base);

    kt.is_id = 1;
    kt.id = RT_STRING_ID;
    kt.nlen = 0;
    kn.is_id = 1;
    kn.id = (uid >> 4) + 1u;
    kn.nlen = 0;
    err = find_leaf(api, &im, &kt, &kn, &entry);
    if (err != 0) {
        fle_set_last_error(err);
        R[0] = 0;
        return;
    }
    leaf_data(api, &im, entry, &data, &size);

    idx = uid & 15u;
    for (i = 0;; i++) {
        if (p + 2 > size) fle_stop(api, "malformed string block (shorter than its 16 strings)");
        len = (uint16_t)fle_ld(data + p, 2);
        if (p + 2 + 2ull * len > size) fle_stop(api, "malformed string block (a length runs past the block)");
        if (i == idx) break;
        p += 2 + 2ull * len;
    }
    if (len == 0) {
        fle_set_last_error(FLE_ERROR_RESOURCE_NAME_NOT_FOUND);
        R[0] = 0;
        return;
    }
    if (cch == 0) {
        fle_st(buf, 8, data + p + 2);
        R[0] = len;
        return;
    }
    {
        uint32_t n = len < (uint32_t)(cch - 1) ? len : (uint32_t)(cch - 1);
        memcpy((void *)fle_addr(buf, (unsigned)(2u * n), 1), (const void *)fle_addr(data + p + 2, (unsigned)(2u * n), 0), 2ull * n);
        fle_st(buf + 2ull * n, 2, 0);
        R[0] = n;
    }
}

/* Does the lifted DLL have the RT_GROUP_CURSOR with integer id `id` (decision 0061, used by LoadImageW with hInst = the DLL)?
 * The group is a NEWHEADER (Reserved 0, ResType RES_CURSOR = 2, ResCount) and ResCount RESDIR entries of 14 bytes, the last
 * member of each being the id of an RT_CURSOR component (Learn: Resource File Formats, NEWHEADER, RESDIR). Returns 1 when the
 * group exists and every component it names exists as an RT_CURSOR; 0 when the group or a component is absent. The last
 * error is not touched. A group that does not have the documented form stops the run. */
static int cursor_group_ok_key(const char *api, const res_img *imp, const res_key *knp)
{
    res_img im = *imp;
    res_key kt, kn = *knp;
    uint64_t data;
    uint32_t entry, size, count, i;

    kt.is_id = 1;
    kt.id = 12u; /* RT_GROUP_CURSOR = RT_CURSOR + 11 (Learn: Resource Types) */
    kt.nlen = 0;
    if (find_leaf(api, &im, &kt, &kn, &entry) != 0) return 0;
    leaf_data(api, &im, entry, &data, &size);
    if (size < 6u) fle_stop(api, "RT_GROUP_CURSOR is shorter than its NEWHEADER");
    if ((uint16_t)fle_ld(data, 2) != 0 || (uint16_t)fle_ld(data + 2, 2) != 2) fle_stop(api, "RT_GROUP_CURSOR NEWHEADER is not (Reserved 0, ResType RES_CURSOR)");
    count = (uint16_t)fle_ld(data + 4, 2);
    if (count == 0 || 6ull + 14ull * count > size) fle_stop(api, "RT_GROUP_CURSOR ResCount does not fit the resource");
    kt.id = 1u; /* RT_CURSOR */
    for (i = 0; i < count; i++) {
        kn.is_id = 1;
        kn.nlen = 0;
        kn.id = (uint16_t)fle_ld(data + 6u + 14u * i + 12u, 2);
        if (find_leaf(api, &im, &kt, &kn, &entry) != 0) return 0;
    }
    return 1;
}

int fle_res_cursor_group_ok_at(const char *api, uint64_t base, uint32_t id)
{
    res_img im;
    res_key kn;

    img_open(api, &im, base);
    kn.is_id = 1;
    kn.id = id;
    kn.nlen = 0;
    return cursor_group_ok_key(api, &im, &kn);
}

/* The same for the lifted DLL (hInst = FLE_EXE_HANDLE). */
int fle_res_cursor_group_ok(const char *api, uint32_t id)
{
    return fle_res_cursor_group_ok_at(api, FLE_EXE_HANDLE, id);
}

/* Decision 0118 wall 8 (provisional): the icon of an RT_GROUP_ICON that user32!LoadIconW(hInstance, name) loads from a module image. LoadIconW is in fle_gdi.c, which
 * reaches this function through a weak reference (its source test forbids this file's name). Learn (LoadIconW): "If hInstance is non-NULL, lpIconName specifies the icon
 * resource either by name or ordinal"; "The function searches the icon resource for the icon most appropriate for the current display"; "LoadIcon can only load an icon whose
 * size conforms to the SM_CXICON and SM_CYICON system metric values". Learn (LookupIconIdFromDirectoryEx): "The image closest but not exceed the requested size is selected. If
 * two or more images of that size are present, the one that matches the color depth of the display is chosen. If no images exactly match the color depth of the display, the
 * image with the greatest color depth that does not exceed the color depth of the display is chosen. If all exceed the color depth, the one with the lowest color depth is
 * chosen." Learn (NEWHEADER, RESDIR, ICONRESDIR): the group is a NEWHEADER (Reserved 0, ResType RES_ICON = 1, ResCount) and ResCount RESDIR entries of 14 bytes: Width and
 * Height (a BYTE each; "The value 0 is accepted as representing a width of 256"), ColorCount, reserved, Planes, BitCount, BytesInRes, IconCursorId (the id of the RT_ICON).
 * The size is cx by cy; "closest but not exceed" is read as the largest area among the entries with Width <= cx and Height <= cy (the page gives no measure of closeness
 * for a non-square size; the entries of FLEngine are square); the depth of an entry is Planes * BitCount (the pages do not say which field a depth is; every entry of
 * FLEngine has one plane, and an entry with another number of planes stops). Two entries that tie after the rules (same size and same depth) are not decided by the page:
 * the run stops. The RT_ICON that the chosen entry names must exist and its BITMAPINFOHEADER must agree with the entry (biWidth = Width, biHeight = 2 * Height for the XOR
 * and AND images, biBitCount = BitCount); a PNG-compressed component (the "Vista" form) is not read and stops.
 * out[0] = the VA of the group's data (the identity of the group in the image), out[1] = Width, out[2] = Height, out[3] = depth, out[4] = IconCursorId. Returns 1; 0 when the
 * module has no RT_GROUP_ICON of that name (the caller decides: the LoadIcon page names no error code). The last error is not touched. */
int fle_icon_group_pick(const char *api, uint64_t base, uint64_t name, uint32_t cx, uint32_t cy, uint32_t depth, uint64_t out[5])
{
    res_img im;
    res_key kt, kn;
    uint64_t data, cdata;
    uint32_t entry, size, csize, count, i, w, h, d, bw = 0, bh = 0, bd = 0, area, best_area = 0, tie = 0;
    uint16_t bid = 0;
    int have = 0;

    base = check_module(api, base, 0);
    img_open(api, &im, base);
    kt.is_id = 1;
    kt.id = 14u; /* RT_GROUP_ICON = RT_ICON + 11 (Learn: Resource Types) */
    kt.nlen = 0;
    key_from_guest(api, name, &kn);
    if (find_leaf(api, &im, &kt, &kn, &entry) != 0) return 0;
    leaf_data(api, &im, entry, &data, &size);
    if (size < 6u) fle_stop(api, "RT_GROUP_ICON is shorter than its NEWHEADER");
    if ((uint16_t)fle_ld(data, 2) != 0 || (uint16_t)fle_ld(data + 2, 2) != 1) fle_stop(api, "RT_GROUP_ICON NEWHEADER is not (Reserved 0, ResType RES_ICON)");
    count = (uint16_t)fle_ld(data + 4, 2);
    if (count == 0 || 6ull + 14ull * count > size) fle_stop(api, "RT_GROUP_ICON ResCount does not fit the resource");
    for (i = 0; i < count; i++) {
        uint64_t e = data + 6u + 14u * i;

        w = (uint8_t)fle_ld(e, 1);
        h = (uint8_t)fle_ld(e + 1, 1);
        if (w == 0) w = 256;
        if (h == 0) h = 256;
        if ((uint16_t)fle_ld(e + 4, 2) != 1) fle_stop(api, "an RT_GROUP_ICON entry has Planes other than 1: the pages do not say how a depth is made of it");
        if (w > cx || h > cy) continue;
        area = w * h;
        if (area > best_area) {
            best_area = area;
            bw = w;
            bh = h;
        }
    }
    if (best_area == 0) fle_stop(api, "every entry of the RT_GROUP_ICON exceeds the requested size: the page gives no rule for it");
    for (i = 0; i < count; i++) {
        uint64_t e = data + 6u + 14u * i;
        int better;

        w = (uint8_t)fle_ld(e, 1);
        h = (uint8_t)fle_ld(e + 1, 1);
        if (w == 0) w = 256;
        if (h == 0) h = 256;
        if (w != bw || h != bh) continue;
        d = (uint16_t)fle_ld(e + 6, 2);
        if (!have) better = 1;
        else if (bd == depth) better = 0;                       /* an exact match is kept */
        else if (d == depth) better = 1;
        else if (bd < depth) better = d < depth && d > bd;      /* the greatest that does not exceed the display */
        else better = d < bd;                                   /* all so far exceed: the lowest */
        if (have && d == bd) tie++;
        if (better) {
            bd = d;
            bid = (uint16_t)fle_ld(e + 12, 2);
            tie = 0;
            have = 1;
        }
    }
    if (tie != 0) fle_stop(api, "two entries of the RT_GROUP_ICON have the same size and the same depth: the page does not say which one is chosen");
    kt.id = 3u; /* RT_ICON */
    kn.is_id = 1;
    kn.id = bid;
    kn.nlen = 0;
    if (find_leaf(api, &im, &kt, &kn, &entry) != 0) fle_stop(api, "the RT_ICON that the chosen RT_GROUP_ICON entry names does not exist");
    leaf_data(api, &im, entry, &cdata, &csize);
    if (csize >= 4u && (uint32_t)fle_ld(cdata, 4) == 0x474e5089u) fle_stop(api, "the chosen RT_ICON is PNG-compressed: not read");
    if (csize < 40u || (uint32_t)fle_ld(cdata, 4) != 40u || (uint32_t)fle_ld(cdata + 4, 4) != bw || (uint32_t)fle_ld(cdata + 8, 4) != 2u * bh ||
        (uint16_t)fle_ld(cdata + 14, 2) != bd)
        fle_stop(api, "the BITMAPINFOHEADER of the chosen RT_ICON does not agree with the RT_GROUP_ICON entry");
    out[0] = data;
    out[1] = bw;
    out[2] = bh;
    out[3] = bd;
    out[4] = bid;
    return 1;
}

/* Decision 0125 (PROVISIONAL), wall 54: user32!LoadCursorW(hInstance, lpCursorName) with a non-NULL hInstance, for the cursors of the guest's own resources (the guest's Delphi Screen loads its custom cursors
 * by a string name: f_9f2870 passes the image base and L"VT_HEADERSPLIT"). fle_cursor.c reaches this function through a weak reference (its source test forbids this file's name). Learn (LoadCursorW): "If
 * hInstance is non-NULL, lpCursorName specifies the cursor resource either by name or ordinal"; "The LoadCursor function loads the cursor resource only if it has not been loaded; otherwise, it retrieves the
 * handle to the existing resource" (one handle for one cursor: the handle is a pure function of the arguments); the return value is "the handle to the newly loaded cursor", NULL on failure ("To get extended
 * error information, call GetLastError", no value named). The cursor of a name is the RT_GROUP_CURSOR of that name in the image of hInstance (the lifted DLL or a real module, the way LoadImageW picks it, decisions
 * 0061 and 0104); no image and no hot spot is read. Its handle is FLE_LI_MODULE_HANDLE_BASE + FLE_LI_REALMOD_STRIDE * k + 4 * (FLE_LI_NAMED_FIRST + p), k = 0 for the lifted DLL and index + 1 for a real module, p the
 * position of the name among the NAMED entries of the RT_GROUP_CURSOR directory (the directory keeps names in a fixed order: the same name is the same handle on every call and in every run): A CHOICE WITHOUT A
 * SOURCE (S-loadcursor-handle; Learn gives no handle value). An ordinal (MAKEINTRESOURCE) in 32762..32767 gets the handle LoadImageW gives for it (the same shared resource); every other ordinal stops (no guest call
 * loads one). A name or ordinal with no RT_GROUP_CURSOR returns NULL with the last error the resource search found (ERROR_RESOURCE_TYPE_NOT_FOUND, _NAME_NOT_FOUND or _DATA_NOT_FOUND: a CHOICE WITHOUT A SOURCE,
 * S-loadcursor-missing: the FindResource page names none and the LoadCursor page names none). A module that is neither the lifted DLL nor a real module stops (check_module). Returns 1 with *handle set, or 0 with
 * *handle = 0 and the last error set. */
int fle_cursor_load_named(const char *api, uint64_t inst, uint64_t name, uint64_t *handle)
{
    res_img im;
    res_key kn, kt;
    uint64_t base = check_module(api, inst, 0), h = FLE_LI_MODULE_HANDLE_BASE;
    uint32_t f, off, nn, i;
    int rm = -1;

    if (base != FLE_EXE_HANDLE) {
        rm = FLE_REALMOD_PRESENT() ? fle_realmod_find_handle(base) : -1;
        if (rm < 0) fle_stop(api, "hInstance is neither the lifted DLL nor a real module: not answered (decision 0125)");
        h += FLE_LI_REALMOD_STRIDE * (uint64_t)(rm + 1);
    }
    img_open(api, &im, base);
    key_from_guest(api, name, &kn);
    if (kn.is_id) {
        if (kn.id < FLE_LI_MODULE_FIRST || kn.id > FLE_LI_MODULE_LAST) fle_stop(api, "lpCursorName is an ordinal other than 32762..32767: not answered (decision 0125; the guest loads no other ordinal)");
        if (!cursor_group_ok_key(api, &im, &kn)) {
            fle_set_last_error(FLE_ERROR_RESOURCE_NAME_NOT_FOUND);
            *handle = 0;
            return 0;
        }
        *handle = h + 4u * (uint64_t)(kn.id - FLE_LI_MODULE_FIRST + 1u);
        return 1;
    }
    kt.is_id = 1;
    kt.id = 12u;
    kt.nlen = 0;
    if (im.dir_rva == 0 || !table_find(api, &im, 0, &kt, &f)) {
        fle_set_last_error(im.dir_rva == 0 ? FLE_ERROR_RESOURCE_DATA_NOT_FOUND : FLE_ERROR_RESOURCE_TYPE_NOT_FOUND);
        *handle = 0;
        return 0;
    }
    if (!(f >> 31)) fle_stop(api, "malformed resource directory (data entry at the type level)");
    off = f & 0x7fffffffu;
    nn = d16(api, &im, off + 12);
    for (i = 0; i < nn; i++) {
        uint32_t a = d32(api, &im, off + 16u + 8u * i);

        if (!(a >> 31)) fle_stop(api, "malformed resource directory (name entry without the string flag)");
        if (name_equal(api, &im, a, &kn)) break;
    }
    if (i == nn || !cursor_group_ok_key(api, &im, &kn)) {
        fle_set_last_error(FLE_ERROR_RESOURCE_NAME_NOT_FOUND);
        *handle = 0;
        return 0;
    }
    if (i >= FLE_LI_NAMED_MAX) fle_stop(api, "more than 256 named RT_GROUP_CURSOR entries: the handle form has room for 256 (decision 0125)");
    *handle = h + 4u * (uint64_t)(FLE_LI_NAMED_FIRST + i);
    return 1;
}

/* Decision 0129 (PROVISIONAL), S-version-info-real-resource: the bytes of the RT_VERSION resource (type 16) of the image mapped at FLE_EXE_HANDLE (the lifted DLL, FLEngine_x64.dll), for
 * version!GetFileVersionInfoSizeW / GetFileVersionInfoW / VerQueryValueW (fle_vfs.c). It is the resource that the PE holds, byte for byte (a VS_VERSIONINFO: Learn "VS_VERSIONINFO structure" and
 * "Version Information"); no value is made here. Returns 1 with *va = the VA of the data inside the mapped image and *size = its size, or 0 when the image has no RT_VERSION resource (the leaf is
 * the first of the first name: the PE of FLEngine has exactly one (name 1, language 1033), measured with pefile; several names or languages stop, as find_leaf does for several languages). */
int fle_res_version_block(const char *api, uint64_t *va, uint32_t *size)
{
    res_img im;
    res_key type;
    uint32_t e, f, off, n;

    img_open(api, &im, FLE_EXE_HANDLE);
    if (im.dir_rva == 0) return 0;
    type.is_id = 1;
    type.id = 16u;   /* RT_VERSION: Learn "Resource Types" (the number 16 is winuser.h = UNOFFICIAL; the page names RT_VERSION only) */
    type.nlen = 0;
    if (!table_find(api, &im, 0, &type, &f)) return 0;
    if (!(f >> 31)) fle_stop(api, "malformed resource directory (data entry at the type level)");
    off = f & 0x7fffffffu;
    n = (uint32_t)d16(api, &im, off + 12) + (uint32_t)d16(api, &im, off + 14);
    if (n == 0) return 0;
    if (n > 1) fle_stop(api, "several RT_VERSION names: the choice is not defined");
    f = d32(api, &im, off + 20);
    if (!(f >> 31)) fle_stop(api, "malformed resource directory (data entry at the name level)");
    off = f & 0x7fffffffu;
    n = (uint32_t)d16(api, &im, off + 12) + (uint32_t)d16(api, &im, off + 14);
    if (n == 0) return 0;
    if (n > 1) fle_stop(api, "several language leaves for the RT_VERSION: the language choice is not defined");
    e = d32(api, &im, off + 20);
    if (e >> 31) fle_stop(api, "malformed resource directory (subdirectory at the language level)");
    leaf_data(api, &im, e, va, size);
    return 1;
}
