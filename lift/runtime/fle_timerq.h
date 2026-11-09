/* Timer-queue timers (decision 0171, wall ia1-W3: kernel32!CreateTimerQueueTimer at a click on a menu, a channel or the playlist).
 *
 * A timer-queue timer runs its callback on a thread of the system's thread pool after DueTime ms and then every Period ms (Learn CreateTimerQueueTimer). Here every timer is one cooperative guest thread
 * (fle_thread_spawn, decision 0122) that sleeps on the virtual clock (fle_thread_sleep) and calls the guest callback through fle_call_guest; so the callback runs at the virtual time it is due, between two
 * waits of the other threads, exactly like the thread of a CreateThread. Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9), from the fifth on the guest stack
 * (fle_arg); the Win32 return value in R[0]; the guest return address stays on the stack for the caller's ret.
 *
 * kernel32!CreateTimerQueueTimer: BOOL f(PHANDLE phNewTimer rcx, HANDLE TimerQueue rdx, WAITORTIMERCALLBACK Callback r8, PVOID Parameter r9, DWORD DueTime, DWORD Period, ULONG Flags).
 * kernel32!ChangeTimerQueueTimer: BOOL f(HANDLE TimerQueue rcx, HANDLE Timer rdx, ULONG DueTime r8, ULONG Period r9).
 * kernel32!DeleteTimerQueueTimer: BOOL f(HANDLE TimerQueue rcx, HANDLE Timer rdx, HANDLE CompletionEvent r8).
 * Only the default timer queue (TimerQueue NULL) exists: no CreateTimerQueue is imported by the guest. */
#ifndef FLE_TIMERQ_H
#define FLE_TIMERQ_H

#include <stdint.h>

/* The handle of timer slot i is FLE_TIMERQ_HANDLE_BASE + 4 * (i + 1), never reused while the slot's thread is alive (a choice without a source: Learn gives no value for a handle). The region lies above the thread
 * handles (fle_thread.h, 0x7FFAA0000000) and below the stub rows of the FRUITY harness (0x7FFB00000000). */
#define FLE_TIMERQ_HANDLE_BASE UINT64_C(0x7FFAB0000000)
#define FLE_TIMERQ_MAX 64u

void fle_imp_CreateTimerQueueTimer(void);
void fle_imp_ChangeTimerQueueTimer(void);
void fle_imp_DeleteTimerQueueTimer(void);

/* Inspection for tests: the number of timers that are alive (created and neither deleted nor finished). */
unsigned fle_timerq_live(void);

#endif
