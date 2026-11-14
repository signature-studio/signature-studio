/* The virtual window table (docs/decisions/0071-virtual-windows.md, PROVISIONAL; the pages read and the guest's call sites are in
 * docs/investigations/window-design.md). Decision 0122 (cooperative guest threads, fle_thread.c): a window belongs to the thread that created it, and each thread has its own
 * message queue; the message functions are at the end of this file. A window is {window procedure, hInstance, style, exstyle, class atom,
 * live}; its handle is FLE_WIN_HANDLE_BASE + 4 * (index + 1); nothing is ever removed (DestroyWindow is not answered). Only the one
 * form of CreateWindowExW the guest reaches (the call at 0x52a8be: a hidden pop-up tool window of size 0, no parent, no menu, no
 * creation parameter) is answered; every other form stops the run through fle_stop (fle_env.h), and so does every argument of
 * SetWindowLongPtrW and DefWindowProcW that the pages leave open or the guest does not use. Nothing here reads the host: no real
 * window, no display, no message queue, no module, no clock. The last error is never touched. */
#define _GNU_SOURCE
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_host.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_callback.h"
#include "fle_sys.h"
#include "fle_class.h"
#include "fle_gdc.h"
#include "fle_gdi.h"
#include "fle_cursor.h"
#include "fle_loadimage.h"
#include "fle_monitor.h"
#include "fle_thread.h"
#include "fle_win.h"
#include "fle_visrgn.h"
#include "fle_input.h"
#include "fle_defproc_zero.h"

_Static_assert(FLE_WIN_HANDLE_BASE + FLE_WIN_REGION_SIZE <= UINT64_C(0x80000000), "the window handles are below 2 GiB");
_Static_assert(FLE_WIN_HANDLE_BASE + FLE_WIN_REGION_SIZE <= FLE_GDC_HANDLE_BASE, "the region lies before the screen DCs");
_Static_assert(FLE_WIN_HANDLE_BASE + FLE_WIN_REGION_SIZE <= FLE_GDI_HANDLE_BASE, "the region lies before the GDI objects");
_Static_assert(FLE_WIN_HANDLE_BASE + FLE_WIN_REGION_SIZE <= FLE_CURSOR_HANDLE_BASE, "the region lies before the standard cursors");
_Static_assert(FLE_WIN_HANDLE_BASE + FLE_WIN_REGION_SIZE <= FLE_LI_MODULE_HANDLE_BASE, "the region lies before the DLL's cursors and icons");
_Static_assert(FLE_WIN_HANDLE_BASE + FLE_WIN_REGION_SIZE <= FLE_MONITOR_HANDLE_BASE, "the region lies before the monitor");
_Static_assert(FLE_WIN_HANDLE_BASE > UINT64_C(0x7F000000), "the region is far above the image (0x400000..0x18C0000)");
_Static_assert(FLE_MENU_HANDLE_BASE + FLE_MENU_REGION_SIZE <= FLE_WIN_HANDLE_BASE, "the menu region lies before the window handles");
_Static_assert(FLE_MENU_HANDLE_BASE > UINT64_C(0x7F000000), "the menu region is far above the image");

/* Decision 0123: BeginPaint and EndPaint take and give back the screen DC of fle_gdc.c, and the timers read the virtual clock of fle_sys.c. Weak, so that a link without them (the unit tests of the window table) still links: BeginPaint then
 * stops by name and the timers use a clock that never moves. */
void fle_imp_GetDC(void) __attribute__((weak));
int fle_gdi_region_bounds(uint64_t h, int32_t *l, int32_t *t, int32_t *r, int32_t *b) __attribute__((weak));
/* decision 0126: weak, so that a unit-test link of this file without fle_gdi.c / fle_monitor.c (decisions 0071, 0123) still links; a call without them stops by name */
int32_t fle_gdi_cursor_display_count(void) __attribute__((weak));
uint64_t fle_gdi_cursor_current(void) __attribute__((weak));
uint64_t fle_monitor_resolve(const char *api, int intersects, uint32_t flags) __attribute__((weak));
int fle_monitor_rect_intersects(int32_t l, int32_t t, int32_t r, int32_t b) __attribute__((weak));
int fle_gdi_region_set_rect(uint64_t h, int32_t l, int32_t t, int32_t r, int32_t b) __attribute__((weak));
int fle_gdi_region_pieces(uint64_t h, int32_t (*out)[4], uint32_t max, uint32_t *n) __attribute__((weak));   /* decision 0247 */
int fle_gdi_region_set_pieces(uint64_t h, const int32_t (*p)[4], uint32_t n) __attribute__((weak));   /* decision 0247 */
int fle_gdi_region_covers(uint64_t h, int32_t l, int32_t t, int32_t r, int32_t b) __attribute__((weak));
void fle_imp_ReleaseDC(void) __attribute__((weak));
/* Decision 0125: a destroyed window takes its private DC with it (fle_gdc.c). Weak: a link without fle_gdc.c has no DC. */
void fle_gdc_window_destroyed(uint64_t hwnd) __attribute__((weak));
/* Decision 0126: the drawing surface of a top-level window goes with the window (fle_raster.c). Weak: a link without it has no surface. */
void fle_raster_window_destroyed(uint32_t lid) __attribute__((weak));
/* decision 0140 (layered windows, fle_layered.c): weak, like the two above */
void fle_layered_exstyle_changed(uint64_t hwnd, uint32_t old_ex, uint32_t new_ex) __attribute__((weak));
/* Decision 0209 (crash1): menus made by CreateMenu / CreatePopupMenu (fle_crash1_menu.c) answer CheckMenuItem, EnableMenuItem and DeleteMenu themselves; weak: a link without that file has only the window menus below. */
extern int fle_crash1_menu_owns(uint64_t h) __attribute__((weak));
extern uint32_t fle_crash1_menu_check_item(uint64_t h, uint32_t id, uint32_t flags) __attribute__((weak));
extern uint32_t fle_crash1_menu_enable_item(uint64_t h, uint32_t id, uint32_t flags) __attribute__((weak));
extern uint32_t fle_crash1_menu_delete_item(uint64_t h, uint32_t id, uint32_t flags) __attribute__((weak));
void fle_layered_window_destroyed(uint64_t hwnd) __attribute__((weak));
uint64_t fle_sys_clock_ticks(void) __attribute__((weak));

/* Weak, so that a link without fle_sys.c (the unit tests of the window table) sees no VirtualAlloc region at all and hands every procedure to fle_call_guest. */
int fle_sys_region_at(uint64_t addr, fle_sys_region_view *out) __attribute__((weak));

struct wprop {
    int is_atom;                                /* lpString was an atom (the low word, the high-order word zero) */
    uint16_t atom;
    uint16_t name[FLE_CLASS_NAME_MAX_UNITS + 1];
    uint16_t nlen;
    uint64_t data;
};

typedef struct {
    uint64_t wndproc;
    uint64_t hinst;
    uint32_t style;
    uint32_t exstyle;
    uint16_t atom;
    int live;
    int32_t x, y, cx, cy;                       /* decision 0119: the window rectangle given at creation (screen coordinates; no parent) */
    uint16_t text[FLE_WIN_TEXT_MAX_UNITS + 1];  /* decision 0119: lpWindowName at creation, NUL-terminated */
    uint16_t text_len;
    int wts_registered;                         /* decision 0119: WTSRegisterSessionNotification was called for this window */
    uint32_t wts_flags;                         /* the dwFlags of the first registration (later ones are ignored: Learn) */
    uint64_t icon_big, icon_small;              /* decision 0119: the handles set by WM_SETICON (ICON_BIG, ICON_SMALL); 0 = none */
    int has_menu;                               /* decision 0119: GetSystemMenu(hwnd, FALSE) made the copy of the window menu */
    uint32_t menu_items;                        /* bit k = item k is in the menu (FLE_SC order below) */
    uint64_t userdata;                          /* decision 0119: GWLP_USERDATA ("Its value is initially zero") */
    int msgonly;                                /* decision 0121: a message-only window (hWndParent HWND_MESSAGE at creation) */
    uint32_t owner;                             /* decision 0122: the index of the guest thread that created the window (Learn: the window belongs to the thread that created it) */
    /* Decision 0123 (PROVISIONAL), the window-management state (docs/decisions/0123-window-model-and-walls.md; R1..R10 of docs/investigations/detach-window.md section 3.2). STATE ONLY: nothing is drawn. */
    uint32_t lid;                               /* R5: the logical window id (never reused); the handle is a separate value (tab index) */
    int64_t parent;                             /* R1: the parent window of a WS_CHILD window (index), -1 = none (top-level: the desktop is the parent) */
    int64_t owner_win;                          /* R1: the owner window of an owned top-level window (index), -1 = none. A window has a parent OR an owner, never both (Learn SetWindowLongPtrW) */
    int32_t px, py;                             /* R2: the position of the window rectangle: screen coordinates for a top-level window, the parent's client coordinates for a child */
    int32_t pcx, pcy;                           /* the window size (the client size is the same: the non-client area is 0, see nc_zero) */
    int visible;                                /* the WS_VISIBLE bit of style (kept in style too) */
    int enabled;                                /* not WS_DISABLED */
    int destroyed;                              /* DestroyWindow ran (live == 0 too) */
    int iconic, zoomed;                         /* show state (SW_SHOWMINIMIZED / SW_SHOWMAXIMIZED) */
    int32_t rest_x, rest_y, rest_cx, rest_cy;   /* the restored rectangle (kept across minimize/maximize) */
    int restore_max;                            /* decision 0258: the window was maximized when it was minimized (ShowWindow(SW_RESTORE) then gives the maximized rectangle back) */
    int min_set;                                /* decision 0267: the window was minimized once (GetWindowPlacement then reports ptMinPosition (-32000, -32000), not (-1, -1)) */
    int32_t upd_l, upd_t, upd_r, upd_b;         /* the update region as one bounding rectangle in client coordinates (empty = none) */
    int upd_erase;                              /* bErase of some InvalidateRect */
    int upd_delayed;                            /* decision 0323: WM_ERASEBKGND was sent for this update region and answered 0: it stays marked for erasing, without a second message (Wine 10.0: UPDATE_DELAYED_ERASE): BeginPaint then reports fErase = TRUE */
    fle_visrgn *updr;                           /* decision 0247 (alpha3): the update region when it is more than one rectangle (disjoint pieces, client coordinates; upd_l..upd_b is their bounding box); NULL = the region is the rectangle upd_l..upd_b */
    int paint_internal;                         /* RedrawWindow RDW_INTERNALPAINT (not used) */
    int in_paint;                               /* between BeginPaint and EndPaint */
    int ever_shown;                             /* ShowWindow was called at least once */
    uint64_t hfont;                             /* decision 0123: the font handle of WM_SETFONT (kept as a number; nothing is drawn) */
    int no_redraw;                              /* decision 0126: WM_SETREDRAW with wParam FALSE: drawing in the window (and its children) is not shown until WM_SETREDRAW TRUE */
    uint64_t wid;                               /* decision 0123: GWLP_ID (CreateWindowExW's hMenu of a child window; Learn SetWindowLongPtrW: "the identifier of the child window") */
    uint64_t rgn;                               /* decision 0171: the window region set by SetWindowRgn (a GDI region handle owned by the system; 0 = none); decision 0244: the compositor shows only the part of the window inside it (fle_win_window_region_box) */
    int32_t dwm_margins[4];                     /* decision 0244: DwmExtendFrameIntoClientArea's MARGINS (left, right, top, bottom), all 0 = the frame is not extended into the client area (fle_win_frame_margins) */
    int sop_hidden;                             /* decision 0171: hidden by ShowOwnedPopups(FALSE) (ShowOwnedPopups(TRUE) shows only those; Learn)*/
    uint64_t hmenu;                             /* decision 0123: the menu handle set by SetMenu (0 = none; a handle other than NULL is not modelled: SetMenu stops) */
    uint32_t nprops;                            /* R7: the property list (SetPropW / GetPropW / RemovePropW) */
    struct wprop *props;
} win;

static win *tab;
static uint64_t tab_n, tab_cap;
static int64_t fle_win_owner_tid;
static uint32_t buffered_paint_init_count;     /* decision 0119: BufferedPaintInit calls not yet matched by BufferedPaintUnInit (the latter is not answered) */

static void fle_win_check_thread(const char *api)
{
    int64_t tid = (int64_t)fle_host_tid();

    if (fle_win_owner_tid == 0) fle_win_owner_tid = tid;
    if (tid != fle_win_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

uint64_t fle_win_count(void)
{
    return tab_n;
}

/* The index of the window that `h` is the handle of, or -1. The whole 64 bits are compared: a value that only agrees in the low 32
 * bits is not a handle of this table (Learn says the low 32 bits are significant; no form of the guest gives another value). */
static int64_t index_of(uint64_t h)
{
    uint64_t off;

    if (h <= FLE_WIN_HANDLE_BASE || h >= FLE_WIN_HANDLE_BASE + FLE_WIN_REGION_SIZE) return -1;
    off = h - FLE_WIN_HANDLE_BASE;
    if (off % FLE_WIN_HANDLE_STEP != 0) return -1;
    off = off / FLE_WIN_HANDLE_STEP - 1u;
    if (off >= tab_n) return -1;
    return (int64_t)off;
}

int fle_win_get(uint64_t hwnd, fle_win_view *v)
{
    int64_t i = index_of(hwnd);

    if (i < 0) return 0;
    v->wndproc = tab[i].wndproc;
    v->hinst = tab[i].hinst;
    v->style = tab[i].style;
    v->exstyle = tab[i].exstyle;
    v->atom = tab[i].atom;
    v->live = tab[i].live;
    v->x = tab[i].x;
    v->y = tab[i].y;
    v->cx = tab[i].cx;
    v->cy = tab[i].cy;
    v->text_len = tab[i].text_len;
    memcpy(v->text, tab[i].text, sizeof v->text);
    v->wts_registered = tab[i].wts_registered;
    v->wts_flags = tab[i].wts_flags;
    v->icon_big = tab[i].icon_big;
    v->icon_small = tab[i].icon_small;
    v->userdata = tab[i].userdata;
    v->msgonly = tab[i].msgonly;
    v->owner = tab[i].owner;
    return 1;
}

/* Decision 0209 (crash1): set the extended-style bits of `mask` to `value` on the live window `hwnd` (shell32!DragAcceptFiles sets and clears WS_EX_ACCEPTFILES, fle_crash1_drag.c). 1 when the window exists and is live, else 0 (nothing changes). */
int fle_win_set_exstyle_bits(uint64_t hwnd, uint32_t mask, uint32_t value)
{
    int64_t i = index_of(hwnd);

    if (i < 0 || !tab[i].live) return 0;
    tab[i].exstyle = (tab[i].exstyle & ~mask) | (value & mask);
    return 1;
}

/* The window `h` for an API that needs a live window of this table; anything else stops (the pages give no value for a bad handle:
 * SetWindowLongPtrW and DefWindowProcW only say "the return value is zero ... GetLastError" or "depends on the message"). */
static int64_t need_window(const char *api, uint64_t h)
{
    char why[160];
    int64_t i = index_of(h);

    if (i < 0) {
        snprintf(why, sizeof why, "hwnd 0x%" PRIx64 " is not a window of this table (NULL, a window of another kind, or never created): the page gives no value for a bad handle", h);
        fle_stop(api, why);
    }
    if (!tab[i].live) {
        snprintf(why, sizeof why, "hwnd 0x%" PRIx64 " is a window whose creation failed (it was destroyed): the page gives no value for a bad handle", h);
        fle_stop(api, why);
    }
    return i;
}

/* ---- Decision 0119 (PROVISIONAL): the rest of the window model, appended after the three functions of decision 0071 ---- */

/* The window procedure of a window, called with one message (decision 0119, wall 13). A procedure that is an address in a VirtualAlloc region is the
 * thunk of the guest's Delphi MakeObjectInstance (f_52a5b0, decision 0071 investigation section 3.3), which the guest wrote itself at run time:
 *     slot:        E8 rel32                 call template        (pushes slot + 5)
 *     template:    41 5B                    pop r11              (r11 = slot + 5; the stack is as at the slot's entry)
 *                  FF 25 00 00 00 00        jmp [rip + 0]        (the 8 bytes right after it: the guest function that the thunk jumps to)
 * The bytes are read as the instructions they are: when the slot, the template and the pointer have exactly this shape, the guest function the
 * pointer names is called through fle_call_guest with r11 = slot + 5 (which is all that the three instructions do to the machine state); the
 * function reads TMethod (Code, Data) at r11 and r11 + 8. Any other shape (the page may not be run, the bytes differ, the call leaves the
 * region, the pointer is not a function entry: fle_call_guest stops by name) stops here by name. Any other address is a function entry of the
 * image or stops in fle_call_guest. Nothing is invented: a window procedure that is not one of these two forms never runs. */
/* The thunk shape check shared by call_wndproc (a call from the runtime) and fle_win_try_thunk (a call from the guest's own code through fle_dispatch_miss, decision 0123 wall 31). Returns the guest function that the
 * thunk jumps to, and sets R[11] = slot + 5 as the three instructions do. Any other shape stops by name (`api` names the stopped call). */
static uint64_t thunk_target(const char *api, uint64_t proc, const fle_sys_region_view *rvp)
{
    const fle_sys_region_view rv = *rvp;
    uint64_t target, jump, page;
    int32_t rel;
    char why[200];

    if (proc + 5 + 8 + 8 > rv.base + rv.size) fle_stop(api, "the window procedure lies at the end of a VirtualAlloc region: not the thunk of MakeObjectInstance (decision 0119)");
    page = (proc - rv.base) / FLE_PAGE_SIZE;
    if (rv.pages[page] != 0x10u && rv.pages[page] != 0x20u && rv.pages[page] != 0x40u && rv.pages[page] != 0x80u) {
        snprintf(why, sizeof why, "the window procedure 0x%" PRIx64 " lies in a page of a VirtualAlloc region that is not marked for running code: not called (decision 0119)", proc);
        fle_stop(api, why);
    }
    if (fle_ld(proc, 1) != 0xE8u) {
        snprintf(why, sizeof why, "the window procedure 0x%" PRIx64 " is in a VirtualAlloc region and does not start with E8 (call rel32): not the thunk of MakeObjectInstance (decision 0119)", proc);
        fle_stop(api, why);
    }
    rel = (int32_t)(uint32_t)fle_ld(proc + 1, 4);
    target = proc + 5 + (uint64_t)(int64_t)rel;
    if (target < rv.base || target + 16 > rv.base + rv.size) fle_stop(api, "the E8 call of the window procedure leaves its VirtualAlloc region: not the thunk of MakeObjectInstance (decision 0119)");
    if (fle_ld(target, 8) != FLE_THUNK_TEMPLATE) {
        snprintf(why, sizeof why, "the bytes at 0x%" PRIx64 " (the target of the window procedure's call) are not `pop r11; jmp [rip+0]` (41 5B FF 25 00 00 00 00): not the thunk of MakeObjectInstance (decision 0119)", target);
        fle_stop(api, why);
    }
    jump = fle_ld(target + 8, 8);
    R[11] = proc + 5;
    return jump;
}

/* Decision 0123 (wall 31): the guest's own `call [reg+disp]` of the slot (fle_dispatch_miss ends here). The slot is entered as the three instructions do: the return address the caller pushed stays at [rsp] and
 * the guest function it jumps to is called through fle_dispatch (the same stack, the same arguments: nothing of R is changed but r11). Returns 0 when `va` is not in a VirtualAlloc region of this table (the caller then
 * stops by name as it always did); a form that is in such a region and is not the slot stops here by name with the reason. */
/* Decision 0139 (PROVISIONAL, S-com-slot-range): the method slots of the COM objects that fle_dsound.c hands to the guest are pseudo-VAs of a range of their own. The generated code reaches no other hook when a guest `call [reg+disp]`
 * finds a value that is not a function entry (fle_dispatch_miss asks fle_win_try_thunk first and only that), so the slots are answered here, before the thunk shape check: 1 and the method has run, or 0 and the check goes on. The
 * generated code, and with it every object compiled from it, is not changed. */
int fle_com_try_slot(uint64_t va) __attribute__((weak));

int fle_win_try_thunk(uint64_t va)
{
    fle_sys_region_view rv;
    uint64_t jump;

    if (fle_com_try_slot && fle_com_try_slot(va)) return 1;
    if (!fle_sys_region_at || !fle_sys_region_at(va, &rv)) return 0;
    jump = thunk_target("indirect call", va, &rv);
    fle_dispatch(jump);
    return 1;
}

static void callwndproc_hooks(int64_t wi, uint64_t h, uint32_t msg, uint64_t wp, uint64_t lp);   /* decision 0171 */

/* decision 0222 (diagnostic, opt-in): with fle_win_trace != 0 and a harness that defines fle_win_trace_msg / fle_win_trace_z (harness/fruity_g3.c, native fork-probe builds: the g3 command `wintrace on`), every message that reaches a
 * window procedure (before the call and, with its result, after it) and every z-order move is reported to it. Nothing here changes what the window model does. */
int fle_win_trace;
void fle_win_trace_msg(int phase, const char *api, uint32_t lid, const char *cls, uint32_t msg, uint64_t wp, uint64_t lp, uint64_t result) __attribute__((weak));
void fle_win_trace_z(const char *what, uint32_t lid, const char *cls, uint32_t after_lid, const char *after_cls) __attribute__((weak));
static int64_t index_of_fwd(uint64_t h);
static void w2_desc(uint64_t h, uint32_t *lid, char *cls, size_t cap)
{
    int64_t i = index_of_fwd(h);

    cls[0] = 0;
    *lid = 0;
    if (i >= 0) {
        uint16_t nm[FLE_CLASS_NAME_MAX_UNITS + 1], nl = 0, k;

        *lid = tab[i].lid;
        if (fle_class_name_of(tab[i].atom, nm, &nl)) {
            for (k = 0; k < nl && k + 1 < cap; k++) cls[k] = nm[k] < 128 ? (char)nm[k] : '?';
            cls[k] = 0;
        }
    }
}

static uint64_t call_wndproc_untraced(const char *api, uint64_t proc, uint64_t h, uint32_t msg, uint64_t wp, uint64_t lp)
{
    fle_sys_region_view rv;
    uint64_t jump;

    if (!fle_sys_region_at || !fle_sys_region_at(proc, &rv)) return fle_call_guest(proc, h, msg, wp, lp);
    jump = thunk_target(api, proc, &rv);
    return fle_call_guest(jump, h, msg, wp, lp);
}

static uint64_t call_wndproc(const char *api, uint64_t proc, uint64_t h, uint32_t msg, uint64_t wp, uint64_t lp)
{
    uint64_t r;
    uint32_t lid;
    char cls[64];

    if (!fle_win_trace || !fle_win_trace_msg) return call_wndproc_untraced(api, proc, h, msg, wp, lp);
    w2_desc(h, &lid, cls, sizeof cls);
    fle_win_trace_msg(0, api, lid, cls, msg, wp, lp, 0);
    r = call_wndproc_untraced(api, proc, h, msg, wp, lp);
    fle_win_trace_msg(1, api, lid, cls, msg, wp, lp, r);
    return r;
}

static int64_t index_of_fwd(uint64_t h) { return index_of(h); }

static int64_t live_index(uint64_t h)
{
    int64_t i = index_of(h);

    return (i >= 0 && tab[i].live) ? i : -1;
}

int fle_win_synth_message(unsigned t, uint64_t fh, uint32_t fmin, uint32_t fmax, int remove, uint32_t qs, fle_tmsg *out);
static uint64_t clk_now(void) { return fle_sys_clock_ticks ? fle_sys_clock_ticks() : 0u; }
static void input_window_cancelmode(int64_t i);
static void input_set_focus_default(const char *api, int64_t i);
static void timers_of_window_gone(int64_t w);
static void input_window_destroyed(int64_t i);
static void defproc_windowposchanged(const char *api, int64_t i, uint64_t wpa);
static void note_visibility_changed(void);
static int is_locked_or_child(int64_t i);
static int is_no_redraw_chain(int64_t i);
/* ---- Decision 0123 (PROVISIONAL): the window-management state. Helpers. ---- */
static uint32_t next_lid = 1;
static int g2_seen;                            /* the first visible top-level window has appeared (G2) */
static char g2_text[12000];

static uint64_t handle_of(int64_t i) { return FLE_WIN_HANDLE_BASE + (uint64_t)FLE_WIN_HANDLE_STEP * ((uint64_t)i + 1u); }

/* The window `i` is a top-level window of the desktop (no parent window; owned or not). A message-only window is not part of the z-order and is not enumerated (Learn Window Features). */
static int is_toplevel(int64_t i) { return tab[i].parent < 0 && !tab[i].msgonly; }

/* The parent of a child window, the owner of an owned window, or -1. R1: kept in two fields. */
static int64_t root_of(int64_t i)
{
    while (tab[i].parent >= 0) i = tab[i].parent;
    return i;
}

/* R2: the screen position of the upper-left corner of the client area of window i. The non-client area is 0 (nc_zero: WM_NCCALCSIZE is sent and the answer of DefWindowProcW leaves the rectangle as it is, so the
 * client rectangle is the window rectangle): a CHOICE (Learn does not give frame sizes; a real window has a frame and a caption that the VCL forms of FL do not have: they are borderless in the sense of bsNone). */
static void client_origin(int64_t i, int32_t *ox, int32_t *oy)
{
    int32_t x = 0, y = 0;

    for (;;) {
        x += tab[i].px;
        y += tab[i].py;
        if (tab[i].parent < 0) break;
        i = tab[i].parent;
    }
    *ox = x;
    *oy = y;
}

/* R3: the z-order is one explicit list per sibling group (the children of one parent window; parent -1 = the top-level windows of the desktop). `zl` holds the windows of ALL groups in one sequence, top first;
 * the order of the members of one group is the order in which they appear. TOPMOST windows come before the others of their group (Learn Window Features: "All topmost windows appear in the z-order before any
 * non-topmost windows"; this is enforced when a window is inserted or its TOPMOST bit changes, see z_place). The window table index and the handle do not depend on this order. */
static int64_t *zl;
static uint32_t zl_n, zl_cap;

static int in_group(int64_t k, int64_t parent) { return tab[k].live && tab[k].parent == parent && !(parent < 0 && tab[k].msgonly); }

void fle_visrgn_set_rect(fle_visrgn *v, fle_rrect r) __attribute__((weak));
int fle_visrgn_subtract(fle_visrgn *v, fle_rrect s) __attribute__((weak));
void fle_visrgn_intersect_rect(fle_visrgn *v, fle_rrect c) __attribute__((weak));
int fle_visrgn_intersect(fle_visrgn *a, const fle_visrgn *b) __attribute__((weak));
fle_rrect fle_visrgn_bounds(const fle_visrgn *v) __attribute__((weak));
/* decision 0214: the pixels of a window surface that the window manager moves (SetWindowPos without SWP_NOCOPYBITS) and the dirty-rectangle notice for them; weak like the rest of the raster side */
const fle_rsurf *fle_raster_surface_peek(uint32_t lid) __attribute__((weak));
void fle_raster_wrote(const fle_rsurf *s, fle_rrect r, const char *api) __attribute__((weak));
uint8_t *fle_r_row(const fle_rsurf *s, uint32_t y) __attribute__((weak));
int fle_r_direct(const fle_rsurf *s) __attribute__((weak));
void fle_gdc_set_paint_rect(uint64_t h, int valid, int32_t l, int32_t t, int32_t r, int32_t b) __attribute__((weak));
void fle_gdc_set_paint_pieces(uint64_t h, const int32_t (*p)[4], uint32_t n) __attribute__((weak));   /* decision 0247 */
int fle_gdc_is_window_dc(uint64_t h) __attribute__((weak));   /* decision 0323 */
void fle_imp_FillRect(void) __attribute__((weak));   /* decision 0323: DefWindowProc(WM_ERASEBKGND) fills with the class background brush */
void fle_imp_GetClipBox(void) __attribute__((weak));
static void upd_clear_rect(int64_t i);   /* decision 0247 (alpha3): the update region as a set of rectangles; see upd_add */
static void upd_set_full(int64_t i);

static void zl_remove(int64_t i)
{
    uint32_t k;

    for (k = 0; k < zl_n; k++) {
        if (zl[k] == i) {
            memmove(zl + k, zl + k + 1, (zl_n - k - 1u) * sizeof *zl);
            zl_n--;
            return;
        }
    }
}

static void zl_insert_at(uint32_t pos, int64_t i)
{
    if (zl_n == zl_cap) {
        uint32_t nc = zl_cap ? zl_cap * 2u : 32u;
        int64_t *n = (int64_t *)realloc(zl, nc * sizeof *zl);

        if (!n) fle_stop("window table", "out of memory");
        zl = n;
        zl_cap = nc;
    }
    memmove(zl + pos + 1, zl + pos, (zl_n - pos) * sizeof *zl);
    zl[pos] = i;
    zl_n++;
}

/* The position in `zl` of the first member of the group of `parent` (zl_n when the group is empty) and the position just after its last member. */
static void group_span(int64_t parent, uint32_t *first, uint32_t *end)
{
    uint32_t k, f = zl_n, e = zl_n;

    for (k = 0; k < zl_n; k++) {
        if (in_group(zl[k], parent)) {
            if (f == zl_n) f = k;
            e = k + 1u;
        }
    }
    *first = f == zl_n ? zl_n : f;
    *end = e;
}

/* The members of the group of `parent`, top first. */
static uint32_t siblings(int64_t parent, int64_t *idx, uint32_t cap)
{
    uint32_t k, n = 0;

    for (k = 0; k < zl_n && n < cap; k++)
        if (in_group(zl[k], parent)) idx[n++] = zl[k];
    return n;
}

#define SIB_MAX 4096u

/* Places window i in the list of its group (it must not be in `zl`): mode 0 = top (of its TOPMOST class: a non-topmost window goes below the topmost ones), 1 = bottom of the group, 2 = directly below window `after`.
 * A topmost window never goes below a non-topmost one and the reverse (Learn: "All topmost windows appear in the z-order before any non-topmost windows"). */
static void z_place(int64_t i, int mode, int64_t after)
{
    int64_t parent = tab[i].parent;
    int top = (tab[i].exstyle & FLE_WS_EX_TOPMOST) != 0;
    uint32_t first, end, k, pos;

    group_span(parent, &first, &end);
    if (first == zl_n) {
        zl_insert_at(zl_n, i);
        return;
    }
    /* the span of the topmost windows of the group is [first, first + nt) and the others follow */
    {
        uint32_t nt = 0;

        for (k = first; k < end; k++)
            if (in_group(zl[k], parent) && (tab[zl[k]].exstyle & FLE_WS_EX_TOPMOST)) nt++;
        if (mode == 0) pos = top ? first : first + nt;
        else if (mode == 1) pos = top ? first + nt : end;
        else {
            uint32_t ap = zl_n;

            for (k = first; k < end; k++)
                if (zl[k] == after) ap = k;
            if (ap == zl_n) pos = first + nt;
            else {
                int atop = (tab[after].exstyle & FLE_WS_EX_TOPMOST) != 0;

                pos = ap + 1u;
                if (top && !atop) pos = first + nt;      /* a topmost window cannot go below a non-topmost one: it stays at the bottom of the topmost block */
                if (!top && atop) pos = first + nt;      /* a non-topmost window cannot go above a topmost one: it goes to the top of the non-topmost block */
            }
        }
    }
    zl_insert_at(pos, i);
}

static void w2_ztrace(const char *what, int64_t i, int64_t after);
static void z_to_top(int64_t i)
{
    w2_ztrace("top", i, -1);
    zl_remove(i);
    z_place(i, 0, -1);
}

static void z_to_bottom(int64_t i)
{
    w2_ztrace("bottom", i, -1);
    zl_remove(i);
    z_place(i, 1, -1);
}

static void z_below(int64_t i, int64_t after)
{
    w2_ztrace("below", i, after);
    zl_remove(i);
    z_place(i, 2, after);
}

static void w2_ztrace(const char *what, int64_t i, int64_t after)
{
    if (fle_win_trace && fle_win_trace_z) {
        uint32_t lid, alid = 0;
        char cls[64], acls[64];

        w2_desc(handle_of(i), &lid, cls, sizeof cls);
        acls[0] = 0;
        if (after >= 0) w2_desc(handle_of(after), &alid, acls, sizeof acls);
        fle_win_trace_z(what, lid, cls, alid, acls);
    }
}

/* The window `i` is visible in the sense of Learn IsWindowVisible: it and all its parents have WS_VISIBLE. */
static int is_visible(int64_t i)
{
    for (;;) {
        if (!(tab[i].style & FLE_WS_VISIBLE)) return 0;
        if (tab[i].parent < 0) return 1;
        i = tab[i].parent;
    }
}

static int is_enabled_chain(int64_t i)
{
    for (;;) {
        if (tab[i].style & FLE_WS_DISABLED) return 0;
        if (tab[i].parent < 0) return 1;
        i = tab[i].parent;
    }
}

/* Weak: the lifted-entry counter of the harness (harness/wasm/trace_native.c); a link without it reports 0. */
uint64_t trace_call_count(void) __attribute__((weak));

static void u16_ascii(char *out, size_t cap, const uint16_t *u, size_t n)
{
    size_t k;

    for (k = 0; k < n && k + 1 < cap; k++) out[k] = u[k] < 128 && u[k] >= 32 ? (char)u[k] : '?';
    out[k] = 0;
}

/* One line for a window of the table (decision 0126: the G2 evidence of the main form and the first non-background pixel print the table in this form: class, title, rectangle, styles, parent, owner, visible, z position). */
static void table_line(FILE *f, int64_t i)
{
    char title[120], cls[80];
    uint16_t nm[FLE_CLASS_NAME_MAX_UNITS + 1], nl = 0;
    uint32_t z = 0, k, first, end;
    int64_t par = tab[i].parent >= 0 ? tab[i].parent : tab[i].owner_win;

    u16_ascii(title, sizeof title, tab[i].text, tab[i].text_len < 100 ? tab[i].text_len : 100);
    cls[0] = 0;
    if (fle_class_name_of(tab[i].atom, nm, &nl)) u16_ascii(cls, sizeof cls, nm, nl < 79 ? nl : 79);
    group_span(tab[i].parent, &first, &end);
    for (k = first; k < end; k++) {
        if (!in_group(zl[k], tab[i].parent)) continue;
        if (zl[k] == i) break;
        z++;
    }
    fprintf(f, "    handle %#" PRIx64 " lid %u class \"%s\" title \"%s\" rect (%d, %d, %d x %d) style %#x exstyle %#x %s %#" PRIx64 " visible %d enabled %d z %u thread %u%s\n",
            handle_of(i), (unsigned)tab[i].lid, cls, title, (int)tab[i].px, (int)tab[i].py, (int)tab[i].pcx, (int)tab[i].pcy, (unsigned)tab[i].style, (unsigned)tab[i].exstyle,
            tab[i].parent >= 0 ? "parent" : "owner", par >= 0 ? handle_of(par) : (uint64_t)0, is_visible(i), tab[i].enabled, (unsigned)z, (unsigned)tab[i].owner, tab[i].msgonly ? " message-only" : "");
}

void fle_win_timer_report(FILE *f);

void fle_win_table_report(FILE *f)
{
    uint64_t k;
    int64_t idx[SIB_MAX];
    uint32_t n = siblings(-1, idx, SIB_MAX), z, live = 0;

    for (k = 0; k < tab_n; k++) live += tab[k].live != 0;
    fprintf(f, "  window table: %" PRIu64 " entries, %u live; top-level windows in z-order, top first, each followed by its children (z = position in its group, top first):\n", tab_n, (unsigned)live);
    for (z = 0; z < n; z++) {
        int64_t stack[64], sp = 0, i;

        stack[sp++] = idx[z];
        while (sp > 0) {
            int64_t ch[SIB_MAX];
            uint32_t m, q;

            i = stack[--sp];
            table_line(f, i);
            m = siblings(i, ch, SIB_MAX);
            for (q = m; q > 0 && sp < 64; q--) stack[sp++] = ch[q - 1];
        }
    }
}

static void g2_describe(FILE *f)
{
    fprintf(f, "G2 evidence (decision 0123): the window table when the first visible top-level window appeared (function entries since the program began: %" PRIu64 ")\n",
            trace_call_count ? trace_call_count() : (uint64_t)0);
    fle_win_table_report(f);
}

/* decision 0222: the diagnostic of decision 0126 that printed the window table once when a window of one particular class of the application (its main form) became visible is removed: it branched on the application's
 * class name (docs/policies/lift-fidelity.md) and nothing read it. G2 above (the first visible top-level window, whichever it is) stays. */

/* Called when a window became visible (CreateWindowExW with WS_VISIBLE, ShowWindow, SetWindowPos with SWP_SHOWWINDOW). The first time a top-level window of the table is visible, the table is recorded (G2 of decision 0123);
 * (decision 0126's second record, for the application's main form, is removed: see above). */
static void note_visibility_changed(void)
{
    uint64_t k;
    char *buf = NULL;
    size_t len = 0;
    FILE *m;

    if (g2_seen) return;
    for (k = 0; k < tab_n; k++) {
        if (tab[k].live && is_toplevel((int64_t)k) && is_visible((int64_t)k)) break;
    }
    if (k == tab_n) return;
    g2_seen = 1;
    m = open_memstream(&buf, &len);
    if (m) {
        g2_describe(m);
        fclose(m);
        snprintf(g2_text, sizeof g2_text, "%s", buf ? buf : "");
        free(buf);
    }
    fputs(g2_text, stderr);
}

uint32_t fle_win_visible_toplevel_count(void)
{
    uint32_t n = 0;
    uint64_t k;

    for (k = 0; k < tab_n; k++)
        if (tab[k].live && is_toplevel((int64_t)k) && is_visible((int64_t)k)) n++;
    return n;
}

int fle_win_g2_seen(void) { return g2_seen; }

void fle_win_g2_report(FILE *f)
{
    if (g2_seen) fputs(g2_text, f);
}

/* The answer for an hWnd that is not a live window of this table (NULL, never issued, or a window whose creation failed): 0 and
 * ERROR_INVALID_WINDOW_HANDLE (1400). A CHOICE WITHOUT A SOURCE (decision 0119, PM's decision 1): the Learn pages of SendMessageW,
 * SetClassLongPtrW, GetWindowLongPtrW, SetWindowLongPtrW say only "zero ... GetLastError" for a failure; System Error Codes lists 1400
 * "Invalid window handle." and does not name an API. */
static void invalid_window(void)
{
    fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE);
    R[0] = 0;
}

#define FLE_HWND_BROADCAST UINT64_C(0xFFFF)
#define FLE_WM_QUIT_ID FLE_WM_QUIT
#define FLE_ERROR_NOT_ENOUGH_QUOTA_W FLE_ERROR_NOT_ENOUGH_QUOTA
#define FLE_QS_POSTMESSAGE_W FLE_QS_POSTMESSAGE
#define FLE_QS_ALLPOSTMESSAGE_W FLE_QS_ALLPOSTMESSAGE

/* user32!SendMessageW: LRESULT f(HWND hWnd rcx, UINT Msg rdx, WPARAM wParam r8, LPARAM lParam r9). Learn: "The SendMessage function calls
 * the window procedure for the specified window and does not return until the window procedure has processed the message"; "If the
 * specified window was created by the calling thread, the window procedure is called immediately as a subroutine"; the return value
 * "specifies the result of the message processing; it depends on the message sent". HWND_BROADCAST (0xFFFF) is defined by the page (all
 * top-level windows): not modelled, stops. A window of this table gets the message through fle_call_guest and the result is the whole
 * 64 bits of the procedure's return value. An hWnd that is not a live window of this table (NULL included): 0 and last error 1400. */
void fle_imp_SendMessageW(void)
{
    static const char api[] = "SendMessageW";
    uint64_t h = R[1], wp = R[8], lp = R[9];
    uint32_t msg = (uint32_t)R[2];
    int64_t i;

    fle_win_check_thread(api);
    if (h == FLE_HWND_BROADCAST) fle_stop(api, "hWnd is HWND_BROADCAST (0xffff): sending to all top-level windows is not modelled (decision 0119)");
    i = live_index(h);
    if (i < 0) {
        invalid_window();
        return;
    }
    if (fle_thread_current && tab[i].owner != fle_thread_current()) {
        /* Decision 0122 (Learn SendMessage: "If the specified window was created by a different thread, the system switches to that thread and calls the appropriate window procedure. Messages sent
         * between threads are processed only when the receiving thread executes message retrieval code. The sending thread is blocked until the receiving thread processes the message. However, the
         * sending thread will process incoming nonqueued messages while waiting for its message to be processed."): the message is queued as a sent message of the owner, the sender waits, and the
         * window procedure runs in the owner's context when the owner calls GetMessageW, PeekMessageW, WaitMessage or MsgWaitForMultipleObjects. */
        fle_sent sm;

        memset(&sm, 0, sizeof sm);
        sm.hwnd = h;
        sm.msg = msg;
        sm.wp = wp;
        sm.lp = lp;
        fle_thread_send(tab[i].owner, &sm);
        fle_thread_wait_sent(api, &sm, tab[i].owner);
        R[0] = sm.result;
        return;
    }
    callwndproc_hooks(i, h, msg, wp, lp);
    R[0] = call_wndproc(api, tab[i].wndproc, h, msg, wp, lp);
}

/* user32!SendMessageA (decision 0229, ref1, PROVISIONAL; the wall W32 of decision 0191 that the reference project's first Enter reaches). The A and W functions of a message call differ in the character data: the A form carries
 * ANSI (the system code page) text and characters, the window procedure of a window created with a Unicode class gets UTF-16 (Learn, "Conventions for Function Prototypes" and the pages of WM_SETTEXT, WM_GETTEXT, WM_CHAR ...: the system
 * converts for the ANSI caller). A message that carries no text and no character is the same message in both forms and is sent as SendMessageW sends it. The messages that carry text or a character
 * (WM_SETTEXT 0x0C, WM_GETTEXT 0x0D, WM_GETTEXTLENGTH 0x0E, WM_CREATE 1, WM_NCCREATE 0x81, WM_SETTINGCHANGE 0x1A, WM_DEVMODECHANGE 0x1B, WM_CHAR 0x102, WM_DEADCHAR 0x103, WM_SYSCHAR 0x106, WM_SYSDEADCHAR 0x107, WM_MENUCHAR 0x120,
 * WM_CHARTOITEM 0x2F, WM_IME_CHAR 0x286, WM_MDICREATE 0x220) need the conversion, which is not modelled: they stop by name (S-ref1-sendmessagea-text). The class-specific messages from WM_USER (0x400) up are not classified here:
 * they are sent as in the W form (a class-specific message that carries text is not told apart; the engine's use of one was not seen). */
void fle_imp_SendMessageA(void)
{
    static const char api[] = "SendMessageA";
    uint32_t msg = (uint32_t)R[2];

    switch (msg) {
    case 0x0001: case 0x000C: case 0x000D: case 0x000E: case 0x001A: case 0x001B: case 0x002F: case 0x0081: case 0x0102: case 0x0103: case 0x0106: case 0x0107: case 0x0120: case 0x0220: case 0x0286:
        fle_stop(api, "this message carries text or a character: the conversion between the ANSI form and the Unicode window procedure is not modelled (S-ref1-sendmessagea-text)");
    default:
        break;
    }
    fle_imp_SendMessageW();
}

/* user32!SetClassLongPtrW: ULONG_PTR f(HWND hWnd rcx, int nIndex rdx, LONG_PTR dwNewLong r8). Learn: "the previous value of the specified
 * offset" on success; "If the function fails, the return value is zero. To get extended error information, call GetLastError". An hWnd
 * that is not a live window of this table (NULL included): 0 and last error 1400 (a choice without a source, see invalid_window). A live
 * window of this table: nIndex GCLP_HICON (-14, the low 32 bits of rdx) replaces the hIcon of the window's class in the class table (fle_class.c)
 * and returns the previous one (0 when the class was registered without an icon); every other index stops by name (decision 0119, wall 15). */
void fle_imp_SetClassLongPtrW(void)
{
    static const char api[] = "SetClassLongPtrW";
    uint64_t h = R[1], np = R[8], prev = 0;
    int32_t index = (int32_t)(uint32_t)R[2];
    int64_t i;
    char why[160];

    fle_win_check_thread(api);
    i = live_index(h);
    if (i < 0) {
        invalid_window();
        return;
    }
    if (index != FLE_GCLP_HICON) {
        snprintf(why, sizeof why, "nIndex %d is not GCLP_HICON (-14): the other class values (cursor, brush, menu, style, module, procedure, extra memory) are not modelled (decision 0119)", (int)index);
        fle_stop(api, why);
    }
    fle_class_set_hicon(api, tab[i].atom, np, &prev);
    R[0] = prev;
}

/* The six items of a fresh window menu, in the order of the bits of win.menu_items (decision 0119, wall 16). */
static const uint32_t menu_cmds[6] = { FLE_SC_SIZE, FLE_SC_MOVE, FLE_SC_MINIMIZE, FLE_SC_MAXIMIZE, FLE_SC_CLOSE, FLE_SC_RESTORE };
#define MENU_ALL ((1u << 6) - 1u)

int fle_win_menu_items(uint64_t hwnd)
{
    int64_t i = index_of(hwnd);

    if (i < 0 || !tab[i].has_menu) return -1;
    return (int)tab[i].menu_items;
}

/* The window whose menu has the handle `m`, or -1 (m must be in the menu region, a multiple of 4 above the base, and of a window that has made its menu). */
static int64_t menu_owner(uint64_t m)
{
    uint64_t off;

    if (m <= FLE_MENU_HANDLE_BASE || m >= FLE_MENU_HANDLE_BASE + FLE_MENU_REGION_SIZE) return -1;
    off = m - FLE_MENU_HANDLE_BASE;
    if (off % FLE_WIN_HANDLE_STEP != 0) return -1;
    off = off / FLE_WIN_HANDLE_STEP - 1u;
    if (off >= tab_n || !tab[off].has_menu) return -1;
    return (int64_t)off;
}

/* user32!GetSystemMenu: HMENU f(HWND hWnd rcx, BOOL bRevert edx) (decision 0119, wall 16, PROVISIONAL). Learn: "Enables the application to access the window menu ... for copying
 * and modifying"; bRevert FALSE: "returns a handle to the copy of the window menu currently in use. The copy is initially identical to the window menu, but it can be modified";
 * TRUE: "resets the window menu back to the default state. The previous window menu, if any, is destroyed"; "If the bRevert parameter is FALSE, the return value is a handle to a
 * copy of the window menu. If the bRevert parameter is TRUE, the return value is NULL". Answered: a live window of this table with WS_SYSMENU (the page does not say what a window
 * without it gets: stops). FALSE: the handle of the window's menu (made on the first call, the same afterwards); TRUE: the six items are back and NULL is returned. bRevert is a BOOL
 * (the low 32 bits of rcx's second argument; any nonzero value is TRUE). The last error is not touched. */
void fle_imp_GetSystemMenu(void)
{
    static const char api[] = "GetSystemMenu";
    uint64_t h = R[1];
    uint32_t revert = (uint32_t)R[2];
    int64_t i;

    fle_win_check_thread(api);
    i = need_window(api, h);
    if (!(tab[i].style & FLE_WS_SYSMENU)) fle_stop(api, "the window has no WS_SYSMENU: the page does not say what GetSystemMenu returns for it (decision 0119)");
    if (revert) {
        tab[i].has_menu = 1;
        tab[i].menu_items = MENU_ALL;
        R[0] = 0;
        return;
    }
    if (!tab[i].has_menu) {
        tab[i].has_menu = 1;
        tab[i].menu_items = MENU_ALL;
    }
    R[0] = FLE_MENU_HANDLE_BASE + (uint64_t)FLE_WIN_HANDLE_STEP * ((uint64_t)i + 1u);
}

/* user32!DeleteMenu: BOOL f(HMENU hMenu rcx, UINT uPosition edx, UINT uFlags r8d) (decision 0119, wall 16, PROVISIONAL). Learn: "Deletes an item from the specified menu"; MF_BYCOMMAND
 * (0): "uPosition gives the identifier of the menu item ... the default"; MF_BYPOSITION (0x400): the zero-based position; "If the function succeeds, the return value is nonzero. If
 * the function fails, the return value is zero. To get extended error information, call GetLastError". Answered: the menu of a window of this table (FLE_MENU_HANDLE_BASE + ...), uFlags
 * MF_BYCOMMAND, uPosition one of the six SC_ commands that is still in the menu: the item is removed and TRUE returned. A command that is not in the menu (or not one of the six), a
 * menu handle that is not one of this table, and uFlags other than MF_BYCOMMAND stop (the page gives no failure code). The menu is not drawn and no message is sent. */
void fle_imp_DeleteMenu(void)
{
    static const char api[] = "DeleteMenu";
    uint64_t m = R[1];
    uint32_t pos = (uint32_t)R[2], flags = (uint32_t)R[8];
    int64_t i = menu_owner(m);
    unsigned k;
    char why[200];

    fle_win_check_thread(api);
    if (fle_crash1_menu_owns != NULL && fle_crash1_menu_owns(m)) { R[0] = fle_crash1_menu_delete_item(m, pos, flags); return; }   /* decision 0209 */
    if (i < 0) {
        snprintf(why, sizeof why, "hMenu 0x%" PRIx64 " is not the window menu of a window of this table: the page gives no failure code (decision 0119)", m);
        fle_stop(api, why);
    }
    if (flags != FLE_MF_BYCOMMAND) {
        snprintf(why, sizeof why, "uFlags 0x%x is not MF_BYCOMMAND (0): other forms (MF_BYPOSITION ...) are not modelled (decision 0119)", (unsigned)flags);
        fle_stop(api, why);
    }
    for (k = 0; k < 6; k++) {
        if (menu_cmds[k] == pos) break;
    }
    if (k == 6 || !(tab[i].menu_items & (1u << k))) {
        snprintf(why, sizeof why, "item 0x%x is not in the window menu: the page gives no failure code and no last error (decision 0119)", (unsigned)pos);
        fle_stop(api, why);
    }
    tab[i].menu_items &= ~(1u << k);
    R[0] = 1;
}

/* wtsapi32!WTSRegisterSessionNotification: BOOL f(HWND hWnd rcx, DWORD dwFlags edx) (decision 0119, wall 11, PROVISIONAL). Learn (wtsapi32.h, Windows Vista
 * and later): "Registers the specified window to receive session change notifications"; the return value is TRUE on success, FALSE otherwise ("To get
 * extended error information, call GetLastError", no code given); "If the window handle passed in this function is already registered, the value of the
 * dwFlags parameter is ignored". dwFlags is "one of the following values": NOTIFY_FOR_THIS_SESSION or NOTIFY_FOR_ALL_SESSIONS (the page lists the names, not
 * the numbers: 0 and 1 are those of the SDK header wtsapi32.h, which this work did not read: UNCONFIRMED). Answered: a live window of this table and
 * dwFlags 0 or 1: the window is recorded as registered (the flags of the first call are kept) and TRUE is returned; no WM_WTSSESSION_CHANGE is ever sent (this
 * environment has one session and no session change). A window that is not a live window of this table, or another dwFlags: stops (the page gives no value). */
void fle_imp_WTSRegisterSessionNotification(void)
{
    static const char api[] = "WTSRegisterSessionNotification";
    uint64_t h = R[1];
    uint32_t flags = (uint32_t)R[2];
    int64_t i;

    fle_win_check_thread(api);
    i = need_window(api, h);
    if (flags != FLE_NOTIFY_FOR_THIS_SESSION && flags != FLE_NOTIFY_FOR_ALL_SESSIONS) fle_stop(api, "dwFlags is neither NOTIFY_FOR_THIS_SESSION nor NOTIFY_FOR_ALL_SESSIONS: the page gives no value for another one (decision 0119)");
    if (!tab[i].wts_registered) {
        tab[i].wts_registered = 1;
        tab[i].wts_flags = flags;
    }
    R[0] = 1;
}

/* uxtheme!BufferedPaintInit: HRESULT f(void) (decision 0119, wall 12, PROVISIONAL). Learn (uxtheme.h, Windows Vista and later): "Initialize buffered painting for the
 * current thread"; "If this function succeeds, it returns S_OK. Otherwise, it returns an HRESULT error code"; "An application may call this API multiple times, as
 * long as each call to BufferedPaintInit is balanced with a call to BufferedPaintUnInit"; "Typically, this function is called before creating the main application
 * window". Answered: S_OK (0), the count of calls is kept. The page names no failure condition for this single guest thread, so no error is answered. BeginBufferedPaint,
 * EndBufferedPaint and BufferedPaintUnInit are not answered (drawing; the guest's calls reach the delay-load failure of a name that is not in the response table). */
void fle_imp_BufferedPaintInit(void)
{
    fle_win_check_thread("BufferedPaintInit");
    buffered_paint_init_count++;
    R[0] = 0;
}

uint32_t fle_win_buffered_paint_init_count(void)
{
    return buffered_paint_init_count;
}

/* user32!CreateWindowExW; see fle_win.h for the arguments. Learn: a handle to the new window on success, NULL on failure
 * (GetLastError, no value given: not touched); "sends WM_NCCREATE, WM_NCCALCSIZE, and WM_CREATE messages to the window being
 * created" before it returns (and, Managing Application State, WM_NCCREATE before WM_CREATE); WM_NCCREATE: "return TRUE to
 * continue creation of the window ... FALSE, the CreateWindow or CreateWindowEx function will return a NULL handle"; WM_CREATE:
 * "return zero to continue ... –1, the window is destroyed and ... returns a NULL handle".
 *
 * Decision 0121 (wall 20) adds a third form: a message-only window (hWndParent HWND_MESSAGE, dwExStyle 0, dwStyle 0, the rest as the first form); see the code.
 * Answered: dwExStyle WS_EX_TOOLWINDOW, dwStyle WS_POPUP (the whole DWORD), X = Y = nWidth = nHeight = 0 (the low 32 bits of each
 * stack slot: the guest stores them as DWORDs), hWndParent NULL, hMenu NULL, hInstance the lifted DLL, lpParam NULL, lpWindowName
 * NULL or an empty string, lpClassName a string that fle_class_lookup resolves. A class of the guest's own static names that is not
 * registered yet makes the function fail: NULL, the table untouched. Everything else stops.
 *
 * The window enters the table (handle FLE_WIN_HANDLE_BASE + 4 * (index + 1), live) before the first message, so the window
 * procedure may use it (SetWindowLongPtrW, DefWindowProcW). WM_NCCREATE (wParam 0, lParam the CREATESTRUCTW), WM_NCCALCSIZE (wParam
 * FALSE, lParam a RECT, the proposed window rectangle) and WM_CREATE (wParam 0, lParam the same CREATESTRUCTW) go to the window's
 * procedure through fle_call_guest, one after the other; the procedure is read again before each one (WM_NCCREATE may have replaced
 * it through SetWindowLongPtrW). Results (the whole 64 bits of the LRESULT): WM_NCCREATE 1 continues, 0 fails (NULL); WM_NCCALCSIZE
 * 0; WM_CREATE 0 continues, 0xFFFFFFFFFFFFFFFF fails (NULL). Any other value stops (Learn says "should return"; the pages do not say
 * what another value means). A failed window stays in the table as not live; no WM_DESTROY / WM_NCDESTROY / WM_GETMINMAXINFO /
 * WM_SIZE / WM_MOVE / WM_SHOWWINDOW / WM_PARENTNOTIFY / WM_WINDOWPOS* is sent (decision 0071). */
void fle_imp_CreateWindowExW(void)
{
    static const char api[] = "CreateWindowExW";
    uint32_t ex = (uint32_t)R[1], style = (uint32_t)R[9];
    uint64_t classname = R[2], wname = R[8], sp = R[4];
    uint32_t x = (uint32_t)fle_arg(5), y = (uint32_t)fle_arg(6), cx = (uint32_t)fle_arg(7), cy = (uint32_t)fle_arg(8);
    uint64_t parent = fle_arg(9), menu = fle_arg(10), hinst = fle_arg(11), param = fle_arg(12);
    uint64_t wndproc = 0, h, rect, cs, r;
    uint16_t atom = 0;
    uint64_t idx;
    char why[160];

    uint16_t text[FLE_WIN_TEXT_MAX_UNITS + 1];
    uint16_t text_len = 0;
    int msg_form;
    int64_t cw_parent = -1;
    int cw_is_child = 0;

    fle_win_check_thread(api);
    msg_form = parent == FLE_HWND_MESSAGE;
    /* Decision 0123 (PROVISIONAL): every form of CreateWindowExW is answered by the rules of the Learn pages instead of the three forms of decisions 0071, 0119 and 0121 (which are special cases of them).
     * hWndParent (Learn CreateWindowExW, Window Features "Owned Windows"): NULL; a window handle of this table, which is the PARENT of a WS_CHILD window and the OWNER of any other window (a child window cannot
     * own: "If hwndParent identifies a child window, the system assigns ownership to the top-level parent window of the child window"); HWND_MESSAGE (a message-only window). A WS_CHILD window with no parent
     * is "Cannot create a top-level child window" (System Error Codes 1406 ERROR_TLW_WITH_WSCHILD: the code names the condition; the CreateWindowEx page gives no code: a choice) and fails with NULL. A handle that is
     * not a live window of this table (not NULL, not HWND_MESSAGE) fails with NULL and 1400 (a choice without a source: the page says "an invalid parameter value" is a reason of failure and names no code).
     * CW_USEDEFAULT: valid only for overlapped windows (Learn); for a pop-up or child window x and y (and the size) are set to zero; for an overlapped window the default position and size are a choice (below).
     * Styles and extended styles are kept as given (the page lists the values; an unknown bit is kept). WS_VISIBLE makes the window visible at creation (Learn CreateWindowExW / ShowWindow: "Applications create
     * their main window by calling CreateWindow with the WS_VISIBLE flag set"); WM_SHOWWINDOW is then sent after WM_CREATE (Learn WM_SHOWWINDOW: "If a window has the WS_VISIBLE style when it is created, the window
     * receives this message after it is created, but before it is displayed"). hMenu: a child window's hMenu is its identifier (kept, GWLP_ID); for any other window a non-NULL hMenu stops (menus are not modelled). */
    {
        int64_t par = -1;
        int is_child = (style & FLE_WS_CHILD) != 0;

        if (parent != 0 && !msg_form) {
            par = live_index(parent);
            if (par < 0) {
                fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
                R[0] = 0;
                return;
            }
            if (tab[par].msgonly) fle_stop(api, "hWndParent is a message-only window (a handle to an existing message-only window): not modelled");
        }
        if (is_child && par < 0 && !msg_form) {
            fle_set_last_error(1406u);
            R[0] = 0;
            return;
        }
        if (msg_form) {
            if (ex != 0 || style != 0) {
                snprintf(why, sizeof why, "a message-only window (hWndParent HWND_MESSAGE) with dwExStyle 0x%x and dwStyle 0x%x: only dwExStyle 0 and dwStyle 0 are answered (decision 0121)", (unsigned)ex, (unsigned)style);
                fle_stop(api, why);
            }
        }
        if (menu != 0 && !is_child) fle_stop(api, "hMenu is not NULL for a window that is not a child: menus are not modelled (decision 0071)");
        if (!fle_class_hinst_known(hinst)) fle_stop(api, "hInstance is not the lifted DLL (0x400000): not answered (decision 0071)");
        /* lpParam (Learn CreateWindowExW): "Pointer to a value to be passed to the window through the CREATESTRUCT structure (lpCreateParams member) pointed to by the lParam param of the WM_CREATE message": any value is passed as it is
         * (decision 0123: the value is not looked at; MDI client/child structures are not modelled: the classes of those two are not answered). */
        if (wname != 0) {
            uint16_t i;

            for (i = 0; i <= FLE_WIN_TEXT_MAX_UNITS; i++) {
                uint16_t c = (uint16_t)fle_ld(wname + 2u * i, 2);

                if (c == 0) break;
                if (i == FLE_WIN_TEXT_MAX_UNITS) fle_stop(api, "lpWindowName is longer than 256 units: not answered (decision 0119)");
                text[i] = c;
            }
            text_len = i;
        }
        text[text_len] = 0;
        cw_parent = par;
        cw_is_child = is_child;
    }

    if (!fle_class_lookup(api, classname, hinst, &atom, &wndproc)) {
        R[0] = 0;
        return;
    }
    if (tab_n + 2 >= FLE_WIN_REGION_SIZE / FLE_WIN_HANDLE_STEP) fle_stop(api, "the window handle region is exhausted");
    if (tab_n >= tab_cap) {
        uint64_t nc = tab_cap ? tab_cap * 2 : 16;
        win *n = (win *)realloc(tab, nc * sizeof *tab);

        if (!n) fle_stop(api, "out of memory");
        tab = n;
        tab_cap = nc;
    }
    /* The position and size (Learn CreateWindowExW, X and nWidth): CW_USEDEFAULT is valid only for overlapped windows; for a pop-up or child window the position and the size are zero. For an overlapped window (neither
     * WS_POPUP nor WS_CHILD) the default is a CHOICE WITHOUT A SOURCE: the page says "the system selects the default position" and "a default width that extends from the initial x-coordinate to the right edge of the screen" and
     * "a default height that extends from the initial y-coordinate to the top of the icon area"; the virtual screen is 1920 x 1080 (decision 0053): x = y = 0, width = 1920, height = 1080. */
    {
        int popup = (style & FLE_WS_POPUP) != 0;

        if (x == FLE_CW_USEDEFAULT) {
            if (popup || cw_is_child) {
                x = 0;
                y = 0;
            } else {
                x = 0;
                y = 0;
            }
        }
        if (cx == FLE_CW_USEDEFAULT) {
            if (popup || cw_is_child) {
                cx = 0;
                cy = 0;
            } else {
                cx = (uint32_t)FLE_GDC_HORZRES - x;
                cy = (uint32_t)FLE_GDC_VERTRES - y;
            }
        }
    }
    idx = tab_n++;
    memset(&tab[idx], 0, sizeof tab[idx]);
    tab[idx].lid = next_lid++;
    tab[idx].wid = cw_is_child ? menu : 0;
    tab[idx].parent = cw_is_child ? cw_parent : -1;
    tab[idx].owner_win = (!cw_is_child && cw_parent >= 0) ? root_of(cw_parent) : -1;
    tab[idx].enabled = !(style & FLE_WS_DISABLED);
    tab[idx].px = (int32_t)x;
    tab[idx].py = (int32_t)y;
    tab[idx].pcx = (int32_t)cx;
    tab[idx].pcy = (int32_t)cy;
    tab[idx].rest_x = (int32_t)x;
    tab[idx].rest_y = (int32_t)y;
    tab[idx].rest_cx = (int32_t)cx;
    tab[idx].rest_cy = (int32_t)cy;
    tab[idx].upd_l = tab[idx].upd_t = tab[idx].upd_r = tab[idx].upd_b = 0;
    tab[idx].updr = NULL;
    tab[idx].wndproc = wndproc;
    tab[idx].hinst = hinst;
    tab[idx].style = style;
    tab[idx].exstyle = ex;
    tab[idx].atom = atom;
    tab[idx].live = 1;
    tab[idx].msgonly = msg_form;
    tab[idx].owner = fle_thread_current ? fle_thread_current() : 0u;
    tab[idx].x = (int32_t)x;
    tab[idx].y = (int32_t)y;
    tab[idx].cx = (int32_t)cx;
    tab[idx].cy = (int32_t)cy;
    memcpy(tab[idx].text, text, ((size_t)text_len + 1u) * 2u);
    tab[idx].text_len = text_len;
    h = FLE_WIN_HANDLE_BASE + (uint64_t)FLE_WIN_HANDLE_STEP * (idx + 1u);

    /* The RECT and the CREATESTRUCTW live on the guest stack below the API's frame; the callee is called below them. */
    rect = (sp - 16) & ~UINT64_C(15);
    cs = (rect - FLE_CREATESTRUCTW_SIZE) & ~UINT64_C(15);
    fle_st(cs + FLE_CS_OFF_CREATEPARAMS, 8, param);
    fle_st(cs + FLE_CS_OFF_HINSTANCE, 8, hinst);
    fle_st(cs + FLE_CS_OFF_MENU, 8, menu);
    fle_st(cs + FLE_CS_OFF_PARENT, 8, parent);
    fle_st(cs + FLE_CS_OFF_CY, 4, cy);
    fle_st(cs + FLE_CS_OFF_CX, 4, cx);
    fle_st(cs + FLE_CS_OFF_Y, 4, y);
    fle_st(cs + FLE_CS_OFF_X, 4, x);
    fle_st(cs + FLE_CS_OFF_STYLE, 4, style);
    fle_st(cs + FLE_CS_OFF_STYLE + 4, 4, 0);
    fle_st(cs + FLE_CS_OFF_NAME, 8, wname);
    fle_st(cs + FLE_CS_OFF_CLASS, 8, classname);
    fle_st(cs + FLE_CS_OFF_EXSTYLE, 4, ex);
    fle_st(cs + FLE_CS_OFF_EXSTYLE + 4, 4, 0);
    /* the proposed window rectangle (Learn WM_NCCALCSIZE: "On entry, the structure contains the proposed window rectangle"): (x, y, x + cx, y + cy) */
    fle_st(rect, 4, (uint32_t)(int32_t)x);
    fle_st(rect + 4, 4, (uint32_t)(int32_t)y);
    fle_st(rect + 8, 4, (uint32_t)((int32_t)x + (int32_t)cx));
    fle_st(rect + 12, 4, (uint32_t)((int32_t)y + (int32_t)cy));
    R[4] = cs;

    r = call_wndproc(api, tab[idx].wndproc, h, FLE_WM_NCCREATE, 0, cs);
    if (r == 0) {
        tab[idx].live = 0;
        R[4] = sp;
        R[0] = 0;
        return;
    }
    /* Decision 0123 (S-wmcreate, a choice without a source): the page says TRUE continues and FALSE fails; any nonzero value is TRUE (a BOOL in an LRESULT: the low 32 bits are the BOOL, but the whole value is nonzero: continue). */

    r = call_wndproc(api, tab[idx].wndproc, h, FLE_WM_NCCALCSIZE, 0, rect);
    if (r != 0) {
        snprintf(why, sizeof why, "WM_NCCALCSIZE (wParam FALSE) returned 0x%" PRIx64 ": the page says the application should return zero", r);
        fle_stop(api, why);
    }

    r = call_wndproc(api, tab[idx].wndproc, h, FLE_WM_CREATE, 0, cs);
    if (r == UINT64_MAX) {
        tab[idx].live = 0;
        R[4] = sp;
        R[0] = 0;
        return;
    }
    /* Decision 0123 (S-wmcreate, a choice without a source): the page says zero continues and -1 destroys the window (the CreateWindowEx result is NULL); it gives no meaning to another value. The guest's own handler of the
     * window class TLightweightLayeredControl (f_71e2e0) returns 1. Any value other than -1 continues, like zero. */

    R[4] = sp;
    /* Decision 0219 (txt1; PM decision: ON by default, -DFLE_NO_CREATE_SIZE_MOVE restores the old sequence). After WM_CREATE returned a value other than -1, a WS_CHILD or WS_POPUP window receives WM_SIZE (wParam
     * SIZE_RESTORED, lParam the client width in the low word and the height in the high word) and then WM_MOVE (lParam the client origin: the parent's client coordinates for a child, screen coordinates for a pop-up;
     * the non-client area is 0 here, so both are the stored position). Source: Wine 10.0's user32 measured with harness/txt1/create_seq.c (child: NCCREATE NCCALCSIZE CREATE SIZE(0,cx,cy) MOVE(x,y) [PARENTNOTIFY]
     * [SHOWWINDOW]; pop-up: the same without PARENTNOTIFY; a WS_OVERLAPPEDWINDOW window that is not visible gets neither). The Learn pages of CreateWindowEx, WM_CREATE, WM_SIZE and WM_MOVE do not list this pair
     * ("WM_SIZE: Sent to a window after its size has changed"), and no real Windows was asked: Wine is the only source (S-txt1-create-size). Why the guest needs it: a VCL control (TQuickMemo) lays out its
     * scroll bars on its first WM_SIZE; without it the vertical scroller stays visible at (0, 0, 0 x 0), the control's text area is 0 wide and the text of every TMsgForm is not drawn (docs/decisions/0219-msgform-text.md).
     * Not sent (no source): a message-only window, a window created with WS_MINIMIZE (0x20000000) or WS_MAXIMIZE (0x01000000) (the wParam would be SIZE_MINIMIZED / SIZE_MAXIMIZED), and a window the guest destroyed
     * inside WM_CREATE; a window the guest destroys inside WM_SIZE gets no WM_MOVE. */
    if (tab[idx].live && !msg_form && (style & (FLE_WS_CHILD | FLE_WS_POPUP)) && !(style & (0x20000000u | 0x01000000u))) {
        uint64_t lp = ((uint64_t)(uint16_t)tab[idx].pcy << 16) | (uint16_t)tab[idx].pcx;

        (void)call_wndproc(api, tab[idx].wndproc, h, FLE_WM_SIZE, 0, lp);
        if (tab[idx].live) {
            lp = ((uint64_t)(uint16_t)tab[idx].py << 16) | (uint16_t)tab[idx].px;
            (void)call_wndproc(api, tab[idx].wndproc, h, FLE_WM_MOVE, 0, lp);
        }
        R[4] = sp;
    }
    /* R3 (Learn CreateWindowExW, Remarks): "If the created window is a child window, its default position is at the bottom of the Z-order. If the created window is a top-level window, its default position is at the
     * top of the Z-order (but beneath all topmost windows unless the created window is itself topmost)". A message-only window has no z-order (Learn Window Features). */
    if (!msg_form) {
        if (tab[idx].parent >= 0) {
            z_place((int64_t)idx, 1, -1);
        } else {
            /* the topmost bit given at creation is honoured: a window created WS_EX_TOPMOST is topmost (Learn Extended Window Styles) */
            z_place((int64_t)idx, 0, -1);
        }
    }
    if (style & FLE_WS_VISIBLE) {
        tab[idx].visible = 1;
        tab[idx].ever_shown = 1;
        /* Learn WM_SHOWWINDOW: a window created with WS_VISIBLE "receives this message after it is created, but before it is displayed": wParam TRUE, lParam 0 */
        (void)call_wndproc(api, tab[idx].wndproc, h, FLE_WM_SHOWWINDOW, 1, 0);
        note_visibility_changed();
    }
    R[0] = h;
}

/* user32!SetWindowLongPtrW: LONG_PTR f(HWND hWnd rcx, int nIndex rdx, LONG_PTR dwNewLong r8). Learn: the return value is the
 * previous value of the specified offset; failure returns zero and sets the last error (no value given). About Window Procedures:
 * with GWL_WNDPROC "returns the address of the window's original window procedure". Answered: a live window of this table, nIndex
 * GWLP_WNDPROC (-4, the low 32 bits of rdx), dwNewLong not 0. The new value is kept as a number (it need not be a function entry
 * of the image: the guest's Delphi MakeObjectInstance gives the address of a block it wrote itself, decision 0071); it is called
 * only by a later message, through call_wndproc. The result is the procedure the window had. Decision 0119 (wall 17) adds GWL_EXSTYLE
 * (-20) and GWLP_USERDATA (-21), see below. Every other index (style, id, parent, instance, extra bytes) stops by name. A window that is
 * not a live window of this table (NULL included) answers 0 and the last error 1400 (decision 0119, the PM's decision 1: a CHOICE WITHOUT
 * A SOURCE, see invalid_window; old, decision 0071: the run stopped by name). Otherwise the last error is not touched. */
void fle_imp_SetWindowLongPtrW(void)
{
    static const char api[] = "SetWindowLongPtrW";
    uint64_t h = R[1], np = R[8];
    int32_t index = (int32_t)(uint32_t)R[2];
    int64_t i;
    char why[320];

    fle_win_check_thread(api);
    i = live_index(h);
    if (i < 0) {
        invalid_window();
        return;
    }
    if (index == FLE_GWL_EXSTYLE) {
        /* Decision 0119, wall 17: Learn "GWL_EXSTYLE -20 Sets a new extended window style", returns the previous value. The value is kept as given (the low 32 bits: an
         * extended style is a DWORD). "Certain window data is cached, so changes you make using SetWindowLongPtr will not take effect until you call the SetWindowPos function":
         * this table has no cache, so the new value is the one GetWindowLongPtrW reports at once (a choice: nothing in this runtime reads the cached copy).
         * Decision 0123 (R4): WS_EX_TOPMOST is not changed by this call (Learn Extended Window Styles: "To add or remove this style, use the SetWindowPos function"): the bit keeps its value. */
        R[0] = tab[i].exstyle;
        tab[i].exstyle = ((uint32_t)np & ~FLE_WS_EX_TOPMOST) | (tab[i].exstyle & FLE_WS_EX_TOPMOST);
        if (fle_layered_exstyle_changed) fle_layered_exstyle_changed(h, (uint32_t)R[0], tab[i].exstyle);   /* decision 0140: the layering bit set again is a new layered window */
        return;
    }
    if (index == FLE_GWL_STYLE) {
        /* Decision 0123: Learn "GWL_STYLE -16 Sets a new window style", returns the previous value (the page: the return value is the previous value of the specified offset). The value is kept as given in the low 32 bits.
         * WS_VISIBLE is part of the style and a window is visible by it (Learn IsWindowVisible), so the model follows the bit: setting it makes the window visible without ShowWindow and without WM_SHOWWINDOW (a choice without a
         * source: the page says the data is cached until SetWindowPos; the cache is not modelled: the new value is seen at once). WS_CHILD and WS_POPUP are kept as given: changing them does not change the parent or owner
         * (Learn SetParent: "SetParent does not modify the WS_CHILD or WS_POPUP window styles"; the same independence is used the other way). A style change that makes WS_DISABLED differ is the state of the window (EnableWindow
         * is the call that notifies: here the state follows the bit without WM_ENABLE). */
        uint32_t ns = (uint32_t)np;

        R[0] = (uint64_t)(int64_t)(int32_t)tab[i].style;
        tab[i].style = ns;
        tab[i].visible = (ns & FLE_WS_VISIBLE) != 0;
        tab[i].enabled = !(ns & FLE_WS_DISABLED);
        if (tab[i].visible) note_visibility_changed();
        return;
    }
    if (index == FLE_GWLP_ID) {
        /* Decision 0123: Learn "GWLP_ID -12 Sets a new identifier of the child window. The window cannot be a top-level window." A top-level window: 0 and ERROR_INVALID_PARAMETER (87, a choice: the page names no code). */
        if (tab[i].parent < 0) {
            fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
            R[0] = 0;
            return;
        }
        R[0] = tab[i].wid;
        tab[i].wid = np;
        return;
    }
    if (index == FLE_GWLP_HINSTANCE) {
        R[0] = tab[i].hinst;
        tab[i].hinst = np;
        return;
    }
    if (index == FLE_GWLP_HWNDPARENT) {
        /* Decision 0123 (R1): Learn "GWLP_HWNDPARENT -8 Sets a new owner for a top-level window"; "GWLP_HWNDPARENT is used to change the owner of a top-level window, not the parent of a child window"; "A window can have either a
         * parent or an owner, or neither, but never both simultaneously". A top-level window: the owner field is set (0 = no owner; a handle of this table = that window's root); the return value is the previous owner (0 when none).
         * A child window: the page says not to (use SetParent): stops. */
        int64_t o = -1;

        if (tab[i].parent >= 0) fle_stop(api, "GWLP_HWNDPARENT of a child window: the page says to use SetParent and gives no behaviour");
        if (np != 0) {
            o = live_index(np);
            if (o < 0) {
                fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
                R[0] = 0;
                return;
            }
            o = root_of(o);
            if (o == i) fle_stop(api, "a window as its own owner: the page does not say");
        }
        R[0] = tab[i].owner_win >= 0 ? handle_of(tab[i].owner_win) : 0;
        tab[i].owner_win = o;
        return;
    }
    if (index == FLE_GWLP_USERDATA) {
        /* Decision 0119, wall 17: Learn "GWLP_USERDATA -21 Sets the user data associated with the window ... Its value is initially zero"; returns the previous value. */
        R[0] = tab[i].userdata;
        tab[i].userdata = np;
        return;
    }
    if (index != FLE_GWLP_WNDPROC) {
        snprintf(why, sizeof why, "nIndex %d is not GWLP_WNDPROC (-4), GWL_EXSTYLE (-20), GWL_STYLE (-16), GWLP_ID (-12), GWLP_HINSTANCE (-6), GWLP_HWNDPARENT (-8) or GWLP_USERDATA (-21): extra window memory is not modelled (decisions 0071, 0119, 0123)", (int)index);
        fle_stop(api, why);
    }
    if (np == 0) fle_stop(api, "dwNewLong is 0 for GWLP_WNDPROC: the page does not say what a NULL window procedure means (decision 0071)");
    R[0] = tab[i].wndproc;
    tab[i].wndproc = np;
}

/* user32!GetWindowLongPtrW: LONG_PTR f(HWND hWnd rcx, int nIndex rdx) (decision 0119, wall 17, PROVISIONAL). Learn: "If the function succeeds, the return value is the requested value. If the
 * function fails, the return value is zero. To get extended error information, call GetLastError"; GWL_EXSTYLE -20 "Retrieves the extended window styles", GWL_STYLE -16 "the window
 * styles", GWLP_HINSTANCE -6 "a handle to the application instance", GWLP_HWNDPARENT -8 "a handle to the parent window, if there is one", GWLP_USERDATA -21 "Its value is initially zero",
 * GWLP_WNDPROC -4 "the pointer to the window procedure, or a handle representing the pointer". Answered: a live window of this table and one of these six indexes or GWLP_ID -12 (decision 0123: the stored values;
 * GWLP_HWNDPARENT is the parent or else the owner). A window that is not a live window of this table (NULL included): 0 and last error 1400 (a choice without a source, see invalid_window). Any other index
 * (extra window memory, DWLP_*) stops: the page gives the extra memory of a class that this table does not keep. */
void fle_imp_GetWindowLongPtrW(void)
{
    static const char api[] = "GetWindowLongPtrW";
    uint64_t h = R[1];
    int32_t index = (int32_t)(uint32_t)R[2];
    int64_t i;
    char why[200];

    fle_win_check_thread(api);
    i = live_index(h);
    if (i < 0) {
        invalid_window();
        return;
    }
    switch (index) {
    case FLE_GWL_EXSTYLE:    R[0] = tab[i].exstyle; return;
    case FLE_GWL_STYLE:      R[0] = (uint64_t)(int64_t)(int32_t)tab[i].style; return;
    case FLE_GWLP_HINSTANCE: R[0] = tab[i].hinst; return;
    case FLE_GWLP_HWNDPARENT:
        if (tab[i].msgonly) fle_stop(api, "GWLP_HWNDPARENT of a message-only window: the pages do not say what the parent of a message-only window is (decision 0121)");
        /* Decision 0123 (R1): "a handle to the parent window, if there is one" (GetWindowLongPtrW page): the parent of a child window, else the owner of a top-level window, else 0. */
        R[0] = tab[i].parent >= 0 ? handle_of(tab[i].parent) : (tab[i].owner_win >= 0 ? handle_of(tab[i].owner_win) : 0);
        return;
    case FLE_GWLP_ID:
        R[0] = tab[i].wid;
        return;
    case FLE_GWLP_USERDATA:  R[0] = tab[i].userdata; return;
    case FLE_GWLP_WNDPROC:   R[0] = tab[i].wndproc; return;
    default:
        snprintf(why, sizeof why, "nIndex %d is not one of GWL_EXSTYLE, GWL_STYLE, GWLP_ID, GWLP_HINSTANCE, GWLP_HWNDPARENT, GWLP_USERDATA, GWLP_WNDPROC: extra memory and DWLP_* are not modelled (decisions 0119, 0123)", (int)index);
        fle_stop(api, why);
    }
}

/* user32!SetWindowTextW: BOOL f(HWND hWnd rcx, LPCWSTR lpString rdx) (decision 0119, wall 17, PROVISIONAL). Learn: "Changes the text of the specified window's title bar (if it has one)";
 * "If the target window is owned by the current process, SetWindowText causes a WM_SETTEXT message to be sent to the specified window or control" (the whole call is that message: Learn
 * WM_SETTEXT: "The DefWindowProc function sets and displays the window text", "The return value is TRUE if the text is set"); success is nonzero, failure zero ("To get extended error
 * information, call GetLastError"). Answered: a live window of this table: WM_SETTEXT (wParam 0, lParam lpString) goes to the window's procedure through call_wndproc (a window procedure
 * that passes it to DefWindowProcW makes the text the window's text, see there) and its result is the return value of SetWindowTextW (a choice: the page says the function returns
 * "nonzero" on success and the message returns TRUE if the text is set). lpString NULL: the page marks the parameter [in, optional] and gives no meaning to NULL: it is passed on as NULL and
 * DefWindowProcW makes the text empty (a choice without a source). A window that is not a live window of this table: 0 and last error 1400 (a choice without a source). */
void fle_imp_SetWindowTextW(void)
{
    static const char api[] = "SetWindowTextW";
    uint64_t h = R[1], lp = R[2];
    int64_t i;

    fle_win_check_thread(api);
    i = live_index(h);
    if (i < 0) {
        invalid_window();
        return;
    }
    callwndproc_hooks(i, h, FLE_WM_SETTEXT, 0, lp);
    R[0] = call_wndproc(api, tab[i].wndproc, h, FLE_WM_SETTEXT, 0, lp);
}

/* user32!DefWindowProcW: LRESULT f(HWND hWnd rcx, UINT Msg rdx, WPARAM wParam r8, LPARAM lParam r9). Learn: "Calls the default
 * window procedure to provide default processing for any window messages that an application does not process"; the return value
 * "is the result of the message processing and depends on the message". The pages give no result for any message (WM_NCCREATE:
 * an application that processes it "should return TRUE to continue"; WM_CREATE: "should return zero to continue"; WM_NCCALCSIZE:
 * "should return zero" when wParam is FALSE): the results below are PROVISIONAL (decision 0071), chosen so that the default
 * processing never stops the creation of a window.
 * Answered: a live window of this table and one of the three messages CreateWindowExW sends, with a non-NULL lParam:
 *   WM_NCCREATE (0x81) -> 1; WM_CREATE (0x01) -> 0; WM_NCCALCSIZE (0x83) with wParam FALSE -> 0 and the RECT left as it is (a pop-up
 *   window of this table has no frame, so its client rectangle is its window rectangle; the one form the table holds).
 * Any other message, WM_NCCALCSIZE with wParam TRUE (NCCALCSIZE_PARAMS), a NULL lParam, or a window that is not of this table stops
 * by name. wParam of WM_NCCREATE and WM_CREATE is not used (Learn) and is not looked at. The last error is not touched. */
static void get_min_max_info(const char *api, int64_t i, int32_t out[8]);
static uint64_t def_erase_background(const char *api, int64_t wi, uint64_t dc);
static void erase_now_one(const char *api, int64_t i);

void fle_imp_DefWindowProcW(void)
{
    static const char api[] = "DefWindowProcW";
    uint64_t h = R[1], wp = R[8], lp = R[9];
    uint32_t msg = (uint32_t)R[2];
    char why[240];

    int64_t wi;

    fle_win_check_thread(api);
    wi = need_window(api, h);
    switch (msg) {
    case FLE_WM_NCCREATE:
        if (lp == 0) fle_stop(api, "WM_NCCREATE with a NULL lParam: the page says lParam points to the CREATESTRUCT");
        R[0] = 1;
        return;
    case FLE_WM_CREATE:
        if (lp == 0) fle_stop(api, "WM_CREATE with a NULL lParam: the page says lParam points to the CREATESTRUCT");
        R[0] = 0;
        return;
    case FLE_WM_NCCALCSIZE:
        if (wp != 0) fle_stop(api, "WM_NCCALCSIZE with wParam not FALSE (NCCALCSIZE_PARAMS): not answered (decision 0071)");
        if (lp == 0) fle_stop(api, "WM_NCCALCSIZE with a NULL lParam: the page says lParam points to a RECT");
        R[0] = 0;
        return;
    case FLE_WM_SETTEXT: {
        /* Decision 0119, wall 17 (PROVISIONAL). Learn (WM_SETTEXT): "lParam: A pointer to a null-terminated string that is the window text"; "The DefWindowProc function sets and displays the
         * window text"; "The return value is TRUE if the text is set". The text is kept (at most FLE_WIN_TEXT_MAX_UNITS units, longer stops; a NULL lParam is the empty string: a choice
         * without a source). Nothing is displayed. The result is TRUE (1). */
        uint16_t n = 0;

        if (lp != 0) {
            for (n = 0; n <= FLE_WIN_TEXT_MAX_UNITS; n++) {
                uint16_t c = (uint16_t)fle_ld(lp + 2u * n, 2);

                if (c == 0) break;
                if (n == FLE_WIN_TEXT_MAX_UNITS) fle_stop(api, "WM_SETTEXT with a string longer than 256 units: not answered (decision 0119)");
                tab[wi].text[n] = c;
            }
        }
        tab[wi].text[n] = 0;
        tab[wi].text_len = n;
        R[0] = 1;
        return;
    }
    case FLE_WM_SETICON: {
        /* Decision 0119, wall 14 (PROVISIONAL). Learn (WM_SETICON): wParam ICON_BIG (1) or ICON_SMALL (0), lParam "A handle to the new large or small icon. If this parameter is
         * NULL, the icon indicated by wParam is removed"; "The return value is a handle to the previous large or small icon, depending on the value of wParam. It is NULL if the
         * window previously had no icon of the type indicated by wParam"; "The DefWindowProc function returns a handle to the previous large or small icon associated with the
         * window". Any other wParam is not defined by the page: stops. The handle is kept as a number (the page does not say what a handle that is not an icon does). */
        uint64_t *slot, prev;

        if (wp != FLE_ICON_BIG && wp != FLE_ICON_SMALL) {
            snprintf(why, sizeof why, "WM_SETICON with wParam 0x%" PRIx64 ": the page defines ICON_BIG (1) and ICON_SMALL (0) only (decision 0119)", wp);
            fle_stop(api, why);
        }
        slot = wp == FLE_ICON_BIG ? &tab[wi].icon_big : &tab[wi].icon_small;
        prev = *slot;
        *slot = lp;
        R[0] = prev;
        return;
    }
    /* Decision 0123 (PROVISIONAL): the default processing of the messages the window model sends. Each answer is the one of the page. */
    case FLE_WM_WINDOWPOSCHANGING:
        /* Learn WM_WINDOWPOSCHANGING: "For a window with the WS_OVERLAPPED or WS_THICKFRAME style, the DefWindowProc function sends the WM_GETMINMAXINFO message to the window. This is done to validate the new size and
         * position of the window and to enforce the CS_BYTEALIGNCLIENT and CS_BYTEALIGNWINDOW client styles." WS_OVERLAPPED is 0: a window that is neither a pop-up (WS_POPUP) nor a child (WS_CHILD) is an overlapped window (Learn
         * Window Styles). Decision 0258 (alpha7) sends the message (the defaults of get_min_max_info with the window's own answer); the page says nothing about clipping the size to the tracking sizes and nothing is clipped
         * (Wine 10.0 clips cx, cy to them: UNOFFICIAL, not done). The result is 0. S-defproc-poschanging. */
        if (lp == 0) fle_stop(api, "WM_WINDOWPOSCHANGING with a NULL lParam: the page says lParam points to a WINDOWPOS");
        if ((tab[wi].style & 0x00040000u) || !(tab[wi].style & (FLE_WS_POPUP | FLE_WS_CHILD))) {
            int32_t mm[8];

            get_min_max_info(api, wi, mm);
        }
        R[0] = 0;
        return;
    case FLE_WM_WINDOWPOSCHANGED:
        /* Learn WM_WINDOWPOSCHANGED: "By default, the DefWindowProc function sends the WM_SIZE and WM_MOVE messages to the window." */
        if (lp == 0) fle_stop(api, "WM_WINDOWPOSCHANGED with a NULL lParam: the page says lParam points to a WINDOWPOS");
        defproc_windowposchanged(api, wi, lp);
        R[0] = 0;
        return;
    case FLE_WM_SIZE:
    case FLE_WM_MOVE:
    case FLE_WM_SHOWWINDOW:
    case FLE_WM_ENABLE:
    case FLE_WM_SETFOCUS:
    case FLE_WM_KILLFOCUS:
    case FLE_WM_CAPTURECHANGED:
    case FLE_WM_NCACTIVATE:
        /* "If an application processes this message, it should return zero" for these: the default processing has no further effect in a model that draws nothing; WM_NCACTIVATE: DefWindowProc draws the title bar (nothing
         * is drawn): the answer for wParam FALSE is TRUE ("should return TRUE to indicate that the system should proceed with the default processing"); for wParam TRUE the value is ignored. WM_SHOWWINDOW: "The
         * DefWindowProc function hides or shows the window, as specified by the message": the visibility is already the state. */
        R[0] = msg == FLE_WM_NCACTIVATE ? 1u : 0u;
        return;
    case FLE_WM_CANCELMODE:
        /* "When the WM_CANCELMODE message is sent, the DefWindowProc function cancels internal processing of standard scroll bar input, cancels internal menu processing, and releases the mouse capture." */
        input_window_cancelmode(wi);
        R[0] = 0;
        return;
    case FLE_WM_ACTIVATE:
        /* Learn WM_ACTIVATE: "If the window is being activated and is not minimized, the DefWindowProc function sets the keyboard focus to the window." Only the low word of wParam says active (non-zero) or inactive. */
        if ((wp & 0xFFFFu) != 0 && !((wp >> 16) & 0xFFFFu) && !tab[wi].iconic) input_set_focus_default(api, wi);
        R[0] = 0;
        return;
    case FLE_WM_DESTROY:
    case FLE_WM_NCDESTROY:
        /* Learn WM_DESTROY / WM_NCDESTROY: "If an application processes this message, it should return zero". DefWindowProc has no further effect in a model that draws nothing (the page of WM_NCDESTROY: "This message frees any
         * memory internally allocated for the window": the model's own cleanup is done by DestroyWindow after the message). The result is 0. */
        R[0] = 0;
        return;
    case 0x000Du: { /* WM_GETTEXT */
        /* Learn WM_GETTEXT: wParam the maximum number of characters including the terminating NUL; lParam the buffer; "The return value is the number of characters copied, not including the terminating null character"; "The
         * DefWindowProc function copies the text associated with the window into the specified buffer and returns the number of characters copied"; "For other windows, the text is the window title". The text kept for the window
         * (WM_SETTEXT, lpWindowName at creation). wParam 0: nothing is copied, 0 is returned (a choice: the page gives no value for it). A NULL buffer with wParam > 0 stops. */
        uint64_t max = wp & 0xFFFFFFFFu, k, n;

        if (max == 0) {
            R[0] = 0;
            return;
        }
        if (lp == 0) fle_stop(api, "WM_GETTEXT with a NULL buffer: the page gives no behaviour");
        n = tab[wi].text_len;
        if (n > max - 1u) n = max - 1u;
        for (k = 0; k < n; k++) fle_st(lp + 2u * k, 2, tab[wi].text[k]);
        fle_st(lp + 2u * n, 2, 0);
        R[0] = n;
        return;
    }
    case 0x000Eu: /* WM_GETTEXTLENGTH */
        /* Learn WM_GETTEXTLENGTH: "The return value is the length of the text in characters, not including the terminating null character"; "For other windows, the text is the window title". The exact length (the page allows a larger
         * value for ANSI/DBCS mixtures: not applicable to a Unicode window). */
        R[0] = tab[wi].text_len;
        return;
    case 0x0030u: /* WM_SETFONT */
        /* Learn WM_SETFONT: "Sets the font that a control is to use when drawing text"; wParam the HFONT ("If this parameter is NULL, the control uses the default system font"); lParam low word: redraw now; "This message does not
         * return a value". The page describes the message of CONTROLS: it does not say what DefWindowProc does with it for a window that is not a control: the font handle is kept as a number and nothing else happens (a choice
         * without a source, S-setfont; a plain window class has no font to draw with). The result is 0. */
        tab[wi].hfont = wp;
        R[0] = 0;
        return;
    case 0x0031u: /* WM_GETFONT */
        /* Learn WM_GETFONT: "a handle to the font used by the control, or NULL if the control is using the system font": the kept handle (0 = the system font). */
        R[0] = tab[wi].hfont;
        return;
    case 0x000Bu: /* WM_SETREDRAW */
        /* Decision 0126 (PROVISIONAL), wall 65. Learn WM_SETREDRAW: "You send the WM_SETREDRAW message to a window to allow changes in that window to be redrawn, or to prevent changes in that window from being redrawn"; wParam
         * "The redraw state. If this parameter is TRUE, then the content can be redrawn after a change. If this parameter is FALSE, then the content can't be redrawn after a change"; lParam "isn't used"; "Your application should
         * return 0 if it processes this message"; the page's recipe: "call this message with wParam set to FALSE, add the items, and then call the message again with wParam set to TRUE. Finally, your application can call
         * RedrawWindow(hWnd, NULL, NULL, RDW_ERASE | RDW_FRAME | RDW_INVALIDATE | RDW_ALLCHILDREN) to cause the list box to be repainted". DefWindowProc keeps the state (S-setredraw-default: the page describes the message
         * for a window that handles it; the default procedure keeps the flag, a reading) and returns 0. While it is FALSE the window and its children are drawn on with an empty visible region (S-setredraw-region:
         * the drawing calls discard their output like those at a locked window; the page says "can't be redrawn"). */
        tab[wi].no_redraw = wp != 0 ? 0 : 1;
        R[0] = 0;
        return;
    case FLE_WM_ERASEBKGND:
        /* Decision 0323. Learn WM_ERASEBKGND: "The DefWindowProc function erases the background by using the class background brush specified by the hbrBackground member of the WNDCLASS structure. If hbrBackground is NULL,
         * the application should process the WM_ERASEBKGND message and erase the background." and "An application should return nonzero ... if it erases the background; otherwise, it should return zero". It filled nothing
         * and answered 1 before (decision 0123, a choice without a source). Now: the class brush fills the clip box of the DC (wParam) and the answer is 1; a class without a brush answers 0 (Wine 10.0, win32u/defwnd.c
         * default_window_proc, UNOFFICIAL: the same two answers), so BeginPaint reports fErase = TRUE for it. */
        R[0] = def_erase_background(api, wi, wp);
        return;
    case FLE_WM_NCPAINT:
    case FLE_WM_PAINT:
        /* Learn WM_PAINT: "The DefWindowProc function validates the update region. The function may also send the WM_NCPAINT message to the window procedure if the window frame must be painted and send the
         * WM_ERASEBKGND message if the window background must be erased." Decision 0323: the background is erased first when the update region is marked for it (BeginPaint's WM_ERASEBKGND, as Wine 10.0's
         * default_window_proc does with BeginPaint / EndPaint: UNOFFICIAL); then the update region is validated. Nothing else is drawn. */
        if (msg == FLE_WM_PAINT) {
            if (tab[wi].upd_erase && tab[wi].upd_r > tab[wi].upd_l && tab[wi].upd_b > tab[wi].upd_t) erase_now_one(api, wi);
            upd_clear_rect(wi);
            tab[wi].upd_erase = 0;
        }
        R[0] = 0;
        return;
    case FLE_WM_CLOSE:
        /* Learn WM_CLOSE: "By default, the DefWindowProc function calls the DestroyWindow function to destroy the window." */
        {
            uint64_t sv = R[4];

            R[1] = handle_of(wi);
            fle_imp_DestroyWindow();
            R[4] = sv;
            R[0] = 0;
        }
        return;
    case FLE_WM_GETMINMAXINFO:
        /* Learn WM_GETMINMAXINFO: lParam "A pointer to a MINMAXINFO structure that contains the default maximized position and dimensions, and the default minimum and maximum tracking sizes. An application can override the
         * defaults by setting the members of this structure"; "If an application processes this message, it should return zero". The default processing leaves the defaults in the structure (they are what the message
         * carried: get_min_max_info) and returns 0. Decision 0258 (alpha7); until then this stopped. S-defproc-getminmaxinfo. */
        if (lp == 0) fle_stop(api, "WM_GETMINMAXINFO with a NULL lParam: the page says lParam points to a MINMAXINFO");
        R[0] = 0;
        return;
    default:
        /* Decision 0136 (PROVISIONAL): the mouse and key messages, WM_NCHITTEST, WM_SETCURSOR, WM_MOUSEACTIVATE, WM_SYSCOMMAND: fle_input.c */
        if (fle_input_defproc) {
            uint64_t res = 0;

            if (fle_input_defproc(api, h, msg, wp, lp, &res)) {
                R[0] = res;
                return;
            }
        }
        /* Decision 0123 (S-privmsg, a choice without a source): a message number from WM_USER (0x0400) up (Learn WM_APP, About Messages: 0x0400..0x7FFF for private window classes, 0x8000..0xBFFF for applications, 0xC000..0xFFFF the
         * string messages of RegisterWindowMessage) is a message that belongs to an application: the system's default processing has nothing to do for it. The page gives no result of DefWindowProc for such a message: 0 is used. A message below
         * WM_USER that is not one of those handled above is a system message whose default processing the model does not know: it stops. */
        if (msg >= 0x0400u) {
            R[0] = 0;
            return;
        }
        /* Decision 0171 (PROVISIONAL), wall ia1-W10 (S-ia1-defproc-1; Tab, the arrow keys and Ctrl+Z/C/V/X reach DefWindowProcW with these messages): WM_CHANGEUISTATE 0x127 and WM_UPDATEUISTATE 0x128 (Learn: "If an application
         * processes this message, it should return zero"; the UI state is the focus/accelerator cue flags, which only change how controls are drawn: nothing is drawn or kept here, 0 is the result) and EM_CANUNDO 0xC6
         * (an edit control message: Learn EM_CANUNDO "If the edit control can undo, the return value is nonzero"; a window that is not an edit control has no default processing for it, 0 is the result). Other messages below
         * WM_USER still stop. */
        if (msg == 0x0127u || msg == 0x0128u || msg == 0x00C6u) {
            R[0] = 0;
            return;
        }
        /* decision 0222: a message whose Learn page names no default processing of DefWindowProc (the page does not mention DefWindowProc, or only tells the application to pass the message on) is answered 0; the table is
         * generated from the pages (fle_defproc_zero.h, tools/mk-defproc-table.py). WM_CHILDACTIVATE (0x22) and WM_PARENTNOTIFY (0x210) belong to it: the window manager sends them. The messages whose page describes a default
         * and that are not implemented stay a named stop. */
        if (msg == 0x0022u || msg == 0x0210u) {
            R[0] = 0;
            return;
        }
        if (msg < 0x0400u) {
            unsigned lo = 0, hi = fle_defproc_zero_n;

            while (lo < hi) {
                unsigned mid = (lo + hi) / 2;

                if (fle_defproc_zero[mid] == msg) {
                    R[0] = 0;
                    return;
                }
                if (fle_defproc_zero[mid] < msg) lo = mid + 1;
                else hi = mid;
            }
        }
        /* Decision 0212 (PROVISIONAL, S-displaychange-def: a choice without a source), WM_DISPLAYCHANGE 0x7E and WM_SETTINGCHANGE 0x1A (fle_input.c sends them when the page's window is resized): Learn says that an application that processes either
         * "should return zero"; the pages give no default processing, so the default is 0 and the message is not passed on to child windows. */
        if (msg == 0x007Eu || msg == 0x001Au) {
            R[0] = 0;
            return;
        }
        snprintf(why, sizeof why, "message 0x%x: the default processing is not answered (decision 0071: WM_NCCREATE, WM_NCCALCSIZE with wParam FALSE, WM_CREATE; decision 0119: WM_SETICON, WM_SETTEXT; decision 0123: the window messages)", (unsigned)msg);
        fle_stop(api, why);
    }
}

/* ---- Decision 0122 (PROVISIONAL): the message queues. One FIFO per guest thread (fle_thread.c); a window's posted messages go to the queue of its owner thread. ---- */

/* The messages that other threads sent (SendMessageW across threads) to the running thread's windows: each is answered by the window's procedure in the running thread's context, the result is
 * handed back, and the sender is woken (Learn SendMessage: "processed only when the receiving thread executes message retrieval code"). */
void fle_win_process_sent(const char *api)
{
    unsigned me = fle_thread_current ? fle_thread_current() : 0u;
    fle_sent *sm;

    if (!fle_thread_dequeue_sent) return;
    while ((sm = fle_thread_dequeue_sent(me)) != NULL) {
        int64_t i = live_index(sm->hwnd);

        if (i >= 0) callwndproc_hooks(i, sm->hwnd, sm->msg, sm->wp, sm->lp);
        sm->result = i >= 0 ? call_wndproc(api, tab[i].wndproc, sm->hwnd, sm->msg, sm->wp, sm->lp) : 0;
        sm->done = 1;
        fle_thread_wake_check();
    }
}

static void need_scheduler(const char *api)
{
    if (!fle_thread_queue || !fle_thread_post) fle_stop(api, "no thread scheduler is linked (fle_thread.c): the message queue is not available");
}

static void fill_msg(uint64_t pmsg, const fle_tmsg *m)
{
    fle_st(pmsg + FLE_MSG_OFF_HWND, 8, m->hwnd);
    fle_st(pmsg + FLE_MSG_OFF_MESSAGE, 4, m->msg);
    fle_st(pmsg + FLE_MSG_OFF_MESSAGE + 4, 4, 0);
    fle_st(pmsg + FLE_MSG_OFF_WPARAM, 8, m->wp);
    fle_st(pmsg + FLE_MSG_OFF_LPARAM, 8, m->lp);
    fle_st(pmsg + FLE_MSG_OFF_TIME, 4, m->time);
    fle_st(pmsg + FLE_MSG_OFF_PTX, 4, (uint32_t)m->ptx);
    fle_st(pmsg + FLE_MSG_OFF_PTY, 4, (uint32_t)m->pty);
    fle_st(pmsg + FLE_MSG_OFF_PRIVATE, 4, 0);
    if (fle_input_note_message) fle_input_note_message(fle_thread_current ? fle_thread_current() : 0u, m->time, m->ptx, m->pty);        /* decision 0136: GetMessageTime / GetMessagePos */
}

/* user32!PostMessageW: BOOL f(HWND hWnd rcx, UINT Msg edx, WPARAM wParam r8, LPARAM lParam r9). Learn (PostMessageW): hWnd NULL "behaves like a call to PostThreadMessage with the dwThreadId parameter set
 * to the identifier of the current thread"; HWND_BROADCAST (0xffff) posts to all top-level windows (not modelled: stops); the message is put "in the message queue associated with the thread that created
 * the specified window" (the Remarks example) and the call returns without waiting; "There is a limit of 10,000 posted messages per message queue ... GetLastError returns ERROR_NOT_ENOUGH_QUOTA (1816)
 * when the limit is hit"; "Do not post the WM_QUIT message using PostMessage; use the PostQuitMessage function" (the page does not say what happens: stops). An hWnd that is not a live window of this
 * table (and not NULL): 0 and the last error 1400 (a choice without a source, as invalid_window). The return value is nonzero on success. A thread message (hWnd NULL) stays in the queue of the calling
 * thread with hwnd NULL (Learn MSG: "NULL when the message is a thread message"). */
void fle_imp_PostMessageW(void)
{
    static const char api[] = "PostMessageW";
    uint64_t h = R[1], wp = R[8], lp = R[9];
    uint32_t msg = (uint32_t)R[2];
    unsigned to;
    int64_t i;

    fle_win_check_thread(api);
    need_scheduler(api);
    if (h == FLE_HWND_BROADCAST) fle_stop(api, "hWnd is HWND_BROADCAST (0xffff): posting to all top-level windows is not modelled (decision 0122)");
    if (msg == FLE_WM_QUIT_ID) fle_stop(api, "WM_QUIT through PostMessageW: the page says to use PostQuitMessage and does not say what the call does (decision 0122)");
    if (h == 0) {
        to = fle_thread_current();
    } else {
        i = live_index(h);
        if (i < 0) {
            invalid_window();
            return;
        }
        to = tab[i].owner;
    }
    if (!fle_thread_post(to, h, msg, wp, lp)) {
        fle_set_last_error(FLE_ERROR_NOT_ENOUGH_QUOTA_W);
        R[0] = 0;
        return;
    }
    R[0] = 1;
}

/* The window filter of GetMessageW and PeekMessageW (Learn: "The window must belong to the current thread"; NULL: any window of the thread and the thread messages; -1: only the thread messages).
 * Returns 1 when the filter is usable. */
static int filter_ok(uint64_t fh)
{
    int64_t i;

    if (fh == 0 || fh == UINT64_MAX) return 1;
    i = live_index(fh);
    return i >= 0 && tab[i].owner == fle_thread_current();
}

/* user32!GetMessageW: BOOL f(LPMSG lpMsg rcx, HWND hWnd rdx, UINT wMsgFilterMin r8d, UINT wMsgFilterMax r9d). Learn: the pending sent messages are delivered first ("the system delivers pending, nonqueued
 * messages"), then the first posted message that matches the filter is removed and returned (nonzero); WM_QUIT makes it return zero ("always retrieves WM_QUIT messages, no matter which values you
 * specify"); an hWnd that is not a window of the calling thread makes it return -1 ("the function fails if hWnd is an invalid window handle ... To get extended error information, call GetLastError": the
 * code 1400 is a choice without a source). With nothing to return it waits (the thread blocks, the others run). WM_QUIT is delivered only when no other posted message matches (Learn "About Messages and
 * Message Queues": WM_QUIT "kept in the queue and forwarded ... only when the queue contains no other messages"). The retrieval marks the messages old (Learn GetQueueStatus). */
void fle_imp_GetMessageW(void)
{
    static const char api[] = "GetMessageW";
    uint64_t pmsg = R[1], fh = R[2];
    uint32_t fmin = (uint32_t)R[8], fmax = (uint32_t)R[9];
    fle_tqueue *q;

    fle_win_check_thread(api);
    need_scheduler(api);
    if (pmsg == 0) fle_stop(api, "lpMsg is NULL: the page gives no behaviour");
    q = fle_thread_queue(fle_thread_current(), 1);
    if (!filter_ok(fh)) {
        invalid_window();
        R[0] = 0xFFFFFFFFu;
        return;
    }
    for (;;) {
        int pos;

        fle_win_process_sent(api);
        pos = fle_thread_queue_find(q, fh, fmin, fmax);
        if (pos >= 0) {
            fle_tmsg m = fle_thread_queue_remove(q, pos);

            fill_msg(pmsg, &m);
            q->changebits = 0;
            R[0] = 1;
            return;
        }
        /* WM_QUIT is kept in the queue and forwarded "only when the queue contains no other messages" (Learn About Messages and Message Queues): a WM_PAINT or a WM_TIMER that is due is a message of the queue (decision 0123)
         * and comes before it. */
        {
            fle_tmsg sm;

            if (fle_win_synth_message(fle_thread_current(), fh, fmin, fmax, 1, 0xFFFFu, &sm)) {
                fill_msg(pmsg, &sm);
                q->changebits = 0;
                R[0] = 1;
                return;
            }
        }
        if (q->quit) {
            fle_tmsg m;

            memset(&m, 0, sizeof m);
            m.msg = FLE_WM_QUIT_ID;
            m.wp = (uint64_t)(int64_t)q->quit_code;
            fill_msg(pmsg, &m);
            q->quit = 0;
            q->changebits = 0;
            R[0] = 0;
            return;
        }
        fle_thread_wait_message(api, 1, fh, fmin, fmax);
    }
}

/* user32!PeekMessageW: BOOL f(LPMSG lpMsg rcx, HWND hWnd rdx, UINT wMsgFilterMin r8d, UINT wMsgFilterMax r9d, UINT wRemoveMsg [5]). Learn: the same retrieval as GetMessageW without waiting: nonzero when
 * a message is available, zero when none is; PM_REMOVE removes it, PM_NOREMOVE leaves it; PM_NOYIELD is accepted (nothing to release). "By default, all message types are processed. To specify that only
 * certain message should be processed, specify one or more of": PM_QS_INPUT (mouse and keyboard messages: none exists here), PM_QS_POSTMESSAGE (all posted messages including timers and hotkeys),
 * PM_QS_PAINT, PM_QS_SENDMESSAGE (all sent messages): with one of them given, only the types named are processed, the sent messages included (S-pmqs-sent: a reading of "only certain message should be processed";
 * the page also says the system dispatches the pending sent messages "during this call"). WM_QUIT is a posted message (GetQueueStatus lists it with QS_POSTMESSAGE: S-pmqs-quit). A high word that is not a union of the four
 * documented values, and low-word bits other than PM_REMOVE and PM_NOYIELD, stop. Like GetMessageW it delivers the pending sent messages first and marks the messages old (Learn: "PeekMessage, GetMessage and WaitMessage mark messages in the queue as old
 * messages"). WM_QUIT is returned (nonzero) like any message once no other posted message matches and PM_REMOVE removes it. An hWnd that is not a window of the calling thread stops (the page gives no
 * failure value). */
void fle_imp_PeekMessageW(void)
{
    static const char api[] = "PeekMessageW";
    uint64_t pmsg = R[1], fh = R[2];
    uint32_t fmin = (uint32_t)R[8], fmax = (uint32_t)R[9], rm = (uint32_t)fle_arg(5);
    uint32_t hi = rm >> 16, qs = 0xFFFFu;
    int want_sent = 1, want_post = 1;
    fle_tqueue *q;
    int pos;

    fle_win_check_thread(api);
    need_scheduler(api);
    if (pmsg == 0) fle_stop(api, "lpMsg is NULL: the page gives no behaviour");
    if ((rm & 0xFFFFu) & ~(FLE_PM_REMOVE | FLE_PM_NOYIELD) || (hi & ~FLE_PM_QS_ALL_HI)) {
        char why[200];

        snprintf(why, sizeof why, "wRemoveMsg %#x has a bit that is not PM_REMOVE, PM_NOYIELD or a PM_QS_* value of the page: not modelled (decision 0126)", (unsigned)rm);
        fle_stop(api, why);
    }
    if (hi != 0) {
        want_sent = (hi & FLE_QS_SENDMESSAGE) != 0;
        want_post = (hi & (FLE_QS_POSTMESSAGE | FLE_QS_HOTKEY | FLE_QS_TIMER)) != 0;
        qs = 0;
        if (hi & FLE_QS_PAINT) qs |= FLE_QS_PAINT;
        if (hi & FLE_QS_TIMER) qs |= FLE_QS_TIMER;
        qs |= hi & FLE_QS_INPUT_BITS;                  /* decision 0136: PM_QS_INPUT = QS_KEY | QS_MOUSE (Learn PeekMessage) */
    }
    q = fle_thread_queue(fle_thread_current(), 1);
    if (!filter_ok(fh)) fle_stop(api, "hWnd is not a live window of the calling thread, NULL or -1: the page gives no failure value for PeekMessage (decision 0122)");
    if (want_sent) fle_win_process_sent(api);
    pos = want_post ? fle_thread_queue_find(q, fh, fmin, fmax) : -1;
    if (pos >= 0) {
        fle_tmsg m = (rm & FLE_PM_REMOVE) ? fle_thread_queue_remove(q, pos) : q->q[(q->head + (uint32_t)pos) % q->cap];

        fill_msg(pmsg, &m);
        q->changebits = 0;
        R[0] = 1;
        return;
    }
    {
        fle_tmsg sm;

        if (fle_win_synth_message(fle_thread_current(), fh, fmin, fmax, (rm & FLE_PM_REMOVE) != 0, qs, &sm)) {
            fill_msg(pmsg, &sm);
            q->changebits = 0;
            R[0] = 1;
            return;
        }
    }
    if (q->quit && want_post) {
        fle_tmsg m;

        memset(&m, 0, sizeof m);
        m.msg = FLE_WM_QUIT_ID;
        m.wp = (uint64_t)(int64_t)q->quit_code;
        fill_msg(pmsg, &m);
        if (rm & FLE_PM_REMOVE) q->quit = 0;
        q->changebits = 0;
        R[0] = 1;
        return;
    }
    q->changebits = 0;
    R[0] = 0;
}

/* user32!WaitMessage: BOOL f(void). Learn: "suspends the thread and does not return until a new message is placed in the thread's message queue"; messages that PeekMessage, GetMessage, GetQueueStatus,
 * WaitMessage or MsgWaitForMultipleObjects already looked at are not new. The thread waits (the others run). Sent messages from other threads are delivered (they are not "placed in the message queue":
 * after delivering them it keeps waiting: a reading). The return value is nonzero. */
void fle_imp_WaitMessage(void)
{
    static const char api[] = "WaitMessage";
    fle_tqueue *q;

    fle_win_check_thread(api);
    need_scheduler(api);
    q = fle_thread_queue(fle_thread_current(), 1);
    for (;;) {
        fle_win_process_sent(api);
        if (q->changebits & (FLE_QS_POSTMESSAGE_W | FLE_QS_ALLPOSTMESSAGE_W | FLE_QS_INPUT_BITS)) break;
        {
            /* decision 0136: a message the retrieval call would make (an input event of the host, a hover, a WM_TIMER that is due, a WM_PAINT) is a message that appeared in the queue; without this a thread that
             * msg_ready of fle_thread.c wakes for it would wait again at once and spin (the message is not "placed" by a post). Nothing is removed or consumed here (remove = 0). */
            fle_tmsg sm;

            if (fle_win_synth_message(fle_thread_current(), 0, 0, 0, 0, 0xFFFFu, &sm)) break;
        }
        fle_thread_wait_message(api, 2, 0, 0, 0);
    }
    q->changebits = 0;
    R[0] = 1;
}

/* user32!PostQuitMessage: VOID f(int nExitCode ecx). Learn: "posts a WM_QUIT message to the thread's message queue and returns immediately"; the exit code is the wParam of WM_QUIT (an int as a
 * WPARAM: sign-extended). One WM_QUIT per queue is kept (a second call replaces the code: a choice). */
void fle_imp_PostQuitMessage(void)
{
    fle_tqueue *q;

    fle_win_check_thread("PostQuitMessage");
    need_scheduler("PostQuitMessage");
    q = fle_thread_queue(fle_thread_current(), 1);
    q->quit = 1;
    q->quit_code = (int32_t)(uint32_t)R[1];
    q->changebits |= FLE_QS_POSTMESSAGE_W | FLE_QS_ALLPOSTMESSAGE_W;
    fle_thread_wake_check();
}

/* user32!PostThreadMessageW: BOOL f(DWORD idThread ecx, UINT Msg edx, WPARAM wParam r8, LPARAM lParam r9) (decision 0147, PROVISIONAL). Learn (PostThreadMessageW): the message is put in the message queue of
 * the thread idThread and the call "returns without waiting for the thread to process the message"; it fails when idThread "is not a valid thread identifier, or if the thread specified by idThread does not have
 * a message queue" (GetLastError returns ERROR_INVALID_THREAD_ID, 1444: the queue of a thread exists after its first call of a User or GDI function, here after its first call that needs one: fle_thread_queue) and
 * with ERROR_NOT_ENOUGH_QUOTA (1816) "when the message limit is hit" (10,000 per queue, as PostMessageW); "The thread to which the message is posted retrieves the message by calling the GetMessage or PeekMessage
 * function. The hwnd member of the returned MSG structure is NULL"; a message not associated with a window "cannot be dispatched by the DispatchMessage function" (DispatchMessageW returns 0 for it). A thread
 * that has ended has no queue and no valid identifier any more: 1444 (a reading; the page says nothing about the identifier of a thread that ended). WM_QUIT: Learn (About Messages and Message Queues) "The WM_PAINT,
 * WM_TIMER, and WM_QUIT messages, however, are kept in the queue and are forwarded to the window procedure only when the queue contains no other messages", and (WM_QUIT) "This message causes the GetMessage function
 * to return zero": a WM_QUIT posted this way is the same kept WM_QUIT as PostQuitMessage makes (one per queue, wParam is the exit code; a second one replaces the code: S10 of decision 0122). The nonzero return value
 * on success is the page's. The identifiers of the guest's threads are FLE_EMULATED_TID + the table index of fle_thread.c. */
void fle_imp_PostThreadMessageW(void)
{
    static const char api[] = "PostThreadMessageW";
    uint32_t id = (uint32_t)R[1], msg = (uint32_t)R[2];
    uint64_t wp = R[8], lp = R[9];
    int idx;
    fle_tqueue *q;

    fle_win_check_thread(api);
    need_scheduler(api);
    idx = fle_thread_index_of_vid(id);
    q = idx >= 0 && !fle_thread_is_done((unsigned)idx) ? fle_thread_queue((unsigned)idx, 0) : NULL;
    if (!q) {
        fle_set_last_error(FLE_ERROR_INVALID_THREAD_ID);
        R[0] = 0;
        return;
    }
    if (msg == FLE_WM_QUIT_ID) {
        q->quit = 1;
        q->quit_code = (int32_t)(uint32_t)wp;
        q->changebits |= FLE_QS_POSTMESSAGE_W | FLE_QS_ALLPOSTMESSAGE_W;
        fle_thread_wake_check();
        R[0] = 1;
        return;
    }
    if (!fle_thread_post((unsigned)idx, 0, msg, wp, lp)) {
        fle_set_last_error(FLE_ERROR_NOT_ENOUGH_QUOTA_W);
        R[0] = 0;
        return;
    }
    R[0] = 1;
}

/* user32!TranslateMessage: BOOL f(const MSG *lpMsg rcx). Learn: key messages (WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP) produce character messages and return nonzero; "If the message is not
 * translated (that is, a character message is not posted to the thread's message queue), the return value is zero"; the message is not modified. No key message exists in this environment (nothing
 * posts one), so one that reaches here stops; every other message returns 0. */
void fle_imp_TranslateMessage(void)
{
    uint64_t pmsg = R[1];
    uint32_t msg;

    fle_win_check_thread("TranslateMessage");
    if (pmsg == 0) fle_stop("TranslateMessage", "lpMsg is NULL: the page gives no behaviour");
    msg = (uint32_t)fle_ld(pmsg + FLE_MSG_OFF_MESSAGE, 4);
    if (msg == 0x0100u || msg == 0x0101u || msg == 0x0104u || msg == 0x0105u) {
        /* decision 0136: the character message of a key-down is made by fle_input.c (the US layout, S-us-layout); without it the call stops as before */
        if (!fle_input_translate) fle_stop("TranslateMessage", "a key message: the keyboard layout that makes the character message is not modelled (decision 0122)");
        R[0] = (uint64_t)fle_input_translate("TranslateMessage", pmsg);
        return;
    }
    R[0] = 0;
}

/* user32!DispatchMessageW / DispatchMessageA: LRESULT f(const MSG *lpMsg rcx). Learn (one page for both): "dispatches a message to a window procedure"; the return value "specifies the value returned by the window procedure". A message with
 * hwnd NULL is a thread message that "cannot be dispatched by the DispatchMessage function" (PostThreadMessage page): returns 0 and calls nothing (a reading: the page gives no value). A message for a
 * live window of this table owned by the calling thread goes to the window's procedure through fle_call_guest in this thread's context. WM_TIMER with a TIMERPROC in lParam (the page: "lParam points to a
 * function that is called instead of the window procedure") and a window of another thread stop (nothing makes them here).
 * decision 0222: a message whose window is no longer there (the person closed it between the posting and the dispatch: the queue of a thread can still hold what was retrieved before) has no window
 * procedure to go to: the answer is 0 and nothing is called (Wine 10.0 win32u dispatch_message: "if (!is_window(hwnd)) return 0"; Learn is silent: S-dispatch-dead-window). It stopped before ("the page gives no
 * behaviour"; sweep1's wall: F12 in the piano roll, MSG hwnd = a closed window, message 0xB021). Reverse switch: FLE_DISPATCH_STOPS_ON_DEAD_WINDOW.
 * The A variant (decision 0222; sweep1's wall at user32!DispatchMessageA): the same dispatch. The window procedures of this model are one kind; Learn (Unicode and ANSI pages) says the system converts the
 * strings of the messages that carry text between the ANSI and the Unicode form for a window of the other kind: a message that carries text (WM_CREATE, WM_NCCREATE, WM_SETTEXT, WM_GETTEXT, and a character
 * message above 0x7F) stops here by name, any other message goes through as it is (S-dispatch-a-passthrough). */
static void dispatch_message(const char *api, int ansi)
{
    uint64_t pmsg = R[1], h, wp, lp;
    uint32_t msg;
    int64_t i;

    fle_win_check_thread(api);
    need_scheduler(api);
    if (pmsg == 0) fle_stop(api, "lpMsg is NULL: the page gives no behaviour");
    h = fle_ld(pmsg + FLE_MSG_OFF_HWND, 8);
    msg = (uint32_t)fle_ld(pmsg + FLE_MSG_OFF_MESSAGE, 4);
    wp = fle_ld(pmsg + FLE_MSG_OFF_WPARAM, 8);
    lp = fle_ld(pmsg + FLE_MSG_OFF_LPARAM, 8);
    if (h == 0) {
        R[0] = 0;
        return;
    }
    if (ansi && (msg == 0x0001u || msg == 0x0081u || msg == 0x000Cu || msg == 0x000Du || ((msg == 0x0102u || msg == 0x0103u || msg == 0x0106u || msg == 0x0107u) && wp > 0x7Fu)))
        fle_stop(api, "an ANSI message that carries text (WM_CREATE, WM_NCCREATE, WM_SETTEXT, WM_GETTEXT, a character above 0x7F): the conversion to the Unicode form the page describes is not modelled");
    if (msg == FLE_WM_TIMER && lp != 0) {
        /* Learn WM_TIMER / SetTimer: "DispatchMessage will call the TimerProc callback function specified in the call to the SetTimer" and "the DispatchMessage calls the callback function instead of calling the window
         * procedure when it processes WM_TIMER with a non-NULL lParam"; TIMERPROC(HWND, UINT, UINT_PTR, DWORD): the window, WM_TIMER, the timer id, the milliseconds since the system started (the virtual clock). */
        R[0] = 0;
        (void)fle_call_guest(lp, h, FLE_WM_TIMER, wp, (uint32_t)(clk_now() / FLE_TICKS_PER_MS));
        R[0] = 0;
        return;
    }
    i = live_index(h);
    if (i < 0) {
        R[0] = 0;
        return;
    }
    if (tab[i].owner != fle_thread_current()) fle_stop(api, "the message is for a window that another thread owns: the page gives no behaviour");
    R[0] = call_wndproc(api, tab[i].wndproc, h, msg, wp, lp);
}

void fle_imp_DispatchMessageW(void) { dispatch_message("DispatchMessageW", 0); }
void fle_imp_DispatchMessageA(void) { dispatch_message("DispatchMessageA", 1); }

/* The window table for the harness's stop report. Decision 0126: followed by the table in the form of the evidence blocks (class, title, parent, z) and the drawing surfaces (fle_raster.c, a weak reference). */
void fle_raster_report(FILE *f) __attribute__((weak));

void fle_win_report(FILE *f)
{
    uint64_t i;

    fprintf(f, "windows: %" PRIu64 "\n", tab_n);
    for (i = 0; i < tab_n; i++) {
        fprintf(f, "  window %#" PRIx64 ": owner thread %u atom %#x wndproc %#" PRIx64 " style %#x exstyle %#x rect (%d, %d, %d x %d)%s%s\n",
                FLE_WIN_HANDLE_BASE + (uint64_t)FLE_WIN_HANDLE_STEP * (i + 1u), (unsigned)tab[i].owner, (unsigned)tab[i].atom, tab[i].wndproc, (unsigned)tab[i].style,
                (unsigned)tab[i].exstyle, (int)tab[i].x, (int)tab[i].y, (int)tab[i].cx, (int)tab[i].cy, tab[i].msgonly ? " message-only" : "", tab[i].live ? "" : " (not live)");
    }
    fle_win_table_report(f);
    fle_win_timer_report(f);
    if (fle_raster_report) fle_raster_report(f);
}

/* ======================================================================================================================================================================
 * Decision 0123 (PROVISIONAL): window management. STATE ONLY: no pixel is drawn, no real input exists. Every answer below is from a Learn page named in the comment; where the page is silent the choice is named as
 * "a choice without a source" and listed in docs/decisions/0123-window-model-and-walls.md (list S-win). The window procedure is called through call_wndproc in the context of the thread that owns the window.
 * ====================================================================================================================================================================== */

#define HWND_NONE UINT64_C(0)

static void put_i32(uint64_t a, int32_t v) { fle_st(a, 4, (uint32_t)v); }

/* A hWnd of the API that must be a live window of this table (not the desktop). Failure: 0/FALSE and ERROR_INVALID_WINDOW_HANDLE (1400, a choice without a source: the pages say "To get extended error
 * information, call GetLastError" and name no code). */
static int64_t win_arg(uint64_t h)
{
    int64_t i = live_index(h);

    if (i < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
        R[0] = 0;
    }
    return i;
}

/* The message to the window procedure of window i (SendMessage semantics: called immediately; a window of another thread is reached through the sent-message queue of fle_thread.c). */
static uint64_t send_to(const char *api, int64_t i, uint32_t msg, uint64_t wp, uint64_t lp)
{
    fle_sent sm;

    if (fle_thread_current && tab[i].owner != fle_thread_current()) {
        memset(&sm, 0, sizeof sm);
        sm.hwnd = handle_of(i);
        sm.msg = msg;
        sm.wp = wp;
        sm.lp = lp;
        fle_thread_send(tab[i].owner, &sm);
        fle_thread_wait_sent(api, &sm, tab[i].owner);
        return sm.result;
    }
    callwndproc_hooks(i, handle_of(i), msg, wp, lp);
    return call_wndproc(api, tab[i].wndproc, handle_of(i), msg, wp, lp);
}

/* ---- queries ---- */

/* user32!IsWindow: BOOL f(HWND rcx). Learn: nonzero if the handle identifies an existing window. A destroyed window and a value that was never issued are not existing windows. The desktop window exists. */
void fle_imp_IsWindow(void)
{
    uint64_t h = R[1];

    fle_win_check_thread("IsWindow");
    R[0] = (h == FLE_DESKTOP_HANDLE || live_index(h) >= 0) ? 1u : 0u;
}

/* user32!IsWindowVisible: BOOL f(HWND rcx). Learn: nonzero when the window, its parent, its parent's parent ... have WS_VISIBLE. An invalid handle: 0 (the page's "Otherwise, the return value is zero"; the last error is
 * not touched). The desktop window is visible (a choice: the page does not say). */
void fle_imp_IsWindowVisible(void)
{
    uint64_t h = R[1];
    int64_t i;

    fle_win_check_thread("IsWindowVisible");
    if (h == FLE_DESKTOP_HANDLE) {
        R[0] = 1;
        return;
    }
    i = live_index(h);
    R[0] = (i >= 0 && is_visible(i)) ? 1u : 0u;
}

/* user32!IsWindowEnabled: BOOL f(HWND rcx). Learn: nonzero if the window is enabled. An invalid handle: 0 (a choice: no page text; no last error). */
void fle_imp_IsWindowEnabled(void)
{
    int64_t i;

    fle_win_check_thread("IsWindowEnabled");
    i = live_index(R[1]);
    R[0] = (i >= 0 && tab[i].enabled) ? 1u : 0u;
}

void fle_imp_IsIconic(void)
{
    int64_t i;

    fle_win_check_thread("IsIconic");
    i = live_index(R[1]);
    R[0] = (i >= 0 && tab[i].iconic) ? 1u : 0u;
}

void fle_imp_IsZoomed(void)
{
    int64_t i;

    fle_win_check_thread("IsZoomed");
    i = live_index(R[1]);
    R[0] = (i >= 0 && tab[i].zoomed) ? 1u : 0u;
}

/* user32!GetDesktopWindow: HWND f(void). Learn: a handle to the desktop window. The value FLE_DESKTOP_HANDLE is a choice without a source. */
void fle_imp_GetDesktopWindow(void)
{
    fle_win_check_thread("GetDesktopWindow");
    R[0] = FLE_DESKTOP_HANDLE;
}

/* user32!GetParent: HWND f(HWND rcx). Learn: "If the window is a child window, the return value is a handle to the parent window. If the window is a top-level window with the WS_POPUP style, the return value is a handle
 * to the owner window. If the function fails, the return value is NULL ... This function typically fails for one of the following reasons: The window is a top-level window that is unowned or does not have the
 * WS_POPUP style. The owner window has WS_POPUP style" (the last sentence is read: an owner that is itself a WS_POPUP window makes GetParent fail with NULL; the page's wording is that of the failure list and is
 * applied literally here). The last error of a NULL result is not changed (a choice: the page gives no code). */
void fle_imp_GetParent(void)
{
    int64_t i;

    fle_win_check_thread("GetParent");
    i = live_index(R[1]);
    if (i < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
        R[0] = 0;
        return;
    }
    if (tab[i].parent >= 0) {
        R[0] = handle_of(tab[i].parent);
        return;
    }
    if ((tab[i].style & FLE_WS_POPUP) && tab[i].owner_win >= 0 && !(tab[tab[i].owner_win].style & FLE_WS_POPUP)) {
        R[0] = handle_of(tab[i].owner_win);
        return;
    }
    R[0] = 0;
}

/* user32!GetWindow: HWND f(HWND rcx, UINT edx). Learn GetWindow: GW_CHILD (5) the child at the top of the Z order (children only, not descendants); GW_ENABLEDPOPUP (6) the enabled pop-up owned by the window (the first found
 * with GW_HWNDNEXT), else the window itself; GW_HWNDFIRST (0), GW_HWNDLAST (1): the highest/lowest window of the same type (a topmost window: a topmost window; a top-level window: a top-level window; a child: a sibling);
 * GW_HWNDNEXT (2): the window below, GW_HWNDPREV (3): above; GW_OWNER (4): the owner. No such window: NULL. */
void fle_imp_GetWindow(void)
{
    static const char api[] = "GetWindow";
    uint64_t h = R[1];
    uint32_t cmd = (uint32_t)R[2];
    int64_t i, idx[SIB_MAX];
    uint32_t n, k, me = 0;

    fle_win_check_thread(api);
    if (h == FLE_DESKTOP_HANDLE) {
        n = siblings(-1, idx, SIB_MAX);
        if (cmd == FLE_GW_CHILD) R[0] = n ? handle_of(idx[0]) : 0;
        else if (cmd == FLE_GW_HWNDFIRST || cmd == FLE_GW_HWNDLAST || cmd == FLE_GW_HWNDNEXT || cmd == FLE_GW_HWNDPREV || cmd == FLE_GW_OWNER) R[0] = 0;
        else fle_stop(api, "uCmd is not one of the values of the page: not defined");
        return;
    }
    i = live_index(h);
    if (i < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
        R[0] = 0;
        return;
    }
    switch (cmd) {
    case FLE_GW_OWNER:
        R[0] = tab[i].owner_win >= 0 ? handle_of(tab[i].owner_win) : 0;
        return;
    case FLE_GW_CHILD:
        n = siblings(i, idx, SIB_MAX);
        R[0] = n ? handle_of(idx[0]) : 0;
        return;
    case FLE_GW_ENABLEDPOPUP: {
        uint64_t k2;

        for (k2 = 0; k2 < tab_n; k2++) {
            if (tab[k2].live && tab[k2].owner_win == i && (tab[k2].style & FLE_WS_POPUP) && tab[k2].enabled) {
                R[0] = handle_of((int64_t)k2);
                return;
            }
        }
        R[0] = h;
        return;
    }
    case FLE_GW_HWNDFIRST:
    case FLE_GW_HWNDLAST:
    case FLE_GW_HWNDNEXT:
    case FLE_GW_HWNDPREV: {
        int top = (tab[i].exstyle & FLE_WS_EX_TOPMOST) != 0;
        uint32_t lo = 0, hi;

        n = siblings(tab[i].parent, idx, SIB_MAX);
        for (k = 0; k < n; k++)
            if (idx[k] == i) me = k;
        /* "If the specified window is a topmost window, the handle identifies a topmost window. If the specified window is a top-level window, the handle identifies a top-level window": the group is
         * the topmost block for a topmost window and the non-topmost block otherwise, for top-level windows; a child's group is its siblings. */
        hi = n;
        if (tab[i].parent < 0) {
            uint32_t nt = 0;

            for (k = 0; k < n; k++)
                if (tab[idx[k]].exstyle & FLE_WS_EX_TOPMOST) nt++;
            if (top) hi = nt;
            else lo = nt;
        }
        if (cmd == FLE_GW_HWNDFIRST) R[0] = hi > lo ? handle_of(idx[lo]) : 0;
        else if (cmd == FLE_GW_HWNDLAST) R[0] = hi > lo ? handle_of(idx[hi - 1u]) : 0;
        else if (cmd == FLE_GW_HWNDNEXT) R[0] = (me + 1u < hi) ? handle_of(idx[me + 1u]) : 0;
        else R[0] = (me > lo) ? handle_of(idx[me - 1u]) : 0;
        return;
    }
    default:
        fle_stop(api, "uCmd is not one of the values of the page: not defined");
    }
}

/* user32!GetAncestor: HWND f(HWND rcx, UINT edx). Learn: GA_PARENT (1): the parent, "does not include the owner"; GA_ROOT (2): "the root window by walking the chain of parent windows"; GA_ROOTOWNER (3): "the owned root window by
 * walking the chain of parent and owner windows returned by GetParent". "If this parameter is the desktop window, the function returns NULL." A top-level window's parent is the desktop (GA_PARENT of a top-level window is
 * the desktop window: the page does not say, a choice: the desktop handle; for GA_ROOT the top-level window itself). Invalid handle: NULL. */
void fle_imp_GetAncestor(void)
{
    static const char api[] = "GetAncestor";
    uint64_t h = R[1];
    uint32_t fl = (uint32_t)R[2];
    int64_t i;

    fle_win_check_thread(api);
    if (h == FLE_DESKTOP_HANDLE) {
        R[0] = 0;
        return;
    }
    i = live_index(h);
    if (i < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
        R[0] = 0;
        return;
    }
    switch (fl) {
    case FLE_GA_PARENT:
        R[0] = tab[i].parent >= 0 ? handle_of(tab[i].parent) : FLE_DESKTOP_HANDLE;
        return;
    case FLE_GA_ROOT:
        R[0] = handle_of(root_of(i));
        return;
    case FLE_GA_ROOTOWNER: {
        int64_t k = i;

        for (;;) {
            if (tab[k].parent >= 0) k = tab[k].parent;
            else if (tab[k].owner_win >= 0) k = tab[k].owner_win;
            else break;
        }
        R[0] = handle_of(k);
        return;
    }
    default:
        fle_stop(api, "gaFlags is not GA_PARENT, GA_ROOT or GA_ROOTOWNER: the page defines no other value");
    }
}

/* user32!IsChild: BOOL f(HWND hWndParent rcx, HWND hWnd rdx). Learn: nonzero if the window is a child or descendant of the specified parent window. */
void fle_imp_IsChild(void)
{
    int64_t p, c;

    fle_win_check_thread("IsChild");
    p = live_index(R[1]);
    c = live_index(R[2]);
    R[0] = 0;
    if (p < 0 || c < 0) return;
    while (tab[c].parent >= 0) {
        c = tab[c].parent;
        if (c == p) {
            R[0] = 1;
            return;
        }
    }
}

/* user32!GetWindowThreadProcessId: DWORD f(HWND rcx, LPDWORD rdx). Learn: the identifier of the thread that created the window; "If the window handle is invalid, the return value is zero"; *lpdwProcessId receives the process
 * identifier when not NULL and "if the function fails, the value of the variable is unchanged". The process id is FLE_EMULATED_PID (fle_proc.h, decision 0056). The thread id is the virtual id of the owner thread. */
void fle_imp_GetWindowThreadProcessId(void)
{
    int64_t i;
    uint64_t pp = R[2];

    fle_win_check_thread("GetWindowThreadProcessId");
    i = live_index(R[1]);
    if (i < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
        R[0] = 0;
        return;
    }
    if (pp != 0) fle_st(pp, 4, 0x1000u);
    R[0] = fle_thread_vid ? fle_thread_vid(tab[i].owner) : 0x1001u;
}

/* ---- geometry ---- */

/* user32!GetWindowRect: BOOL f(HWND rcx, LPRECT rdx). Learn: "the screen coordinates of the upper-left and lower-right corners of the window ... the bottom-right coordinates ... are exclusive"; a child window's rectangle is
 * in screen coordinates too (Window Features: "The coordinates are relative to the upper left corner of the screen, even for a child window"). */
void fle_imp_GetWindowRect(void)
{
    int64_t i;
    uint64_t out = R[2];
    int32_t ox, oy;

    fle_win_check_thread("GetWindowRect");
    if (out == 0) fle_stop("GetWindowRect", "lpRect is NULL: the page gives no behaviour");
    if (R[1] == FLE_DESKTOP_HANDLE) {
        put_i32(out, 0);
        put_i32(out + 4, 0);
        put_i32(out + 8, FLE_GDC_HORZRES);
        put_i32(out + 12, FLE_GDC_VERTRES);
        R[0] = 1;
        return;
    }
    i = win_arg(R[1]);
    if (i < 0) return;
    /* the window rectangle in screen coordinates: the origin of the PARENT's client area plus the window's own position */
    if (tab[i].parent >= 0) client_origin(tab[i].parent, &ox, &oy);
    else ox = oy = 0;
    put_i32(out, ox + tab[i].px);
    put_i32(out + 4, oy + tab[i].py);
    put_i32(out + 8, ox + tab[i].px + tab[i].pcx);
    put_i32(out + 12, oy + tab[i].py + tab[i].pcy);
    R[0] = 1;
}

/* user32!MonitorFromWindow: HMONITOR f(HWND hwnd rcx, DWORD dwFlags edx) (decision 0126, PROVISIONAL, wall 100; the notes are in fle_monitor.c). The bounding rectangle of the window is its window rectangle in screen coordinates (as
 * GetWindowRect gives it); a minimized window uses the rectangle it had before it was minimized (Learn). The desktop window is the screen rectangle. */
void fle_imp_MonitorFromWindow(void)
{
    static const char api[] = "MonitorFromWindow";
    uint32_t flags = (uint32_t)R[2];
    int64_t i;
    int32_t ox = 0, oy = 0, l, t, r, b;

    fle_win_check_thread(api);
    if (!fle_monitor_resolve || !fle_monitor_rect_intersects) fle_stop(api, "no monitor model is linked (decision 0126)");
    if (R[1] == FLE_DESKTOP_HANDLE) {
        R[0] = fle_monitor_resolve(api, 1, flags);
        return;
    }
    i = live_index(R[1]);
    if (i < 0) {
        R[0] = fle_monitor_resolve(api, 0, flags);
        return;
    }
    if (tab[i].parent >= 0) client_origin(tab[i].parent, &ox, &oy);
    if (tab[i].iconic) {
        l = ox + tab[i].rest_x;
        t = oy + tab[i].rest_y;
        r = l + tab[i].rest_cx;
        b = t + tab[i].rest_cy;
    } else {
        l = ox + tab[i].px;
        t = oy + tab[i].py;
        r = l + tab[i].pcx;
        b = t + tab[i].pcy;
    }
    R[0] = fle_monitor_resolve(api, fle_monitor_rect_intersects(l, t, r, b), flags);
}

/* ---- user32!SetWindowsHookExW and UnhookWindowsHookEx (decision 0126, PROVISIONAL, wall 101) ----
 * Learn SetWindowsHookExW: "Installs an application-defined hook procedure into a hook chain"; idHook the type; lpfn "A pointer to the hook procedure ... Otherwise (a thread of the current process), lpfn can point to a hook procedure in
 * the code associated with the current process"; hmod "must be set to NULL if the dwThreadId parameter specifies a thread created by the current process and if the hook procedure is within the code associated with the current
 * process"; dwThreadId "the identifier of the thread with which the hook procedure is to be associated"; the result is "the handle to the hook procedure" or NULL (GetLastError), "An error may occur if the hMod parameter is NULL
 * and the dwThreadId parameter is zero or specifies the identifier of a thread created by another process". Learn UnhookWindowsHookEx: "Removes a hook procedure installed in a hook chain"; nonzero on success, zero on failure
 * (GetLastError).
 * Answered: WH_MOUSE (7) and WH_KEYBOARD (2) (the numbers are on the page) installed for a thread of this process (dwThreadId = the id of a live guest thread: fle_thread_index_of_vid, or the first thread's id 0x1001) with hmod NULL and a
 * non-NULL lpfn. (S-hook-no-events of this decision, "recorded and NEVER CALLED: no mouse or keyboard message exists", ended with decision 0323: fle_win_input_call_hooks calls them when GetMessage / PeekMessage is about to return a
 * mouse or key message.) Every other hook type stops (WH_CALLWNDPROC, WH_GETMESSAGE, WH_CBT, WH_MSGFILTER, ... would be called by the window messages that do occur: not modelled), as do dwThreadId 0 (a global hook needs a DLL
 * and every thread of the desktop), a thread id that is not a thread of this process (no failure code is on the page), hmod not NULL and lpfn NULL. The handle is FLE_HOOK_HANDLE_BASE + 4 * (k + 1), never reused (a choice: the page
 * gives no value); at most 64 live hooks (a limit of the virtual machine: the 65th stops). UnhookWindowsHookEx of a handle that is not a live hook of this table returns 0, the last error left alone (the page names no code). */
#define FLE_HOOK_HANDLE_BASE UINT64_C(0x7FFA0000)
#define FLE_HOOK_MAX         64u
#define FLE_WH_KEYBOARD      2
#define FLE_WH_CALLWNDPROC   4   /* decision 0171 */
#define FLE_WH_MOUSE         7

/* Decision 0196 (PROVISIONAL; supersedes "never reused" and "at most 64 live hooks" above where they differ): hook_count counts the LIVE hooks (it counted every hook ever installed, so a sixty-fifth
 * SetWindowsHookExW stopped the run after about 32 opened menus although each menu removed its hooks again). A removed hook frees its table slot. The handle value is no longer the slot index:
 * it is FLE_HOOK_HANDLE_BASE + 4 * n for the n-th hook installed (the same value as before for the first FLE_HOOK_SPAN installs), and after FLE_HOOK_SPAN installs the values start again, skipping
 * the ones that live hooks hold, so UnhookWindowsHookEx of a handle that is not a live hook still returns 0 for FLE_HOOK_SPAN further installs (S-tbl1-hook-span: no source). The chain is walked newest
 * first by `order`, not by slot; a free slot is taken in the order in which the slots were released (never-used first). */
#define FLE_HOOK_SPAN 0x10000u
static struct {
    int live;
    int id;
    uint64_t proc;
    uint32_t thread;
    uint64_t handle;
    uint64_t order;     /* install order (kept after the hook is removed, so that a hook procedure that removes itself can still call CallNextHookEx) */
    uint64_t freed;     /* when the slot was released; 0 = never used */
} hooks[FLE_HOOK_MAX];
static uint32_t hook_count;
static uint64_t hook_issued, hook_clock;

static uint64_t hook_next_handle(void)
{
    for (;;) {
        uint64_t h = FLE_HOOK_HANDLE_BASE + 4u * (hook_issued++ % FLE_HOOK_SPAN + 1u);
        unsigned k;

        for (k = 0; k < FLE_HOOK_MAX; k++)
            if (hooks[k].live && hooks[k].handle == h) break;
        if (k == FLE_HOOK_MAX) return h;
    }
}

void fle_imp_SetWindowsHookExW(void)
{
    static const char api[] = "SetWindowsHookExW";
    int id = (int)(uint32_t)R[1];
    uint64_t proc = R[2], hmod = R[8];
    uint32_t tid = (uint32_t)R[9];
    char why[200];

    fle_win_check_thread(api);
    if (id != FLE_WH_MOUSE && id != FLE_WH_KEYBOARD && id != FLE_WH_CALLWNDPROC) {
        snprintf(why, sizeof why, "idHook %d is not WH_MOUSE (7), WH_KEYBOARD (2) or WH_CALLWNDPROC (4): the other hooks are called by messages that do occur here and are not modelled (decisions 0126, 0171)", id);
        fle_stop(api, why);
    }
    if (tid == 0) fle_stop(api, "dwThreadId is 0: a global hook needs a DLL and every thread of the desktop (decision 0126)");
    if (tid != FLE_EMULATED_TID && !(fle_thread_index_of_vid != NULL && fle_thread_index_of_vid(tid) >= 0))
        fle_stop(api, "dwThreadId is not the id of a thread of this process: the page gives no failure code (decision 0126)");
    if (proc == 0) fle_stop(api, "lpfn is NULL: the page gives no behaviour");
    if (hmod != 0) fle_stop(api, "hmod is not NULL for a thread of the current process: the page says it must be NULL");
    if (hook_count >= FLE_HOOK_MAX) fle_stop(api, "more than 64 hooks are alive: a limit of the virtual machine");
    {
        unsigned k, f = FLE_HOOK_MAX;

        for (k = 0; k < FLE_HOOK_MAX; k++)
            if (!hooks[k].live && (f == FLE_HOOK_MAX || hooks[k].freed < hooks[f].freed)) f = k;
        hooks[f].live = 1;
        hooks[f].id = id;
        hooks[f].proc = proc;
        hooks[f].thread = tid;
        hooks[f].handle = hook_next_handle();
        hooks[f].order = ++hook_clock;
        hook_count++;
        R[0] = hooks[f].handle;
    }
}

void fle_imp_UnhookWindowsHookEx(void)
{
    uint64_t h = R[1];
    uint64_t k;

    fle_win_check_thread("UnhookWindowsHookEx");
    for (k = 0; k < FLE_HOOK_MAX; k++) {
        if (hooks[k].live && hooks[k].handle == h) {
            hooks[k].live = 0;
            hooks[k].freed = ++hook_clock;
            hook_count--;
            R[0] = 1;
            return;
        }
    }
    R[0] = 0;
}

uint32_t fle_win_hook_live(void)
{
    return hook_count;
}

/* ---- Decision 0171 (PROVISIONAL, wall ia1-W14: the engine's own pop-up windows install a WH_CALLWNDPROC hook for the current thread when a menu or context menu opens): WH_CALLWNDPROC ----
 * Learn CallWndProc callback (WH_CALLWNDPROC): "The CallWndProc hook procedure is an application-defined or library-defined callback function used with the SetWindowsHookEx function. The HOOKPROC type defines a pointer to this
 * callback function. CallWndProc is a placeholder for the application-defined or library-defined function name": LRESULT CALLBACK CallWndProc(int nCode, WPARAM wParam, LPARAM lParam); nCode "Specifies whether the hook procedure must
 * process the message. If nCode is HC_ACTION (0), the hook procedure must process the message"; wParam "Specifies whether the message was sent by the current thread. If the message was sent by the current thread, it is nonzero";
 * lParam "A pointer to a CWPSTRUCT structure that contains details about the message" ({LPARAM lParam; WPARAM wParam; UINT message; HWND hwnd;}, 32 bytes on x64); "the hook procedure ... should call CallNextHookEx". The hook is
 * called for a message that is SENT to a window procedure (SendMessageW, and the messages the window functions send); a message that is posted and dispatched by DispatchMessage does not go through it. The hooks of a thread
 * form a chain, the most recently installed first. Here the hooks are called from SendMessageW, send_to, SetWindowTextW and the sent messages of another thread; the messages that CreateWindowExW and SetWindowPos send are not
 * shown to the hook (S-ia1-hook-1: the page lists them as 'sent by the system' only in the CBT hook's text).
 * user32!CallNextHookEx: LRESULT f(HHOOK hhk rcx, int nCode edx, WPARAM wParam r8, LPARAM lParam r9). Learn: "Passes the hook information to the next hook procedure in the current hook chain"; hhk "This parameter is ignored" (Windows
 * 10 era page text); the result is the value of the next procedure, or 0 when the chain ends (S-ia1-hook-2: the page says "The return value is the value returned by the next hook procedure in the chain"; 0 at the end of the chain is a
 * choice). Called outside a hook procedure it stops (the current hook is unknown). A WH_MOUSE / WH_KEYBOARD hook is called since decision 0323 (fle_win_input_call_hooks below), and its procedure walks its chain with this function the same way. */
#define FLE_HC_ACTION 0
static int cur_hook[8];
static unsigned cur_hook_n;

static int hook_older(int from_k, int id, uint32_t thread)
{
    int k, best = -1;

    for (k = 0; k < (int)FLE_HOOK_MAX; k++)
        if (hooks[k].live && hooks[k].id == id && hooks[k].thread == thread && hooks[k].order < hooks[from_k].order && (best < 0 || hooks[k].order > hooks[best].order)) best = k;
    return best;
}

/* Calls the WH_CALLWNDPROC chain of the thread that owns window `wi` for the message (h, msg, wp, lp); `sent_by_current` is the wParam of the hook procedure. */
static void callwndproc_hooks(int64_t wi, uint64_t h, uint32_t msg, uint64_t wp, uint64_t lp)
{
    uint32_t vid;
    int k;
    uint64_t sp0, cwp;
    unsigned me;

    if (hook_count == 0) return;
    vid = fle_thread_vid ? fle_thread_vid(tab[wi].owner) : FLE_EMULATED_TID;
    {
        int j;

        for (k = -1, j = 0; j < (int)FLE_HOOK_MAX; j++)
            if (hooks[j].live && hooks[j].id == FLE_WH_CALLWNDPROC && hooks[j].thread == vid && (k < 0 || hooks[j].order > hooks[k].order)) k = j;
    }
    if (k < 0) return;
    if (cur_hook_n >= sizeof cur_hook / sizeof cur_hook[0]) fle_stop("WH_CALLWNDPROC", "hook procedures are nested more than 8 deep (a limit of the virtual machine)");
    me = fle_thread_current ? fle_thread_current() : 0u;
    sp0 = R[4];
    R[4] = (R[4] - 64u) & ~UINT64_C(15);
    cwp = R[4];
    fle_st(cwp + 0, 8, lp);
    fle_st(cwp + 8, 8, wp);
    fle_st(cwp + 16, 8, msg);
    fle_st(cwp + 24, 8, h);
    cur_hook[cur_hook_n++] = k;
    (void)fle_call_guest(hooks[k].proc, FLE_HC_ACTION, tab[wi].owner == me ? 1u : 0u, cwp, 0);
    cur_hook_n--;
    R[4] = sp0;
}

void fle_imp_CallNextHookEx(void)
{
    static const char api[] = "CallNextHookEx";
    uint64_t n_code = (uint32_t)R[2], wp = R[8], lp = R[9];
    int cur, nx;

    fle_win_check_thread(api);
    if (cur_hook_n == 0) fle_stop(api, "called outside a hook procedure: the current hook is unknown (hhk is ignored by the page's text)");
    cur = cur_hook[cur_hook_n - 1];
    nx = hook_older(cur, hooks[cur].id, hooks[cur].thread);
    if (nx < 0) {
        R[0] = 0;
        return;
    }
    {
        uint64_t sp0 = R[4], res;

        R[4] = (R[4] - 32u) & ~UINT64_C(15);
        cur_hook[cur_hook_n++] = nx;
        res = fle_call_guest(hooks[nx].proc, (uint64_t)(int64_t)(int32_t)n_code, wp, lp, 0);
        cur_hook_n--;
        R[4] = sp0;
        R[0] = res;
    }
}

/* Decision 0323 (replaces S-hook-no-events of decision 0126: "the hook is recorded and NEVER CALLED: no mouse or keyboard message exists in this environment"; the input round trip of decision 0136 made them exist, and the
 * decision's own condition for a second look, "when the input model is built", was met). Learn SetWindowsHookEx: a thread-specific hook procedure is called for the thread it was installed for. Learn MouseProc: "The system
 * calls this function whenever an application calls the GetMessage or PeekMessage function and there is a mouse message to be processed"; nCode HC_ACTION (0) "The wParam and lParam parameters contain information about
 * a mouse message", HC_NOREMOVE (3) "...contain information about a mouse message, and the mouse message has not been removed from the message queue (An application called the PeekMessage function, specifying the
 * PM_NOREMOVE flag.)"; wParam "The identifier of the mouse message"; lParam "A pointer to a MOUSEHOOKSTRUCT structure"; the return value is "If nCode is less than zero, the hook procedure must return the value returned
 * by CallNextHookEx. If nCode is greater than or equal to zero, and the hook procedure did not process the message, it is highly recommended that you call CallNextHookEx and return the value it returns ... If the hook
 * procedure processed the message, it may return a nonzero value to prevent the system from passing the message to the target window procedure." Learn KeyboardProc is the same with wParam "the virtual-key code" and
 * lParam "the repeat count, scan code, extended-key flag, context code, previous key-state flag, and transition-state flag" (the lParam of the key message). The hooks of a thread form a chain, the most recently installed
 * first (the chain of WH_CALLWNDPROC, above); CallNextHookEx walks it. Called by fle_input.c at the point where GetMessage / PeekMessage is about to return a mouse or key message of `thread` (Wine 10.0, win32u/message.c
 * process_mouse_message and process_keyboard_message: the same two calls, nCode HC_ACTION for a removal and HC_NOREMOVE for PM_NOREMOVE, the message dropped when the answer is nonzero: UNOFFICIAL).
 * `blob` (len bytes, NULL = none) is copied onto the guest stack and its address is lParam (the MOUSEHOOKSTRUCT). Returns 1 when the chain of hook `id` for the thread has a hook (the answer of the newest hook is then in
 * *result), 0 when it has none. */
int fle_win_input_call_hooks(int id, unsigned thread, int ncode, uint64_t wp, uint64_t lp, const void *blob, unsigned len, uint64_t *result)
{
    uint32_t vid;
    int k, j;
    uint64_t sp0, res;

    if (hook_count == 0) return 0;
    vid = fle_thread_vid ? fle_thread_vid(thread) : FLE_EMULATED_TID;
    for (k = -1, j = 0; j < (int)FLE_HOOK_MAX; j++)
        if (hooks[j].live && hooks[j].id == id && hooks[j].thread == vid && (k < 0 || hooks[j].order > hooks[k].order)) k = j;
    if (k < 0) return 0;
    if (cur_hook_n >= sizeof cur_hook / sizeof cur_hook[0]) fle_stop("WH_MOUSE / WH_KEYBOARD", "hook procedures are nested more than 8 deep (a limit of the virtual machine)");
    sp0 = R[4];
    R[4] = (R[4] - ((uint64_t)len + 64u)) & ~UINT64_C(15);
    if (blob != NULL && len != 0) {
        unsigned q;

        for (q = 0; q < len; q++) fle_st(R[4] + q, 1, ((const uint8_t *)blob)[q]);
        lp = R[4];
    }
    cur_hook[cur_hook_n++] = k;
    res = fle_call_guest(hooks[k].proc, (uint64_t)(int64_t)ncode, wp, lp, 0);
    cur_hook_n--;
    R[4] = sp0;
    *result = res;
    return 1;
}

/* user32!GetClientRect: BOOL f(HWND rcx, LPRECT rdx). Learn: "The left and top members are zero. The right and bottom members contain the width and height of the window." The non-client area is 0 here (nc_zero). */
void fle_imp_GetClientRect(void)
{
    int64_t i;
    uint64_t out = R[2];

    fle_win_check_thread("GetClientRect");
    if (out == 0) fle_stop("GetClientRect", "lpRect is NULL: the page gives no behaviour");
    if (R[1] == FLE_DESKTOP_HANDLE) {
        put_i32(out, 0);
        put_i32(out + 4, 0);
        put_i32(out + 8, FLE_GDC_HORZRES);
        put_i32(out + 12, FLE_GDC_VERTRES);
        R[0] = 1;
        return;
    }
    i = win_arg(R[1]);
    if (i < 0) return;
    put_i32(out, 0);
    put_i32(out + 4, 0);
    put_i32(out + 8, tab[i].pcx);
    put_i32(out + 12, tab[i].pcy);
    R[0] = 1;
}

/* user32!ClientToScreen: BOOL f(HWND rcx, LPPOINT rdx). Learn: the client coordinates are replaced by the screen coordinates, relative to the upper-left corner of the screen; failure returns 0. */
void fle_imp_ClientToScreen(void)
{
    int64_t i;
    uint64_t p = R[2];
    int32_t ox, oy;

    fle_win_check_thread("ClientToScreen");
    if (p == 0) fle_stop("ClientToScreen", "lpPoint is NULL: the page gives no behaviour");
    if (R[1] == FLE_DESKTOP_HANDLE) {
        R[0] = 1;
        return;
    }
    i = win_arg(R[1]);
    if (i < 0) return;
    client_origin(i, &ox, &oy);
    put_i32(p, (int32_t)fle_ld(p, 4) + ox);
    put_i32(p + 4, (int32_t)fle_ld(p + 4, 4) + oy);
    R[0] = 1;
}

/* user32!ScreenToClient: BOOL f(HWND rcx, LPPOINT rdx). Learn: the screen coordinates are replaced by coordinates relative to the upper-left corner of the window's client area. */
void fle_imp_ScreenToClient(void)
{
    int64_t i;
    uint64_t p = R[2];
    int32_t ox, oy;

    fle_win_check_thread("ScreenToClient");
    if (p == 0) fle_stop("ScreenToClient", "lpPoint is NULL: the page gives no behaviour");
    if (R[1] == FLE_DESKTOP_HANDLE) {
        R[0] = 1;
        return;
    }
    i = win_arg(R[1]);
    if (i < 0) return;
    client_origin(i, &ox, &oy);
    put_i32(p, (int32_t)fle_ld(p, 4) - ox);
    put_i32(p + 4, (int32_t)fle_ld(p + 4, 4) - oy);
    R[0] = 1;
}

/* user32!MapWindowPoints: int f(HWND from rcx, HWND to rdx, LPPOINT pts r8, UINT n r9). Learn: NULL or HWND_DESKTOP = screen coordinates; the points are in device units; the return value: the low word is the number of
 * pixels added to the horizontal coordinate of each source point, the high word the vertical; failure returns 0. Mirrored windows (WS_EX_LAYOUTRTL) are not modelled: a window with that style stops. */
void fle_imp_MapWindowPoints(void)
{
    static const char api[] = "MapWindowPoints";
    uint64_t hf = R[1], ht = R[2], pts = R[8];
    uint32_t n = (uint32_t)R[9], k;
    int32_t fx = 0, fy = 0, tx = 0, ty = 0, dx, dy;
    int64_t i;

    fle_win_check_thread(api);
    if (pts == 0 && n != 0) fle_stop(api, "lpPoints is NULL with cPoints > 0: the page gives no behaviour");
    if (hf != 0 && hf != FLE_DESKTOP_HANDLE) {
        i = win_arg(hf);
        if (i < 0) return;
        if (tab[i].exstyle & 0x00400000u) fle_stop(api, "a mirrored window (WS_EX_LAYOUTRTL): not modelled");
        client_origin(i, &fx, &fy);
    }
    if (ht != 0 && ht != FLE_DESKTOP_HANDLE) {
        i = win_arg(ht);
        if (i < 0) return;
        if (tab[i].exstyle & 0x00400000u) fle_stop(api, "a mirrored window (WS_EX_LAYOUTRTL): not modelled");
        client_origin(i, &tx, &ty);
    }
    dx = fx - tx;
    dy = fy - ty;
    for (k = 0; k < n; k++) {
        put_i32(pts + 8u * k, (int32_t)fle_ld(pts + 8u * k, 4) + dx);
        put_i32(pts + 8u * k + 4, (int32_t)fle_ld(pts + 8u * k + 4, 4) + dy);
    }
    R[0] = ((uint64_t)(uint16_t)dy << 16) | (uint16_t)dx;
}

/* ---- show, position, size, z-order (R3, R4, R8) ---- */

/* WINDOWPOS on x64 (Learn WINDOWPOS lists the members; the offsets are derived from the member types): HWND hwnd @0, HWND hwndInsertAfter @8, int x @16, int y @20, int cx @24, int cy @28, UINT flags @32 = 40 bytes
 * (4 bytes of tail padding). Learn gives no sizeof. */
#define WP_SIZE 40u


/* ---- Decision 0214 (present2; second diff; api2's GAP-U1 of decision 0210): the update region of a window WITHOUT WS_CLIPCHILDREN reaches its child windows ----
 * Learn Window Features / Clipping: "The system does not automatically clip a child window from the parent window's client area. This means the parent window draws over the child window if it carries out any drawing in the
 * same location as the child window. However, the system does clip the child window from the parent window's client area if the parent window has the WS_CLIPCHILDREN style." Learn RedrawWindow: "By default, the windows
 * affected by RedrawWindow depend on whether the specified window has the WS_CLIPCHILDREN style. Child windows that are not the WS_CLIPCHILDREN style are unaffected; non-WS_CLIPCHILDREN windows are recursively validated or
 * invalidated until a WS_CLIPCHILDREN window is encountered." Wine 10.0 (api2, lift/harness/api2/win_upd.c) does the same for InvalidateRect: parent (0,0,200,200) without WS_CLIPCHILDREN, child at (20,20) 100x100:
 * Invalidate(parent) gives the child (0,0,100,100); Invalidate(parent, [30,30,60,60]) gives the child (10,10,40,40); with WS_CLIPCHILDREN the child gets nothing. The rule: the rectangle is added to the window; if the window has no
 * WS_CLIPCHILDREN, each visible child it touches gets the part that lies on the child (in the child's coordinates) by the same rule, so the descent stops below a window that has WS_CLIPCHILDREN (that window itself is reached).
 * S-0214-u1: the model has one update rectangle per window (S-updregion), so a child gets the bounding rectangle of the part of the parent's rectangle that lies on it. */
#define FLE_WS_CLIPCHILDREN_ 0x02000000u

/* ---- Decision 0247 (alpha3, PROVISIONAL): the update region of a window is a set of rectangles, not one bounding rectangle (it was S-updregion of decision 0126) ----
 * Learn "The Update Region": "the system adds only the invalid portion of the window to the update region"; InvalidateRect / InvalidateRgn / RedrawWindow "adds a rectangle / region to the specified window's update region";
 * ValidateRect / ValidateRgn "validates the client area within a rectangle / region by removing it from the current update region"; GetUpdateRgn "retrieves the update region of a window by copying it into the specified region";
 * GetUpdateRect "the smallest rectangle that completely encloses the update region"; BeginPaint "automatically sets the clipping region of the device context to exclude any area outside the update region", rcPaint of
 * PAINTSTRUCT = "the update rectangle" (the bounding box). The region is kept as disjoint rectangles (a fle_visrgn) in tab[i].updr when it is not one rectangle; upd_l..upd_b stays the bounding box (so that every reader of
 * the bounding box is unchanged). A region of more than UPD_PIECES_MAX rectangles is folded to its bounding box (S-0247-fold: an over-estimate, the way every region was before; the folds are counted, fle_win_update_folds). */
#define UPD_PIECES_MAX 32u
static uint64_t upd_folds;

uint64_t fle_win_update_folds(void) { return upd_folds; }

static void upd_clear_rect(int64_t i)
{
    tab[i].upd_delayed = 0;
    tab[i].upd_l = tab[i].upd_t = tab[i].upd_r = tab[i].upd_b = 0;
    free(tab[i].updr);
    tab[i].updr = NULL;
}

static void upd_set_rect(int64_t i, int32_t l, int32_t t, int32_t r, int32_t b)
{
    free(tab[i].updr);
    tab[i].updr = NULL;
    tab[i].upd_l = l;
    tab[i].upd_t = t;
    tab[i].upd_r = r;
    tab[i].upd_b = b;
}

static void upd_set_full(int64_t i) { upd_set_rect(i, 0, 0, tab[i].pcx, tab[i].pcy); }

/* Decision 0319 section 23 (alpha1, PROVISIONAL; O9 / the Balance form: the child windows of a form that was shown never got a WM_PAINT): showing a window gives its whole client area to paint AND the whole client
 * area of every visible descendant, whatever their clip styles. Wine 10.0 (UNOFFICIAL; server/window.c set_window_pos: the exposed region of a window that becomes visible goes to redraw_window(win, ..., RDW_INVALIDATE |
 * RDW_ERASE | RDW_FRAME | RDW_ALLCHILDREN), which recurses into every child that has WS_VISIBLE, WS_CLIPCHILDREN or not: "if ((win->style & WS_CLIPCHILDREN) && !(flags & RDW_ALLCHILDREN)) return" is the only stop and
 * ALLCHILDREN is set). A descendant that had WS_VISIBLE while an ancestor was hidden had no update region (S-0247-hidden: a window that is not visible has none), so when the ancestor is shown it is now visible and must paint.
 * Learn BeginPaint: the update region is set "by the system after sizing, moving, creating, scrolling, or any other operation that affects the client area". No class or window-kind test. */
static void upd_show_descendants(int64_t i)
{
    int64_t idx[SIB_MAX];
    uint32_t n = siblings(i, idx, SIB_MAX), k;

    for (k = 0; k < n; k++) {
        int64_t c = idx[k];

        if (!tab[c].live || !(tab[c].style & FLE_WS_VISIBLE) || tab[c].iconic || tab[c].pcx <= 0 || tab[c].pcy <= 0) continue;
        upd_set_full(c);
        tab[c].upd_erase = 1;
        upd_show_descendants(c);
    }
}

static void upd_get(int64_t i, fle_visrgn *v)
{
    if (!(tab[i].upd_r > tab[i].upd_l && tab[i].upd_b > tab[i].upd_t)) v->n = 0;
    else if (tab[i].updr) {
        v->n = tab[i].updr->n;
        memcpy(v->r, tab[i].updr->r, (size_t)v->n * sizeof v->r[0]);
    } else {
        v->n = 1;
        v->r[0] = (fle_rrect){ tab[i].upd_l, tab[i].upd_t, tab[i].upd_r, tab[i].upd_b };
    }
}

/* a test link that leaves fle_visrgn.c out (its functions are weak) keeps the old rule: the update region is its bounding rectangle */
static int upd_rgn_ok(void) { return fle_visrgn_set_rect != NULL && fle_visrgn_subtract != NULL; }

static fle_rrect upd_bounds(const fle_visrgn *v)
{
    fle_rrect b = { 0, 0, 0, 0 };
    uint32_t k;

    for (k = 0; k < v->n; k++) {
        if (k == 0 || v->r[k].x0 < b.x0) b.x0 = v->r[k].x0;
        if (k == 0 || v->r[k].y0 < b.y0) b.y0 = v->r[k].y0;
        if (k == 0 || v->r[k].x1 > b.x1) b.x1 = v->r[k].x1;
        if (k == 0 || v->r[k].y1 > b.y1) b.y1 = v->r[k].y1;
    }
    return b;
}

/* v becomes the update region of window i (v->n == 0: none) */
static void upd_put(int64_t i, const fle_visrgn *v)
{
    fle_rrect bb;

    if (v->n == 0) {
        upd_clear_rect(i);
        return;
    }
    bb = upd_bounds(v);
    if (v->n == 1 || v->n > UPD_PIECES_MAX) {
        if (v->n > 1) upd_folds++;
        upd_set_rect(i, (int32_t)bb.x0, (int32_t)bb.y0, (int32_t)bb.x1, (int32_t)bb.y1);
        return;
    }
    if (!tab[i].updr) tab[i].updr = (fle_visrgn *)malloc(sizeof *tab[i].updr);
    if (!tab[i].updr) {
        upd_folds++;
        upd_set_rect(i, (int32_t)bb.x0, (int32_t)bb.y0, (int32_t)bb.x1, (int32_t)bb.y1);
        return;
    }
    tab[i].updr->n = v->n;
    memcpy(tab[i].updr->r, v->r, (size_t)v->n * sizeof v->r[0]);
    tab[i].upd_l = (int32_t)bb.x0;
    tab[i].upd_t = (int32_t)bb.y0;
    tab[i].upd_r = (int32_t)bb.x1;
    tab[i].upd_b = (int32_t)bb.y1;
}

/* the update region minus the rectangle (l, t, r, b); an empty result clears the erase request too (nothing is left to erase) */
static void upd_minus_rect(int64_t i, int32_t l, int32_t t, int32_t r, int32_t b)
{
    fle_visrgn v;

    upd_get(i, &v);
    if (v.n == 0 || r <= l || b <= t) return;
    if (!upd_rgn_ok()) {
        if (l <= tab[i].upd_l && t <= tab[i].upd_t && r >= tab[i].upd_r && b >= tab[i].upd_b) {
            upd_clear_rect(i);
            tab[i].upd_erase = 0;
        }
        return;
    }
    if (!fle_visrgn_subtract(&v, (fle_rrect){ l, t, r, b })) {
        fle_rrect bb = upd_bounds(&v);

        upd_folds++;
        if (bb.x1 > bb.x0) upd_set_rect(i, (int32_t)bb.x0, (int32_t)bb.y0, (int32_t)bb.x1, (int32_t)bb.y1);
        return;
    }
    upd_put(i, &v);
    if (v.n == 0) { tab[i].upd_erase = 0; tab[i].upd_delayed = 0; }
}

static void upd_add(int64_t i, int32_t l, int32_t t, int32_t r, int32_t b, int erase)
{
    fle_visrgn v, nw;

    if (l < 0) l = 0;
    if (t < 0) t = 0;
    if (r > tab[i].pcx) r = tab[i].pcx;
    if (b > tab[i].pcy) b = tab[i].pcy;
    if (r <= l || b <= t) return;
    if (!is_visible(i)) return;   /* S-0247-hidden (Wine 10.0, harness/alpha3/win_upd2.c): an invisible window has no update region; ShowWindow gives the whole client area */
    upd_get(i, &v);
    if (v.n == 0) {
        upd_set_rect(i, l, t, r, b);
    } else if (!upd_rgn_ok()) {
        upd_set_rect(i, tab[i].upd_l < l ? tab[i].upd_l : l, tab[i].upd_t < t ? tab[i].upd_t : t, tab[i].upd_r > r ? tab[i].upd_r : r, tab[i].upd_b > b ? tab[i].upd_b : b);
    } else {
        uint32_t k;
        int ok = 1;

        fle_visrgn_set_rect(&nw, (fle_rrect){ l, t, r, b });
        for (k = 0; k < v.n && nw.n && ok; k++) ok = fle_visrgn_subtract(&nw, v.r[k]);
        if (!ok || v.n + nw.n > UPD_PIECES_MAX) {
            /* fold: the bounding box of the old region and the new rectangle */
            int32_t bl = tab[i].upd_l < l ? tab[i].upd_l : l, bt = tab[i].upd_t < t ? tab[i].upd_t : t, br = tab[i].upd_r > r ? tab[i].upd_r : r, bb = tab[i].upd_b > b ? tab[i].upd_b : b;

            upd_folds++;
            upd_set_rect(i, bl, bt, br, bb);
        } else if (nw.n) {
            for (k = 0; k < nw.n; k++) v.r[v.n + k] = nw.r[k];
            v.n += nw.n;
            upd_put(i, &v);
        }
    }
    if (erase) tab[i].upd_erase = 1;
}

/* the children part of the rule, for a window to which (l, t, r, b) (its client coordinates) was just added */
static void upd_children(int64_t i, int32_t l, int32_t t, int32_t r, int32_t b, int erase)
{
    int64_t idx[SIB_MAX];
    uint32_t n, k;
    if (tab[i].style & FLE_WS_CLIPCHILDREN_) return;
    if (l < 0) l = 0;
    if (t < 0) t = 0;
    if (r > tab[i].pcx) r = tab[i].pcx;
    if (b > tab[i].pcy) b = tab[i].pcy;
    if (r <= l || b <= t) return;
    n = siblings(i, idx, SIB_MAX);
    for (k = 0; k < n; k++) {
        int64_t c = idx[k];
        int32_t cl, ct, cr, cb;

        if (!tab[c].live || !(tab[c].style & FLE_WS_VISIBLE) || tab[c].iconic) continue;
        cl = l > tab[c].px ? l : tab[c].px;
        ct = t > tab[c].py ? t : tab[c].py;
        cr = r < tab[c].px + tab[c].pcx ? r : tab[c].px + tab[c].pcx;
        cb = b < tab[c].py + tab[c].pcy ? b : tab[c].py + tab[c].pcy;
        if (cr <= cl || cb <= ct) continue;
        cl -= tab[c].px;
        cr -= tab[c].px;
        ct -= tab[c].py;
        cb -= tab[c].py;
        upd_add(c, cl, ct, cr, cb, erase);
        upd_children(c, cl, ct, cr, cb, erase);
    }
}

static void upd_invalidate(int64_t i, int32_t l, int32_t t, int32_t r, int32_t b, int erase)
{
    upd_add(i, l, t, r, b, erase);
    upd_children(i, l, t, r, b, erase);
}

/* ---- Decision 0214 (present2; PROVISIONAL; docs/0214-present-path.md): what the window manager does around a child window that is moved, sized, shown, hidden or restacked ----
 * Learn SetWindowPos: SWP_NOCOPYBITS "Discards the entire contents of the client area. If this flag is not specified, the valid contents of the client area are saved and copied back into the client area after the
 * window is sized or repositioned."; SWP_NOREDRAW "Does not redraw changes. If this flag is set, no repainting of any kind occurs. This applies to the client area, the nonclient area (including the title bar and scroll
 * bars), and any part of the parent window uncovered as a result of the window being moved. When this flag is set, the application must explicitly invalidate or redraw any parts of the window and parent window that need
 * redrawing." Learn BeginPaint / WM_PAINT ("The WM_PAINT Message": "The system sends this message to a window procedure when changes to the window have altered the content of the client area"; "The Update Region": "The
 * system adds only the invalid portion of the window to the update region"; BeginPaint: the update region is set "by the system after sizing, moving, creating, scrolling, or any other operation that affects the
 * client area"). So without SWP_NOREDRAW the parts of a window (the parent, the siblings beneath) that the change makes visible, and the parts of the moved window that the copy of its bits does not fill, are
 * invalid; before this decision only a SIZE change invalidated anything, so a child that only moved left its old picture standing in the parent's pixels (the 'trail' of a dragged FL panel) and its new place unpainted.
 * Model: for the windows of the changed window's tree that the old or new rectangle touches, the visible region (fle_win_dc_region, CLIPCHILDREN | CLIPSIBLINGS: where the window is the topmost one) is taken before
 * and after; a window's update rectangle (one bounding rectangle: S-updregion) is extended by (after minus before). For the moved window itself the bits of its old visible region (minus what was invalid) are copied to the
 * new place first (the part of the new visible region they reach counts as valid), as ScrollWindowEx does. Top-level windows are not touched: each has its own surface and the browser composes them (decision 0141).
 * S-0214-erase: the system-made invalid area asks for the background to be erased (upd_erase = 1; WM_ERASEBKGND comes from BeginPaint as for any other invalid area). */
#define UNC_MAX 160u
typedef struct {
    int64_t w;
    fle_visrgn v0;      /* visible region before (CLIPCHILDREN | CLIPSIBLINGS), surface coordinates */
    fle_visrgn v0n;     /* decision 0247b: the same with the children NOT clipped (CLIPSIBLINGS only); only for a window without WS_CLIPCHILDREN */
    fle_visrgn valid0n; /* v0n minus the pending update rectangles of the window and of its descendants (the bits that a copy may keep) */
    int has_n;
    fle_visrgn valid0;  /* the same minus the window's pending update rectangle: the bits that are valid */
    fle_visrgn cp0;     /* subject only: the region before with its children (CLIPSIBLINGS), minus the pending update rectangle of every window below it: what a copy of bits may take */
    int sub;            /* the window is the changed window or one of its descendants */
} unc_item;
typedef struct {
    int64_t subject, root;
    int32_t rx, ry;
    int32_t ox0, oy0;   /* the subject's client origin before the change (screen coordinates) */
    uint32_t n;
    unc_item *it;
    fle_rrect u;        /* the union of the changed window's old and new rectangle, surface coordinates */
    int degraded;       /* the snapshot is not complete (more windows than UNC_MAX touch u, or a region needed more than FLE_VISRGN_MAX rectangles): see unc_end */
} unc_snap;
static unc_snap *unc_carry;      /* a snapshot that ShowWindow took before it cleared WS_VISIBLE; apply_pos takes it over */

static int unc_in_tree(int64_t x, int64_t root) { return root_of(x) == root; }
static int unc_is_desc(int64_t x, int64_t a)
{
    for (; x >= 0; x = tab[x].parent)
        if (x == a) return 1;
    return 0;
}
static fle_rrect unc_rect(int64_t w, int32_t rx, int32_t ry)
{
    int32_t ox, oy;
    fle_rrect q;

    client_origin(w, &ox, &oy);
    q.x0 = (int64_t)ox - rx;
    q.y0 = (int64_t)oy - ry;
    q.x1 = q.x0 + tab[w].pcx;
    q.y1 = q.y0 + tab[w].pcy;
    return q;
}
static int unc_touch(fle_rrect a, fle_rrect b) { return a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1; }
/* the pending update rectangle of window w in surface coordinates (empty: x1 <= x0) */
static fle_rrect unc_pending(int64_t w, int32_t rx, int32_t ry)
{
    fle_rrect q = { 0, 0, 0, 0 };
    int32_t ox, oy;

    /* decision 0262 (alpha8; S-0247-hidden: an invisible window has no update region): the rectangle that a window still holds from the time it was visible (SetWindowPos(SWP_HIDEWINDOW) / ShowWindow(SW_HIDE) leave it
     * in place) is not a pending area. FL's TQuickScrollBoxScroller (lid 87) is shown, sized and hidden again before it is ever painted; its stale 1254 x 19 rectangle took 19 rows of the Mixer panel out of the bits that
     * a move may copy, and every move of the panel invalidated them again (Wine 10.0, harness/alpha8/move_paint.c: after the same calls only the strip of the parent that the move uncovered is invalid). */
    if (!is_visible(w)) return q;
    if (tab[w].upd_r <= tab[w].upd_l || tab[w].upd_b <= tab[w].upd_t) return q;
    client_origin(w, &ox, &oy);
    q.x0 = (int64_t)ox - rx + tab[w].upd_l;
    q.y0 = (int64_t)oy - ry + tab[w].upd_t;
    q.x1 = (int64_t)ox - rx + tab[w].upd_r;
    q.y1 = (int64_t)oy - ry + tab[w].upd_b;
    return q;
}
static int unc_ovf;     /* set when a region needed more than FLE_VISRGN_MAX rectangles (fle_win_dc_region answered -1) */
static int unc_region(int64_t w, uint32_t cf, fle_visrgn *out)
{
    int r = fle_win_dc_region(handle_of(w), cf, out);

    if (r < 0) unc_ovf = 1;
    if (r != 1) out->n = 0;
    return r == 1;
}

/* Takes the snapshot of the windows around `i` (before its change), or NULL when nothing is to be done: the window is top-level, hidden, in a minimised tree, or the region functions are not linked.
 * (nx, ny, ncx, ncy) is the rectangle the window is going to have (client coordinates of the parent). */
static unc_snap *unc_begin(int64_t i, int32_t nx, int32_t ny, int32_t ncx, int32_t ncy)
{
    unc_snap *sn;
    fle_rrect o, nw, u;
    int32_t px = 0, py = 0;
    uint64_t k;
    uint32_t n = 0;
    int64_t order[UNC_MAX];

    if (!fle_visrgn_set_rect || !fle_visrgn_subtract || !fle_visrgn_intersect_rect || !fle_visrgn_intersect || !fle_visrgn_bounds) return NULL;
    if (i < 0 || !tab[i].live || tab[i].parent < 0 || tab[i].msgonly || !is_visible(i)) return NULL;
    sn = (unc_snap *)calloc(1, sizeof *sn);
    if (!sn) return NULL;
    sn->subject = i;
    sn->root = root_of(i);
    sn->rx = tab[sn->root].px;
    sn->ry = tab[sn->root].py;
    if (tab[sn->root].iconic) { free(sn); return NULL; }
    o = unc_rect(i, sn->rx, sn->ry);
    client_origin(i, &sn->ox0, &sn->oy0);
    if (tab[i].parent >= 0) client_origin(tab[i].parent, &px, &py);
    nw.x0 = (int64_t)px + nx - sn->rx;
    nw.y0 = (int64_t)py + ny - sn->ry;
    nw.x1 = nw.x0 + ncx;
    nw.y1 = nw.y0 + ncy;
    u.x0 = o.x0 < nw.x0 ? o.x0 : nw.x0;
    u.y0 = o.y0 < nw.y0 ? o.y0 : nw.y0;
    u.x1 = o.x1 > nw.x1 ? o.x1 : nw.x1;
    u.y1 = o.y1 > nw.y1 ? o.y1 : nw.y1;
    unc_ovf = 0;
    sn->u = u;
    for (k = 0; k < tab_n; k++) {
        if (!tab[k].live || tab[k].msgonly || !unc_in_tree((int64_t)k, sn->root) || !is_visible((int64_t)k)) continue;
        if (!unc_touch(unc_rect((int64_t)k, sn->rx, sn->ry), u)) continue;
        if (n < UNC_MAX) order[n++] = (int64_t)k;
        else sn->degraded = 1;
    }
    sn->it = (unc_item *)calloc(n ? n : 1u, sizeof *sn->it);
    if (!sn->it) { free(sn); return NULL; }
    sn->n = n;
    for (k = 0; k < n; k++) {
        unc_item *it = &sn->it[k];
        fle_rrect pend;

        it->w = order[k];
        it->sub = unc_is_desc(it->w, i);
        unc_region(it->w, FLE_VR_CLIPCHILDREN | FLE_VR_CLIPSIBLINGS, &it->v0);
        it->has_n = !(tab[it->w].style & FLE_WS_CLIPCHILDREN_);
        if (it->has_n) {
            uint64_t d;

            unc_region(it->w, FLE_VR_CLIPSIBLINGS, &it->v0n);
            it->valid0n = it->v0n;
            pend = unc_pending(it->w, sn->rx, sn->ry);
            if (pend.x1 > pend.x0 && pend.y1 > pend.y0) fle_visrgn_subtract(&it->valid0n, pend);
            for (d = 0; d < tab_n; d++) {
                if (!tab[d].live || (int64_t)d == it->w || !unc_is_desc((int64_t)d, it->w)) continue;
                pend = unc_pending((int64_t)d, sn->rx, sn->ry);
                if (pend.x1 > pend.x0 && pend.y1 > pend.y0) fle_visrgn_subtract(&it->valid0n, pend);
            }
        }
        it->valid0 = it->v0;
        pend = unc_pending(it->w, sn->rx, sn->ry);
        if (pend.x1 > pend.x0 && pend.y1 > pend.y0) fle_visrgn_subtract(&it->valid0, pend);
        if (it->w == i) {
            unc_region(it->w, FLE_VR_CLIPSIBLINGS, &it->cp0);
            /* pending update rectangles inside the window (its own and its descendants'): those bits are not valid */
            {
                uint64_t d;

                for (d = 0; d < tab_n; d++) {
                    if (!tab[d].live || (int64_t)d == i || !unc_is_desc((int64_t)d, i)) continue;
                    pend = unc_pending((int64_t)d, sn->rx, sn->ry);
                    if (pend.x1 > pend.x0 && pend.y1 > pend.y0) fle_visrgn_subtract(&it->cp0, pend);
                }
            }
            pend = unc_pending(i, sn->rx, sn->ry);
            if (pend.x1 > pend.x0 && pend.y1 > pend.y0) fle_visrgn_subtract(&it->cp0, pend);
        }
    }
    if (unc_ovf) sn->degraded = 1;
    return sn;
}

static void unc_translate(fle_visrgn *v, int64_t dx, int64_t dy)
{
    uint32_t k;

    for (k = 0; k < v->n; k++) {
        v->r[k].x0 += dx;
        v->r[k].x1 += dx;
        v->r[k].y0 += dy;
        v->r[k].y1 += dy;
    }
}
/* a -= b */
static void unc_minus(fle_visrgn *a, const fle_visrgn *b)
{
    uint32_t k;

    for (k = 0; k < b->n && a->n; k++) fle_visrgn_subtract(a, b->r[k]);
}
/* the window's update rectangle grows by the bounding rectangle of `nv` (surface coordinates), kept inside its client rectangle */
static void unc_invalidate(int64_t w, const fle_visrgn *nv, int32_t rx, int32_t ry)
{
    fle_rrect b;
    int32_t ox, oy;
    int64_t l, t, r, bt;

    if (!nv->n) return;
    b = fle_visrgn_bounds(nv);
    client_origin(w, &ox, &oy);
    l = b.x0 - ((int64_t)ox - rx);
    t = b.y0 - ((int64_t)oy - ry);
    r = b.x1 - ((int64_t)ox - rx);
    bt = b.y1 - ((int64_t)oy - ry);
    if (l < 0) l = 0;
    if (t < 0) t = 0;
    if (r > tab[w].pcx) r = tab[w].pcx;
    if (bt > tab[w].pcy) bt = tab[w].pcy;
    if (r <= l || bt <= t) return;
    upd_invalidate(w, (int32_t)l, (int32_t)t, (int32_t)r, (int32_t)bt, 1);
}

/* The bits: the old visible region of the window (with its children), moved by (dx, dy), intersected with the new one; the rows are gathered first and then written, so that the source and destination may overlap. */
static void unc_copy_bits(const char *api, unc_snap *sn, const unc_item *subj, int64_t dx, int64_t dy, fle_visrgn *copied)
{
    fle_visrgn c1;
    const fle_rsurf *surf;
    uint32_t k, bpp;
    uint8_t *buf = NULL, *p;
    size_t total = 0;
    fle_rrect *rr;

    copied->n = 0;
    if (!fle_raster_surface_peek || !fle_raster_wrote || !fle_r_row || !fle_r_direct) return;
    surf = fle_raster_surface_peek(tab[sn->root].lid);
    if (!surf || !fle_r_direct(surf)) return;
    bpp = surf->bpp / 8u;
    *copied = subj->cp0;
    unc_translate(copied, dx, dy);
    if (!unc_region(sn->subject, FLE_VR_CLIPSIBLINGS, &c1)) { copied->n = 0; return; }
    fle_visrgn_intersect(copied, &c1);
    /* inside the surface */
    {
        fle_rrect sb = { 0, 0, (int64_t)surf->width, (int64_t)surf->height };

        fle_visrgn_intersect_rect(copied, sb);
        if (copied->n) {
            fle_visrgn s2 = *copied;

            unc_translate(&s2, -dx, -dy);
            fle_visrgn_intersect_rect(&s2, sb);
            unc_translate(&s2, dx, dy);
            fle_visrgn_intersect(copied, &s2);
        }
    }
    rr = copied->r;
    for (k = 0; k < copied->n; k++) total += (size_t)(rr[k].x1 - rr[k].x0) * (size_t)(rr[k].y1 - rr[k].y0) * bpp;
    if (!total) { copied->n = 0; return; }
    buf = (uint8_t *)malloc(total);
    if (!buf) { copied->n = 0; return; }
    p = buf;
    for (k = 0; k < copied->n; k++) {
        int64_t y, w = rr[k].x1 - rr[k].x0;

        for (y = rr[k].y0; y < rr[k].y1; y++) {
            memcpy(p, fle_r_row(surf, (uint32_t)(y - dy)) + (size_t)(rr[k].x0 - dx) * bpp, (size_t)w * bpp);
            p += (size_t)w * bpp;
        }
    }
    p = buf;
    for (k = 0; k < copied->n; k++) {
        int64_t y, w = rr[k].x1 - rr[k].x0;

        for (y = rr[k].y0; y < rr[k].y1; y++) {
            memcpy(fle_r_row(surf, (uint32_t)y) + (size_t)rr[k].x0 * bpp, p, (size_t)w * bpp);
            p += (size_t)w * bpp;
        }
        fle_raster_wrote(surf, rr[k], api);
    }
    free(buf);
}

/* A snapshot that is not complete (more windows touch the changed rectangle than a snapshot holds, or a visible region needs more rectangles than a fle_visrgn keeps) cannot say which part of which window the change uncovered. The
 * answer that stays inside Learn's SetWindowPos ("any part of the parent window uncovered as a result of the window being moved" is invalid) is the over-estimate: every visible window of the tree that the old or the new
 * rectangle touches is invalid in the part that the union covers, and no bits are copied (nothing is taken for valid). Invalid more than necessary only costs a repaint; invalid less leaves the old picture standing (decision 0240). */
static void unc_invalidate_union(const unc_snap *sn)
{
    uint64_t k;

    for (k = 0; k < tab_n; k++) {
        fle_rrect q;
        fle_visrgn nv;

        if (!tab[k].live || tab[k].msgonly || !unc_in_tree((int64_t)k, sn->root) || !is_visible((int64_t)k)) continue;
        q = unc_rect((int64_t)k, sn->rx, sn->ry);
        if (q.x0 < sn->u.x0) q.x0 = sn->u.x0;
        if (q.y0 < sn->u.y0) q.y0 = sn->u.y0;
        if (q.x1 > sn->u.x1) q.x1 = sn->u.x1;
        if (q.y1 > sn->u.y1) q.y1 = sn->u.y1;
        if (q.x1 <= q.x0 || q.y1 <= q.y0) continue;
        fle_visrgn_set_rect(&nv, q);
        unc_invalidate((int64_t)k, &nv, sn->rx, sn->ry);
    }
}

/* After the change: copies the bits (a child that moved; not with SWP_NOCOPYBITS) and invalidates what the change made visible. `redraw` = no SWP_NOREDRAW. Frees the snapshot. */
static void unc_end(const char *api, unc_snap *sn, int copy, int redraw)
{
    uint32_t k;
    fle_visrgn copied, v1, nv, old;
    int64_t dx = 0, dy = 0;
    int moved_copy = 0;
    const unc_item *subj = NULL;
    int32_t ox, oy;

    if (!sn) return;
    for (k = 0; k < sn->n; k++)
        if (sn->it[k].w == sn->subject) subj = &sn->it[k];
    copied.n = 0;
    if (tab[sn->subject].live) {
        client_origin(sn->subject, &ox, &oy);
        dx = (int64_t)ox - sn->ox0;
        dy = (int64_t)oy - sn->oy0;
    }
    unc_ovf = 0;
    if (copy && subj && !sn->degraded && (dx != 0 || dy != 0) && tab[sn->subject].live && is_visible(sn->subject)) {
        unc_copy_bits(api, sn, subj, dx, dy, &copied);
        moved_copy = copied.n > 0;
    }
    if (redraw) {
        int degraded = sn->degraded || unc_ovf;

        for (k = 0; k < sn->n && !degraded; k++) {
            const unc_item *it = &sn->it[k];

            if (!tab[it->w].live || !is_visible(it->w)) continue;
            if (!unc_region(it->w, FLE_VR_CLIPCHILDREN | FLE_VR_CLIPSIBLINGS, &v1)) {
                if (unc_ovf) degraded = 1;
                continue;
            }
            nv = v1;
            if (it->sub && (dx != 0 || dy != 0)) {
                /* the window moved with the subject: what was valid before and has been copied here is valid now; without a copy none of it is */
                if (moved_copy) {
                    old = it->valid0;
                    unc_translate(&old, dx, dy);
                    fle_visrgn_intersect(&old, &copied);
                    unc_minus(&nv, &old);
                }
            } else {
                unc_minus(&nv, &it->v0);
            }
            if (nv.n) unc_invalidate(it->w, &nv, sn->rx, sn->ry);
            /* decision 0247b (alpha3b2): a window that does not clip its children also repaints, over them, what a sibling above it (with its children) used to cover: the increase of its visible region with the children not clipped
             * (the guest's TEventEditForm draws its TNumBar itself and the TFXForm its tracks; TNumBar answers WM_PAINT with ValidateRgn(NULL)). The parent of a moved child gets the old place by the part above (decision 0214), as before. */
            if (it->has_n) {
                fle_visrgn v1n;

                if (!unc_region(it->w, FLE_VR_CLIPSIBLINGS, &v1n)) {
                    if (unc_ovf) degraded = 1;
                } else {
                    if (it->sub && (dx != 0 || dy != 0)) {
                        if (moved_copy) {
                            old = it->valid0n;
                            unc_translate(&old, dx, dy);
                            fle_visrgn_intersect(&old, &copied);
                            unc_minus(&v1n, &old);
                        }
                    } else {
                        unc_minus(&v1n, &it->v0n);
                    }
                    if (v1n.n) unc_invalidate(it->w, &v1n, sn->rx, sn->ry);
                }
            }
        }
        if (degraded) unc_invalidate_union(sn);
    }
    free(sn->it);
    free(sn);
}


/* Applies a position/size/z-order/show change to window i with the full message sequence of Learn (SetWindowPos, WM_WINDOWPOSCHANGING, WM_WINDOWPOSCHANGED, WM_SIZE, WM_MOVE, MoveWindow):
 *   1. WM_WINDOWPOSCHANGING (unless SWP_NOSENDCHANGING), lParam = WINDOWPOS; the procedure may change x, y, cx, cy, hwndInsertAfter and flags ("While this message is being processed, modifying any of the values
 *      in WINDOWPOS affects the window's new size, position, or place in the Z order"; "Changes to some SWP flags, including SWP_NOACTIVATE and SWP_NOOWNERZORDER, are ignored").
 *   2. the change itself: size/position unless SWP_NOSIZE / SWP_NOMOVE; the z-order unless SWP_NOZORDER (HWND_TOP, BOTTOM, TOPMOST, NOTOPMOST or a window); SWP_SHOWWINDOW / SWP_HIDEWINDOW. R4: WS_EX_TOPMOST is set by
 *      HWND_TOPMOST and cleared by HWND_NOTOPMOST and by placing a topmost window at the bottom or after a non-topmost window (Learn SetWindowPos); a window made topmost makes its owned windows topmost, and a window
 *      made non-topmost makes its owners and owned windows non-topmost.
 *   3. WM_NCCALCSIZE when the size changes or SWP_FRAMECHANGED (Learn SetWindowPos: "WM_NCCALCSIZE is sent only when the window's size is being changed"; MoveWindow's page lists WM_WINDOWPOSCHANGING,
 *      WM_WINDOWPOSCHANGED, WM_MOVE, WM_SIZE and WM_NCCALCSIZE and gives no order: sent BEFORE WM_WINDOWPOSCHANGED here: a choice without a source, S-order).
 *   4. WM_WINDOWPOSCHANGED, lParam = the WINDOWPOS as applied. DefWindowProcW answers it by sending WM_MOVE (when the position changed or SWP_FRAMECHANGED) and WM_SIZE (when the size changed) (Learn WM_WINDOWPOSCHANGED:
 *      "By default, the DefWindowProc function sends the WM_SIZE and WM_MOVE messages"). WM_MOVE lParam: the client origin, screen coordinates for a top-level window and parent client coordinates for a child
 *      (Learn WM_MOVE); WM_SIZE wParam SIZE_RESTORED (0), lParam the client width in the low word and the height in the high word (Learn WM_SIZE).
 * The WINDOWPOS lives on the guest stack below the API's frame. */
/* decision 0222: the position of window i among its siblings, top = 0 (-1 when it is not in its group); used to see whether SetWindowPos changed the z-order at all */
static int64_t sib_pos(int64_t i)
{
    int64_t idx[SIB_MAX];
    uint32_t n = siblings(tab[i].parent, idx, SIB_MAX), k;

    for (k = 0; k < n; k++)
        if (idx[k] == i) return (int64_t)k;
    return -1;
}
static void activate_for_swp(const char *api, int64_t i);
/* decision 0222: Learn Window Features / Owned Windows: "An owned window is always above its owner in the z-order"; SetWindowPos SWP_NOOWNERZORDER: "Does not change the owner window's position in the Z order" (so without it a
 * change of the z-order of a window carries its owner and its owned windows: Wine 10.0 win32u/window.c moves the owned windows with the owner, 'flags 0x2413' of win_act.c scenario 8). After window i was placed: i goes above
 * its owner(s) when it was put below, and the windows it owns are put above it (each directly above its owner, in the order they had). Only top-level windows (the group of the desktop). */
static uint32_t zl_index(int64_t w)
{
    uint32_t k;

    for (k = 0; k < zl_n; k++)
        if (zl[k] == w) return k;
    return zl_n;
}
static void owner_z_fix(int64_t i)
{
    int64_t o;
    uint64_t q;
    int64_t owned[64];
    uint32_t m = 0, a;

    if (tab[i].parent >= 0 || tab[i].msgonly) return;
    for (o = tab[i].owner_win; o >= 0 && tab[o].live && tab[o].parent < 0; o = tab[o].owner_win) {
        if (sib_pos(i) > sib_pos(o)) {
            uint32_t po;

            zl_remove(i);
            po = zl_index(o);
            zl_insert_at(po < zl_n ? po : zl_n, i);
        }
    }
    for (q = 0; q < tab_n && m < 64; q++)
        if (tab[q].live && tab[q].owner_win == i && tab[q].parent < 0) owned[m++] = (int64_t)q;
    for (a = 0; a < m; a++) {              /* the order among the owned windows is the order of the z list */
        int64_t k = owned[a];

        if (sib_pos(k) > sib_pos(i)) {
            uint32_t pi;

            zl_remove(k);
            pi = zl_index(i);
            zl_insert_at(pi < zl_n ? pi : zl_n, k);
        }
        owner_z_fix(k);
    }
}
/* decision 0222: set by SetWindowPos to 1 / 0 = "the window was / was not visible when the call began" (-1 = the caller has already set WS_VISIBLE itself: ShowWindow); apply_pos reads and clears it */
static int swp_vis0 = -1;

static void apply_pos(const char *api, int64_t i, uint64_t after, int32_t x, int32_t y, int32_t cx, int32_t cy, uint32_t flags)
{
    uint64_t sp = R[4], wp = (sp - 64u) & ~UINT64_C(15);
    uint64_t h = handle_of(i);
    int32_t ox0 = tab[i].px, oy0 = tab[i].py, ocx = tab[i].pcx, ocy = tab[i].pcy;
    int32_t nx, ny, ncx, ncy;
    int moved, sized;
    uint32_t nflags;
    uint64_t nafter;
    uint32_t zflags;
    unc_snap *usn;
    int vis0;
    int64_t zpos0;
    int ztop0;

    fle_st(wp, 8, h);
    fle_st(wp + 8, 8, after);
    put_i32(wp + 16, x);
    put_i32(wp + 20, y);
    put_i32(wp + 24, cx);
    put_i32(wp + 28, cy);
    fle_st(wp + 32, 4, flags);
    fle_st(wp + 36, 4, 0);
    R[4] = wp;
    if (!(flags & FLE_SWP_NOSENDCHANGING)) {
        uint64_t r = send_to(api, i, FLE_WM_WINDOWPOSCHANGING, 0, wp);

        if (r != 0) {
            char why[120];

            snprintf(why, sizeof why, "WM_WINDOWPOSCHANGING returned 0x%" PRIx64 ": the page says an application that processes it should return zero", r);
            fle_stop(api, why);
        }
    }
    nafter = fle_ld(wp + 8, 8);
    nx = (int32_t)fle_ld(wp + 16, 4);
    ny = (int32_t)fle_ld(wp + 20, 4);
    ncx = (int32_t)fle_ld(wp + 24, 4);
    ncy = (int32_t)fle_ld(wp + 28, 4);
    nflags = (uint32_t)fle_ld(wp + 32, 4);
    /* "Changes to some SWP flags, including SWP_NOACTIVATE and SWP_NOOWNERZORDER, are ignored when they are modified during WM_WINDOWPOSCHANGING": those two bits keep the value of the call. */
    nflags = (nflags & ~(FLE_SWP_NOACTIVATE | FLE_SWP_NOOWNERZORDER)) | (flags & (FLE_SWP_NOACTIVATE | FLE_SWP_NOOWNERZORDER));
    vis0 = swp_vis0;
    swp_vis0 = -1;
    zpos0 = sib_pos(i);
    ztop0 = (tab[i].exstyle & FLE_WS_EX_TOPMOST) != 0;
    {   /* decision 0214: the windows around this one, before the change (a snapshot that ShowWindow took before it cleared WS_VISIBLE comes first) */
        unc_snap *carried = unc_carry && unc_carry->subject == i ? unc_carry : NULL;

        if (carried) unc_carry = NULL;
        usn = carried ? carried : unc_begin(i, (nflags & FLE_SWP_NOMOVE) ? tab[i].px : nx, (nflags & FLE_SWP_NOMOVE) ? tab[i].py : ny, (nflags & FLE_SWP_NOSIZE) ? tab[i].pcx : ncx, (nflags & FLE_SWP_NOSIZE) ? tab[i].pcy : ncy);
    }
    moved = !(nflags & FLE_SWP_NOMOVE) && (nx != tab[i].px || ny != tab[i].py);
    sized = !(nflags & FLE_SWP_NOSIZE) && (ncx != tab[i].pcx || ncy != tab[i].pcy);
    if (!(nflags & FLE_SWP_NOMOVE)) {
        tab[i].px = nx;
        tab[i].py = ny;
    }
    if (!(nflags & FLE_SWP_NOSIZE)) {
        tab[i].pcx = ncx;
        tab[i].pcy = ncy;
    }
    if (!tab[i].iconic && !tab[i].zoomed) {
        tab[i].rest_x = tab[i].px;
        tab[i].rest_y = tab[i].py;
        tab[i].rest_cx = tab[i].pcx;
        tab[i].rest_cy = tab[i].pcy;
    }
    tab[i].x = tab[i].px;
    tab[i].y = tab[i].py;
    tab[i].cx = tab[i].pcx;
    tab[i].cy = tab[i].pcy;
    zflags = nflags;
    if (!(zflags & FLE_SWP_NOZORDER) && !tab[i].msgonly) {
        int64_t target = -1;

        if (nafter == FLE_HWND_TOPMOST) {
            tab[i].exstyle |= FLE_WS_EX_TOPMOST;
            z_to_top(i);
            /* "When a non-topmost window is made topmost, its owned windows are also made topmost. Its owners, however, are not changed." */
            {
                uint64_t k;

                for (k = 0; k < tab_n; k++)
                    if (tab[k].live && tab[k].owner_win == i && !(tab[k].exstyle & FLE_WS_EX_TOPMOST)) {
                        tab[k].exstyle |= FLE_WS_EX_TOPMOST;
                        z_to_top((int64_t)k);
                    }
            }
        } else if (nafter == FLE_HWND_NOTOPMOST) {
            /* "Places the window above all non-topmost windows (that is, behind all topmost windows). This flag has no effect if the window is already a non-topmost window." */
            if (tab[i].exstyle & FLE_WS_EX_TOPMOST) {
                tab[i].exstyle &= ~FLE_WS_EX_TOPMOST;
                z_to_top(i);
                /* "When a topmost window is made non-topmost, its owners and its owned windows are also made non-topmost windows." */
                {
                    uint64_t k;
                    int64_t o = tab[i].owner_win;

                    if (o >= 0 && (tab[o].exstyle & FLE_WS_EX_TOPMOST)) {
                        tab[o].exstyle &= ~FLE_WS_EX_TOPMOST;
                        z_to_top(o);
                    }
                    for (k = 0; k < tab_n; k++)
                        if (tab[k].live && tab[k].owner_win == i && (tab[k].exstyle & FLE_WS_EX_TOPMOST)) {
                            tab[k].exstyle &= ~FLE_WS_EX_TOPMOST;
                            z_to_top((int64_t)k);
                        }
                }
            }
        } else if (nafter == FLE_HWND_TOP) {
            z_to_top(i);
        } else if (nafter == FLE_HWND_BOTTOM) {
            /* "If the hWnd parameter identifies a topmost window, the window loses its topmost status and is placed at the bottom of all other windows." */
            tab[i].exstyle &= ~FLE_WS_EX_TOPMOST;
            z_to_bottom(i);
        } else {
            target = live_index(nafter);
            if (target >= 0 && target != i && tab[target].parent == tab[i].parent) {
                /* "If a topmost window is repositioned ... after any non-topmost window, it is no longer topmost." */
                if ((tab[i].exstyle & FLE_WS_EX_TOPMOST) && !(tab[target].exstyle & FLE_WS_EX_TOPMOST)) tab[i].exstyle &= ~FLE_WS_EX_TOPMOST;
                z_below(i, target);
            } else if (target < 0) {
                fle_stop(api, "hWndInsertAfter is not a window of this table and not one of HWND_TOP, HWND_BOTTOM, HWND_TOPMOST, HWND_NOTOPMOST: the page gives no behaviour");
            }
        }
    }
    if (!(zflags & FLE_SWP_NOZORDER) && !(flags & FLE_SWP_NOOWNERZORDER) && !tab[i].msgonly) owner_z_fix(i);
    if (nflags & FLE_SWP_SHOWWINDOW) tab[i].style |= FLE_WS_VISIBLE, tab[i].visible = 1, tab[i].ever_shown = 1;
    if (nflags & FLE_SWP_HIDEWINDOW) tab[i].style &= ~FLE_WS_VISIBLE, tab[i].visible = 0;
    if (sized || (nflags & FLE_SWP_FRAMECHANGED)) {
        /* WM_NCCALCSIZE with wParam FALSE: lParam a RECT, the proposed window rectangle on entry (Learn); the answer must be 0. The window rectangle is in the coordinates of the parent's client area or the screen: the same
         * numbers as the stored position. */
        uint64_t rect = (wp - 16u) & ~UINT64_C(15);
        uint64_t r;

        put_i32(rect, tab[i].px);
        put_i32(rect + 4, tab[i].py);
        put_i32(rect + 8, tab[i].px + tab[i].pcx);
        put_i32(rect + 12, tab[i].py + tab[i].pcy);
        R[4] = rect;
        r = send_to(api, i, FLE_WM_NCCALCSIZE, 0, rect);
        if (r != 0) {
            char why[120];

            snprintf(why, sizeof why, "WM_NCCALCSIZE (wParam FALSE) returned 0x%" PRIx64 ": the page says the application should return zero", r);
            fle_stop(api, why);
        }
        R[4] = wp;
    }
    /* Decision 0132 (PROVISIONAL), wall 21 (S-swp-unchanged-geometry). A call that does not change the size (the position) of the window reports SWP_NOSIZE (SWP_NOMOVE) in the WINDOWPOS of WM_WINDOWPOSCHANGED, so that DefWindowProc
     * sends no WM_SIZE (WM_MOVE) for it. Learn (WM_SIZE): "Sent to a window after its size has changed"; (WM_MOVE): "Sent after a window has been moved"; (WM_WINDOWPOSCHANGED): "Sent to a window whose size, position, or place in the Z
     * order has changed". Without it the guest's own WM_SIZE handler, which calls SetWindowPos with the size it already has (the Delphi control: SetWindowPos(h, 0, 0, 0, cx, cy, SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOMOVE)),
     * is entered again by every call, 16 callbacks deep (the stop of decision 0064). That the fix-up is made in the flags, as the operating system does, is a reading of its behaviour (not on a Learn page). FRAMECHANGED still sends both. */
    if (!sized) nflags |= FLE_SWP_NOSIZE;
    if (!moved) nflags |= FLE_SWP_NOMOVE;
    /* decision 0222 (replaces decision 0132's S-swp-unchanged-geometry, which sent WM_WINDOWPOSCHANGED for every call): Learn WM_WINDOWPOSCHANGED "Sent to a window whose size, position, or place in the Z order has
     * changed as a result of a call to the SetWindowPos function or another window-management function"; Wine 10.0 (win32u/window.c fixup_swp_flags, set_window_pos) takes the flags of what really changed
     * (already the right size -> SWP_NOSIZE, already the right position -> SWP_NOMOVE, already in place in the z-order -> SWP_NOZORDER, SWP_SHOWWINDOW cleared when the window is visible, SWP_HIDEWINDOW
     * cleared when it is not) and sends WM_WINDOWPOSCHANGED "unless (flags & (FRAMECHANGED | HIDEWINDOW | SHOWWINDOW | NOSIZE | NOMOVE | NOZORDER)) == (NOSIZE | NOMOVE | NOZORDER)". (win_act.c, win_seq.c) */
    if (zflags & FLE_SWP_NOZORDER || (sib_pos(i) == zpos0 && ((tab[i].exstyle & FLE_WS_EX_TOPMOST) != 0) == ztop0)) nflags |= FLE_SWP_NOZORDER;
    if (vis0 == 1) nflags &= ~FLE_SWP_SHOWWINDOW;
    else if (vis0 == 0) nflags &= ~FLE_SWP_HIDEWINDOW;
    fle_st(wp, 8, h);
    fle_st(wp + 8, 8, nafter);
    put_i32(wp + 16, tab[i].px);
    put_i32(wp + 20, tab[i].py);
    put_i32(wp + 24, tab[i].pcx);
    put_i32(wp + 28, tab[i].pcy);
    /* Wine 10.0 set_window_pos: WM_WINDOWPOSCHANGED carries SWP_NOCLIENTMOVE / SWP_NOCLIENTSIZE (0x1000 / 0x0800) unless the client area moved / changed its size (txt1's create_seq.c: flags=0x1897 for a hide; UNOFFICIAL; the windows
     * of this model have no non-client area, so the client area moves and changes size with the window) */
    fle_st(wp + 32, 4, nflags | (moved ? 0u : 0x1000u) | (sized ? 0u : 0x0800u));
    fle_st(wp + 36, 4, 0);
    /* Learn SWP_NOACTIVATE: "Does not activate the window. If this flag is not set, the window is activated and moved to the top of either the topmost or non-topmost group"; Window Features: "Other functions can cause the
     * system to activate a different top-level window, including SetWindowPos, DeferWindowPos, SetWindowPlacement, and DestroyWindow". A child window is not activated; it gets Learn WM_CHILDACTIVATE ("Sent to a child
     * window when the user clicks the window's title bar or when the window is activated, moved, or sized"; Wine 10.0 sends it from SetWindowPos for a child without SWP_NOACTIVATE, win32u/window.c: the page names the cases, not the
     * call: S-childactivate-from-swp). The activation comes before WM_WINDOWPOSCHANGED (Wine 10.0: the order of set_window_pos; UNOFFICIAL). A window that is hidden by the call is not activated. */
    if (!(flags & (FLE_SWP_NOACTIVATE | FLE_SWP_HIDEWINDOW))) {
        if (tab[i].parent >= 0 && !(tab[i].style & FLE_WS_POPUP)) (void)send_to(api, i, 0x0022u, 0, 0);
        else activate_for_swp(api, i);
    }
    if ((nflags & (FLE_SWP_FRAMECHANGED | FLE_SWP_HIDEWINDOW | FLE_SWP_SHOWWINDOW | FLE_SWP_NOSIZE | FLE_SWP_NOMOVE | FLE_SWP_NOZORDER)) != (FLE_SWP_NOSIZE | FLE_SWP_NOMOVE | FLE_SWP_NOZORDER))
        (void)send_to(api, i, FLE_WM_WINDOWPOSCHANGED, 0, wp);
    R[4] = sp;
    (void)ox0;
    (void)oy0;
    (void)ocx;
    (void)ocy;
    if (sized || moved || (nflags & (FLE_SWP_SHOWWINDOW | FLE_SWP_FRAMECHANGED))) {
        /* the update region: a size change invalidates the whole client area (Learn BeginPaint: "The update region is set ... by the system after sizing, moving, creating, scrolling") */
        if (!(nflags & FLE_SWP_NOREDRAW) && sized && is_visible(i)) {
            upd_set_full(i);
        }
    }
    if ((nflags & FLE_SWP_SHOWWINDOW) && vis0 == 0 && !(nflags & FLE_SWP_NOREDRAW) && is_visible(i) && tab[i].pcx > 0 && tab[i].pcy > 0) {
        /* decision 0319 section 23: SetWindowPos(SWP_SHOWWINDOW) on a window that was not visible exposes the whole window and all its visible descendants (Wine 10.0 server/window.c set_window_pos: the old visible
         * region was empty, so everything is exposed: redraw_window(..., RDW_ALLCHILDREN)). ShowWindow does the same for itself in fle_imp_ShowWindow. */
        upd_set_full(i);
        tab[i].upd_erase = 1;
        upd_show_descendants(i);
    }
    if (nflags & FLE_SWP_SHOWWINDOW) note_visibility_changed();
    unc_end(api, usn, !(flags & FLE_SWP_NOCOPYBITS), !(nflags & FLE_SWP_NOREDRAW));
}

/* ---- Decision 0319 (PROVISIONAL; PM 07:2x; O9 'Settings -> General: scrolling breaks the picture, only the hovered element is repaired'): the bits and the update area of user32!ScrollWindowEx / ScrollWindow on the window model of decision 0214 ----
 * Learn ScrollWindowEx: scrolls "the contents of the specified window's client area"; "If the window has the WS_CLIPCHILDREN style, the returned areas specified by hrgnUpdate and prcUpdate represent the total area of the
 * scrolled window that must be updated, including any areas in child windows that need updating"; SW_SCROLLCHILDREN "Scrolls all child windows that intersect the rectangle pointed to by the prcScroll parameter ... The system
 * sends a WM_MOVE message to all child windows that intersect the prcScroll rectangle, even if they do not move"; SW_INVALIDATE / SW_ERASE invalidate "the area that is scrolled from"; all coordinates are client coordinates.
 * Learn ScrollWindow (Remarks): "If the lpRect parameter is NULL, the positions of any child windows in the window are offset by the amount specified by the XAmount and YAmount parameters; invalid (unpainted) areas in the window
 * are also offset. ... If lpRect is not NULL, the positions of child windows are not changed and invalid areas in the window are not offset."
 * Learn ScrollDC (lprcClip): "If lprcClip is NULL, the entire client area is used."
 * Where the pages are silent the reference is Wine 10 (UNOFFICIAL; probes in docs/0319 section 1.5, harness scroll_contract.c): (T3) prcClip NULL with a partial prcScroll means the CLIENT AREA, bits are moved beyond the scroll rectangle;
 * (T2a) a WS_CLIPCHILDREN parent does not move the pixels of a child (they are not its bits) and the destination of bits whose source was hidden by the child is invalid, the update area reported is 0,0..300,190 for a 300x300 window, a
 * child at 100,100 60x60 and dy 30; (T2b) without WS_CLIPCHILDREN the child's pixels move with the parent's bits; (T1/T4) with a pending update rectangle and prcScroll NULL, the update region afterwards holds the old rectangle, its
 * moved copy and the uncovered strip; hrgnUpdate's box is the union, prcUpdate is the bounding rectangle of the uncovered strip only; the return value is SIMPLEREGION for one uncovered rectangle.
 * Model: the window's visible region with its own clip style (CLIPSIBLINGS, and CLIPCHILDREN when the window has WS_CLIPCHILDREN) is the set of bits that exist (decision 0214's fle_win_dc_region); the bits of (scroll rectangle
 * inside it) move by (dx, dy) into (clip rectangle inside the visible region); what the scroll rectangle held there and the move did not cover is the uncovered area; with prcScroll NULL the pending update rectangle of the
 * window moves with the bits and stays where it was (Wine). Before this decision the pixels of the whole top-level surface rectangle moved (children and siblings included) and a pending update rectangle was not touched
 * (S2, S4 of docs/0319). The update area of a window is one bounding rectangle (S-updregion), so each invalidation adds the bounding rectangle of its region. */
static void scroll_copy_region(const char *api, int64_t root, const fle_visrgn *d, int64_t dx, int64_t dy)
{
    const fle_rsurf *surf;
    fle_visrgn c = *d, s2;
    uint32_t k, bpp;
    uint8_t *buf, *p;
    size_t total = 0;
    fle_rrect sb;

    if (!fle_raster_surface_peek || !fle_raster_wrote || !fle_r_row || !fle_r_direct) return;
    surf = fle_raster_surface_peek(tab[root].lid);
    if (!surf || !fle_r_direct(surf)) return;
    bpp = surf->bpp / 8u;
    sb.x0 = 0; sb.y0 = 0; sb.x1 = (int64_t)surf->width; sb.y1 = (int64_t)surf->height;
    fle_visrgn_intersect_rect(&c, sb);
    if (!c.n) return;
    s2 = c;
    unc_translate(&s2, -dx, -dy);
    fle_visrgn_intersect_rect(&s2, sb);
    unc_translate(&s2, dx, dy);
    fle_visrgn_intersect(&c, &s2);
    for (k = 0; k < c.n; k++) total += (size_t)(c.r[k].x1 - c.r[k].x0) * (size_t)(c.r[k].y1 - c.r[k].y0) * bpp;
    if (!total) return;
    buf = (uint8_t *)malloc(total);
    if (!buf) return;
    p = buf;
    for (k = 0; k < c.n; k++) {   /* gather first, then write: the source and destination may overlap */
        int64_t y, w = c.r[k].x1 - c.r[k].x0;

        for (y = c.r[k].y0; y < c.r[k].y1; y++) {
            memcpy(p, fle_r_row(surf, (uint32_t)(y - dy)) + (size_t)(c.r[k].x0 - dx) * bpp, (size_t)w * bpp);
            p += (size_t)w * bpp;
        }
    }
    p = buf;
    for (k = 0; k < c.n; k++) {
        int64_t y, w = c.r[k].x1 - c.r[k].x0;

        for (y = c.r[k].y0; y < c.r[k].y1; y++) {
            memcpy(fle_r_row(surf, (uint32_t)y) + (size_t)c.r[k].x0 * bpp, p, (size_t)w * bpp);
            p += (size_t)w * bpp;
        }
        fle_raster_wrote(surf, c.r[k], api);
    }
    free(buf);
}

/* the bounding rectangle of `v` in the client coordinates of the window whose client origin is (cx0, cy0) in surface coordinates; 0 when v is empty */
static int scroll_bounds_client(const fle_visrgn *v, int64_t cx0, int64_t cy0, int32_t out[4])
{
    fle_rrect b;

    if (!v->n) return 0;
    b = fle_visrgn_bounds(v);
    out[0] = (int32_t)(b.x0 - cx0); out[1] = (int32_t)(b.y0 - cy0); out[2] = (int32_t)(b.x1 - cx0); out[3] = (int32_t)(b.y1 - cy0);
    return 1;
}

int fle_win_scroll(const char *api, uint64_t hwnd, int32_t dx, int32_t dy, const int32_t *scroll, const int32_t *clip, uint32_t flags, fle_win_scroll_result *res)
{
    int64_t i = live_index(hwnd), root;
    int32_t ox, oy, rx, ry, bb[4];
    fle_rrect cl, sr, kr, pend;
    fle_visrgn vis, src, dgeo, geo, mv;
    uint32_t cf;
    uint64_t d;
    int has_pending = 0;

    memset(res, 0, sizeof *res);
    if (i < 0) { res->invalid = 1; return 1; }
    if (!fle_visrgn_set_rect || !fle_visrgn_subtract || !fle_visrgn_intersect_rect || !fle_visrgn_intersect || !fle_visrgn_bounds) return 0;
    root = root_of(i);
    if (tab[i].msgonly || tab[root].iconic) return 0;
    rx = tab[root].px; ry = tab[root].py;
    client_origin(i, &ox, &oy);
    cl.x0 = (int64_t)ox - rx; cl.y0 = (int64_t)oy - ry; cl.x1 = cl.x0 + tab[i].pcx; cl.y1 = cl.y0 + tab[i].pcy;
    sr = cl;
    if (scroll) {
        fle_rrect s;
        s.x0 = cl.x0 + scroll[0]; s.y0 = cl.y0 + scroll[1]; s.x1 = cl.x0 + scroll[2]; s.y1 = cl.y0 + scroll[3];
        if (s.x0 < sr.x0) s.x0 = sr.x0;
        if (s.y0 < sr.y0) s.y0 = sr.y0;
        if (s.x1 > sr.x1) s.x1 = sr.x1;
        if (s.y1 > sr.y1) s.y1 = sr.y1;
        if (s.x1 < s.x0) s.x1 = s.x0;
        if (s.y1 < s.y0) s.y1 = s.y0;
        sr = s;
    }
    kr = cl;   /* prcClip NULL: the whole client area (ScrollDC page; Wine probe T3), not the scroll rectangle */
    if (clip) {
        fle_rrect c;
        c.x0 = cl.x0 + clip[0]; c.y0 = cl.y0 + clip[1]; c.x1 = cl.x0 + clip[2]; c.y1 = cl.y0 + clip[3];
        if (c.x0 < kr.x0) c.x0 = kr.x0;
        if (c.y0 < kr.y0) c.y0 = kr.y0;
        if (c.x1 > kr.x1) c.x1 = kr.x1;
        if (c.y1 > kr.y1) c.y1 = kr.y1;
        if (c.x1 < c.x0) c.x1 = c.x0;
        if (c.y1 < c.y0) c.y1 = c.y0;
        kr = c;
    }
    unc_ovf = 0;
    cf = FLE_VR_CLIPSIBLINGS | ((tab[i].style & FLE_WS_CLIPCHILDREN_) ? FLE_VR_CLIPCHILDREN : 0u);
    vis.n = 0;
    if (is_visible(i) && !unc_region(i, cf, &vis) && unc_ovf) return 0;   /* a region needed more rectangles than a fle_visrgn keeps: the caller falls back */
    /* the bits that exist: (scroll rectangle) inside the visible region */
    src = vis;
    fle_visrgn_intersect_rect(&src, sr);
    /* where they land: moved by (dx, dy), inside the clip rectangle and the visible region */
    dgeo = src;
    unc_translate(&dgeo, dx, dy);
    fle_visrgn_intersect_rect(&dgeo, kr);
    fle_visrgn_intersect(&dgeo, &vis);
    /* the uncovered area: what the scroll rectangle held inside the clip rectangle and the move did not cover */
    geo = src;
    fle_visrgn_intersect_rect(&geo, kr);
    unc_minus(&geo, &dgeo);
    /* the pending update rectangle moves with the bits (ScrollWindowEx: Wine probes U1 with prcScroll NULL and U2 with prcScroll = the client rectangle give the same update region); it stays where it was too (Wine). ScrollWindow with an
     * lpRect does not offset the invalid areas (Learn ScrollWindow, Remarks): the caller says so with the internal bit 0x80000000 (not an SW_ flag) */
    mv.n = 0;
    if (!(flags & 0x80000000u) && is_visible(i)) {
        pend = unc_pending(i, rx, ry);
        has_pending = pend.x1 > pend.x0 && pend.y1 > pend.y0;
        if (has_pending) {
            fle_visrgn_set_rect(&mv, pend);
            fle_visrgn_intersect_rect(&mv, sr);
            unc_translate(&mv, dx, dy);
            fle_visrgn_intersect_rect(&mv, kr);
            fle_visrgn_intersect(&mv, &vis);
        }
    }
    /* the bits */
    if ((dx != 0 || dy != 0) && dgeo.n) scroll_copy_region(api, root, &dgeo, dx, dy);
    /* the answer */
    res->parts = (int)geo.n;
    if (scroll_bounds_client(&geo, cl.x0, cl.y0, bb)) {
        res->unc[0] = bb[0]; res->unc[1] = bb[1]; res->unc[2] = bb[2]; res->unc[3] = bb[3];
        res->upd[0] = bb[0]; res->upd[1] = bb[1]; res->upd[2] = bb[2]; res->upd[3] = bb[3];
    }
    if (mv.n && scroll_bounds_client(&mv, cl.x0, cl.y0, bb)) {
        if (res->parts == 0) { res->upd[0] = bb[0]; res->upd[1] = bb[1]; res->upd[2] = bb[2]; res->upd[3] = bb[3]; }
        else {
            if (bb[0] < res->upd[0]) res->upd[0] = bb[0];
            if (bb[1] < res->upd[1]) res->upd[1] = bb[1];
            if (bb[2] > res->upd[2]) res->upd[2] = bb[2];
            if (bb[3] > res->upd[3]) res->upd[3] = bb[3];
        }
    }
    /* the invalid areas: the moved pending rectangle always (its bits are stale), the uncovered area with SW_INVALIDATE / SW_ERASE */
    if (mv.n && scroll_bounds_client(&mv, cl.x0, cl.y0, bb)) upd_invalidate(i, bb[0], bb[1], bb[2], bb[3], 1);
    if ((flags & 0x6u) != 0 && geo.n && scroll_bounds_client(&geo, cl.x0, cl.y0, bb)) upd_invalidate(i, bb[0], bb[1], bb[2], bb[3], (flags & 0x4u) ? 1 : 0);
    /* SW_SCROLLCHILDREN: the child windows that intersect the scroll rectangle move with it (WM_WINDOWPOSCHANGED, WM_MOVE; the pixels moved above when the window does not clip its children), and every child that
     * intersects gets a WM_MOVE even when it does not move (Learn) */
    if (flags & 0x1u) {
        for (d = 0; d < tab_n; d++) {
            fle_rrect cr;
            uint64_t sv;

            if (!tab[d].live || tab[d].parent != i) continue;
            cr.x0 = cl.x0 + tab[d].px; cr.y0 = cl.y0 + tab[d].py; cr.x1 = cr.x0 + tab[d].pcx; cr.y1 = cr.y0 + tab[d].pcy;
            if (!unc_touch(cr, sr)) continue;
            if (dx == 0 && dy == 0) {
                (void)send_to(api, (int64_t)d, FLE_WM_MOVE, 0, ((uint64_t)(uint16_t)tab[d].py << 16) | (uint16_t)tab[d].px);
                continue;
            }
            sv = R[4];
            apply_pos(api, (int64_t)d, 0, tab[d].px + dx, tab[d].py + dy, tab[d].pcx, tab[d].pcy, FLE_SWP_NOSIZE | FLE_SWP_NOZORDER | FLE_SWP_NOACTIVATE | FLE_SWP_NOREDRAW | FLE_SWP_NOCOPYBITS);
            R[4] = sv;
            if (cf & FLE_VR_CLIPCHILDREN) upd_invalidate((int64_t)d, 0, 0, tab[d].pcx, tab[d].pcy, 1);   /* the parent's bits did not carry the child's pixels */
        }
    }
    return 1;
}

/* DefWindowProcW(WM_WINDOWPOSCHANGED) sends WM_MOVE and WM_SIZE (Learn WM_WINDOWPOSCHANGED, WM_SIZE, WM_MOVE). Called from fle_imp_DefWindowProcW. */
static void defproc_windowposchanged(const char *api, int64_t i, uint64_t wpa)
{
    int32_t x = (int32_t)fle_ld(wpa + 16, 4), y = (int32_t)fle_ld(wpa + 20, 4), cx = (int32_t)fle_ld(wpa + 24, 4), cy = (int32_t)fle_ld(wpa + 28, 4);
    uint32_t flags = (uint32_t)fle_ld(wpa + 32, 4);
    int32_t ox, oy;
    uint64_t lp;

    (void)x;
    (void)y;
    /* WM_MOVE: lParam = the client origin: screen coordinates for an overlapped or pop-up window and the parent's client coordinates for a child (Learn WM_MOVE) = the stored position (non-client area 0) */
    if (!(flags & FLE_SWP_NOMOVE) || (flags & FLE_SWP_FRAMECHANGED)) {
        ox = tab[i].px;
        oy = tab[i].py;
        lp = ((uint64_t)(uint16_t)oy << 16) | (uint16_t)ox;
        (void)call_wndproc(api, tab[i].wndproc, handle_of(i), FLE_WM_MOVE, 0, lp);
    }
    if (!(flags & FLE_SWP_NOSIZE) || (flags & FLE_SWP_FRAMECHANGED)) {
        /* WM_SIZE: wParam the kind of resizing (SIZE_RESTORED 0, SIZE_MINIMIZED 1, SIZE_MAXIMIZED 2), lParam the client width (low word) and height (high word) (Learn WM_SIZE: "only the low-order 16 bits of each") */
        uint64_t kind = tab[i].iconic ? 1u : tab[i].zoomed ? 2u : 0u;

        lp = ((uint64_t)(uint16_t)cy << 16) | (uint16_t)cx;
        (void)call_wndproc(api, tab[i].wndproc, handle_of(i), FLE_WM_SIZE, kind, lp);
    }
}

/* user32!SetWindowPos: BOOL f(HWND rcx, HWND hWndInsertAfter rdx, int X r8, int Y r9, int cx [5], int cy [6], UINT uFlags [7]). See apply_pos. SWP_ASYNCWINDOW: "If the calling thread and the thread that owns the window
 * are attached to different input queues, the system posts the request to the thread that owns the window": a window of another thread with SWP_ASYNCWINDOWPOS stops (not modelled). Activation: unless SWP_NOACTIVATE or
 * SWP_NOZORDER, "the window is activated and moved to the top of either the topmost or non-topmost group": the model activates it (see activate). An invalid hWnd: 0 and 1400. */
static void activate_for_swp(const char *api, int64_t i);

void fle_imp_SetWindowPos(void)
{
    static const char api[] = "SetWindowPos";
    uint64_t h = R[1], after = R[2];
    int32_t x = (int32_t)(uint32_t)R[8], y = (int32_t)(uint32_t)R[9], cx = (int32_t)(uint32_t)fle_arg(5), cy = (int32_t)(uint32_t)fle_arg(6);
    uint32_t flags = (uint32_t)fle_arg(7);
    int64_t i;

    fle_win_check_thread(api);
    if (h == FLE_DESKTOP_HANDLE) fle_stop(api, "hWnd is the desktop window: the page does not say what moving it does");
    i = win_arg(h);
    if (i < 0) return;
    if ((flags & FLE_SWP_ASYNCWINDOWPOS) && fle_thread_current && tab[i].owner != fle_thread_current()) fle_stop(api, "SWP_ASYNCWINDOWPOS for a window of another thread: not modelled");
    if (flags & ~(FLE_SWP_NOSIZE | FLE_SWP_NOMOVE | FLE_SWP_NOZORDER | FLE_SWP_NOREDRAW | FLE_SWP_NOACTIVATE | FLE_SWP_FRAMECHANGED | FLE_SWP_SHOWWINDOW | FLE_SWP_HIDEWINDOW | FLE_SWP_NOCOPYBITS |
                  FLE_SWP_NOOWNERZORDER | FLE_SWP_NOSENDCHANGING | FLE_SWP_DEFERERASE | FLE_SWP_ASYNCWINDOWPOS))
        fle_stop(api, "uFlags has a bit that the page does not list");
    if ((flags & FLE_SWP_SHOWWINDOW) && (flags & FLE_SWP_HIDEWINDOW)) fle_stop(api, "SWP_SHOWWINDOW and SWP_HIDEWINDOW together: the page does not say");
    swp_vis0 = (tab[i].style & FLE_WS_VISIBLE) != 0;
    apply_pos(api, i, after, x, y, cx, cy, flags);
    R[0] = 1;
}

/* user32!MoveWindow: BOOL f(HWND hWnd rcx, int X edx, int Y r8d, int nWidth r9d, int nHeight [5], BOOL bRepaint [6]). Learn: "MoveWindow sends the WM_WINDOWPOSCHANGING, WM_WINDOWPOSCHANGED, WM_MOVE, WM_SIZE, and WM_NCCALCSIZE messages to the
 * window"; bRepaint TRUE: "the system sends the WM_PAINT message to the window procedure immediately after moving the window (that is, the MoveWindow function calls the UpdateWindow function)"; FALSE: no repainting of
 * any kind. It is SetWindowPos with SWP_NOZORDER | SWP_NOACTIVATE (a reading: the page does not say which flags it uses; a choice) and no show or hide. */
void fle_imp_MoveWindow(void)
{
    static const char api[] = "MoveWindow";
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8], w = (int32_t)(uint32_t)R[9];
    int32_t hgt = (int32_t)(uint32_t)fle_arg(5);
    uint32_t repaint = (uint32_t)fle_arg(6);
    int64_t i;

    fle_win_check_thread(api);
    i = win_arg(R[1]);
    if (i < 0) return;
    apply_pos(api, i, 0, x, y, w, hgt, FLE_SWP_NOZORDER | FLE_SWP_NOACTIVATE | (repaint ? 0u : FLE_SWP_NOREDRAW));
    if (repaint) {
        R[1] = handle_of(i);
        fle_imp_UpdateWindow();
    }
    R[0] = 1;
}

/* user32!ShowWindow: BOOL f(HWND rcx, int nCmdShow edx). Learn: the return value is nonzero if the window was previously visible and zero if it was previously hidden. SW_HIDE (0) hides the window and activates another
 * window; SW_SHOWNORMAL (1) / SW_RESTORE (9) activate and display it (restoring a minimized or maximized window); SW_SHOWMINIMIZED (2) / SW_MINIMIZE (6) / SW_SHOWMINNOACTIVE (7) minimize; SW_SHOWMAXIMIZED (3) maximizes; SW_SHOWNOACTIVATE (4),
 * SW_SHOWNA (8) show without activating; SW_SHOW (5) activates and displays in the current size and position. SW_SHOWDEFAULT (10) and SW_FORCEMINIMIZE (11) are not modelled (stop). "This parameter is ignored the first time an
 * application calls ShowWindow, if the program that launched the application provides a STARTUPINFO structure": GetStartupInfoW answers dwFlags 0 here (fle_env.c), so STARTF_USESHOWWINDOW is not set and nCmdShow is used
 * as given. WM_SHOWWINDOW (wParam TRUE or FALSE, lParam 0) is sent when the visibility changes, except for SW_SHOWNORMAL (Learn WM_SHOWWINDOW: "not sent ... When the SW_SHOWNORMAL flag is specified in the call to
 * ShowWindow"); that it is not sent when the visibility does not change is a choice without a source. Showing a window sends WM_WINDOWPOSCHANGING/WM_WINDOWPOSCHANGED (Learn WM_WINDOWPOSCHANGING: sent "to a window whose size, position, position in the z-order, or show state is about to change"). A minimized window
 * is hidden from the z-order of its owned windows (not modelled). */
static void input_window_hidden(const char *api, int64_t i);

/* Decision 0258 (alpha7), WM_GETMINMAXINFO. Learn WM_GETMINMAXINFO: "Sent to a window when the size or position of the window is about to change. An application can use this message to override the window's default
 * maximized size and position, or its default minimum or maximum tracking size"; wParam "not used"; lParam "A pointer to a MINMAXINFO structure that contains the default maximized position and dimensions, and the default
 * minimum and maximum tracking sizes. An application can override the defaults by setting the members of this structure"; "If an application processes this message, it should return zero". Learn MINMAXINFO: ptMaxSize
 * "The maximized width and the maximized height of the window. For top-level windows, this value is based on the width of the primary monitor", ptMaxPosition "the position of the left side ... and the top of the maximized window.
 * For top-level windows, this value is based on the position of the primary monitor", ptMinTrackSize "can be obtained programmatically from the system metrics SM_CXMINTRACK and SM_CYMINTRACK", ptMaxTrackSize "based on the size of
 * the virtual screen and can be obtained programmatically from the system metrics SM_CXMAXTRACK and SM_CYMAXTRACK". The windows of this model have no non-client area (nc_zero), so the maximized window is the work area of
 * the primary monitor (SPI_GETWORKAREA = rcWork = the monitor: fle_monitor.h) at its origin, the maximum tracking size is the virtual screen (SM_CXVIRTUALSCREEN x SM_CYVIRTUALSCREEN: no frame is added), the minimum tracking size is 0 x 0
 * (no border: Wine 10.0 get_min_max_info uses 2 * the frame = 0 for a window without WS_DLGFRAME / WS_BORDER; UNOFFICIAL; the model has no border), ptReserved is the window origin (Wine 10.0; the page: "Reserved"; UNOFFICIAL).
 * The return value of the window procedure is not used (Windows ignores it). out[0..1] = ptMaxSize, [2..3] = ptMaxPosition, [4..5] = ptMinTrackSize, [6..7] = ptMaxTrackSize as the window procedure left them. S-getminmaxinfo. */
static void get_min_max_info(const char *api, int64_t i, int32_t out[8])
{
    uint64_t sp = R[4], mm = (sp - 64u) & ~UINT64_C(15);
    uint32_t k;

    put_i32(mm, tab[i].px);
    put_i32(mm + 4, tab[i].py);
    put_i32(mm + 8, FLE_MONITOR_WORK_RIGHT - FLE_MONITOR_WORK_LEFT);
    put_i32(mm + 12, FLE_MONITOR_WORK_BOTTOM - FLE_MONITOR_WORK_TOP);
    put_i32(mm + 16, FLE_MONITOR_WORK_LEFT);
    put_i32(mm + 20, FLE_MONITOR_WORK_TOP);
    put_i32(mm + 24, 0);
    put_i32(mm + 28, 0);
    put_i32(mm + 32, FLE_GDC_HORZRES);
    put_i32(mm + 36, FLE_GDC_VERTRES);
    R[4] = mm;
    (void)send_to(api, i, FLE_WM_GETMINMAXINFO, 0, mm);
    R[4] = sp;
    for (k = 0; k < 8; k++) out[k] = (int32_t)fle_ld(mm + 8 + 4u * k, 4);
}

/* decision 0222: set by DestroyWindow around its hide of a window that is not a child: the hide that destroying such a window makes is a SetWindowPos(SWP_HIDEWINDOW) and sends no WM_SHOWWINDOW (Wine 10.0, win_act.c scenario 6; UNOFFICIAL) */
static int sw_silent;
void fle_imp_ShowWindow(void)
{
    static const char api[] = "ShowWindow";
    int32_t cmd = (int32_t)(uint32_t)R[2];
    int64_t i = win_arg(R[1]);
    int was, now, was_iconic, was_zoomed;
    uint32_t swp;

    fle_win_check_thread(api);
    if (i < 0) return;
    if (cmd < 0 || cmd > 9) {
        char why[100];

        snprintf(why, sizeof why, "nCmdShow %d: SW_SHOWDEFAULT (10), SW_FORCEMINIMIZE (11) and values outside the list are not modelled", (int)cmd);
        fle_stop(api, why);
    }
    was = (tab[i].style & FLE_WS_VISIBLE) != 0;
    now = cmd != FLE_SW_HIDE;
    was_iconic = tab[i].iconic != 0;
    was_zoomed = tab[i].zoomed != 0;
    tab[i].ever_shown = 1;
    /* Decision 0258 (alpha7), S18. A top-level window that changes between the normal, minimized and maximized states is moved to the rectangle of the new state and gets the messages of that change (WM_GETMINMAXINFO for the
     * maximized rectangle, WM_WINDOWPOSCHANGING, WM_NCCALCSIZE, WM_WINDOWPOSCHANGED, and from DefWindowProc WM_MOVE and WM_SIZE with SIZE_MINIMIZED / SIZE_MAXIMIZED / SIZE_RESTORED): Learn ShowWindow "SW_SHOWMAXIMIZED: Activates the
     * window and displays it as a maximized window"; "SW_SHOWNORMAL / SW_RESTORE: If the window is minimized, maximized, or arranged, the system restores it to its original size and position"; "SW_SHOWNOACTIVATE: Displays a
     * window in its most recent size and position"; "SW_MINIMIZE: Minimizes the specified window"; Learn WM_SIZE "SIZE_MAXIMIZED 2 The window has been maximized", "SIZE_MINIMIZED 1", "SIZE_RESTORED 0"; "The DefWindowProc function sends the
     * WM_SIZE and WM_MOVE messages when it processes the WM_WINDOWPOSCHANGED message". The order (UNOFFICIAL, Wine 10.0 win32u/window.c show_window / window_min_maximize): the maximized rectangle is asked first (WM_GETMINMAXINFO),
     * then WM_SHOWWINDOW when the visibility changes (Learn), then the SetWindowPos of the new rectangle with SWP_FRAMECHANGED. The rectangle of a minimized window is the one it had (the model has no icon position: S-minimized-rect),
     * and its WM_SIZE carries the client size it has. A restore of a window that was maximized when it was minimized gives the maximized rectangle back (Wine WIN_RESTORE_MAX; UNOFFICIAL). A child window keeps the flags only (no MDI). */
    {
        int top = tab[i].parent < 0 && !tab[i].msgonly;
        int cur = was_iconic ? 1 : was_zoomed ? 2 : 0, want = -1, fin;
        int32_t rx = tab[i].px, ry = tab[i].py, rcx = tab[i].pcx, rcy = tab[i].pcy;
        int change, do_rect = 0, restore_max = tab[i].restore_max;

        if (cmd == FLE_SW_SHOWMINIMIZED || cmd == FLE_SW_MINIMIZE || cmd == FLE_SW_SHOWMINNOACTIVE) want = 1;
        else if (cmd == FLE_SW_SHOWMAXIMIZED) want = 2;
        else if (cmd == FLE_SW_SHOWNORMAL || cmd == FLE_SW_RESTORE || (top && cmd == FLE_SW_SHOWNOACTIVATE)) want = 0;
        fin = want;
        if (top && want == 0 && was_iconic && restore_max) fin = 2;
        change = top && want >= 0 && (fin != cur || (fin == 2 && !was));
        if (change) {
            if (fin == 2) {
                int32_t mm[8];

                get_min_max_info(api, i, mm);
                rx = mm[2];
                ry = mm[3];
                rcx = mm[0];
                rcy = mm[1];
                do_rect = 1;
            } else if (fin == 0) {
                rx = tab[i].rest_x;
                ry = tab[i].rest_y;
                rcx = tab[i].rest_cx;
                rcy = tab[i].rest_cy;
                do_rect = 1;
            } else if (fin == 1) {
                /* Decision 0267 (alpha10; S-minimized-rect, replaces the choice of decision 0258 that a minimized window keeps its rectangle): Wine 10.0 (UNOFFICIAL, harness/alpha10/syscmd_seq.c) moves a minimized top-level window to
                 * (-32000, -32000) (WM_MOVE x = y = -32000) and its client area is 0 x 0 (GetClientRect 0, 0, 0, 0; WM_SIZE SIZE_MINIMIZED with lParam 0): Learn WM_SIZE "SIZE_MINIMIZED The window has been minimized"; Learn Window Features,
                 * Minimized: "the system reduces a minimized window to the size of its taskbar button". The model has no non-client area, so the window is 0 x 0 too (Wine's window rectangle is 160 x 31 = SM_CXMINIMIZED x SM_CYMINIMIZED
                 * with the client 0 x 0; the numbers differ only in GetWindowRect and in the cx, cy of WM_WINDOWPOSCHANGING / CHANGED). The restored rectangle (rest_*) is kept by apply_pos while the window is iconic. */
                rx = -32000;
                ry = -32000;
                rcx = 0;
                rcy = 0;
                do_rect = 1;
                tab[i].min_set = 1;
            }
            restore_max = fin == 1 ? was_zoomed : 0;
        }
        if (fin >= 0) {
            tab[i].iconic = fin == 1;
            tab[i].zoomed = fin == 2;
            if (change) tab[i].restore_max = restore_max;
        }
        if (was != now || change) {
            if (now) {
                tab[i].style |= FLE_WS_VISIBLE;
                tab[i].visible = 1;
            }
            swp = FLE_SWP_NOZORDER | FLE_SWP_NOACTIVATE | (was == now ? 0u : now ? FLE_SWP_SHOWWINDOW : FLE_SWP_HIDEWINDOW);
            if (change) swp |= FLE_SWP_FRAMECHANGED;
            if (!do_rect) swp |= FLE_SWP_NOMOVE | FLE_SWP_NOSIZE;
            /* WM_SHOWWINDOW first (the page: it is sent when the visibility state is changed by ShowWindow), then the position change notices */
            if (was != now && cmd != FLE_SW_SHOWNORMAL && !sw_silent) (void)send_to(api, i, FLE_WM_SHOWWINDOW, now ? 1u : 0u, 0);   /* Learn WM_SHOWWINDOW: "not sent ... when the SW_SHOWNORMAL flag is specified in the call to the ShowWindow function" */
            if (!now) {
                /* decision 0214: the windows around the one that is hidden, while it is still visible; apply_pos takes the snapshot over */
                if (unc_carry) { free(unc_carry->it); free(unc_carry); }
                unc_carry = unc_begin(i, tab[i].px, tab[i].py, tab[i].pcx, tab[i].pcy);
                tab[i].style &= ~FLE_WS_VISIBLE;
                tab[i].visible = 0;
            }
            /* Wine 10.0 show_window passes an empty rectangle to SetWindowPos when the state does not change (WM_WINDOWPOSCHANGING shows 0, 0, 0, 0 with SWP_NOMOVE | SWP_NOSIZE); the window keeps its place */
            apply_pos(api, i, 0, do_rect ? rx : 0, do_rect ? ry : 0, do_rect ? rcx : 0, do_rect ? rcy : 0, swp);
            if (unc_carry && unc_carry->subject == i) { free(unc_carry->it); free(unc_carry); unc_carry = NULL; }
            if (now) note_visibility_changed();
        }
    }
    if (now && !was && tab[i].pcx > 0 && tab[i].pcy > 0) {   /* decision 0247: the whole client area, whatever was invalid while the window was hidden (it was: only when nothing was invalid, which a bounding rectangle hid) */
        /* a window that becomes visible has its whole client area to paint (Learn BeginPaint: the update region is set "by the system after ... creating"): a choice for the show */
        upd_set_full(i);
        tab[i].upd_erase = 1;
        upd_show_descendants(i);   /* decision 0319 section 23 */
    }
    if (cmd == FLE_SW_HIDE) input_window_hidden(api, i);
    /* Learn ShowWindow: SW_SHOWNORMAL, SW_SHOWMINIMIZED, SW_SHOWMAXIMIZED, SW_SHOW and SW_RESTORE "Activate" the window; SW_SHOWNOACTIVATE, SW_SHOWMINNOACTIVE and SW_SHOWNA do not (activate_for_swp: a visible, enabled
     * top-level window without WS_EX_NOACTIVATE of this thread; a child window is not activated by it, nor is its top-level window). */
    /* A call that changes nothing activates nothing (Wine 10.0 show_window: SW_SHOW on a visible window, SW_SHOWNORMAL / SW_RESTORE on a visible window that is neither minimized nor maximized, SW_SHOWMINIMIZED on a visible
     * minimized window and SW_SHOWMAXIMIZED on a visible maximized window return before any SetWindowPos; UNOFFICIAL: Learn does not say; harness/api2/win_act.c scenario 13). */
    if (tab[i].parent < 0 && (cmd == FLE_SW_SHOWNORMAL || cmd == FLE_SW_SHOWMINIMIZED || cmd == FLE_SW_SHOWMAXIMIZED || cmd == 5 /* SW_SHOW */ || cmd == FLE_SW_RESTORE)
        && !(was && (cmd == 5 || ((cmd == FLE_SW_SHOWNORMAL || cmd == FLE_SW_RESTORE) && !was_iconic && !was_zoomed) || (cmd == FLE_SW_SHOWMINIMIZED && was_iconic) || (cmd == FLE_SW_SHOWMAXIMIZED && was_zoomed))))
        activate_for_swp(api, i);
    R[0] = was ? 1u : 0u;
}

/* user32!EnableWindow: BOOL f(HWND rcx, BOOL bEnable edx). Learn: "If the window was previously disabled, the return value is nonzero. If the window was not previously disabled, the return value is zero." "If the window is
 * being disabled, the system sends a WM_CANCELMODE message. If the enabled state of a window is changing, the system sends a WM_ENABLE message after the WM_CANCELMODE message. (These messages are sent before EnableWindow
 * returns.)" "If a window is already disabled, its child windows are implicitly disabled, although they are not sent a WM_ENABLE message." A disabled window cannot keep the focus ("A disabled control cannot receive
 * the keyboard focus"): the focus and the capture of the thread move to NULL when the window (or a descendant) is disabled. WS_DISABLED is the state (Learn Window Styles). */
static void input_window_disabled(int64_t i);

void fle_imp_EnableWindow(void)
{
    static const char api[] = "EnableWindow";
    int enable = ((uint32_t)R[2]) != 0;
    int64_t i = win_arg(R[1]);
    int was_disabled;

    fle_win_check_thread(api);
    if (i < 0) return;
    was_disabled = (tab[i].style & FLE_WS_DISABLED) != 0;
    if (enable && was_disabled) {
        tab[i].style &= ~FLE_WS_DISABLED;
        tab[i].enabled = 1;
        (void)send_to(api, i, FLE_WM_ENABLE, 1, 0);
    } else if (!enable && !was_disabled) {
        (void)send_to(api, i, FLE_WM_CANCELMODE, 0, 0);
        tab[i].style |= FLE_WS_DISABLED;
        tab[i].enabled = 0;
        input_window_disabled(i);
        (void)send_to(api, i, FLE_WM_ENABLE, 0, 0);
    }
    R[0] = was_disabled ? 1u : 0u;
}

/* user32!SetParent: HWND f(HWND hWndChild rcx, HWND hWndNewParent rdx). Learn: the return value is a handle to the previous parent window; NULL as the new parent makes the desktop the parent; HWND_MESSAGE makes the
 * window message-only (not modelled: stops). "SetParent does not modify the WS_CHILD or WS_POPUP window styles of the window whose parent is being changed" (R6). A new parent that is a window of this table: the
 * window becomes its child (parent field set, owner field cleared: a window has a parent OR an owner). A child moved to the desktop: parent -1. The window keeps its position numbers (they are now relative to the new parent's client
 * area: the page does not say whether the screen position is kept; a choice: the numbers are kept, S-setparent). The window is put at the top of the z-order of its new group (Learn: the page does not say; a choice). "If the
 * window identified by the hWndChild parameter is visible, the system performs the appropriate redrawing and repainting": the whole client area is invalidated. The previous parent of a top-level window is the desktop window
 * (a choice: the page says "a handle to the previous parent window"). */
void fle_imp_SetParent(void)
{
    static const char api[] = "SetParent";
    uint64_t hc = R[1], hn = R[2];
    int64_t c = win_arg(hc), np = -1, prev;

    fle_win_check_thread(api);
    if (c < 0) return;
    if (hn == FLE_HWND_MESSAGE) fle_stop(api, "hWndNewParent is HWND_MESSAGE: turning a window into a message-only window is not modelled");
    if (hn != 0 && hn != FLE_DESKTOP_HANDLE) {
        np = live_index(hn);
        if (np < 0) {
            fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
            R[0] = 0;
            return;
        }
        if (np == c) fle_stop(api, "a window as its own parent: the page does not say");
        {
            int64_t k = np;

            while (k >= 0) {
                if (k == c) fle_stop(api, "the new parent is a descendant of the window: the page does not say");
                k = tab[k].parent;
            }
        }
    }
    prev = tab[c].parent;
    R[0] = prev >= 0 ? handle_of(prev) : FLE_DESKTOP_HANDLE;
    if (tab[c].parent == np) return;
    zl_remove(c);
    tab[c].parent = np;
    tab[c].owner_win = -1;
    if (np >= 0 && !(tab[c].style & FLE_WS_CHILD) && !(tab[c].style & FLE_WS_POPUP)) { /* nothing: the styles are not modified */ }
    z_place(c, 0, -1);
    if (is_visible(c) && tab[c].pcx > 0 && tab[c].pcy > 0) {
        upd_set_full(c);
    }
    note_visibility_changed();
}

/* user32!BringWindowToTop: BOOL f(HWND rcx). Learn: brings the window to the top of the Z order; "similar to calling the SetWindowPos function to change a window's position in the Z order. BringWindowToTop does not make a
 * window a top-level window." It is SetWindowPos(hWnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) here (the page says "similar"; the flags are a choice). */
void fle_imp_BringWindowToTop(void)
{
    static const char api[] = "BringWindowToTop";
    int64_t i = win_arg(R[1]);

    fle_win_check_thread(api);
    if (i < 0) return;
    apply_pos(api, i, FLE_HWND_TOP, 0, 0, 0, 0, FLE_SWP_NOMOVE | FLE_SWP_NOSIZE);     /* no SWP_NOACTIVATE: apply_pos activates (Learn BringWindowToTop: "If the window is a top-level window, it is activated. If the window is a child window, the top-level parent window associated with the child window is activated.") */
    R[0] = 1;
}

/* user32!GetTopWindow: HWND f(HWND rcx). Learn: the child at the top of the Z order; NULL (hWnd NULL: the window at the top of the Z order = the top-level window at the top); no children: NULL. */
void fle_imp_GetTopWindow(void)
{
    int64_t idx[SIB_MAX], i;
    uint32_t n;

    fle_win_check_thread("GetTopWindow");
    if (R[1] == 0 || R[1] == FLE_DESKTOP_HANDLE) {
        n = siblings(-1, idx, SIB_MAX);
        R[0] = n ? handle_of(idx[0]) : 0;
        return;
    }
    i = win_arg(R[1]);
    if (i < 0) return;
    n = siblings(i, idx, SIB_MAX);
    R[0] = n ? handle_of(idx[0]) : 0;
}

/* user32!GetWindowTextW: int f(HWND rcx, LPWSTR rdx, int r8d). Learn: copies at most nMaxCount characters including the NUL; the return value is the length copied without the NUL; zero for an invalid handle or an empty
 * text. For a window of this process the page says WM_GETTEXT is sent: here the text kept by DefWindowProcW (WM_SETTEXT) is read directly (a choice: sending WM_GETTEXT, 0x000D, needs the DefWindowProcW answer). */
void fle_imp_GetWindowTextW(void)
{
    uint64_t out = R[2];
    int32_t max = (int32_t)(uint32_t)R[8];
    int64_t i;
    int32_t n, k;

    fle_win_check_thread("GetWindowTextW");
    i = live_index(R[1]);
    if (i < 0) {
        R[0] = 0;
        return;
    }
    if (out == 0 || max <= 0) {
        R[0] = 0;
        return;
    }
    n = tab[i].text_len;
    if (n > max - 1) n = max - 1;
    for (k = 0; k < n; k++) fle_st(out + 2u * (uint64_t)k, 2, tab[i].text[k]);
    fle_st(out + 2u * (uint64_t)n, 2, 0);
    R[0] = (uint32_t)n;
}

/* ---- input state: focus, active window, foreground window, capture (Learn: per thread / per input queue) ----
 * Learn GetFocus: "the window with the keyboard focus for the current thread's message queue"; GetActiveWindow: "the active window attached to the calling thread's message queue"; GetCapture: "the capture window associated with the
 * current thread"; GetForegroundWindow: the window the user is working with (one in the system). Each guest thread has its own focus, active and capture (fle_thread.c gives every thread a queue; the three values are kept
 * here, indexed by the thread index: a choice: the pages say the input state belongs to the input queue and each thread has its own queue unless AttachThreadInput joins them, which is not modelled).
 * The foreground window is global. There is no real input: nothing changes these values except the calls below and the destruction, hiding or disabling of a window. */
#define MAXT 256u
static struct { int64_t focus, active, capture; } inp[MAXT];
static int64_t fg_window = -1;
static int inp_init;

static void inp_ensure(void)
{
    unsigned k;

    if (inp_init) return;
    for (k = 0; k < MAXT; k++) inp[k].focus = inp[k].active = inp[k].capture = -1;
    inp_init = 1;
}

static unsigned cur_thread(void) { return fle_thread_current ? fle_thread_current() : 0u; }

/* The focus window of the thread must be a window of the thread that is not disabled and is visible? (Learn SetFocus: "If the hWnd parameter is invalid or the window is not attached to the calling thread's message
 * queue, the return value is NULL ... Extended error ERROR_INVALID_PARAMETER (0x57) means that window is in disabled state."). */
static void send_focus_msgs(const char *api, int64_t from, int64_t to)
{
    /* "This function sends a WM_KILLFOCUS message to the window that loses the keyboard focus and a WM_SETFOCUS message to the window that receives the keyboard focus." wParam: the other window (Learn WM_KILLFOCUS, WM_SETFOCUS). */
    if (from >= 0 && tab[from].live) (void)send_to(api, from, FLE_WM_KILLFOCUS, to >= 0 ? handle_of(to) : 0, 0);
    if (to >= 0 && tab[to].live) (void)send_to(api, to, FLE_WM_SETFOCUS, from >= 0 ? handle_of(from) : 0, 0);
}

/* decision 0222: "The system positions the active window at the top of the z-order for windows of the same type" (Learn Window Features / Z-Order); SetActiveWindow: "The window will be brought into the foreground (top
 * of Z-Order)"; SetWindowPos: unless SWP_NOACTIVATE "the window is activated and moved to the top of either the topmost or non-topmost group". The activated top-level window goes to the top of its group
 * (z_place keeps the topmost ones above it), and "When a window comes to the top of z-order, so do its child windows" (they travel with it) and "An owned window is always above its owner" (its owned windows
 * are raised after it, in their own order). The raise is made through SetWindowPos(HWND_TOP, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE), the call the system makes for it, so that the windows around get the
 * invalidation of what is uncovered and the window gets its WM_WINDOWPOSCHANGING / CHANGED. The messages' order inside the system call is a reading (S-activate-raise-order: before WM_NCACTIVATE / WM_ACTIVATE). */
static void raise_owned(const char *api, int64_t o)
{
    int64_t idx[SIB_MAX];
    uint32_t n = siblings(-1, idx, SIB_MAX), k;
    int64_t owned[64];
    uint32_t m = 0;

    for (k = 0; k < n && m < 64; k++)
        if (tab[idx[k]].owner_win == o && tab[idx[k]].live) owned[m++] = idx[k];
    while (m > 0) {                      /* the lowest of them first: the order among them is kept */
        int64_t w = owned[--m];

        apply_pos(api, w, FLE_HWND_TOP, 0, 0, 0, 0, FLE_SWP_NOMOVE | FLE_SWP_NOSIZE | FLE_SWP_NOACTIVATE);
        raise_owned(api, w);
    }
}
static void raise_activated(const char *api, int64_t i)
{
    int64_t idx[SIB_MAX];
    uint32_t n, nt = 0, k;
    int top = (tab[i].exstyle & FLE_WS_EX_TOPMOST) != 0;

    if (tab[i].parent >= 0 || tab[i].msgonly) return;
    n = siblings(-1, idx, SIB_MAX);
    for (k = 0; k < n; k++)
        if (tab[idx[k]].exstyle & FLE_WS_EX_TOPMOST) nt++;
    if (sib_pos(i) == (top ? 0 : (int64_t)nt)) {         /* already at the top of its group: nothing to move (the owned windows are above it already) */
        return;
    }
    apply_pos(api, i, FLE_HWND_TOP, 0, 0, 0, 0, FLE_SWP_NOMOVE | FLE_SWP_NOSIZE | FLE_SWP_NOACTIVATE);
    raise_owned(api, i);
}

static void do_activate(const char *api, int64_t i, uint64_t how)
{
    unsigned t = cur_thread();
    int64_t prev;

    inp_ensure();
    prev = inp[t].active;
    if (prev == i) return;
    raise_activated(api, i);
    /* Learn WM_NCACTIVATE / WM_ACTIVATE: the window being deactivated gets WM_NCACTIVATE FALSE and WM_ACTIVATE WA_INACTIVE (lParam = the window being activated); the window being activated gets WM_NCACTIVATE TRUE and
     * WM_ACTIVATE WA_ACTIVE (1) or WA_CLICKACTIVE (2) (lParam = the window being deactivated). Order: the deactivated window first (a reading of the pages; the page of Window Features says only that both get WM_ACTIVATE). */
    if (prev >= 0 && tab[prev].live) {
        (void)send_to(api, prev, FLE_WM_NCACTIVATE, 0, handle_of(i));
        (void)send_to(api, prev, FLE_WM_ACTIVATE, 0, handle_of(i));
    }
    inp[t].active = i;
    /* Learn GetForegroundWindow: "the window with which the user is currently working": the model has one application, so the window that its (foreground) thread activates is the foreground window */
    if (fg_window >= 0 && fg_window < (int64_t)tab_n && tab[fg_window].owner == t) fg_window = i;
    (void)send_to(api, i, FLE_WM_NCACTIVATE, 1, prev >= 0 ? handle_of(prev) : 0);
    (void)send_to(api, i, FLE_WM_ACTIVATE, how, prev >= 0 ? handle_of(prev) : 0);
    /* "If the window is being activated and is not minimized, the DefWindowProc function sets the keyboard focus to the window": done by DefWindowProcW(WM_ACTIVATE) below, not here. */
}

static void activate_for_swp(const char *api, int64_t i)
{
    /* Learn SetWindowPos: unless SWP_NOACTIVATE the window is activated. Only a visible top-level window that is not WS_EX_NOACTIVATE and is enabled becomes active (Learn Extended Window Styles: WS_EX_NOACTIVATE "A
     * top-level window created with this style does not become the foreground window"; Window Features: "A window must be enabled before it can be activated"). */
    int64_t r = root_of(i);

    if (!is_visible(r) || !tab[r].enabled || (tab[r].exstyle & FLE_WS_EX_NOACTIVATE)) return;
    if (tab[r].owner != cur_thread()) return;
    do_activate(api, r, 1);
    if (fg_window < 0) fg_window = r;
}

static void input_window_disabled(int64_t i)
{
    unsigned t;

    inp_ensure();
    for (t = 0; t < MAXT; t++) {
        int64_t k;

        for (k = inp[t].focus; k >= 0; k = tab[k].parent) {
            if (k == i) {
                /* Learn Window Features, Disabled Windows: "If an application uses the EnableWindow function to disable a keyboard-focus window, the window loses the keyboard focus in addition to being disabled. EnableWindow then
                 * sets the keyboard focus to NULL"; Learn WM_KILLFOCUS: "Sent to a window immediately before it loses the keyboard focus" (wParam: the window that receives the focus = NULL). Wine 10.0 sends it before WM_ENABLE
                 * (win_act.c scenario 11). */
                int64_t lost = inp[t].focus;

                inp[t].focus = -1;
                if (tab[lost].live && t == cur_thread()) (void)send_to("EnableWindow", lost, FLE_WM_KILLFOCUS, 0, 0);
                break;
            }
        }
        for (k = inp[t].capture; k >= 0; k = tab[k].parent) {
            if (k == i) {
                inp[t].capture = -1;
                break;
            }
        }
    }
}

/* decision 0222: the window that is activated when the active window goes away (hidden, destroyed). Learn ShowWindow SW_HIDE: "Hides the window and activates another window"; Window Features: "If the window being
 * destroyed is the active window, both the active and focus states are transferred to another window. The window that becomes the active window is the next window, as determined by the ALT+ESC key combination."
 * ALT+ESC cycles the top-level windows in z-order, so: the owner of an owned pop-up first (Wine 10.0 win32u/window.c activate_other_window: UNOFFICIAL; the page is silent), else the next window below it in the
 * z-order that can be activated, else (wrapping) the first one from the top. A window can be activated when it is a visible, enabled top-level window of the thread without WS_EX_NOACTIVATE. */
static int can_activate_win(int64_t k, unsigned t)
{
    return tab[k].live && tab[k].parent < 0 && !tab[k].msgonly && is_visible(k) && tab[k].enabled && tab[k].owner == t && !(tab[k].exstyle & FLE_WS_EX_NOACTIVATE);
}
static int64_t next_activatable(int64_t i, unsigned t)
{
    int64_t idx[SIB_MAX];
    uint32_t n = siblings(-1, idx, SIB_MAX), q, pos = n;

    if ((tab[i].style & FLE_WS_POPUP) && tab[i].owner_win >= 0) {
        int64_t o = root_of(tab[i].owner_win);

        if (o != i && can_activate_win(o, t)) return o;
    }
    for (q = 0; q < n; q++)
        if (idx[q] == i) pos = q;
    for (q = (pos < n ? pos + 1u : 0u); q < n; q++)
        if (idx[q] != i && can_activate_win(idx[q], t)) return idx[q];
    for (q = 0; q < (pos < n ? pos : n); q++)
        if (idx[q] != i && can_activate_win(idx[q], t)) return idx[q];
    return -1;
}

static void input_window_hidden(const char *api, int64_t i)
{
    unsigned t;

    inp_ensure();
    for (t = 0; t < MAXT; t++) {
        int64_t k;

        if (inp[t].active == i) {
            /* "Hides the window and activates another window" (Learn ShowWindow): the next window gets WM_NCACTIVATE / WM_ACTIVATE and the hidden one WM_NCACTIVATE FALSE / WM_ACTIVATE WA_INACTIVE (do_activate) */
            int64_t c = next_activatable(i, t);
            int was_fg = fg_window == i;

            if (c >= 0 && t == cur_thread()) {
                do_activate(api, c, 1);
                if (was_fg) fg_window = c;
            } else {
                inp[t].active = c;
                if (was_fg) fg_window = c;
            }
        }
        /* the focus is transferred with the activation (Learn Window Features: "both the active and focus states are transferred to another window"): when the activation above has not taken it away from this
         * window (WM_ACTIVATE's default processing sets the focus of the new active window and sends WM_KILLFOCUS to the old holder), it is cleared here */
        for (k = inp[t].focus; k >= 0; k = tab[k].parent)
            if (k == i) {
                inp[t].focus = -1;
                break;
            }
        for (k = inp[t].capture; k >= 0; k = tab[k].parent)
            if (k == i) {
                inp[t].capture = -1;
                break;
            }
    }
}

static void input_window_destroyed(int64_t i)
{
    unsigned t;

    inp_ensure();
    for (t = 0; t < MAXT; t++) {
        if (inp[t].focus == i) inp[t].focus = -1;
        if (inp[t].active == i) inp[t].active = -1;
        if (inp[t].capture == i) inp[t].capture = -1;
    }
    if (fg_window == i) fg_window = -1;
}

/* DefWindowProcW(WM_CANCELMODE): releases the mouse capture of the thread when the window or a descendant holds it (Learn WM_CANCELMODE). */
static void input_window_cancelmode(int64_t i)
{
    unsigned t = cur_thread();
    int64_t k;

    inp_ensure();
    for (k = inp[t].capture; k >= 0; k = tab[k].parent)
        if (k == i) {
            inp[t].capture = -1;
            break;
        }
}

/* DefWindowProcW(WM_ACTIVATE active): "sets the keyboard focus to the window" (Learn WM_ACTIVATE): only when the thread has no focus window inside it already. */
static void input_set_focus_default(const char *api, int64_t i)
{
    unsigned t = cur_thread();
    int64_t k, prev;

    inp_ensure();
    prev = inp[t].focus;
    for (k = prev; k >= 0; k = tab[k].parent)
        if (k == i) return;
    if (!tab[i].enabled) return;
    inp[t].focus = i;
    send_focus_msgs(api, prev, i);
}

/* user32!SetFocus: HWND f(HWND rcx). Learn: the return value is the handle of the window that previously had the keyboard focus; "If the hWnd parameter is invalid or the window is not attached to the calling thread's
 * message queue, the return value is NULL. To get extended error information, call GetLastError. Extended error ERROR_INVALID_PARAMETER (0x57) means that window is in disabled state." "This function sends a WM_KILLFOCUS
 * message to the window that loses the keyboard focus and a WM_SETFOCUS message to the window that receives the keyboard focus. It also activates either the window that receives the focus or the parent of the window that
 * receives the focus." (the activation: of the top-level window that contains it, unless it is already active). hWnd NULL: "keystrokes are ignored": the focus becomes NULL (a window that loses it gets WM_KILLFOCUS with
 * wParam 0). A window that the calling thread does not own: NULL and ERROR_INVALID_PARAMETER... the page says "not attached to the calling thread's message queue": the last error then is a choice: 1400 for an invalid handle,
 * 87 for a disabled window (the page names 87 for it), 5 for a window of another thread (a choice without a source). */
void fle_imp_SetFocus(void)
{
    static const char api[] = "SetFocus";
    uint64_t h = R[1];
    unsigned t = cur_thread();
    int64_t i = -1, prev, r;

    fle_win_check_thread(api);
    inp_ensure();
    prev = inp[t].focus;
    if (h != 0) {
        i = live_index(h);
        if (i < 0) {
            fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
            R[0] = 0;
            return;
        }
        if (tab[i].owner != t) {
            fle_set_last_error(FLE_ERROR_ACCESS_DENIED_W);
            R[0] = 0;
            return;
        }
        if (!is_enabled_chain(i)) {
            fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
            R[0] = 0;
            return;
        }
        r = root_of(i);
        /* the window that receives the focus is activated first (Learn: "It also activates either the window that receives the focus or the parent of the window that receives the focus") when it is not the active window */
        if (inp[t].active != r && is_visible(r) && !(tab[r].exstyle & FLE_WS_EX_NOACTIVATE)) do_activate(api, r, 1);
        if (inp[t].focus != i) {
            inp[t].focus = i;
            send_focus_msgs(api, prev, i);
        }
    } else if (prev >= 0) {
        inp[t].focus = -1;
        if (tab[prev].live) (void)send_to(api, prev, FLE_WM_KILLFOCUS, 0, 0);
    }
    R[0] = (prev >= 0 && tab[prev].live) ? handle_of(prev) : 0;
}

/* user32!GetFocus: HWND f(void). Learn: the window with the keyboard focus for the current thread's message queue; NULL when the queue has none. */
void fle_imp_GetFocus(void)
{
    unsigned t = cur_thread();

    fle_win_check_thread("GetFocus");
    inp_ensure();
    R[0] = (inp[t].focus >= 0 && tab[inp[t].focus].live) ? handle_of(inp[t].focus) : 0;
}

/* user32!SetActiveWindow: HWND f(HWND rcx). Learn: "The SetActiveWindow function activates a window, but not if the application is in the background. ... If the window identified by the hWnd parameter was created by the
 * calling thread, the active window status of the calling thread is set to hWnd. Otherwise, the active window status of the calling thread is set to NULL." The return value is the handle of the window that was previously
 * active; NULL on failure. The application is in the foreground in this model (there is one application) unless the foreground window is another thread's... the foreground window is set to the activated window when none is set. */
void fle_imp_SetActiveWindow(void)
{
    static const char api[] = "SetActiveWindow";
    unsigned t = cur_thread();
    int64_t i, prev;

    fle_win_check_thread(api);
    inp_ensure();
    i = win_arg(R[1]);
    if (i < 0) return;
    prev = inp[t].active;
    if (tab[i].owner != t) {
        inp[t].active = -1;
        R[0] = (prev >= 0 && tab[prev].live) ? handle_of(prev) : 0;
        return;
    }
    if (tab[i].parent >= 0) i = root_of(i);
    if (!tab[i].enabled) {
        fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
        R[0] = 0;
        return;
    }
    do_activate(api, i, 1);
    if (fg_window < 0) fg_window = i;
    R[0] = (prev >= 0 && tab[prev].live) ? handle_of(prev) : 0;
}

/* user32!GetActiveWindow: HWND f(void). Learn: the active window attached to the calling thread's message queue; otherwise NULL. */
void fle_imp_GetActiveWindow(void)
{
    unsigned t = cur_thread();

    fle_win_check_thread("GetActiveWindow");
    inp_ensure();
    R[0] = (inp[t].active >= 0 && tab[inp[t].active].live) ? handle_of(inp[t].active) : 0;
}

/* user32!GetGuiResources: DWORD f(HANDLE hProcess rcx, DWORD uiFlags edx) (decision 0129, wall 12). Learn: "Retrieves the count of handles to graphical user interface (GUI) objects in use by the specified process";
 * uiFlags: GR_GDIOBJECTS 0 "the count of GDI objects", GR_USEROBJECTS 1 "the count of USER objects", GR_GDIOBJECTS_PEAK 2, GR_USEROBJECTS_PEAK 4; "If the function succeeds, the return value is the count of handles to GUI
 * objects in use by the process. If no GUI objects are in use, the return value is zero. If the function fails, the return value is zero" (+ GetLastError). hProcess GR_GLOBAL (the page: "the resource usage is
 * reported across all processes in the current session"; its value is winuser.h = UNOFFICIAL, and nothing here reads the session). The model has one process, so the pseudo handle of GetCurrentProcess names it. Answered:
 * the pseudo handle (0xFFFFFFFFFFFFFFFF) with GR_GDIOBJECTS = the number of live objects of the virtual GDI table (fle_gdi_live_count: palettes, fonts, bitmaps, DCs, brushes, pens ... that the guest made and has not
 * deleted, and the stock objects it was handed: a choice, S-guires-count: the page does not say what is counted beyond "handles to GUI objects"), GR_USEROBJECTS = the number of live windows of the window table
 * (the other USER objects, menus, cursors, icons, hooks, timers, are not added: S-guires-count). The peak values and every other hProcess stop (the page's value for a process that is not this one needs a process
 * model). Withdrawn when the GUI object counts are measured on a real Windows or the model counts every USER object. */
uint64_t fle_gdi_live_count(void) __attribute__((weak));

void fle_imp_GetGuiResources(void)
{
    static const char api[] = "GetGuiResources";
    uint64_t h = R[1];
    uint32_t flags = (uint32_t)R[2];
    uint64_t k, live = 0;

    fle_win_check_thread(api);
    if (h != UINT64_MAX) fle_stop(api, "hProcess is not the pseudo handle of GetCurrentProcess: no other process exists (decision 0129)");
    if (flags == 0) {
        if (!fle_gdi_live_count) fle_stop(api, "no GDI table is linked (decision 0129)");
        R[0] = (uint32_t)fle_gdi_live_count();
        return;
    }
    if (flags == 1) {
        for (k = 0; k < tab_n; k++) live += tab[k].live != 0;
        R[0] = (uint32_t)live;
        return;
    }
    fle_stop(api, "uiFlags is a peak value or not on the page: the peak counts are not kept (decision 0129)");
}

/* user32!GetLastActivePopup: HWND f(HWND hWnd rcx) (decision 0129, wall 9). Learn: "Determines which pop-up window owned by the specified window was most recently active"; the return value "is the same as the hWnd
 * parameter, if any of the following conditions are met: The window identified by hWnd was most recently active. The window identified by hWnd does not own any pop-up windows. The window identified by hWnd is not a
 * top-level window, or it is owned by another window." The pages give the three conditions that return hWnd and say nothing of how the "most recently active" popup is kept. This model keeps no activation history of its
 * own (inp[].active is the current active window only). Answered: an invalid or destroyed window is NULL with ERROR_INVALID_WINDOW_HANDLE (1400, the choice of decision 0119); a window that is not top-level or that is
 * owned by another window is returned as it is (the third condition); a top-level window with no live owned pop-up window (the second condition) is returned as it is; a window that is the active window of any
 * thread is returned as it is (the first condition); a top-level window that owns a live pop-up window and is not active returns the owned pop-up window that is active, if one is, and otherwise itself
 * (S-lastactivepopup: "most recently active" is read as "the one that is active now, else hWnd": no history is kept, so a popup that was active earlier and is not now is not reported). Withdrawn when the model
 * keeps the order of activation. */
void fle_imp_GetLastActivePopup(void)
{
    uint64_t h = R[1];
    int64_t i = live_index(h), k;
    unsigned t;

    fle_win_check_thread("GetLastActivePopup");
    if (i < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
        R[0] = 0;
        return;
    }
    inp_ensure();
    if (tab[i].parent >= 0 || tab[i].owner_win >= 0) {
        R[0] = h;
        return;
    }
    for (t = 0; t < MAXT; t++)
        if (inp[t].active == i) {
            R[0] = h;
            return;
        }
    for (k = 0; k < (int64_t)tab_n; k++) {
        if (!tab[k].live || tab[k].owner_win != i || !(tab[k].style & FLE_WS_POPUP)) continue;
        for (t = 0; t < MAXT; t++)
            if (inp[t].active == k) {
                R[0] = handle_of(k);
                return;
            }
    }
    R[0] = h;
}

/* user32!GetForegroundWindow: HWND f(void). Learn: the foreground window, which "can be NULL in certain circumstances, such as when a window is losing activation". The model has no user: the foreground window is the
 * first window that a call activated (SetActiveWindow, SetForegroundWindow, SetWindowPos) and then the last one that SetForegroundWindow named; NULL until then (a choice without a source: no input has made any window
 * the foreground window). */
void fle_imp_GetForegroundWindow(void)
{
    fle_win_check_thread("GetForegroundWindow");
    R[0] = (fg_window >= 0 && tab[fg_window].live) ? handle_of(fg_window) : 0;
}

/* user32!SetForegroundWindow: BOOL f(HWND rcx). Learn: nonzero if the window was brought to the foreground, zero if not; the process may set the foreground window only under the listed conditions, among them "The calling
 * process is the foreground process" and "There is currently no foreground window, and thus no foreground process": both hold in this model (one process: the model gives the right to its own windows). The window becomes
 * the foreground window and the active window of its thread (the window must be a visible top-level window of an enabled state: a child is not brought: the top-level window containing it is, a choice). */
void fle_imp_SetForegroundWindow(void)
{
    static const char api[] = "SetForegroundWindow";
    int64_t i = win_arg(R[1]), r;
    unsigned saved;

    fle_win_check_thread(api);
    if (i < 0) return;
    r = root_of(i);
    if (!tab[r].enabled || !is_visible(r)) {
        R[0] = 0;
        return;
    }
    saved = tab[r].owner;
    (void)saved;
    if (tab[r].owner == cur_thread()) do_activate(api, r, 1);
    else {
        /* a window of another thread: the activation is that thread's: its active window changes without a message to it (the model has no way to run its queue): record it */
        inp_ensure();
        inp[tab[r].owner].active = r;
    }
    fg_window = r;
    R[0] = 1;
}

/* user32!SetCapture: HWND f(HWND rcx). Learn: "Only the foreground window can capture the mouse" (a background window "receives messages only for mouse events that occur when the cursor hot spot is within the visible portion of the
 * window": real input only: here the capture is recorded for any window of the calling thread); the return value is the handle of the window that had previously captured the mouse, NULL if none; "A handle to the window
 * in the current thread that is to capture the mouse": a window of another thread: NULL and 1408 (a choice). A window that has the capture and is replaced gets WM_CAPTURECHANGED (Learn WM_CAPTURECHANGED: "lParam: A handle to
 * the window gaining the mouse capture"). */
void fle_imp_SetCapture(void)
{
    static const char api[] = "SetCapture";
    unsigned t = cur_thread();
    int64_t i, prev;

    fle_win_check_thread(api);
    inp_ensure();
    i = win_arg(R[1]);
    if (i < 0) return;
    if (tab[i].owner != t) {
        fle_set_last_error(FLE_ERROR_WINDOW_OF_OTHER_THREAD);
        R[0] = 0;
        return;
    }
    prev = inp[t].capture;
    inp[t].capture = i;
    if (prev >= 0 && prev != i && tab[prev].live) (void)send_to(api, prev, FLE_WM_CAPTURECHANGED, 0, handle_of(i));
    R[0] = (prev >= 0 && tab[prev].live) ? handle_of(prev) : 0;
}

/* user32!GetCapture: HWND f(void). Learn: the capture window associated with the current thread; NULL "if no window in the thread has captured the mouse". */
void fle_imp_GetCapture(void)
{
    unsigned t = cur_thread();

    fle_win_check_thread("GetCapture");
    inp_ensure();
    R[0] = (inp[t].capture >= 0 && tab[inp[t].capture].live) ? handle_of(inp[t].capture) : 0;
}

/* user32!ReleaseCapture: BOOL f(void). Learn: nonzero on success. "A window receives [WM_CAPTURECHANGED] even if it calls ReleaseCapture itself" (Learn WM_CAPTURECHANGED): the window that had the capture gets it with
 * lParam = NULL (the page: "A handle to the window gaining the mouse capture"; none gains it). With no capture: nothing is sent and the call succeeds (the page gives no failure condition). */
void fle_imp_ReleaseCapture(void)
{
    static const char api[] = "ReleaseCapture";
    unsigned t = cur_thread();
    int64_t prev;

    fle_win_check_thread(api);
    inp_ensure();
    prev = inp[t].capture;
    inp[t].capture = -1;
    if (prev >= 0 && tab[prev].live) (void)send_to(api, prev, FLE_WM_CAPTURECHANGED, 0, 0);
    R[0] = 1;
}

/* ---- destruction, enumeration, search ---- */

/* decision 0218 (ship3): the posted messages of a destroyed window are removed from its thread's queue. Windows: a window that was destroyed gets no more messages; the classic page of DestroyWindow said that the function
 * "flushes the thread message queue" (the page that Learn shows today no longer says it: UNOFFICIAL, and the system behaviour in Wine 10.0 is the same: server queue_cleanup_window removes the posted and the sent messages
 * of the window). The real case: Enter in the VCL's name box: TranslateMessage posts the WM_CHAR to the box, the key down closes (destroys) it, and the VCL's IsKeyMsg then asked IsWindowUnicode of the gone window for the
 * WM_CHAR (FALSE: SendMessageA, which has no row: "unsupported instruction at 0x4253a0", W32 of decision 0191). Only messages with that window handle go; thread messages (hwnd 0, WM_QUIT) stay. */
static void flush_posted_of(uint64_t h, unsigned owner)
{
    fle_tqueue *q;
    uint32_t k, n = 0;

    if (!fle_thread_queue) return;
    q = fle_thread_queue(owner, 0);
    if (!q || !q->count) return;
    for (k = 0; k < q->count; k++) {
        fle_tmsg *m = &q->q[(q->head + k) % q->cap];

        if (m->hwnd == h) continue;
        if (n != k) q->q[(q->head + n) % q->cap] = *m;
        n++;
    }
    q->count = n;
}

/* user32!DestroyWindow: BOOL f(HWND rcx). Learn: "A thread cannot use DestroyWindow to destroy a window created by a different thread." (fails: 0 and 1408 ERROR_WINDOW_OF_OTHER_THREAD: the code is a choice); "If the specified
 * window is a parent or owner window, DestroyWindow automatically destroys the associated child or owned windows when it destroys the parent or owner window. The function first destroys child or owned windows, and then it
 * destroys the parent or owner window." (Window Features / R5); WM_DESTROY is "sent first to the window being destroyed and then to the child windows (if any) as they are destroyed"; WM_NCDESTROY "is sent after the child
 * windows have been destroyed" (Learn WM_DESTROY, WM_NCDESTROY). For a child window without WS_EX_NOPARENTNOTIFY "a WM_PARENTNOTIFY message is sent to the parent" (not sent here: the page for WM_PARENTNOTIFY could not be
 * read, S-parentnotify). The window leaves the z-order and every input state; a destroyed window is not live (the handle is never reused: decision 0071). The first window of the thread being destroyed does not end the
 * thread (no WM_QUIT is made: only PostQuitMessage makes one). The order of one window's destruction: the window's own WM_DESTROY first (parents before their children: "the window being destroyed and then the child windows
 * as they are destroyed"), then each child window's whole destruction, owned windows before children are not distinguished (a choice: children first by z-order, then owned windows by creation), then WM_NCDESTROY. */
static void destroy_tree(const char *api, int64_t i)
{
    int64_t idx[SIB_MAX];
    uint32_t n, k;
    uint64_t q;

    if (!tab[i].live) return;
    (void)send_to(api, i, FLE_WM_DESTROY, 0, 0);
    n = siblings(i, idx, SIB_MAX);
    for (k = 0; k < n; k++) destroy_tree(api, idx[k]);
    for (q = 0; q < tab_n; q++)
        if (tab[q].live && tab[q].owner_win == i) destroy_tree(api, (int64_t)q);
    if (tab[i].live) (void)send_to(api, i, FLE_WM_NCDESTROY, 0, 0);
    flush_posted_of(handle_of(i), tab[i].owner);
    input_window_destroyed(i);
    if (fle_input_window_gone) fle_input_window_gone(handle_of(i));
    timers_of_window_gone(i);
    if (fle_gdc_window_destroyed) fle_gdc_window_destroyed(handle_of(i));
    if (fle_layered_window_destroyed) fle_layered_window_destroyed(handle_of(i));   /* decision 0140 */
    if (fle_raster_window_destroyed && tab[i].parent < 0) fle_raster_window_destroyed(tab[i].lid);     /* decision 0126 (S-surface-life): the surface is that of a top-level window and goes with it */
    zl_remove(i);
    tab[i].live = 0;
    tab[i].destroyed = 1;
    tab[i].visible = 0;
    free(tab[i].updr);   /* decision 0247 */
    tab[i].updr = NULL;
}

/* decision 0222: the end of the process. Learn Window Features / Window Destruction: "An application eventually destroys the main window of the application (before terminating)"; "If the application does not release
 * the resources, the system will free any resources not freed by the application". FL's main form answers its close box with PostQuitMessage and the engine returns without a DestroyWindow or ShowWindow call
 * (native probe of decision 0222), so the windows are still in the table when the host process would end. When the process ends the system destroys what is left: every window goes without a message (its thread
 * is gone; the procedures cannot run), so the picture of the desktop is empty. Called by the host when CreateFruityInstance has returned. */
void fle_win_process_ended(void)
{
    uint64_t q;

    for (q = 0; q < tab_n; q++) {
        if (!tab[q].live) continue;
        input_window_destroyed((int64_t)q);
        if (fle_input_window_gone) fle_input_window_gone(handle_of((int64_t)q));
        timers_of_window_gone((int64_t)q);
        if (fle_gdc_window_destroyed) fle_gdc_window_destroyed(handle_of((int64_t)q));
        if (fle_layered_window_destroyed) fle_layered_window_destroyed(handle_of((int64_t)q));
        if (fle_raster_window_destroyed && tab[q].parent < 0) fle_raster_window_destroyed(tab[q].lid);
        zl_remove((int64_t)q);
        tab[q].live = 0;
        tab[q].destroyed = 1;
        tab[q].visible = 0;
    }
}

void fle_imp_DestroyWindow(void)
{
    static const char api[] = "DestroyWindow";
    int64_t i = win_arg(R[1]);

    fle_win_check_thread(api);
    if (i < 0) return;
    if (fle_thread_current && tab[i].owner != fle_thread_current()) {
        fle_set_last_error(FLE_ERROR_WINDOW_OF_OTHER_THREAD);
        R[0] = 0;
        return;
    }
    /* Learn Window Features: "When a window is destroyed, the system hides the window, if it is visible, and then removes any internal data associated with the window"; "If the window being destroyed is the active
     * window, both the active and focus states are transferred to another window. The window that becomes the active window is the next window, as determined by the ALT+ESC key combination." The hide is
     * ShowWindow(SW_HIDE) (WM_SHOWWINDOW, WM_WINDOWPOSCHANGING / CHANGED, the activation of the next window); Wine 10.0 hides first as well (win32u/window.c destroy_window). Only the window named in the call
     * is hidden; its descendants go with it. */
    if ((tab[i].style & FLE_WS_VISIBLE) && (tab[i].style & FLE_WS_CHILD) && tab[i].parent >= 0 && !is_visible(tab[i].parent)) {
        /* Wine 10.0 show_window: a child of an invisible parent only loses WS_VISIBLE after WM_SHOWWINDOW(FALSE) (txt1's create_seq.c case 'child WS_VISIBLE'; UNOFFICIAL) */
        (void)send_to(api, i, FLE_WM_SHOWWINDOW, 0, 0);
        if (!tab[i].live) {
            R[0] = 1;
            return;
        }
        tab[i].style &= ~FLE_WS_VISIBLE;
        tab[i].visible = 0;
    } else if (tab[i].style & FLE_WS_VISIBLE) {
        uint64_t sv1 = R[1], sv2 = R[2];

        R[1] = handle_of(i);
        R[2] = FLE_SW_HIDE;
        sw_silent = (tab[i].style & FLE_WS_CHILD) == 0;   /* Wine destroy_window: a child is hidden by ShowWindow(SW_HIDE) (WM_SHOWWINDOW), another window by SetWindowPos(SWP_HIDEWINDOW) (none) */
        fle_imp_ShowWindow();
        sw_silent = 0;
        R[1] = sv1;
        R[2] = sv2;
        if (!tab[i].live) {
            R[0] = 1;
            return;
        }
    }
    /* The owned windows of this thread go first, each through DestroyWindow itself, so that an owned window that is the active window passes the activation on (Learn Window Features: "If the window being destroyed is the
     * active window, both the active and focus states are transferred to another window"; Wine 10.0 user_destroy_window recurses into the owned windows and send_destroy_message activates another window for each; UNOFFICIAL
     * for the order; harness/api2/win_act.c scenario 15). Without it the active owned window was destroyed with its owner and the thread was left with no active window. */
    if (!(tab[i].style & FLE_WS_CHILD)) {
        uint64_t q;

        for (q = 0; q < tab_n; q++) {
            if (tab[q].live && tab[q].owner_win == i && !(tab[q].style & FLE_WS_CHILD) && (!fle_thread_current || tab[q].owner == fle_thread_current())) {
                uint64_t sv1 = R[1];

                R[1] = handle_of((int64_t)q);
                fle_imp_DestroyWindow();
                R[1] = sv1;
                q = (uint64_t)-1;
            }
        }
        if (!tab[i].live) {
            R[0] = 1;
            return;
        }
    }
    {   /* decision 0214: what the destroyed child uncovers is invalid in the windows beneath it */
        unc_snap *usn = unc_begin(i, tab[i].px, tab[i].py, tab[i].pcx, tab[i].pcy);

        destroy_tree(api, i);
        unc_end(api, usn, 0, 1);
    }
    R[0] = 1;
}

/* user32!EnumWindows: BOOL f(WNDENUMPROC rcx, LPARAM rdx). Learn: the callback is called for each top-level window with the handle and lParam; "If EnumWindowsProc returns zero, the return value is also zero"; otherwise
 * nonzero; "The EnumWindows function does not enumerate child windows"; "does not enumerate" a message-only window (Window Features). The order is the z-order from the top ("more reliable than calling GetWindow in a loop":
 * the page gives no order; the z-order from the top is a choice). The list is taken before the first call ("does not enumerate a child window that is ... created during the enumeration process" is on the EnumChildWindows
 * page; the same is used here). The callback is a guest function with two arguments (HWND, LPARAM): its result is the low 32 bits (BOOL). */
static uint64_t enum_list(int64_t parent, int64_t *idx, uint32_t cap) { return siblings(parent, idx, cap); }

void fle_imp_EnumWindows(void)
{
    static const char api[] = "EnumWindows";
    uint64_t proc = R[1], lp = R[2];
    int64_t idx[SIB_MAX];
    uint32_t n, k;

    fle_win_check_thread(api);
    if (proc == 0) fle_stop(api, "lpEnumFunc is NULL: the page gives no behaviour");
    n = (uint32_t)enum_list(-1, idx, SIB_MAX);
    for (k = 0; k < n; k++) {
        uint64_t r;

        if (!tab[idx[k]].live) continue;
        r = fle_call_guest(proc, handle_of(idx[k]), lp, 0, 0);
        if ((uint32_t)r == 0) {
            R[0] = 0;
            return;
        }
    }
    R[0] = 1;
}

/* user32!EnumChildWindows: BOOL f(HWND rcx, WNDENUMPROC rdx, LPARAM r8). Learn: hWndParent NULL = EnumWindows; children and their descendants are enumerated; the return value is not used. A child that is destroyed before it is
 * enumerated is not enumerated. Order: a window then its descendants (a child's children right after it), the children in z-order from the top (a choice). */
static void enum_children(const char *api, int64_t parent, uint64_t proc, uint64_t lp, int *stop)
{
    int64_t idx[SIB_MAX];
    uint32_t n = (uint32_t)enum_list(parent, idx, SIB_MAX), k;

    for (k = 0; k < n && !*stop; k++) {
        uint64_t r;

        if (!tab[idx[k]].live) continue;
        r = fle_call_guest(proc, handle_of(idx[k]), lp, 0, 0);
        if ((uint32_t)r == 0) {
            *stop = 1;
            return;
        }
        enum_children(api, idx[k], proc, lp, stop);
    }
}

void fle_imp_EnumChildWindows(void)
{
    static const char api[] = "EnumChildWindows";
    uint64_t hp = R[1], proc = R[2], lp = R[8];
    int stop = 0;

    fle_win_check_thread(api);
    if (proc == 0) fle_stop(api, "lpEnumFunc is NULL: the page gives no behaviour");
    if (hp == 0 || hp == FLE_DESKTOP_HANDLE) {
        R[1] = proc;
        R[2] = lp;
        fle_imp_EnumWindows();
        return;
    }
    {
        int64_t i = win_arg(hp);

        if (i < 0) return;
        enum_children(api, i, proc, lp, &stop);
    }
    R[0] = 1;
}

/* user32!EnumThreadWindows: BOOL f(DWORD dwThreadId ecx, WNDENUMPROC rdx, LPARAM r8). Learn: "If the callback function returns TRUE for all windows in the thread specified by dwThreadId, the return value is TRUE. If the
 * callback function returns FALSE on any enumerated window, or if there are no windows found in the thread specified by dwThreadId, the return value is FALSE." The windows of the thread are its top-level windows... and
 * children? The page does not say (VCL uses it to find the thread's windows): the top-level windows, z-order from the top, then no children (a choice). An id that no thread has: no windows: FALSE. */
void fle_imp_EnumThreadWindows(void)
{
    static const char api[] = "EnumThreadWindows";
    uint32_t tid = (uint32_t)R[1];
    uint64_t proc = R[2], lp = R[8];
    int64_t idx[SIB_MAX];
    uint32_t n, k;
    int found = 0;
    int idxt = fle_thread_index_of_vid ? fle_thread_index_of_vid(tid) : (tid == 0x1001u ? 0 : -1);

    fle_win_check_thread(api);
    if (proc == 0) fle_stop(api, "lpfn is NULL: the page gives no behaviour");
    if (idxt < 0) {
        R[0] = 0;
        return;
    }
    n = (uint32_t)enum_list(-1, idx, SIB_MAX);
    for (k = 0; k < n; k++) {
        uint64_t r;

        if (!tab[idx[k]].live || tab[idx[k]].owner != (uint32_t)idxt) continue;
        found = 1;
        r = fle_call_guest(proc, handle_of(idx[k]), lp, 0, 0);
        if ((uint32_t)r == 0) {
            R[0] = 0;
            return;
        }
    }
    R[0] = found ? 1u : 0u;
}

/* user32!FindWindowW: HWND f(LPCWSTR lpClassName rcx, LPCWSTR lpWindowName rdx). Learn: the top-level window whose class name and window name match; a NULL class matches any class; a NULL window name matches all names;
 * "This function does not search child windows"; "does not modify the last error value"; the failure value is NULL. The class name is compared with the name of the window's class (fle_class.c): the comparison is case-insensitive
 * (Windows class names are not case-sensitive? the page does not say: ASCII-insensitive is a choice); an atom (IS_INTRESOURCE) is compared with the window's atom. The first match in z-order from the top. The window name is the
 * text kept for the window. */
void fle_imp_FindWindowW(void)
{
    static const char api[] = "FindWindowW";
    uint64_t pc = R[1], pn = R[2];
    int64_t idx[SIB_MAX];
    uint32_t n, k;
    uint16_t cname[FLE_CLASS_NAME_MAX_UNITS + 1], nname[FLE_WIN_TEXT_MAX_UNITS + 1];
    uint16_t cl = 0, nl = 0;

    fle_win_check_thread(api);
    if (pc != 0 && (pc >> 16) != 0) {
        for (cl = 0; cl <= FLE_CLASS_NAME_MAX_UNITS; cl++) {
            uint16_t c = (uint16_t)fle_ld(pc + 2u * cl, 2);

            if (c == 0) break;
            if (cl == FLE_CLASS_NAME_MAX_UNITS) fle_stop(api, "lpClassName is longer than 256 units: not answered");
            cname[cl] = c;
        }
    }
    if (pn != 0) {
        for (nl = 0; nl <= FLE_WIN_TEXT_MAX_UNITS; nl++) {
            uint16_t c = (uint16_t)fle_ld(pn + 2u * nl, 2);

            if (c == 0) break;
            if (nl == FLE_WIN_TEXT_MAX_UNITS) fle_stop(api, "lpWindowName is longer than 256 units: not answered");
            nname[nl] = c;
        }
    }
    n = (uint32_t)enum_list(-1, idx, SIB_MAX);
    for (k = 0; k < n; k++) {
        int64_t i = idx[k];
        uint16_t nm[FLE_CLASS_NAME_MAX_UNITS + 1], nmlen = 0;

        if (!tab[i].live) continue;
        if (pc != 0) {
            if ((pc >> 16) == 0) {
                if (tab[i].atom != (uint16_t)pc) continue;
            } else {
                if (!fle_class_name_of(tab[i].atom, nm, &nmlen)) continue;
                if (fle_w_icmp(nm, nmlen, cname, cl) != 0) continue;
            }
        }
        if (pn != 0 && fle_w_icmp(tab[i].text, tab[i].text_len, nname, nl) != 0) continue;
        R[0] = handle_of(i);
        return;
    }
    R[0] = 0;
}

/* user32!UnregisterClassW: BOOL f(LPCWSTR lpClassName rcx, HINSTANCE hInstance rdx). Learn: nonzero on success; "If the class could not be found or if a window still exists that was created with the specified class, the
 * return value is zero"; "Before calling this function, an application must destroy all windows created with the specified class."; "System classes ... cannot be unregistered". The codes are a choice from System Error
 * Codes (1411 ERROR_CLASS_DOES_NOT_EXIST, 1412 ERROR_CLASS_HAS_WINDOWS): the page names none. Implemented in fle_class.c (the class table) with the window count asked of this file. */
uint32_t fle_win_count_of_atom(uint16_t atom)
{
    uint64_t k;
    uint32_t n = 0;

    for (k = 0; k < tab_n; k++)
        if (tab[k].live && tab[k].atom == atom) n++;
    return n;
}

/* Decision 0125 (PROVISIONAL): see fle_win.h. The root of a child chain is the top-level window (R10); an owned top-level window is its own root (it has its own surface). */
/* Decision 0140 (S-ulw-no-messages): what UpdateLayeredWindow does to the rectangle of a layered window: the position and/or the size change, with none of the messages of SetWindowPos (the pages of UpdateLayeredWindow say
 * nothing about messages). 1 on success, 0 when hwnd is not a live window. */
int fle_win_layered_rect(uint64_t hwnd, int set_pos, int32_t x, int32_t y, int set_size, int32_t cx, int32_t cy)
{
    int64_t i = live_index(hwnd);

    if (i < 0) return 0;
    if (set_pos) {
        tab[i].px = x;
        tab[i].py = y;
    }
    if (set_size) {
        tab[i].pcx = cx;
        tab[i].pcy = cy;
    }
    if (!tab[i].iconic && !tab[i].zoomed) {
        tab[i].rest_x = tab[i].px;
        tab[i].rest_y = tab[i].py;
        tab[i].rest_cx = tab[i].pcx;
        tab[i].rest_cy = tab[i].pcy;
    }
    tab[i].x = tab[i].px;
    tab[i].y = tab[i].py;
    tab[i].cx = tab[i].pcx;
    tab[i].cy = tab[i].pcy;
    return 1;
}

/* Decision 0319 section 13 (alpha1, PROVISIONAL; O1 'the picture of the clip that is dragged is not drawn'): Learn GetDCEx: DCX_LOCKWINDOWUPDATE "Allows drawing even if there is a LockWindowUpdate call in effect that would
 * otherwise exclude this window. This value is used for drawing during tracking." (0x400, UNOFFICIAL numbers as in fle_gdc.h). A DC of GetDCEx with that flag therefore ignores the lock of LockWindowUpdate; every other DC
 * of a locked window (or of a child of it) keeps the empty visible region (Learn LockWindowUpdate, decision 0126). FL calls GetDCEx with 0x402 and 0x412 (= DCX_CACHE [| DCX_CLIPSIBLINGS] | DCX_LOCKWINDOWUPDATE) while
 * it holds LockWindowUpdate (7 call sites): the drawing of a drag was discarded. `dcx` = the raw flags of a GetDCEx DC, 0 for every other DC. */
#define FLE_DCX_LOCKWINDOWUPDATE_ 0x00000400u
int fle_win_dc_query_ex(uint64_t hwnd, uint32_t dcx, fle_win_dcq *out);
int fle_win_dc_query(uint64_t hwnd, fle_win_dcq *out)
{
    return fle_win_dc_query_ex(hwnd, 0u, out);
}

int fle_win_dc_query_ex(uint64_t hwnd, uint32_t dcx, fle_win_dcq *out)
{
    int64_t i, r, a;
    uint32_t cs = 0;
    int32_t rx = 0, ry = 0, ox, oy;
    int32_t l, t, rr, b;

    memset(out, 0, sizeof *out);
    if (hwnd == FLE_DESKTOP_HANDLE) {
        out->root = FLE_DESKTOP_HANDLE;
        out->cx = FLE_GDC_HORZRES;
        out->cy = FLE_GDC_VERTRES;
        out->root_cx = FLE_GDC_HORZRES;
        out->root_cy = FLE_GDC_VERTRES;
        out->visible = 1;
        out->clip_r = FLE_GDC_HORZRES;
        out->clip_b = FLE_GDC_VERTRES;
        return 1;
    }
    i = live_index(hwnd);
    if (i < 0) return 0;
    r = root_of(i);
    out->root = handle_of(r);
    out->lid = tab[i].lid;
    out->root_lid = tab[r].lid;
    if (fle_class_style_of(tab[i].atom, &cs)) out->class_style = cs;
    out->style = tab[i].style;
    out->exstyle = tab[i].exstyle;
    out->thread = tab[i].owner;
    out->atom = tab[i].atom;
    out->cx = tab[i].pcx;
    out->cy = tab[i].pcy;
    /* decision 0126: the surface is the top-level window's (R10): its origin is that window's upper-left corner (the non-client area is 0) */
    client_origin(i, &ox, &oy);
    rx = tab[r].px;
    ry = tab[r].py;
    out->root_cx = tab[r].pcx;
    out->root_cy = tab[r].pcy;
    out->org_x = ox - rx;
    out->org_y = oy - ry;
    out->visible = is_visible(i) && ((dcx & FLE_DCX_LOCKWINDOWUPDATE_) != 0u || !is_locked_or_child(i)) && !is_no_redraw_chain(i);       /* decision 0126: a locked window's DC has an empty visible region (LockWindowUpdate page); decision 0319: not for a GetDCEx DC with DCX_LOCKWINDOWUPDATE */
    l = out->org_x;
    t = out->org_y;
    rr = l + tab[i].pcx;
    b = t + tab[i].pcy;
    for (a = tab[i].parent; a >= 0; a = tab[a].parent) {
        int32_t ax, ay, al, at2, ar, ab;

        client_origin(a, &ax, &ay);
        al = ax - rx;
        at2 = ay - ry;
        ar = al + tab[a].pcx;
        ab = at2 + tab[a].pcy;
        if (l < al) l = al;
        if (t < at2) t = at2;
        if (rr > ar) rr = ar;
        if (b > ab) b = ab;
    }
    if (rr < l) rr = l;
    if (b < t) b = t;
    out->clip_l = l;
    out->clip_t = t;
    out->clip_r = rr;
    out->clip_b = b;
    return 1;
}

/* Decision 0142 (PROVISIONAL; docs/decisions/0142-window-visible-region.md): the visible region of the DC of window `hwnd` with the clip flags `cf` (fle_visrgn.h FLE_VR_*), in the surface coordinates of the top-level window
 * (the same as fle_win_dc_query's org_x / clip_*). Returns 1 and the region; 0 when the window is not in the table or fle_visrgn.c is not linked; -1 when the region needs more rectangles than fle_visrgn keeps.
 *   The window rectangle: the client rectangle (the non-client area is 0: nc_zero), kept inside the client rectangles of all its ancestors (Learn Window Features, Child Windows / Positioning: "No part of a child window
 *   ever appears outside the borders of its parent window"). WS_CLIPCHILDREN / DCX_CLIPCHILDREN: minus the visible children (Learn Window Features / Clipping; GetDCEx: "Excludes the visible regions of all child windows
 *   below the window"). WS_CLIPSIBLINGS / DCX_CLIPSIBLINGS: minus the visible siblings above the window in the z-order of its group (GetDCEx: "Excludes the visible regions of all sibling windows above the window").
 *   A child window is grouped with its parent in z-order (Learn Window Features / Z-Order), so a sibling above an ANCESTOR that has WS_CLIPSIBLINGS covers the window too: the siblings above each ancestor are taken out when
 *   that ancestor has the style (S-ancestor-siblings: a reading). A sibling with WS_EX_TRANSPARENT does not take its area out of the siblings beneath it (Learn Extended Window Styles: "The window should not be painted until
 *   siblings beneath the window ... have been painted": they have to be able to draw there; S-ex-transparent). A hidden window and a minimized one take nothing out; a window with a minimized ancestor has an empty region
 *   (S-minimized-empty: Learn Window Features, Minimized: "the system reduces a minimized window to the size of its taskbar button"). FLE_VR_PARENTCLIP: the region is the parent's, computed with the parent's own
 *   WS_CLIPSIBLINGS and without its WS_CLIPCHILDREN (GetDCEx DCX_PARENTCLIP). Top-level windows are not clipped against each other here: the browser composes them (decision 0141). */
#define FLE_WS_EX_TRANSPARENT_ 0x00000020u
#define FLE_WS_CLIPSIBLINGS_   0x04000000u

static fle_rrect vr_window_rect(int64_t w, int32_t rx, int32_t ry)
{
    int32_t ox, oy;
    fle_rrect q;

    client_origin(w, &ox, &oy);
    q.x0 = (int64_t)ox - rx;
    q.y0 = (int64_t)oy - ry;
    q.x1 = q.x0 + tab[w].pcx;
    q.y1 = q.y0 + tab[w].pcy;
    return q;
}

int fle_win_dc_region(uint64_t hwnd, uint32_t cf, fle_visrgn *out) { return fle_win_dc_region_within(hwnd, cf, NULL, out); }

int fle_win_dc_region_within(uint64_t hwnd, uint32_t cf, const fle_rrect *within, fle_visrgn *out)
{
    int64_t i, r, w, a, p, idx[SIB_MAX];
    uint32_t n, k;
    int32_t rx, ry;
    int cs;

    if (!fle_visrgn_set_rect || !fle_visrgn_subtract || !fle_visrgn_intersect_rect) return 0;
    i = live_index(hwnd);
    if (i < 0) return 0;
    for (a = i; a >= 0; a = tab[a].parent)
        if (tab[a].iconic) {
            out->n = 0;
            return 1;
        }
    r = root_of(i);
    rx = tab[r].px;
    ry = tab[r].py;
    w = i;
    if ((cf & FLE_VR_PARENTCLIP) && tab[i].parent >= 0) {
        w = tab[i].parent;
        cf = (tab[w].style & FLE_WS_CLIPSIBLINGS_) ? FLE_VR_CLIPSIBLINGS : 0u;
    }
    fle_visrgn_set_rect(out, vr_window_rect(w, rx, ry));
    if (within) fle_visrgn_intersect_rect(out, *within);
    if (cf & FLE_VR_CLIPCHILDREN) {
        n = siblings(w, idx, SIB_MAX);
        for (k = 0; k < n; k++) {
            int64_t c = idx[k];

            if (!tab[c].visible || tab[c].iconic) continue;
            if (!fle_visrgn_subtract(out, vr_window_rect(c, rx, ry))) return -1;
        }
    }
    cs = (cf & FLE_VR_CLIPSIBLINGS) != 0;
    for (a = w; tab[a].parent >= 0;) {
        p = tab[a].parent;
        if (cs) {
            n = siblings(p, idx, SIB_MAX);
            for (k = 0; k < n && idx[k] != a; k++) {
                int64_t s = idx[k];

                if (!tab[s].visible || tab[s].iconic || (tab[s].exstyle & FLE_WS_EX_TRANSPARENT_)) continue;
                if (!fle_visrgn_subtract(out, vr_window_rect(s, rx, ry))) return -1;
            }
        }
        fle_visrgn_intersect_rect(out, vr_window_rect(p, rx, ry));
        cs = (tab[p].style & FLE_WS_CLIPSIBLINGS_) != 0;
        a = p;
    }
    return 1;
}

/* The window class name of an atom is kept by fle_class.c. */

/* ---- timers, update regions, painting (decision 0123: WM_TIMER and WM_PAINT are made here, and nothing else is made) ----
 * Learn SetTimer: WM_TIMER is posted to the thread's queue with the elapsed time-out as the period; "WM_TIMER is a low-priority message. The GetMessage and PeekMessage functions post this message only when no other
 * higher-priority messages are in the thread's message queue" (WM_TIMER page); GetMessage: "If no filter is specified, messages are processed in the following order: Sent messages, Posted messages, Input (hardware) messages and
 * system internal events, Sent messages (again), WM_PAINT messages, WM_TIMER messages". WM_PAINT: "The system sends a WM_PAINT message to a window whenever its update region is not empty and there are no other messages in the
 * application queue for that window" (InvalidateRect); "GetMessage returns the WM_PAINT message when there are no other messages in the application's message queue". So both are made by the retrieval call of the thread
 * that owns the window, after the posted messages: first a WM_PAINT of a window with a non-empty update region, then the WM_TIMER that is due. Virtual time: a timer is due when the virtual clock (fle_sys.h) has reached its
 * deadline; a thread that waits for a message with a timer pending waits until the earliest deadline (the scheduler of fle_thread.c asks fle_win_next_timer_tick). The period is uElapse, minimum USER_TIMER_MINIMUM (10 ms)
 * and maximum USER_TIMER_MAXIMUM (0x7FFFFFFF) (Learn SetTimer). A timer fires at most once per retrieval (the page gives no catching-up rule: a timer that is late by several periods yields ONE WM_TIMER and its next deadline is
 * the retrieval time plus the period: a choice without a source, S-timer). */
#define USER_TIMER_MINIMUM_ 0x0000000Au
#define USER_TIMER_MAXIMUM_ 0x7FFFFFFFu
#define MAX_TIMERS 256u

typedef struct {
    int used;
    int64_t win;           /* the window (index) or -1 for a thread timer (hWnd NULL) */
    unsigned thread;       /* the owner thread */
    uint64_t id;           /* nIDEvent, or the generated id */
    uint32_t period_ms;
    uint64_t deadline;     /* virtual clock ticks */
    uint64_t proc;         /* TIMERPROC or 0 */
} wtimer;

static wtimer timers[MAX_TIMERS];
/* Decision 0136: the WM_TIMER log (the G3 record: the virtual time of each delivery of each window timer). */
#define TIMER_LOG_MAX 8192u
static fle_timer_log_t timer_log[TIMER_LOG_MAX];
static unsigned timer_log_n;
/* fle_g1log.c, decision 0199 (observation only, FLE_G3_TIMER_LOG=1): weak, so that a build that does not link that file (the unit tests that compile fle_win.c with a few other files) is unchanged */
static void timer_log_add(const wtimer *tm, uint64_t now)
{
    if (timer_log_n < TIMER_LOG_MAX) {
        fle_timer_log_t *e = &timer_log[timer_log_n];

        e->hwnd = tm->win >= 0 ? handle_of(tm->win) : 0;
        e->id = tm->id;
        e->period_ms = tm->period_ms;
        e->due_ticks = tm->deadline;
        e->delivered_ticks = now;
    }
    timer_log_n++;
}
unsigned fle_win_timer_log_count(void) { return timer_log_n < TIMER_LOG_MAX ? timer_log_n : TIMER_LOG_MAX; }
int fle_win_timer_log_get(unsigned i, fle_timer_log_t *out)
{
    if (i >= fle_win_timer_log_count()) return 0;
    *out = timer_log[i];
    return 1;
}
static uint64_t next_timer_id = 1;            /* the ids generated for hWnd NULL (Learn SetTimer: "an integer identifying the new timer") */


/* Decision 0126 (G3 record): the window timers (SetTimer) that exist, with their owner thread, id, period, due time and whether the window is live. A pending timer of the first thread means that the virtual clock can still make a WM_TIMER. */
void fle_win_timer_report(FILE *f)
{
    unsigned k, n = 0;

    for (k = 0; k < MAX_TIMERS; k++) n += timers[k].used != 0;
    fprintf(f, "timers (SetTimer): %u live\n", n);
    for (k = 0; k < MAX_TIMERS; k++) {
        if (!timers[k].used) continue;
        fprintf(f, "  timer slot %u: thread %u id %#" PRIx64 " window %s%#" PRIx64 " period %u ms due %" PRIu64 " ticks proc %#" PRIx64 "\n", k, timers[k].thread, timers[k].id, timers[k].win >= 0 ? "" : "(none) ",
                timers[k].win >= 0 ? handle_of(timers[k].win) : (uint64_t)0, (unsigned)timers[k].period_ms, timers[k].deadline, timers[k].proc);
    }
}

/* Decision 0129: the number of live window timers and the longest period among them in ms (0 when none), for the stable-idle detector of the harness. A timer of a window that is not live is not counted (it can never fire). */
uint32_t fle_win_timer_live_count(void)
{
    unsigned k;
    uint32_t n = 0;

    for (k = 0; k < MAX_TIMERS; k++)
        if (timers[k].used && !(timers[k].win >= 0 && !tab[timers[k].win].live)) n++;
    return n;
}

uint64_t fle_win_timer_max_period_ms(void)
{
    unsigned k;
    uint64_t best = 0;

    for (k = 0; k < MAX_TIMERS; k++) {
        if (!timers[k].used || (timers[k].win >= 0 && !tab[timers[k].win].live)) continue;
        if (timers[k].period_ms > best) best = timers[k].period_ms;
    }
    return best;
}

uint64_t fle_win_next_timer_tick(unsigned idx)
{
    unsigned k;
    uint64_t best = 0;

    for (k = 0; k < MAX_TIMERS; k++) {
        if (!timers[k].used || timers[k].thread != idx) continue;
        if (timers[k].win >= 0 && !tab[timers[k].win].live) continue;
        if (best == 0 || timers[k].deadline < best) best = timers[k].deadline;
    }
    if (fle_input_next_tick) {
        uint64_t hv = fle_input_next_tick(idx);          /* decision 0136: the due time of a WM_MOUSEHOVER */

        if (hv != 0 && (best == 0 || hv < best)) best = hv;
    }
    return best;
}

/* user32!SetTimer: UINT_PTR f(HWND rcx, UINT_PTR nIDEvent rdx, UINT uElapse r8d, TIMERPROC lpTimerFunc r9). Learn: see above; hWnd "must be owned by the calling thread"; an existing timer with the same hWnd and nIDEvent is replaced
 * ("the timer is reset"); hWnd NULL with an nIDEvent that matches no timer: a new id is generated; "If the function succeeds and the hWnd parameter is NULL, the return value is an integer identifying the new timer ... If
 * the hWnd parameter is not NULL, the return value is a nonzero integer"; failure: 0 and the last error (1400 for an invalid window, 1408 for another thread's window: choices). The generated ids count from 1 (a choice
 * without a source: the page gives no value). The table holds 256 timers (a limit of this environment: 0 and ERROR_NOT_ENOUGH_MEMORY 8 beyond). */
void fle_imp_SetTimer(void)
{
    static const char api[] = "SetTimer";
    uint64_t h = R[1], id = R[2], proc = R[9];
    uint32_t el = (uint32_t)R[8];
    unsigned t = cur_thread(), k, slot = MAX_TIMERS;
    int64_t w = -1;

    fle_win_check_thread(api);
    if (h != 0) {
        w = win_arg(h);
        if (w < 0) return;
        if (tab[w].owner != t) {
            fle_set_last_error(FLE_ERROR_WINDOW_OF_OTHER_THREAD);
            R[0] = 0;
            return;
        }
    }
    if (el < USER_TIMER_MINIMUM_) el = USER_TIMER_MINIMUM_;
    if (el > USER_TIMER_MAXIMUM_) el = USER_TIMER_MAXIMUM_;
    for (k = 0; k < MAX_TIMERS; k++) {
        if (timers[k].used && timers[k].thread == t && timers[k].win == w && (w >= 0 || timers[k].id == id) && timers[k].id == (w >= 0 ? id : timers[k].id) && (w >= 0 ? 1 : id != 0)) {
            slot = k;
            break;
        }
    }
    if (slot == MAX_TIMERS) {
        for (k = 0; k < MAX_TIMERS; k++)
            if (!timers[k].used) {
                slot = k;
                break;
            }
        if (slot == MAX_TIMERS) {
            fle_set_last_error(8u);
            R[0] = 0;
            return;
        }
        timers[slot].id = w >= 0 ? id : next_timer_id++;
    }
    if (w >= 0 && id == 0) {
        /* "A nonzero timer identifier ... If the call is not intended to replace an existing timer, nIDEvent should be 0 if the hWnd is NULL": with a window, a zero id is not described: stops */
        fle_stop(api, "hWnd is not NULL and nIDEvent is 0: the page says the identifier is nonzero and gives no behaviour for 0");
    }
    timers[slot].used = 1;
    timers[slot].win = w;
    timers[slot].thread = t;
    timers[slot].period_ms = el;
    timers[slot].proc = proc;
    timers[slot].deadline = clk_now() + (uint64_t)el * FLE_TICKS_PER_MS;
    R[0] = timers[slot].id;
}

/* user32!KillTimer: BOOL f(HWND rcx, UINT_PTR uIDEvent rdx). Learn: nonzero on success, zero on failure ("To get extended error information, call GetLastError"); "The KillTimer function does not remove WM_TIMER messages
 * already posted to the message queue": the model keeps no posted WM_TIMER (they are made at retrieval), so nothing is left. A timer that does not exist: 0 and 1400... the page names no code: ERROR_INVALID_PARAMETER 87
 * (a choice). */
void fle_imp_KillTimer(void)
{
    static const char api[] = "KillTimer";
    uint64_t h = R[1], id = R[2];
    unsigned t = cur_thread(), k;
    int64_t w = -1;

    fle_win_check_thread(api);
    if (h != 0) {
        w = live_index(h);
        if (w < 0) {
            fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
            R[0] = 0;
            return;
        }
    }
    for (k = 0; k < MAX_TIMERS; k++) {
        if (timers[k].used && timers[k].win == w && timers[k].id == id && (w >= 0 || timers[k].thread == t)) {
            timers[k].used = 0;
            R[0] = 1;
            return;
        }
    }
    fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
    R[0] = 0;
}

/* Killing every timer of a destroyed window (Learn: a window's timers are destroyed with it: not on the pages read; a choice: otherwise a timer would post WM_TIMER to a window that is gone). */
static void timers_of_window_gone(int64_t w)
{
    unsigned k;

    for (k = 0; k < MAX_TIMERS; k++)
        if (timers[k].used && timers[k].win == w) timers[k].used = 0;
}

/* The window of thread `t` that has a non-empty update region and is visible and not iconic, first in z-order from the top among the top-level windows and then their children... a window with an update region of the thread:
 * the first one found by walking the z-order list from the top (top-level windows, each followed by its descendants). */
static int64_t paint_pending_window(unsigned t, uint64_t filter_h, int *filter_ok)
{
    uint32_t k;

    (void)filter_ok;
    for (k = 0; k < zl_n; k++) {
        int64_t i = zl[k];

        if (!tab[i].live || tab[i].owner != t || tab[i].in_paint) continue;
        if (tab[i].upd_r <= tab[i].upd_l || tab[i].upd_b <= tab[i].upd_t) continue;
        if (!is_visible(i) || tab[root_of(i)].iconic) continue;
        if (filter_h != 0 && filter_h != UINT64_MAX && handle_of(i) != filter_h) {
            /* "PeekMessage retrieves messages associated with the window identified by the hWnd parameter or any of its children as specified by the IsChild function" */
            int64_t a = tab[i].parent;
            int hit = 0;

            while (a >= 0) {
                if (handle_of(a) == filter_h) hit = 1;
                a = tab[a].parent;
            }
            if (!hit) continue;
        }
        return i;
    }
    return -1;
}

int fle_win_has_paint(unsigned idx) { return paint_pending_window(idx, 0, NULL) >= 0; }

/* The synthesised messages for GetMessageW / PeekMessageW of thread `t` (called after no posted message matched): fills `out` with a WM_PAINT or a WM_TIMER and returns 1; 0 when none is due. `remove` = the message is
 * consumed (PM_REMOVE / GetMessage): a WM_PAINT is not removed from the model (the update region stays until BeginPaint or ValidateRect: "GetMessage does not remove WM_PAINT messages from the queue. The messages remain in
 * the queue until processed" and "PeekMessage normally does not remove WM_PAINT messages ... However, if a WM_PAINT message has a NULL update region, PeekMessage does remove it": the region is never NULL here); a WM_TIMER
 * advances the timer to its next deadline (retrieval time + period: S-timer). `fmin`/`fmax` the message filter: 0, 0 = any. `qs` the QS_* types that may be made (decision 0126, PM_QS_PAINT / PM_QS_POSTMESSAGE:
 * QS_PAINT lets WM_PAINT through, QS_TIMER WM_TIMER). */
/* Decision 0129: how many WM_PAINT and WM_TIMER messages the retrieval calls made (information for the stable-idle report of the harness; they decide nothing). */
static uint64_t synth_timers, synth_paints;
void fle_win_synth_counts(uint64_t *timers, uint64_t *paints)
{
    *timers = synth_timers;
    *paints = synth_paints;
}

int fle_win_synth_message(unsigned t, uint64_t fh, uint32_t fmin, uint32_t fmax, int remove, uint32_t qs, fle_tmsg *out)
{
    int64_t w;
    unsigned k;
    uint64_t now = clk_now();

    if (fle_input_synth && fle_input_synth(t, fh, fmin, fmax, remove, qs, out)) return 1;            /* decision 0136: "Input (hardware) messages" come before WM_PAINT and WM_TIMER */
    if (fmin == 0 && fmax == 0) {
        fmin = 0;
        fmax = 0xFFFFFFFFu;
    }
    if ((qs & FLE_QS_PAINT) && FLE_WM_PAINT >= fmin && FLE_WM_PAINT <= fmax && fh != UINT64_MAX) {
        w = paint_pending_window(t, fh, NULL);
        if (w >= 0) {
            memset(out, 0, sizeof *out);
            out->hwnd = handle_of(w);
            out->msg = FLE_WM_PAINT;
            out->time = (uint32_t)(now / FLE_TICKS_PER_MS);
            synth_paints++;
            return 1;
        }
    }
    if ((qs & FLE_QS_TIMER) && FLE_WM_TIMER >= fmin && FLE_WM_TIMER <= fmax) {
        wtimer *best = NULL;

        for (k = 0; k < MAX_TIMERS; k++) {
            wtimer *tm = &timers[k];

            if (!tm->used || tm->thread != t || tm->deadline > now) continue;
            if (tm->win >= 0) {
                if (!tab[tm->win].live) continue;
                if (fh != 0 && fh != UINT64_MAX && handle_of(tm->win) != fh) continue;
            } else if (fh != 0) {
                if (fh != UINT64_MAX) continue;
            }
            if (!best || tm->deadline < best->deadline) best = tm;
        }
        if (best) {
            memset(out, 0, sizeof *out);
            out->hwnd = best->win >= 0 ? handle_of(best->win) : 0;
            out->msg = FLE_WM_TIMER;
            out->wp = best->id;
            out->lp = best->proc;
            out->time = (uint32_t)(now / FLE_TICKS_PER_MS);
            if (remove) {
                timer_log_add(best, now);
                best->deadline = now + (uint64_t)best->period_ms * FLE_TICKS_PER_MS;
            }
            synth_timers++;
            return 1;
        }
    }
    return 0;
}

/* user32!InvalidateRect: BOOL f(HWND rcx, const RECT *rdx, BOOL bErase r8d). Learn: the rectangle (client coordinates; NULL = the entire client area) is added to the update region; "If the bErase parameter is TRUE for any
 * part of the update region, the background is erased in the entire region"; hWnd NULL: "the system invalidates and redraws all windows ... and sends the WM_ERASEBKGND and WM_NCPAINT messages before the function returns"
 * (not modelled: stops). The update region is kept as ONE bounding rectangle (the page keeps a region; the bounding rectangle is a choice: the union of rectangles is larger than the region, S-updregion). Clipped to the
 * client area. An invalid window: 0 (the page names no error); the invalid handle code 1400 is set. */
void fle_imp_InvalidateRect(void)
{
    static const char api[] = "InvalidateRect";
    uint64_t h = R[1], pr = R[2];
    uint32_t erase = (uint32_t)R[8];
    int64_t i;
    int32_t l = 0, t2 = 0, r, b;

    fle_win_check_thread(api);
    if (h == 0) fle_stop(api, "hWnd is NULL: the page says every window is invalidated and redrawn: not modelled");
    i = win_arg(h);
    if (i < 0) return;
    r = tab[i].pcx;
    b = tab[i].pcy;
    if (pr != 0) {
        l = (int32_t)fle_ld(pr, 4);
        t2 = (int32_t)fle_ld(pr + 4, 4);
        r = (int32_t)fle_ld(pr + 8, 4);
        b = (int32_t)fle_ld(pr + 12, 4);
        if (l < 0) l = 0;
        if (t2 < 0) t2 = 0;
        if (r > tab[i].pcx) r = tab[i].pcx;
        if (b > tab[i].pcy) b = tab[i].pcy;
    }
    if (r > l && b > t2) upd_invalidate(i, l, t2, r, b, erase != 0);   /* decision 0214 (GAP-U1): a window without WS_CLIPCHILDREN passes it on to the children it covers */
    R[0] = 1;
}

/* user32!ValidateRect: BOOL f(HWND rcx, const RECT *rdx). Learn: "the rectangle ... to be removed from the update region. If this parameter is NULL, the entire client area is removed". With one bounding rectangle: removing a
 * rectangle that covers the update rectangle empties it; a partial removal leaves the bounding rectangle as it is (the region is not split: S-updregion). hWnd NULL: not modelled (stops). */
void fle_imp_ValidateRect(void)
{
    static const char api[] = "ValidateRect";
    uint64_t h = R[1], pr = R[2];
    int64_t i;

    fle_win_check_thread(api);
    if (h == 0) fle_stop(api, "hWnd is NULL: the page says every window is invalidated and redrawn: not modelled");
    i = win_arg(h);
    if (i < 0) return;
    if (pr == 0) {
        upd_clear_rect(i);
        tab[i].upd_erase = 0;
    } else {
        int32_t l = (int32_t)fle_ld(pr, 4), t2 = (int32_t)fle_ld(pr + 4, 4), r = (int32_t)fle_ld(pr + 8, 4), b = (int32_t)fle_ld(pr + 12, 4);

        upd_minus_rect(i, l, t2, r, b);   /* decision 0247: the rectangle is removed from the update region (it was: only when it covered the whole bounding rectangle) */
    }
    R[0] = 1;
}

/* user32!ValidateRgn: BOOL f(HWND rcx, HRGN rdx) (decision 0126, PROVISIONAL, wall 96). Learn: "validates the client area within a region by removing the region from the current update region of the specified window"; hRgn "If this
 * parameter is NULL, the entire client area is removed"; "The specified region must have been created by a region function. The region coordinates are assumed to be client coordinates"; nonzero on success. The update region is ONE
 * bounding rectangle (S-updregion), so the rule of ValidateRect applies: the update rectangle is emptied when the region covers all of it, and a partial removal leaves the rectangle as it is. hRgn that is not a live region of
 * fle_gdi.c: 0 (the page: "If the function fails, the return value is zero"; the last error is left alone: the page names none). An invalid window: 0 with 1400 (win_arg). */
void fle_imp_ValidateRgn(void)
{
    static const char api[] = "ValidateRgn";
    uint64_t h = R[1], rgn = R[2];
    int64_t i;
    int c;

    fle_win_check_thread(api);
    if (h == 0) fle_stop(api, "hWnd is NULL: the page does not define it");
    i = win_arg(h);
    if (i < 0) return;
    if (rgn == 0) {
        upd_clear_rect(i);
        tab[i].upd_erase = 0;
        R[0] = 1;
        return;
    }
    if (fle_gdi_region_covers == NULL) fle_stop(api, "no GDI region table is linked");
    c = fle_gdi_region_covers(rgn, tab[i].upd_l, tab[i].upd_t, tab[i].upd_r, tab[i].upd_b);
    if (c < 0) {
        R[0] = 0;
        return;
    }
    if (c > 0) {
        upd_clear_rect(i);
        tab[i].upd_erase = 0;
    } else if (fle_gdi_region_pieces != NULL) {
        /* decision 0247: the region is removed from the update region piece by piece (it was: only when it covered the whole bounding rectangle) */
        int32_t pc[128][4];
        uint32_t np = 0, k;

        if (fle_gdi_region_pieces(rgn, pc, 128, &np) > 0)
            for (k = 0; k < np; k++) upd_minus_rect(i, pc[k][0], pc[k][1], pc[k][2], pc[k][3]);
    }
    R[0] = 1;
}

/* user32!GetUpdateRect: BOOL f(HWND rcx, LPRECT rdx, BOOL bErase r8d). Learn: nonzero if the update region is not empty; the enclosing rectangle in device units (client coordinates); lpRect NULL: only the answer; "If this parameter is
 * TRUE and the update region is not empty, GetUpdateRect sends a WM_ERASEBKGND message to the specified window to erase the background" (not modelled with bErase TRUE: stops). An empty region: lpRect is the empty
 * rectangle (0,0,0,0) (the page says "retrieves an empty update region" after BeginPaint). */
void fle_imp_GetUpdateRect(void)
{
    static const char api[] = "GetUpdateRect";
    uint64_t h = R[1], pr = R[2];
    uint32_t erase = (uint32_t)R[8];
    int64_t i;
    int nonempty;

    fle_win_check_thread(api);
    i = win_arg(h);
    if (i < 0) return;
    nonempty = tab[i].upd_r > tab[i].upd_l && tab[i].upd_b > tab[i].upd_t;
    if (erase && nonempty) fle_stop(api, "bErase TRUE with a non-empty update region: the WM_ERASEBKGND the page says is sent is not modelled");
    if (pr != 0) {
        put_i32(pr, nonempty ? tab[i].upd_l : 0);
        put_i32(pr + 4, nonempty ? tab[i].upd_t : 0);
        put_i32(pr + 8, nonempty ? tab[i].upd_r : 0);
        put_i32(pr + 12, nonempty ? tab[i].upd_b : 0);
    }
    R[0] = nonempty ? 1u : 0u;
}

#define FLE_GDI_REGION_ERROR_ 0u /* ERROR of GetUpdateRgn / GetRgnBox (Learn: "ERROR"; the value 0 as in fle_gdi.c) */

/* user32!GetUpdateRgn: int f(HWND rcx, HRGN rdx, BOOL bErase r8d) (decision 0126, PROVISIONAL, wall 93). Learn: "retrieves the update region of a window by copying it into the specified region. The coordinates of the update region
 * are relative to the upper-left corner of the window (that is, they are client coordinates)"; the result is the complexity of the region ("COMPLEXREGION: more than one rectangle, ERROR, NULLREGION: Region
 * is empty, SIMPLEREGION: Region is a single rectangle"; the numbers 3, 0, 1, 2 are those of GetRgnBox, fle_gdi.h). The update region of a window is ONE bounding rectangle here (S-updregion), so the result is SIMPLEREGION or NULLREGION and the
 * region receives that rectangle (client coordinates; the non-client area is 0: nc_zero). hRgn that is not a live region of fle_gdi.c, and an invalid window: ERROR (the page: "An error occurred"; the last error is left alone: the
 * page names none; an invalid window handle sets 1400 like the other window calls through win_arg). bErase TRUE with a non-empty update region stops, as GetUpdateRect does (the WM_ERASEBKGND of the page is not modelled). */
void fle_imp_GetUpdateRgn(void)
{
    static const char api[] = "GetUpdateRgn";
    uint64_t h = R[1], rgn = R[2];
    uint32_t erase = (uint32_t)R[8];
    int64_t i;
    int nonempty;
    int32_t l = 0, t = 0, r = 0, b = 0;

    fle_win_check_thread(api);
    i = win_arg(h);
    if (i < 0) {
        R[0] = FLE_GDI_REGION_ERROR_;
        return;
    }
    if (fle_gdi_region_set_rect == NULL) fle_stop(api, "no GDI region table is linked");
    nonempty = tab[i].upd_r > tab[i].upd_l && tab[i].upd_b > tab[i].upd_t;
    if (erase && nonempty) fle_stop(api, "bErase TRUE with a non-empty update region: the erasing the page says is done is not modelled");
    if (nonempty) {
        l = tab[i].upd_l;
        t = tab[i].upd_t;
        r = tab[i].upd_r;
        b = tab[i].upd_b;
    }
    if (nonempty && tab[i].updr && fle_gdi_region_set_pieces != NULL) {
        /* decision 0247: the region receives every rectangle of the update region (COMPLEXREGION), not their bounding box */
        int32_t pc[FLE_VISRGN_MAX][4];
        uint32_t k, np = tab[i].updr->n;

        for (k = 0; k < np; k++) {
            pc[k][0] = (int32_t)tab[i].updr->r[k].x0;
            pc[k][1] = (int32_t)tab[i].updr->r[k].y0;
            pc[k][2] = (int32_t)tab[i].updr->r[k].x1;
            pc[k][3] = (int32_t)tab[i].updr->r[k].y1;
        }
        if (!fle_gdi_region_set_pieces(rgn, (const int32_t (*)[4])pc, np)) {
            R[0] = FLE_GDI_REGION_ERROR_;
            return;
        }
        R[0] = FLE_GDI_COMPLEXREGION;
        return;
    }
    if (!fle_gdi_region_set_rect(rgn, l, t, r, b)) {
        R[0] = FLE_GDI_REGION_ERROR_;
        return;
    }
    R[0] = nonempty ? FLE_GDI_SIMPLEREGION : FLE_GDI_NULLREGION;
}

/* user32!UpdateWindow: BOOL f(HWND rcx). Learn: "updates the client area of the specified window by sending a WM_PAINT message to the window if the window's update region is not empty. The function sends a WM_PAINT message
 * directly to the window procedure of the specified window, bypassing the application queue. If the update region is empty, no message is sent." */
void fle_imp_UpdateWindow(void)
{
    static const char api[] = "UpdateWindow";
    int64_t i = win_arg(R[1]);
    uint64_t sp;

    fle_win_check_thread(api);
    if (i < 0) return;
    if (tab[i].upd_r > tab[i].upd_l && tab[i].upd_b > tab[i].upd_t && is_visible(i)) {
        sp = R[4];
        (void)send_to(api, i, FLE_WM_PAINT, 0, 0);
        R[4] = sp;
    }
    R[0] = 1;
}

/* PAINTSTRUCT on x64 (Learn lists the members; the offsets are derived from the member types): HDC hdc @0, BOOL fErase @8, RECT rcPaint @12, BOOL fRestore @28, BOOL fIncUpdate @32, BYTE rgbReserved[32] @36 = 72 bytes
 * (4 bytes of tail padding). Learn gives no sizeof. */
#define PS_SIZE 72u

/* user32!BeginPaint: HDC f(HWND rcx, LPPAINTSTRUCT rdx). Learn: returns "a handle to a display device context for the specified window"; "automatically sets the clipping region ... automatically validates the entire client area"
 * ("BeginPaint automatically validates the update region, so any call to GetUpdateRect made immediately after the call to BeginPaint retrieves an empty update region"); "If the update region is marked for erasing,
 * BeginPaint sends a WM_ERASEBKGND message to the window"; fErase nonzero "if the application should erase the background"; rcPaint "the rectangle in which the painting is requested, in device units relative to the upper-left
 * corner of the client area". The DC is the DC of the window: a screen DC of the model (fle_gdc.c issues the handle of GetDC(NULL); the DC of a window needs a window object: this file asks fle_gdc for a window DC: not
 * available: the DC of the entire screen is given (a choice without a source: drawing is not modelled, S-paintdc) and counted. WM_ERASEBKGND is sent when the update region is marked for erasing (wParam the DC): the
 * result is nonzero if the application erased: then fErase = 0, else fErase = nonzero (Learn WM_ERASEBKGND and PAINTSTRUCT). DefWindowProcW(WM_ERASEBKGND) answers 1 (it "erases the background using the class background
 * brush": nothing is drawn: the erase is reported as done; a choice). */
void fle_imp_BeginPaint(void)
{
    static const char api[] = "BeginPaint";
    uint64_t h = R[1], ps = R[2];
    int64_t i;
    uint64_t dc, sp = R[4];
    int32_t l, t2, r, b;
    int erase, delayed;
    uint64_t er = 1;
    uint32_t bp_np = 0;
    int32_t bp_pc[FLE_GDC_PAINT_PIECES][4];

    fle_win_check_thread(api);
    if (ps == 0) fle_stop(api, "lpPaint is NULL: the page gives no behaviour");
    i = live_index(h);
    if (i < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
        R[0] = 0;
        return;
    }
    if (tab[i].in_paint) fle_stop(api, "BeginPaint for a window that is already between BeginPaint and EndPaint: the page says each call must be matched by EndPaint");
    if (!fle_imp_GetDC || !fle_imp_ReleaseDC) fle_stop(api, "no virtual screen DC is linked (fle_gdc.c): the DC of BeginPaint is not available");
    /* the DC: GetDC(hWnd) of fle_gdc.c is called as the guest would (the handle goes to PAINTSTRUCT.hdc and EndPaint releases it). Decision 0125 (S-paintdc, was GetDC(NULL) in decision 0123): the DC is the
     * DC of the WINDOW (Learn BeginPaint: "a handle to a display device context for the specified window"), the private or class DC of its class style or a common DC (Learn GetDC); still state only. */
    {
        uint64_t sv[5] = { R[0], R[1], R[2], R[8], R[9] };

        R[1] = h;
        fle_imp_GetDC();
        dc = R[0];
        R[0] = sv[0];
        R[1] = sv[1];
        R[2] = sv[2];
        R[8] = sv[3];
        R[9] = sv[4];
    }
    l = tab[i].upd_l;
    t2 = tab[i].upd_t;
    r = tab[i].upd_r;
    b = tab[i].upd_b;
    erase = tab[i].upd_erase;
    delayed = tab[i].upd_delayed;
    if (!(r > l && b > t2)) {
        l = t2 = r = b = 0;
    }
    {   /* decision 0247: the pieces of the update region go to the DC (its clip) */
        fle_visrgn uv;
        uint32_t k;

        upd_get(i, &uv);
        bp_np = uv.n > 1 && uv.n <= FLE_GDC_PAINT_PIECES ? uv.n : 0;
        for (k = 0; k < bp_np; k++) {
            bp_pc[k][0] = (int32_t)uv.r[k].x0;
            bp_pc[k][1] = (int32_t)uv.r[k].y0;
            bp_pc[k][2] = (int32_t)uv.r[k].x1;
            bp_pc[k][3] = (int32_t)uv.r[k].y1;
        }
    }
    tab[i].in_paint = 1;
    upd_clear_rect(i);
    tab[i].upd_erase = 0;
    /* decision 0142: the DC is clipped to the update region until EndPaint (Learn BeginPaint: "automatically sets the clipping region of the device context to exclude any area outside the update region").
     * Decision 0323: this comes BEFORE the WM_ERASEBKGND below, as in Wine 10.0 (win32u/dce.c send_erase: the DC is made with the update region as its clip, then the message is sent): the erase of the guest's own
     * procedure, and DefWindowProc's fill, reach the update region only. It was after the message, so an erase that fills "the client area" overpainted pixels that were valid. */
    if (fle_gdc_set_paint_rect) fle_gdc_set_paint_rect(dc, 1, l, t2, r, b);
    if (bp_np > 1 && fle_gdc_set_paint_pieces) fle_gdc_set_paint_pieces(dc, (const int32_t (*)[4])bp_pc, bp_np);
    if (erase && r > l) {
        R[4] = sp;
        er = send_to(api, i, FLE_WM_ERASEBKGND, dc, 0);
        R[4] = sp;
    }
    fle_st(ps, 8, dc);
    fle_st(ps + 8, 4, ((erase && r > l && er == 0) || (delayed && r > l)) ? 1u : 0u);
    put_i32(ps + 12, l);
    put_i32(ps + 16, t2);
    put_i32(ps + 20, r);
    put_i32(ps + 24, b);
    fle_st(ps + 28, 4, 0);
    fle_st(ps + 32, 4, 0);
    {
        unsigned q;

        for (q = 0; q < 32; q += 4) fle_st(ps + 36 + q, 4, 0);   /* rgbReserved[32] @36 .. 67: "Reserved; used internally by the system" */
    }
    R[4] = sp;
    R[0] = dc;
}

/* user32!EndPaint: BOOL f(HWND rcx, const PAINTSTRUCT *rdx). Learn: "The return value is always nonzero"; "EndPaint releases the display device context that BeginPaint retrieved". The DC of PAINTSTRUCT.hdc is released through
 * ReleaseDC(NULL, hdc) of fle_gdc.c. A window that is not between BeginPaint and EndPaint stops (the page: "Each call to BeginPaint must have a corresponding call to the EndPaint function"). */
void fle_imp_EndPaint(void)
{
    static const char api[] = "EndPaint";
    uint64_t h = R[1], ps = R[2];
    int64_t i;

    fle_win_check_thread(api);
    if (ps == 0) fle_stop(api, "lpPaint is NULL: the page gives no behaviour");
    i = live_index(h);
    if (i < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
        R[0] = 0;
        return;
    }
    if (!tab[i].in_paint) fle_stop(api, "EndPaint for a window that is not between BeginPaint and EndPaint: the page says each BeginPaint has one EndPaint");
    {
        uint64_t sv[5] = { R[0], R[1], R[2], R[8], R[9] };

        R[1] = h;
        R[2] = fle_ld(ps, 8);
        if (fle_gdc_set_paint_rect) fle_gdc_set_paint_rect(R[2], 0, 0, 0, 0, 0);
        fle_imp_ReleaseDC();
        R[0] = sv[0];
        R[1] = sv[1];
        R[2] = sv[2];
        R[8] = sv[3];
        R[9] = sv[4];
    }
    tab[i].in_paint = 0;
    R[0] = 1;
}

/* ---- the cursor position and the window under it ---- */
/* user32!LockWindowUpdate: BOOL f(HWND hWndLock rcx) (decision 0126, PROVISIONAL, wall 64). Learn (LockWindowUpdate): "disables or enables drawing in the specified window. Only one window can be locked at a time"; "If this
 * parameter is NULL, drawing in the locked window is enabled"; "If the function succeeds, the return value is nonzero. If the function fails, the return value is zero, indicating that an error occurred or another window
 * was already locked"; "If an application with a locked window (or any locked child windows) calls the GetDC, GetDCEx, or BeginPaint function, the called function returns a device context with a visible region that is
 * empty. This will occur until the application unlocks the window by calling LockWindowUpdate, specifying a value of NULL for hWndLock"; "If an application attempts to draw within a locked window, the system records
 * the extent of the attempted operation in a bounding rectangle. When the window is unlocked, the system invalidates the area within this bounding rectangle"; "does not make the specified window invisible and does not
 * clear the WS_VISIBLE style bit"; "A locked window cannot be moved". Answered: a live window of this table and no other window locked: it is locked, 1; NULL unlocks and returns 1 (also when nothing was locked: the page
 * names no failure for it, S-lock-null); a window while another is locked: 0 (the page's "another window was already locked"); locking the window that is already locked: 0 (S-lock-twice: it is "another" lock for the same
 * window; the page does not say); an hWnd that is not a live window: 0 with 1400 (S-err). A DC got for the locked window or a child of it (GetDC, GetDCEx, GetWindowDC, BeginPaint) has an empty visible region: the
 * drawing calls of fle_gdi.c discard their output (fle_win_dc_query reports visible = 0) and the extent of what was attempted is recorded as a bounding rectangle (the whole client area of the DC's window: S-lock-extent,
 * the pixels of the attempt are not computed) that the unlock invalidates (InvalidateRect of that window, so that a WM_PAINT follows). The locked window is not moved: MoveWindow and SetWindowPos on it fail?  Not modelled
 * (S-lock-move: the call is answered as for any window; the page's sentence is not enforced). */
static int64_t locked_window = -1;
static int lock_attempted;                     /* a drawing call was discarded for the locked window */
static int32_t lock_l, lock_t, lock_r, lock_b;  /* the bounding rectangle of the attempted drawing, in the client coordinates of the locked window */

static int is_no_redraw_chain(int64_t i)
{
    for (; i >= 0; i = tab[i].parent)
        if (tab[i].no_redraw) return 1;
    return 0;
}

static int is_locked_or_child(int64_t i)
{
    if (locked_window < 0) return 0;
    for (; i >= 0; i = tab[i].parent)
        if (i == locked_window) return 1;
    return 0;
}

void fle_win_lock_attempted(uint64_t hwnd)
{
    int64_t i = index_of(hwnd);

    if (i < 0 || !is_locked_or_child(i)) return;
    lock_attempted = 1;
    lock_l = 0;
    lock_t = 0;
    lock_r = tab[locked_window].pcx;
    lock_b = tab[locked_window].pcy;
}

void fle_imp_LockWindowUpdate(void)
{
    static const char api[] = "LockWindowUpdate";
    uint64_t h = R[1];
    int64_t i;

    fle_win_check_thread(api);
    if (h == 0) {
        if (locked_window >= 0 && lock_attempted && tab[locked_window].live && lock_r > lock_l && lock_b > lock_t) {
            upd_set_rect(locked_window, lock_l, lock_t, lock_r, lock_b);
        }
        locked_window = -1;
        lock_attempted = 0;
        R[0] = 1;
        return;
    }
    i = live_index(h);
    if (i < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE_W);
        R[0] = 0;
        return;
    }
    if (locked_window >= 0) {
        R[0] = 0;
        return;
    }
    locked_window = i;
    lock_attempted = 0;
    R[0] = 1;
}

/* ---- menus (decision 0126, PROVISIONAL, STATE ONLY: nothing is drawn, nothing is tracked) ----
 * The only menu handles this runtime issues are the window menu of GetSystemMenu (decision 0119: six SC_ commands, a bit set per window) and none else: SetMenu stops for a handle other than NULL, the menu of a window is
 * NULL. A menu call with the menu handle NULL, or with a handle that is not a menu of this table, finds NO menu: the calls that Learn defines by "If the menu item does not exist" answer that, the others fail.
 * user32!CheckMenuItem: DWORD f(HMENU hMenu rcx, UINT uIDCheckItem edx, UINT uCheck r8d). Learn (CheckMenuItem): "Sets the state of the specified menu item's check-mark attribute to either selected or clear"; "The return
 * value specifies the previous state of the menu item (either MF_CHECKED or MF_UNCHECKED). If the menu item does not exist, the return value is UINT_MAX" (0xFFFFFFFF). The guest (VCL TMenuItem.SetChecked of a form whose
 * menu has no handle, wall 63: hMenu NULL, item 0x75, MF_CHECKED) asks about an item of a menu that does not exist: the item does not exist, UINT_MAX, the last error untouched (the page names none). The same rule for
 * EnableMenuItem ("If the menu item does not exist, the return value is -1" = 0xFFFFFFFF). A real menu handle of the window menu: the six SC_ commands exist (their state is not kept: S-sysmenu-state: any check or
 * enable of one of them answers the previous state MF_UNCHECKED / MF_ENABLED = 0 and keeps nothing); another item of it does not exist. */
static uint32_t menu_item_state(const char *api, uint64_t m, uint32_t item, uint32_t flags)
{
    int64_t i = menu_owner(m);
    unsigned k;

    if (i < 0) return 0xFFFFFFFFu;
    if (flags & FLE_MF_BYPOSITION) {
        if (item >= 6u) return 0xFFFFFFFFu;
        return (tab[i].menu_items & (1u << item)) ? 0u : 0xFFFFFFFFu;
    }
    for (k = 0; k < 6; k++)
        if (menu_cmds[k] == item) break;
    (void)api;
    if (k == 6 || !(tab[i].menu_items & (1u << k))) return 0xFFFFFFFFu;
    return 0u;
}

void fle_imp_CheckMenuItem(void)
{
    fle_win_check_thread("CheckMenuItem");
    if (fle_crash1_menu_owns != NULL && fle_crash1_menu_owns(R[1])) { R[0] = fle_crash1_menu_check_item(R[1], (uint32_t)R[2], (uint32_t)R[8]); return; }
    R[0] = menu_item_state("CheckMenuItem", R[1], (uint32_t)R[2], (uint32_t)R[8]);
}

void fle_imp_EnableMenuItem(void)
{
    fle_win_check_thread("EnableMenuItem");
    if (fle_crash1_menu_owns != NULL && fle_crash1_menu_owns(R[1])) { R[0] = fle_crash1_menu_enable_item(R[1], (uint32_t)R[2], (uint32_t)R[8]); return; }
    R[0] = menu_item_state("EnableMenuItem", R[1], (uint32_t)R[2], (uint32_t)R[8]);
}

/* user32!GetKeyState: SHORT f(int nVirtKey ecx) and user32!GetAsyncKeyState: SHORT f(int vKey ecx) (decision 0126, PROVISIONAL, wall 61). Learn (GetKeyState): "Retrieves the status of the specified virtual key. The status specifies
 * whether the key is up, down, or toggled"; "If the high-order bit is 1, the key is down; otherwise, it is up. If the low-order bit is 1, the key is toggled. A key, such as the CAPS LOCK key, is toggled if it is turned on";
 * "The key status returned from this function changes as a thread reads key messages from its message queue. The status does not reflect the interrupt-level state associated with the hardware. Use the GetAsyncKeyState
 * function to retrieve that information"; nVirtKey is a virtual-key code (a letter or digit by its ASCII value); VK_SHIFT, VK_CONTROL, VK_MENU and the left/right codes are accepted. This environment has NO input: no key
 * message is ever read and no hardware exists, so every key is up and untoggled: the result is 0 (the page gives no value for "no input"; the answer follows from there being no key message: S-no-keys). Every integer is
 * accepted (the page lists no failing value). Stateless: no thread rule. */
/* Decision 0136 (PROVISIONAL): with fle_input.c linked the key and button state is the one the host fed in (GetKeyState: as of the key messages the thread read; GetAsyncKeyState: the hardware state); without it, or before
 * any event, every key is up (S-no-keys, above). */
void fle_imp_GetKeyState(void) { R[0] = fle_input_key_state ? fle_input_key_state((uint32_t)R[1], 0) : 0; }
void fle_imp_GetAsyncKeyState(void) { R[0] = fle_input_key_state ? fle_input_key_state((uint32_t)R[1], 1) : 0; }

/* The pointer position (decision 0126, PROVISIONAL, wall 74; decision 0123 answered (0, 0) for GetCursorPos and had no setter). Learn (GetCursorPos): "retrieves the position of the mouse cursor, in screen coordinates";
 * (SetCursorPos): "Moves the cursor to the specified screen coordinates. If the new coordinates are not within the screen rectangle set by the most recent ClipCursor function call, the system automatically adjusts
 * the coordinates so that the cursor stays within the rectangle"; "Returns nonzero if successful or zero otherwise"; (GetPhysicalCursorPos) "Retrieves the position of the cursor in physical coordinates",
 * (SetPhysicalCursorPos) its counterpart. There is no input device here and no ClipCursor (no row): the position is a state of the runtime, starting at (0, 0) (S-cursor-start: decision 0123's value), set by the setters
 * to the given point clamped to the virtual screen [0, 1920) x [0, 1080) (S-cursor-screen: the page's "adjusts the coordinates so that the cursor stays within the rectangle" is applied with the screen as the rectangle),
 * and read back by the getters. The physical and the logical position are the same (no DPI virtualisation: fle_gdc.h 96 DPI, S-physical-cursor). */
static int32_t ptr_x, ptr_y;

void fle_imp_GetCursorPos(void)
{
    uint64_t p = R[1];

    fle_win_check_thread("GetCursorPos");
    if (p == 0) fle_stop("GetCursorPos", "lpPoint is NULL: the page gives no behaviour");
    put_i32(p, ptr_x);
    put_i32(p + 4, ptr_y);
    R[0] = 1;
}

void fle_imp_GetPhysicalCursorPos(void)
{
    uint64_t p = R[1];

    fle_win_check_thread("GetPhysicalCursorPos");
    if (p == 0) fle_stop("GetPhysicalCursorPos", "lpPoint is NULL: the page gives no behaviour");
    put_i32(p, ptr_x);
    put_i32(p + 4, ptr_y);
    R[0] = 1;
}

static void set_pointer(const char *api)
{
    int32_t x = (int32_t)(uint32_t)R[1], y = (int32_t)(uint32_t)R[2];

    fle_win_check_thread(api);
    ptr_x = x < 0 ? 0 : x > FLE_GDC_HORZRES - 1 ? FLE_GDC_HORZRES - 1 : x;
    ptr_y = y < 0 ? 0 : y > FLE_GDC_VERTRES - 1 ? FLE_GDC_VERTRES - 1 : y;
    R[0] = 1;
}

/* user32!GetCursorInfo: BOOL f(PCURSORINFO pci rcx) (decision 0126, PROVISIONAL, wall 77). Learn (GetCursorInfo): "Retrieves information about the global cursor"; "you must set the cbSize member to sizeof(CURSORINFO) before
 * calling this function"; "If the function succeeds, the return value is nonzero. If the function fails, the return value is zero"; CURSORINFO { DWORD cbSize; DWORD flags; HCURSOR hCursor; POINT ptScreenPos } = 24 bytes
 * on x64 (4 + 4 + 8 + 8); flags: "The cursor is hidden" (0), CURSOR_SHOWING 0x1, CURSOR_SUPPRESSED 0x2 (Windows 8: touch or pen input). Answered: cbSize 24 (any other: failure, 0, ERROR_INVALID_PARAMETER 87: S-cursorinfo-size,
 * the page names no code); flags CURSOR_SHOWING when the display count of ShowCursor (fle_gdi.c) is 0 or more, 0 ("hidden") when it is negative (Learn ShowCursor: the cursor is displayed when the count is 0 or more);
 * hCursor the handle SetCursor set last (0 = none: the page says only "a handle to the cursor"); ptScreenPos the pointer position of GetCursorPos. CURSOR_SUPPRESSED is never reported (no touch or pen input exists). */
void fle_imp_GetCursorInfo(void)
{
    uint64_t p = R[1];

    fle_win_check_thread("GetCursorInfo");
    if (p == 0) fle_stop("GetCursorInfo", "pci is NULL: the page gives no behaviour");
    if ((uint32_t)fle_ld(p, 4) != 24u) {
        fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
        R[0] = 0;
        return;
    }
    if (!fle_gdi_cursor_display_count || !fle_gdi_cursor_current) fle_stop("GetCursorInfo", "no GDI cursor state is linked (decision 0126)");
    fle_st(p + 4, 4, fle_gdi_cursor_display_count() >= 0 ? 1u : 0u);
    fle_st(p + 8, 8, fle_gdi_cursor_current());
    put_i32(p + 16, ptr_x);
    put_i32(p + 20, ptr_y);
    R[0] = 1;
}

/* Decision 0136: the host moves the pointer (fle_input.c); the position is the one GetCursorPos reads. */
void fle_win_pointer_set(int32_t x, int32_t y)
{
    ptr_x = x;
    ptr_y = y;
}

void fle_win_pointer_get(int32_t *x, int32_t *y)
{
    *x = ptr_x;
    *y = ptr_y;
}

void fle_imp_SetCursorPos(void) { set_pointer("SetCursorPos"); }
void fle_imp_SetPhysicalCursorPos(void) { set_pointer("SetPhysicalCursorPos"); }

/* Decision 0244 (alpha1): a window that has a window region (SetWindowRgn) contains only the points inside it. The page of WindowFromPoint says "the window that contains the point"; the region as the window's shape for the pointer is
 * not on the pages of SetWindowRgn (UNOFFICIAL: Wine 10.0, win32u/window.c is_point_in_window, tests a point against the window region). (lx, ly) are in the window's own coordinates. */
static int in_window_region(int64_t i, int32_t lx, int32_t ly)
{
    int32_t l, t, r, b;

    if (tab[i].rgn == 0 || !fle_gdi_region_bounds || !fle_gdi_region_bounds(tab[i].rgn, &l, &t, &r, &b)) return 1;
    return lx >= l && lx < r && ly >= t && ly < b;
}

/* user32!WindowFromPoint: HWND f(POINT Point rcx). The POINT is passed by value in rcx (a struct of 8 bytes: x low 32 bits, y high 32 bits). Learn: "a handle to the window that contains the point. If no window exists at the point,
 * NULL"; "does not retrieve a handle to a hidden or disabled window, even if the point is within the window". The search: the visible, enabled top-level windows from the top of the z-order; inside the first one that
 * contains the point, the deepest visible and enabled child that contains it (children in z-order from the top). A window that is not visible, or that is disabled, is skipped together with its descendants. */
static int64_t hit_test(int64_t parent, int32_t px, int32_t py)
{
    int64_t idx[SIB_MAX];
    uint32_t n = siblings(parent, idx, SIB_MAX), k;

    for (k = 0; k < n; k++) {
        int64_t i = idx[k];
        int32_t ox, oy;

        if (!tab[i].live || !(tab[i].style & FLE_WS_VISIBLE) || !tab[i].enabled || tab[i].iconic) continue;
        if (tab[i].parent >= 0) client_origin(tab[i].parent, &ox, &oy);
        else ox = oy = 0;
        if (px < ox + tab[i].px || py < oy + tab[i].py || px >= ox + tab[i].px + tab[i].pcx || py >= oy + tab[i].py + tab[i].pcy) continue;
        if (!in_window_region(i, px - ox - tab[i].px, py - oy - tab[i].py)) continue;
        {
            int64_t c = hit_test(i, px, py);

            return c >= 0 ? c : i;
        }
    }
    return -1;
}

void fle_imp_WindowFromPoint(void)
{
    int32_t x = (int32_t)(uint32_t)R[1], y = (int32_t)(uint32_t)(R[1] >> 32);
    int64_t i;

    fle_win_check_thread("WindowFromPoint");
    i = hit_test(-1, x, y);
    R[0] = i >= 0 ? handle_of(i) : 0;
}

/* user32!IsWindowUnicode: BOOL f(HWND rcx). Learn: nonzero when the window is a native Unicode window; "The character set of the window is determined by the use of the RegisterClass function": a class registered with RegisterClassW (the only
 * registration this runtime answers; RegisterClassA and RegisterClassExW are not imported by the guest) makes a Unicode window. An invalid handle: 0 (the page names no error; no last error: a choice). */
void fle_imp_IsWindowUnicode(void)
{
    int64_t i;

    fle_win_check_thread("IsWindowUnicode");
    i = live_index(R[1]);
    R[0] = i >= 0 ? 1u : 0u;
}

/* user32!GetClassNameW: int f(HWND rcx, LPWSTR rdx, int r8d). Learn: copies the class name of the window's class; "the class name string is truncated to nMaxCount-1 characters"; the result is the number of characters copied without the NUL;
 * 0 on failure (an invalid window: 1400, a choice). The name comes from the class table (fle_class.c). */
void fle_imp_GetClassNameW(void)
{
    uint64_t out = R[2];
    int32_t max = (int32_t)(uint32_t)R[8];
    int64_t i;
    uint16_t nm[FLE_CLASS_NAME_MAX_UNITS + 1], nl = 0;
    int32_t n, k;

    fle_win_check_thread("GetClassNameW");
    i = win_arg(R[1]);
    if (i < 0) return;
    if (out == 0 || max <= 0) {
        R[0] = 0;
        return;
    }
    if (!fle_class_name_of(tab[i].atom, nm, &nl)) fle_stop("GetClassNameW", "the window's class is not in the class table");
    n = nl;
    if (n > max - 1) n = max - 1;
    for (k = 0; k < n; k++) fle_st(out + 2u * (uint64_t)k, 2, nm[k]);
    fle_st(out + 2u * (uint64_t)n, 2, 0);
    R[0] = (uint32_t)n;
}

/* ---- the property list of a window (R7): SetPropW, GetPropW, RemovePropW (Learn; the page of each) ---- */
/* The key lpString is a string or an atom: "If this parameter is an atom, it must be a global atom created by a previous call to the GlobalAddAtom function. The atom must be placed in the low-order word of lpString; the
 * high-order word must be zero." (SetPropW). A string key is compared without case folding? The page does not say: compared exactly (a choice without a source, S-prop: Windows converts a string to an atom, and atom names are
 * not case-sensitive in the global atom table: the model compares ASCII-case-insensitively as the atom table does: choice). An atom key and a string key never match each other here (the global atom table is not modelled:
 * GlobalAddAtom is not answered). */
static int prop_key(const char *api, uint64_t p, struct wprop *k)
{
    uint16_t i;

    memset(k, 0, sizeof *k);
    if ((p >> 16) == 0) {
        if (p == 0) fle_stop(api, "lpString is NULL: the page gives no behaviour");
        k->is_atom = 1;
        k->atom = (uint16_t)p;
        return 1;
    }
    for (i = 0; i <= FLE_CLASS_NAME_MAX_UNITS; i++) {
        uint16_t c = (uint16_t)fle_ld(p + 2u * i, 2);

        if (c == 0) break;
        if (i == FLE_CLASS_NAME_MAX_UNITS) fle_stop(api, "the property name is longer than 256 units: not answered");
        k->name[i] = c;
    }
    k->nlen = i;
    return 1;
}

static int prop_find(int64_t w, const struct wprop *k)
{
    uint32_t i;

    for (i = 0; i < tab[w].nprops; i++) {
        const struct wprop *q = &tab[w].props[i];

        if (q->is_atom != k->is_atom) continue;
        if (k->is_atom ? q->atom == k->atom : fle_w_icmp(q->name, q->nlen, k->name, k->nlen) == 0) return (int)i;
    }
    return -1;
}

/* user32!SetPropW: BOOL f(HWND rcx, LPCWSTR rdx, HANDLE r8). Learn: adds the string/atom and the data handle to the window's property list ("The data handle can identify any value useful to the application"); an existing entry of
 * the key gets the new data (the page does not say: the entry is replaced: a choice); nonzero on success; zero on failure. An invalid window: 0 and 1400 (a choice). */
void fle_imp_SetPropW(void)
{
    static const char api[] = "SetPropW";
    struct wprop k;
    int64_t w;
    int at;

    fle_win_check_thread(api);
    w = win_arg(R[1]);
    if (w < 0) return;
    prop_key(api, R[2], &k);
    k.data = R[8];
    at = prop_find(w, &k);
    if (at >= 0) {
        tab[w].props[at].data = k.data;
        R[0] = 1;
        return;
    }
    {
        struct wprop *n = (struct wprop *)realloc(tab[w].props, (tab[w].nprops + 1u) * sizeof *n);

        if (!n) fle_stop(api, "out of memory");
        tab[w].props = n;
        tab[w].props[tab[w].nprops++] = k;
    }
    R[0] = 1;
}

/* user32!GetPropW: HANDLE f(HWND rcx, LPCWSTR rdx). Learn: "If the property list contains the string, the return value is the associated data handle. Otherwise, the return value is NULL." An invalid window: NULL (the page: "Otherwise"). */
void fle_imp_GetPropW(void)
{
    static const char api[] = "GetPropW";
    struct wprop k;
    int64_t w;
    int at;

    fle_win_check_thread(api);
    w = live_index(R[1]);
    if (w < 0) {
        R[0] = 0;
        return;
    }
    prop_key(api, R[2], &k);
    at = prop_find(w, &k);
    R[0] = at >= 0 ? tab[w].props[at].data : 0;
}

/* user32!RemovePropW: HANDLE f(HWND rcx, LPCWSTR rdx). Learn: "The return value identifies the specified data. If the data cannot be found in the specified property list, the return value is NULL"; "The return value is the hData
 * value that was passed to SetProp". The entry is removed. Properties left on a window at WM_NCDESTROY are not checked (the page: "an application must remove all entries it has added"; a leak is the guest's). */
void fle_imp_RemovePropW(void)
{
    static const char api[] = "RemovePropW";
    struct wprop k;
    int64_t w;
    int at;

    fle_win_check_thread(api);
    w = live_index(R[1]);
    if (w < 0) {
        R[0] = 0;
        return;
    }
    prop_key(api, R[2], &k);
    at = prop_find(w, &k);
    if (at < 0) {
        R[0] = 0;
        return;
    }
    R[0] = tab[w].props[at].data;
    memmove(&tab[w].props[at], &tab[w].props[at + 1], (tab[w].nprops - (uint32_t)at - 1u) * sizeof tab[w].props[0]);
    tab[w].nprops--;
}

/* user32!CallWindowProcW: LRESULT f(WNDPROC lpPrevWndFunc rcx, HWND hWnd rdx, UINT Msg r8d, WPARAM wParam r9, LPARAM lParam [5]). Learn: "Passes message information to the specified window procedure"; "Use the CallWindowProc function
 * for window subclassing ... An application must pass any messages not processed by the new window procedure to the previous window procedure by calling CallWindowProc"; the return value "specifies the result of the message
 * processing and depends on the message sent"; lpPrevWndFunc "is actually either the address of a window or dialog box procedure, or a special internal value meaningful only to CallWindowProc" (not modelled: a value that is not an address
 * of a function of the image, of a response-table row, or a MakeObjectInstance slot stops in fle_call_guest / thunk_target). The previous procedure is called as the window procedure of the window `hWnd` in the context of the
 * calling thread (CallWindowProc is a plain call, not a SendMessage): through call_wndproc, which handles the thunk form; DefWindowProcW (the pseudo-VA of its response row) is the previous procedure of every window created
 * with the class of a guest that registered `DefWindowProcW` as its lpfnWndProc. */
void fle_imp_CallWindowProcW(void)
{
    static const char api[] = "CallWindowProcW";
    uint64_t prev = R[1], h = R[2], wp = R[9], lp = fle_arg(5);
    uint32_t msg = (uint32_t)R[8];

    fle_win_check_thread(api);
    if (prev == 0) fle_stop(api, "lpPrevWndFunc is NULL: the page gives no behaviour");
    R[0] = call_wndproc(api, prev, h, msg, wp, lp);
}

/* user32!AdjustWindowRectEx: BOOL f(LPRECT lpRect rcx, DWORD dwStyle edx, BOOL bMenu r8d, DWORD dwExStyle r9d) (decision 0126, PROVISIONAL, wall 84). Learn (AdjustWindowRectEx): "Calculates the required size of the window
 * rectangle, based on the desired size of the client rectangle. The window rectangle can then be passed to the CreateWindowEx function to create a window whose client area is the desired size"; nonzero on success; the page
 * gives no frame, caption or menu-bar size, and does not say what a NULL lpRect does. The windows of this model have a non-client area of 0 (nc_zero: the client rectangle is the window rectangle, GetClientRect and
 * client_origin say so), so the window rectangle that gives the desired client area is the desired client rectangle itself: the rectangle is returned as it is (S-adjustrect-nc-zero: the answer follows from nc_zero, not from
 * Windows' frame metrics; withdrawal: when a non-client area is modelled with sizes from a source). dwStyle, bMenu and dwExStyle do not change the result for the same reason (the page: WS_OVERLAPPED "cannot be specified": not
 * checked, S-adjustrect-style-unchecked). A NULL lpRect stops (the page gives no behaviour). */
void fle_imp_AdjustWindowRectEx(void)
{
    static const char api[] = "AdjustWindowRectEx";

    fle_win_check_thread(api);
    if (R[1] == 0) fle_stop(api, "lpRect is NULL: the page gives no behaviour (decision 0126)");
    R[0] = 1;
}

/* user32!SetMenu: BOOL f(HWND rcx, HMENU rdx). Learn: assigns the menu to the window; "If this parameter is NULL, the window's current menu is removed"; "A menu can be assigned to any window that is not a child window"; "The window is
 * redrawn to reflect the menu change"; "The SetMenu function replaces the previous menu, if any, but it does not destroy it"; nonzero on success. Answered: hMenu NULL on a live window that is not a child (the window has no menu to remove:
 * success; when the window has a menu its handle is cleared). A menu handle other than NULL stops (menus are not modelled: no HMENU is issued by this runtime but the window menu of GetSystemMenu, which is not a menu to set). A child window:
 * 0 and ERROR_INVALID_PARAMETER 87 (the page says it cannot be assigned: the code is a choice). Changing the menu changes the client area of a real window (the menu bar takes space); the window rectangle is kept and the non-client area
 * stays 0 here (nc_zero). */
void fle_imp_SetMenu(void)
{
    static const char api[] = "SetMenu";
    int64_t i;
    uint64_t m = R[2];

    fle_win_check_thread(api);
    i = win_arg(R[1]);
    if (i < 0) return;
    if (tab[i].parent >= 0 || (tab[i].style & FLE_WS_CHILD)) {
        fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
        R[0] = 0;
        return;
    }
    if (m != 0) fle_stop(api, "hMenu is not NULL: menus are not modelled (decision 0123)");
    tab[i].hmenu = 0;
    R[0] = 1;
}

/* user32!GetMenu: HMENU f(HWND rcx). Learn: the menu handle; NULL when the window has no menu; undefined for a child window (NULL here). */
void fle_imp_GetMenu(void)
{
    int64_t i;

    fle_win_check_thread("GetMenu");
    i = live_index(R[1]);
    R[0] = i >= 0 ? tab[i].hmenu : 0;
}

/* user32!DrawMenuBar: BOOL f(HWND rcx). Learn: redraws the menu bar; nonzero on success. Nothing is drawn: a live window answers 1; an invalid window 0 and 1400. */
void fle_imp_DrawMenuBar(void)
{
    int64_t i;

    fle_win_check_thread("DrawMenuBar");
    i = win_arg(R[1]);
    if (i < 0) return;
    R[0] = 1;
}

/* ---- scroll bars: the standard ones (SetScrollInfo, GetScrollInfo) and the flat ones (comctl32; decision 0123 walls 36-37, PROVISIONAL, STATE ONLY) ----
 * A window has two scroll bars (SB_HORZ 0, SB_VERT 1; SB_CTL 2 is the scroll bar control: not modelled, stops). Each has a SCROLLINFO of the page (Learn SCROLLINFO): nMin, nMax, nPage, nPos; nTrackPos is not settable ("the
 * SetScrollInfo function ignores this member"). Learn SetScrollInfo: "performs range checking on the values specified by the nPage and nPos members ... The nPage member must specify a value from 0 to nMax - nMin + 1. The nPos member must
 * specify a value between nMin and nMax - max(nPage - 1, 0). If either value is beyond its range, the function sets it to a value that is just within the range."; the return value is the current position of the scroll box.
 * Flat scroll bars: Learn InitializeFlatSB: "Initializes flat scroll bars for a particular window", nonzero when successful; "This function must be called before any other flat scroll bar functions are called"; FlatSB_* functions need "This
 * window handle ... passed previously in a call to InitializeFlatSB" (without it: 0, a choice). Learn also says: "Flat scroll bar functions are implemented in Comctl32.dll versions 4.71 through 5.82. Comctl32.dll versions 6.00 and
 * higher do not support flat scroll bars": a real Windows 10 with the comctl32 6.x of a manifest would not do what is modelled (the page does not say what it returns): the model answers success (S-flatsb). The flat and the standard scroll bar of a
 * window share the one SCROLLINFO (a choice: the page does not say whether they are separate). SIF_* values: Learn SCROLLINFO: SIF_RANGE 0x1, SIF_PAGE 0x2, SIF_POS 0x4, SIF_DISABLENOSCROLL 0x8, SIF_TRACKPOS 0x10, SIF_ALL = RANGE | PAGE | POS | TRACKPOS
 * (the page lists the names; the numbers are those of the SDK header, UNOFFICIAL). */
#define SIF_RANGE_ 0x1u
#define SIF_PAGE_  0x2u
#define SIF_POS_   0x4u
#define SIF_DISABLENOSCROLL_ 0x8u
#define SIF_TRACKPOS_ 0x10u
#define SIF_KNOWN_ (SIF_RANGE_ | SIF_PAGE_ | SIF_POS_ | SIF_DISABLENOSCROLL_ | SIF_TRACKPOS_)
#define SCROLLINFO_SIZE 28u   /* UINT cbSize, UINT fMask, int nMin, int nMax, UINT nPage, int nPos, int nTrackPos = 7 * 4 */

typedef struct { int32_t min, max; uint32_t page; int32_t pos; int enabled; int shown; } sbinfo;
struct wsb { int64_t w; int flat; sbinfo bar[2]; uint64_t prop[16]; };
static struct wsb sbs[128];
static unsigned nsb;

static struct wsb *sb_get(int64_t w, int create)
{
    unsigned k;

    for (k = 0; k < nsb; k++)
        if (sbs[k].w == w) return &sbs[k];
    if (!create) return NULL;
    if (nsb == sizeof sbs / sizeof sbs[0]) fle_stop("scroll bar", "more windows with scroll bars than the runtime holds");
    memset(&sbs[nsb], 0, sizeof sbs[nsb]);
    sbs[nsb].w = w;
    return &sbs[nsb++];
}

/* Applies a SCROLLINFO to the bar, with the range checking of the page; returns the position. */
static int32_t sb_set(sbinfo *b, uint32_t mask, int32_t mn, int32_t mx, uint32_t page, int32_t pos)
{
    int64_t span, lim;

    if (mask & SIF_RANGE_) {
        b->min = mn;
        b->max = mx;
    }
    span = (int64_t)b->max - (int64_t)b->min + 1;
    if (mask & SIF_PAGE_) b->page = page;
    if (b->page > (uint64_t)(span > 0 ? span : 0)) b->page = (uint32_t)(span > 0 ? span : 0);
    if (mask & SIF_POS_) b->pos = pos;
    lim = (int64_t)b->max - (b->page > 0 ? (int64_t)b->page - 1 : 0);
    if (b->pos > lim) b->pos = (int32_t)lim;
    if (b->pos < b->min) b->pos = b->min;
    return b->pos;
}

static uint32_t sb_read_info(const char *api, uint64_t p, int32_t *mn, int32_t *mx, uint32_t *page, int32_t *pos)
{
    uint32_t sz, mask;

    if (p == 0) fle_stop(api, "lpsi is NULL: the page gives no behaviour");
    sz = (uint32_t)fle_ld(p, 4);
    mask = (uint32_t)fle_ld(p + 4, 4);
    if (sz != SCROLLINFO_SIZE) {
        char why[100];

        snprintf(why, sizeof why, "SCROLLINFO.cbSize %u is not sizeof(SCROLLINFO) = %u: the page says the caller must set it", (unsigned)sz, (unsigned)SCROLLINFO_SIZE);
        fle_stop(api, why);
    }
    if (mask & ~SIF_KNOWN_) fle_stop(api, "SCROLLINFO.fMask has a bit that the page does not list");
    *mn = (int32_t)fle_ld(p + 8, 4);
    *mx = (int32_t)fle_ld(p + 12, 4);
    *page = (uint32_t)fle_ld(p + 16, 4);
    *pos = (int32_t)fle_ld(p + 20, 4);
    return mask;
}

static void sb_write_info(uint64_t p, uint32_t mask, const sbinfo *b)
{
    if (mask & SIF_RANGE_) {
        put_i32(p + 8, b->min);
        put_i32(p + 12, b->max);
    }
    if (mask & SIF_PAGE_) fle_st(p + 16, 4, b->page);
    if (mask & SIF_POS_) put_i32(p + 20, b->pos);
    if (mask & SIF_TRACKPOS_) put_i32(p + 24, b->pos);   /* nTrackPos: the position while the user drags: there is no user: the position (a choice) */
}

/* user32!SetScrollInfo: int f(HWND rcx, int nBar edx, LPCSCROLLINFO r8, BOOL redraw r9d). Learn: see above. nBar SB_CTL stops (the window would be a scroll bar control). An invalid window: 0 and 1400. */
void fle_imp_SetScrollInfo(void)
{
    static const char api[] = "SetScrollInfo";
    int32_t bar = (int32_t)(uint32_t)R[2], mn, mx, pos;
    uint32_t mask, page;
    int64_t w;
    struct wsb *s;

    fle_win_check_thread(api);
    w = win_arg(R[1]);
    if (w < 0) return;
    if (bar != 0 && bar != 1) fle_stop(api, "nBar is not SB_HORZ or SB_VERT: the scroll bar control (SB_CTL) is not modelled");
    mask = sb_read_info(api, R[8], &mn, &mx, &page, &pos);
    s = sb_get(w, 1);
    R[0] = (uint32_t)sb_set(&s->bar[bar], mask, mn, mx, page, pos);
}

/* user32!GetScrollInfo: BOOL f(HWND rcx, int nBar edx, LPSCROLLINFO r8). Learn: retrieves the parameters named by fMask; nonzero on success. A window that never had its scroll bar set: the bar has the range 0..0, page 0, position 0 (a choice:
 * the page gives no initial values; a standard window scroll bar that is not shown has no range). */
void fle_imp_GetScrollInfo(void)
{
    static const char api[] = "GetScrollInfo";
    int32_t bar = (int32_t)(uint32_t)R[2], mn, mx, pos;
    uint32_t mask, page;
    int64_t w;
    struct wsb *s;
    sbinfo zero;

    fle_win_check_thread(api);
    w = win_arg(R[1]);
    if (w < 0) return;
    if (bar != 0 && bar != 1) fle_stop(api, "nBar is not SB_HORZ or SB_VERT: the scroll bar control (SB_CTL) is not modelled");
    mask = sb_read_info(api, R[8], &mn, &mx, &page, &pos);
    memset(&zero, 0, sizeof zero);
    s = sb_get(w, 0);
    sb_write_info(R[8], mask, s ? &s->bar[bar] : &zero);
    R[0] = 1;
}

/* user32!ShowScrollBar: BOOL f(HWND hWnd rcx, int wBar edx, BOOL bShow r8d). Decision 0132 (PROVISIONAL), wall 20 (STATE ONLY like the rest of this section). Learn (ShowScrollBar): "shows or hides the specified scroll bar"; wBar is SB_BOTH
 * ("a window's standard horizontal and vertical scroll bars"; 3, winuser.h: UNOFFICIAL here, the page names the constants without numbers; SB_HORZ 0, SB_VERT 1, SB_CTL 2 are those of the SetScrollInfo section), SB_CTL (a scroll bar
 * control: not modelled, stops), SB_HORZ or SB_VERT; bShow "If this parameter is TRUE, the scroll bar is shown; otherwise, it is hidden"; "If the function succeeds, the return value is nonzero. If the function fails, the return
 * value is zero. To get extended error information, call GetLastError". Answered: a live window: the shown flag of the bar(s) is kept (S-showscrollbar-state: the window's WS_HSCROLL / WS_VSCROLL style bits, its client rectangle and
 * its non-client area are NOT changed: the page does not say, and this model has no scroll bar metrics) and the result is 1. An invalid window: 0 and ERROR_INVALID_WINDOW_HANDLE (1400), the answer of SetScrollInfo here. A wBar that
 * is not 0..3 stops: the page lists four values. */
void fle_imp_ShowScrollBar(void)
{
    static const char api[] = "ShowScrollBar";
    int32_t bar = (int32_t)(uint32_t)R[2];
    int shown = (uint32_t)R[8] != 0;
    int64_t w;
    struct wsb *s;

    fle_win_check_thread(api);
    w = win_arg(R[1]);
    if (w < 0) return;
    if (bar == 2) fle_stop(api, "wBar is SB_CTL: the scroll bar control is not modelled");
    if (bar < 0 || bar > 3) fle_stop(api, "wBar is none of SB_HORZ (0), SB_VERT (1), SB_CTL (2) and SB_BOTH (3): the page lists these four");
    s = sb_get(w, 1);
    if (bar == 0 || bar == 3) s->bar[0].shown = shown;
    if (bar == 1 || bar == 3) s->bar[1].shown = shown;
    R[0] = 1;
}

/* For tests and tools: 1 when ShowScrollBar last showed the standard bar (0 = SB_HORZ, 1 = SB_VERT) of the window, 0 when it hid it or never touched it. */
int fle_win_scrollbar_shown(uint64_t hwnd, int bar)
{
    int64_t w = live_index(hwnd);
    struct wsb *s;

    if (w < 0 || (bar != 0 && bar != 1)) return 0;
    s = sb_get(w, 0);
    return s ? s->bar[bar].shown : 0;
}

void fle_imp_InitializeFlatSB(void)
{
    int64_t w;
    struct wsb *s;

    fle_win_check_thread("InitializeFlatSB");
    w = win_arg(R[1]);
    if (w < 0) return;
    s = sb_get(w, 1);
    s->flat = 1;
    R[0] = 1;
}

/* index: WSB_PROP_* of commctrl.h (the page lists the names, not the numbers: the values are those of the SDK header, UNOFFICIAL: CYVSCROLL 0x1, CXHSCROLL 0x2, CYHSCROLL 0x4, CXVSCROLL 0x8, CXHTHUMB 0x10, CYVTHUMB 0x20, VBKGCOLOR 0x40,
 * HBKGCOLOR 0x80, VSTYLE 0x100, HSTYLE 0x200, WINSTYLE 0x400, PALETTE 0x800). The value is kept per bit; an index with more than one bit or above 0x800 stops. */
void fle_imp_FlatSB_SetScrollProp(void)
{
    static const char api[] = "FlatSB_SetScrollProp";
    uint32_t idx = (uint32_t)R[2];
    int64_t w;
    struct wsb *s;
    unsigned bit;

    fle_win_check_thread(api);
    w = win_arg(R[1]);
    if (w < 0) return;
    if (idx == 0 || (idx & (idx - 1u)) != 0 || idx > 0x800u) fle_stop(api, "index is not one WSB_PROP_* value of the SDK header (the page names the values and gives no numbers)");
    s = sb_get(w, 0);
    if (!s || !s->flat) {
        R[0] = 0;
        return;
    }
    for (bit = 0; (1u << bit) != idx; bit++) { }
    s->prop[bit] = R[8];
    R[0] = 1;
}

/* comctl32!FlatSB_SetScrollInfo: int f(HWND rcx, int code edx, LPSCROLLINFO r8, BOOL fRedraw r9d). Learn: sets the information of the flat scroll bar named by code (SB_HORZ, SB_VERT); returns "the current scroll position. If the call
 * changes the scroll position, then the previous position is returned". The model returns the position BEFORE the call (the page's second sentence; for an unchanged position the two sentences agree). The window needs InitializeFlatSB
 * (0 otherwise). */
void fle_imp_FlatSB_SetScrollInfo(void)
{
    static const char api[] = "FlatSB_SetScrollInfo";
    int32_t bar = (int32_t)(uint32_t)R[2], mn, mx, pos, prev;
    uint32_t mask, page;
    int64_t w;
    struct wsb *s;

    fle_win_check_thread(api);
    w = win_arg(R[1]);
    if (w < 0) return;
    if (bar != 0 && bar != 1) fle_stop(api, "code is not SB_HORZ or SB_VERT: the page lists these two");
    s = sb_get(w, 0);
    if (!s || !s->flat) {
        R[0] = 0;
        return;
    }
    mask = sb_read_info(api, R[8], &mn, &mx, &page, &pos);
    prev = s->bar[bar].pos;
    (void)sb_set(&s->bar[bar], mask, mn, mx, page, pos);
    R[0] = (uint32_t)prev;
}

void fle_imp_FlatSB_GetScrollInfo(void)
{
    static const char api[] = "FlatSB_GetScrollInfo";
    int32_t bar = (int32_t)(uint32_t)R[2], mn, mx, pos;
    uint32_t mask, page;
    int64_t w;
    struct wsb *s;

    fle_win_check_thread(api);
    w = win_arg(R[1]);
    if (w < 0) return;
    if (bar != 0 && bar != 1) fle_stop(api, "code is not SB_HORZ or SB_VERT: the page lists these two");
    s = sb_get(w, 0);
    if (!s || !s->flat) {
        R[0] = 0;
        return;
    }
    mask = sb_read_info(api, R[8], &mn, &mx, &page, &pos);
    sb_write_info(R[8], mask, &s->bar[bar]);
    R[0] = 1;
}

/* comctl32!FlatSB_SetScrollPos: int f(HWND rcx, int code edx, int pos r8d, BOOL fRedraw r9d). Learn: "Returns the previous position of the thumb"; FlatSB_GetScrollPos: "Returns the current thumb position of the specified flat scroll bar".
 * A window without InitializeFlatSB: 0 (a choice: the page names the requirement and gives no failure value). */
void fle_imp_FlatSB_SetScrollPos(void)
{
    static const char api[] = "FlatSB_SetScrollPos";
    int32_t bar = (int32_t)(uint32_t)R[2];
    int64_t w;
    struct wsb *s;
    int32_t prev;

    fle_win_check_thread(api);
    w = win_arg(R[1]);
    if (w < 0) return;
    if (bar != 0 && bar != 1) fle_stop(api, "code is not SB_HORZ or SB_VERT: the page lists these two");
    s = sb_get(w, 0);
    if (!s || !s->flat) {
        R[0] = 0;
        return;
    }
    prev = s->bar[bar].pos;
    (void)sb_set(&s->bar[bar], SIF_POS_, 0, 0, 0, (int32_t)(uint32_t)R[8]);
    R[0] = (uint32_t)prev;
}

void fle_imp_FlatSB_GetScrollPos(void)
{
    static const char api[] = "FlatSB_GetScrollPos";
    int32_t bar = (int32_t)(uint32_t)R[2];
    int64_t w;
    struct wsb *s;

    fle_win_check_thread(api);
    w = win_arg(R[1]);
    if (w < 0) return;
    if (bar != 0 && bar != 1) fle_stop(api, "code is not SB_HORZ or SB_VERT: the page lists these two");
    s = sb_get(w, 0);
    R[0] = (s && s->flat) ? (uint32_t)s->bar[bar].pos : 0u;
}

/* ---- window placement (R2, R8; decision 0123 wall 38, PROVISIONAL) ----
 * WINDOWPLACEMENT on x64 (Learn lists the members; the offsets are derived from the member types): UINT length @0, UINT flags @4, UINT showCmd @8, POINT ptMinPosition @12, POINT ptMaxPosition @20, RECT rcNormalPosition @28,
 * (decision 0267: there is no rcDevice in the SDK header outside _MAC, so the structure is 44 bytes; it was read as 60 = Learn's member list, and FL passes 44). Learn: the page of GetWindowPlacement says only that length must be sizeof(WINDOWPLACEMENT). Learn GetWindowPlacement: "The flags member ... retrieved by this function is always zero. If the
 * window ... is maximized, the showCmd member is SW_SHOWMAXIMIZED. If the window is minimized, showCmd is SW_SHOWMINIMIZED. Otherwise, it is SW_SHOWNORMAL"; "GetWindowPlacement fails if lpwndpl->length is not set correctly" (FALSE); the
 * coordinates of ptMinPosition, ptMaxPosition and rcNormalPosition are "workspace coordinates" for a top-level window without WS_EX_TOOLWINDOW and screen coordinates otherwise: the workspace is the screen minus the taskbar and the other
 * application toolbars: this environment has none (decision 0065: the work area is the monitor rectangle), so the two are the same numbers. ptMinPosition and ptMaxPosition: the page does not say what they hold when never set: (-1, -1)
 * is the value of the SDK's own documentation of the structure ("-1" = default) is NOT on the pages read: (-1, -1) is a CHOICE WITHOUT A SOURCE (S-placement) for both. rcNormalPosition: the restored rectangle (in the coordinates of the
 * window: parent client coordinates for a child: the page says "The window's coordinates when the window is in the restored position"). rcDevice: not described by the page (the member has no description): zeros. */
#define WPL_SIZE 44u   /* decision 0267 (alpha10): sizeof(WINDOWPLACEMENT) of the SDK header on x64 is 44 (UINT x3, POINT x2, RECT: rcDevice is declared only for _MAC; the Learn page lists it); Delphi's TWindowPlacement and Wine's header are 44, and FL passes length = 44: with 60 every call of FL failed (ERROR_INVALID_PARAMETER) and left the structure as it was */

void fle_imp_GetWindowPlacement(void)
{
    static const char api[] = "GetWindowPlacement";
    int64_t i;
    uint64_t p = R[2];
    uint32_t len;

    fle_win_check_thread(api);
    if (p == 0) fle_stop(api, "lpwndpl is NULL: the page gives no behaviour");
    i = win_arg(R[1]);
    if (i < 0) return;
    len = (uint32_t)fle_ld(p, 4);
    if (len != WPL_SIZE) {
        /* the page: "GetWindowPlacement fails if lpwndpl->length is not set correctly": FALSE; the code is a choice (87) */
        fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
        R[0] = 0;
        return;
    }
    /* flags: Learn GetWindowPlacement says "always zero"; Learn WINDOWPLACEMENT: WPF_RESTORETOMAXIMIZED (0x2) "The restored window will be maximized, regardless of whether it was maximized before it was minimized. This setting is
     * only valid the next time the window is restored ... only valid when the SW_SHOWMINIMIZED value is specified for the showCmd member": decision 0267 sets it for a minimized window that was maximized (Wine 10.0 does, and keeps it
     * afterwards: not copied). ptMinPosition: (-1, -1) until the window is minimized, (-32000, -32000) after (Wine 10.0, UNOFFICIAL: S-placement-minpos); ptMaxPosition (-1, -1) (Wine: also when maximized). */
    fle_st(p + 4, 4, (tab[i].iconic && tab[i].restore_max) ? 2u : 0u);
    fle_st(p + 8, 4, tab[i].zoomed ? (uint32_t)FLE_SW_SHOWMAXIMIZED : tab[i].iconic ? (uint32_t)FLE_SW_SHOWMINIMIZED : (uint32_t)FLE_SW_SHOWNORMAL);
    put_i32(p + 12, tab[i].min_set ? -32000 : -1);
    put_i32(p + 16, tab[i].min_set ? -32000 : -1);
    put_i32(p + 20, -1);
    put_i32(p + 24, -1);
    put_i32(p + 28, tab[i].rest_x);
    put_i32(p + 32, tab[i].rest_y);
    put_i32(p + 36, tab[i].rest_x + tab[i].rest_cx);
    put_i32(p + 40, tab[i].rest_y + tab[i].rest_cy);
    R[0] = 1;
}

/* user32!SetWindowPlacement: BOOL f(HWND rcx, const WINDOWPLACEMENT *rdx) (decision 0267, alpha10, PROVISIONAL). Learn: "Sets the show state and the restored, minimized, and maximized positions of the specified window";
 * flags WPF_SETMINPOSITION 0x1 "The x- and y-coordinates of the minimized window may be specified", WPF_RESTORETOMAXIMIZED 0x2 "The restored window will be maximized, regardless of whether it was maximized before it was minimized. This
 * setting is only valid the next time the window is restored ... only valid when the SW_SHOWMINIMIZED value is specified for the showCmd member"; length must be sizeof(WINDOWPLACEMENT) (44: FALSE and ERROR_INVALID_PARAMETER
 * otherwise, as GetWindowPlacement). The order (Wine 10.0, UNOFFICIAL, harness/alpha10/setplace_seq.c): the restored rectangle rcNormalPosition is stored, and a window in the normal state is moved and sized to it at once
 * (SetWindowPos with SWP_NOZORDER | SWP_NOACTIVATE, also when the window is hidden); then ShowWindow(showCmd) (SW_HIDE of a hidden window is nothing; a window that is hidden and gets SW_SHOWNORMAL is shown). A minimized or
 * maximized window keeps its state and only its restored rectangle changes. WPF_RESTORETOMAXIMIZED is applied after the show state. ptMinPosition and ptMaxPosition are not kept (the model has no icon position:
 * ptMinPosition is (-32000, -32000) after a minimize, (-1, -1) before; ptMaxPosition (-1, -1): GetWindowPlacement). */
void fle_imp_SetWindowPlacement(void)
{
    static const char api[] = "SetWindowPlacement";
    int64_t i;
    uint64_t p = R[2], h = R[1];
    uint32_t len, flags, show;
    int32_t l, t, r, b;

    fle_win_check_thread(api);
    if (p == 0) fle_stop(api, "lpwndpl is NULL: the page gives no behaviour");
    i = win_arg(h);
    if (i < 0) return;
    len = (uint32_t)fle_ld(p, 4);
    if (len != WPL_SIZE) {
        fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
        R[0] = 0;
        return;
    }
    flags = (uint32_t)fle_ld(p + 4, 4);
    show = (uint32_t)fle_ld(p + 8, 4);
    l = (int32_t)fle_ld(p + 28, 4);
    t = (int32_t)fle_ld(p + 32, 4);
    r = (int32_t)fle_ld(p + 36, 4);
    b = (int32_t)fle_ld(p + 40, 4);
    if (tab[i].iconic || tab[i].zoomed) {
        tab[i].rest_x = l;
        tab[i].rest_y = t;
        tab[i].rest_cx = r - l;
        tab[i].rest_cy = b - t;
    } else {
        apply_pos(api, i, 0, l, t, r - l, b - t, FLE_SWP_NOZORDER | FLE_SWP_NOACTIVATE);
    }
    {
        uint64_t sv4 = R[4];

        R[1] = h;
        R[2] = show;
        fle_imp_ShowWindow();
        R[4] = sv4;
    }
    if ((flags & 2u) && show == FLE_SW_SHOWMINIMIZED) tab[i].restore_max = 1;
    R[0] = 1;
}

/* ---- BeginDeferWindowPos, DeferWindowPos, EndDeferWindowPos (decision 0123 wall 39, PROVISIONAL) ----
 * Learn BeginDeferWindowPos: returns a handle that "identifies the multiple-window-position structure" (an internal structure); NULL when the system lacks resources; nNumWindows is the initial capacity (the structure grows).
 * DeferWindowPos: stores the target position; "All windows in a multiple-window-position structure must have the same parent"; "The handle returned by this function may differ from the handle passed to the function. The new handle
 * that this function returns should be passed during the next call"; "If a call to DeferWindowPos fails, the application should abandon the window-positioning operation and not call EndDeferWindowPos". EndDeferWindowPos: "sends the
 * WM_WINDOWPOSCHANGING and WM_WINDOWPOSCHANGED messages to each window identified in the internal structure"; nonzero on success; the order of the windows is the order of DeferWindowPos (a choice: the page gives none).
 * The handle of a structure is FLE_DWP_HANDLE_BASE + 4 * (k + 1) for the k-th BeginDeferWindowPos, and DeferWindowPos returns the SAME handle (the page allows a different one; a choice without a source, S-dwp). A handle that is not a live
 * structure: 0 and ERROR_INVALID_HANDLE... the page names no code: 6 (a choice). The structure is freed by EndDeferWindowPos. Up to 8 structures at once, 64 windows each (limits of this environment). */
#define DWP_BASE UINT64_C(0x7F970000)   /* 64 KiB of handles, directly before the menu region (FLE_MENU_HANDLE_BASE 0x7F980000); a choice without a source */
_Static_assert(DWP_BASE + 0x10000u <= FLE_MENU_HANDLE_BASE, "the DeferWindowPos handles lie before the menus");
_Static_assert(DWP_BASE > UINT64_C(0x7F000000), "the DeferWindowPos handles are far above the image");
#define DWP_MAX 8u
#define DWP_WIN_MAX 64u

typedef struct { int64_t w; uint64_t after; int32_t x, y, cx, cy; uint32_t flags; } dwp_ent;
static struct { int live; int64_t parent; uint32_t n; dwp_ent e[DWP_WIN_MAX]; } dwps[DWP_MAX];
static unsigned dwp_issued;

static int dwp_index(uint64_t h)
{
    uint64_t off;

    if (h <= DWP_BASE || h >= DWP_BASE + 0x10000u) return -1;
    off = h - DWP_BASE;
    if (off % 4u) return -1;
    off = off / 4u - 1u;
    return (off < dwp_issued && off < DWP_MAX && dwps[off].live) ? (int)off : -1;
}

void fle_imp_BeginDeferWindowPos(void)
{
    static const char api[] = "BeginDeferWindowPos";
    int32_t n = (int32_t)(uint32_t)R[1];
    unsigned k;

    fle_win_check_thread(api);
    if (n < 0) {
        fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
        R[0] = 0;
        return;
    }
    for (k = 0; k < DWP_MAX; k++)
        if (!dwps[k].live) break;
    if (k == DWP_MAX) {
        fle_set_last_error(8u);
        R[0] = 0;
        return;
    }
    memset(&dwps[k], 0, sizeof dwps[k]);
    dwps[k].live = 1;
    dwps[k].parent = -2;
    if (k >= dwp_issued) dwp_issued = k + 1u;
    R[0] = DWP_BASE + 4u * ((uint64_t)k + 1u);
}

void fle_imp_DeferWindowPos(void)
{
    static const char api[] = "DeferWindowPos";
    uint64_t hd = R[1], hw = R[2], after = R[8];
    int32_t x = (int32_t)(uint32_t)R[9], y = (int32_t)(uint32_t)fle_arg(5), cx = (int32_t)(uint32_t)fle_arg(6), cy = (int32_t)(uint32_t)fle_arg(7);
    uint32_t flags = (uint32_t)fle_arg(8);
    int d = dwp_index(hd);
    int64_t w;

    fle_win_check_thread(api);
    if (d < 0) {
        fle_set_last_error(FLE_W32_ERR_INVALID_HANDLE);
        R[0] = 0;
        return;
    }
    w = win_arg(hw);
    if (w < 0) return;
    if (dwps[d].parent == -2) dwps[d].parent = tab[w].parent;
    else if (dwps[d].parent != tab[w].parent) {
        /* "All windows in a multiple-window-position structure must have the same parent": a window with another parent makes the call fail (the page names no code: 87, a choice) */
        fle_set_last_error(FLE_ERROR_INVALID_PARAMETER_W);
        R[0] = 0;
        return;
    }
    if (dwps[d].n == DWP_WIN_MAX) {
        fle_set_last_error(8u);
        R[0] = 0;
        return;
    }
    dwps[d].e[dwps[d].n].w = w;
    dwps[d].e[dwps[d].n].after = after;
    dwps[d].e[dwps[d].n].x = x;
    dwps[d].e[dwps[d].n].y = y;
    dwps[d].e[dwps[d].n].cx = cx;
    dwps[d].e[dwps[d].n].cy = cy;
    dwps[d].e[dwps[d].n].flags = flags;
    dwps[d].n++;
    R[0] = hd;
}

void fle_imp_EndDeferWindowPos(void)
{
    static const char api[] = "EndDeferWindowPos";
    int d = dwp_index(R[1]);
    uint32_t k;
    uint64_t sp = R[4];

    fle_win_check_thread(api);
    if (d < 0) {
        fle_set_last_error(FLE_W32_ERR_INVALID_HANDLE);
        R[0] = 0;
        return;
    }
    for (k = 0; k < dwps[d].n; k++) {
        dwp_ent *e = &dwps[d].e[k];

        if (!tab[e->w].live) continue;
        apply_pos(api, e->w, e->after, e->x, e->y, e->cx, e->cy, e->flags);
        R[4] = sp;
    }
    dwps[d].live = 0;
    R[4] = sp;
    R[0] = 1;
}

/* user32!RedrawWindow: BOOL f(HWND rcx, const RECT *rdx, HRGN r8, UINT flags r9d). Learn: invalidates or validates the update region and controls repainting. hWnd NULL = the desktop window (not modelled: stops); hrgnUpdate non-NULL: regions are not
 * modelled (stops); "If both the hrgnUpdate and lprcUpdate parameters are NULL, the entire client area is added to the update region". Flags (Learn; the numbers are those of the SDK header, UNOFFICIAL: RDW_INVALIDATE 0x1, INTERNALPAINT 0x2, ERASE 0x4, VALIDATE 0x8,
 * NOINTERNALPAINT 0x10, NOERASE 0x20, NOCHILDREN 0x40, ALLCHILDREN 0x80, UPDATENOW 0x100, ERASENOW 0x200, FRAME 0x400, NOFRAME 0x800; decision 0262 (alpha8): NOFRAME and NOCHILDREN were 0x40 and 0x800, swapped, so FL's RDW_INVALIDATE | RDW_NOCHILDREN (0x41) invalidated the children of a window without WS_CLIPCHILDREN; the page lists the names): RDW_INVALIDATE adds the rectangle (the whole client area when none) to the update
 * region of the window (and with RDW_ALLCHILDREN of its descendants); RDW_VALIDATE removes it; RDW_ERASE (with INVALIDATE) marks the region for erasing; RDW_UPDATENOW sends WM_PAINT to the windows concerned before the function returns
 * ("WM_NCPAINT, WM_ERASEBKGND, and WM_PAINT messages, if necessary": the model sends WM_PAINT through the window procedure, whose default validates the region); RDW_ERASENOW sends WM_ERASEBKGND to a window whose region is marked
 * for erasing (Learn: WM_NCPAINT and WM_ERASEBKGND "if necessary"; the DC for WM_ERASEBKGND is the screen DC). RDW_INTERNALPAINT / RDW_NOINTERNALPAINT / RDW_FRAME / RDW_NOFRAME / RDW_NOERASE are accepted and have no state here (a WM_PAINT with an
 * empty region is not made: a choice, S-internalpaint). The result is nonzero. An invalid window: 0 and 1400. */
#define RDW_INVALIDATE_   0x0001u
#define RDW_INTERNALPAINT_ 0x0002u
#define RDW_ERASE_        0x0004u
#define RDW_VALIDATE_     0x0008u
#define RDW_NOINTERNALPAINT_ 0x0010u
#define RDW_NOERASE_      0x0020u
#define RDW_NOFRAME_      0x0800u
#define RDW_ALLCHILDREN_  0x0080u
#define RDW_UPDATENOW_    0x0100u
#define RDW_ERASENOW_     0x0200u
#define RDW_FRAME_        0x0400u
#define RDW_NOCHILDREN_   0x0040u
#define RDW_KNOWN_ (RDW_INVALIDATE_ | RDW_INTERNALPAINT_ | RDW_ERASE_ | RDW_VALIDATE_ | RDW_NOINTERNALPAINT_ | RDW_NOERASE_ | RDW_NOFRAME_ | RDW_ALLCHILDREN_ | RDW_UPDATENOW_ | RDW_ERASENOW_ | RDW_FRAME_ | RDW_NOCHILDREN_)

static void redraw_one(int64_t i, int have_rect, int32_t l, int32_t t, int32_t r, int32_t b, uint32_t fl)
{
    if (fl & RDW_INVALIDATE_) {
        int32_t cl = have_rect ? (l < 0 ? 0 : l) : 0, ct = have_rect ? (t < 0 ? 0 : t) : 0, cr = have_rect ? (r > tab[i].pcx ? tab[i].pcx : r) : tab[i].pcx,
                cb = have_rect ? (b > tab[i].pcy ? tab[i].pcy : b) : tab[i].pcy;

        if (cr > cl && cb > ct) upd_add(i, cl, ct, cr, cb, (fl & RDW_ERASE_) != 0);
    }
    if (fl & RDW_VALIDATE_) {
        if (!have_rect) {
            upd_clear_rect(i);
            tab[i].upd_erase = 0;
        } else upd_minus_rect(i, l, t, r, b);   /* decision 0247: the rectangle is removed from the update region */
    }
    if (fl & RDW_NOERASE_) tab[i].upd_erase = 0;
}

static void redraw_tree(int64_t i, int have_rect, int32_t l, int32_t t, int32_t r, int32_t b, uint32_t fl, int children)
{
    redraw_one(i, have_rect, l, t, r, b, fl);
    if (children) {
        int64_t idx[SIB_MAX];
        uint32_t n = siblings(i, idx, SIB_MAX), k;

        for (k = 0; k < n; k++) redraw_tree(idx[k], 0, 0, 0, 0, 0, fl, 1);
    }
}

/* Decision 0323: DefWindowProc(WM_ERASEBKGND). Learn WM_ERASEBKGND: "The DefWindowProc function erases the background by using the class background brush specified by the hbrBackground member of the WNDCLASS structure. If
 * hbrBackground is NULL, the application should process the WM_ERASEBKGND message and erase the background." Learn WNDCLASSW hbrBackground: a handle to the class background brush, or a system colour index plus 1 (the
 * values FillRect takes). Wine 10.0 (win32u/defwnd.c default_window_proc, UNOFFICIAL): no brush: the answer is 0 and nothing is drawn; else the clip box of the DC (a CS_PARENTDC class: the client rectangle, "can't use
 * GetClipBox with a parent DC or we fill the whole parent") is filled with the brush and the answer is 1. The DC given by BeginPaint / RedrawWindow is clipped to the update region (this file), so filling a
 * rectangle that covers it paints the update region and nothing else; a DC of a window therefore gets the client rectangle (fle_gdi.c's GetClipBox answers memory DCs only and stops for a window DC). A memory DC
 * (a test that sends the message with its own DC) gets its clip box. The DC of the entire screen stops by name (its clip box is not modelled). A brush that FillRect cannot take (a pattern brush) stops there. */
static uint64_t def_erase_background(const char *api, int64_t wi, uint64_t dc)
{
    uint64_t hbr = 0, sv[5];
    uint8_t rc[16];
    uint64_t prc = (uint64_t)(uintptr_t)rc;

    if (!fle_class_hbrbackground_of(tab[wi].atom, &hbr) || hbr == 0) return 0;
    if (!fle_imp_FillRect || !fle_imp_GetClipBox || !fle_gdc_is_window_dc) fle_stop(api, "no GDI is linked (fle_gdi.c, fle_gdc.c): the class background brush of the window cannot be filled (decision 0323)");
    memset(rc, 0, sizeof rc);
    if (fle_gdc_is_window_dc(dc)) {
        put_i32(prc + 8, tab[wi].pcx);
        put_i32(prc + 12, tab[wi].pcy);
    } else {
        uint64_t type;

        sv[0] = R[0]; sv[1] = R[1]; sv[2] = R[2]; sv[3] = R[8]; sv[4] = R[9];
        R[1] = dc;
        R[2] = prc;
        fle_imp_GetClipBox();
        type = R[0];
        R[0] = sv[0]; R[1] = sv[1]; R[2] = sv[2]; R[8] = sv[3]; R[9] = sv[4];
        if (type == 0) return 0;      /* ERROR: nothing to fill */
    }
    sv[0] = R[0]; sv[1] = R[1]; sv[2] = R[2]; sv[3] = R[8]; sv[4] = R[9];
    R[1] = dc;
    R[2] = prc;
    R[8] = hbr;
    fle_imp_FillRect();
    R[0] = sv[0]; R[1] = sv[1]; R[2] = sv[2]; R[8] = sv[3]; R[9] = sv[4];
    return 1;
}

/* Decision 0323: the WM_ERASEBKGND of a window whose update region is marked for erasing, as RDW_ERASENOW / RDW_UPDATENOW and the default WM_PAINT send it. wParam is the DC of the window (GetDC(hWnd), released when
 * the message has been answered), clipped to the update region while the message is handled (Wine 10.0 win32u/dce.c send_erase: GetDCEx with the update region, DCX_INTERSECTRGN | DCX_USESTYLE). The erase request
 * is cleared when the message was answered with nonzero (Learn WM_ERASEBKGND: "If the application returns zero, the window will remain marked for erasing"). decision 0253 (alpha5) made the DC a real one: a window
 * procedure that fills with Message.DC (TEdgeBrowser of FL: FillRect(0, ...)) stopped the run at RedrawWindow(RDW_ERASE | RDW_UPDATENOW) when it was 0. */
static void erase_now_one(const char *api, int64_t i)
{
    uint64_t sp = R[4], dc = 0, h = handle_of(i), er;
    int32_t pc[FLE_GDC_PAINT_PIECES][4];
    uint32_t np = 0, k;
    fle_visrgn uv;

    if (!fle_imp_GetDC || !fle_imp_ReleaseDC) fle_stop(api, "no virtual screen DC is linked (fle_gdc.c): the DC of WM_ERASEBKGND is not available");
    {
        uint64_t sv[5] = { R[0], R[1], R[2], R[8], R[9] };

        R[1] = h;
        fle_imp_GetDC();
        dc = R[0];
        R[0] = sv[0];
        R[1] = sv[1];
        R[2] = sv[2];
        R[8] = sv[3];
        R[9] = sv[4];
    }
    upd_get(i, &uv);
    if (uv.n > 1 && uv.n <= FLE_GDC_PAINT_PIECES) {
        np = uv.n;
        for (k = 0; k < np; k++) {
            pc[k][0] = (int32_t)uv.r[k].x0;
            pc[k][1] = (int32_t)uv.r[k].y0;
            pc[k][2] = (int32_t)uv.r[k].x1;
            pc[k][3] = (int32_t)uv.r[k].y1;
        }
    }
    if (fle_gdc_set_paint_rect) fle_gdc_set_paint_rect(dc, 1, tab[i].upd_l, tab[i].upd_t, tab[i].upd_r, tab[i].upd_b);
    if (np > 1 && fle_gdc_set_paint_pieces) fle_gdc_set_paint_pieces(dc, (const int32_t (*)[4])pc, np);
    er = send_to(api, i, FLE_WM_ERASEBKGND, dc, 0);
    R[4] = sp;
    if (fle_gdc_set_paint_rect) fle_gdc_set_paint_rect(dc, 0, 0, 0, 0, 0);
    {
        uint64_t sv[5] = { R[0], R[1], R[2], R[8], R[9] };

        R[1] = h;
        R[2] = dc;
        fle_imp_ReleaseDC();
        R[0] = sv[0];
        R[1] = sv[1];
        R[2] = sv[2];
        R[8] = sv[3];
        R[9] = sv[4];
    }
    /* Learn WM_ERASEBKGND: nonzero = "no further erasing is required"; zero = "the window will remain marked for erasing" (BeginPaint does the same: fErase is TRUE only when the message returned 0) */
    tab[i].upd_erase = 0;
    if (er == 0) tab[i].upd_delayed = 1;
}

static void paint_now_tree(const char *api, int64_t i, uint32_t fl, int children)
{
    uint64_t sp = R[4];

    if (tab[i].live && is_visible(i) && tab[i].upd_r > tab[i].upd_l && tab[i].upd_b > tab[i].upd_t) {
        if ((fl & RDW_ERASENOW_ || fl & RDW_UPDATENOW_) && tab[i].upd_erase) erase_now_one(api, i);
        if (fl & RDW_UPDATENOW_) {
            (void)send_to(api, i, FLE_WM_PAINT, 0, 0);
            R[4] = sp;
        }
    }
    if (children) {
        int64_t idx[SIB_MAX];
        uint32_t n = siblings(i, idx, SIB_MAX), k;

        for (k = 0; k < n; k++) paint_now_tree(api, idx[k], fl, 1);
    }
}

void fle_imp_RedrawWindow(void)
{
    static const char api[] = "RedrawWindow";
    uint64_t h = R[1], pr = R[2], rgn = R[8];
    uint32_t fl = (uint32_t)R[9];
    int64_t i;
    int have_rect = 0, children;
    int32_t l = 0, t = 0, r = 0, b = 0;

    fle_win_check_thread(api);
    if (h == 0) fle_stop(api, "hWnd is NULL (the desktop window): not modelled");
    if (fl & ~RDW_KNOWN_) fle_stop(api, "flags has a bit that the page does not list");
    i = win_arg(h);
    if (i < 0) return;
    /* decision 0126 (wall 91, S-redraw-rgn-bounds): "lprcUpdate ... is ignored if the hrgnUpdate parameter identifies a region". The update region of a window is kept as one bounding rectangle (S-updregion), so the region is
     * taken as its bounding box, in client coordinates like the rectangle (the page says "device units" for the rectangle and gives no unit for the region: a reading). A handle that is not a live region of fle_gdi.c stops
     * (the page does not define it). An empty region adds nothing (a reading: "the update region" is empty). */
    if (rgn != 0) {
        if (fle_gdi_region_bounds == NULL) fle_stop(api, "no GDI region table is linked");
        if (!fle_gdi_region_bounds(rgn, &l, &t, &r, &b)) fle_stop(api, "hrgnUpdate is not a live region of this runtime: the page does not define it");
        have_rect = 1;
        pr = 0;
        if (fle_gdi_region_pieces != NULL) {
            /* decision 0247: the region is added to / removed from the update region rectangle by rectangle (it was: its bounding box) */
            int32_t pc[128][4];
            uint32_t np = 0, k;

            if (fle_gdi_region_pieces(rgn, pc, 128, &np) > 0 && np > 1) {
                children = (fl & RDW_ALLCHILDREN_) && !(fl & RDW_NOCHILDREN_);
                for (k = 0; k < np; k++) redraw_tree(i, 1, pc[k][0], pc[k][1], pc[k][2], pc[k][3], fl & ~(uint32_t)(RDW_UPDATENOW_ | RDW_ERASENOW_), children);
                if ((fl & RDW_INVALIDATE_) && !(fl & (RDW_ALLCHILDREN_ | RDW_NOCHILDREN_)))
                    for (k = 0; k < np; k++) upd_children(i, pc[k][0], pc[k][1], pc[k][2], pc[k][3], (fl & RDW_ERASE_) != 0);
                if (fl & (RDW_UPDATENOW_ | RDW_ERASENOW_)) paint_now_tree(api, i, fl, children);
                R[0] = 1;
                return;
            }
        }
    }
    if (pr != 0) {
        have_rect = 1;
        l = (int32_t)fle_ld(pr, 4);
        t = (int32_t)fle_ld(pr + 4, 4);
        r = (int32_t)fle_ld(pr + 8, 4);
        b = (int32_t)fle_ld(pr + 12, 4);
    }
    /* "By default, the windows affected by RedrawWindow depend on whether the specified window has the WS_CLIPCHILDREN style. Child windows that are not the WS_CLIPCHILDREN style are unaffected": the model affects the descendants only
     * with RDW_ALLCHILDREN (a choice without a source for the default: the children of a non-WS_CLIPCHILDREN window are affected by the real default; the model leaves that out: S-redraw). */
    children = (fl & RDW_ALLCHILDREN_) && !(fl & RDW_NOCHILDREN_);
    redraw_tree(i, have_rect, l, t, r, b, fl, children);
    /* decision 0214 (GAP-U1, Learn RedrawWindow: "non-WS_CLIPCHILDREN windows are recursively validated or invalidated until a WS_CLIPCHILDREN window is encountered"): without RDW_ALLCHILDREN / RDW_NOCHILDREN the
     * invalidation goes to the children of a window that has no WS_CLIPCHILDREN, with the part of the rectangle that lies on each (the validation is not propagated: a child stays invalid, which only repaints it) */
    if ((fl & RDW_INVALIDATE_) && !(fl & (RDW_ALLCHILDREN_ | RDW_NOCHILDREN_))) {
        upd_children(i, have_rect ? l : 0, have_rect ? t : 0, have_rect ? r : tab[i].pcx, have_rect ? b : tab[i].pcy, (fl & RDW_ERASE_) != 0);
    }
    if (fl & (RDW_UPDATENOW_ | RDW_ERASENOW_)) paint_now_tree(api, i, fl, children);
    R[0] = 1;
}

/* ---- Decision 0136 (PROVISIONAL): what runtime/fle_input.c (the input round trip) needs of the window table ---- */

/* Decision 0323: every visible window that contains the screen point, in the order of Wine 10.0's list for window_from_point (server/window.c get_window_children_from_point and all_windows_from_point, UNOFFICIAL): the
 * siblings from the top of the z-order, and for each the windows below it that contain the point (recursively, the same rule) BEFORE the window itself. The first entry is what fle_win_input_hit gives. Learn WM_NCHITTEST,
 * HTTRANSPARENT: "In a window currently covered by another window in the same thread (the message will be sent to underlying windows in the same thread until one of them returns a code that is not HTTRANSPARENT)":
 * fle_input.c walks this list.
 * Decision 0324 (replaces S-disabled-drops of decision 0136, which found the deepest window even when it was disabled and dropped the event): a DISABLED CHILD window (WS_CHILD, not WS_POPUP, WS_DISABLED) is not in the
 * list, with everything below it, so the window beneath it, its parent, is found (Wine 10.0, server/window.c is_point_in_window: "disabled child"); the children of any other disabled window are not looked at
 * (get_window_children_from_point: "if point is in client area, and window is not minimized or disabled, check children"), so a disabled pop-up or top-level window is found itself, and fle_input.c answers it
 * with HTERROR (window_from_point: "If window is minimized or disabled, return at once"). Learn WindowFromPoint: "does not retrieve a handle to a hidden or disabled window". */
static unsigned hit_list_rec(int64_t parent, int32_t px, int32_t py, uint64_t *out, unsigned max, unsigned n)
{
    int64_t idx[SIB_MAX];
    uint32_t cnt = siblings(parent, idx, SIB_MAX), k;

    for (k = 0; k < cnt && n < max; k++) {
        int64_t i = idx[k];
        int32_t ox, oy;

        if (!tab[i].live || !(tab[i].style & FLE_WS_VISIBLE) || tab[i].iconic) continue;
        if ((tab[i].style & (FLE_WS_POPUP | FLE_WS_CHILD | FLE_WS_DISABLED)) == (FLE_WS_CHILD | FLE_WS_DISABLED)) continue;      /* decision 0324: a disabled child, and what is below it */
        if (tab[i].parent >= 0) client_origin(tab[i].parent, &ox, &oy);
        else ox = oy = 0;
        if (px < ox + tab[i].px || py < oy + tab[i].py || px >= ox + tab[i].px + tab[i].pcx || py >= oy + tab[i].py + tab[i].pcy) continue;
        if (!in_window_region(i, px - ox - tab[i].px, py - oy - tab[i].py)) continue;
        if (!(tab[i].style & FLE_WS_DISABLED)) n = hit_list_rec(i, px, py, out, max, n);      /* decision 0324: the children of a disabled window are not looked at */
        if (n < max) out[n++] = handle_of(i);
    }
    return n;
}

unsigned fle_win_input_hit_list(int32_t x, int32_t y, uint64_t *out, unsigned max)
{
    if (!tab_n || max == 0) return 0;
    return hit_list_rec(-1, x, y, out, max, 0);
}

uint64_t fle_win_input_hit(int32_t x, int32_t y)
{
    uint64_t h = 0;

    return fle_win_input_hit_list(x, y, &h, 1) ? h : 0;
}

int fle_win_input_view(uint64_t h, fle_win_iview *v)
{
    int64_t i = live_index(h);
    uint32_t cs = 0;

    if (i < 0) return 0;
    memset(v, 0, sizeof *v);
    v->live = 1;
    v->owner = tab[i].owner;
    v->style = tab[i].style;
    v->exstyle = tab[i].exstyle;
    if (fle_class_style_of(tab[i].atom, &cs)) v->class_style = cs;
    v->hcursor = fle_class_hcursor_of(tab[i].atom);
    v->enabled = is_enabled_chain(i);
    v->visible = is_visible(i) && !tab[root_of(i)].iconic;
    client_origin(i, &v->ox, &v->oy);
    v->cx = tab[i].pcx;
    v->cy = tab[i].pcy;
    v->parent = tab[i].parent >= 0 ? handle_of(tab[i].parent) : 0;
    v->root = handle_of(root_of(i));
    v->lid = tab[i].lid;
    {
        uint16_t nmu[FLE_CLASS_NAME_MAX_UNITS + 1], nl = 0, k;

        if (fle_class_name_of(tab[i].atom, nmu, &nl)) {
            for (k = 0; k < nl && k + 1 < sizeof v->cls; k++) v->cls[k] = nmu[k] < 128 ? (char)nmu[k] : '?';
        }
    }
    return 1;
}

uint64_t fle_win_input_send(const char *api, uint64_t h, uint32_t msg, uint64_t wp, uint64_t lp)
{
    int64_t i = live_index(h);
    uint64_t sv4 = R[4], r;

    if (i < 0) return 0;
    r = send_to(api, i, msg, wp, lp);
    R[4] = sv4;
    return r;
}

#define FLE_WS_EX_APPWINDOW_ 0x00040000u
/* Decision 0267 (alpha10): ShowWindow called by the system on behalf of a window procedure that passed WM_SYSCOMMAND to DefWindowProcW (fle_input.c). The registers of the running guest call are kept. */
void fle_win_input_showwindow(const char *api, uint64_t h, int cmd)
{
    uint64_t sv[3] = { R[0], R[1], R[2] }, sv4 = R[4];

    (void)api;
    R[1] = h;
    R[2] = (uint64_t)(uint32_t)cmd;
    fle_imp_ShowWindow();
    R[0] = sv[0];
    R[1] = sv[1];
    R[2] = sv[2];
    R[4] = sv4;
}

/* Decision 0267 (alpha10): the buttons of the task bar of the virtual machine (the host's input: the page shows them). Learn Extended Window Styles: WS_EX_APPWINDOW "Forces a top-level window onto the taskbar when the window is
 * visible"; WS_EX_TOOLWINDOW "A tool window does not appear in the taskbar or in the dialog that appears when the user presses ALT+TAB" (and an owned window has none: the shell's rule, UNOFFICIAL, S-taskbar-owner). So a button is
 * made for every live, visible, top-level window that is not message-only and has WS_EX_APPWINDOW, or has no owner and no WS_EX_TOOLWINDOW. Minimized windows keep their button (they are still WS_VISIBLE). Creation order
 * (the order of the window table). No class name, no window kind. Returns the number of buttons written (at most `max`). */
unsigned fle_win_taskbar_buttons(fle_taskbar_button *out, unsigned max)
{
    unsigned c = 0;
    uint64_t k;

    for (k = 0; k < tab_n && c < max; k++) {
        if (!tab[k].live || tab[k].msgonly || tab[k].parent >= 0 || !(tab[k].style & FLE_WS_VISIBLE)) continue;
        if (!((tab[k].exstyle & FLE_WS_EX_APPWINDOW_) || (tab[k].owner_win < 0 && !(tab[k].exstyle & FLE_WS_EX_TOOLWINDOW)))) continue;
        out[c].hwnd = handle_of((int64_t)k);
        out[c].iconic = tab[k].iconic != 0;
        out[c].zoomed = tab[k].zoomed != 0;
        out[c].lid = tab[k].lid;
        c++;
    }
    return c;
}

uint64_t fle_win_input_capture_any(unsigned *thread)
{
    unsigned t;

    inp_ensure();
    for (t = 0; t < MAXT; t++)
        if (inp[t].capture >= 0 && tab[inp[t].capture].live) {
            *thread = t;
            return handle_of(inp[t].capture);
        }
    return 0;
}

uint64_t fle_win_input_active(void)
{
    unsigned t;

    inp_ensure();
    if (fg_window < 0 || !tab[fg_window].live) return 0;
    t = tab[fg_window].owner;
    return (t < MAXT && inp[t].active >= 0 && tab[inp[t].active].live) ? handle_of(inp[t].active) : 0;
}

/* Learn About Keyboard Input: the keyboard input goes to the foreground window's thread, and there to the window with the keyboard focus; "If no window has the keyboard focus, the active window receives" the key
 * messages (WM_SYSKEYDOWN page). The foreground window is fle_win.c's fg_window (the first window activated, or the one SetForegroundWindow gave it). */
uint64_t fle_win_input_key_target(unsigned *thread)
{
    unsigned t;

    inp_ensure();
    if (fg_window >= 0 && tab[fg_window].live && tab[fg_window].owner < MAXT) {
        t = tab[fg_window].owner;
        *thread = t;
        if (inp[t].focus >= 0 && tab[inp[t].focus].live) return handle_of(inp[t].focus);
        if (inp[t].active >= 0 && tab[inp[t].active].live) return handle_of(inp[t].active);
        return 0;
    }
    /* S-key-target-fallback: no window was made the foreground window (SetFocus and SetActiveWindow do not set it in this model): the first thread that has a focus window takes the keys */
    for (t = 0; t < MAXT; t++)
        if (inp[t].focus >= 0 && tab[inp[t].focus].live) {
            *thread = t;
            return handle_of(inp[t].focus);
        }
    return 0;
}

/* Decision 0141 (PROVISIONAL), read-only for the browser front end (fle_appview.c): the live top-level windows in z-order, top first (the order of the list the stop report prints); the text of a window (ASCII, as the report
 * prints it) and the logical id of its owner window (0 = none). Nothing moves. */
unsigned fle_win_input_toplevels(uint64_t *handles, unsigned max)
{
    int64_t idx[SIB_MAX];
    uint32_t n, k, c = 0;

    if (!tab_n) return 0;
    n = siblings(-1, idx, SIB_MAX);
    for (k = 0; k < n && c < max; k++)
        if (tab[idx[k]].live && is_toplevel(idx[k])) handles[c++] = handle_of(idx[k]);
    return c;
}

/* Decision 0212 (PROVISIONAL, opt-in FLE_SCREEN_REMAXIMIZE=1; S-remaximize, an inference without a page: a window that was maximized is resized by Windows when the display changes, and this model does not do it by itself).
 * Every live top-level window in the zoomed state (ShowWindow SW_SHOWMAXIMIZED) keeps its position and is given the size it had PLUS the change of the screen (dw, dh): a window that covered the screen with an outset (the guest sets
 * (-5, -6, W + 10, H + 12) on its main form) keeps the outset. Done as the guest's own SetWindowPos would do it (WM_WINDOWPOSCHANGING, the position change, WM_SIZE with SIZE_MAXIMIZED). Returns the number of windows moved. */
unsigned fle_win_input_remaximize(int32_t dw, int32_t dh)
{
    uint64_t hs[64];
    unsigned n = fle_win_input_toplevels(hs, 64), k, moved = 0;

    for (k = 0; k < n; k++) {
        int64_t i = live_index(hs[k]);

        if (i < 0 || !tab[i].zoomed || tab[i].iconic) continue;
        apply_pos("display-change", i, 0, tab[i].px, tab[i].py, tab[i].pcx + dw, tab[i].pcy + dh, FLE_SWP_NOZORDER | FLE_SWP_NOACTIVATE);
        moved++;
    }
    return moved;
}

void fle_win_input_text(uint64_t h, char *out, unsigned cap, uint32_t *owner_lid)
{
    int64_t i = live_index(h);

    if (owner_lid) *owner_lid = 0;
    if (cap == 0) return;
    out[0] = 0;
    if (i < 0) return;
    u16_ascii(out, cap, tab[i].text, tab[i].text_len < cap ? tab[i].text_len : cap);
    if (owner_lid && tab[i].owner_win >= 0 && tab[i].owner_win < (int64_t)tab_n) *owner_lid = tab[tab[i].owner_win].lid;
}

void fle_win_input_activate(const char *api, uint64_t h, uint64_t how)
{
    int64_t i = live_index(h), r;

    if (i < 0) return;
    r = root_of(i);
    if (!is_visible(r) || !tab[r].enabled || (tab[r].exstyle & FLE_WS_EX_NOACTIVATE)) return;
    do_activate(api, r, how);
    fg_window = r;
}

/* A timer made by the HOST for a window of the table (decision 0136; a probe for the G3 record, NOT a call of the guest): the window procedure of the guest's own window then gets WM_TIMER from the scheduler exactly as it
 * does for a timer the guest made with SetTimer (owner thread = the window's thread, same table, same rules: minimum 10 ms, one WM_TIMER per retrieval, S-timer). Windows lets a process set timers only for windows
 * of the calling thread, so this is a test fixture and the record says so. Returns 1 when set. */
int fle_win_host_timer(uint64_t hwnd, uint64_t id, uint32_t ms)
{
    int64_t w = live_index(hwnd);
    unsigned k, slot = MAX_TIMERS;

    if (w < 0 || id == 0) return 0;
    if (ms < USER_TIMER_MINIMUM_) ms = USER_TIMER_MINIMUM_;
    for (k = 0; k < MAX_TIMERS; k++)
        if (timers[k].used && timers[k].win == w && timers[k].id == id) {
            slot = k;
            break;
        }
    if (slot == MAX_TIMERS)
        for (k = 0; k < MAX_TIMERS; k++)
            if (!timers[k].used) {
                slot = k;
                break;
            }
    if (slot == MAX_TIMERS) return 0;
    timers[slot].used = 1;
    timers[slot].win = w;
    timers[slot].thread = tab[w].owner;
    timers[slot].id = id;
    timers[slot].period_ms = ms;
    timers[slot].proc = 0;
    timers[slot].deadline = clk_now() + (uint64_t)ms * FLE_TICKS_PER_MS;
    return 1;
}

/* ---- Decision 0171 (PROVISIONAL, walls ia1-W4 and ia1-W5): the system caret (state only) and ShowOwnedPopups ---- */

/* The system caret: one per desktop (Learn CreateCaret: "The system's caret is a shared resource. A window should create a caret only when it has the keyboard focus or is active"; "CreateCaret automatically
 * destroys the previous caret shape, if any, regardless of the window that owns the caret"). STATE ONLY: nothing is drawn (the page says the system makes it flash; no blink timer exists here), the position and
 * the hide count are kept. S-ia1-caret-1. */
static struct { int exists; int64_t win; uint64_t bitmap; int32_t cx, cy, x, y; int32_t hide; } caret;

/* user32!CreateCaret: BOOL f(HWND hWnd rcx, HBITMAP hBitmap rdx, int nWidth r8d, int nHeight r9d). Learn: creates the shape (a line, block or bitmap) and assigns ownership to the window; hBitmap NULL = solid,
 * (HBITMAP)1 = gray; 0 for the width or height = the system border width/height (1 pixel at 96 DPI: SM_CXBORDER, SM_CYBORDER, S-ia1-caret-2); "The caret is initially hidden. To show the caret, call ShowCaret";
 * nonzero on success. An invalid window: 0 and ERROR_INVALID_WINDOW_HANDLE (win_arg). */
void fle_imp_CreateCaret(void)
{
    int64_t i;

    fle_win_check_thread("CreateCaret");
    i = win_arg(R[1]);
    if (i < 0) return;
    caret.exists = 1;
    caret.win = i;
    caret.bitmap = R[2];
    caret.cx = (int32_t)(uint32_t)R[8] ? (int32_t)(uint32_t)R[8] : 1;
    caret.cy = (int32_t)(uint32_t)R[9] ? (int32_t)(uint32_t)R[9] : 1;
    caret.x = caret.y = 0;
    caret.hide = 1;
    R[0] = 1;
}

/* user32!DestroyCaret: BOOL f(void). Learn: "Destroys the caret's current shape, frees the caret from the window that owns it, and removes the caret from the screen. If the calling thread does not own the caret,
 * the caret is not destroyed" and the return value is zero; nonzero otherwise. The page names no error code (the last error is left as it is). */
void fle_imp_DestroyCaret(void)
{
    fle_win_check_thread("DestroyCaret");
    if (!caret.exists) {
        R[0] = 0;
        return;
    }
    caret.exists = 0;
    caret.win = -1;
    caret.hide = 0;
    R[0] = 1;
}

/* user32!HideCaret and ShowCaret: BOOL f(HWND hWnd rcx). Learn: hWnd "the window that owns the caret; if NULL, searches the current task for the window that owns the caret"; hiding is cumulative ("If HideCaret
 * has been called five times in a row, ShowCaret must be called five times to show the caret"); ShowCaret shows the caret only if the window owns it and it has a shape; nonzero on success, zero on failure.
 * Failure here: no caret, or a window that does not own it (the page names no error code: the last error is left as it is). */
static int caret_owner_arg(void)
{
    if (!caret.exists) return 0;
    if (R[1] == 0) return 1;
    return live_index(R[1]) == caret.win;
}

void fle_imp_HideCaret(void)
{
    fle_win_check_thread("HideCaret");
    if (!caret_owner_arg()) {
        R[0] = 0;
        return;
    }
    caret.hide++;
    R[0] = 1;
}

void fle_imp_ShowCaret(void)
{
    fle_win_check_thread("ShowCaret");
    if (!caret_owner_arg()) {
        R[0] = 0;
        return;
    }
    if (caret.hide > 0) caret.hide--;
    R[0] = 1;
}

/* user32!SetCaretPos: BOOL f(int X ecx, int Y edx). Learn: "Moves the caret to the specified coordinates" (client coordinates of the owner window); nonzero on success; zero when there is no caret (no error
 * code is named). */
void fle_imp_SetCaretPos(void)
{
    fle_win_check_thread("SetCaretPos");
    if (!caret.exists) {
        R[0] = 0;
        return;
    }
    caret.x = (int32_t)(uint32_t)R[1];
    caret.y = (int32_t)(uint32_t)R[2];
    R[0] = 1;
}

/* Inspection for tests: 0 = no caret; else 1 + (hidden ? 0 : 2); position in *x, *y. */
int fle_win_caret_state(int32_t *x, int32_t *y, int32_t *hide)
{
    if (!caret.exists) return 0;
    if (x) *x = caret.x;
    if (y) *y = caret.y;
    if (hide) *hide = caret.hide;
    return 1;
}

/* user32!ShowOwnedPopups: BOOL f(HWND hWnd rcx, BOOL fShow edx). Learn: "Shows or hides all pop-up windows owned by the specified window"; TRUE: "all hidden pop-up windows are shown", FALSE: "all visible pop-up
 * windows are hidden"; Remarks: "ShowOwnedPopups shows only windows hidden by a previous call to ShowOwnedPopups" (a pop-up hidden with ShowWindow is not shown by TRUE); nonzero on success. A pop-up window is a
 * window with WS_POPUP owned (owner_win) by the window; descendants of an owned window are not visited (the page does not say). An invalid window: 0 and ERROR_INVALID_WINDOW_HANDLE. */
void fle_imp_ShowOwnedPopups(void)
{
    int64_t i, k;
    int show = (uint32_t)R[2] != 0;

    fle_win_check_thread("ShowOwnedPopups");
    i = win_arg(R[1]);
    if (i < 0) return;
    for (k = 0; k < (int64_t)tab_n; k++) {
        if (!tab[k].live || tab[k].owner_win != i || !(tab[k].style & FLE_WS_POPUP)) continue;
        if (!show && tab[k].visible) {
            tab[k].sop_hidden = 1;
            tab[k].style &= ~FLE_WS_VISIBLE;
            tab[k].visible = 0;
        } else if (show && tab[k].sop_hidden) {
            tab[k].sop_hidden = 0;
            tab[k].style |= FLE_WS_VISIBLE;
            tab[k].visible = 1;
            note_visibility_changed();
        }
    }
    R[0] = 1;
}

/* user32!LogicalToPhysicalPoint and PhysicalToLogicalPoint: BOOL f(HWND hWnd rcx, LPPOINT lpPoint rdx) (decision 0171, PROVISIONAL, wall ia1-W8: delay-loaded by the engine; the delay-load helper raised
 * 0xC06D007F because the virtual user32 had no such export, which stopped the tempo box, the pattern selector and others). Learn: "Converts the logical (physical) coordinates of a point in a window to physical
 * (logical) coordinates"; hWnd "the window whose transform is used for the conversion. Top level windows are fully supported. In the case of child windows, only the area of overlap with the parent window is
 * supported"; lpPoint "[in, out] ... The new coordinates are copied into this structure if the function succeeds"; nonzero on success. The process is DPI-unaware on one 96 DPI monitor (decisions 0053, 0099), so
 * logical and physical coordinates are the same: the point is left as it is and TRUE is returned (S-ia1-l2p-1). A window that is not live: 0 and ERROR_INVALID_WINDOW_HANDLE (win_arg); a NULL lpPoint stops (the
 * page gives no behaviour). */
static void point_identity(const char *api)
{
    int64_t i;

    fle_win_check_thread(api);
    i = win_arg(R[1]);
    if (i < 0) return;
    if (R[2] == 0) fle_stop(api, "lpPoint is NULL: the page gives no behaviour");
    (void)fle_ld(R[2], 8);
    R[0] = 1;
}

void fle_imp_LogicalToPhysicalPoint(void) { point_identity("LogicalToPhysicalPoint"); }
void fle_imp_PhysicalToLogicalPoint(void) { point_identity("PhysicalToLogicalPoint"); }

/* user32!SetWindowRgn: int f(HWND hWnd rcx, HRGN hRgn rdx, BOOL bRedraw r8d) (decision 0171, PROVISIONAL, wall ia1-W15: the engine's pop-up menu window shapes itself with it when an item is clicked). Learn: "Sets the window region of a window. The
 * window region determines the area within the window where the system permits drawing. The system does not display any portion of a window that lies outside of the window region"; hRgn NULL "sets the window region to NULL"; bRedraw TRUE: "the
 * system sends the WM_WINDOWPOSCHANGING and WM_WINDOWPOSCHANGED messages to the window"; "After a successful call to SetWindowRgn, the operating system owns the region specified by the region handle hRgn ... do not delete this region
 * handle. The operating system deletes the region handle when it is no longer needed"; nonzero on success, zero on failure. Answered: a live window and hRgn NULL or a live region of the GDI model (fle_gdi_region_bounds); the region the window had is
 * deleted (the system owned it); WM_WINDOWPOSCHANGING / WM_WINDOWPOSCHANGED go out through apply_pos with SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE when bRedraw is nonzero. Decision 0244 replaces S-ia1-rgn-1 (state only): the region is kept, the compositor (fle_desk.c) draws only the part of
 * the window inside it and the pointer's hit test (WindowFromPoint, the input of fle_input.c) finds the window only inside it (UNOFFICIAL, see in_window_region). An invalid window: 0 and ERROR_INVALID_WINDOW_HANDLE; a hRgn that is not a region of this model stops by name. */
/* The GDI model is optional in a link (tests/window.rs links fle_win.c without fle_gdi.c): the two references are weak, and SetWindowRgn stops by name when they are missing. */
extern int fle_gdi_region_bounds(uint64_t h, int32_t *l, int32_t *t, int32_t *r, int32_t *b) __attribute__((weak));
extern void fle_imp_DeleteObject(void) __attribute__((weak));

void fle_imp_SetWindowRgn(void)
{
    static const char api[] = "SetWindowRgn";
    uint64_t h = R[1], rgn = R[2];
    uint32_t redraw = (uint32_t)R[8];
    int64_t i;
    int32_t l, t, r, b;

    fle_win_check_thread(api);
    i = win_arg(h);
    if (i < 0) return;
    if (!fle_gdi_region_bounds || !fle_imp_DeleteObject) fle_stop(api, "the GDI region model (fle_gdi.c) is not linked");
    if (rgn != 0 && !fle_gdi_region_bounds(rgn, &l, &t, &r, &b)) fle_stop(api, "hRgn is not NULL and not a region made by CreateRectRgn / CreateRectRgnIndirect: other regions are not modelled");
    if (tab[i].rgn != 0 && tab[i].rgn != rgn) {
        uint64_t sp = R[4];

        R[1] = tab[i].rgn;
        R[4] = sp - 8;
        fle_st(R[4], 8, 0xAAAA0003);
        fle_imp_DeleteObject();
        R[4] = sp;
    }
    tab[i].rgn = rgn;
    if (redraw) apply_pos(api, i, 0, tab[i].px, tab[i].py, tab[i].pcx, tab[i].pcy, FLE_SWP_NOMOVE | FLE_SWP_NOSIZE | FLE_SWP_NOZORDER | FLE_SWP_NOACTIVATE);
    R[0] = 1;
}

/* Inspection for tests: the region handle that SetWindowRgn gave the window (0 = none). */
uint64_t fle_win_window_rgn(uint64_t hwnd)
{
    int64_t i = index_of(hwnd);

    return i >= 0 ? tab[i].rgn : 0;
}

/* Decision 0244 (alpha1): the box of the window region in the window's own coordinates (SetWindowRgn, Learn: "The window region determines the area within the window where the system permits drawing. The system does not display any portion of a
 * window that lies outside of the window region"; the coordinates of the region are relative to the upper-left corner of the window: SetWindowRgn, Remarks). 1 and the box [l, r) x [t, b) when the window has a region of the GDI model, else 0. The
 * GDI model has rectangular regions only (CreateRectRgn / CreateRectRgnIndirect), so the box is the region. */
int fle_win_window_region_box(uint64_t hwnd, int32_t *l, int32_t *t, int32_t *r, int32_t *b)
{
    int64_t i = index_of(hwnd);

    if (i < 0 || tab[i].rgn == 0 || !fle_gdi_region_bounds) return 0;
    return fle_gdi_region_bounds(tab[i].rgn, l, t, r, b);
}

/* Decision 0244 (alpha1): the MARGINS that DwmExtendFrameIntoClientArea gave the window. Learn: "Extends the window frame into the client area"; "Use negative margin values to create the 'sheet of glass' effect where the client area is rendered as a
 * solid surface with no window border" (the page's own sample is MARGINS {-1}: one negative member is enough). A call with four zeros takes the extension away (S-alpha1-dwm-zero: the page says nothing about it; zero margins extend nothing). */
void fle_win_set_frame_margins(uint64_t hwnd, const int32_t m[4])
{
    int64_t i = index_of(hwnd);

    if (i < 0) return;
    for (int k = 0; k < 4; k++) tab[i].dwm_margins[k] = m[k];
}

/* 1 and the margins when the window's frame is extended into its client area (some margin is not 0), else 0. */
int fle_win_frame_margins(uint64_t hwnd, int32_t m[4])
{
    int64_t i = index_of(hwnd);

    if (i < 0) return 0;
    if (!(tab[i].dwm_margins[0] | tab[i].dwm_margins[1] | tab[i].dwm_margins[2] | tab[i].dwm_margins[3])) return 0;
    for (int k = 0; k < 4; k++) m[k] = tab[i].dwm_margins[k];
    return 1;
}
