/* DirectSound scenarios for decision 0139 (docs/decisions/0139-audio-s0-s2.md, PROVISIONAL): the guest's part is played by C functions ("guest functions" of a dispatch table, as in harness/web/threads/thread_scenarios.c), the runtime is
 * the real one (fle_dsound.c, fle_audio_ring.c, fle_thread.c, fle_event.c, fle_sys.c ...). The COM calls go through the vtables exactly as the lifted engine does: `call [vtable + 8 * slot]` is fle_dispatch(slot), which ends in
 * fle_dispatch_miss and the hook fle_com_try_slot. usage: <program> list | <scenario name>. Everything printed is virtual (no host address, no host time): the tests compare it with values computed by hand from the Learn rules. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_sys.h"
#include "fle_event.h"
#include "fle_win.h"
#include "fle_thread.h"
#include "fle_dsound.h"
#include "fle_audio_ring.h"

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
#define INF 0xFFFFFFFFu

/* a call of a runtime stub or of a COM slot with up to 8 arguments, the way the lifted code makes it: arguments 5.. at [rsp + 0x28 ..], the return address at [rsp] */
static uint64_t callv(void (*stub)(void), uint64_t slot_va, int n, const uint64_t *a)
{
    uint64_t sp0 = R[4];
    int i;

    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    for (i = 4; i < n; i++) fle_st(R[4] + 0x28 + 8u * (unsigned)(i - 4), 8, a[i]);
    R[1] = n > 0 ? a[0] : 0; R[2] = n > 1 ? a[1] : 0; R[8] = n > 2 ? a[2] : 0; R[9] = n > 3 ? a[3] : 0;
    if (stub) stub(); else fle_dispatch(slot_va);
    R[4] = sp0;
    return R[0];
}
#define STUB(f, ...) ({ uint64_t a_[] = { __VA_ARGS__ }; callv(f, 0, (int)(sizeof a_ / sizeof a_[0]), a_); })
/* method of slot `slot` of the object `obj`: obj is the interface pointer, its first 8 bytes the vtable pointer */
#define VC(obj, slot, ...) ({ uint64_t o_ = (obj); uint64_t a_[] = { o_, __VA_ARGS__ }; callv(NULL, fle_ld(fle_ld(o_, 8) + 8u * (slot), 8), (int)(sizeof a_ / sizeof a_[0]), a_); })
#define VC0(obj, slot) ({ uint64_t o_ = (obj); uint64_t a_[] = { o_ }; callv(NULL, fle_ld(fle_ld(o_, 8) + 8u * (slot), 8), 1, a_); })

enum { QI = 0, ADDREF = 1, RELEASE = 2 };
enum { DS_CREATESOUNDBUFFER = 3, DS_GETCAPS = 4, DS_DUPLICATE = 5, DS_SETCOOP = 6, DS_COMPACT = 7 };
enum { B_GETCAPS = 3, B_GETCURPOS = 4, B_GETFORMAT = 5, B_GETVOLUME = 6, B_GETPAN = 7, B_GETFREQ = 8, B_GETSTATUS = 9, B_INITIALIZE = 10, B_LOCK = 11, B_PLAY = 12, B_SETCURPOS = 13, B_SETFORMAT = 14,
       B_SETVOLUME = 15, B_SETPAN = 16, B_SETFREQ = 17, B_STOP = 18, B_UNLOCK = 19, B_RESTORE = 20, B_SETFX = 21 };
enum { N_SETPOS = 3 };

static int g_argc;
static char **g_argv;
static uint32_t CLOCK_MS(void) { return (uint32_t)(fle_sys_clock_ticks() / FLE_TICKS_PER_MS); }
static uint64_t TICKS(void) { return fle_sys_clock_ticks(); }
static void SLEEP(uint32_t ms) { STUB(fle_imp_Sleep, ms); }
static uint64_t H_EVENT(int manual, int init) { return STUB(fle_imp_CreateEventW, 0, (uint64_t)manual, (uint64_t)init, 0); }
static uint64_t H_CREATE(uint64_t start, uint64_t param) { return STUB(fle_imp_CreateThread, 0, 0, start, param, 0, 0); }

static const uint8_t IID_Notify[16] = { 0x83, 0x07, 0x21, 0xB0, 0xCD, 0x89, 0xD0, 0x11, 0xAF, 0x08, 0x00, 0xA0, 0xC9, 0x25, 0xCD, 0x16 };
static const uint8_t IID_DS[16]     = { 0x83, 0xFA, 0x9A, 0x27, 0x81, 0x49, 0xCE, 0x11, 0xA5, 0x21, 0x00, 0x20, 0xAF, 0x0B, 0xE5, 0x60 };
static const uint8_t IID_DS8[16]    = { 0x93, 0x7E, 0x0A, 0xC5, 0x95, 0xF3, 0x34, 0x48, 0x9E, 0xF6, 0x7F, 0xA9, 0x9D, 0xE5, 0x09, 0x66 };
static const uint8_t IID_Buf8[16]   = { 0x49, 0xA4, 0x25, 0x68, 0x24, 0x75, 0x82, 0x4D, 0x92, 0x0F, 0x50, 0xE3, 0x6A, 0xB3, 0xAB, 0x1E };
static const uint8_t IID_Bogus[16]  = { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 };

/* ---- guest functions ---- */
#define VA(k) (0x1000u + 0x10u * (k))
static int cb_calls, cb_stop_after;
static void enum_cb(void)
{
    uint64_t g = R[1], d = R[2], m = R[8], ctx = R[9];
    char gs[64] = "NULL";

    if (g) { snprintf(gs, sizeof gs, "%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x", (unsigned)fle_ld(g + 3, 1), (unsigned)fle_ld(g + 2, 1), (unsigned)fle_ld(g + 1, 1), (unsigned)fle_ld(g, 1),
             (unsigned)fle_ld(g + 5, 1), (unsigned)fle_ld(g + 4, 1), (unsigned)fle_ld(g + 7, 1), (unsigned)fle_ld(g + 6, 1), (unsigned)fle_ld(g + 8, 1), (unsigned)fle_ld(g + 9, 1), (unsigned)fle_ld(g + 10, 1),
             (unsigned)fle_ld(g + 11, 1), (unsigned)fle_ld(g + 12, 1), (unsigned)fle_ld(g + 13, 1), (unsigned)fle_ld(g + 14, 1), (unsigned)fle_ld(g + 15, 1)); }
    printf("callback %d: guid=%s description=\"%s\" module=\"%s\" ctx=%llu depth=%u\n", ++cb_calls, gs, (const char *)(uintptr_t)d, (const char *)(uintptr_t)m, U(ctx), fle_callback_depth());
    R[0] = (cb_stop_after && cb_calls >= cb_stop_after) ? 0 : 1;
}
static void enum_cb_w(void)
{
    uint64_t d = R[2];
    char s[64];
    unsigned i;

    for (i = 0; i < 63 && fle_ld(d + 2u * i, 2); i++) s[i] = (char)fle_ld(d + 2u * i, 2);
    s[i] = 0;
    printf("callbackW %d: guid=%s description=\"%s\" module-first-unit=%u\n", ++cb_calls, R[1] ? "set" : "NULL", s, (unsigned)fle_ld(R[8], 2));
    R[0] = 1;
}

/* the notification mixer: waits on the two events, prints which and when, and "writes" nothing */
static uint64_t g_evs[2];
static int g_wakes;
static void th_notify_mixer(void)
{
    int n;

    for (n = 0; n < 6; n++) {
        uint64_t hs[2] = { g_evs[0], g_evs[1] };
        uint64_t r = STUB(fle_imp_WaitForMultipleObjects, 2, A(hs), 0, INF);

        printf("mixer woke: index %llu at %llu ticks (%u ms)\n", U(r), U(TICKS()), CLOCK_MS());
        g_wakes++;
    }
}

static void wndproc(void) { R[0] = (R[2] == 0x81) ? 1 : 0; }

static uint64_t g_never;
static void th_sleeper(void) { STUB(fle_imp_WaitForSingleObject, g_never, INF); }

const fle_fn_entry fle_dispatch_table[] = {
    { VA(0), enum_cb },
    { VA(1), enum_cb_w },
    { VA(2), th_notify_mixer },
    { VA(3), th_sleeper },
    { 0x2000, wndproc },
};
const uint64_t fle_dispatch_count = sizeof fle_dispatch_table / sizeof fle_dispatch_table[0];

static void boot(void) { R[4] = E0; fle_st(E0, 8, RET0); fle_teb_set_stack(A(stk), A(stk) + sizeof stk); }

/* ---- helpers for the scenarios ---- */
static uint64_t hwnd_;
static uint64_t mkwnd(void)
{
    static uint16_t cls[] = { 'T','P','U','t','i','l','W','i','n','d','o','w', 0 };
    static uint8_t wc[72];
    uint64_t inst = 0x400000, nm = A(cls), sp0 = R[4], h, w = 0x2000;

    memset(wc, 0, sizeof wc); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
    STUB(fle_imp_RegisterClassW, A(wc));
    R[4] = (R[4] - 0x80) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, 0); fle_st(R[4] + 0x30, 8, 0); fle_st(R[4] + 0x38, 8, 0); fle_st(R[4] + 0x40, 8, 0);
    fle_st(R[4] + 0x48, 8, 0); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, inst); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0x80; R[2] = A(cls); R[8] = 0; R[9] = 0x80000000u;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}

typedef struct { uint16_t tag, ch; uint32_t rate, avg; uint16_t align, bits, cb; } __attribute__((packed)) wfx_t;
static wfx_t pcm(uint32_t rate, uint16_t ch, uint16_t bits) { wfx_t w = { 1, ch, rate, rate * ch * bits / 8u, (uint16_t)(ch * bits / 8), bits, 0 }; return w; }
static wfx_t flt(uint32_t rate, uint16_t ch) { wfx_t w = { 3, ch, rate, rate * ch * 4u, (uint16_t)(ch * 4), 32, 0 }; return w; }
typedef struct { uint32_t size, flags, bytes, reserved; uint64_t fmt; uint8_t guid[16]; } __attribute__((packed)) bufdesc_t;

static uint64_t make_device(int v8)
{
    uint64_t pp = A(&(uint64_t[]){ 0 });
    static uint64_t slot[4];
    static int k;
    uint64_t *p = &slot[k++ & 3];

    (void)pp;
    *p = 0;
    uint64_t hr = STUB(v8 ? fle_imp_DirectSoundCreate8 : fle_imp_DirectSoundCreate, 0, A(p), 0);
    if (hr) printf("create failed %llx\n", U(hr));
    return *p;
}
static uint64_t make_buf(uint64_t dev, uint32_t flags, uint32_t bytes, const void *fmt, uint32_t *hr_out)
{
    static uint64_t slot[8];
    static int k;
    uint64_t *p = &slot[k++ & 7];
    bufdesc_t d;

    memset(&d, 0, sizeof d);
    d.size = 40; d.flags = flags; d.bytes = bytes; d.fmt = A(fmt);
    *p = 0;
    uint64_t hr = VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(p), 0);
    if (hr_out) *hr_out = (uint32_t)hr;
    return *p;
}

/* sets up the "engine" device: window, DirectSound, PRIORITY, primary buffer with the format, one secondary of `frames` frames */
typedef struct { uint64_t dev, prim, sec; int16_t *data; uint32_t bytes; } rig_t;
static rig_t rig_make(uint32_t rate, uint32_t frames, uint32_t sec_flags)
{
    rig_t r;
    wfx_t f = pcm(rate, 2, 16);
    uint32_t hr;

    memset(&r, 0, sizeof r);
    hwnd_ = mkwnd();
    r.dev = make_device(0);
    printf("SetCooperativeLevel %llx\n", U(VC(r.dev, DS_SETCOOP, hwnd_, 2)));
    r.prim = make_buf(r.dev, 1, 0, NULL, &hr);
    printf("primary %llx\n", U(hr));
    printf("SetFormat %llx\n", U(VC(r.prim, B_SETFORMAT, A(&f))));
    r.bytes = frames * 4;
    r.sec = make_buf(r.dev, sec_flags, r.bytes, &f, &hr);
    printf("secondary %llx\n", U(hr));
    return r;
}
static void fill_ramp(rig_t *r)   /* frame k: L = k, R = -k (16 bit); written through Lock / Unlock over the whole buffer */
{
    uint64_t p1 = 0, p2 = 0, hr;
    uint32_t n1 = 0, n2 = 0;
    uint32_t k, frames = r->bytes / 4;
    int16_t *d;

    hr = VC(r->sec, B_LOCK, 0, r->bytes, A(&p1), A(&n1), A(&p2), A(&n2), 2);
    printf("Lock entire hr=%llx n1=%llu p2=%llu n2=%llu\n", U(hr), U(n1), U(p2), U(n2));
    d = (int16_t *)(uintptr_t)p1;
    for (k = 0; k < frames; k++) { d[2 * k] = (int16_t)k; d[2 * k + 1] = (int16_t)-(int)k; }
    printf("Unlock hr=%llx\n", U(VC(r->sec, B_UNLOCK, p1, n1, p2, n2)));
}

/* ===================================== scenarios ===================================== */
static void sc_enum(void)
{
    printf("hr=%llx\n", U(STUB(fle_imp_DirectSoundEnumerateA, VA(0), 7)));
    cb_calls = 0; cb_stop_after = 1;
    printf("hr=%llx (callback stops after one)\n", U(STUB(fle_imp_DirectSoundEnumerateA, VA(0), 8)));
    cb_calls = 0; cb_stop_after = 0;
    printf("hr=%llx (wide)\n", U(STUB(fle_imp_DirectSoundEnumerateW, VA(1), 0)));
    printf("hr=%llx (NULL callback)\n", U(STUB(fle_imp_DirectSoundEnumerateA, 0, 0)));
}

static void sc_create(void)
{
    uint64_t p = 0, ds, ds8, q = 0, bad[16] = { 0x1111111111111111ull, 0x2222222222222222ull };
    uint8_t guid[16] = { 0x00, 0x1E, 0x5D, 0x0F, 0x01, 0x00, 0x00, 0x40, 0x80, 0x00, 0x46, 0x4C, 0x53, 0x54, 0x55, 0x44 };

    printf("Create hr=%llx\n", U(STUB(fle_imp_DirectSoundCreate, 0, A(&p), 0)));
    ds = p;
    printf("vtable first slot is a pseudo-VA in the COM range: %d\n", fle_ld(fle_ld(ds, 8), 8) >= FLE_COM_SLOT_BASE && fle_ld(fle_ld(ds, 8), 8) < FLE_COM_SLOT_BASE + 0x10000);
    p = 0; printf("Create(device guid) hr=%llx\n", U(STUB(fle_imp_DirectSoundCreate, A(guid), A(&p), 0)));
    p = 5; printf("Create(bad guid) hr=%llx ppDS=%llx\n", U(STUB(fle_imp_DirectSoundCreate, A(bad), A(&p), 0)), U(p));
    p = 5; printf("Create(pUnkOuter) hr=%llx ppDS=%llx\n", U(STUB(fle_imp_DirectSoundCreate, 0, A(&p), 1)), U(p));
    printf("Create(NULL pp) hr=%llx\n", U(STUB(fle_imp_DirectSoundCreate, 0, 0, 0)));
    printf("QI IDirectSound hr=%llx same=%d\n", U(VC(ds, QI, A(IID_DS), A(&q))), q == ds);
    q = 7; printf("QI IDirectSound8 on a v1 object hr=%llx out=%llx\n", U(VC(ds, QI, A(IID_DS8), A(&q))), U(q));
    q = 7; printf("QI bogus hr=%llx out=%llx\n", U(VC(ds, QI, A(IID_Bogus), A(&q))), U(q));
    printf("AddRef %llu Release %llu Release %llu\n", U(VC0(ds, ADDREF)), U(VC0(ds, RELEASE)), U(VC0(ds, RELEASE)));
    p = 0; printf("Create8 hr=%llx\n", U(STUB(fle_imp_DirectSoundCreate8, 0, A(&p), 0)));
    ds8 = p;
    q = 0; printf("QI IDirectSound8 on a v8 object hr=%llx same=%d\n", U(VC(ds8, QI, A(IID_DS8), A(&q))), q == ds8);
    {
        uint8_t caps[96];
        uint64_t i;

        memset(caps, 0xEE, sizeof caps); *(uint32_t *)caps = 96;
        printf("GetCaps hr=%llx\n", U(VC(ds8, DS_GETCAPS, A(caps))));
        printf("caps flags=%x min=%u max=%u primary=%u rest-zero=", ((uint32_t *)caps)[1], ((uint32_t *)caps)[2], ((uint32_t *)caps)[3], ((uint32_t *)caps)[4]);
        for (i = 5; i < 24; i++) if (((uint32_t *)caps)[i]) break;
        printf("%d\n", i == 24);
        *(uint32_t *)caps = 95;
        printf("GetCaps(bad size) hr=%llx\n", U(VC(ds8, DS_GETCAPS, A(caps))));
    }
}

static void sc_errors(void)
{
    rig_t r;
    wfx_t f = pcm(44100, 2, 16), f24 = pcm(44100, 2, 24), fbad = pcm(44100, 2, 16), f22 = pcm(22050, 2, 16);
    uint32_t hr;
    uint64_t dev, prim, sec, n, tmp = 0, sz = 0;
    uint8_t buf[64];

    fbad.align = 3;
    hwnd_ = mkwnd();
    dev = make_device(1);
    printf("SetCooperativeLevel(hwnd 0, PRIORITY) %llx\n", U(VC(dev, DS_SETCOOP, 0, 2)));
    printf("SetCooperativeLevel(level 9) %llx\n", U(VC(dev, DS_SETCOOP, hwnd_, 9)));
    printf("SetCooperativeLevel(a handle that is no window) %llx\n", U(VC(dev, DS_SETCOOP, 0x1234, 2)));
    prim = make_buf(dev, 1, 0, NULL, &hr);
    printf("primary hr=%x\n", hr);
    printf("SetFormat before a cooperative level %llx\n", U(VC(prim, B_SETFORMAT, A(&f))));
    printf("SetCooperativeLevel(NORMAL) %llx\n", U(VC(dev, DS_SETCOOP, hwnd_, 1)));
    printf("SetFormat under NORMAL %llx\n", U(VC(prim, B_SETFORMAT, A(&f))));
    printf("SetCooperativeLevel(PRIORITY) %llx\n", U(VC(dev, DS_SETCOOP, hwnd_, 2)));
    printf("SetFormat 24-bit %llx\n", U(VC(prim, B_SETFORMAT, A(&f24))));
    printf("SetFormat bad block align %llx\n", U(VC(prim, B_SETFORMAT, A(&fbad))));
    printf("SetFormat NULL %llx\n", U(VC(prim, B_SETFORMAT, 0)));
    printf("SetFormat %llx\n", U(VC(prim, B_SETFORMAT, A(&f))));
    n = VC(prim, B_GETFORMAT, 0, 0, A(&sz));
    printf("GetFormat(size query) hr=%llx size=%llu\n", U(n), U(sz));
    memset(buf, 0, sizeof buf);
    printf("GetFormat(too small) %llx\n", U(VC(prim, B_GETFORMAT, A(buf), 10, A(&tmp))));
    printf("GetFormat %llx written=%llu tag=%u ch=%u rate=%u bits=%u\n", U(VC(prim, B_GETFORMAT, A(buf), 64, A(&sz))), U(sz), buf[0] | (buf[1] << 8), buf[2], *(uint32_t *)(buf + 4), buf[14]);
    printf("second primary: (stops in a separate scenario)\n");
    {
        bufdesc_t d;
        uint64_t pp = 0;

        memset(&d, 0, sizeof d);
        d.size = 40; d.flags = 1; d.bytes = 100;
        printf("primary with bytes %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 0)));
        d.bytes = 0; d.fmt = A(&f);
        printf("primary with a format %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 0)));
        d.flags = 0x8000; d.bytes = 1000; d.fmt = 0;
        printf("secondary without a format %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 0)));
        d.fmt = A(&f24);
        printf("secondary 24-bit %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 0)));
        d.fmt = A(&f); d.bytes = 1002;
        printf("secondary of 1002 bytes (not a whole number of 4-byte blocks) %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 0)));
        d.bytes = 2; printf("secondary of 2 bytes %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 0)));
        d.bytes = 1000; d.size = 39; printf("bad dwSize %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 0)));
        d.size = 40; d.flags = 0x8000 | 0x10; printf("CTRL3D: (stops, separate scenario)\n");
        d.flags = 0x8000 | 0x4; printf("LOCHARDWARE %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 0)));
        d.flags = 0x8000 | 0x4 | 0x8; printf("LOCHARDWARE|LOCSOFTWARE %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 1)));
        d.flags = 0x8000 | 0x80000000u; printf("unknown flag %llx\n", U(VC(dev, DS_CREATESOUNDBUFFER, A(&d), A(&pp), 0)));
        (void)f22;
    }
    sec = make_buf(dev, 0x8000 | 0x100 | 0x80 | 0x40, 3528, &f, &hr);
    printf("secondary hr=%x\n", hr);
    {
        uint64_t p1 = 0, p2 = 0;
        uint32_t n1 = 0, n2 = 0;

        printf("primary Play without LOOPING %llx\n", U(VC(prim, B_PLAY, 0, 0, 0)));
        printf("primary Play LOOPING %llx\n", U(VC(prim, B_PLAY, 0, 0, 1)));
        printf("Play(reserved 1) %llx\n", U(VC(sec, B_PLAY, 1, 0, 0)));
        printf("Play(priority 5) %llx\n", U(VC(sec, B_PLAY, 0, 5, 0)));
        printf("Play(LOCSOFTWARE on a buffer without LOCDEFER) %llx\n", U(VC(sec, B_PLAY, 0, 0, 4)));
        printf("SetCurrentPosition(on the primary) %llx\n", U(VC(prim, B_SETCURPOS, 0)));
        printf("SetCurrentPosition(3528) %llx\n", U(VC(sec, B_SETCURPOS, 3528)));
        printf("SetCurrentPosition(3) %llx\n", U(VC(sec, B_SETCURPOS, 3)));
        printf("Unlock without Lock %llx\n", U(VC(sec, B_UNLOCK, 0, 0, 0, 0)));
        printf("Lock(offset 3528) %llx\n", U(VC(sec, B_LOCK, 3528, 4, A(&p1), A(&n1), A(&p2), A(&n2), 0)));
        printf("Lock(bytes 3532) %llx\n", U(VC(sec, B_LOCK, 0, 3532, A(&p1), A(&n1), A(&p2), A(&n2), 0)));
        printf("Lock(0 bytes) %llx\n", U(VC(sec, B_LOCK, 0, 0, A(&p1), A(&n1), A(&p2), A(&n2), 0)));
        printf("Lock(bad flag) %llx\n", U(VC(sec, B_LOCK, 0, 4, A(&p1), A(&n1), A(&p2), A(&n2), 4)));
        printf("Lock(on the primary) %llx\n", U(VC(prim, B_LOCK, 0, 4, A(&p1), A(&n1), A(&p2), A(&n2), 0)));
        printf("Lock(3000, 1000) %llx", U(VC(sec, B_LOCK, 3000, 1000, A(&p1), A(&n1), A(&p2), A(&n2), 0)));
        printf(" n1=%llu n2=%llu p2-minus-p1=%lld\n", U(n1), U(n2), (long long)(p2 - p1));
        printf("second Lock %llx\n", U(VC(sec, B_LOCK, 0, 4, A(&p1), A(&n1), A(&p2), A(&n2), 0)));
        printf("SetCooperativeLevel while locked %llx\n", U(VC(dev, DS_SETCOOP, hwnd_, 2)));
        printf("Unlock(more bytes than locked) %llx\n", U(VC(sec, B_UNLOCK, p1, n1 + 1, p2, n2)));
        printf("Unlock(wrong second pointer) %llx\n", U(VC(sec, B_UNLOCK, p1, n1, p2 + 4, n2)));
        printf("Unlock(fewer bytes) %llx\n", U(VC(sec, B_UNLOCK, p1, 100, p2, 0)));
        printf("Lock without ppvAudioPtr2 (3000, 1000) %llx", U(VC(sec, B_LOCK, 3000, 1000, A(&p1), A(&n1), 0, 0, 0)));
        printf(" n1=%llu\n", U(n1));
        printf("Unlock %llx\n", U(VC(sec, B_UNLOCK, p1, n1, 0, 0)));
    }
    {
        uint64_t st = 99, pl = 99, wr = 99;
        uint64_t evs[2] = { H_EVENT(0, 0), H_EVENT(0, 0) };
        struct { uint32_t off; uint32_t pad; uint64_t h; } pos[2] = { { 0, 0, evs[0] }, { 1764, 0, evs[1] } };
        uint64_t nt = 0, ppi = 0;

        printf("GetStatus (stopped) %llx st=%llu\n", U(VC(sec, B_GETSTATUS, A(&st))), U(st));
        printf("GetCurrentPosition (stopped) %llx\n", U(VC(sec, B_GETCURPOS, A(&pl), A(&wr)))); printf("  play=%llu write=%llu\n", U(pl), U(wr));
        printf("GetCurrentPosition(NULL, NULL) %llx\n", U(VC(sec, B_GETCURPOS, 0, 0)));
        printf("QI notify %llx\n", U(VC(sec, QI, A(IID_Notify), A(&nt))));
        printf("notify pointer differs from the buffer: %d\n", nt != sec);
        printf("SetNotificationPositions(2) %llx\n", U(VC(nt, N_SETPOS, 2, A(pos))));
        pos[1].off = 3528; printf("SetNotificationPositions(offset = size) %llx\n", U(VC(nt, N_SETPOS, 2, A(pos))));
        pos[1].off = 1764; pos[1].h = 0x1234; printf("SetNotificationPositions(bad event) %llx\n", U(VC(nt, N_SETPOS, 2, A(pos))));
        pos[1].h = evs[1]; pos[0].off = 0xFFFFFFFFu; printf("SetNotificationPositions(OFFSETSTOP not last) %llx\n", U(VC(nt, N_SETPOS, 2, A(pos))));
        printf("SetNotificationPositions(NULL, 2) %llx\n", U(VC(nt, N_SETPOS, 2, 0)));
        pos[0].off = 0;
        printf("Play %llx\n", U(VC(sec, B_PLAY, 0, 0, 1)));
        printf("SetNotificationPositions while playing %llx\n", U(VC(nt, N_SETPOS, 2, A(pos))));
        printf("GetStatus (playing) %llx st=%llu\n", U(VC(sec, B_GETSTATUS, A(&st))), U(st));
        printf("Stop %llx\n", U(VC(sec, B_STOP)));
        printf("GetStatus (stopped) %llx st=%llu\n", U(VC(sec, B_GETSTATUS, A(&st))), U(st));
        printf("Restore %llx\n", U(VC(sec, B_RESTORE)));
        printf("QI buffer8 on a v8 device %llx\n", U(VC(sec, QI, A(IID_Buf8), A(&ppi))));
        printf("SetVolume(-100) %llx\n", U(VC(sec, B_SETVOLUME, (uint64_t)(uint32_t)-100)));
        printf("SetVolume(1) %llx\n", U(VC(sec, B_SETVOLUME, 1)));
        printf("SetVolume(-10001) %llx\n", U(VC(sec, B_SETVOLUME, (uint64_t)(uint32_t)-10001)));
        printf("SetPan(10001) %llx\n", U(VC(sec, B_SETPAN, 10001)));
        printf("SetFrequency(22050) on a CTRLFREQUENCY-less buffer %llx\n", U(VC(sec, B_SETFREQ, 22050)));
        printf("GetVolume %llx v=%d\n", U(VC(sec, B_GETVOLUME, A(&st))), (int)(int32_t)st);
        printf("refs: Release x2 (QI added two) %llu %llu\n", U(VC0(nt, RELEASE)), U(VC0(ppi, RELEASE)));
    }
    (void)r;
}

/* the engine-like flow with a 10 ms block (441 frames at 44100 Hz): hand-computed expectations in tests/wall_fr15.rs */
static void sc_play(void)
{
    rig_t r = rig_make(44100, 882, 0x8000 | 0x10000);
    fle_audio_ring_t *ring = fle_audio_ring_get();
    uint64_t pl = 0, wr = 0, t0, k;
    float *d = fle_audio_ring_data();

    fill_ramp(&r);
    printf("SetCurrentPosition(0) %llx\n", U(VC(r.sec, B_SETCURPOS, 0)));
    t0 = TICKS();
    printf("Play %llx\n", U(VC(r.sec, B_PLAY, 0, 0, 1)));
    printf("ring after Play: state=%u rate=%u write=%llu read=%llu fill=%u lead=%u\n", ring->state, ring->sample_rate, U(ring->write_frames), U(ring->read_frames), fle_audio_ring_fill(), ring->lead_frames);
    for (k = 1; k <= 8; k++) {
        uint64_t tk;

        SLEEP(1);
        tk = TICKS() - t0;
        VC(r.sec, B_GETCURPOS, A(&pl), A(&wr));
        printf("t+%llu ms: ticks %llu play=%llu write=%llu | consumed=%llu write_frames=%llu\n", U(k), U(tk), U(pl), U(wr), U(fle_audio_consumed()), U(ring->write_frames));
    }
    SLEEP(40);   /* one thread: Sleep moves the clock with no scheduler switch, so the next poll finds the device 40 ms behind (the buffers did not change meanwhile) */
    {
        uint64_t tk = TICKS() - t0;

        VC(r.sec, B_GETCURPOS, A(&pl), A(&wr));
        printf("t+48 ms: ticks %llu play=%llu write=%llu | consumed=%llu write_frames=%llu read_frames=%llu underruns=%u blocks=%llu late=%llu\n", U(tk), U(pl), U(wr), U(fle_audio_consumed()), U(ring->write_frames),
               U(ring->read_frames), ring->underruns, U(ring->blocks_total), U(fle_dsound_stats()->late_frames));
    }
    for (k = 0; k < ring->write_frames; k += 1) {
        float want = (float)(int16_t)(k % 882) / 32768.0f;

        if (d[(k % ring->capacity) * 2] != want || d[(k % ring->capacity) * 2 + 1] != -want) { printf("RING MISMATCH at frame %llu\n", U(k)); break; }
    }
    printf("ring content checked up to frame %llu\n", U(ring->write_frames));
    printf("ring frame 5: %.9g %.9g ; frame 881: %.9g %.9g ; frame 882: %.9g %.9g\n", d[10], d[11], d[881 * 2], d[881 * 2 + 1], d[882 * 2], d[882 * 2 + 1]);
    {
        const dsound_stats *s = fle_dsound_stats();

        printf("stats: locks %llu unlocks %llu plays %llu devices %llu secondary %llu\n", U(s->locks), U(s->unlocks), U(s->plays), U(s->devices_created), U(s->secondary_created));
    }
}

static void sc_stop_resume(void)
{
    rig_t r = rig_make(44100, 882, 0x8000 | 0x10000);
    fle_audio_ring_t *ring = fle_audio_ring_get();
    uint64_t pl = 0, wr = 0, c_resume;
    float *d = fle_audio_ring_data();

    fill_ramp(&r);
    VC(r.sec, B_PLAY, 0, 0, 1);
    SLEEP(5);
    printf("Stop %llx\n", U(VC(r.sec, B_STOP)));
    VC(r.sec, B_GETCURPOS, A(&pl), A(&wr));
    printf("after Stop: play=%llu write=%llu ring write_frames=%llu state=%u\n", U(pl), U(wr), U(ring->write_frames), ring->state);
    SLEEP(30);
    VC(r.sec, B_GETCURPOS, A(&pl), A(&wr));
    printf("30 ms later (stopped): play=%llu write=%llu ring write_frames=%llu\n", U(pl), U(wr), U(ring->write_frames));
    c_resume = fle_audio_consumed();
    printf("Play %llx (device consumed %llu frames before it)\n", U(VC(r.sec, B_PLAY, 0, 0, 1)), U(c_resume));
    SLEEP(2);
    VC(r.sec, B_GETCURPOS, A(&pl), A(&wr));
    printf("2 ms after the second Play: play=%llu write=%llu ring write_frames=%llu consumed=%llu late=%llu\n", U(pl), U(wr), U(ring->write_frames), U(fle_audio_consumed()), U(fle_dsound_stats()->late_frames));
    printf("ring frames from the resume index %llu: ", U(c_resume));
    {
        int k;

        for (k = 0; k < 3; k++) printf("%.6g ", d[((c_resume + (uint64_t)k) % ring->capacity) * 2] * 32768.0f);
        printf("\n");
    }
}

static void sc_notify(void)
{
    rig_t r = rig_make(44100, 882, 0x8000 | 0x10000 | 0x100);
    uint64_t nt = 0, h;
    struct { uint32_t off; uint32_t pad; uint64_t h; } pos[2];

    g_evs[0] = H_EVENT(0, 0); g_evs[1] = H_EVENT(0, 0);
    pos[0].off = 0; pos[0].pad = 0; pos[0].h = g_evs[0];
    pos[1].off = 1764; pos[1].pad = 0; pos[1].h = g_evs[1];
    fill_ramp(&r);
    printf("QI notify %llx\n", U(VC(r.sec, QI, A(IID_Notify), A(&nt))));
    printf("SetNotificationPositions %llx\n", U(VC(nt, N_SETPOS, 2, A(pos))));
    printf("SetCurrentPosition(1764) %llx\n", U(VC(r.sec, B_SETCURPOS, 1764)));
    h = H_CREATE(VA(2), 0);
    printf("thread created\n");
    printf("Play %llx at %llu ticks\n", U(VC(r.sec, B_PLAY, 0, 0, 1)), U(TICKS()));
    SLEEP(75);
    printf("main woke at %llu ticks (%u ms); mixer wakes so far %d\n", U(TICKS()), CLOCK_MS(), g_wakes);
    (void)h;
}

static void sc_notify_alone(void)   /* nobody waits for the events: the scheduler's idle clock must still be moved by the notification times */
{
    rig_t r = rig_make(44100, 882, 0x8000 | 0x10000 | 0x100);
    uint64_t nt = 0;
    struct { uint32_t off; uint32_t pad; uint64_t h; } pos[2];

    g_evs[0] = H_EVENT(0, 0); g_evs[1] = H_EVENT(0, 0);
    pos[0].off = 0; pos[0].pad = 0; pos[0].h = g_evs[0];
    pos[1].off = 1764; pos[1].pad = 0; pos[1].h = g_evs[1];
    fill_ramp(&r);
    VC(r.sec, QI, A(IID_Notify), A(&nt));
    VC(nt, N_SETPOS, 2, A(pos));
    VC(r.sec, B_SETCURPOS, 1764);
    VC(r.sec, B_PLAY, 0, 0, 1);
    g_never = H_EVENT(1, 0);
    H_CREATE(VA(3), 0);   /* a thread that waits for ever: the second thread that a wait for INFINITE needs (fle_thread.c stops a single thread by name) */
    {
        uint64_t hs[2] = { g_evs[0], g_evs[1] };
        uint64_t w = STUB(fle_imp_WaitForMultipleObjects, 2, A(hs), 0, INF);

        printf("main alone with a sleeper: woke index %llu at %llu ticks\n", U(w), U(TICKS()));
        w = STUB(fle_imp_WaitForMultipleObjects, 2, A(hs), 0, INF);
        printf("main alone with a sleeper: woke index %llu at %llu ticks\n", U(w), U(TICKS()));
        w = STUB(fle_imp_WaitForMultipleObjects, 2, A(hs), 0, INF);
        printf("main alone with a sleeper: woke index %llu at %llu ticks\n", U(w), U(TICKS()));
    }
}

static void sc_formats(void)   /* mono, 8-bit, float32, volume and pan, two buffers mixed */
{
    wfx_t f16 = pcm(44100, 2, 16), m16 = pcm(44100, 1, 16), u8 = pcm(44100, 2, 8), fl = flt(44100, 2);
    uint64_t dev, prim, a, b, c, d_, p1 = 0, p2 = 0, st = 0;
    uint32_t n1 = 0, n2 = 0;
    float *rd = fle_audio_ring_data();
    fle_audio_ring_t *ring = fle_audio_ring_get();
    uint32_t hr;
    uint64_t base_idx;

    hwnd_ = mkwnd();
    dev = make_device(0);
    VC(dev, DS_SETCOOP, hwnd_, 2);
    prim = make_buf(dev, 1, 0, NULL, &hr);
    VC(prim, B_SETFORMAT, A(&f16));
    /* A: 16-bit stereo, 8 frames of (16384, -8192); volume ctl; B: float, 8 frames of (0.5, 0.25) */
    a = make_buf(dev, 0x8000 | 0x80 | 0x40, 32, &f16, &hr);
    b = make_buf(dev, 0x8000, 64, &fl, &hr);
    c = make_buf(dev, 0x8000, 16, &m16, &hr);
    d_ = make_buf(dev, 0x8000, 16, &u8, &hr);
    {
        int16_t *w;
        float *fw;
        uint8_t *u;
        int k;

        VC(a, B_LOCK, 0, 32, A(&p1), A(&n1), A(&p2), A(&n2), 0); w = (int16_t *)(uintptr_t)p1; for (k = 0; k < 8; k++) { w[2 * k] = 16384; w[2 * k + 1] = -8192; } VC(a, B_UNLOCK, p1, n1, p2, n2);
        VC(b, B_LOCK, 0, 64, A(&p1), A(&n1), A(&p2), A(&n2), 0); fw = (float *)(uintptr_t)p1; for (k = 0; k < 8; k++) { fw[2 * k] = 0.5f; fw[2 * k + 1] = 0.25f; } VC(b, B_UNLOCK, p1, n1, p2, n2);
        VC(c, B_LOCK, 0, 16, A(&p1), A(&n1), A(&p2), A(&n2), 0); w = (int16_t *)(uintptr_t)p1; for (k = 0; k < 8; k++) w[k] = (int16_t)(k * 1000); VC(c, B_UNLOCK, p1, n1, p2, n2);
        VC(d_, B_LOCK, 0, 16, A(&p1), A(&n1), A(&p2), A(&n2), 0); u = (uint8_t *)(uintptr_t)p1; for (k = 0; k < 8; k++) { u[2 * k] = (uint8_t)(k == 0 ? 0 : k == 1 ? 128 : 255); u[2 * k + 1] = 192; } VC(d_, B_UNLOCK, p1, n1, p2, n2);
    }
    /* each buffer alone, non-looping, 8 frames (181 us): it reaches the output one lead (15 ms: the tests run the scenarios with FLE_AUDIO_LEAD_MS=15) after the Play, so the scenario sleeps 20 ms before it looks at the status */
#define FROM() (ring->write_frames > fle_audio_consumed() ? ring->write_frames : fle_audio_consumed())
#define SHOW(from, n) do { uint64_t k_; for (k_ = (from); k_ < (from) + (n); k_++) printf("(%.7g,%.7g) ", rd[(k_ % ring->capacity) * 2], rd[(k_ % ring->capacity) * 2 + 1]); printf("\n"); } while (0)
    {
        uint64_t from;

        printf("A alone:\n");
        from = FROM();
        VC(a, B_PLAY, 0, 0, 0); SLEEP(20); VC(a, B_GETSTATUS, A(&st));
        printf("  status after the end %llu; ring frames: ", U(st)); SHOW(from, 10);
        printf("B alone (float):\n");
        from = FROM();
        VC(b, B_PLAY, 0, 0, 0); SLEEP(20); VC(b, B_GETSTATUS, A(&st));
        printf("  status after the end %llu; ring frames: ", U(st)); SHOW(from, 10);
        printf("C alone (mono 16-bit, frames 0..7 = k*1000):\n");
        from = FROM();
        VC(c, B_PLAY, 0, 0, 0); SLEEP(20); VC(c, B_GETSTATUS, A(&st));
        printf("  status after the end %llu; ring frames: ", U(st)); SHOW(from, 9);
        printf("D alone (8-bit unsigned: 0, 128, 255 / 192):\n");
        from = FROM();
        VC(d_, B_PLAY, 0, 0, 0); SLEEP(20); VC(d_, B_GETSTATUS, A(&st));
        printf("  status after the end %llu; ring frames: ", U(st)); SHOW(from, 3);
        printf("A looping (pan = 10000: left silent; volume -6000: gain 10^-3) and B (float, 8 frames) started later; B joins at the first frame not yet handed to the output:\n");
        VC(a, B_SETPAN, 10000); VC(a, B_SETVOLUME, (uint64_t)(uint32_t)-6000);
        VC(a, B_SETCURPOS, 0); VC(b, B_SETCURPOS, 0);
        VC(a, B_PLAY, 0, 0, 1); SLEEP(1); VC(a, B_GETSTATUS, A(&st));
        from = FROM();
        VC(b, B_PLAY, 0, 0, 0); SLEEP(20); VC(a, B_GETSTATUS, A(&st));
        printf("  ring frames from B's join: "); SHOW(from, 10);
        printf("  frame before the join: "); SHOW(from - 1, 1);
        VC(b, B_GETSTATUS, A(&st)); printf("  B status %llu, A status %llu\n", U(st), U((VC(a, B_GETSTATUS, A(&st)), st)));
    }
}

static void sc_ring_header(void)
{
    fle_audio_ring_t *r = fle_audio_ring_get();

    printf("magic %08x version %u channels %u capacity %u data_offset %u block_log %u sizeof %zu state %u rate %u external %u\n", r->magic, r->version, r->channels, r->capacity, r->data_offset, r->block_log_len,
           sizeof *r, r->state, r->sample_rate, r->external_consumer);
    printf("offsetof write_frames %zu read_frames %zu underruns %zu blocks_total %zu blocks %zu; sizeof block %zu\n", offsetof(fle_audio_ring_t, write_frames), offsetof(fle_audio_ring_t, read_frames),
           offsetof(fle_audio_ring_t, underruns), offsetof(fle_audio_ring_t, blocks_total), offsetof(fle_audio_ring_t, blocks), sizeof(fle_audio_block));
    {
        uint64_t dev = make_device(0);

        printf("after a device: state %u rate %u\n", r->state, r->sample_rate);
        (void)dev;
    }
}

/* rt_mixer <blocks> [<spin_ms> <spin_every>] [<block_frames>]: the polling mixer of FLEngine (f_12aeec0 in its polling form, audio-m5.md section 4.2) with a sine as the "project": a device at 44100 Hz 16-bit stereo, a secondary buffer of two
 * blocks (4096 frames each by default), SetCurrentPosition(block), Play(LOOPING); then loop { Sleep(1); GetCurrentPosition(&play); half = play < block; when half differs from the previous one, write the half that the play cursor is NOT in
 * (Lock, 4096 frames of 440 Hz at amplitude 0.25, Unlock) and check the write cursor as the engine does (the write cursor inside the written half is an underrun) }. spin_ms > 0 makes every spin_every-th block take that much
 * wall time (wasm only: emscripten_get_now), to see what a slow slice of another thread does. Prints one summary line; in a native run everything is in virtual time and exact. */
#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif
static void sc_rt_mixer(void)
{
    uint32_t blocks = g_argc > 2 ? (uint32_t)atoi(g_argv[2]) : 40, spin_ms = g_argc > 3 ? (uint32_t)atoi(g_argv[3]) : 0, spin_every = g_argc > 4 ? (uint32_t)atoi(g_argv[4]) : 0;
    uint32_t bf = g_argc > 5 ? (uint32_t)atoi(g_argv[5]) : 4096, block = bf * 4, total = 2 * block, written = 0, under = 0, polls = 0;
    int prev_half = 0;
    rig_t r = rig_make(44100, 2 * bf, 0x8000 | 0x10000);
    uint64_t pl = 0, wr = 0, p1 = 0, p2 = 0;
    uint32_t n1 = 0, n2 = 0;
    double phase = 0.0;
    uint64_t t_first_write = 0;
    fle_audio_ring_t *ring = fle_audio_ring_get();

    g_never = H_EVENT(1, 0);
    H_CREATE(VA(3), 0);   /* a second thread (it waits for ever): with one thread Sleep moves the clock without a scheduling decision (fle_thread.c), and the engine always has more than one */
    VC(r.sec, B_LOCK, 0, 0, A(&p1), A(&n1), A(&p2), A(&n2), 2);
    memset((void *)(uintptr_t)p1, 0, n1);
    VC(r.sec, B_UNLOCK, p1, n1, p2, n2);
    VC(r.sec, B_SETCURPOS, block);
    VC(r.sec, B_PLAY, 0, 0, 1);
    printf("rt_mixer: block %u frames, total %u bytes, %u blocks\n", bf, total, blocks);
    while (written < blocks) {
        int half;
        uint32_t off, k;
        int16_t *w;

        SLEEP(1);
        VC(r.sec, B_GETCURPOS, A(&pl), 0);
        polls++;
        half = pl < block;
        if (half == prev_half) continue;
        prev_half = half;
        off = half ? block : 0;
        VC(r.sec, B_LOCK, off, block, A(&p1), A(&n1), A(&p2), A(&n2), 0);
        w = (int16_t *)(uintptr_t)p1;
        for (k = 0; k < bf; k++) {
            double v = 0.25 * sin(phase);

            w[2 * k] = w[2 * k + 1] = (int16_t)(v * 32767.0);
            phase += 2.0 * 3.14159265358979323846 * 440.0 / 44100.0;
        }
#ifdef __EMSCRIPTEN__
        if (spin_ms && spin_every && written % spin_every == spin_every - 1) { double t = emscripten_get_now(); while (emscripten_get_now() - t < (double)spin_ms) { } }
#endif
        VC(r.sec, B_UNLOCK, p1, n1, p2, n2);
        if (!written) t_first_write = TICKS();
        VC(r.sec, B_GETCURPOS, 0, A(&wr));
        if (wr >= off && wr <= off + block - 1) under++;   /* the engine's check: the write cursor inside the half just written */
        written++;
    }
    printf("rt_mixer: written %u blocks, %u polls, the engine's underrun check fired %u times, first write at +%llu ticks of the virtual clock\n", written, polls, under, U(t_first_write));
    printf("rt_mixer: ring write_frames %llu read_frames %llu underruns %u\n", U(ring->write_frames), U(ring->read_frames), ring->underruns);
    if (!ring->external_consumer) {
        /* the stream: zeros for the two blocks before the first written block is reached, then the sine, sample for sample (phase continuous) */
        float *d = fle_audio_ring_data();
        uint64_t k, bad = 0, n = ring->write_frames, limit = (uint64_t)blocks * bf, nz = 0;
        double ph = 0.0;

        /* block j (0-based) is written when the play cursor enters the other half, one block before it is played: it is heard at stream frames [(2 + j) * bf, (3 + j) * bf). The check covers j = 0 .. blocks - 2. */
        for (k = 0; k < 2 * (uint64_t)bf; k++) if (d[(k % ring->capacity) * 2] != 0.0f) nz++;
        for (k = 2 * (uint64_t)bf; k < n && k < limit; k++) {
            float want = (float)(int16_t)(0.25 * sin(ph) * 32767.0) / 32768.0f;

            ph += 2.0 * 3.14159265358979323846 * 440.0 / 44100.0;
            if (d[(k % ring->capacity) * 2] != want || d[(k % ring->capacity) * 2 + 1] != want) { bad++; if (bad < 4) printf("rt_mixer: MISMATCH at %llu\n", U(k)); }
        }
        printf("rt_mixer: stream frames 0..%u non-zero: %llu; frames %u..%llu against the sine: %llu mismatches (ring write_frames %llu)\n", 2 * bf - 1, U(nz), 2 * bf, U(limit - 1), U(bad), U(n));
    }
}


/* ---- the stand-in for the AudioWorklet of decision 0241: harness/audio/sim_transport.c (linked in place of runtime/fle_audio_transport.c) ---- */
void fle_sim_external(int on);
void fle_sim_consumed(uint64_t frames);
uint64_t fle_sim_offered_inside_guard(void);

/* ---- decision 0241: S-ds-late-write. The stand-in transport (fle_sim_external) stands for the AudioWorklet: the play position is the count set by fle_audio_sim_consumed, the ring the C ring. The tests run these
 * scenarios with FLE_AUDIO_LEAD_MS=60 (the default lead: 2646 frames at 44100 Hz) and the guard of the runtime is 256 frames. A device of 8192 frames (2 halves of 4096, as FLEngine's mixer makes it) plays LOOPING from frame 0. ---- */
static void ds_write(rig_t *r, uint32_t first, uint32_t n, int16_t (*val)(uint32_t))   /* frames [first, first + n) of the buffer, one Lock / Unlock */
{
    uint64_t p1 = 0, p2 = 0;
    uint32_t n1 = 0, n2 = 0, k;
    int16_t *d;

    VC(r->sec, B_LOCK, (uint64_t)first * 4u, (uint64_t)n * 4u, A(&p1), A(&n1), A(&p2), A(&n2), 0);
    d = (int16_t *)(uintptr_t)p1;
    for (k = 0; k < n1 / 4u; k++) { int16_t v = val(first + k); d[2 * k] = v; d[2 * k + 1] = (int16_t)-v; }
    d = (int16_t *)(uintptr_t)p2;
    for (k = 0; k < n2 / 4u; k++) { int16_t v = val(first + n1 / 4u + k); d[2 * k] = v; d[2 * k + 1] = (int16_t)-v; }
    VC(r->sec, B_UNLOCK, p1, n1, p2, n2);
}
static int16_t val_ramp(uint32_t k) { return (int16_t)(1 + k % 30000u); }   /* never 0, so that "zero" and "ramp" are told apart */
static int16_t val_sine(uint32_t k) { return (int16_t)lrint(8000.0 * sin(6.283185307179586 * 440.0 * (double)k / 44100.0)); }
static int16_t val_zero(uint32_t k) { (void)k; return 0; }

/* late_write <consumed at the write> <first frame> <frames> [<consumed at the synchronisation, default 1000>]: the device plays from frame 0 (consumed 0), is synchronised at consumed 1000 (so 3646 frames are handed to the output: all silence), then the guest writes the ramp into
 * frames [first, first + frames) of the buffer while the output has read `consumed at the write` frames. Prints how many frames of the region the ring holds as the ramp, as silence, the counters, and anything outside the region that changed. */
static void sc_late_write(void)
{
    uint64_t cw = g_argc > 2 ? (uint64_t)atoll(g_argv[2]) : 1000, first = g_argc > 3 ? (uint64_t)atoll(g_argv[3]) : 1500, n = g_argc > 4 ? (uint64_t)atoll(g_argv[4]) : 1000, presync = g_argc > 5 ? (uint64_t)atoll(g_argv[5]) : 1000;
    rig_t r;
    fle_audio_ring_t *ring;
    float *d;
    uint64_t pl = 0, wr = 0, k, ramp = 0, zero = 0, other = 0, outside = 0, wf0;

    fle_sim_external(1);
    r = rig_make(44100, 8192, 0x8000 | 0x10000);
    ring = fle_audio_ring_get();
    d = fle_audio_ring_data();
    fle_sim_consumed(0);
    printf("Play %llx\n", U(VC(r.sec, B_PLAY, 0, 0, 1)));
    fle_sim_consumed(presync);
    VC(r.sec, B_GETCURPOS, A(&pl), A(&wr));
    wf0 = ring->write_frames;
    printf("before the write: consumed=%llu write_frames=%llu lead=%u external=%u\n", U(fle_audio_consumed()), U(wf0), ring->lead_frames, ring->external_consumer);
    fle_sim_consumed(cw);
    ds_write(&r, (uint32_t)first, (uint32_t)n, val_ramp);
    printf("write: consumed=%llu first=%llu frames=%llu\n", U(cw), U(first), U(n));
    for (k = 0; k < ring->write_frames; k++) {
        float v = d[(k % ring->capacity) * 2];
        int in_region = k >= first && k < first + n;

        if (in_region) {
            if (v == (float)val_ramp((uint32_t)k) / 32768.0f && d[(k % ring->capacity) * 2 + 1] == -v) ramp++;
            else if (v == 0.0f && d[(k % ring->capacity) * 2 + 1] == 0.0f) zero++;
            else other++;
        } else if (v != 0.0f || d[(k % ring->capacity) * 2 + 1] != 0.0f) outside++;
    }
    printf("region: ramp frames=%llu silent frames=%llu other frames=%llu ; outside the region: non-silent frames=%llu ; write_frames=%llu\n", U(ramp), U(zero), U(other), U(outside), U(ring->write_frames));
    {
        uint64_t h = UINT64_C(1469598103934665603);

        for (k = 0; k < ring->write_frames * 2u; k++) {
            uint32_t bits;

            memcpy(&bits, &d[(k / 2u % ring->capacity) * 2u + (k & 1u)], 4);
            h = (h ^ bits) * UINT64_C(1099511628211);
        }
        printf("ring checksum (FNV-1a over the float bits of frames 0..%llu): %016llx\n", U(ring->write_frames), U(h));
    }
    {
        const dsound_stats *s = fle_dsound_stats();

        printf("late writes: events=%llu applied=%llu lost=%llu ; late_frames=%llu ; page counters: events=%.0f applied=%.0f lost=%.0f ; frames offered to the transport inside the guard=%llu\n", U(s->late_writes), U(s->late_write_applied),
               U(s->late_write_lost), U(s->late_frames), fle_audio_rewrite_stat(0), fle_audio_rewrite_stat(1), fle_audio_rewrite_stat(2), U(fle_sim_offered_inside_guard()));
    }
}

/* late_write_sine: the mixer of FLEngine in miniature with a continuous sine (440 Hz, amplitude 8000) in two halves of 4096 frames. Half 0 is written before Play. The output reads 2500 frames, the device is synchronised (so frames
 * 4096.. of the still empty half 1 are already handed over as silence), then half 1 is written - in time (its first frame is 1596 ahead of the output) - and the output goes on. Prints how many frames of the ring differ from the
 * ideal stream, and the largest step between neighbouring frames of the ring beside the largest step of the ideal sine (a step at the border of the rewritten frames would show). */
static void sc_late_write_sine(void)
{
    rig_t r;
    fle_audio_ring_t *ring;
    float *d;
    uint64_t pl = 0, wr = 0, k, bad = 0, ncheck;
    double maxstep = 0.0, maxideal = 0.0, prev = 0.0, prevI = 0.0;

    fle_sim_external(1);
    r = rig_make(44100, 8192, 0x8000 | 0x10000);
    ring = fle_audio_ring_get();
    d = fle_audio_ring_data();
    ds_write(&r, 0, 4096, val_sine);
    fle_sim_consumed(0);
    VC(r.sec, B_PLAY, 0, 0, 1);
    fle_sim_consumed(2000);
    VC(r.sec, B_GETCURPOS, A(&pl), A(&wr));
    printf("handed over before half 1 is written: write_frames=%llu\n", U(ring->write_frames));
    fle_sim_consumed(2500);
    ds_write(&r, 4096, 4096, val_sine);
    fle_sim_consumed(4000);
    VC(r.sec, B_GETCURPOS, A(&pl), A(&wr));
    ncheck = ring->write_frames;
    for (k = 0; k < ncheck; k++) {
        double v = d[(k % ring->capacity) * 2], ideal = (double)val_sine((uint32_t)(k % 8192)) / 32768.0;

        if (v != ideal) bad++;
        if (k > 0) {
            double st = v > prev ? v - prev : prev - v, si = ideal > prevI ? ideal - prevI : prevI - ideal;

            if (st > maxstep) maxstep = st;
            if (si > maxideal) maxideal = si;
        }
        prev = v;
        prevI = ideal;
    }
    printf("sine: checked frames=%llu differing from the ideal stream=%llu ; largest step of the ring=%.9g ideal=%.9g\n", U(ncheck), U(bad), maxstep, maxideal);
    {
        const dsound_stats *s = fle_dsound_stats();

        printf("late writes: events=%llu applied=%llu lost=%llu ; frames offered to the transport inside the guard=%llu\n", U(s->late_writes), U(s->late_write_applied), U(s->late_write_lost), U(fle_sim_offered_inside_guard()));
    }
}

/* scenarios that stop */
static void sc_stop_setfx(void) { rig_t r = rig_make(44100, 882, 0x8000); VC(r.sec, B_SETFX, 0, 0, 0); }
static void sc_stop_duplicate(void) { rig_t r = rig_make(44100, 882, 0x8000); uint64_t o = 0; VC(r.dev, DS_DUPLICATE, r.sec, A(&o)); }
static void sc_stop_3d(void) { wfx_t f = pcm(44100, 2, 16); uint64_t dev = make_device(0); make_buf(dev, 0x8000 | 0x10, 400, &f, NULL); }
static void sc_stop_after_release(void) { rig_t r = rig_make(44100, 882, 0x8000); printf("Release %llu\n", U(VC0(r.sec, RELEASE))); VC(r.sec, B_STOP); }
static void sc_stop_resample(void) { wfx_t f = pcm(22050, 2, 16); rig_t r = rig_make(44100, 882, 0x8000); make_buf(r.dev, 0x8000, 400, &f, NULL); }
static void sc_stop_second_primary(void) { rig_t r = rig_make(44100, 882, 0x8000); make_buf(r.dev, 1, 0, NULL, NULL); }
static void sc_stop_writeprimary(void) { uint64_t dev = make_device(0); hwnd_ = mkwnd(); VC(dev, DS_SETCOOP, hwnd_, 4); }
static void sc_stop_bad_this(void) { rig_t r = rig_make(44100, 882, 0x8000); VC(r.sec + 8, B_STOP); (void)r; }

static const struct { const char *name; void (*fn)(void); int stops; } S[] = {
    { "enum", sc_enum, 0 }, { "create", sc_create, 0 }, { "errors", sc_errors, 0 }, { "play", sc_play, 0 }, { "stop_resume", sc_stop_resume, 0 }, { "notify", sc_notify, 0 },
    { "notify_alone", sc_notify_alone, 0 }, { "rt_mixer", sc_rt_mixer, 0 }, { "late_write", sc_late_write, 0 }, { "late_write_sine", sc_late_write_sine, 0 }, { "formats", sc_formats, 0 }, { "ring_header", sc_ring_header, 0 },
    { "stop_setfx", sc_stop_setfx, 1 }, { "stop_duplicate", sc_stop_duplicate, 1 }, { "stop_3d", sc_stop_3d, 1 }, { "stop_after_release", sc_stop_after_release, 1 }, { "stop_resample", sc_stop_resample, 1 },
    { "stop_second_primary", sc_stop_second_primary, 1 }, { "stop_writeprimary", sc_stop_writeprimary, 1 }, { "stop_bad_this", sc_stop_bad_this, 1 },
};

int main(int argc, char **argv)
{
    size_t i;

    g_argc = argc; g_argv = argv;
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc < 2) { fprintf(stderr, "usage: %s list | <scenario>\n", argv[0]); return 2; }
    if (!strcmp(argv[1], "list")) {
        for (i = 0; i < sizeof S / sizeof S[0]; i++) printf("%s%s\n", S[i].name, S[i].stops ? " (stops)" : "");
        return 0;
    }
    for (i = 0; i < sizeof S / sizeof S[0]; i++) {
        if (strcmp(argv[1], S[i].name)) continue;
        boot();
        S[i].fn();
        printf("scenario %s done, threads %u, virtual clock %u ms\n", S[i].name, fle_thread_count(), CLOCK_MS());
        return 0;
    }
    fprintf(stderr, "unknown scenario %s\n", argv[1]);
    return 2;
}
