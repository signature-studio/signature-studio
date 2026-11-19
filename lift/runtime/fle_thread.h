/* Cooperative guest threads (docs/decisions/0122-cooperative-threads.md, PROVISIONAL; the static survey is docs/investigations/thread-model.md, option T2).
 *
 * The guest's threads run the guest's own instructions, one at a time, all on the one host thread (so fle_host_tid() and the 14 single-thread checks
 * of decision 0054 are unchanged). A thread is given up only inside one of the waiting or yielding APIs below; between two of them a thread runs
 * without interruption -- EXCEPT for the deterministic pseudo-preemption of decision 0147 (the block "decision 0147" below): after the first CreateThread, a slice ends
 * every FLE_PREEMPT_SLICE_DISPATCHES indirect calls of the guest and may give the processor to another ready thread. Which thread runs next is a CHOICE WITHOUT A SOURCE (decision 0122): round robin by creation order, priorities ignored; with fle_thread_set_priorities(1) (decision 0234 (mt3), Learn "Scheduling Priorities") the ready thread of the highest priority, the same priority in rotation. When
 * every thread waits, the virtual clock (fle_sys.h) is moved to the smallest deadline; when no thread can ever run again, the run stops by name.
 *
 * The FIRST thread (the one that called into the guest: the harness's) keeps the legacy single-thread storage (fle_tls_values, the last error of
 * fle_imports.c, the static priority state of fle_proc.c, the __thread locale of fle_locale.c, fle_teb): a program that never creates a thread behaves
 * exactly as before. Only a thread that CreateThread made has its own slots, which the accessors below hand out (NULL for the first thread).
 *
 * Same convention as fle_imports.h for the rows: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9), from the fifth on on the guest stack
 * (fle_arg); the Win32 return value in R[0]; the guest return address stays on the stack for the caller's ret.
 *
 * Native only: the stack switch is makecontext/swapcontext. The emscripten build compiles but stops by name at the first CreateThread (stage S3, JSPI,
 * is not built here). Every function below that other runtime files call is declared weak there (FLE_THREAD_REF): a program that links no fle_thread.c
 * has one thread, as before. */
#ifndef FLE_THREAD_H
#define FLE_THREAD_H

#include <stdint.h>
#include <stdio.h>

#ifdef FLE_THREAD_IMPL
#define FLE_THREAD_REF
#else
#define FLE_THREAD_REF __attribute__((weak))
#endif

/* Handle of the thread with table index i (i = 0 is the first thread, the one that called CreateFruityInstance): FLE_THREAD_HANDLE_BASE + 4 * (i + 1),
 * never reused (a choice without a source: Learn gives no value for a handle). The first thread's handle is never handed out (no import returns it:
 * OpenThread is not imported). The region lies above the Authenticode handles (0x7FFA90000000, fle_vfs.h) and below the stub rows of the FRUITY harness
 * (0x7FFB00000000). */
#define FLE_THREAD_HANDLE_BASE UINT64_C(0x7FFAA0000000)
#define FLE_THREAD_REGION_SIZE UINT64_C(0x10000000)
/* The most threads this table holds (a limit of this environment, not a Windows rule). The guest thread id of table index i is FLE_EMULATED_TID + i
 * (fle_imports.h, decision 0056). */
#define FLE_THREAD_MAX 256u

#define FLE_STILL_ACTIVE                      259u        /* Learn GetExitCodeThread: the constant STILL_ACTIVE (259 is its value in winbase.h) */
#define FLE_CREATE_SUSPENDED                  0x00000004u /* Learn CreateThread flags table */
#define FLE_STACK_SIZE_PARAM_IS_A_RESERVATION 0x00010000u /* Learn CreateThread flags table */
#define FLE_THREAD_DEFAULT_STACK              (UINT64_C(1) << 20) /* Learn CreateThread Remarks: "By default, every thread has one megabyte of stack space" */
#define FLE_THREAD_CSTACK_BYTES               (UINT64_C(16) << 20) /* the host C stack of a thread: a limit of this environment */
#define FLE_MAXIMUM_SUSPEND_COUNT             127u        /* the pages name the constant only; 127 is the winnt.h value (unofficial) */
#define FLE_MAXIMUM_WAIT_OBJECTS              64u         /* the pages name the constant only; 64 is the winnt.h value (unofficial) */
#define FLE_THREAD_TLS_SLOTS                  1088u       /* equals FLE_TLS_MAX_INDEXES of fle_imports.c (tests/wall_fr8.rs checks it) */
#define FLE_THREAD_FLS_SLOTS                  4096u       /* equals FLE_FLS_MAX_INDEXES of fle_imports.c */

/* Wait results (Learn WaitForSingleObject, WaitForMultipleObjects, MsgWaitForMultipleObjects). */
#define FLE_WAIT_TIMEOUT_RESULT 0x00000102u   /* the MsgWait page says "258L" = 0x102 */
#define FLE_WAIT_FAILED_RESULT  0xFFFFFFFFu

/* Queue status bits (Learn GetQueueStatus) and the message ids used here (Learn WM_QUIT: 0x0012 in winuser.h, the pages name the constant). */
#define FLE_QS_KEY            0x0001u
#define FLE_QS_MOUSEMOVE      0x0002u
#define FLE_QS_MOUSEBUTTON    0x0004u
#define FLE_QS_POSTMESSAGE    0x0008u
#define FLE_QS_TIMER          0x0010u
#define FLE_QS_PAINT          0x0020u
#define FLE_QS_SENDMESSAGE    0x0040u
#define FLE_QS_HOTKEY         0x0080u
#define FLE_QS_ALLPOSTMESSAGE 0x0100u
#define FLE_WM_QUIT           0x0012u
#define FLE_QUEUE_LIMIT       10000u      /* Learn PostMessage: "There is a limit of 10,000 posted messages per message queue" */
#define FLE_ERROR_NOT_ENOUGH_QUOTA   1816u /* System Error Codes 1700-3999: ERROR_NOT_ENOUGH_QUOTA 1816 (0x718) */
#define FLE_ERROR_INVALID_THREAD_ID  1444u /* System Error Codes 1300-1699: ERROR_INVALID_THREAD_ID 1444 (0x5A4) */

/* The virtual refresh rate of DwmFlush (decision 0122, with decision 0098's EnumDisplaySettingsW 60 Hz): frame k begins at ceil(k * 10^7 / HZ) ticks of 100 ns
 * since the virtual boot (the clock's zero, fle_sys.h). The 60 was A CHOICE WITHOUT A SOURCE (the Learn page does not say how long DwmFlush waits or what it waits for);
 * on a real Windows the guest runs at the monitor's refresh rate (decision 0166, ui1: the guest's UI cycle follows DwmFlush, measured 2.00x the cycles and the draws at 120).
 * ui1 (decision 0166, PROVISIONAL): FLE_DWM_HZ is the DEFAULT (60u); the environment variable FLE_DWM_HZ=<hertz>, which the manifest's
 * engine.env can carry like the other FLE_* variables, overrides it at run time (fle_display_hz(): read once, 24..1000, anything else is ignored with a line on stderr). The same
 * number is what EnumDisplaySettingsW reports (fle_monitor.c), so the two cannot disagree unless FLE_DISPLAYSETTINGS_FREQUENCY is also set. Unset: 60, the arithmetic and the results are those of decision 0122. */
#define FLE_DWM_HZ 60u
/* The effective refresh rate (hertz) of the virtual display: the environment variable FLE_DWM_HZ, else the FLE_DWM_HZ default above. */
uint32_t fle_display_hz(void) FLE_THREAD_REF;
/* The refresh rate EnumDisplaySettingsW reports (see fle_thread.c): FLE_DISPLAYSETTINGS_FREQUENCY in the environment, else fle_display_hz(). */
uint32_t fle_display_settings_hz(void) FLE_THREAD_REF;

/* ---- decision 0147 (PROVISIONAL): deterministic pseudo-preemption (an exception to decision 0122's "a thread is given up only inside one of the waiting or yielding APIs") ----
 * From the first CreateThread on, every FLE_PREEMPT_SLICE_DISPATCHES indirect calls of the guest (fle_progress_count of fle_dispatch.h; the count is the same for native, wasm, -O0 and -O2) end one slice: the virtual
 * clock is moved by FLE_PREEMPT_SLICE_TICKS, the waits whose time has come are tested and the next ready thread (creation order; with fle_thread_set_priorities(1) one of the same or a higher priority than the running one) runs; the preempted thread stays ready. A slice ends a thread's turn only while the
 * thread is not inside a callback of the runtime (fle_callback_depth() <= FLE_PREEMPT_MAX_DEPTH: the thread's own outermost guest call is depth 1, a window procedure called by DispatchMessageW is depth 2); inside a
 * callback the slice is skipped (the clock is not moved either), so a window procedure, EnumWindows callback or timer procedure still runs without interruption, as under decision 0122. Every value is a CHOICE
 * WITHOUT A SOURCE (S-vclock-*, decision 0147; FLE_PREEMPT_SLICE_TICKS was corrected by decision 0279). There is NO environment variable for them: FLE_PREEMPT, FLE_PREEMPT_DISPATCHES and FLE_PREEMPT_TICKS,
 * which decision 0147 named, do not exist in this runtime (nothing reads them; decision 0243 folded them into these constants). */
#define FLE_PREEMPT_SLICE_DISPATCHES 4096u   /* S-vclock-slice-progress */
#define FLE_PREEMPT_SLICE_TICKS      2500u   /* S-vclock-slice-time: 0.25 ms of 100 ns ticks, so 4096 indirect calls = 0.25 ms, about 61 ns each (decision 0279; decision 0147 chose 10000u = 1 ms = 244 ns, 2.13 times the 114 ns the lifted mixer measured per indirect call in decision 0275) */
#define FLE_PREEMPT_MAX_DEPTH        1u      /* S-vclock-depth */

typedef struct {
    int armed;                     /* the first CreateThread has happened and FLE_PREEMPT is not 0 */
    uint64_t slices;               /* slice ends that moved the clock (the thread was not inside a callback) */
    uint64_t deferred;             /* slice ends skipped because the thread was inside a callback */
    uint64_t switches;             /* slice ends that gave the processor to another thread */
    uint64_t ticks;                /* virtual ticks added by slices */
    uint64_t progress;             /* fle_progress_count now */
} fle_preempt_stats;

/* ---- the message queue of one thread (used by fle_win.c; Learn "About Messages and Message Queues") ---- */
typedef struct {
    uint64_t hwnd;                 /* 0 for a thread message (Learn MSG: "NULL when the message is a thread message") */
    uint32_t msg;
    uint64_t wp, lp;
    uint32_t time;                 /* the virtual clock in ms when it was posted */
    int32_t ptx, pty;              /* the cursor position when it was posted: (0, 0) (the virtual cursor has no position; a choice) */
} fle_tmsg;

typedef struct fle_sent {          /* a message sent by another thread, waiting for the receiving thread's retrieval call (Learn SendMessage) */
    uint64_t hwnd;
    uint32_t msg;
    uint64_t wp, lp;
    uint64_t result;
    int done;
    unsigned sender;               /* thread index of the blocked sender */
    struct fle_sent *next;
} fle_sent;

typedef struct {
    int exists;                    /* a thread has no queue until a call that needs one (Learn: made at the thread's first call of a user function) */
    fle_tmsg *q;                   /* the posted messages, first in first out */
    uint32_t head, count, cap;
    fle_sent *sent_head, *sent_tail;
    int quit;                      /* PostQuitMessage: WM_QUIT is not in the FIFO, it is delivered when no other posted message matches */
    int32_t quit_code;
    uint32_t changebits;           /* QS_ bits of the input added since the thread last looked (Learn GetQueueStatus low word / MsgWait "new input") */
} fle_tqueue;

/* The per-thread state of fle_proc.c: priority, background mode, thread error mode. */
typedef struct {
    int32_t prio;                  /* THREAD_PRIORITY_NORMAL (0) at creation (Learn CreateThread) */
    int background;
    uint32_t error_mode;
    int error_mode_set;
} fle_thread_pstate;

/* ---- the rows (appended to imports_table.rs) ---- */
/* kernel32!CreateThread: HANDLE f(LPSECURITY_ATTRIBUTES rcx, SIZE_T dwStackSize rdx, LPTHREAD_START_ROUTINE r8, LPVOID lpParameter r9, DWORD dwCreationFlags [5], LPDWORD lpThreadId [6]).
 * kernel32!ResumeThread, SuspendThread: DWORD f(HANDLE rcx). kernel32!ExitThread: VOID f(DWORD rcx), does not return. kernel32!GetExitCodeThread: BOOL f(HANDLE rcx, LPDWORD rdx).
 * kernel32!WaitForMultipleObjects: DWORD f(DWORD nCount ecx, const HANDLE *lpHandles rdx, BOOL bWaitAll r8d, DWORD dwMilliseconds r9d).
 * user32!MsgWaitForMultipleObjects: DWORD f(DWORD nCount ecx, const HANDLE *pHandles rdx, BOOL fWaitAll r8d, DWORD dwMilliseconds r9d, DWORD dwWakeMask [5]).
 * dwmapi!DwmFlush: HRESULT f(void) (dwmapi.dll is virtual module N = 39). */
void fle_imp_CreateThread(void);
void fle_imp_ResumeThread(void);
void fle_imp_SuspendThread(void);
void fle_imp_ExitThread(void) __attribute__((noreturn));
void fle_imp_GetExitCodeThread(void);
void fle_imp_WaitForMultipleObjects(void);
/* Decision 0126 (wall 66): kernel32!WaitForMultipleObjectsEx: as WaitForMultipleObjects (no APC or completion routine can be queued here: bAlertable changes nothing). */
void fle_imp_WaitForMultipleObjectsEx(void);
void fle_imp_MsgWaitForMultipleObjects(void);
void fle_imp_DwmFlush(void);

/* ---- what the other runtime files call ---- */
/* The index of the running thread (0 = the first thread; 0 too when no thread was ever created). */
unsigned fle_thread_current(void) FLE_THREAD_REF;
/* kernel32!Sleep: ms 0 gives the processor to another ready thread (returns at once when there is none); INFINITE (0xFFFFFFFF) never times out. */
void fle_thread_sleep(const char *api, uint32_t ms) FLE_THREAD_REF;
/* Decision 0171: a new thread that runs the C function body(arg) (see fle_thread.c); returns its handle, 0 on failure. */
uint64_t fle_thread_spawn(void (*body)(uint64_t), uint64_t arg) FLE_THREAD_REF;
/* kernel32!SwitchToThread: 1 when another thread ran, 0 when there was none ready. */
int fle_thread_switch(void) FLE_THREAD_REF;
/* Waits for the objects (events: fle_event.c; threads). ms is the time-out in ms (0xFFFFFFFF = INFINITE). `msg` != 0 adds the input of the thread's queue (MsgWait...,
 * `mask` = dwWakeMask). Returns the DWORD result (WAIT_OBJECT_0 + i, WAIT_OBJECT_0 + n for input, WAIT_TIMEOUT, WAIT_FAILED with the last error set). */
uint32_t fle_thread_wait_objects(const char *api, uint32_t n, const uint64_t *h, int all, uint32_t ms, int msg, uint32_t mask) FLE_THREAD_REF;
/* True when `h` lies in the thread handle region (an issued or a never issued value). */
int fle_thread_owns_handle(uint64_t h) FLE_THREAD_REF;
/* fle_sys.c: the running thread found the critical section at `addr` owned by another thread; returns with the section acquired. */
void fle_thread_wait_cs(const char *api, uint64_t addr) FLE_THREAD_REF;
/* Something that a waiting thread may wait for happened (an event was set, a section left, a message posted ...): the waiting threads are tested now, in the order they began to wait. */
void fle_thread_wake_check(void) FLE_THREAD_REF;
/* Slots of the running thread, NULL for the first thread (which keeps the legacy storage of its file). */
uint64_t *fle_thread_tls_values(void) FLE_THREAD_REF;
uint32_t *fle_thread_slot_last_error(void) FLE_THREAD_REF;
uint32_t *fle_thread_slot_lcid(void) FLE_THREAD_REF;
uint64_t *fle_thread_slot_fls(void) FLE_THREAD_REF;
/* The priority state of the thread that `handle` names: the pseudo handle of GetCurrentThread (0xFFFFFFFFFFFFFFFE) is the running thread; a handle that CreateThread returned is that
 * thread; ok = 0 for any other value. NULL (with ok = 1) for the first thread, which keeps the static state of fle_proc.c. */
fle_thread_pstate *fle_thread_pstate_of(uint64_t handle, int *ok) FLE_THREAD_REF;
/* Decision 0234 (mt3): SetThreadPriority tells the scheduler that the priority level of the thread that `handle` names is now `level` (the scheduler reads the levels at every choice of the next thread; this call only writes a line of the thread trace). */
void fle_thread_note_priority(uint64_t handle, int32_t level) FLE_THREAD_REF;
/* The message queue of thread idx (made when `create`); NULL when it has none. */
fle_tqueue *fle_thread_queue(unsigned idx, int create) FLE_THREAD_REF;
/* Appends a posted message (time and pt filled in) and wakes the owner when it waits; 0 when the queue is full (the caller sets ERROR_NOT_ENOUGH_QUOTA). */
int fle_thread_post(unsigned idx, uint64_t hwnd, uint32_t msg, uint64_t wp, uint64_t lp) FLE_THREAD_REF;
/* Index of the posted message that matches the filter (hwnd NULL: any; (uint64_t)-1: thread messages only; else that window; min = max = 0: any id), or -1. */
int fle_thread_queue_find(const fle_tqueue *q, uint64_t fhwnd, uint32_t fmin, uint32_t fmax) FLE_THREAD_REF;
/* Removes the posted message at queue position `pos` (from fle_thread_queue_find) and returns it. */
fle_tmsg fle_thread_queue_remove(fle_tqueue *q, int pos) FLE_THREAD_REF;
/* Blocks the running thread until its queue has something for it (kind: 1 = GetMessage with the filter, 2 = WaitMessage); a sent message wakes it too. Returns after one wake. */
void fle_thread_wait_message(const char *api, int kind, uint64_t fhwnd, uint32_t fmin, uint32_t fmax) FLE_THREAD_REF;
/* A sender waits for its sent message to be processed by thread `to` (cross-thread SendMessageW); it also wakes when a message is sent to it (so it can process it). */
void fle_thread_wait_sent(const char *api, fle_sent *s, unsigned to) FLE_THREAD_REF;
/* Number of threads in the table (0 until the first CreateThread: the first thread is entered then); the virtual id and the index of a thread (-1 when no thread has that id). */
unsigned fle_thread_count(void) FLE_THREAD_REF;
unsigned fle_thread_vid(unsigned idx) FLE_THREAD_REF;
int fle_thread_index_of_vid(uint32_t vid) FLE_THREAD_REF;
/* 1 when thread idx has finished. */
int fle_thread_is_done(unsigned idx) FLE_THREAD_REF;
/* Decision 0280, L1c (PROVISIONAL): 1 when thread idx waits in its message loop (GetMessage, WaitMessage, MsgWaitForMultipleObjects) */
int fle_thread_in_message_wait(unsigned idx) FLE_THREAD_REF;
/* The frame boundary DwmFlush waits for: the first 60 Hz boundary strictly after `now` (ticks of 100 ns). */
uint64_t fle_thread_next_frame_tick(uint64_t now) FLE_THREAD_REF;
/* TlsFree: the slot of every other thread is cleared (Learn TlsFree does not say; the index is gone for all threads). */
void fle_thread_tls_clear(uint32_t idx) FLE_THREAD_REF;
/* fle_win.c: a message sent to thread `to` (cross-thread SendMessageW) and the one the receiving thread takes next. */
void fle_thread_send(unsigned to, fle_sent *s) FLE_THREAD_REF;
fle_sent *fle_thread_dequeue_sent(unsigned idx) FLE_THREAD_REF;

/* The input-idle stop (decision 0122, stop condition (b)): when set, the scheduler stops by name at the first moment the first thread waits for input with an empty queue and no
 * other thread can run. Off by default; the FRUITY harness switches it on. */
void fle_thread_set_idle_stop(int on) FLE_THREAD_REF;
/* Decision 0147: the counters of the pseudo-preemption (all zero, armed 0, while no thread was ever created). */
void fle_thread_preempt_stats(fle_preempt_stats *out) FLE_THREAD_REF;
/* Prints the thread table, the queues and the virtual clock (the harness report). */
void fle_thread_report(FILE *f) FLE_THREAD_REF;
/* A trace of every switch on stderr (write(2)), for diagnosis. */
void fle_thread_set_trace(int on) FLE_THREAD_REF;
/* Decision 0234 (mt3, PROVISIONAL): 1 = the scheduler orders the ready threads by their scheduling priority (Learn "Scheduling Priorities": the levels SetThreadPriority kept), 0 (the default) = the rotation in creation order of decision 0122, the priorities are only kept. */
void fle_thread_set_priorities(int on) FLE_THREAD_REF;

#endif
