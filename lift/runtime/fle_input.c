/* Decision 0136 (PROVISIONAL): the input round trip (see fle_input.h). The host queues raw events (fle_input_host_*); the thread that owns the target window turns them into messages when it retrieves them
 * (GetMessageW / PeekMessageW call fle_input_synth after the posted messages): the order of Learn "About Messages and Message Queues" (sent, posted, input, sent, WM_PAINT, WM_TIMER). Every number that a Learn
 * page does not give is a choice WITHOUT A SOURCE, marked S-... (listed with the withdrawal condition in docs/decisions/0136-input-roundtrip-and-timers.md). Learn pages read:
 *   WM_NCHITTEST (inputdev/wm-nchittest), WM_SETCURSOR (menurc/wm-setcursor), WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_MOUSELEAVE, WM_MOUSEHOVER, TrackMouseEvent, TRACKMOUSEEVENT, _TrackMouseEvent,
 *   WM_KEYDOWN, WM_CHAR, GetDoubleClickTime, GetMessageTime, GetMessagePos, MessageBeep (quoted below where they decide a value). */
#define FLE_INPUT_IMPL 1
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_sys.h"
#include "fle_gdc.h"
#include "fle_screen.h"
#include "fle_thread.h"
#include "fle_win.h"
#include "fle_cursor.h"
#include "fle_input.h"

void fle_imp_SetCursor(void) __attribute__((weak));
static uint64_t clk(void) { return fle_sys_clock_ticks(); }
/* ------------------------------------------------------------------------------------------------------------- the key and button state */
#define VK_LBUTTON_ 0x01u
#define VK_RBUTTON_ 0x02u
#define VK_MBUTTON_ 0x04u
#define VK_SHIFT_   0x10u
#define VK_CONTROL_ 0x11u
#define VK_MENU_    0x12u
#define VK_CAPITAL_ 0x14u
#define VK_F4_      0x73u
#define VK_F10_     0x79u
#define VK_NUMLOCK_ 0x90u
#define VK_SCROLL_  0x91u
#define VK_LSHIFT_  0xA0u
#define VK_RSHIFT_  0xA1u
#define VK_LCONTROL_ 0xA2u
#define VK_RCONTROL_ 0xA3u
#define VK_LMENU_   0xA4u
#define VK_RMENU_   0xA5u

static uint8_t key_async[256];     /* GetAsyncKeyState: the hardware state at the time of the fle_input_host_* call (bit 7 down, bit 0 toggled) */
static uint8_t key_sync[256];      /* GetKeyState / GetKeyboardState: "changes as a thread reads key messages from its message queue" (Learn GetKeyState): set when the message is made */

static void set_key(uint8_t *t, uint32_t vk, int down)
{
    uint32_t g = 0;

    if (vk > 255) return;
    if (down && !(t[vk] & 0x80) && (vk == VK_CAPITAL_ || vk == VK_NUMLOCK_ || vk == VK_SCROLL_)) t[vk] ^= 1u;
    t[vk] = (uint8_t)((t[vk] & 1u) | (down ? 0x80u : 0u));
    /* the generic codes follow their left / right keys (Learn GetKeyState: VK_SHIFT, VK_CONTROL, VK_MENU and VK_LSHIFT ... are accepted) */
    if (vk == VK_LSHIFT_ || vk == VK_RSHIFT_) g = VK_SHIFT_;
    else if (vk == VK_LCONTROL_ || vk == VK_RCONTROL_) g = VK_CONTROL_;
    else if (vk == VK_LMENU_ || vk == VK_RMENU_) g = VK_MENU_;
    if (g) {
        int any = (t[g == VK_SHIFT_ ? VK_LSHIFT_ : g == VK_CONTROL_ ? VK_LCONTROL_ : VK_LMENU_] & 0x80) || (t[g == VK_SHIFT_ ? VK_RSHIFT_ : g == VK_CONTROL_ ? VK_RCONTROL_ : VK_RMENU_] & 0x80);

        t[g] = (uint8_t)((t[g] & 1u) | (any ? 0x80u : 0u));
    }
}

/* Learn GetKeyState: "If the high-order bit is 1, the key is down; otherwise, it is up. If the low-order bit is 1, the key is toggled." The result is a SHORT: bit 15 and bit 0. Learn GetAsyncKeyState: the same layout
 * for the state "at the time of the call" (the hardware state). The mouse buttons are virtual keys 1, 2 and 4 in both. */
uint64_t fle_input_key_state(uint32_t vk, int async)
{
    const uint8_t *t = async ? key_async : key_sync;
    uint8_t s;

    vk &= 0xFFu;
    s = t[vk];
    return (uint64_t)(((s & 0x80u) ? 0x8000u : 0u) | (s & 1u));
}

static uint32_t mk_flags(void)
{
    uint32_t f = 0;

    if (key_async[VK_LBUTTON_] & 0x80) f |= 0x0001u;
    if (key_async[VK_RBUTTON_] & 0x80) f |= 0x0002u;
    if (key_async[VK_SHIFT_] & 0x80) f |= 0x0004u;
    if (key_async[VK_CONTROL_] & 0x80) f |= 0x0008u;
    if (key_async[VK_MBUTTON_] & 0x80) f |= 0x0010u;
    return f;
}

/* The US scan codes (set 1) of the virtual keys (S-us-layout; see MapVirtualKeyW below). */
static const uint8_t scan_of_vk[256] = {
    [0x08] = 0x0E, [0x09] = 0x0F, [0x0D] = 0x1C, [0x10] = 0x2A, [0x11] = 0x1D, [0x12] = 0x38, [0x14] = 0x3A, [0x1B] = 0x01, [0x20] = 0x39, [0x21] = 0x49, [0x22] = 0x51, [0x23] = 0x4F, [0x24] = 0x47,
    [0x25] = 0x4B, [0x26] = 0x48, [0x27] = 0x4D, [0x28] = 0x50, [0x2D] = 0x52, [0x2E] = 0x53,
    ['0'] = 0x0B, ['1'] = 0x02, ['2'] = 0x03, ['3'] = 0x04, ['4'] = 0x05, ['5'] = 0x06, ['6'] = 0x07, ['7'] = 0x08, ['8'] = 0x09, ['9'] = 0x0A,
    ['A'] = 0x1E, ['B'] = 0x30, ['C'] = 0x2E, ['D'] = 0x20, ['E'] = 0x12, ['F'] = 0x21, ['G'] = 0x22, ['H'] = 0x23, ['I'] = 0x17, ['J'] = 0x24, ['K'] = 0x25, ['L'] = 0x26, ['M'] = 0x32,
    ['N'] = 0x31, ['O'] = 0x18, ['P'] = 0x19, ['Q'] = 0x10, ['R'] = 0x13, ['S'] = 0x1F, ['T'] = 0x14, ['U'] = 0x16, ['V'] = 0x2F, ['W'] = 0x11, ['X'] = 0x2D, ['Y'] = 0x15, ['Z'] = 0x2C,
    [0x70] = 0x3B, [0x71] = 0x3C, [0x72] = 0x3D, [0x73] = 0x3E, [0x74] = 0x3F, [0x75] = 0x40, [0x76] = 0x41, [0x77] = 0x42, [0x78] = 0x43, [0x79] = 0x44, [0x7A] = 0x57, [0x7B] = 0x58,
    [0xA0] = 0x2A, [0xA1] = 0x36, [0xA2] = 0x1D, [0xA3] = 0x1D, [0xA4] = 0x38, [0xA5] = 0x38,
    [0xBA] = 0x27, [0xBB] = 0x0D, [0xBC] = 0x33, [0xBD] = 0x0C, [0xBE] = 0x34, [0xBF] = 0x35, [0xC0] = 0x29, [0xDB] = 0x1A, [0xDC] = 0x2B, [0xDD] = 0x1B, [0xDE] = 0x28,
};

/* ------------------------------------------------------------------------------------------------------------- the pointer */
void fle_input_pointer(int32_t *x, int32_t *y) { fle_win_pointer_get(x, y); }

/* ------------------------------------------------------------------------------------------------------------- the raw event queue */
typedef struct {
    int kind;
    int32_t x, y;           /* the pointer at the event (screen) */
    int32_t data;           /* button / wheel delta */
    uint32_t vk, scan;
    int ext, repeat;
    uint32_t mk;            /* MK_* flags at the event (the keys and buttons the host held when it fed the event in) */
    uint64_t hwnd;          /* decision 0267: FLE_IN_SYSCMD: the window the task bar addresses */
    uint64_t ticks;
} hwev;
#define EV_MAX 512u
static hwev evq[EV_MAX];
static unsigned ev_head, ev_n;
static unsigned last_consumer = (unsigned)-1;   /* decision 0280, L1c: the thread that last took a host event out of the queue or retrieved a message made of one */

static int ev_thread(const hwev *e, unsigned *thread);

/* Learn GetQueueStatus / MsgWaitForMultipleObjects: QS_MOUSEMOVE, QS_MOUSEBUTTON and QS_KEY say "new input" in the queue of the thread the event is for (WaitMessage and MsgWait return for them). */
static void note_changebits(const hwev *e)
{
    unsigned t;
    fle_tqueue *q;

    if (!ev_thread(e, &t)) return;
    q = fle_thread_queue(t, 1);
    if (q) q->changebits |= e->kind == FLE_IN_MOVE ? FLE_QS_MOUSEMOVE : (e->kind == FLE_IN_KDOWN || e->kind == FLE_IN_KUP) ? FLE_QS_KEY : FLE_QS_MOUSEBUTTON;
}

int fle_input_host_mouse(int kind, int32_t x, int32_t y, int32_t data)
{
    hwev *e;
    unsigned b;

    if (kind != FLE_IN_MOVE && kind != FLE_IN_BDOWN && kind != FLE_IN_BUP && kind != FLE_IN_WHEEL) return 0;
    if (ev_n >= EV_MAX) return 0;
    /* S-pointer-clamp: the pointer cannot leave the virtual screen (the page of SetCursorPos adjusts a point outside the clip rectangle; the screen is the rectangle, as fle_win.c's setter does) */
    if (x < 0) x = 0;
    if (y < 0) y = 0;
    if (x > FLE_GDC_HORZRES - 1) x = FLE_GDC_HORZRES - 1;
    if (y > FLE_GDC_VERTRES - 1) y = FLE_GDC_VERTRES - 1;
    fle_win_pointer_set(x, y);
    if (kind == FLE_IN_BDOWN || kind == FLE_IN_BUP) {
        b = data == (int32_t)FLE_IN_BTN_R ? VK_RBUTTON_ : data == (int32_t)FLE_IN_BTN_M ? VK_MBUTTON_ : VK_LBUTTON_;
        set_key(key_async, b, kind == FLE_IN_BDOWN);
    }
    e = &evq[(ev_head + ev_n) % EV_MAX];
    memset(e, 0, sizeof *e);
    e->kind = kind;
    e->x = x;
    e->y = y;
    e->data = data;
    e->mk = mk_flags();
    e->ticks = clk();
    ev_n++;
    note_changebits(e);
    fle_thread_wake_check();
    return 1;
}

int fle_input_host_key(int kind, uint32_t vk, uint32_t scan, int extended)
{
    hwev *e;

    if (kind != FLE_IN_KDOWN && kind != FLE_IN_KUP) return 0;
    if (ev_n >= EV_MAX || vk == 0 || vk > 255) return 0;
    e = &evq[(ev_head + ev_n) % EV_MAX];
    memset(e, 0, sizeof *e);
    e->kind = kind;
    fle_win_pointer_get(&e->x, &e->y);
    e->vk = vk;
    e->scan = scan;
    e->ext = extended != 0;
    e->repeat = kind == FLE_IN_KDOWN && (key_async[vk] & 0x80) != 0;     /* a key that is down and goes down again: auto-repeat */
    e->ticks = clk();
    set_key(key_async, vk, kind == FLE_IN_KDOWN);
    ev_n++;
    note_changebits(e);
    fle_thread_wake_check();
    return 1;
}

unsigned fle_input_host_pending(void) { return ev_n; }

int fle_input_host_display(uint32_t w, uint32_t h)
{
    hwev *e;

    if (w < FLE_SCREEN_MIN_W || w > FLE_SCREEN_MAX_W || h < FLE_SCREEN_MIN_H || h > FLE_SCREEN_MAX_H) return 0;
    if (ev_n >= EV_MAX) return 0;
    e = &evq[(ev_head + ev_n) % EV_MAX];
    memset(e, 0, sizeof *e);
    e->kind = FLE_IN_DISPLAY;
    e->x = (int32_t)w;
    e->y = (int32_t)h;
    e->ticks = clk();
    ev_n++;
    note_changebits(e);
    fle_thread_wake_check();
    return 1;
}

/* Decision 0267 (alpha10): see fle_input.h. The event is for the thread that owns the window (ev_thread); the system command is sent when that thread retrieves messages (translate: apply_syscmd). */
int fle_input_host_syscommand(uint64_t hwnd, uint32_t cmd)
{
    hwev *e;
    fle_win_iview v;

    if (cmd != 0xF020u && cmd != 0xF030u && cmd != 0xF060u && cmd != 0xF120u) return 0;
    if (!fle_win_input_view(hwnd, &v) || v.parent) return 0;
    if (ev_n >= EV_MAX) return 0;
    e = &evq[(ev_head + ev_n) % EV_MAX];
    memset(e, 0, sizeof *e);
    e->kind = FLE_IN_SYSCMD;
    e->hwnd = hwnd;
    e->data = (int32_t)cmd;
    e->ticks = clk();
    ev_n++;
    note_changebits(e);
    fle_thread_wake_check();
    return 1;
}

/* ------------------------------------------------------------------------------------------------------------- tracking (TrackMouseEvent), the window under the cursor */
#define TRK_MAX 32u
typedef struct {
    int used;
    uint64_t hwnd;
    uint32_t flags;           /* TME_HOVER, TME_LEAVE, TME_NONCLIENT */
    uint32_t hover_ms;        /* the actual time-out (never HOVER_DEFAULT) */
    uint64_t hover_due;       /* virtual ticks, 0 = no hover timer */
    int32_t ax, ay;           /* the hover rectangle's centre (screen) */
} trk;
static trk trks[TRK_MAX];
static uint64_t cursor_win;   /* the window the pointer was in at the last translated move */

/* S-hover-defaults: Learn TRACKMOUSEEVENT: "The system default hover time-out is initially the menu drop-down time, which is 400 milliseconds"; "The system default hover rectangle is the same as the double-click rectangle"
 * (SM_CXDOUBLECLK x SM_CYDOUBLECLK; the page gives no size: 4 x 4 pixels, the value fle_sysmetrics.c answers for SM_CXDOUBLECLK, is used, centred on the point where the hover began). */
#define HOVER_DEFAULT_MS 400u
#define HOVER_RECT_W 4
#define HOVER_RECT_H 4

static trk *trk_find(uint64_t h)
{
    unsigned k;

    for (k = 0; k < TRK_MAX; k++)
        if (trks[k].used && trks[k].hwnd == h) return &trks[k];
    return NULL;
}

static void trk_drop(trk *t) { memset(t, 0, sizeof *t); }

void fle_input_window_gone(uint64_t hwnd)
{
    trk *t = trk_find(hwnd);

    if (t) trk_drop(t);
    if (cursor_win == hwnd) cursor_win = 0;
}

/* Is the pointer over window h: the deepest visible window at the pointer is h. Learn TrackMouseEvent: "If the mouse is not over the specified window or area, a leave notification is generated immediately and no
 * further tracking is performed" (S-over: "over" is the window found by the hit test of the pointer, so a child under the pointer takes it from its parent, as it does for the messages). */
static int pointer_over(uint64_t h)
{
    int32_t x, y;

    fle_win_pointer_get(&x, &y);
    return fle_win_input_hit(x, y) == h;
}

static uint32_t lparam_xy(int32_t x, int32_t y) { return ((uint32_t)(uint16_t)y << 16) | (uint32_t)(uint16_t)x; }

/* comctl32!_TrackMouseEvent / user32!TrackMouseEvent. Learn TRACKMOUSEEVENT: TME_CANCEL "The caller wants to cancel a prior tracking request. The caller should also specify the type of tracking that it wants to
 * cancel"; TME_HOVER "If the caller requests hover tracking while hover tracking is already active, the hover timer will be reset. This flag is ignored if the mouse pointer is not over the specified window or area";
 * TME_LEAVE "If the mouse is not over the specified window or area, a leave notification is generated immediately and no further tracking is performed"; TME_QUERY "The function fills in the structure instead of
 * treating it as a tracking request ... the hover time-out returned is always the actual time-out and not HOVER_DEFAULT". Returns 1 on success; 0 and the last error set by the caller on an invalid window. */
int fle_input_track(uint32_t flags, uint64_t hwnd, uint32_t hover_ms, fle_input_track_t *query)
{
    fle_win_iview v;
    trk *t;
    unsigned owner;

    if (!fle_win_input_view(hwnd, &v)) return 0;
    owner = v.owner;
    t = trk_find(hwnd);
    if (flags & FLE_TME_QUERY) {
        memset(query, 0, sizeof *query);
        query->hwnd = t ? t->hwnd : 0;
        query->flags = t ? t->flags : 0;
        query->hover_ms = t && (t->flags & FLE_TME_HOVER) ? t->hover_ms : 0;
        return 1;
    }
    if (flags & FLE_TME_CANCEL) {
        if (t) {
            t->flags &= ~(flags & (FLE_TME_HOVER | FLE_TME_LEAVE));
            if (flags & FLE_TME_HOVER) t->hover_due = 0;
            if (!(t->flags & (FLE_TME_HOVER | FLE_TME_LEAVE))) trk_drop(t);
        }
        return 1;
    }
    if ((flags & FLE_TME_LEAVE) && !pointer_over(hwnd)) {
        /* the leave notification is generated at once, and "no further tracking is performed" (hover included) */
        (void)fle_thread_post(owner, hwnd, (flags & FLE_TME_NONCLIENT) ? FLE_WM_NCMOUSELEAVE : FLE_WM_MOUSELEAVE, 0, 0);
        if (t) trk_drop(t);
        return 1;
    }
    if (!t) {
        unsigned k;

        for (k = 0; k < TRK_MAX; k++)
            if (!trks[k].used) {
                t = &trks[k];
                break;
            }
        if (!t) fle_stop("TrackMouseEvent", "more than 32 windows are tracked at once (a limit of this environment)");
        memset(t, 0, sizeof *t);
        t->used = 1;
        t->hwnd = hwnd;
    }
    t->flags = (t->flags & ~FLE_TME_NONCLIENT) | (flags & (FLE_TME_HOVER | FLE_TME_LEAVE | FLE_TME_NONCLIENT));
    if ((flags & FLE_TME_HOVER) && pointer_over(hwnd)) {
        int32_t x, y;

        t->hover_ms = hover_ms == FLE_HOVER_DEFAULT ? HOVER_DEFAULT_MS : hover_ms;
        fle_win_pointer_get(&x, &y);
        t->ax = x;
        t->ay = y;
        t->hover_due = clk() + (uint64_t)t->hover_ms * FLE_TICKS_PER_MS;     /* "the hover timer will be reset" */
    } else {
        t->flags &= ~FLE_TME_HOVER;                                         /* "This flag is ignored if the mouse pointer is not over the specified window" */
        t->hover_due = 0;
    }
    if (!(t->flags & (FLE_TME_HOVER | FLE_TME_LEAVE))) trk_drop(t);
    return 1;
}

uint64_t fle_input_next_tick(unsigned thread)
{
    unsigned k;
    uint64_t best = 0;
    fle_win_iview v;

    for (k = 0; k < TRK_MAX; k++) {
        if (!trks[k].used || !trks[k].hover_due) continue;
        if (!fle_win_input_view(trks[k].hwnd, &v) || v.owner != thread) continue;
        if (!best || trks[k].hover_due < best) best = trks[k].hover_due;
    }
    return best;
}

/* ------------------------------------------------------------------------------------------------------------- translation of a raw event into messages */
/* The messages made for one thread, in order, before the retrieval call picks one (a message that PM_NOREMOVE leaves stays here). */
#define OUT_MAX 32u
static struct { fle_tmsg m[OUT_MAX]; unsigned head, n; unsigned thread; } outq;

static void out_push(unsigned thread, uint64_t hwnd, uint32_t msg, uint64_t wp, uint64_t lp, uint64_t ticks, int32_t px, int32_t py)
{
    fle_tmsg *m;

    if (outq.n >= OUT_MAX) return;
    outq.thread = thread;
    m = &outq.m[(outq.head + outq.n) % OUT_MAX];
    memset(m, 0, sizeof *m);
    m->hwnd = hwnd;
    m->msg = msg;
    m->wp = wp;
    m->lp = lp;
    m->time = (uint32_t)(ticks / FLE_TICKS_PER_MS);
    m->ptx = px;
    m->pty = py;
    outq.n++;
}

/* The window that gets the mouse event: the capture window if some window has the capture, else the window at the point. Learn WM_MOUSEMOVE: "If the mouse is not captured, the message is posted to the window that contains
 * the cursor. Otherwise, the message is posted to the window that has captured the mouse." A disabled window (or one with a disabled ancestor) gets no mouse input: the event is dropped (S-disabled-drops: Learn
 * WindowFromPoint "does not retrieve a handle to a hidden or disabled window"; the page of mouse input says nothing about a click on a disabled window, which Windows swallows). */
static uint64_t mouse_target(int32_t x, int32_t y, unsigned *thread, int *captured)
{
    unsigned ct;
    uint64_t c = fle_win_input_capture_any(&ct);
    fle_win_iview v;
    uint64_t h;

    *captured = 0;
    if (c) {
        *captured = 1;
        *thread = ct;
        return c;
    }
    h = fle_win_input_hit(x, y);
    if (!h || !fle_win_input_view(h, &v)) {
        return 0;
    }
    if (!v.enabled) {
        return 0;
    }
    *thread = v.owner;
    return h;
}

static int send_hittest(const char *api, uint64_t h, int32_t x, int32_t y)
{
    return (int16_t)(uint16_t)fle_win_input_send(api, h, FLE_WM_NCHITTEST, 0, lparam_xy(x, y));
}

/* The thread the head event is for. */
static int ev_thread(const hwev *e, unsigned *thread)
{
    int cap;

    if (e->kind == FLE_IN_KDOWN || e->kind == FLE_IN_KUP || e->kind == FLE_IN_WHEEL) return fle_win_input_key_target(thread) != 0;
    if (e->kind == FLE_IN_DISPLAY) {   /* decision 0212: a system event, for the GUI thread: every window of this guest is owned by thread 0 (S-display-thread, a choice) */
        *thread = 0;
        return 1;
    }
    if (e->kind == FLE_IN_SYSCMD) {    /* decision 0267: for the thread that owns the window; a window that is gone has no target (the event is dropped) */
        fle_win_iview v;

        if (!fle_win_input_view(e->hwnd, &v)) return 0;
        *thread = v.owner;
        return 1;
    }
    return mouse_target(e->x, e->y, thread, &cap) != 0;
}

int fle_input_settled(void)
{
    if (ev_n || outq.n) return 0;
    return last_consumer == (unsigned)-1 || fle_thread_in_message_wait(last_consumer);
}

int fle_input_has_event(unsigned thread)
{
    unsigned t;
    uint64_t now = clk();
    fle_win_iview v;
    unsigned k;

    if (outq.n && outq.thread == thread) return 1;
    for (k = 0; k < TRK_MAX; k++)
        if (trks[k].used && trks[k].hover_due && trks[k].hover_due <= now && fle_win_input_view(trks[k].hwnd, &v) && v.owner == thread) return 1;
    if (!ev_n) return 0;
    if (!ev_thread(&evq[ev_head], &t)) {
        /* the event goes to no window: it is consumed by the next retrieval of thread 0 (S-no-target: dropped there) */
        return thread == 0;
    }
    return t == thread;
}

static const uint32_t btn_down[3] = { 0x0201u, 0x0204u, 0x0207u };      /* WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MBUTTONDOWN (winuser.h: not on the Learn pages read) */
static uint64_t last_click_ticks, last_click_win;
static int32_t last_click_x, last_click_y;
static int last_click_btn = -1;
/* S-doubleclick-time: GetDoubleClickTime has no default on its page ("the current double-click time", at most 5000 ms): 500 ms is the value used. The double-click rectangle is 4 x 4 (SM_CXDOUBLECLK). */
#define DOUBLE_CLICK_MS 500u

/* Learn WM_PARENTNOTIFY (inputmsg/wm-parentnotify): sent to the ancestors of a child window while the child is WS_CHILD and has no WS_EX_NOPARENTNOTIFY ("All child windows, except those that have the WS_EX_NOPARENTNOTIFY
 * extended window style, send this message to their parent windows"; "This message is also sent to all ancestor windows of the child window, including the top-level window"). The same loop as Wine 10.0's
 * send_parent_notify (win32u/message.c). The point is given in screen coordinates and goes to each ancestor in its own client coordinates. */
static void send_parent_notify(uint64_t hwnd, uint32_t event, int32_t sx, int32_t sy)
{
    fle_win_iview v, pv;
    unsigned guard;

    for (guard = 0; guard < 64; guard++) {
        if (!fle_win_input_view(hwnd, &v)) break;
        if (!(v.style & 0x40000000u)) break;                                  /* WS_CHILD */
        if (v.exstyle & 0x00000004u) break;                                   /* WS_EX_NOPARENTNOTIFY */
        if (!v.parent || !fle_win_input_view(v.parent, &pv)) break;
        hwnd = v.parent;
        (void)fle_win_input_send("input", hwnd, 0x0210u, event & 0xFFFFu, lparam_xy(sx - pv.ox, sy - pv.oy));
    }
}

/* the pointer left every window: the leave notification of the window it was in */
static void pointer_over_nothing(const hwev *e)
{
    if (e->kind == FLE_IN_MOVE && cursor_win) {
        trk *tr = trk_find(cursor_win);
        fle_win_iview ov;

        if (tr && (tr->flags & FLE_TME_LEAVE) && fle_win_input_view(cursor_win, &ov)) {
            out_push(ov.owner, cursor_win, (tr->flags & FLE_TME_NONCLIENT) ? FLE_WM_NCMOUSELEAVE : FLE_WM_MOUSELEAVE, 0, 0, e->ticks, e->x, e->y);
            trk_drop(tr);
        }
        cursor_win = 0;
    }
}

/* Decision 0323 (replaces S-httransparent of decision 0136, which dropped the event). Learn WM_NCHITTEST, HTTRANSPARENT: "In a window currently covered by another window in the same thread (the message will be sent to
 * underlying windows in the same thread until one of them returns a code that is not HTTRANSPARENT)". The windows that contain the point are walked in the order of Wine 10.0's window_from_point (win32u/window.c,
 * UNOFFICIAL: the list of fle_win_input_hit_list, a child before its parent, a sibling below after the whole of the one above): each one that belongs to the thread of the first one is sent WM_NCHITTEST; the first
 * answer that is not HTTRANSPARENT decides the window and the hit-test code; a window of another thread is not asked (Wine: HTCLIENT, "the hit test is only sent to windows of the current thread"). A disabled
 * window ends the walk with the event dropped, as it does for the first one (S-disabled-drops). When every window answers HTTRANSPARENT there is no window (Wine: the result is NULL). `first` is the window
 * mouse_target found, `thread` its thread. Returns the window (0 = none) and its hit-test code in *ht. */
#define HIT_LIST_MAX 64u
static uint64_t hit_walk(int32_t x, int32_t y, uint64_t first, unsigned thread, int *ht)
{
    uint64_t list[HIT_LIST_MAX];
    unsigned n, k;
    int r = send_hittest("input", first, x, y);

    *ht = r;
    if (r != FLE_HTTRANSPARENT) return first;      /* the common case: the first window decides (no list is made) */
    *ht = FLE_HTNOWHERE;
    n = fle_win_input_hit_list(x, y, list, HIT_LIST_MAX);
    for (k = 0; k < n && list[k] != first; k++) {}
    for (k++; k < n; k++) {                         /* the windows after `first` in the list */
        fle_win_iview cv;

        if (!fle_win_input_view(list[k], &cv)) continue;          /* a window that went away while an earlier one handled WM_NCHITTEST */
        if (!cv.enabled) return 0;
        if (cv.owner != thread) {
            *ht = FLE_HTCLIENT;
            return list[k];
        }
        r = send_hittest("input", list[k], x, y);
        if (r != FLE_HTTRANSPARENT) {
            *ht = r;
            return list[k];
        }
    }
    return 0;
}

static void translate_mouse(const hwev *e)
{
    unsigned tt = 0;
    int cap = 0;
    uint64_t w = mouse_target(e->x, e->y, &tt, &cap);
    fle_win_iview v;
    int ht;
    uint64_t t = e->ticks;
    uint32_t mk = e->mk;

    if (!w || !fle_win_input_view(w, &v)) {
        pointer_over_nothing(e);
        return;
    }
    if (cap) {
        ht = send_hittest("input", w, e->x, e->y);      /* S-capture-client: the answer decides nothing while a window has the capture (below) */
    } else {
        w = hit_walk(e->x, e->y, w, tt, &ht);
        if (!w || !fle_win_input_view(w, &v)) {
            pointer_over_nothing(e);
            return;
        }
    }
    /* S-capture-client: while a window has the capture it gets the client messages (WM_MOUSEMOVE, WM_LBUTTONDOWN ... with coordinates relative to its client area, negative or beyond its size when the pointer is outside)
     * whatever WM_NCHITTEST answered (Learn WM_MOUSEMOVE: "the message is posted to the window that has captured the mouse"; the page says nothing of a non-client message to it). */
    if (cap) ht = FLE_HTCLIENT;
    if (e->kind == FLE_IN_MOVE) {
        trk *tr;

        if (cursor_win != w) {
            trk *old = cursor_win ? trk_find(cursor_win) : NULL;
            fle_win_iview ov;

            if (old && (old->flags & FLE_TME_LEAVE) && fle_win_input_view(cursor_win, &ov)) {
                out_push(ov.owner, cursor_win, (old->flags & FLE_TME_NONCLIENT) ? FLE_WM_NCMOUSELEAVE : FLE_WM_MOUSELEAVE, 0, 0, e->ticks, e->x, e->y);
                trk_drop(old);
            } else if (old) {
                old->hover_due = 0;
            }
            cursor_win = w;
        }
        tr = trk_find(w);
        if (tr && (tr->flags & FLE_TME_HOVER)) {
            /* Learn TrackMouseEvent Remarks: the pointer "is considered to be hovering when it stays within a specified rectangle for a specified period of time": a move out of the rectangle restarts the time */
            if (e->x < tr->ax - HOVER_RECT_W / 2 || e->x > tr->ax + HOVER_RECT_W / 2 || e->y < tr->ay - HOVER_RECT_H / 2 || e->y > tr->ay + HOVER_RECT_H / 2) {
                tr->ax = e->x;
                tr->ay = e->y;
                tr->hover_due = t + (uint64_t)tr->hover_ms * FLE_TICKS_PER_MS;
            }
        }
        if (!cap) (void)fle_win_input_send("input", w, FLE_WM_SETCURSOR, w, ((uint64_t)FLE_WM_MOUSEMOVE << 16) | (uint16_t)ht);
        if (ht == FLE_HTCLIENT) out_push(v.owner, w, FLE_WM_MOUSEMOVE, mk, lparam_xy(e->x - v.ox, e->y - v.oy), t, e->x, e->y);
        else out_push(v.owner, w, FLE_WM_NCMOUSEMOVE, (uint64_t)(int64_t)ht, lparam_xy(e->x, e->y), t, e->x, e->y);
        return;
    }
    if (e->kind == FLE_IN_BDOWN || e->kind == FLE_IN_BUP) {
        unsigned b = e->data == (int32_t)FLE_IN_BTN_R ? 1u : e->data == (int32_t)FLE_IN_BTN_M ? 2u : 0u;
        uint32_t msg = btn_down[b] + (e->kind == FLE_IN_BUP ? 1u : 0u);
        uint32_t vkb = b == 1u ? VK_RBUTTON_ : b == 2u ? VK_MBUTTON_ : VK_LBUTTON_;

        if (e->kind == FLE_IN_BDOWN) {
            if (!cap) {
                /* Learn WM_PARENTNOTIFY: sent to the parent of a child window, and "to all ancestor windows of the child window, including the top-level window", when the user clicks a mouse button over the child
                 * (unless the child has WS_EX_NOPARENTNOTIFY); the notification is the button-down message even for a double click or a non-client click (Wine 10.0 and the page agree: wParam LOWORD the event,
                 * lParam the point in the client coordinates of the ancestor being notified). It comes before WM_MOUSEACTIVATE (Wine 10.0, win_act.c scenario 4). */
                send_parent_notify(w, btn_down[b], e->x, e->y);
                /* Learn WM_MOUSEACTIVATE: "Sent when the cursor is in an inactive window and the user presses a mouse button": wParam the top-level parent, lParam the hit-test value and the mouse message. A child window
                 * is never the active window, so a click on a child of the active top-level window sends it too (the page: "The parent window receives this message only if the child window passes it to the
                 * DefWindowProc function"; Wine 10.0 sends it whenever the clicked window is not the active window and its top-level window is not a WS_CHILD). */
                uint64_t act = v.root ? v.root : w;
                fle_win_iview rv;

                if (fle_win_input_active() != w && fle_win_input_view(act, &rv) && (rv.style & (0x80000000u | 0x40000000u)) != 0x40000000u) {
                    uint64_t r = fle_win_input_send("input", w, FLE_WM_MOUSEACTIVATE, act, ((uint64_t)msg << 16) | (uint16_t)ht);

                    if (r == 0) r = FLE_MA_ACTIVATE;                                                                         /* Wine 10.0: an answer of 0 is taken as MA_ACTIVATE (not on the Learn page: UNOFFICIAL) */
                    if (r == FLE_MA_ACTIVATE || r == FLE_MA_ACTIVATEANDEAT) fle_win_input_activate("input", w, 2u);          /* WA_CLICKACTIVE */
                    if (r == FLE_MA_ACTIVATEANDEAT || r == FLE_MA_NOACTIVATEANDEAT) return;                                  /* "discards the mouse message" */
                }
                (void)fle_win_input_send("input", w, FLE_WM_SETCURSOR, w, ((uint64_t)msg << 16) | (uint16_t)ht);
            }
            /* double click: the class has CS_DBLCLKS (0x8), the same button and window, within the double-click time and rectangle */
            if (ht == FLE_HTCLIENT && (v.class_style & 0x8u) && last_click_btn == (int)b && last_click_win == w && t - last_click_ticks <= (uint64_t)DOUBLE_CLICK_MS * FLE_TICKS_PER_MS &&
                e->x >= last_click_x - 2 && e->x <= last_click_x + 2 && e->y >= last_click_y - 2 && e->y <= last_click_y + 2) {
                msg += 2u;
                last_click_btn = -1;
            } else {
                last_click_btn = (int)b;
                last_click_win = w;
                last_click_ticks = t;
                last_click_x = e->x;
                last_click_y = e->y;
            }
        }
        set_key(key_sync, vkb, e->kind == FLE_IN_BDOWN);
        if (ht == FLE_HTCLIENT) out_push(v.owner, w, msg, mk, lparam_xy(e->x - v.ox, e->y - v.oy), t, e->x, e->y);
        else out_push(v.owner, w, msg - 0x0200u + 0x00A0u, (uint64_t)(int64_t)ht, lparam_xy(e->x, e->y), t, e->x, e->y);
        return;
    }
}

static void translate_wheel(const hwev *e)
{
    unsigned tt = 0;
    uint64_t w = fle_win_input_key_target(&tt);
    fle_win_iview v;

    /* Learn WM_MOUSEWHEEL: "Sent to the focus window when the mouse wheel is rotated"; wParam: the high word the distance (a multiple of WHEEL_DELTA 120), the low word the key flags; lParam the screen point */
    if (!w || !fle_win_input_view(w, &v)) return;
    out_push(v.owner, w, FLE_WM_MOUSEWHEEL, ((uint64_t)(uint16_t)(int16_t)e->data << 16) | e->mk, lparam_xy(e->x, e->y), e->ticks, e->x, e->y);
}

static void translate_key(const hwev *e)
{
    unsigned tt = 0;
    uint64_t w = fle_win_input_key_target(&tt);
    fle_win_iview v;
    uint32_t vk = e->vk, msg, sc = e->scan, lp;
    int down = e->kind == FLE_IN_KDOWN, alt, prev;

    if (!w || !fle_win_input_view(w, &v)) return;
    if (sc == 0) sc = scan_of_vk[e->vk & 0xFFu];
    prev = (key_sync[vk] & 0x80) != 0;
    alt = (key_sync[VK_MENU_] & 0x80) != 0;
    /* Learn WM_KEYDOWN: wParam the virtual-key code; lParam bits 0-15 repeat count, 16-23 scan code, 24 extended-key flag, 29 context code "always 0 for a WM_KEYDOWN", 30 previous key state, 31 transition state "always 0
     * for a WM_KEYDOWN". The system keys (WM_SYSKEYDOWN, Learn: "a key that is pressed when the ALT key is down" and F10) have bit 29 = 1 while Alt is down. */
    if (vk == VK_LSHIFT_ || vk == VK_RSHIFT_) vk = VK_SHIFT_;
    else if (vk == VK_LCONTROL_ || vk == VK_RCONTROL_) vk = VK_CONTROL_;
    else if (vk == VK_LMENU_ || vk == VK_RMENU_) vk = VK_MENU_;
    msg = down ? FLE_WM_KEYDOWN : FLE_WM_KEYUP;
    if ((alt && !(key_sync[VK_CONTROL_] & 0x80)) || e->vk == VK_F10_ || e->vk == VK_LMENU_ || e->vk == VK_RMENU_ || e->vk == VK_MENU_) msg = down ? FLE_WM_SYSKEYDOWN : FLE_WM_SYSKEYUP;
    lp = 1u | ((sc & 0xFFu) << 16) | (e->ext ? 1u << 24 : 0u);
    if (alt && (msg == FLE_WM_SYSKEYDOWN || msg == FLE_WM_SYSKEYUP) && e->vk != VK_F10_) lp |= 1u << 29;
    if (prev) lp |= 1u << 30;
    if (!down) lp |= (1u << 30) | (1u << 31);
    set_key(key_sync, e->vk, down);
    out_push(v.owner, w, msg, vk, lp, e->ticks, e->x, e->y);
}

static void translate_one(const hwev *e)
{
    switch (e->kind) {
    case FLE_IN_MOVE: case FLE_IN_BDOWN: case FLE_IN_BUP: translate_mouse(e); break;
    case FLE_IN_WHEEL: translate_wheel(e); break;
    case FLE_IN_KDOWN: case FLE_IN_KUP: translate_key(e); break;
    default: break;
    }
}

/* The messages of the hover timers that are due (Learn WM_MOUSEHOVER: "Posted to a window when the cursor hovers over the client area of the window for the period of time specified in a prior call to
 * TrackMouseEvent"; "Hover tracking stops when WM_MOUSEHOVER is generated"). */
static void hover_due(unsigned t)
{
    unsigned k;
    uint64_t now = clk();
    fle_win_iview v;

    for (k = 0; k < TRK_MAX; k++) {
        trk *tr = &trks[k];

        if (!tr->used || !tr->hover_due || tr->hover_due > now) continue;
        if (!fle_win_input_view(tr->hwnd, &v) || v.owner != t) continue;
        if (pointer_over(tr->hwnd)) {
            int32_t x, y;

            fle_win_pointer_get(&x, &y);
            out_push(t, tr->hwnd, FLE_WM_MOUSEHOVER, mk_flags(), lparam_xy(x - v.ox, y - v.oy), now, x, y);
        }
        tr->flags &= ~FLE_TME_HOVER;
        tr->hover_due = 0;
        if (!(tr->flags & FLE_TME_LEAVE)) trk_drop(tr);
    }
}

static unsigned thr_msg_last_time[256];
static int32_t thr_msg_last_x[256], thr_msg_last_y[256];

void fle_input_note_message(unsigned thread, uint32_t time, int32_t ptx, int32_t pty)
{
    if (thread >= 256u) return;
    thr_msg_last_time[thread] = time;
    thr_msg_last_x[thread] = ptx;
    thr_msg_last_y[thread] = pty;
}

/* Decision 0212: the display changed. Windows (Learn WM_DISPLAYCHANGE: "Sent to all windows when the display resolution has changed"; wParam "The new image depth of the display, in bits per pixel", lParam "The low-order word
 * specifies the horizontal resolution of the screen. The high-order word specifies the vertical resolution"; WM_SETTINGCHANGE: sent to all top-level windows for a changed system-wide setting, wParam the SPI_* value, here
 * SPI_SETWORKAREA because the work area is the monitor (fle_monitor.h)). The screen takes the new size first (what the guest reads when it handles the message is already the new size). Both messages are SENT to the top-level windows
 * in z-order, top first (a choice: the page gives no order, S-display-order); the 16-bit words of lParam hold the sizes of fle_screen.h (at most 7680 and 4320). The pointer is brought inside the new screen. A size equal to the
 * current one sends nothing. */
static void apply_display(const hwev *e)
{
    uint64_t hs[64];
    unsigned n, k;
    int32_t px, py;
    uint32_t w = (uint32_t)e->x, h = (uint32_t)e->y, ow = fle_screen_w(), oh = fle_screen_h();

    if (w == ow && h == oh) return;
    if (!fle_screen_set(w, h)) return;
    fle_win_pointer_get(&px, &py);
    fle_win_pointer_set(px, py);
    n = fle_win_input_toplevels(hs, 64);
    for (k = 0; k < n; k++) (void)fle_win_input_send("display-change", hs[k], FLE_WM_DISPLAYCHANGE, FLE_GDC_BITSPIXEL, ((uint64_t)h << 16) | w);
    for (k = 0; k < n; k++) (void)fle_win_input_send("display-change", hs[k], FLE_WM_SETTINGCHANGE, FLE_SPI_SETWORKAREA, 0);
    (void)fle_win_input_remaximize((int32_t)w - (int32_t)ow, (int32_t)h - (int32_t)oh);
}

/* decision 0218 (ship3). The queue above holds the messages that were made from the host's input events, each with the window it was made for AT THE TIME IT WAS MADE (the translation runs ahead of the retrieval: a key
 * down and its key up are made one after the other). Windows decides a hardware message's window when the message is retrieved: Learn About Keyboard Input: "the system posts keyboard messages to the message queue
 * of the foreground thread that created the window with the keyboard focus"; Learn WM_KEYUP / WM_KEYDOWN: "to the window with the keyboard focus", "If no window has the keyboard focus, the active window receives". The real
 * case: Enter closes a dialog (the VCL destroys it while it handles the key down); the key up of the same key was made for the dialog and reached the guest as a message for a window that is gone (the guest asked
 * IsWindowUnicode of it: FALSE: SendMessageA, which has no row: "unsupported instruction at 0x4253a0", W32 of decision 0191). Here a message whose window is no longer a live window of its thread gets the window
 * that has the focus now when it is a keyboard message, and is dropped when it is not (Wine 10.0 server queue_cleanup_window removes the messages of a destroyed window; UNOFFICIAL). Nothing else changes: a message
 * for a live window is returned as it was. Returns 0 when the message is to be dropped. */
static int outq_target_alive(fle_tmsg *m, unsigned t)
{
    fle_win_iview v;
    unsigned tt = 0;
    uint64_t h;

    if (fle_win_input_view(m->hwnd, &v)) return 1;
    if (m->msg >= 0x0100u && m->msg <= 0x0109u) {                                        /* WM_KEYDOWN .. WM_SYSDEADCHAR */
        h = fle_win_input_key_target(&tt);
        if (h != 0 && tt == t) {
            m->hwnd = h;
            return 1;
        }
    }
    return 0;
}

/* Decision 0323: the WH_MOUSE and WH_KEYBOARD hooks (fle_win.c fle_win_input_call_hooks has the pages). Called where GetMessage / PeekMessage of thread t is about to return the hardware message *m: a mouse message
 * (WM_MOUSEMOVE .. WM_MOUSEHWHEEL, and the non-client WM_NCMOUSEMOVE .. WM_NCXBUTTONDBLCLK) calls the WH_MOUSE chain with wParam = the message, lParam = a MOUSEHOOKSTRUCT(Ex) (pt = the screen position, hwnd, wHitTestCode =
 * the hit-test code [HTCLIENT for a client message, wParam for a non-client one], dwExtraInfo = 0, mouseData = the message's wParam: Wine 10.0 gives the Ex form to every hook, UNOFFICIAL); a key message (WM_KEYDOWN, WM_KEYUP,
 * WM_SYSKEYDOWN, WM_SYSKEYUP) calls the WH_KEYBOARD chain with wParam = the virtual-key code and lParam = the message's lParam. nCode is HC_ACTION (0) when the message is being removed and HC_NOREMOVE (3) for a peek.
 * A nonzero answer discards the message (Learn: "to prevent the system from passing the message to the target window procedure"). Returns 1 then. WM_CHAR and the like are made by TranslateMessage from a posted
 * message and do not pass through a hook (Learn KeyboardProc: WM_KEYUP / WM_KEYDOWN). A hook procedure that retrieves messages itself does not run the hooks again (the guard). */
#define FLE_WH_KEYBOARD_ 2
#define FLE_WH_MOUSE_ 7
#define FLE_HC_ACTION_ 0
#define FLE_HC_NOREMOVE_ 3
static int in_input_hook;
static int input_hook_swallows(unsigned t, const fle_tmsg *m, int remove)
{
    uint64_t res = 0;
    int ncode = remove ? FLE_HC_ACTION_ : FLE_HC_NOREMOVE_, called;

    if (in_input_hook) return 0;
    if ((m->msg >= 0x0200u && m->msg <= 0x020Eu) || (m->msg >= 0x00A0u && m->msg <= 0x00ADu)) {
        uint8_t hs[40];
        uint32_t ht = m->msg >= 0x00A0u && m->msg <= 0x00ADu ? (uint32_t)m->wp : (uint32_t)FLE_HTCLIENT;
        uint32_t md = (uint32_t)m->wp;

        memset(hs, 0, sizeof hs);
        memcpy(hs + 0, &m->ptx, 4);
        memcpy(hs + 4, &m->pty, 4);
        memcpy(hs + 8, &m->hwnd, 8);
        memcpy(hs + 16, &ht, 4);
        memcpy(hs + 32, &md, 4);
        in_input_hook = 1;
        called = fle_win_input_call_hooks(FLE_WH_MOUSE_, t, ncode, m->msg, 0, hs, sizeof hs, &res);
        in_input_hook = 0;
        return called && res != 0;
    }
    if (m->msg >= 0x0100u && m->msg <= 0x0105u && m->msg != 0x0102u && m->msg != 0x0103u) {   /* WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP (not WM_CHAR, WM_DEADCHAR) */
        in_input_hook = 1;
        called = fle_win_input_call_hooks(FLE_WH_KEYBOARD_, t, ncode, m->wp & 0xFFFFu, m->lp, NULL, 0, &res);
        in_input_hook = 0;
        return called && res != 0;
    }
    return 0;
}

/* Called by GetMessageW and PeekMessageW of thread t after the posted queue found nothing. */
int fle_input_synth(unsigned t, uint64_t fh, uint32_t fmin, uint32_t fmax, int remove, uint32_t qs, fle_tmsg *out)
{
    unsigned guard = 0, k;

    if (!(qs & FLE_QS_INPUT_BITS)) return 0;
    hover_due(t);
    for (;;) {
        if (outq.n == 0) {
            unsigned et;

            if (!ev_n || guard++ > EV_MAX) return 0;
            if (!ev_thread(&evq[ev_head], &et)) {
                ev_head = (ev_head + 1u) % EV_MAX;       /* S-no-target: an event for no window is dropped */
                ev_n--;
                continue;
            }
            if (et != t) return 0;
            {
                hwev e = evq[ev_head];

                ev_head = (ev_head + 1u) % EV_MAX;
                ev_n--;
                last_consumer = t;
                /* S-mousemove-coalesce: a move that is directly followed by another move is dropped (Windows combines consecutive WM_MOUSEMOVE messages; the Learn pages read do not say how) */
                if (e.kind == FLE_IN_MOVE && ev_n && evq[ev_head].kind == FLE_IN_MOVE) continue;
                if (e.kind == FLE_IN_DISPLAY) { apply_display(&e); continue; }
                if (e.kind == FLE_IN_SYSCMD) { (void)fle_win_input_send("task-bar", e.hwnd, FLE_WM_SYSCOMMAND, (uint64_t)(uint32_t)e.data, 0); continue; }
                translate_one(&e);
            }
            continue;
        }
        if (outq.thread != t) return 0;
        if (fmin == 0 && fmax == 0) fmax = 0xFFFFFFFFu;
        for (k = 0; k < outq.n; k++) {
            fle_tmsg *m = &outq.m[(outq.head + k) % OUT_MAX];

            if (!outq_target_alive(m, t)) {
                unsigned j;

                for (j = k; j + 1 < outq.n; j++) outq.m[(outq.head + j) % OUT_MAX] = outq.m[(outq.head + j + 1) % OUT_MAX];
                outq.n--;
                k--;
                continue;
            }
            if (m->msg < fmin || m->msg > fmax) continue;
            if (fh != 0 && fh != UINT64_MAX && m->hwnd != fh) continue;
            *out = *m;
            last_consumer = t;
            if (remove) {
                unsigned j;

                for (j = k; j + 1 < outq.n; j++) outq.m[(outq.head + j) % OUT_MAX] = outq.m[(outq.head + j + 1) % OUT_MAX];
                outq.n--;
            }
            /* decision 0323: the hook runs after the message has left the queue when it is removed (Wine: "if remove is TRUE, remove message first before calling hooks to avoid recursive hook calls") */
            if (input_hook_swallows(t, out, remove)) {
                if (!remove) {
                    unsigned j;

                    for (j = k; j + 1 < outq.n; j++) outq.m[(outq.head + j) % OUT_MAX] = outq.m[(outq.head + j + 1) % OUT_MAX];
                    outq.n--;
                }
                k--;
                continue;
            }
            return 1;
        }
        return 0;
    }
}

/* ------------------------------------------------------------------------------------------------------------- DefWindowProcW, TranslateMessage */
static int32_t sx16(uint64_t v) { return (int32_t)(int16_t)(uint16_t)v; }

int fle_input_defproc(const char *api, uint64_t hwnd, uint32_t msg, uint64_t wp, uint64_t lp, uint64_t *result)
{
    fle_win_iview v;

    if (!fle_win_input_view(hwnd, &v)) return 0;
    switch (msg) {
    case 0x004Au:   /* WM_COPYDATA (decision 0144) */
        /* Learn WM_COPYDATA: "If the receiving application processes this message, it should return TRUE; otherwise, it should return FALSE." The default window procedure processes nothing: FALSE. The VCL's
         * TWinControl.DefaultHandler (what the engine's own handler f_11d1c80 ends with) overwrites the handler's result with this value, as on Windows. S-g8a-copydata-default. */
        *result = 0;
        return 1;
    case FLE_WM_NCHITTEST: {
        /* Learn WM_NCHITTEST: "The return value of the DefWindowProc function is one of the following values, indicating the position of the cursor hot spot". The model's windows have no frame (decision 0123, S-nc-zero):
         * the point is in the client area (HTCLIENT) when it lies inside the window's rectangle, else HTNOWHERE. S-nchittest-client. */
        int32_t x = sx16(lp), y = sx16(lp >> 16);

        *result = (x >= v.ox && y >= v.oy && x < v.ox + v.cx && y < v.oy + v.cy) ? (uint64_t)FLE_HTCLIENT : (uint64_t)FLE_HTNOWHERE;
        return 1;
    }
    case FLE_WM_SETCURSOR: {
        /* Learn WM_SETCURSOR: "The DefWindowProc function passes the WM_SETCURSOR message to a parent window before processing. If the parent window returns TRUE, further processing is halted ... The DefWindowProc function
         * also uses this message to set the cursor to an arrow if it is not in the client area, or to the registered class cursor if it is in the client area." The page gives no result for the default processing:
         * S-setcursor-result: TRUE when the cursor was set, FALSE when not. */
        uint32_t ht = (uint32_t)(lp & 0xFFFFu);
        uint64_t arrow = FLE_CURSOR_HANDLE_BASE + 4u, cur;

        if (v.parent && fle_win_input_send(api, v.parent, FLE_WM_SETCURSOR, wp, lp)) {
            *result = 1;
            return 1;
        }
        cur = ht == (uint32_t)FLE_HTCLIENT ? v.hcursor : arrow;
        if (cur && fle_imp_SetCursor) {
            uint64_t sv[2] = { R[0], R[1] };

            R[1] = cur;
            fle_imp_SetCursor();
            R[0] = sv[0];
            R[1] = sv[1];
            *result = 1;
        } else {
            *result = 0;
        }
        return 1;
    }
    case FLE_WM_MOUSEACTIVATE: {
        /* Learn WM_MOUSEACTIVATE: "The DefWindowProc function passes the message to a child window's parent window before any processing occurs. The parent window determines whether to activate the child window."
         * S-mouseactivate-default: a non-zero answer of the parent is the answer; else MA_ACTIVATE (the page lists the four values; the default one is not named), MA_NOACTIVATE for WS_EX_NOACTIVATE. */
        uint64_t r = v.parent ? fle_win_input_send(api, v.parent, FLE_WM_MOUSEACTIVATE, wp, lp) : 0;

        *result = r ? r : (v.exstyle & 0x08000000u) ? FLE_MA_NOACTIVATE : FLE_MA_ACTIVATE;
        return 1;
    }
    case FLE_WM_MOUSEWHEEL:
        /* Learn WM_MOUSEWHEEL: "The DefWindowProc function propagates the message to the window's parent." */
        *result = v.parent ? fle_win_input_send(api, v.parent, msg, wp, lp) : 0;
        return 1;
    case 0x0087u:
        /* Learn WM_GETDLGCODE: "Sent to the window procedure associated with a control ... the control can respond to the WM_GETDLGCODE message to indicate the types of input it wants to process itself"; the return
         * value is a set of DLGC_* flags. The page gives no result of the DEFAULT processing: S-getdlgcode-zero: 0 (no DLGC flag: the window asks for no input of its own beyond the default handling), the
         * answer of a window that is not a control. Reached by the keyboard path (the VCL asks for the code of the focus window before it handles a key message; wall of decision 0136). */
        *result = 0;
        return 1;
    case FLE_WM_SYSCOMMAND:
        /* S-syscommand: SC_CLOSE (0xF060) becomes WM_CLOSE ("Closes the window", Learn WM_SYSCOMMAND). Decision 0267 (alpha10): SC_MINIMIZE (0xF020 "Minimizes the window"), SC_MAXIMIZE (0xF030 "Maximizes the window") and SC_RESTORE
         * (0xF120 "Restores the window to its normal position and size") are what DefWindowProc turns into ShowWindow(SW_MINIMIZE / SW_MAXIMIZE / SW_RESTORE) (Wine 10.0, UNOFFICIAL: the same message sequence as the ShowWindow call, with no check of
         * WS_MINIMIZEBOX / WS_MAXIMIZEBOX: harness/alpha10/syscmd_seq.c); the other commands (SC_SIZE, SC_MOVE ... a model without a frame) have no effect. The command is the wParam without its low four bits ("the four low-order bits are used internally"). */
        switch (wp & 0xFFF0u) {
        case 0xF060u: (void)fle_win_input_send(api, hwnd, 0x0010u, 0, 0); break;
        case 0xF020u: fle_win_input_showwindow(api, hwnd, 6 /* SW_MINIMIZE */); break;
        case 0xF030u: fle_win_input_showwindow(api, hwnd, 3 /* SW_MAXIMIZE */); break;
        case 0xF120u: fle_win_input_showwindow(api, hwnd, 9 /* SW_RESTORE */); break;
        default: break;
        }
        *result = 0;
        return 1;
    case FLE_WM_SYSKEYDOWN:
        /* Alt+F4: the default processing of WM_SYSKEYDOWN makes the system command (Learn WM_SYSKEYDOWN: "To enable the system menu ... DefWindowProc"). S-altf4. */
        if (wp == VK_F4_ && (lp & (1u << 29))) (void)fle_win_input_send(api, hwnd, FLE_WM_SYSCOMMAND, 0xF060u, 0);
        *result = 0;
        return 1;
    default:
        break;
    }
    /* messages whose default processing has no effect in this model: the mouse and key messages of a window (Learn: "If an application processes this message, it should return zero"; the pages give no result of
     * the default processing, S-mouse-default-zero), WM_MOUSEHOVER / WM_MOUSELEAVE / WM_NCMOUSELEAVE, WM_CONTEXTMENU */
    if ((msg >= 0x0200u && msg <= 0x0209u) || (msg >= 0x0100u && msg <= 0x0109u && msg != FLE_WM_SYSKEYDOWN) || msg == FLE_WM_MOUSEHOVER || msg == FLE_WM_MOUSELEAVE || msg == FLE_WM_NCMOUSELEAVE ||
        (msg >= 0x00A0u && msg <= 0x00A9u) || msg == 0x007Bu || msg == 0x0211u || msg == 0x0212u || msg == 0x0216u || msg == 0x0128u) {
        *result = 0;
        return 1;
    }
    return 0;
}

/* The US keyboard layout (S-us-layout): a virtual key to the character it makes. Learn WM_CHAR: "Posted to the window with the keyboard focus when a WM_KEYDOWN message is translated by the TranslateMessage
 * function"; the page does not give a layout, and no keyboard layout data is on the pages read: the US layout (the input locale 0x0409 of decision 0063) is used. */
static uint32_t vk_to_char(uint32_t vk, int shift, int ctrl, int caps)
{
    uint32_t c = 0;

    if (vk >= 'A' && vk <= 'Z') {
        c = vk + ((shift ^ caps) ? 0u : 0x20u);
        if (ctrl) c = vk - 'A' + 1u;
        return c;
    }
    if (vk >= '0' && vk <= '9') {
        static const char sh[] = ")!@#$%^&*(";

        return shift ? (uint32_t)(unsigned char)sh[vk - '0'] : vk;
    }
    switch (vk) {
    case 0x20: return ' ';
    case 0x0D: return '\r';
    case 0x09: return '\t';
    case 0x08: return '\b';
    case 0x1B: return 0x1Bu;
    case 0xBA: return shift ? ':' : ';';
    case 0xBB: return shift ? '+' : '=';
    case 0xBC: return shift ? '<' : ',';
    case 0xBD: return shift ? '_' : '-';
    case 0xBE: return shift ? '>' : '.';
    case 0xBF: return shift ? '?' : '/';
    case 0xC0: return shift ? '~' : '`';
    case 0xDB: return shift ? '{' : '[';
    case 0xDC: return shift ? '|' : '\\';
    case 0xDD: return shift ? '}' : ']';
    case 0xDE: return shift ? '"' : '\'';
    default: return 0;
    }
}

/* user32!TranslateMessage: BOOL f(const MSG *rcx). Learn: key messages (WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP) are translated into character messages "posted to the calling thread's message queue", nonzero when a
 * character message is posted, zero otherwise; WM_KEYUP and WM_SYSKEYUP are not translated into a character message that is posted (only the key-down ones); the message is not modified. */
int fle_input_translate(const char *api, uint64_t pmsg)
{
    uint64_t h = fle_ld(pmsg + FLE_MSG_OFF_HWND, 8), wp = fle_ld(pmsg + FLE_MSG_OFF_WPARAM, 8), lp = fle_ld(pmsg + FLE_MSG_OFF_LPARAM, 8);
    uint32_t msg = (uint32_t)fle_ld(pmsg + FLE_MSG_OFF_MESSAGE, 4), c;
    int shift = (key_sync[VK_SHIFT_] & 0x80) != 0, ctrl = (key_sync[VK_CONTROL_] & 0x80) != 0, caps = (key_sync[VK_CAPITAL_] & 1) != 0;
    fle_win_iview v;

    (void)api;
    if (msg != FLE_WM_KEYDOWN && msg != FLE_WM_SYSKEYDOWN) return 0;
    if (!h || !fle_win_input_view(h, &v)) return 0;
    c = vk_to_char((uint32_t)wp, shift, ctrl, caps);
    if (c == 0) return 0;
    (void)fle_thread_post(v.owner, h, msg == FLE_WM_KEYDOWN ? FLE_WM_CHAR : FLE_WM_SYSCHAR, c, lp & 0x7FFFFFFFu);
    return 1;
}

/* ------------------------------------------------------------------------------------------------------------- the user32 rows */
/* user32!GetMessageTime: LONG f(void). Learn: "the message time for the last message retrieved by the GetMessage function ... the elapsed time, in milliseconds, from the time the system was started to the time the
 * message was created (that is, placed in the thread's message queue)". The time of the thread's last retrieved message (0 before any); PeekMessageW is counted like GetMessageW (S-lastmsg-peek: the page names GetMessage). */
void fle_imp_GetMessageTime(void)
{
    unsigned t = fle_thread_current ? fle_thread_current() : 0u;

    R[0] = t < 256u ? thr_msg_last_time[t] : 0u;
}

/* user32!GetMessagePos: DWORD f(void). Learn: "the cursor position for the last message retrieved by the GetMessage function"; x the low short, y the high short. */
void fle_imp_GetMessagePos(void)
{
    unsigned t = fle_thread_current ? fle_thread_current() : 0u;

    R[0] = t < 256u ? lparam_xy(thr_msg_last_x[t], thr_msg_last_y[t]) : 0u;
}

/* user32!GetMessageExtraInfo: LPARAM f(void). The value SetMessageExtraInfo set; nothing sets it here and no device gives one: 0 (S-extrainfo-zero). */
void fle_imp_GetMessageExtraInfo(void) { R[0] = 0; }

/* user32!GetDoubleClickTime: UINT f(void). See S-doubleclick-time. */
void fle_imp_GetDoubleClickTime(void) { R[0] = DOUBLE_CLICK_MS; }

/* user32!GetKeyboardState: BOOL f(PBYTE lpKeyState rcx). Learn: copies the status of the 256 virtual keys to the buffer; the state is the thread's (as of its key messages): key_sync. Nonzero on success; a NULL buffer: 0 and
 * ERROR_NOACCESS... the page names no code: the call stops (S-no-null-buffer). */
void fle_imp_GetKeyboardState(void)
{
    uint64_t p = R[1];
    unsigned k;

    if (p == 0) fle_stop("GetKeyboardState", "lpKeyState is NULL: the page gives no behaviour");
    for (k = 0; k < 256u; k++) fle_st(p + k, 1, key_sync[k]);
    R[0] = 1;
}

/* Decision 0171 (PROVISIONAL, wall ia1-W2: the toolbar's Shift/Ctrl/Alt buttons): user32!SetKeyboardState: BOOL f(LPBYTE lpKeyState rcx). Learn: "Copies an array of keyboard key states into the calling thread's
 * keyboard input-state table. This is the same table accessed by the GetKeyboardState and GetKeyState functions. Changes made to this table do not affect keyboard input to any other thread"; nonzero on success.
 * The table is key_sync (GetKeyState, GetKeyboardState); key_async (GetAsyncKeyState, the hardware) is not touched, and nothing is posted. A NULL buffer: the page gives no behaviour, the call stops (as GetKeyboardState does). */
void fle_imp_SetKeyboardState(void)
{
    uint64_t p = R[1];
    unsigned k;

    if (p == 0) fle_stop("SetKeyboardState", "lpKeyState is NULL: the page gives no behaviour");
    for (k = 0; k < 256u; k++) key_sync[k] = (uint8_t)fle_ld(p + k, 1);
    R[0] = 1;
}

/* user32!MapVirtualKeyW: UINT f(UINT uCode ecx, UINT uMapType edx). Learn: MAPVK_VK_TO_VSC (0), MAPVK_VSC_TO_VK (1), MAPVK_VK_TO_CHAR (2), MAPVK_VSC_TO_VK_EX (3), MAPVK_VK_TO_VSC_EX (4); 0 when there is no
 * translation. The US layout, scan code set 1 (S-us-layout): letters, digits, the function keys, the editing keys; every other code maps to 0. */

void fle_imp_MapVirtualKeyW(void)
{
    uint32_t code = (uint32_t)R[1], type = (uint32_t)R[2], k;

    switch (type) {
    case 0: case 4:
        R[0] = code < 256u ? scan_of_vk[code] : 0u;
        return;
    case 1: case 3:
        for (k = 1; k < 256u; k++)
            if (scan_of_vk[k] == (code & 0xFFu) && scan_of_vk[k]) {
                R[0] = k;
                return;
            }
        R[0] = 0;
        return;
    case 2:
        R[0] = vk_to_char(code, 0, 0, 0) & 0xFFu;
        if (code >= 'A' && code <= 'Z') R[0] = code;     /* the page: the character is the unshifted one, "uppercase" for a letter */
        return;
    default:
        fle_stop("MapVirtualKeyW", "uMapType is not one of the five MAPVK values of the page");
    }
}

/* user32!MessageBeep: BOOL f(UINT uType ecx). Learn: "Plays a waveform sound"; nonzero on success. There is no sound device: nothing is played and the call succeeds (S-beep-silent). */
void fle_imp_MessageBeep(void) { R[0] = 1; }
