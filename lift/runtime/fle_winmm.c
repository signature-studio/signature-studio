/* winmm answers (docs/decisions/0030-apis-a9-a10.md): no MIDI device, multimedia-timer bookkeeping. Every rule is
 * from the Microsoft Learn page named in the comment; what a page leaves open stops the run through fle_stop. */
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_winmm.h"

/* ---- MIDI device counts ---- */
/* MSDN midiInGetNumDevs, midiOutGetNumDevs: "A return value of zero means that there are no devices (not that
 * there is no error)." */
void fle_imp_midiInGetNumDevs(void) { R[0] = FLE_WINMM_MIDI_IN_DEVS; }
void fle_imp_midiOutGetNumDevs(void) { R[0] = FLE_WINMM_MIDI_OUT_DEVS; }

/* MSDN midiInGetDevCapsW: the identifier "varies from zero to one less than the number of devices present";
 * MMSYSERR_BADDEVICEID is "The specified device identifier is out of range"; "If cbmic is zero, nothing is copied,
 * and the function returns MMSYSERR_NOERROR"; MMSYSERR_INVALPARAM is "The specified pointer or structure is
 * invalid". With no device every identifier is out of range. The pages do not say which error wins when several
 * apply (a zero size, a NULL pointer), so only a call with a non-NULL pointer and a non-zero size is answered.
 * The identifier may also be "a properly cast device handle"; no handle exists, but a value that does not fit
 * a 32-bit identifier could be one, so it stops. */
static void fle_winmm_devcaps(const char *api, int is_out)
{
    uint64_t id = R[1], pcaps = R[2];
    uint32_t cb = (uint32_t)R[8];

    if (pcaps == 0) fle_stop(api, "the caps pointer is NULL: the page does not say if INVALPARAM or BADDEVICEID wins");
    if (cb == 0) fle_stop(api, "cb is zero: the page says NOERROR and nothing copied, but not when the identifier is out of range");
    if (id > 0xFFFFFFFFu && id != UINT64_C(0xFFFFFFFFFFFFFFFF))
        fle_stop(api, "identifier above 32 bits: could be a device handle, none exists");
    if (is_out && (uint32_t)id == FLE_MIDI_MAPPER) {
        /* Decision 0126 (PROVISIONAL), wall 78 (S-midi-mapper-nodriver): the guest asks the capabilities of MIDI_MAPPER (the Microsoft GS Wavetable Synth of a real machine) although this environment has no MIDI output device.
         * The page of midiOutGetDevCapsW: "The MIDI_MAPPER constant is also a valid device identifier" and lists MMSYSERR_NODRIVER ("The driver is not installed.") among the errors; it does not say which error the mapper
         * gives when no device exists. A CHOICE WITHOUT A SOURCE: MMSYSERR_NODRIVER (6), the mapper being a driver that is not installed (and not BADDEVICEID, which the page defines for an out-of-range number: the
         * mapper is not a number in the range). The structure is not written. */
        R[0] = FLE_MMSYSERR_NODRIVER;
        return;
    }
    R[0] = FLE_MMSYSERR_BADDEVICEID;
}

void fle_imp_midiInGetDevCapsW(void) { fle_winmm_devcaps("midiInGetDevCapsW", 0); }
void fle_imp_midiOutGetDevCapsW(void) { fle_winmm_devcaps("midiOutGetDevCapsW", 1); }

/* MSDN midiOutGetErrorTextW: "If cchText is zero, nothing is copied, and the function returns
 * MMSYSERR_NOERROR." The text of an error is a system string that the page does not give, and the page does not
 * define the range for MMSYSERR_BADERRNUM, so every other call stops. */
void fle_imp_midiOutGetErrorTextW(void)
{
    uint32_t cch = (uint32_t)R[8];

    if (cch == 0) { R[0] = FLE_MMSYSERR_NOERROR; return; }
    fle_stop("midiOutGetErrorTextW", "the error text is a system string that the page does not give, and BADERRNUM has no defined range");
}

/* MSDN midiOutOpen: "MIDIERR_NODEVICE: No MIDI port was found. This error occurs only when the mapper is
 * opened." "MMSYSERR_BADDEVICEID: The specified device identifier is out of range." The identifier is a UINT;
 * MIDI_MAPPER is (UINT)-1. The five callback types are the only flag values the page lists; anything else, or a
 * NULL phmo, is not described. The handle location is left alone on failure (the page says it is filled with a
 * handle on success). */
void fle_imp_midiOutOpen(void)
{
    static const char *api = "midiOutOpen";
    uint64_t phmo = R[1];
    uint32_t id = (uint32_t)R[2], open = (uint32_t)fle_arg(5), type = open & FLE_CALLBACK_TYPEMASK;

    if (phmo == 0) fle_stop(api, "phmo is NULL (the page does not say what is returned)");
    if ((open & ~FLE_CALLBACK_TYPEMASK) != 0) fle_stop(api, "fdwOpen has a flag other than a callback type (the page lists only those)");
    if (type != FLE_CALLBACK_NULL && type != FLE_CALLBACK_WINDOW && type != FLE_CALLBACK_THREAD &&
        type != FLE_CALLBACK_FUNCTION && type != FLE_CALLBACK_EVENT)
        fle_stop(api, "fdwOpen callback type is not one of the five on the page");
    R[0] = id == FLE_MIDI_MAPPER ? FLE_MIDIERR_NODEVICE : FLE_MMSYSERR_BADDEVICEID;
}

/* MSDN midiOutShortMsg: "MMSYSERR_INVALHANDLE: The specified device handle is invalid." No device is ever
 * opened, so no handle is valid. */
void fle_imp_midiOutShortMsg(void) { R[0] = FLE_MMSYSERR_INVALHANDLE; }

/* ---- Multimedia timer resolution ---- */
static struct { uint32_t period, count; } fle_winmm_periods[FLE_WINMM_PERIOD_SLOTS];

uint32_t fle_winmm_period_outstanding(void)
{
    uint32_t i, n = 0;
    for (i = 0; i < FLE_WINMM_PERIOD_SLOTS; i++) n += fle_winmm_periods[i].count;
    return n;
}

uint32_t fle_winmm_period_lowest(void)
{
    uint32_t i, low = 0;
    for (i = 0; i < FLE_WINMM_PERIOD_SLOTS; i++)
        if (fle_winmm_periods[i].count != 0 && (low == 0 || fle_winmm_periods[i].period < low)) low = fle_winmm_periods[i].period;
    return low;
}

static int fle_winmm_in_range(uint32_t period) { return period >= FLE_WINMM_PERIOD_MIN_MS && period <= FLE_WINMM_PERIOD_MAX_MS; }

/* MSDN timeBeginPeriod: TIMERR_NOERROR, or TIMERR_NOCANDO "if the resolution specified in uPeriod is out of
 * range". The range is the one timeGetDevCaps reports (inference: the page does not define it). Each call must be
 * matched by a timeEndPeriod with the same value, and "an application can make multiple timeBeginPeriod calls as
 * long as each call is matched", so the calls are counted per value. */
void fle_imp_timeBeginPeriod(void)
{
    uint32_t period = (uint32_t)R[1], i, free_slot = FLE_WINMM_PERIOD_SLOTS;

    if (!fle_winmm_in_range(period)) { R[0] = FLE_TIMERR_NOCANDO; return; }
    for (i = 0; i < FLE_WINMM_PERIOD_SLOTS; i++) {
        if (fle_winmm_periods[i].count != 0 && fle_winmm_periods[i].period == period) {
            if (fle_winmm_periods[i].count == UINT32_MAX) fle_stop("timeBeginPeriod", "too many unmatched calls for one period");
            fle_winmm_periods[i].count++;
            R[0] = FLE_TIMERR_NOERROR;
            return;
        }
        if (fle_winmm_periods[i].count == 0 && free_slot == FLE_WINMM_PERIOD_SLOTS) free_slot = i;
    }
    if (free_slot == FLE_WINMM_PERIOD_SLOTS) fle_stop("timeBeginPeriod", "too many different periods outstanding (limit of this runtime)");
    fle_winmm_periods[free_slot].period = period;
    fle_winmm_periods[free_slot].count = 1;
    R[0] = FLE_TIMERR_NOERROR;
}

/* MSDN timeEndPeriod: TIMERR_NOERROR, or TIMERR_NOCANDO if uPeriod is out of range. A value in range that no
 * timeBeginPeriod call is waiting for is not described by the page, so it stops. */
void fle_imp_timeEndPeriod(void)
{
    uint32_t period = (uint32_t)R[1], i;

    if (!fle_winmm_in_range(period)) { R[0] = FLE_TIMERR_NOCANDO; return; }
    for (i = 0; i < FLE_WINMM_PERIOD_SLOTS; i++) {
        if (fle_winmm_periods[i].count != 0 && fle_winmm_periods[i].period == period) {
            fle_winmm_periods[i].count--;
            R[0] = FLE_TIMERR_NOERROR;
            return;
        }
    }
    fle_stop("timeEndPeriod", "no matching timeBeginPeriod for this period (the page does not say what is returned)");
}

/* MSDN timeGetDevCaps: fills a TIMECAPS (UINT wPeriodMin, UINT wPeriodMax); TIMERR_NOCANDO when ptc is NULL, "or
 * the cbtc parameter is invalid". Which sizes are invalid is not said; only the exact size of TIMECAPS (8) is
 * answered. */
void fle_imp_timeGetDevCaps(void)
{
    uint64_t ptc = R[1];
    uint32_t cb = (uint32_t)R[2];

    if (ptc == 0) { R[0] = FLE_TIMERR_NOCANDO; return; }
    if (cb != FLE_WINMM_TIMECAPS_SIZE) fle_stop("timeGetDevCaps", "cbtc is not sizeof(TIMECAPS) = 8: the page does not say which sizes are invalid");
    fle_st(ptc, 4, FLE_WINMM_PERIOD_MIN_MS);
    fle_st(ptc + 4, 4, FLE_WINMM_PERIOD_MAX_MS);
    R[0] = FLE_TIMERR_NOERROR;
}

/* ---- msacm32!acmMetrics (decision 0098, PROVISIONAL) ----
 * MMRESULT acmMetrics(HACMOBJ hao, UINT uMetric, LPVOID pMetric). Learn (acmMetrics): ACM_METRIC_MAX_SIZE_FORMAT "Returned value is the
 * size of the largest WAVEFORMATEX structure. If hao is NULL, the return value is the largest WAVEFORMATEX structure in the system. ...
 * The pMetric parameter must point to a buffer of a size equal to a DWORD value."; "Returns zero if successful or an error otherwise".
 * The guest asks (NULL, 50, &dword) and ignores the result code; it allocates the returned number of bytes for a format buffer.
 * This environment has no ACM driver of its own (the same kind of setting as "no MIDI device" above), so the largest WAVEFORMATEX is the
 * structure itself: 18 bytes. The page gives no number; the value is a choice. The other forms (another metric, a non-NULL hao, a NULL
 * pMetric) have results the page lists as errors without saying which one applies, so they stop. */
void fle_imp_acmMetrics(void)
{
    static const char api[] = "acmMetrics";
    uint64_t hao = R[1], p = R[8];
    uint32_t metric = (uint32_t)R[2];

    if (metric != FLE_ACM_METRIC_MAX_SIZE_FORMAT) fle_stop(api, "uMetric is not ACM_METRIC_MAX_SIZE_FORMAT (50): only the metric the guest asks is answered (decision 0098)");
    if (hao != 0) fle_stop(api, "hao is not NULL: no ACM object exists (decision 0098)");
    if (p == 0) fle_stop(api, "pMetric is NULL: the page does not say which error applies (decision 0098)");
    fle_st(p, 4, FLE_ACM_WAVEFORMATEX_SIZE);
    R[0] = FLE_MMSYSERR_NOERROR;
}
