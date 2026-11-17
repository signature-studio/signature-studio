/* Event objects for IAT slots reached by a tail jump (docs/decisions/0050-event-sync.md; the pages read are listed in
 * docs/investigations/event-sync-run.md). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8),
 * R[9] (r9); the Win32 return value in R[0]; the guest return address stays on the stack for the caller's ret.
 * Single guest thread only: an event is a state machine {manual-reset or auto-reset, signaled or not}, and a wait that
 * would have to block (the event is not signaled and the time-out is not 0) stops the run through fle_stop (fle_env.h),
 * because there is no other thread that could ever signal it and the virtual clock (fle_sys.h) is not moved by a wait. A
 * case the Microsoft pages leave open, and every case this runtime does not implement, stops the run too.
 * Threads, mutexes, semaphores, timers, named events, WaitForMultipleObjects and the alertable waits are not implemented. */
#ifndef FLE_EVENT_H
#define FLE_EVENT_H

#include <stdint.h>

/* Handles of event objects (decision 0050): FLE_EVENT_HANDLE_BASE + 4 * (i + 1) for the i-th event, never reused, in a
 * region of FLE_EVENT_REGION_SIZE bytes above the file-mapping handles (fle_vmap.h). The page gives no value for a
 * handle, so the value is a choice. */
#define FLE_EVENT_HANDLE_BASE UINT64_C(0x7FFA80000000)
#define FLE_EVENT_REGION_SIZE UINT64_C(0x10000000)

/* WaitForSingleObject results (Microsoft Learn, WaitForSingleObject, "Return value") and INFINITE (Learn: the
 * dwMilliseconds text; the number 0xFFFFFFFF is from windows-sys, docs.rs, unofficial). */
#define FLE_WAIT_OBJECT_0 0x00000000u
#define FLE_WAIT_TIMEOUT  0x00000102u
#define FLE_WAIT_FAILED   0xFFFFFFFFu
#define FLE_WAIT_INFINITE 0xFFFFFFFFu

/* kernel32!CreateEventW: HANDLE f(LPSECURITY_ATTRIBUTES, BOOL bManualReset, BOOL bInitialState, LPCWSTR lpName).
 * kernel32!SetEvent, ResetEvent: BOOL f(HANDLE). kernel32!WaitForSingleObject: DWORD f(HANDLE, DWORD dwMilliseconds). */
void fle_imp_CreateEventW(void);
void fle_imp_SetEvent(void);
void fle_imp_ResetEvent(void);
void fle_imp_WaitForSingleObject(void);

/* Decision 0122: -1 when `h` is not a live event handle, else its state (0 or 1); with consume != 0 a signaled auto-reset event becomes nonsignaled (the wait of a thread is satisfied by it).
 * fle_thread.c uses it to test the waits of the threads. */
int fle_event_wait_state(uint64_t h, int consume);

/* Decision 0139: the host-side SetEvent of the audio device (fle_event.c): 1 when `h` is a live event handle (it is signaled), else 0. */
int fle_event_host_set(uint64_t h);

/* Live event handles (for tests and tools), and the state of one handle: -1 when `h` is not a live event handle, 0
 * when it is not signaled, 1 when it is signaled. Neither moves any state. */
uint64_t fle_event_handle_count(void);
int fle_event_state(uint64_t h);

#endif
