/* The clipboard (decision 0171, #1389). See fle_clip.h. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_glob.h"
#include "fle_clip.h"

/* The page's side (harness/web/clip/fle_clip_lib.js). fle_clip_host_write: the text of a SetClipboardData(CF_UNICODETEXT), `bytes` bytes of UTF-16 without the NUL,
 * handed to the page, which writes it to the operating system's clipboard. fle_clip_host_read: an ASYNC import (JSPI, as fle_filedlg_host_pick): the engine waits until
 * the page answers with the operating system's text as a malloc'd NUL-terminated UTF-16LE string, or NULL (refused, unavailable, no text, no answer in time). The browser
 * build links the library that defines both; a native build defines them in its test link. fle_win_api_trace_on: the window-API trace (fle_win.h).
 * fle_clip_host_probe (#1406): an ASYNC import too. The question of a format (IsClipboardFormatAvailable, CountClipboardFormats, EnumClipboardFormats) asks the page whether
 * the operating system's clipboard holds text: 1 or 0, and no content is read for it. */
extern void fle_clip_host_write(const void *units, uint32_t bytes);
extern void *fle_clip_host_read(void);
extern int fle_clip_host_probe(void);
extern int fle_win_api_trace_on(void) __attribute__((weak));

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
/* #1389: the operating system's text, read once in an open session (clip_os_done); clip_os_h is its handle (0: no text, or the read was refused) */
static int clip_os_done;
static uint64_t clip_os_h;

static int clip_trace(void) { return fle_win_api_trace_on && fle_win_api_trace_on(); }

/* The operating system's text as a movable handle with its NUL (0: none, or refused). The page answers through the JSPI import; the buffer is the page's malloc. */
static uint64_t os_text_read(void)
{
    uint16_t *s;
    size_t n = 0;
    uint64_t h;

    s = fle_clip_host_read();
    if (!s) return 0;
    while (s[n]) n++;
    h = n ? fle_glob_movable_from(s, 2 * (n + 1)) : 0;
    free(s);
    return h;
}

/* In an open session the operating system's text is read at its first use and kept until the next OpenClipboard (Learn OpenClipboard: the clipboard is
 * locked for the session; the handle stays valid until the clipboard is emptied, so the guest's handles are not released by CloseClipboard). */
static uint64_t os_text_session(void)
{
    if (!clip_os_done) {
        clip_os_h = os_text_read();
        clip_os_done = 1;
    }
    return clip_os_h;
}

/* The session's handle is released when the next session starts, unless the guest put it on the clipboard itself (SetClipboardData with that handle). */
static void os_text_release(void)
{
    unsigned i;

    if (clip_os_h == 0) return;
    for (i = 0; i < clip_n; i++) if (clip_items[i].handle == clip_os_h) { clip_os_h = 0; return; }
    fle_glob_movable_drop(clip_os_h);
    clip_os_h = 0;
}

/* A question about the formats (#1406): whether the operating system's clipboard holds text, as the page answers fle_clip_host_probe (no content is read). */
static int os_text_probe(void) { return fle_clip_host_probe() != 0; }

/* the handle of the guest's own CF_UNICODETEXT on the virtual clipboard (0: none) */
static uint64_t virtual_text_handle(void)
{
    unsigned i;

    for (i = 0; i < clip_n; i++) if (clip_items[i].format == CF_UNICODETEXT) return clip_items[i].handle;
    return 0;
}

/* CF_TEXT and CF_OEMTEXT made from a UTF-16 text (Learn, Synthesized Clipboard Formats: the system converts the text): one byte a character, a character that is not
 * ASCII is '?' (S-ia1-clip-5: the page names no code page). The units up to the NUL; the result has its NUL. 0 when the host refuses the memory. */
static uint64_t text_bytes_of(uint64_t h)
{
    const uint8_t *p;
    uint64_t n, k;
    const uint16_t *s16;
    char *buf;
    uint64_t r;

    if (!fle_glob_movable_bytes(h, &p, &n)) return 0;
    s16 = (const uint16_t *)p;
    buf = malloc(n / 2 + 1);
    if (!buf) return 0;
    for (k = 0; 2 * (k + 1) <= n && s16[k]; k++) buf[k] = s16[k] < 0x80 ? (char)s16[k] : '?';
    buf[k] = 0;
    r = fle_glob_movable_from(buf, k + 1);
    free(buf);
    return r;
}

/* The text of a SetClipboardData(CF_UNICODETEXT): the units up to the NUL (within the block's size) go to the page. */
static void clip_send_text(uint64_t h)
{
    const uint8_t *p;
    uint64_t n, k = 0;
    const uint16_t *s;

    if (!fle_glob_movable_bytes(h, &p, &n)) return;
    s = (const uint16_t *)p;
    while (2 * (k + 1) <= n && s[k]) k++;
    fle_clip_host_write(p, (uint32_t)(2 * k));
}

/* the first 120 units of a clipboard text for the trace: printable ASCII as it is, anything else as '?' */
static void clip_trace_text(uint64_t h)
{
    const uint8_t *p;
    uint64_t n, k, m = 0;
    const uint16_t *s;
    char buf[128];

    if (!fle_glob_movable_bytes(h, &p, &n)) { fprintf(stderr, " text=(no block)"); return; }
    s = (const uint16_t *)p;
    for (k = 0; 2 * (k + 1) <= n && s[k] && m < 120; k++, m++) buf[m] = s[k] >= 0x20 && s[k] < 0x7f ? (char)s[k] : '?';
    buf[m] = 0;
    fprintf(stderr, " bytes=%llu text=\"%s\"", (unsigned long long)n, buf);
}

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
    os_text_release();   /* the previous session's text: the guest has finished with it */
    clip_os_done = 0;
    clip_open = 1;
    clip_open_hwnd = hwnd;
    R[0] = 1;
    if (clip_trace()) fprintf(stderr, "window-api: OpenClipboard hwnd=%#llx\n", (unsigned long long)hwnd);
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
    clip_os_done = 0;
    R[0] = 1;
    if (clip_trace()) fprintf(stderr, "window-api: CloseClipboard\n");
}

/* Learn EmptyClipboard: "empties the clipboard and frees handles to data in the clipboard. The function then assigns ownership of the clipboard to the window
 * that currently has the clipboard open." The clipboard must be open. The operating system's text is gone with the rest (the session's read is not repeated). */
void fle_imp_EmptyClipboard(void)
{
    if (!clip_open) {
        fle_set_last_error(FLE_CLIP_ERR_NOT_OPEN);
        R[0] = 0;
        return;
    }
    clip_n = 0;
    os_text_release();
    clip_os_done = 1;
    R[0] = 1;
    if (clip_trace()) fprintf(stderr, "window-api: EmptyClipboard\n");
}

/* The formats as a question sees them: all_formats, and the operating system's text formats (CF_UNICODETEXT, CF_TEXT, CF_OEMTEXT) when it holds text (#1406). */
static unsigned formats_now(uint32_t *out, unsigned cap)
{
    static const uint32_t text[3] = { CF_UNICODETEXT, CF_TEXT, CF_OEMTEXT };
    unsigned n = all_formats(out, cap - 3), k, j;

    if (!os_text_probe()) return n;
    for (k = 0; k < 3; k++) {
        int dup = 0;
        for (j = 0; j < n; j++) if (out[j] == text[k]) dup = 1;
        if (!dup) out[n++] = text[k];
    }
    return n;
}

/* Learn IsClipboardFormatAvailable: "Determines whether the clipboard contains data in the specified format." Zero when the format is not there. No
 * clipboard open is needed. The page gives no last-error for "not available": it is left unchanged (S-ia1-clip-4). CF_UNICODETEXT is available when the
 * operating system's clipboard holds text (#1389); the question does not read the text (#1406: fle_clip_host_probe). */
void fle_imp_IsClipboardFormatAvailable(void)
{
    uint32_t f = (uint32_t)R[1], all[CLIP_MAX * 3];
    unsigned n = all_formats(all, CLIP_MAX * 3), i;
    int os = (f == CF_UNICODETEXT || f == CF_TEXT || f == CF_OEMTEXT) && os_text_probe();
    R[0] = os ? 1 : 0;
    for (i = 0; i < n; i++) if (all[i] == f) R[0] = 1;
    if (clip_trace()) fprintf(stderr, "window-api: IsClipboardFormatAvailable fmt=%u -> %u (os text %d, open %d)\n", f, (unsigned)R[0], os, clip_open);
}

/* Learn CountClipboardFormats: "the number of different data formats currently on the clipboard" (0 for an empty one). */
void fle_imp_CountClipboardFormats(void)
{
    uint32_t all[CLIP_MAX * 3 + 3];
    R[0] = formats_now(all, CLIP_MAX * 3 + 3);
}

/* Learn EnumClipboardFormats: "format 0 to start the enumeration; the next format is returned; 0 at the end (GetLastError ERROR_SUCCESS)". The clipboard must
 * be open (the page: "you must open the clipboard before enumerating its formats"; ERROR_CLIPBOARD_NOT_OPEN when not, S-ia1-clip-3). */
void fle_imp_EnumClipboardFormats(void)
{
    uint32_t f = (uint32_t)R[1], all[CLIP_MAX * 3 + 3];
    unsigned n, i;
    if (!clip_open) {
        fle_set_last_error(FLE_CLIP_ERR_NOT_OPEN);
        R[0] = 0;
        return;
    }
    n = formats_now(all, CLIP_MAX * 3 + 3);
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
        if (clip_trace()) fprintf(stderr, "window-api: GetClipboardData fmt=%u -> 0 (not open)\n", f);
        return;
    }
    if (f == CF_UNICODETEXT || f == CF_TEXT || f == CF_OEMTEXT) {
        uint64_t h = os_text_session();
        if (h) {
            R[0] = f == CF_UNICODETEXT ? h : text_bytes_of(h);
            if (clip_trace()) {
                fprintf(stderr, "window-api: GetClipboardData fmt=%u -> %#llx (os text)", f, (unsigned long long)R[0]);
                clip_trace_text(h);
                fprintf(stderr, "\n");
            }
            return;
        }
    }
    for (i = 0; i < clip_n; i++) {
        if (clip_items[i].format == f) {
            if (clip_items[i].handle == 0) fle_stop("GetClipboardData", "delayed rendering (SetClipboardData with a NULL handle) needs WM_RENDERFORMAT to the owner: not implemented");
            R[0] = clip_items[i].handle;
            if (clip_trace()) fprintf(stderr, "window-api: GetClipboardData fmt=%u -> %#llx\n", f, (unsigned long long)R[0]);
            return;
        }
    }
    if ((f == CF_TEXT || f == CF_OEMTEXT) && virtual_text_handle() != 0) {   /* the guest's own Unicode text, made into the byte format */
        R[0] = text_bytes_of(virtual_text_handle());
        if (clip_trace()) fprintf(stderr, "window-api: GetClipboardData fmt=%u -> %#llx (from the guest's text)\n", f, (unsigned long long)R[0]);
        return;
    }
    {
        uint32_t all[CLIP_MAX * 3];
        unsigned n = all_formats(all, CLIP_MAX * 3), k;
        for (k = 0; k < n; k++) if (all[k] == f) fle_stop("GetClipboardData", "a synthesized format (the system converts the data): not implemented");
    }
    R[0] = 0;
    if (clip_trace()) fprintf(stderr, "window-api: GetClipboardData fmt=%u -> 0\n", f);
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
        if (clip_trace()) fprintf(stderr, "window-api: SetClipboardData fmt=%u handle=%#llx -> 0 (not open)\n", f, (unsigned long long)h);
        return;
    }
    if (clip_trace()) {
        fprintf(stderr, "window-api: SetClipboardData fmt=%u handle=%#llx", f, (unsigned long long)h);
        if (f == CF_UNICODETEXT && h) clip_trace_text(h);
        fprintf(stderr, "\n");
    }
    if (f == CF_UNICODETEXT && h) clip_send_text(h);   /* #1389: the page writes the text to the operating system's clipboard */
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
