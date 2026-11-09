/* The guest's structured exception handling: kernel32!RaiseException and ntdll!RtlUnwindEx for lifted code (docs/decisions/0132-seh-s1.md, PROVISIONAL).
 * See fle_seh.h for the overview. The Microsoft pages that this file follows are saved in docs/investigations/seh2/learn/ (x64 exception handling, RaiseException,
 * RtlUnwindEx, EXCEPTION_RECORD, CONTEXT, RUNTIME_FUNCTION); what they leave open is a numbered S- choice in the decision, marked "S-seh-..." below. */
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_callback.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_mxcsr.h"
#include "fle_x87ctl.h"
#include "fle_realmod.h"
#include "fle_thread.h"
#include "fle_seh.h"

size_t trace_depth(void) __attribute__((weak));
void trace_set_depth(size_t d) __attribute__((weak));

/* ---- constants (winnt.h values; the pages name them without numbers except where noted in the decision: S-seh-flag-values) ---- */
#define EXC_NONCONTINUABLE 0x01u
#define EXC_UNWINDING      0x02u
#define EXC_EXIT_UNWIND    0x04u
#define EXC_TARGET_UNWIND  0x20u
#define EXC_MAXIMUM_PARAMETERS 15u
#define UNW_EHANDLER 1u
#define UNW_UHANDLER 2u
#define UNW_CHAININFO 4u
#define REC_SIZE 0x98u    /* EXCEPTION_RECORD on x64: 4+4+8+8+4(+4)+15*8 */
#define CTX_SIZE 0x4D0u   /* CONTEXT on x64 */
#define DC_SIZE  0x50u    /* DISPATCHER_CONTEXT as the guest handler reads it: ControlPc .. ScopeIndex */
#define FRAME_LIMIT 100000u

typedef struct {
    uint64_t g[16];
    uint64_t rip;
    fle_xmm x[16];
} uctx;

typedef struct {
    uint64_t control_pc, image_base, rf_va, func, establisher, entry_sp, handler_va, data_va;
    uint32_t flags;
    int has_rf;
} frame_t;

struct fle_seh_disp {
    uctx raise;            /* the context of the caller of RaiseException (S-seh-raise-context) */
    uint64_t rec;          /* guest address of the EXCEPTION_RECORD */
    unsigned thread;
    int pass;              /* 1 = searching, 2 = unwinding */
    uint32_t code, flags, nargs;
    uint64_t ret_addr;
    unsigned frames1, calls2;
};

static unsigned stat_frames, stat_calls2;

unsigned fle_seh_stat_frames(void) { return stat_frames; }
unsigned fle_seh_stat_unwind_calls(void) { return stat_calls2; }

static uint8_t ld8(uint64_t a) { return (uint8_t)fle_ld(a, 1); }
static uint16_t ld16(uint64_t a) { return (uint16_t)fle_ld(a, 2); }
static uint32_t ld32(uint64_t a) { return (uint32_t)fle_ld(a, 4); }
static uint64_t ld64(uint64_t a) { return fle_ld(a, 8); }

/* ---- the images (their headers are read from memory: the harness maps every image at its base) ---- */
typedef struct {
    uint64_t base, size, pdata;
    uint32_t npdata;
} img_t;
#define IMG_MAX 12
static img_t imgs[IMG_MAX];
static int nimgs = -1;

static int read_img(uint64_t base, img_t *im)
{
    uint64_t pe, opt;
    uint32_t prva, psz;

    if (ld16(base) != 0x5a4d) return 0;
    pe = base + ld32(base + 0x3c);
    if (ld32(pe) != 0x4550) return 0;
    opt = pe + 24;
    if (ld16(opt) != 0x20b) return 0;
    im->base = base;
    im->size = ld32(opt + 56);
    prva = ld32(opt + 112 + 3 * 8);
    psz = ld32(opt + 112 + 3 * 8 + 4);
    im->pdata = prva ? base + prva : 0;
    im->npdata = psz / 12u;
    return 1;
}

static void load_images(void)
{
    img_t im;

    nimgs = 0;
    if (read_img(FLE_EXE_HANDLE, &im)) imgs[nimgs++] = im;
    if (fle_realmod_total && fle_realmod_base_of) {
        uint64_t n = fle_realmod_total(), k;

        for (k = 0; k < n && nimgs < IMG_MAX; k++) {
            uint64_t b = fle_realmod_base_of((int)k);

            if (b && read_img(b, &im)) imgs[nimgs++] = im;
        }
    }
}

/* For tests that map an image of their own: replaces the list (bases of mapped PE images). */
void fle_seh_set_images(const uint64_t *bases, unsigned n);
void fle_seh_set_images(const uint64_t *bases, unsigned n)
{
    unsigned i;
    img_t im;

    nimgs = 0;
    for (i = 0; i < n && nimgs < IMG_MAX; i++)
        if (read_img(bases[i], &im)) imgs[nimgs++] = im;
}

static const img_t *img_of(uint64_t pc)
{
    int i;

    if (nimgs < 0) load_images();
    for (i = 0; i < nimgs; i++)
        if (pc >= imgs[i].base && pc - imgs[i].base < imgs[i].size) return &imgs[i];
    return NULL;
}

/* The RUNTIME_FUNCTION that holds the RVA (binary search of the sorted table), or 0. */
static uint64_t find_rf(const img_t *im, uint32_t rva)
{
    uint32_t lo = 0, hi = im->npdata;

    while (lo < hi) {
        uint32_t mid = lo + (hi - lo) / 2;
        uint64_t e = im->pdata + 12ull * mid;
        uint32_t b = ld32(e), en = ld32(e + 4);

        if (rva < b) hi = mid;
        else if (rva >= en) lo = mid + 1;
        else return e;
    }
    return 0;
}

/* ---- the virtual unwind of one frame (Learn: x64 exception handling, "Unwind procedure" and "UNWIND_CODE") ---- */
static uint32_t ia2_last_code;      /* decision 0191 (diagnostics only): the code and the argument pointer of the RaiseException being dispatched, to name a delay-load failure in the stop below */
static uint64_t ia2_last_args;
static void delay_names(uint64_t args, char *dll, size_t dn, char *proc, size_t pn);
static void stop_at(const char *api, const char *what, uint64_t pc)
{
    char why[420];
    size_t used;

    snprintf(why, sizeof why, "%s (ControlPc %#llx)", what, (unsigned long long)pc);
    used = strlen(why);
    if ((ia2_last_code == 0xC06D007Fu || ia2_last_code == 0xC06D007Eu) && ia2_last_args != 0) {
        char dll[64], proc[96];
        delay_names(ia2_last_args, dll, sizeof dll, proc, sizeof proc);
        snprintf(why + used, sizeof why - used, " [delay-load failure %#x: dll %.60s proc %.90s]", (unsigned)ia2_last_code, dll, proc);
    }
    fle_stop(api, why);
}

/* Unwinds the frame whose context is *c (c->rip = its ControlPc) into its caller's context and describes the frame. `want` is UNW_EHANDLER or UNW_UHANDLER. */
static void unwind_one(uctx *c, frame_t *f, uint32_t want, const char *api)
{
    uint64_t pc = c->rip, rsp = c->g[4], rfa, ui, tail;
    const img_t *im;
    uint32_t begin, off;
    int first = 1;
    unsigned guard = 0;
    unsigned regf = 0;
    uint64_t foff = 0;

    memset(f, 0, sizeof *f);
    f->control_pc = pc;
    im = img_of(pc);
    if (!im) {
        if (pc == FLE_CB_RETURN_ADDRESS)
            stop_at(api, "the exception reaches the return address of a function that the runtime called (fle_call_guest, a host callback): an exception that crosses a host callback is not modelled (decision 0132)", pc);
        stop_at(api, "ControlPc is in no mapped image, so no unwind data exists for it", pc);
    }
    f->image_base = im->base;
    rfa = find_rf(im, (uint32_t)(pc - im->base));
    if (!rfa) {
        /* Learn: "If the search doesn't find a function table entry, the code is assumed to be part of a leaf function, and RSP directly addresses the return pointer." */
        f->establisher = rsp;
        f->entry_sp = rsp;
        c->rip = ld64(rsp);
        c->g[4] = rsp + 8;
        return;
    }
    f->has_rf = 1;
    f->rf_va = rfa;
    begin = ld32(rfa);
    f->func = im->base + begin;
    off = (uint32_t)(pc - im->base) - begin;
    ui = im->base + (ld32(rfa + 8) & ~1u);
    for (;;) {
        uint8_t b0 = ld8(ui), prolog = ld8(ui + 1), ncodes = ld8(ui + 2), fr = ld8(ui + 3);
        uint32_t fl = (uint32_t)(b0 >> 3);
        uint64_t codes = ui + 4, base_save;
        unsigned i;

        if ((b0 & 7) != 1) stop_at(api, "unwind info version is not 1 (the EPILOG codes of version 2 are not modelled)", pc);
        if (first) {
            regf = fr & 15u;
            foff = 16ull * (fr >> 4);
            if (off <= prolog) stop_at(api, "ControlPc is in the prolog of its function (a return address inside a prolog: not modelled)", pc);
            f->flags = fl;
            f->establisher = regf ? c->g[regf] - foff : rsp;
        }
        for (i = 0; i < ncodes; i++) {
            uint16_t slot = ld16(codes + 2ull * i);
            unsigned op = (slot >> 8) & 15u, info = slot >> 12;
            uint64_t v;

            base_save = regf ? c->g[regf] - foff : rsp;
            switch (op) {
            case 0: /* UWOP_PUSH_NONVOL */
                c->g[info] = ld64(rsp);
                rsp += 8;
                break;
            case 1: /* UWOP_ALLOC_LARGE */
                if (info == 0) {
                    v = 8ull * ld16(codes + 2ull * (i + 1));
                    i += 1;
                } else {
                    v = (uint64_t)ld16(codes + 2ull * (i + 1)) | ((uint64_t)ld16(codes + 2ull * (i + 2)) << 16);
                    i += 2;
                }
                rsp += v;
                break;
            case 2: /* UWOP_ALLOC_SMALL */
                rsp += 8ull * info + 8;
                break;
            case 3: /* UWOP_SET_FPREG */
                if (!regf) stop_at(api, "UWOP_SET_FPREG in an unwind info with no frame register", pc);
                rsp = c->g[regf] - foff;
                break;
            case 4: /* UWOP_SAVE_NONVOL */
                v = 8ull * ld16(codes + 2ull * (i + 1));
                i += 1;
                c->g[info] = ld64(base_save + v);
                break;
            case 5: /* UWOP_SAVE_NONVOL_FAR */
                v = (uint64_t)ld16(codes + 2ull * (i + 1)) | ((uint64_t)ld16(codes + 2ull * (i + 2)) << 16);
                i += 2;
                c->g[info] = ld64(base_save + v);
                break;
            case 8: /* UWOP_SAVE_XMM128 */
                v = 16ull * ld16(codes + 2ull * (i + 1));
                i += 1;
                c->x[info] = fle_ldx(base_save + v);
                break;
            case 9: /* UWOP_SAVE_XMM128_FAR */
                v = (uint64_t)ld16(codes + 2ull * (i + 1)) | ((uint64_t)ld16(codes + 2ull * (i + 2)) << 16);
                i += 2;
                c->x[info] = fle_ldx(base_save + v);
                break;
            default:
                stop_at(api, "an unwind code that is not modelled (UWOP_PUSH_MACHFRAME or an EPILOG code)", pc);
            }
        }
        tail = ui + 4 + 2ull * ((ncodes + 1u) & ~1u);
        if (fl & UNW_CHAININFO) {
            if (++guard > 64) stop_at(api, "unwind info chain longer than 64", pc);
            ui = im->base + (ld32(tail + 8) & ~1u);
            first = 0;
            continue;
        }
        if (fl & (UNW_EHANDLER | UNW_UHANDLER)) {
            if (first) f->flags = fl;
            f->flags = fl;
            if (fl & want) {
                f->handler_va = im->base + ld32(tail);
                f->data_va = tail + 4;
            }
        }
        break;
    }
    f->entry_sp = rsp;
    c->rip = ld64(rsp);
    if (rsp + 8 <= c->g[4]) stop_at(api, "the virtual unwind did not move the stack pointer up", pc);
    c->g[4] = rsp + 8;
}

/* ---- the structures the guest handler reads ---- */
static void ctx_to_guest(uint64_t a, const uctx *c)
{
    unsigned i;

    memset((void *)fle_addr(a, CTX_SIZE, 1), 0, CTX_SIZE);
    fle_st(a + 0x30, 4, 0x0010000Bu);              /* S-seh-context-flags: CONTEXT_FULL (AMD64 | CONTROL | INTEGER | FLOATING_POINT) */
    fle_st(a + 0x34, 4, fle_mxcsr);                /* S-seh-context-fp-state */
    fle_st(a + 0x38, 2, 0x33);                     /* S-seh-context-segments: user-mode CS / SS values of x64 Windows */
    fle_st(a + 0x42, 2, 0x2b);
    fle_st(a + 0x44, 4, 0x202);                    /* S-seh-context-eflags */
    for (i = 0; i < 16; i++) fle_st(a + 0x78 + 8ull * i, 8, c->g[i]);
    fle_st(a + 0xf8, 8, c->rip);
    fle_st(a + 0x100, 2, fle_x87.cw);              /* FltSave.ControlWord */
    fle_st(a + 0x118, 4, fle_mxcsr);               /* FltSave.MxCsr */
    for (i = 0; i < 16; i++) {
        const fle_xmm *x = (i >= 6) ? &c->x[i] : &X[i];

        memcpy((void *)fle_addr(a + 0x1a0 + 16ull * i, 16, 1), x, 16);
    }
}

static void make_dc(uint64_t dc, const frame_t *f, uint64_t target_ip, uint64_t ctx)
{
    memset((void *)fle_addr(dc, DC_SIZE, 1), 0, DC_SIZE);
    fle_st(dc + 0x00, 8, f->control_pc);
    fle_st(dc + 0x08, 8, f->image_base);
    fle_st(dc + 0x10, 8, f->rf_va);
    fle_st(dc + 0x18, 8, f->establisher);
    fle_st(dc + 0x20, 8, target_ip);
    fle_st(dc + 0x28, 8, ctx);
    fle_st(dc + 0x30, 8, f->handler_va);
    fle_st(dc + 0x38, 8, f->data_va);
    fle_st(dc + 0x40, 8, 0);                       /* S-seh-history-table: NULL */
    fle_st(dc + 0x48, 4, 0);                       /* ScopeIndex */
}

static unsigned cur_thread(void) { return fle_thread_current ? fle_thread_current() : 0; }

/* ---- registrations (the wrapper of fe-lift's output) ---- */
void fle_seh_enter(fle_seh_reg *r, uint64_t func)
{
    r->prev = fle_seh_top;
    r->func = func;
    r->entry_sp = R[4];
    r->landing = fle_landing_top;
    r->cbdepth = fle_callback_depth();
    r->tdepth = trace_depth ? trace_depth() : 0;
    r->thread = cur_thread();
    r->land_ip = 0;
    fle_seh_top = r;
}

static struct {
    int valid;
    uctx c;
    uint64_t rax;
} pend;

void fle_seh_restore(fle_seh_reg *r)
{
    unsigned i;

    (void)r;
    if (!pend.valid) fle_stop("RtlUnwindEx", "a landing was entered with no pending unwind (internal)");
    pend.valid = 0;
    for (i = 0; i < 16; i++) R[i] = pend.c.g[i];
    R[0] = pend.rax;
    for (i = 6; i < 16; i++) X[i] = pend.c.x[i];
}

/* ---- RaiseException (pass 1) ---- */
/* Decision 0171 (diagnostic only): the MSVC delay-load helper raises VcppException(ERROR_SEVERITY_ERROR, ERROR_PROC_NOT_FOUND = 0xC06D007F) / (..., ERROR_MOD_NOT_FOUND = 0xC06D007E) with ExceptionInformation[0] =
 * a DelayLoadInfo: szDll at +0x18, dlp.szProcName at +0x28 (delayimp.h, 64-bit layout). Names them so that the missing import is found. */
static void delay_names(uint64_t args, char *dll, size_t dn, char *proc, size_t pn)
{
    uint64_t info = ld64(args), pd = info ? ld64(info + 0x18) : 0, pp = info ? ld64(info + 0x28) : 0;
    size_t q;

    snprintf(dll, dn, "?");
    snprintf(proc, pn, "?");
    for (q = 0; pd && q + 1 < dn; q++) { dll[q] = (char)fle_ld(pd + q, 1); dll[q + 1] = 0; if (!dll[q]) break; }
    for (q = 0; pp && q + 1 < pn; q++) { proc[q] = (char)fle_ld(pp + q, 1); proc[q + 1] = 0; if (!proc[q]) break; }
}

void fle_seh_raise(void)
{
    static const char api[] = "RaiseException";
    struct fle_seh_disp d;
    uint8_t recbuf[REC_SIZE + 16], ctxbuf[CTX_SIZE + 16], dcbuf[DC_SIZE + 16];
    uint64_t rec = ((uint64_t)(uintptr_t)recbuf + 15u) & ~UINT64_C(15), ctx = ((uint64_t)(uintptr_t)ctxbuf + 15u) & ~UINT64_C(15),
             dc = ((uint64_t)(uintptr_t)dcbuf + 15u) & ~UINT64_C(15);
    uint32_t code = (uint32_t)R[1], flags = (uint32_t)R[2], n = (uint32_t)R[8];
    uint64_t args = R[9], k;
    uctx c;
    unsigned i, frames = 0;
    char why[520];

    if (fle_seh_cur) {
        snprintf(why, sizeof why, "exception code %#x raised while another exception is being dispatched on this thread (collided and nested exceptions are not modelled, decision 0132)", (unsigned)code);
        fle_stop(api, why);
    }
    if (args != 0 && n > EXC_MAXIMUM_PARAMETERS) {
        snprintf(why, sizeof why, "exception code %#x: %u arguments (the page: the value must not exceed EXCEPTION_MAXIMUM_PARAMETERS = 15)", (unsigned)code, (unsigned)n);
        fle_stop(api, why);
    }
    if (args == 0) n = 0;
    memset(&d, 0, sizeof d);
    for (i = 0; i < 16; i++) d.raise.g[i] = R[i];
    d.raise.rip = ld64(R[4]);
    d.raise.g[4] = R[4] + 8;
    for (i = 6; i < 16; i++) d.raise.x[i] = X[i];
    d.rec = rec;
    d.thread = cur_thread();
    d.pass = 1;
    d.code = code;
    d.flags = flags;
    d.nargs = n;
    d.ret_addr = d.raise.rip;

    /* EXCEPTION_RECORD (Learn: EXCEPTION_RECORD; RaiseException: "values specified ... can be retrieved ... GetExceptionInformation") */
    memset((void *)fle_addr(rec, REC_SIZE, 1), 0, REC_SIZE);
    fle_st(rec + 0x00, 4, code);                   /* the code as given (S-seh-record-code: bit 28 is not cleared) */
    fle_st(rec + 0x04, 4, flags & EXC_NONCONTINUABLE); /* S-seh-record-flags: EXCEPTION_NONCONTINUABLE only; EXCEPTION_SOFTWARE_ORIGINATE is not set */
    fle_st(rec + 0x08, 8, 0);
    fle_st(rec + 0x10, 8, d.ret_addr);             /* S-seh-exception-address: the guest's return address after the call of RaiseException */
    fle_st(rec + 0x18, 4, n);
    for (k = 0; k < n; k++) fle_st(rec + 0x20 + 8 * k, 8, ld64(args + 8 * k));

    fle_seh_cur = &d;
    ia2_last_code = (uint32_t)code;
    ia2_last_args = n >= 1 ? args : 0;
    c = d.raise;
    for (;;) {
        frame_t f;
        uctx next = c;

        if (++frames > FRAME_LIMIT) fle_stop(api, "more than 100000 frames searched (a loop in the virtual unwind)");
        stat_frames = frames;
        unwind_one(&next, &f, UNW_EHANDLER, api);
        if (f.handler_va) {
            uint64_t r;
            int32_t disp;

            ctx_to_guest(ctx, &c);
            make_dc(dc, &f, 0, ctx);
            r = fle_call_guest(f.handler_va, rec, f.establisher, ctx, dc);
            disp = (int32_t)(uint32_t)r;
            if (disp == 1) {
                /* ExceptionContinueSearch */
            } else if (disp == 0) {
                snprintf(why, sizeof why, "the handler of the frame at %#llx returned ExceptionContinueExecution (resuming after a handled exception is not modelled; decision 0132)", (unsigned long long)f.control_pc);
                fle_stop(api, why);
            } else {
                snprintf(why, sizeof why, "the handler of the frame at %#llx returned the disposition %d (nested and collided exceptions are not modelled)", (unsigned long long)f.control_pc, (int)disp);
                fle_stop(api, why);
            }
        }
        c = next;
        if (c.rip == 0) break;
    }
    snprintf(why, sizeof why, "exception code %#x, flags %#x, %u argument(s) raised; no frame-based handler took it (%u frames searched)", (unsigned)code, (unsigned)flags, (unsigned)n, frames);
    if ((code == 0xC06D007Fu || code == 0xC06D007Eu) && n >= 1) {
        char dll[64], proc[96];
        size_t used = strlen(why);

        delay_names(args, dll, sizeof dll, proc, sizeof proc);
        snprintf(why + used, sizeof why - used, " [delay-load: dll %.60s proc %.90s]", dll, proc);
    }
    fle_stop(api, why);
    abort();
}

/* ---- RtlUnwindEx (pass 2, then the continuation at TargetIp) ---- */
void fle_seh_unwind_ex(void)
{
    static const char api[] = "RtlUnwindEx";
    uint64_t target_frame = R[1], target_ip = R[2], rec_in = R[8], retval = R[9], ctx_in = fle_arg(5);
    struct fle_seh_disp *d = fle_seh_cur;
    uint8_t ctxbuf[CTX_SIZE + 16], dcbuf[DC_SIZE + 16];
    uint64_t ctx = ((uint64_t)(uintptr_t)ctxbuf + 15u) & ~UINT64_C(15), dc = ((uint64_t)(uintptr_t)dcbuf + 15u) & ~UINT64_C(15), rec;
    uctx c;
    unsigned frames = 0, calls = 0;
    char why[700];

    if (!d) fle_stop(api, "called with no exception being dispatched on this thread (decision 0132 models RtlUnwindEx only as the continuation of RaiseException)");
    if (d->thread != cur_thread()) fle_stop(api, "the exception being dispatched belongs to another guest thread (the unwind would cross a thread switch)");
    if (target_frame == 0) fle_stop(api, "TargetFrame is NULL (an exit unwind is not modelled)");
    (void)ctx_in;   /* the page: the context "stores context during the unwind operation": the dispatcher uses its own buffer (S-seh-unwind-context) */
    rec = rec_in ? rec_in : d->rec;
    fle_st(rec + 4, 4, ld32(rec + 4) | EXC_UNWINDING);   /* S-seh-unwind-flags */
    d->pass = 2;
    c = d->raise;
    for (;;) {
        frame_t f;
        uctx next = c;
        int is_target;

        if (++frames > FRAME_LIMIT) fle_stop(api, "more than 100000 frames unwound (a loop in the virtual unwind)");
        unwind_one(&next, &f, UNW_UHANDLER, api);
        is_target = f.has_rf && f.establisher == target_frame;
        if (f.has_rf && f.establisher > target_frame) {
            snprintf(why, sizeof why, "the unwind passed TargetFrame %#llx without meeting a frame with that EstablisherFrame (STATUS_INVALID_UNWIND_TARGET)", (unsigned long long)target_frame);
            fle_stop(api, why);
        }
        if (f.handler_va) {
            uint64_t r;
            int32_t disp;

            if (is_target) fle_st(rec + 4, 4, ld32(rec + 4) | EXC_TARGET_UNWIND);
            ctx_to_guest(ctx, &c);
            make_dc(dc, &f, target_ip, ctx);
            calls++;
            stat_calls2 = calls;
            r = fle_call_guest(f.handler_va, rec, f.establisher, ctx, dc);
            disp = (int32_t)(uint32_t)r;
            if (disp != 1) {
                snprintf(why, sizeof why, "the handler of the frame at %#llx returned the disposition %d during the unwind (collided unwinds are not modelled)", (unsigned long long)f.control_pc, (int)disp);
                fle_stop(api, why);
            }
        }
        if (is_target) {
            fle_seh_reg *reg;
            uint64_t rsp_check = f.entry_sp;

            for (reg = fle_seh_top; reg; reg = reg->prev)
                if (reg->func == f.func && reg->entry_sp == rsp_check) break;
            if (!reg) {
                size_t used;
                unsigned shown = 0;
                fle_seh_reg *q;

                snprintf(why, sizeof why,
                         "the target frame is the function at %#llx (return address at %#llx), which has no landing registration: it was not lifted with --seh-handler (its except landing %#llx cannot be entered)",
                         (unsigned long long)f.func, (unsigned long long)rsp_check, (unsigned long long)target_ip);
                /* Decision 0171 (diagnostic): the registrations that exist, newest first (function, entry_sp) */
                used = strlen(why);
                for (q = fle_seh_top; q && shown < 6 && used + 40 < sizeof why; q = q->prev, shown++)
                    used += (size_t)snprintf(why + used, sizeof why - used, " [reg %#llx sp %#llx]", (unsigned long long)q->func, (unsigned long long)q->entry_sp);
                fle_stop(api, why);
            }
            if (reg->thread != cur_thread()) fle_stop(api, "the registration of the target frame belongs to another guest thread (the jump would cross a thread switch)");
            pend.valid = 1;
            pend.c = c;
            pend.rax = retval;
            reg->land_ip = target_ip;
            fle_seh_top = reg;
            fle_landing_top = reg->landing;
            fle_callback_depth_set(reg->cbdepth);
            if (trace_set_depth) trace_set_depth(reg->tdepth);
            fle_seh_cur = NULL;
            _longjmp(reg->jb, 1);
        }
        c = next;
        if (c.rip == 0) fle_stop(api, "the unwind reached the end of the stack without meeting TargetFrame");
    }
}
