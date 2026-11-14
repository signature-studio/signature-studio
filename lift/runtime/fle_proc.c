/* Process and thread answers (docs/decisions/0020-process-and-time.md; the pages read are listed in
 * docs/investigations/import-proctime-spec.md). Single guest thread. A case the pages leave open stops the
 * run through fle_stop (fle_env.h); nothing is turned silently into a success. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <unistd.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_proc.h"
#include "fle_screen.h"
#include "fle_com.h"
#include "fle_thread.h"

/* Same rule as the other runtime files: the first caller owns the state, another thread stops the run. */
static long fle_proc_owner_tid;

static void fle_proc_check_thread(const char *api)
{
    long tid = fle_host_tid();

    if (fle_proc_owner_tid == 0) fle_proc_owner_tid = tid;
    if (tid != fle_proc_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

/* MSDN GetCurrentProcess: the pseudo handle is "currently (HANDLE)-1". */
void fle_imp_GetCurrentProcess(void)
{
    fle_proc_check_thread("GetCurrentProcess");
    R[0] = FLE_PSEUDO_PROCESS_HANDLE;
}

/* MSDN GetCurrentProcessId: DWORD; the page gives no value. FLE_EMULATED_PID is a setting of this
 * environment. The result does not depend on the host (no getpid). */
void fle_imp_GetCurrentProcessId(void)
{
    fle_proc_check_thread("GetCurrentProcessId");
    R[0] = FLE_EMULATED_PID;
}

/* MSDN GetCurrentThread: "a special constant"; the value is not on the page. -2 is an assumption. */
void fle_imp_GetCurrentThread(void)
{
    fle_proc_check_thread("GetCurrentThread");
    R[0] = FLE_PSEUDO_THREAD_HANDLE;
}

/* MSDN SetLastError: VOID; sets the calling thread's last-error code (any 32-bit value). */
void fle_imp_SetLastError(void)
{
    fle_proc_check_thread("SetLastError");
    fle_set_last_error((uint32_t)R[1]);
}

/* ---- Error mode ----
 * MSDN SetErrorMode: the values are 0, SEM_FAILCRITICALERRORS 1, SEM_NOGPFAULTERRORBOX 2,
 * SEM_NOALIGNMENTFAULTEXCEPT 4, SEM_NOOPENFILEERRORBOX 0x8000; the return value is the previous mode;
 * once SEM_NOALIGNMENTFAULTEXCEPT is set, clearing it is ignored (parameter text). Bits outside the four
 * values are not defined by the page: the run stops. The initial mode 0 is the page's "system default". */
static uint32_t fle_proc_error_mode;
static int fle_thread_error_mode_set;
static uint32_t fle_thread_error_mode;

void fle_imp_SetErrorMode(void)
{
    uint32_t mode = (uint32_t)R[1];
    uint32_t prev = fle_proc_error_mode;

    fle_proc_check_thread("SetErrorMode");
    if (mode & ~(FLE_SEM_FAILCRITICALERRORS | FLE_SEM_NOGPFAULTERRORBOX | FLE_SEM_NOALIGNMENTFAULTEXCEPT |
                 FLE_SEM_NOOPENFILEERRORBOX))
        fle_stop("SetErrorMode", "a bit that the page does not list");
    if (prev & FLE_SEM_NOALIGNMENTFAULTEXCEPT) mode |= FLE_SEM_NOALIGNMENTFAULTEXCEPT;
    fle_proc_error_mode = mode;
    R[0] = prev;
}

/* MSDN SetThreadErrorMode: values 0, 1, 2, 0x8000 (the page does not list 4); lpOldMode may be NULL and
 * gets the thread's previous mode; nonzero on success. "A thread inherits the error mode of the process":
 * until this thread sets a mode, the previous mode reported is the process mode (reading of that
 * sentence). */
void fle_imp_SetThreadErrorMode(void)
{
    uint32_t mode = (uint32_t)R[1];
    uint64_t pold = R[2];
    uint32_t prev;

    fle_proc_check_thread("SetThreadErrorMode");
    /* Decision 0126 (PROVISIONAL), wall 67 (S-threaderrormode-alignment): the guest's thread (a thread of FLEngine, 0x819110) passes SEM_NOALIGNMENTFAULTEXCEPT (4), a bit that the page of SetThreadErrorMode does not list
     * (it lists 0, SEM_FAILCRITICALERRORS 1, SEM_NOGPFAULTERRORBOX 2 and SEM_NOOPENFILEERRORBOX 0x8000) and for which it names no failure ("If the function fails, the return value is zero. To get extended error
     * information, call GetLastError": no code). The SetErrorMode page lists the bit and says that once it is set clearing it is ignored: the same bit is accepted here and kept in the thread's mode as any listed one
     * (the success value 1; the bit has no effect in this environment: no alignment fault exists in lifted code). Another bit still stops. */
    if (mode & ~(FLE_SEM_FAILCRITICALERRORS | FLE_SEM_NOGPFAULTERRORBOX | FLE_SEM_NOALIGNMENTFAULTEXCEPT | FLE_SEM_NOOPENFILEERRORBOX))
        fle_stop("SetThreadErrorMode", "a bit that neither the page of SetThreadErrorMode nor that of SetErrorMode lists");
    {
        /* Decision 0122: the thread error mode belongs to the running guest thread (the first thread keeps the statics above). */
        fle_thread_pstate *ps = fle_thread_pstate_of ? ({ int ok; fle_thread_pstate_of(FLE_PSEUDO_THREAD_HANDLE, &ok); }) : NULL;
        uint32_t *mode_p = ps ? &ps->error_mode : &fle_thread_error_mode;
        int *set_p = ps ? &ps->error_mode_set : &fle_thread_error_mode_set;

        prev = *set_p ? *mode_p : fle_proc_error_mode;
        if (pold) fle_st(pold, 4, prev);
        *mode_p = mode;
        *set_p = 1;
    }
    R[0] = 1;
}

/* ---- Priorities ----
 * Only the two pseudo handles exist in this environment (no OpenProcess or CreateThread answer), so any
 * other handle is outside what the run can have produced: stop. The page's failure conditions for
 * background mode name the codes 402/403 (process) and 400/401 (thread) in System Error Codes. A value that
 * the page does not list stops the run. */
static uint32_t fle_prio_class = FLE_NORMAL_PRIORITY_CLASS; /* the page: "no special scheduling needs" */
static int fle_proc_background;
static int32_t fle_thread_prio;                              /* the page: threads start at NORMAL (0) */

/* Decision 0234 (mt3): the scheduler (fle_thread.c) reads the level of the first thread here; the other threads keep theirs in fle_thread_pstate. */
int32_t fle_proc_first_thread_priority(void) { return fle_thread_prio; }
static int fle_thread_background;

void fle_imp_SetPriorityClass(void)
{
    uint64_t h = R[1];
    uint32_t cls = (uint32_t)R[2];

    fle_proc_check_thread("SetPriorityClass");
    if (h != FLE_PSEUDO_PROCESS_HANDLE) fle_stop("SetPriorityClass", "a handle that GetCurrentProcess did not return");
    switch (cls) {
    case FLE_PROCESS_MODE_BACKGROUND_BEGIN:
        if (fle_proc_background) {
            fle_set_last_error(FLE_ERR_PROCESS_MODE_ALREADY_BACKGROUND);
            R[0] = 0;
            return;
        }
        fle_proc_background = 1;
        break;
    case FLE_PROCESS_MODE_BACKGROUND_END:
        if (!fle_proc_background) {
            fle_set_last_error(FLE_ERR_PROCESS_MODE_NOT_BACKGROUND);
            R[0] = 0;
            return;
        }
        fle_proc_background = 0;
        break;
    case FLE_NORMAL_PRIORITY_CLASS:
    case FLE_IDLE_PRIORITY_CLASS:
    case FLE_HIGH_PRIORITY_CLASS:
    case FLE_REALTIME_PRIORITY_CLASS:
    case FLE_BELOW_NORMAL_PRIORITY_CLASS:
    case FLE_ABOVE_NORMAL_PRIORITY_CLASS:
        fle_prio_class = cls;
        break;
    default:
        fle_stop("SetPriorityClass", "a priority class that the page does not list");
    }
    R[0] = 1;
}

/* kernel32!GetThreadPriority: int f(HANDLE hThread rcx) (decision 0129, wall 10). Learn: "Retrieves the priority value for the specified thread"; "If the function succeeds, the return value is the thread's priority
 * level" (one of -15, -2, -1, 0, 1, 2, 15, and -7 to 6 for a REALTIME class); "If the function fails, the return value is THREAD_PRIORITY_ERROR_RETURN" (0x7FFFFFFF in winbase.h: UNOFFICIAL; the page names the
 * constant only) and the last error says why. The value is the one SetThreadPriority kept for the thread (decision 0122 kept it and never used it; decision 0234 (mt3)'s scheduler orders the ready threads by it): the start value is NORMAL (0), the page's "threads start at
 * NORMAL" of SetThreadPriority. The pseudo handle of GetCurrentThread names the running thread; a handle that CreateThread returned names that thread; a closed one fails with ERROR_INVALID_HANDLE (6, the choice
 * made for SetThreadPriority); any other handle value stops (as SetThreadPriority does). THREAD_MODE_BACKGROUND_* are not priority values (they are modes) and are not returned. */
void fle_imp_GetThreadPriority(void)
{
    uint64_t h = R[1];
    int ok = 1;
    fle_thread_pstate *ps = NULL;

    fle_proc_check_thread("GetThreadPriority");
    if (h != FLE_PSEUDO_THREAD_HANDLE && !(fle_thread_owns_handle && fle_thread_owns_handle(h)))
        fle_stop("GetThreadPriority", "a handle that GetCurrentThread or CreateThread did not return");
    if (fle_thread_pstate_of) ps = fle_thread_pstate_of(h, &ok);
    if (!ok) {
        fle_set_last_error(FLE_W32_ERR_INVALID_HANDLE);
        R[0] = 0x7FFFFFFFu;
        return;
    }
    R[0] = (uint32_t)(ps ? ps->prio : fle_thread_prio);
}

void fle_imp_SetThreadPriority(void)
{
    uint64_t h = R[1];
    int32_t p = (int32_t)(uint32_t)R[2];
    /* Decision 0122: a handle that CreateThread returned names that thread (the page: hThread is "a handle to the thread whose priority value is to be set"); the pseudo handle of
     * GetCurrentThread names the running thread. The first thread keeps the statics above. The priority is kept and the scheduler orders the ready threads by it (decision 0234 (mt3); decision 0122 had ignored it, a choice). */
    int ok = 1;
    fle_thread_pstate *ps = NULL;
    int32_t *prio_p = &fle_thread_prio;
    int *bg_p = &fle_thread_background;

    fle_proc_check_thread("SetThreadPriority");
    if (h != FLE_PSEUDO_THREAD_HANDLE && !(fle_thread_owns_handle && fle_thread_owns_handle(h)))
        fle_stop("SetThreadPriority", "a handle that GetCurrentThread or CreateThread did not return");
    if (fle_thread_pstate_of) ps = fle_thread_pstate_of(h, &ok);
    if (!ok) {
        fle_set_last_error(FLE_W32_ERR_INVALID_HANDLE); /* a thread handle that was closed: the page names no code; 6 as for the other handle kinds */
        R[0] = 0;
        return;
    }
    if (ps) {
        prio_p = &ps->prio;
        bg_p = &ps->background;
    }
    switch (p) {
    case FLE_THREAD_MODE_BACKGROUND_BEGIN:
        if (h != FLE_PSEUDO_THREAD_HANDLE) fle_stop("SetThreadPriority", "THREAD_MODE_BACKGROUND_BEGIN with a handle other than GetCurrentThread's: the page says it is for the calling thread only");
        if (*bg_p) {
            fle_set_last_error(FLE_ERR_THREAD_MODE_ALREADY_BACKGROUND);
            R[0] = 0;
            return;
        }
        *bg_p = 1;
        break;
    case FLE_THREAD_MODE_BACKGROUND_END:
        if (h != FLE_PSEUDO_THREAD_HANDLE) fle_stop("SetThreadPriority", "THREAD_MODE_BACKGROUND_END with a handle other than GetCurrentThread's: the page says it is for the calling thread only");
        if (!*bg_p) {
            fle_set_last_error(FLE_ERR_THREAD_MODE_NOT_BACKGROUND);
            R[0] = 0;
            return;
        }
        *bg_p = 0;
        break;
    case -15: case -2: case -1: case 0: case 1: case 2: case 15:
        *prio_p = p;
        if (fle_thread_note_priority) fle_thread_note_priority(h, p);
        break;
    case -7: case -6: case -5: case -4: case -3: case 3: case 4: case 5: case 6:
        /* the page: allowed "if the thread has the REALTIME_PRIORITY_CLASS base class" */
        if (fle_prio_class != FLE_REALTIME_PRIORITY_CLASS)
            fle_stop("SetThreadPriority", "a REALTIME-only priority value while the class is not REALTIME");
        *prio_p = p;
        if (fle_thread_note_priority) fle_thread_note_priority(h, p);
        break;
    default:
        fle_stop("SetThreadPriority", "a priority value that the page does not list");
    }
    R[0] = 1;
}

/* MSDN SwitchToThread: zero when "there are no other threads ready to execute". There is one thread, so
 * the answer is always 0. A loop that waits for another thread through this call would spin forever;
 * the limit below (a runtime choice, not from a page) turns that into a stop. */
#define FLE_SWITCH_LIMIT UINT64_C(1000000)
static uint64_t fle_switch_calls;

void fle_imp_SwitchToThread(void)
{
    fle_proc_check_thread("SwitchToThread");
    if (fle_thread_switch && fle_thread_count && fle_thread_count() > 1) {
        /* Decision 0122: another thread that is ready runs now and the return value is 1 (the page: nonzero when execution switched to another thread); with none ready it is 0. The
         * limit below counts the calls that found nobody to switch to. */
        if (fle_thread_switch()) {
            R[0] = 1;
            return;
        }
    }
    if (++fle_switch_calls > FLE_SWITCH_LIMIT)
        fle_stop("SwitchToThread", "called over 1,000,000 times with no other thread to switch to (a wait on a thread that does not exist)");
    R[0] = 0;
}

/* MSDN OutputDebugStringW: "If the application does not have a debugger and the system debugger is not
 * active, OutputDebugString does nothing." The run has no debugger. The string (optional, may be NULL) is
 * not read. */
void fle_imp_OutputDebugStringW(void)
{
    fle_proc_check_thread("OutputDebugStringW");
}

/* ---- Requests to end the process ----
 * Reaching one of these means the program asked to stop. Returning would let it run on after an exit
 * request, and a clean exit would hide that the run ended there, so the run is stopped with a message
 * that names the call and its arguments (decision 0020). */
void fle_imp_ExitProcess(void)
{
    char why[160];

    snprintf(why, sizeof why, "the program asked to end the process (exit code %u); stopped here, not exited", (unsigned)(uint32_t)R[1]);
    fle_stop("ExitProcess", why);
}

void fle_imp_TerminateProcess(void)
{
    char why[200];
    uint64_t h = R[1];

    if (h == FLE_PSEUDO_PROCESS_HANDLE) {
        snprintf(why, sizeof why, "the program asked to terminate itself (exit code %u); stopped here", (unsigned)(uint32_t)R[2]);
        fle_stop("TerminateProcess", why);
    }
    snprintf(why, sizeof why, "handle %#llx is not the current process (no other process exists here)", (unsigned long long)h);
    fle_stop("TerminateProcess", why);
}

/* ---- Exception, unwinding and debugger APIs that stop by name (decision 0078) ----
 * dsp_ippv2_x64.dll imports these for its fatal-error handlers (__report_gsfailure, __scrt_fastfail, the invalid-parameter
 * handler) and its C++ exception code. Reading its code, every call site is on one of those paths and none is on the
 * DLL_PROCESS_ATTACH line or on the line of an IPP function (the sites are listed in decision 0078). Producing a machine context
 * (RtlCaptureContext, RtlVirtualUnwind) or a function table entry (RtlLookupFunctionEntry) for lifted code needs a CONTEXT and unwind
 * data, which this runtime does not have (the register file R[] is not a CONTEXT). Reaching one therefore ends the run, naming the
 * call, as RaiseException does above. Nothing is returned for any of them. */
#define FLE_NAMED_STOP(api, why) \
    void fle_imp_##api(void) { fle_stop(#api, why); }

FLE_NAMED_STOP(RtlCaptureContext, "the machine context of lifted code is not modelled (reached only on a fatal-error path of the lifted DLL)")
FLE_NAMED_STOP(RtlLookupFunctionEntry, "no unwind data is modelled (reached only on a fatal-error or exception path of the lifted DLL)")
FLE_NAMED_STOP(RtlVirtualUnwind, "no unwind data is modelled (reached only on a fatal-error or exception path of the lifted DLL)")
/* Decision 0132: RtlUnwindEx is the continuation of RaiseException in fle_seh.c when that file is linked (weak reference: a program that does not link it keeps the
 * named stop of decision 0078, with its text). */
void fle_seh_unwind_ex(void) __attribute__((weak));
void fle_imp_RtlUnwindEx(void)
{
    if (fle_seh_unwind_ex) fle_seh_unwind_ex();
    fle_stop("RtlUnwindEx", "structured exception handling is not modelled (reached only when the lifted DLL unwinds a C++ exception)");
}
FLE_NAMED_STOP(RtlPcToFileHeader, "no image-from-address lookup is modelled (reached only on an exception path of the lifted DLL)")
FLE_NAMED_STOP(UnhandledExceptionFilter, "no exception is modelled (reached only on a fatal-error path of the lifted DLL)")
FLE_NAMED_STOP(SetUnhandledExceptionFilter, "no exception is modelled (reached only on a fatal-error path of the lifted DLL)")

/* kernel32!IsDebuggerPresent: BOOL f(void) (decision 0126, PROVISIONAL, wall 99; it was a named stop of decision 0078 that assumed a fatal-error path). Learn: "If the current process is running in the context of a debugger, the return
 * value is nonzero. If the current process is not running in the context of a debugger, the return value is zero." This virtual process has no debugger (nothing attaches to it: S-no-debugger, the definition of the environment, not a
 * measurement of a real machine): 0. The guest's use at 0x526700 is the thread-naming idiom (Learn "How to: Set a Thread Name in Native Code": the exception 0x406D1388 is raised "if the debugger is present"): with 0 the guest does
 * not raise it. */
void fle_imp_IsDebuggerPresent(void)
{
    R[0] = 0;
}

/* kernel32!SetHandleCount (MSDN: SetHandleCount, decision 0102 W10, provisional): "Changes the number of file handles available to a process.
 * ... For Windows Win32 systems, this API has no effect"; the return value is "the number of available file handles". The page does not say
 * what number that is when the call has no effect; the requested count is returned (the caller's own request stays available). The
 * start-up of a statically linked MSVC CRT calls it once, at the end of _ioinit, and does not use the result. */
void fle_imp_SetHandleCount(void)
{
    R[0] = (uint32_t)R[1];
}

/* kernel32!EncodePointer / DecodePointer (MSDN: EncodePointer, DecodePointer; decision 0102 W8, provisional; EncodePointer was a named
 * stop in decision 0078, whose reading was that only the C++ exception code of dsp_ippv2_x64.dll reaches it; the start-up of a statically
 * linked MSVC CRT in freetype_x64.dll reaches it, and DecodePointer after it). The EncodePointer page (archived, v=vs.85): "Encodes
 * the specified pointer", "The function returns the encoded pointer", it "obfuscates the pointer value with a secret ... different for
 * each process", and "A pointer must be decoded before it can be used" (DecodePointer is the inverse; its page was not fetched). The
 * transform is a bijection on 64 bits with a fixed process secret: rotate-right by (secret & 63) of (p ^ secret). The page gives no
 * value and no relation to NULL (NULL does not encode to NULL here, as the page makes no promise); the secret below is this runtime's
 * own constant, so the values are deterministic from run to run and are not Windows's. */
#define FLE_POINTER_COOKIE UINT64_C(0x5A3C96E1D2B478F0)

static uint64_t ror64(uint64_t v, unsigned n)
{
    n &= 63u;
    return n ? (v >> n) | (v << (64u - n)) : v;
}

void fle_imp_EncodePointer(void)
{
    R[0] = ror64(R[1] ^ FLE_POINTER_COOKIE, (unsigned)(FLE_POINTER_COOKIE & 63u));
}

void fle_imp_DecodePointer(void)
{
    unsigned n = (unsigned)(FLE_POINTER_COOKIE & 63u);
    uint64_t v = R[1];
    v = n ? (v << n) | (v >> (64u - n)) : v;
    R[0] = v ^ FLE_POINTER_COOKIE;
}

/* MSDN RaiseException: with no frame-based handler the default action "is to call the ExitProcess
 * function". Structured exception handling is not modelled in the lifted code, so no handler can be
 * found and the request is treated like ExitProcess: stop, naming the code, flags and argument count. */
/* Decision 0132: with fle_seh.c linked (weak reference) RaiseException dispatches the exception to the guest's frame-based handlers (the named stop below is what a
 * program without fle_seh.c does, and what decision 0020 specified). */
void fle_seh_raise(void) __attribute__((weak));

void fle_imp_RaiseException(void)
{
    char why[220];

    if (fle_seh_raise) fle_seh_raise();

    snprintf(why, sizeof why,
             "exception code %#x, flags %#x, %u argument(s) raised; no structured exception handling is modelled, stopped here",
             (unsigned)(uint32_t)R[1], (unsigned)(uint32_t)R[2], (unsigned)(uint32_t)R[8]);
    fle_stop("RaiseException", why);
}

/* shcore!GetProcessDpiAwareness (decision 0099, provisional): HRESULT GetProcessDpiAwareness(HANDLE hprocess, PROCESS_DPI_AWARENESS *value).
 * Learn (shellscalingapi.h, Windows 8.1 and later, Shcore.dll): hprocess "NULL: the current process is queried"; S_OK, E_INVALIDARG ("the
 * handle or pointer passed in is not valid"), E_ACCESSDENIED. This environment has one process and cannot create another handle (no
 * OpenProcess), so NULL and the GetCurrentProcess pseudo handle are the current process and any other handle is not valid. The awareness of
 * the current process is a setting of this environment, not a rule of the page: PROCESS_DPI_UNAWARE (0), because nothing says that the host
 * process declares DPI awareness and the virtual screen is 96 DPI on one monitor (decision 0053). The delay-loaded guest asks only for
 * "is it per-monitor aware" (== 2), so 0 sends it down the non-per-monitor path. rcx, rdx. */
void fle_imp_GetProcessDpiAwareness(void)
{
    uint64_t h = R[1], pv = R[2];

    fle_proc_check_thread("GetProcessDpiAwareness");
    if ((h != 0 && h != FLE_PSEUDO_PROCESS_HANDLE) || pv == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    fle_st(pv, 4, FLE_PROCESS_DPI_UNAWARE);
    R[0] = FLE_HR_S_OK;
}

/* rpcrt4!UuidCreate (decision 0118 wall 7, provisional): RPC_STATUS UuidCreate(UUID *Uuid). Learn (UuidCreate): "The UuidCreate function creates a new UUID";
 * Uuid "Returns a pointer to the created UUID"; the return values are RPC_S_OK (the call succeeded), RPC_S_UUID_LOCAL_ONLY and RPC_S_UUID_NO_ADDRESS. The page
 * says that the UUID "cannot be traced to the ethernet address of the computer" and gives no algorithm and no version. A new UUID per call is the page's;
 * the values are a choice of this environment: no host randomness, clock or address is used (a run must give the same answers every time), so the UUIDs
 * are the version 4 form of RFC 4122 section 4.4 ("Set the two most significant bits (bits 6 and 7) of the clock_seq_hi_and_reserved to zero and one,
 * respectively. Set the four most significant bits (bits 12 through 15) of the time_hi_and_version field to the 4-bit version number [4]. Set all the other
 * bits to randomly (or pseudo-randomly) chosen values") filled from SplitMix64 (Steele, Lea, Flood, "Fast splittable pseudorandom number generators", OOPSLA
 * 2014; the constants are those of that generator) started from FLE_UUID_SEED. The UUID structure is the GUID structure of guiddef.h (Learn, GUID): Data1 (4
 * bytes), Data2 (2), Data3 (2), Data4 (8), little endian in memory on this platform, so the version nibble is the high nibble of byte 7 and the variant bits
 * are the top two bits of byte 8. A NULL pointer is not defined by the page: it stops. The result is RPC_S_OK (0): RPC_S_UUID_LOCAL_ONLY is for a UUID that
 * is unique to this computer only, which the page does not say of this one. */
static uint64_t fle_uuid_state = FLE_UUID_SEED;

static uint64_t fle_uuid_next(void)
{
    uint64_t z = (fle_uuid_state += UINT64_C(0x9E3779B97F4A7C15));

    z = (z ^ (z >> 30)) * UINT64_C(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)) * UINT64_C(0x94D049BB133111EB);
    return z ^ (z >> 31);
}

void fle_imp_UuidCreate(void)
{
    uint64_t p = R[1];
    uint8_t b[16];
    uint64_t lo, hi;
    unsigned i;

    fle_proc_check_thread("UuidCreate");
    if (p == 0) fle_stop("UuidCreate", "Uuid is NULL: the page does not define it (decision 0118)");
    lo = fle_uuid_next();
    hi = fle_uuid_next();
    for (i = 0; i < 8; i++) {
        b[i] = (uint8_t)(lo >> (8 * i));
        b[8 + i] = (uint8_t)(hi >> (8 * i));
    }
    b[7] = (uint8_t)((b[7] & 0x0Fu) | 0x40u);
    b[8] = (uint8_t)((b[8] & 0x3Fu) | 0x80u);
    for (i = 0; i < 16; i++) fle_st(p + i, 1, b[i]);
    R[0] = 0;
}
