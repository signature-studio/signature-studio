/* The virtual clipboard (decision 0171). See fle_clip.h. */
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_clip.h"

/* winerror.h: ERROR_CLIPBOARD_NOT_OPEN (Learn, System Error Codes 1400-1699: "Thread does not have a clipboard open.") */
#define FLE_CLIP_ERR_NOT_OPEN 1418u
#define FLE_CLIP_ERR_OPEN_ELSEWHERE FLE_W32_ERR_ACCESS_DENIED

/* The predefined formats (Learn, Standard Clipboard Formats). */
#define CF_TEXT 1u
#define CF_BITMAP 2u
#define CF_METAFILEPICT 3u
#define CF_OEMTEXT 7u
#define CF_DIB 8u
#define CF_PALETTE 9u
#define CF_UNICODETEXT 13u
#define CF_ENHMETAFILE 14u
#define CF_DIBV5 17u

#define CLIP_MAX 64u

typedef struct { uint32_t format; uint64_t handle; } clip_item;

static clip_item clip_items[CLIP_MAX];
static unsigned clip_n;
static int clip_open;
static uint64_t clip_open_hwnd;
static uint64_t clip_owner;

/* Learn, Synthesized Clipboard Formats: the formats that the system makes available when the given format is on the clipboard.
 * CF_PALETTE comes with CF_DIB and CF_DIBV5 only when CF_BITMAP's palette is there; it is not synthesized here (S-ia1-clip-2). */
static unsigned synth_of(uint32_t have, uint32_t out[3])
{
    switch (have) {
    case CF_BITMAP: out[0] = CF_DIB; out[1] = CF_DIBV5; return 2;
    case CF_DIB: out[0] = CF_BITMAP; out[1] = CF_DIBV5; return 2;
    case CF_DIBV5: out[0] = CF_BITMAP; out[1] = CF_DIB; return 2;
    case CF_ENHMETAFILE: out[0] = CF_METAFILEPICT; return 1;
    case CF_METAFILEPICT: out[0] = CF_ENHMETAFILE; return 1;
    case CF_OEMTEXT: out[0] = CF_TEXT; out[1] = CF_UNICODETEXT; return 2;
    case CF_TEXT: out[0] = CF_OEMTEXT; out[1] = CF_UNICODETEXT; return 2;
    case CF_UNICODETEXT: out[0] = CF_OEMTEXT; out[1] = CF_TEXT; return 2;
    default: return 0;
    }
}

/* The formats as an enumeration sees them: the real ones in the order they were set, then the synthesized ones not already there. */
static unsigned all_formats(uint32_t *out, unsigned cap)
{
    unsigned n = 0, i, k;
    for (i = 0; i < clip_n && n < cap; i++) out[n++] = clip_items[i].format;
    for (i = 0; i < clip_n; i++) {
        uint32_t s[3];
        unsigned m = synth_of(clip_items[i].format, s);
        for (k = 0; k < m; k++) {
            unsigned j;
            int dup = 0;
            for (j = 0; j < n; j++) if (out[j] == s[k]) dup = 1;
            if (!dup && n < cap) out[n++] = s[k];
        }
    }
    return n;
}

/* Learn OpenClipboard: "OpenClipboard fails if another window has the clipboard open." The only window that can have it open is the guest's, so a
 * second open by a different window fails (S-ia1-clip-1: the same window opening twice succeeds; the page does not say). */
void fle_imp_OpenClipboard(void)
{
    uint64_t hwnd = R[1];
    if (clip_open && clip_open_hwnd != hwnd) {
        fle_set_last_error(FLE_CLIP_ERR_OPEN_ELSEWHERE);
        R[0] = 0;
        return;
    }
    clip_open = 1;
    clip_open_hwnd = hwnd;
    R[0] = 1;
}

/* Learn CloseClipboard: fails when the clipboard is not open ("the calling thread does not have the clipboard open": ERROR_CLIPBOARD_NOT_OPEN, S-ia1-clip-3). */
void fle_imp_CloseClipboard(void)
{
    if (!clip_open) {
        fle_set_last_error(FLE_CLIP_ERR_NOT_OPEN);
        R[0] = 0;
        return;
    }
    clip_open = 0;
    clip_open_hwnd = 0;
    R[0] = 1;
}

/* Learn EmptyClipboard: "empties the clipboard and frees handles to data in the clipboard. The function then assigns ownership of the clipboard to the window
 * that currently has the clipboard open." The clipboard must be open. */
void fle_imp_EmptyClipboard(void)
{
    if (!clip_open) {
        fle_set_last_error(FLE_CLIP_ERR_NOT_OPEN);
        R[0] = 0;
        return;
    }
    clip_n = 0;
    clip_owner = clip_open_hwnd;
    R[0] = 1;
}

/* Learn IsClipboardFormatAvailable: "Determines whether the clipboard contains data in the specified format." Zero when the format is not there. No
 * clipboard open is needed. The page gives no last-error for "not available": it is left unchanged (S-ia1-clip-4). */
void fle_imp_IsClipboardFormatAvailable(void)
{
    uint32_t f = (uint32_t)R[1], all[CLIP_MAX * 3];
    unsigned n = all_formats(all, CLIP_MAX * 3), i;
    R[0] = 0;
    for (i = 0; i < n; i++) if (all[i] == f) R[0] = 1;
}

/* Learn CountClipboardFormats: "the number of different data formats currently on the clipboard" (0 for an empty one). */
void fle_imp_CountClipboardFormats(void)
{
    uint32_t all[CLIP_MAX * 3];
    R[0] = all_formats(all, CLIP_MAX * 3);
}

/* Learn EnumClipboardFormats: "format 0 to start the enumeration; the next format is returned; 0 at the end (GetLastError ERROR_SUCCESS)". The clipboard must
 * be open (the page: "you must open the clipboard before enumerating its formats"; ERROR_CLIPBOARD_NOT_OPEN when not, S-ia1-clip-3). */
void fle_imp_EnumClipboardFormats(void)
{
    uint32_t f = (uint32_t)R[1], all[CLIP_MAX * 3];
    unsigned n, i;
    if (!clip_open) {
        fle_set_last_error(FLE_CLIP_ERR_NOT_OPEN);
        R[0] = 0;
        return;
    }
    n = all_formats(all, CLIP_MAX * 3);
    if (f == 0) {
        fle_set_last_error(FLE_W32_ERR_SUCCESS);
        R[0] = n ? all[0] : 0;
        return;
    }
    for (i = 0; i < n; i++) {
        if (all[i] == f) {
            fle_set_last_error(FLE_W32_ERR_SUCCESS);
            R[0] = i + 1 < n ? all[i + 1] : 0;
            return;
        }
    }
    fle_set_last_error(FLE_W32_ERR_INVALID_PARAMETER);
    R[0] = 0;
}

/* Learn GetClipboardData: "Retrieves data from the clipboard in a specified format. The clipboard must have been opened." NULL when it is not open or the
 * format is not there. A synthesized format would need the converted data: stops by name (S-ia1-clip-2). */
void fle_imp_GetClipboardData(void)
{
    uint32_t f = (uint32_t)R[1];
    unsigned i;
    if (!clip_open) {
        fle_set_last_error(FLE_CLIP_ERR_NOT_OPEN);
        R[0] = 0;
        return;
    }
    for (i = 0; i < clip_n; i++) {
        if (clip_items[i].format == f) {
            if (clip_items[i].handle == 0) fle_stop("GetClipboardData", "delayed rendering (SetClipboardData with a NULL handle) needs WM_RENDERFORMAT to the owner: not implemented");
            R[0] = clip_items[i].handle;
            return;
        }
    }
    {
        uint32_t all[CLIP_MAX * 3];
        unsigned n = all_formats(all, CLIP_MAX * 3), k;
        for (k = 0; k < n; k++) if (all[k] == f) fle_stop("GetClipboardData", "a synthesized format (the system converts the data): not implemented");
    }
    R[0] = 0;
}

/* Learn SetClipboardData: "Places data on the clipboard in a specified clipboard format. The window must be the current clipboard owner, and the application
 * must have called the OpenClipboard function." Returns the handle of the data. A format set twice replaces the first. */
void fle_imp_SetClipboardData(void)
{
    uint32_t f = (uint32_t)R[1];
    uint64_t h = R[2];
    unsigned i;
    if (!clip_open) {
        fle_set_last_error(FLE_CLIP_ERR_NOT_OPEN);
        R[0] = 0;
        return;
    }
    for (i = 0; i < clip_n; i++) {
        if (clip_items[i].format == f) {
            clip_items[i].handle = h;
            R[0] = h;
            return;
        }
    }
    if (clip_n >= CLIP_MAX) fle_stop("SetClipboardData", "more than 64 formats on the clipboard (a limit of this runtime)");
    clip_items[clip_n].format = f;
    clip_items[clip_n].handle = h;
    clip_n++;
    R[0] = h;
}

unsigned fle_clip_count(void) { return clip_n; }
uint32_t fle_clip_format(unsigned i) { return i < clip_n ? clip_items[i].format : 0; }
int fle_clip_is_open(void) { return clip_open; }

/* ---- The OLE clipboard: ole32!OleSetClipboard, OleIsCurrentClipboard, OleFlushClipboard (the data object pointer only) ----
 * Learn OleSetClipboard: "Places a pointer to a specific data object onto the clipboard"; NULL empties the clipboard (Learn OleFlushClipboard: "To empty the clipboard, call OleSetClipboard specifying a NULL value
 * for its parameter"). The object is kept with one reference of its own (IUnknown::AddRef = vtable slot 1; the previous object gets Release = slot 2). S-ia1-ole-1: the formats that the object offers are NOT
 * placed on the virtual Win32 clipboard (that needs IDataObject::EnumFormatEtc), the Win32 clipboard is emptied. Learn OleIsCurrentClipboard: S_OK when the pointer is the one OleSetClipboard placed and it
 * is still there, S_FALSE otherwise. Learn OleFlushClipboard: renders the formats onto the clipboard and releases the pointer; S_OK. S-ia1-ole-2: the rendering is not done (the pointer is released only);
 * with no data object on the clipboard it is S_OK (the page lists no other success). */
#define FLE_S_OK 0u
#define FLE_S_FALSE 1u

static uint64_t ole_obj;

static void com_call0(uint64_t obj, unsigned slot)
{
    uint64_t vtbl = fle_ld(obj, 8);
    uint64_t fn = fle_ld(vtbl + 8u * slot, 8);

    fle_call_guest(fn, obj, 0, 0, 0);
}

void fle_imp_OleSetClipboard(void)
{
    uint64_t obj = R[1], old = ole_obj;

    if (obj != 0) com_call0(obj, 1);
    ole_obj = obj;
    clip_n = 0;
    clip_owner = 0;
    if (old != 0) com_call0(old, 2);
    R[0] = FLE_S_OK;
}

void fle_imp_OleIsCurrentClipboard(void)
{
    R[0] = (R[1] != 0 && R[1] == ole_obj) ? FLE_S_OK : FLE_S_FALSE;
}

void fle_imp_OleFlushClipboard(void)
{
    uint64_t old = ole_obj;

    ole_obj = 0;
    if (old != 0) com_call0(old, 2);
    R[0] = FLE_S_OK;
}

uint64_t fle_clip_ole_object(void) { return ole_obj; }
