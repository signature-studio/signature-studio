/* Decision 0141 (PROVISIONAL): the read-only view of the engine for the browser front end (see fle_appview.h). Reads the window table and the cursor state through the accessors that already exist
 * (fle_win_input_*, fle_gdi_cursor_current, fle_cursor_image_idc_of) and two small ones added to fle_win.c; changes nothing. */
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#else
#define EMSCRIPTEN_KEEPALIVE
#endif

#include "fle_runtime.h"
#include "fle_cursor.h"
#include "fle_gdi.h"
#include "fle_input.h"
#include "fle_appview.h"
#include "fle_res.h"
#include "fle_screen.h"

/* the layout app/src/engine/appview.ts reads (app/test/unit/appview.test.ts reads these lines and compares them with its constants) */
_Static_assert(sizeof(fle_av_window) == 144, "AV_WINDOW_BYTES");
_Static_assert(offsetof(fle_av_window, x) == 16 && offsetof(fle_av_window, style) == 32 && offsetof(fle_av_window, cls) == 48 && offsetof(fle_av_window, title) == 80, "window fields");
_Static_assert(offsetof(fle_av_state, seq) == 16 && offsetof(fle_av_state, cursor) == 20 && offsetof(fle_av_state, pointer_x) == 32 && offsetof(fle_av_state, n_windows) == 44 && offsetof(fle_av_state, screen_w) == 48 && offsetof(fle_av_state, screen_h) == 52, "header fields");
_Static_assert(offsetof(fle_av_state, windows) == 68, "AV_HEADER_BYTES");

extern uint64_t fle_gdi_cursor_current(void) __attribute__((weak));
extern int fle_cursor_is_shared_handle(uint64_t h) __attribute__((weak));
extern int fle_gdi_kind(uint64_t h) __attribute__((weak));
extern int fle_cursor_picture(uint64_t h, fle_cursor_pic *out, const char **why) __attribute__((weak));
extern int fle_gdi_cursor_picture(uint64_t h, fle_cursor_pic *out, const char **why) __attribute__((weak));

_Static_assert(sizeof(fle_cursor_pic_state) == 256, "CURSOR_PICTURE_BYTES");
_Static_assert(offsetof(fle_cursor_pic_state, ordinal) == 36 && offsetof(fle_cursor_pic_state, rgba_addr) == 56 && offsetof(fle_cursor_pic_state, raw_addr) == 60 && offsetof(fle_cursor_pic_state, name) == 64 && offsetof(fle_cursor_pic_state, why) == 128, "cursor picture fields");

static void copy_ascii(char *dst, unsigned cap, const char *src)
{
    unsigned k;

    for (k = 0; k + 1u < cap && src[k]; k++) dst[k] = src[k];
    dst[k] = 0;
}

/* Decision 0260 (PROVISIONAL), #1392: the picture of the cursor with the low 32 bits `handle` (fle_appview.h): a cursor made by CreateIconIndirect from its bitmaps (fle_gdi.c), else a
 * cursor loaded from a module (the resource, fle_res.c). Read by the worker for each handle that the osstate names. */
EMSCRIPTEN_KEEPALIVE const fle_cursor_pic_state *fle_cursor_picture_poll(uint32_t handle)
{
    static fle_cursor_pic_state st;
    fle_cursor_pic pic;
    const char *why = "this build has no cursor picture (fle_cursor_picture)";
    int made = fle_gdi_kind && fle_gdi_kind((uint64_t)handle) == FLE_GDI_KIND_ICONCUR;
    int ok;

    memset(&st, 0, sizeof st);
    st.magic = FLE_CP_MAGIC;
    st.handle = handle;
    if (made) ok = fle_gdi_cursor_picture != NULL && fle_gdi_cursor_picture((uint64_t)handle, &pic, &why);
    else ok = fle_cursor_picture != NULL && fle_cursor_picture((uint64_t)handle, &pic, &why);
    if (ok) {
        st.ok = 1;
        st.width = pic.width;
        st.height = pic.height;
        st.hot_x = pic.hot_x;
        st.hot_y = pic.hot_y;
        st.cursor_id = pic.cursor_id;
        st.named = pic.named;
        st.ordinal = pic.ordinal;
        st.invert = pic.invert;
        st.alpha = pic.alpha;
        st.transparent = pic.transparent;
        st.opaque = pic.opaque;
        st.rgba_addr = (uint32_t)(uintptr_t)pic.rgba;
        st.raw_addr = (uint32_t)(uintptr_t)pic.raw;
        memcpy(st.name, pic.name, sizeof st.name);
    } else {
        copy_ascii(st.why, sizeof st.why, why);
    }
    return &st;
}

EMSCRIPTEN_KEEPALIVE const fle_av_state *fle_av_poll(void)
{
    static fle_av_state st, prev;
    static uint32_t seq;
    uint64_t hs[FLE_AV_MAX_WINDOWS], cur, active, focus, capture;
    unsigned n, k, ft = 0, ct = 0;

    memset(&st, 0, sizeof st);
    st.magic = FLE_AV_MAGIC;
    st.version = FLE_AV_VERSION;
    st.entry_size = (uint32_t)sizeof(fle_av_window);
    st.max_windows = FLE_AV_MAX_WINDOWS;
    cur = fle_gdi_cursor_current ? fle_gdi_cursor_current() : 0;
    st.cursor = (uint32_t)cur;
    if (cur) {
        st.cursor_idc = fle_cursor_image_idc_of(cur);
        st.cursor_shared = st.cursor_idc == 0 && fle_cursor_is_shared_handle && fle_cursor_is_shared_handle(cur) ? 1u : 0u;
        st.cursor_made = st.cursor_idc == 0 && st.cursor_shared == 0 && fle_gdi_kind && fle_gdi_kind(cur) == FLE_GDI_KIND_ICONCUR ? 1u : 0u;
    }
    fle_win_pointer_get(&st.pointer_x, &st.pointer_y);
    st.pending_events = fle_input_host_pending();
    st.screen_w = fle_screen_w();
    st.screen_h = fle_screen_h();
    active = fle_win_input_active();
    focus = fle_win_input_key_target(&ft);
    capture = fle_win_input_capture_any(&ct);
    n = fle_win_input_toplevels(hs, FLE_AV_MAX_WINDOWS);
    for (k = 0; k < n; k++) {
        fle_win_iview v;
        fle_av_window *w = &st.windows[st.n_windows];
        uint32_t owner = 0;

        if (!fle_win_input_view(hs[k], &v)) continue;
        w->lid = v.lid;
        w->hwnd = (uint32_t)hs[k];
        w->flags = (v.visible ? FLE_AV_W_VISIBLE : 0u) | (v.enabled ? FLE_AV_W_ENABLED : 0u) | ((v.exstyle & 0x8u) ? FLE_AV_W_TOPMOST : 0u);
        if (active == hs[k]) w->flags |= FLE_AV_W_ACTIVE;
        if (focus) {
            fle_win_iview fv;

            if (fle_win_input_view(focus, &fv) && fv.root == hs[k]) w->flags |= FLE_AV_W_FOCUS;
        }
        if (capture) {
            fle_win_iview cv;

            if (fle_win_input_view(capture, &cv) && cv.root == hs[k]) w->flags |= FLE_AV_W_CAPTURE;
        }
        w->x = v.ox;
        w->y = v.oy;
        w->width = v.cx;
        w->height = v.cy;
        w->style = v.style;
        w->exstyle = v.exstyle;
        w->thread = v.owner;
        copy_ascii(w->cls, FLE_AV_CLASS_BYTES, v.cls);
        fle_win_input_text(hs[k], w->title, FLE_AV_TITLE_BYTES, &owner);
        w->owner_lid = owner;
        st.n_windows++;
    }
    if (memcmp(&st, &prev, sizeof st) != 0) {
        prev = st;
        seq++;
    }
    st.seq = seq;
    return &st;
}
