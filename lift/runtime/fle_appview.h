/* Decision 0141 (PROVISIONAL): what the browser front end (app/) reads from the engine while it is idle, in ONE read-only call. Nothing in it moves a state of the engine; it only copies:
 * the cursor SetCursor set last (so the page can show the cursor the guest chose), the pointer, the number of input events not yet turned into messages, and the live top-level windows in z-order
 * (top first) with the facts the page needs to lay them out (position, size, visible, enabled, owner, the active / focus / capture window). The pixels are not here (they are in the surface table of
 * harness/fruity_surface.c). The structure is read by JS from the linear memory as little-endian 32-bit words; app/src/engine/appview.ts is the reader and app/test/unit/appview.test.ts pins the layout. */
#ifndef FLE_APPVIEW_H
#define FLE_APPVIEW_H

#include <stdint.h>

#define FLE_AV_MAGIC 0x50414c46u       /* "FLAP" as bytes 'F' 'L' 'A' 'P' */
#define FLE_AV_VERSION 1u
#define FLE_AV_MAX_WINDOWS 32u
#define FLE_AV_CLASS_BYTES 32u
#define FLE_AV_TITLE_BYTES 64u

/* flags of a window */
#define FLE_AV_W_VISIBLE 0x1u          /* the window and its parents have WS_VISIBLE and it is not minimised */
#define FLE_AV_W_ENABLED 0x2u          /* it is not disabled (a disabled window drops mouse input: S-disabled-drops of decision 0136) */
#define FLE_AV_W_TOPMOST 0x4u          /* WS_EX_TOPMOST */
#define FLE_AV_W_ACTIVE 0x8u           /* the active window of the foreground thread */
#define FLE_AV_W_FOCUS 0x10u           /* it holds the keyboard focus (or contains the focus window) */
#define FLE_AV_W_CAPTURE 0x20u         /* it holds (or contains the window that holds) the mouse capture */

typedef struct {
    uint32_t lid, hwnd, flags, owner_lid;
    int32_t x, y, width, height;
    uint32_t style, exstyle, thread, reserved;
    char cls[FLE_AV_CLASS_BYTES];
    char title[FLE_AV_TITLE_BYTES];
} fle_av_window;                       /* 144 bytes */

typedef struct {
    uint32_t magic, version, entry_size, max_windows;
    uint32_t seq;                      /* +1 on every poll that found anything different from the poll before */
    uint32_t cursor;                   /* the low 32 bits of the handle SetCursor set last, 0 = none (the handles are below 2 GiB: fle_cursor.h) */
    uint32_t cursor_idc;               /* the IDC_* identifier when the handle is a standard cursor of LoadCursorW / LoadImageW(NULL, ...), else 0 */
    uint32_t cursor_shared;            /* 1 when the handle is a cursor loaded from a module (fle_cursor_is_shared_handle) and not a standard one */
    int32_t pointer_x, pointer_y;      /* the pointer on the virtual screen (what GetCursorPos answers) */
    uint32_t pending_events;           /* raw input events queued by the host and not yet turned into messages */
    uint32_t n_windows;
    uint32_t screen_w, screen_h;       /* decision 0212: the size of the virtual screen the guest is told (fle_screen.h); 0 in a build older than this field: the page then takes 1920 x 1080 */
    uint32_t reserved[3];
    fle_av_window windows[FLE_AV_MAX_WINDOWS];
} fle_av_state;

/* Fills the state and returns its address (always the same). Exported to JS in the browser build. */
const fle_av_state *fle_av_poll(void);

#endif
