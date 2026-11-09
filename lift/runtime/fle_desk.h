/* Decision 0223 (desk1, PROVISIONAL): the desktop compositor, the runtime's part of what Windows calls the Desktop Window Manager. It reads the window table (fle_win.c: the live top-level windows in z-order, their
 * positions and visibility), the drawing surface of every top-level window (fle_raster.c) and the layered-window state (fle_layered.c), and keeps ONE picture of the virtual screen in the engine's linear memory:
 * the size of the screen (fle_screen.h: 1920 x 1080 unless the run says otherwise, and a display change makes a new picture), 4 bytes per pixel in the byte order R, G, B, A (A = 255), top-down, no padding. The browser front end shows that one picture (Worker -> OffscreenCanvas, no copy through JS) and knows nothing
 * about windows. The compositor DECIDES NOTHING about windows: the order, the position, the visibility and the layering are what the window table and the layered-window state say; it only draws them.
 *
 * What is drawn (Microsoft Learn; the sentences are quoted in docs/decisions/0223-desktop-compositor.md):
 *   - the windows from the bottom of the z-order to the top ("Window Features / Z Order": a window higher in the z-order is drawn over those below it);
 *   - an ordinary window opaquely (its surface is BGRX: the fourth byte is not an alpha);
 *   - a layered window (WS_EX_LAYERED) not at all until UpdateLayeredWindow or SetLayeredWindowAttributes has been called ("Window Features / Layered Windows": it will not become visible until then);
 *   - SetLayeredWindowAttributes: LWA_ALPHA blends the window with bAlpha (Dst = Src * bAlpha/255 + Dst * (1 - bAlpha/255)), LWA_COLORKEY makes the pixels of that colour transparent;
 *   - UpdateLayeredWindow: the surface is a 32-bit bitmap with a premultiplied alpha (AC_SRC_ALPHA): Src = Src * SourceConstantAlpha / 255 in all four channels, then Dst = Src + (1 - Src.A) * Dst (BLENDFUNCTION).
 *   - decision 0244 (alpha1): a window whose frame is extended into its client area (DwmExtendFrameIntoClientArea; a negative margin is the sheet of glass = the whole window) is drawn, outside the inner rectangle that the margins leave,
 *     as a premultiplied ARGB over what is below with the same formula (the fourth byte of its surface is the alpha: a BitBlt from a 32-bit DIB carries it, fle_gdi.c); inside the inner rectangle it is opaque as any window;
 *   - decision 0244: a window that has a window region (SetWindowRgn) is drawn inside the region only ("The system does not display any portion of a window that lies outside of the window region").
 * The line of fle_desk_describe for a window ends with its class and its title; the layer words before them are: layered premultiplied alpha key glass(x0,y0,x1,y1) region(l,t,r,b).
 * Where no window is, the picture has the colour FLE_DESK_BACKGROUND (a choice: the colour of the page that showed the windows before this compositor, #05070a).
 *
 * Only the damaged part is drawn again: a rectangle of a window's surface that was written (fle_raster_wrote -> fle_desk_wrote), and the old and the new rectangle of a window whose position, size, visibility or
 * layering changed, and, when the z-order changed, the rectangles of the windows from the first place that differs upwards. fle_desk_compose() brings the picture up to date and returns the rectangles that changed
 * since the previous call (at most FLE_DESK_MAX_RECTS; overlapping rectangles are merged, and when there are more they are merged by the smallest growth). What was drawn again is compared with the picture
 * and only the pixels that DIFFER are reported (the bounding rectangle of each damaged rectangle's changes; none when nothing changed): an engine that writes the same pixels again, as the idle redraw of the
 * main form does once per frame, costs the browser nothing. The first call, a new screen size and fle_desk_resync() report the whole picture. Single guest thread (decision 0017).
 *
 * The whole file is always compiled (decision 0243); fle_raster.c reaches fle_desk_wrote through a weak reference, so a native link list without this file leaves it null. */
#ifndef FLE_DESK_H
#define FLE_DESK_H

#include <stdint.h>

#define FLE_DESK_MAGIC 0x4b534544u      /* "DESK" as bytes 'D' 'E' 'S' 'K' */
#define FLE_DESK_VERSION 1u
#define FLE_DESK_MAX_RECTS 32u
#define FLE_DESK_MAX_WINDOWS 64u
#define FLE_DESK_BACKGROUND_R 5u
#define FLE_DESK_BACKGROUND_G 7u
#define FLE_DESK_BACKGROUND_B 10u

/* what the browser reads (little-endian 32-bit words from the linear memory; app/src/engine/desk.ts is the reader and app/test/unit/desk.test.ts pins the layout) */
typedef struct {
    int32_t x0, y0, x1, y1;             /* [x0, x1) x [y0, y1) on the picture */
} fle_desk_rect;

typedef struct {
    uint32_t magic, version;
    uint32_t width, height, stride;     /* pixels, pixels, bytes between two rows */
    uint32_t pixels;                    /* the address of the picture in the linear memory (allocated once, never moves; the memory's ArrayBuffer is replaced when the memory grows, the address stays) */
    uint32_t gen;                       /* +1 on every call of fle_desk_compose that returned a rectangle */
    uint32_t n_rects;                   /* the changed rectangles since the previous call */
    uint32_t n_windows;                 /* the windows drawn now */
    uint32_t flags;                     /* bit 0: the whole picture is new (the first call, or a lost picture) */
    uint32_t reserved[6];
    fle_desk_rect rects[FLE_DESK_MAX_RECTS];
} fle_desk_state;

/* Brings the picture up to date and returns the state (always the same address). The rectangles are consumed: the next call reports only what changed after this one. */
const fle_desk_state *fle_desk_compose(void);

/* fle_raster.c calls this (a weak reference: a link list without fle_desk.c leaves it null) after the rectangle r of the surface of top-level window `lid` was written. The rectangle is in surface coordinates. */
void fle_desk_wrote(uint32_t lid, int64_t x0, int64_t y0, int64_t x1, int64_t y1);

/* The picture itself, as a pointer of the host's width (fle_desk_state.pixels is the 32-bit address of the wasm build; a native test needs the full pointer). */
const uint8_t *fle_desk_picture(void);

/* The next call of fle_desk_compose reports the whole picture (the browser lost its copy: a lost canvas context). */
void fle_desk_resync(void);

/* A text line per window drawn at the last compose (lid, rectangle, layer, and since desk2 the window class name and title as printable ASCII), for the page's diagnostics (?debug=1) and for the tests; NUL-terminated, valid until the next compose. */
const char *fle_desk_describe(void);


/* ---- Decision 0240 (gpu1, PROVISIONAL): the SCENE (how the browser shows the desktop is decided by what it can do: app/src/engine/worker.ts makeDeskPresenter). The same window table, surfaces and layered state, but the compositor does not touch a pixel: it describes what is to be shown and the browser (a WebGL2 context in the
 * engine Worker, app/src/engine/desk-gl.ts) keeps one texture per top-level window and draws them, bottom to top. fle_desk_scene_mode(1) switches the runtime to it (fle_desk_compose is then not called; a test aid that needs the CPU picture
 * calls fle_desk_resync() + fle_desk_compose() and fle_desk_resync() again). What is described, all from the window table: for each window drawn (bottom first) its lid, the origin on the picture, the size of its surface, the address and stride of the
 * surface's pixels (B, G, R, X bytes, top-down), the layering (alpha, colour key, per-pixel alpha); and the rectangles of the surfaces that were written since the previous call (fle_raster_wrote -> fle_desk_wrote), one bounding rectangle per
 * window, or the whole surface for a window that is new, has another size, or after fle_desk_resync(). Moving, restacking, showing or hiding a window changes the description and uploads no pixel. */
#define FLE_SCENE_MAGIC 0x4e454353u     /* "SCEN" */
#define FLE_SCENE_VERSION 2u
#define FLE_SCENE_MAX_WINDOWS FLE_DESK_MAX_WINDOWS
#define FLE_SCENE_MAX_UPDATES FLE_DESK_MAX_WINDOWS
#define FLE_SCENE_FLAG_WHOLE 1u         /* every window is described as new (the first call, a new screen size, fle_desk_resync()) */
#define FLE_SCENE_FLAG_CHANGED 2u       /* a window appeared, went, moved, was resized, restacked or re-layered since the previous call */
#define FLE_SCENE_LAYERED 8u            /* fle_scene_win.layer: the window is layered; with FLE_SCENE_ALPHA / _KEY / _PREMUL the same meaning as in the compositor (LWA_ALPHA or UpdateLayeredWindow's constant alpha / LWA_COLORKEY / per-pixel alpha) */
#define FLE_SCENE_ALPHA 1u
#define FLE_SCENE_GLASS 16u             /* fle_scene_win.layer, not layered (decision 0244, DwmExtendFrameIntoClientArea): outside the inner box (gx0, gy0, gx1, gy1) a pixel is a premultiplied ARGB over what is below (the fourth byte is A), inside it the window is opaque */
#define FLE_SCENE_KEY 2u
#define FLE_SCENE_PREMUL 4u

typedef struct {
    uint32_t lid, x, y, w, h;           /* x, y: the origin on the picture (int32 in two's complement); w, h: the surface */
    uint32_t bits, stride;              /* the address of the surface's pixels in the linear memory (valid until the next call) and the bytes between two rows */
    uint32_t layer, alpha, key;         /* FLE_SCENE_*, the constant alpha (0..255), the colour key as 0x00BBGGRR from the window's COLORREF */
    uint32_t fresh;                     /* 1: the texture is to be (re)allocated with this size; the window's whole surface is in the updates */
    uint32_t reserved;
    int32_t gx0, gy0, gx1, gy1;         /* FLE_SCENE_GLASS: the inner rectangle of the client area that the extended frame does not reach, in the window's own coordinates */
    int32_t cl, ct, cr, cb;             /* the box of the window region (SetWindowRgn), in the window's own coordinates, inside [0, w] x [0, h]; a pixel outside it is not drawn */
} fle_scene_win;
typedef struct { uint32_t lid; int32_t x0, y0, x1, y1; } fle_scene_upd;      /* [x0, x1) x [y0, y1) in the window's surface */
typedef struct {
    uint32_t magic, version, width, height, gen, n_windows, n_updates, flags, background;   /* background: R, G, B in the low 24 bits */
    uint32_t reserved[7];
    fle_scene_win win[FLE_SCENE_MAX_WINDOWS];
    fle_scene_upd upd[FLE_SCENE_MAX_UPDATES];
} fle_scene;

/* 1: from now on the description is made by fle_desk_scene (and fle_desk_wrote only collects the rectangles); 0: the compositor of fle_desk_compose again. */
void fle_desk_scene_mode(int on);
/* The description of the picture as the window table is now; the collected rectangles are consumed. Always the same address. */
const fle_scene *fle_desk_scene(void);

#endif
