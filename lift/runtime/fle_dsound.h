/* The DirectSound model of the virtual machine (decision 0139, PROVISIONAL; replaces decision 0126's "no device", S-ds-none). See fle_dsound.c. Same convention as fle_imports.h for the rows: arguments in R[1] (rcx), R[2] (rdx),
 * R[8] (r8), R[9] (r9), from the fifth on on the guest stack (fle_arg); the result (an HRESULT) in R[0]. */
#ifndef FLE_DSOUND_H
#define FLE_DSOUND_H

#include <stdint.h>

#define FLE_DS_OK 0u   /* DS_OK is S_OK (0); the Learn pages name the value DS_OK and the error codes, the number is the HRESULT S_OK */

/* The pseudo-VAs of the method slots of the COM objects (S-com-slot-range): FLE_COM_SLOT_BASE + FLE_COM_SLOT_STEP * method id. The range lies above the response table's rows (0x7FF900000010 + 0x10 * k, a few thousand) and
 * below the event handles (0x7FFA80000000), the Authenticode handles (0x7FFA90000000), the thread handles (0x7FFAA0000000) and the harness's stub rows (0x7FFB00000000). */
#define FLE_COM_SLOT_BASE UINT64_C(0x7FF940000000)
#define FLE_COM_SLOT_STEP UINT64_C(0x10)

/* dsound.dll!DirectSoundEnumerateA(LPDSENUMCALLBACKA rcx, LPVOID rdx), DirectSoundEnumerateW (the same with a LPDSENUMCALLBACKW), DirectSoundCreate(LPCGUID rcx, LPDIRECTSOUND *rdx, LPUNKNOWN r8),
 * DirectSoundCreate8(LPCGUID rcx, LPDIRECTSOUND8 *rdx, LPUNKNOWN r8): rows of the response table. */
void fle_imp_DirectSoundEnumerateA(void);
void fle_imp_DirectSoundEnumerateW(void);
void fle_imp_DirectSoundCreate(void);
void fle_imp_DirectSoundCreate8(void);

/* The method slot hook of fle_dispatch_miss (fle_dispatch.h): 1 and the method has run when `va` is a slot, else 0. */
int fle_com_try_slot(uint64_t va);

/* The device as the scheduler sees it (fle_thread.c; weak there): signals the notification events the play cursor has crossed (fle_audio_poll); the virtual time at which the next notification offset is reached, 0 results as "none" (fle_audio_next_tick). */
void fle_audio_poll(void);
int fle_audio_next_tick(uint64_t *out);

/* Counters for tests and for the report (no state moves). */
typedef struct {
    uint64_t enumerations, devices_created, secondary_created, locks, unlocks, unlock_bytes, plays, stops, getpos, notifications, late_frames, late_writes, late_write_applied, late_write_lost;
} dsound_stats;
const dsound_stats *fle_dsound_stats(void);

#endif
