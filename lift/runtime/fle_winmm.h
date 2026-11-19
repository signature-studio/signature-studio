/* winmm answers for a machine WITHOUT MIDI devices, and the multimedia-timer calls
 * (docs/decisions/0030-apis-a9-a10.md; the pages read are listed there). Same convention as fle_imports.h:
 * arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9), stack arguments through fle_arg (fle_env.h); the
 * Win32 return value in R[0]; the guest return address stays on the stack for the caller's ret.
 *
 * "No MIDI device" is a SETTING OF THIS LIFT ENVIRONMENT, not a fact about any real machine. The pages say a
 * count of zero "means that there are no devices (not that there is no error)". No audio or MIDI output exists:
 * no handle is ever created, so every handle is invalid. A case that the pages leave open stops the run through
 * fle_stop (fle_env.h): "fle: <api>: <why>" and abort(). Single guest thread only.
 *
 * timeGetTime is answered by the virtual clock in fle_sys.c (decision 0017). The timer-resolution calls here do
 * not touch that clock: it has no scheduler, Sleep(ms) adds exactly ms, so a requested resolution changes
 * nothing that the guest can observe except the bookkeeping that these functions return. */
#ifndef FLE_WINMM_H
#define FLE_WINMM_H

#include <stdint.h>

/* MMRESULT values. The Microsoft Learn pages name these codes but give no numbers; the numbers are from the
 * Windows SDK headers as mirrored by mingw-w64 (mmsyscom.h, mmeapi.h, timeapi.h) and the windows-sys constants
 * (both agree). Non-official sources. */
#define FLE_MMSYSERR_NOERROR      0u
#define FLE_MMSYSERR_BADDEVICEID  2u
#define FLE_MMSYSERR_INVALHANDLE  5u
#define FLE_MMSYSERR_NODRIVER     6u   /* decision 0126: the number is from the same headers (MMSYSERR_BASE 0 + 6) */
#define FLE_MIDIERR_NODEVICE      68u
#define FLE_TIMERR_NOERROR        0u
#define FLE_TIMERR_NOCANDO        97u

/* midiOutOpen fdwOpen callback type (the page lists the five names; the numbers are from the same headers). */
#define FLE_CALLBACK_TYPEMASK     0x00070000u
#define FLE_CALLBACK_NULL         0x00000000u
#define FLE_CALLBACK_WINDOW       0x00010000u
#define FLE_CALLBACK_THREAD       0x00020000u
#define FLE_CALLBACK_FUNCTION     0x00030000u
#define FLE_CALLBACK_EVENT        0x00050000u

/* MIDI_MAPPER is (UINT)-1. */
#define FLE_MIDI_MAPPER           0xFFFFFFFFu

/* Number of MIDI devices of this environment. */
#define FLE_WINMM_MIDI_IN_DEVS    0u
#define FLE_WINMM_MIDI_OUT_DEVS   0u

/* TIMECAPS (two UINT, 8 bytes): the resolution range that timeGetDevCaps reports and that timeBeginPeriod and
 * timeEndPeriod accept. SETTING OF THIS ENVIRONMENT: the minimum is 1 ms because the virtual clock counts in
 * whole milliseconds; the maximum 1000000 is a value commonly reported on real systems (unofficial, from
 * memory; the pages give none). */
#define FLE_WINMM_TIMECAPS_SIZE   8u
#define FLE_WINMM_PERIOD_MIN_MS   1u
#define FLE_WINMM_PERIOD_MAX_MS   1000000u

/* At most this many different periods may be outstanding at once (a limit of this runtime). */
#define FLE_WINMM_PERIOD_SLOTS    32u

/* Outstanding timeBeginPeriod requests (for tests and tools). The count of requests not yet ended, and the
 * lowest requested period (0 when none), which is the one the pages say Windows uses. */
uint32_t fle_winmm_period_outstanding(void);
uint32_t fle_winmm_period_lowest(void);

/* winmm!midiInGetNumDevs, midiOutGetNumDevs: UINT f(void). */
void fle_imp_midiInGetNumDevs(void);
void fle_imp_midiOutGetNumDevs(void);

/* winmm!midiInGetDevCapsW, midiOutGetDevCapsW: MMRESULT f(UINT_PTR uDeviceID, LPMIDIxxCAPSW, UINT cb).
 * rcx, rdx, r8d. */
void fle_imp_midiInGetDevCapsW(void);
void fle_imp_midiOutGetDevCapsW(void);

/* winmm!midiOutGetErrorTextW: MMRESULT f(MMRESULT, LPWSTR, UINT). ecx, rdx, r8d. */
void fle_imp_midiOutGetErrorTextW(void);

/* winmm!midiOutOpen: MMRESULT f(LPHMIDIOUT, UINT uDeviceID, DWORD_PTR cb, DWORD_PTR inst, DWORD fdwOpen).
 * rcx, edx, r8, r9, then fdwOpen on the stack. winmm!midiOutShortMsg: MMRESULT f(HMIDIOUT, DWORD). rcx, edx. */
void fle_imp_midiOutOpen(void);
void fle_imp_midiOutShortMsg(void);

/* winmm!timeBeginPeriod, timeEndPeriod: MMRESULT f(UINT). ecx. winmm!timeGetDevCaps: MMRESULT f(LPTIMECAPS, UINT).
 * rcx, edx. */
void fle_imp_timeBeginPeriod(void);
void fle_imp_timeEndPeriod(void);
void fle_imp_timeGetDevCaps(void);

/* msacm32!acmMetrics (decision 0098, PROVISIONAL): MMRESULT f(HACMOBJ hao, UINT uMetric, LPVOID pMetric). rcx, edx, r8.
 * Only (NULL, ACM_METRIC_MAX_SIZE_FORMAT, non-NULL) is answered: this environment has no ACM driver whose WAVEFORMATEX is larger than
 * the structure itself, so the largest WAVEFORMATEX is sizeof(WAVEFORMATEX) = 18 (Learn, WAVEFORMATEX: 3 x 2 + 2 x 4 + 2 x 2 = 18).
 * The metric number 50 is from the Windows SDK header as mirrored by mingw-w64 (msacm.h; the Learn page lists names only) and is what
 * the guest passes. Every other form stops by name. */
#define FLE_ACM_METRIC_MAX_SIZE_FORMAT 50u
#define FLE_ACM_WAVEFORMATEX_SIZE      18u
void fle_imp_acmMetrics(void);

#endif
