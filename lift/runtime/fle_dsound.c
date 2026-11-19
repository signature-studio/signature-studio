/* The DirectSound model of the virtual machine (decision 0139, PROVISIONAL; it replaces the device list of decision 0126, S-ds-none, which this file held: "no device").
 *
 * The virtual machine has ONE playback device. dsound!DirectSoundEnumerateA/W call the callback for it (the primary device is enumerated twice, first with a NULL GUID: Learn, DSEnumCallback). DirectSoundCreate / DirectSoundCreate8 return
 * an IDirectSound / IDirectSound8 object. CreateSoundBuffer makes the primary buffer (no memory, no format: it describes the device) and secondary buffers (a circular byte buffer in host memory that Lock hands to the guest). The device
 * reads the playing buffers at the play cursor, mixes them and appends float32 stereo frames to the output ring (runtime/fle_audio_ring.h) `lead` frames ahead of the consumed ones; that lead is the distance of the write cursor from the
 * play cursor. The play position comes from ONE counter, fle_audio_consumed(): the virtual clock natively, the AudioWorklet in the browser (fle_audio_ring.h). Notification events (IDirectSoundNotify) are signaled when the play cursor
 * crosses an offset; the scheduler (fle_thread.c) polls fle_audio_poll and asks fle_audio_next_tick for the next such time, so that a thread that waits for the event is woken by the virtual clock.
 *
 * COM objects: a host allocation whose first 8 bytes are the vtable pointer; the vtable slots are pseudo-VAs of a range of their own (FLE_COM_SLOT_BASE + 0x10 * method id, S-com-slot-range) that the guest's own `call [reg+disp]`
 * leaves in fle_dispatch_miss (fle_dispatch.h), which asks fle_win_try_thunk first (fle_win.c), and that asks fle_com_try_slot. Every slot of every vtable is filled; a method that is not modeled stops the run by name ("IDirectSoundBuffer::SetFX ..."). The slot order is the
 * order of dsound.h (the SDK header, UNOFFICIAL; the Learn pages list the methods alphabetically), checked against the guest: the offsets that FLEngine calls (docs/investigations/audio-m5.md section 3.2) are GetCurrentPosition 0x20,
 * GetFormat 0x28, GetStatus 0x48, Lock 0x58, Play 0x60, SetCurrentPosition 0x68, SetFormat 0x70, Stop 0x90, Unlock 0x98, Restore 0xa0, SetNotificationPositions 0x18, SetCooperativeLevel 0x30, CreateSoundBuffer 0x18.
 *
 * Every rule below that a Microsoft Learn page gives is cited by its method; everything else is a CHOICE WITHOUT A SOURCE named S-ds-... (the list is in docs/decisions/0139-audio-s0-s2.md with the withdrawal conditions). The numeric
 * values of the flags, the HRESULTs and the IIDs are those of dsound.h / the Learn "DirectSound Return Values by Number" table (the table is Learn; the header is UNOFFICIAL).
 *
 * Plain C: no host clock, no thread, no file. */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_event.h"
#include "fle_sys.h"
#include "fle_win.h"
#include "fle_audio_ring.h"
#include "fle_dsound.h"

/* weak: a link without the window table (fle_win.c) or the callback mechanism (fle_callback.c) still builds; the checks that need them are then off or stop by name */
int fle_win_get(uint64_t hwnd, fle_win_view *v) __attribute__((weak));
uint64_t fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9) __attribute__((weak));
void fle_sys_clock_advance_to(uint64_t ticks);   /* fle_sys.c */

/* ---- values (Learn "DirectSound Return Values by Number"; flags: dsound.h, UNOFFICIAL) ---- */
#define HR_OK                  0x00000000u
#define HR_UNSUPPORTED         0x80004001u
#define HR_NOINTERFACE         0x80004002u   /* COM E_NOINTERFACE (Learn "COM Error Codes"); the DirectSound table lists DSERR_NOINTERFACE as 0x000001AE, which is not an HRESULT (S-ds-noiface) */
#define HR_INVALIDPARAM        0x80070057u
#define HR_OUTOFMEMORY         0x8007000Eu
#define HR_NOAGGREGATION       0x80040110u   /* CLASS_E_NOAGGREGATION (DSERR_NOAGGREGATION; Learn "COM Error Codes") */
#define HR_INVALIDCALL         0x88780032u
#define HR_PRIOLEVELNEEDED     0x88780046u
#define HR_BADFORMAT           0x88780064u
#define HR_NODRIVER            0x88780078u
#define HR_BUFFERLOST          0x88780096u

#define DSBCAPS_PRIMARYBUFFER  0x00000001u
#define DSBCAPS_LOCHARDWARE    0x00000004u
#define DSBCAPS_LOCSOFTWARE    0x00000008u
#define DSBCAPS_CTRL3D         0x00000010u
#define DSBCAPS_CTRLFREQUENCY  0x00000020u
#define DSBCAPS_CTRLPAN        0x00000040u
#define DSBCAPS_CTRLVOLUME     0x00000080u
#define DSBCAPS_CTRLPOSITIONNOTIFY 0x00000100u
#define DSBCAPS_CTRLFX         0x00000200u
#define DSBCAPS_STICKYFOCUS    0x00004000u
#define DSBCAPS_GLOBALFOCUS    0x00008000u
#define DSBCAPS_GETCURRENTPOSITION2 0x00010000u
#define DSBCAPS_MUTE3DATMAXDISTANCE 0x00020000u
#define DSBCAPS_LOCDEFER       0x00040000u
#define DSBCAPS_STATIC         0x00000002u
#define DSBCAPS_KNOWN (DSBCAPS_PRIMARYBUFFER | DSBCAPS_STATIC | DSBCAPS_LOCHARDWARE | DSBCAPS_LOCSOFTWARE | DSBCAPS_CTRL3D | DSBCAPS_CTRLFREQUENCY | DSBCAPS_CTRLPAN | DSBCAPS_CTRLVOLUME | \
                       DSBCAPS_CTRLPOSITIONNOTIFY | DSBCAPS_CTRLFX | DSBCAPS_STICKYFOCUS | DSBCAPS_GLOBALFOCUS | DSBCAPS_GETCURRENTPOSITION2 | DSBCAPS_MUTE3DATMAXDISTANCE | DSBCAPS_LOCDEFER)
#define DSBPLAY_LOOPING        0x00000001u
#define DSBPLAY_LOCHARDWARE    0x00000002u
#define DSBPLAY_LOCSOFTWARE    0x00000004u
#define DSBPLAY_TERMINATEBY_TIME     0x00000008u
#define DSBPLAY_TERMINATEBY_DISTANCE 0x00000010u
#define DSBPLAY_TERMINATEBY_PRIORITY 0x00000020u
#define DSBSTATUS_PLAYING      0x00000001u
#define DSBSTATUS_LOOPING      0x00000004u
#define DSBLOCK_FROMWRITECURSOR 0x00000001u
#define DSBLOCK_ENTIREBUFFER   0x00000002u
#define DSSCL_NORMAL           1u
#define DSSCL_PRIORITY         2u
#define DSSCL_EXCLUSIVE        3u
#define DSSCL_WRITEPRIMARY     4u
#define DSBPN_OFFSETSTOP       0xFFFFFFFFu
#define DSBNOTIFICATIONS_MAX   100000u
#define DSBSIZE_MIN            4u
#define DSBPAN_RIGHT           10000
#define DSBVOLUME_MIN          (-10000)
#define DSBFREQUENCY_MIN       100u
#define DSBFREQUENCY_MAX       200000u
#define DSBFREQUENCY_ORIGINAL  0u
#define DSCAPS_PRIMARYMONO     0x00000001u
#define DSCAPS_PRIMARYSTEREO   0x00000002u
#define DSCAPS_PRIMARY8BIT     0x00000004u
#define DSCAPS_PRIMARY16BIT    0x00000008u
#define DSCAPS_CONTINUOUSRATE  0x00000010u
#define DSCAPS_SIZE            96u
#define DSBUFFERDESC_SIZE      40u    /* sizeof(DSBUFFERDESC) on x64: 4 DWORDs, a pointer, a GUID (the guest passes 0x28: audio-m5.md section 3.3) */
#define DSBUFFERDESC1_SIZE     24u    /* DSBUFFERDESC1 of DirectX 7: 4 DWORDs and a pointer (no guid3DAlgorithm) */

#define WAVE_FORMAT_PCM_       1u
#define WAVE_FORMAT_IEEE_FLOAT_ 3u
#define WAVE_FORMAT_EXTENSIBLE_ 0xFFFEu

#define MAX_DEV   8u
#define MAX_BUF   32u
#define MAX_BYTES (UINT32_C(64) << 20)   /* a limit of this environment: one buffer of at most 64 MiB */

/* ---- identities ---- */
static const uint8_t IID_IUnknown_[16]           = { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46 };
static const uint8_t IID_IDirectSound_[16]       = { 0x83, 0xFA, 0x9A, 0x27, 0x81, 0x49, 0xCE, 0x11, 0xA5, 0x21, 0x00, 0x20, 0xAF, 0x0B, 0xE5, 0x60 };
static const uint8_t IID_IDirectSound8_[16]      = { 0x93, 0x7E, 0x0A, 0xC5, 0x95, 0xF3, 0x34, 0x48, 0x9E, 0xF6, 0x7F, 0xA9, 0x9D, 0xE5, 0x09, 0x66 };
static const uint8_t IID_IDirectSoundBuffer_[16] = { 0x85, 0xFA, 0x9A, 0x27, 0x81, 0x49, 0xCE, 0x11, 0xA5, 0x21, 0x00, 0x20, 0xAF, 0x0B, 0xE5, 0x60 };
static const uint8_t IID_IDirectSoundBuffer8_[16] = { 0x49, 0xA4, 0x25, 0x68, 0x24, 0x75, 0x82, 0x4D, 0x92, 0x0F, 0x50, 0xE3, 0x6A, 0xB3, 0xAB, 0x1E };
static const uint8_t IID_IDirectSoundNotify_[16] = { 0x83, 0x07, 0x21, 0xB0, 0xCD, 0x89, 0xD0, 0x11, 0xAF, 0x08, 0x00, 0xA0, 0xC9, 0x25, 0xCD, 0x16 };
static const uint8_t SUB_PCM[16]   = { 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71 };
static const uint8_t SUB_FLOAT[16] = { 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71 };
/* DSDEVID_DefaultPlayback / DSDEVID_DefaultVoicePlayback (dsound.h, UNOFFICIAL): Learn DirectSoundCreate8 lists them as "Equivalent to NULL" / "Default voice playback device" */
static const uint8_t DEV_DEFAULT[16] = { 0x00, 0x00, 0xF0, 0xDE, 0x6D, 0x9C, 0xED, 0x47, 0xAA, 0xF1, 0x4D, 0xDA, 0x8F, 0x2B, 0x5C, 0x03 };
static const uint8_t DEV_VOICE[16]   = { 0x02, 0x00, 0xF0, 0xDE, 0x6D, 0x9C, 0xED, 0x47, 0xAA, 0xF1, 0x4D, 0xDA, 0x8F, 0x2B, 0x5C, 0x03 };

/* S-ds-device-strings / S-ds-guid: the virtual device. Learn gives no value for any of them. The first callback is the primary device ("Primary Sound Driver" is the description that Windows gives it); the second is the
 * same device with its GUID. The GUID ends in the ASCII of "FLSTUD". The module name is empty for both. */
static const uint8_t DEV_GUID[16] = { 0x00, 0x1E, 0x5D, 0x0F, 0x01, 0x00, 0x00, 0x40, 0x80, 0x00, 0x46, 0x4C, 0x53, 0x54, 0x55, 0x44 };
static const char DESC_PRIMARY_A[] = "Primary Sound Driver";
static const char DESC_DEVICE_A[]  = "Browser Audio Output";
static const char MODULE_A[]       = "";
static const uint16_t DESC_PRIMARY_W[] = { 'P','r','i','m','a','r','y',' ','S','o','u','n','d',' ','D','r','i','v','e','r', 0 };
static const uint16_t DESC_DEVICE_W[]  = { 'B','r','o','w','s','e','r',' ','A','u','d','i','o',' ','O','u','t','p','u','t', 0 };
static const uint16_t MODULE_W[]       = { 0 };

/* ---- the method ids = the slot numbers of the pseudo-VAs ---- */
#define METHODS(X) \
    X(DS_QI, "IDirectSound::QueryInterface") X(DS_ADDREF, "IDirectSound::AddRef") X(DS_RELEASE, "IDirectSound::Release") \
    X(DS_CREATESOUNDBUFFER, "IDirectSound::CreateSoundBuffer") X(DS_GETCAPS, "IDirectSound::GetCaps") X(DS_DUPLICATE, "IDirectSound::DuplicateSoundBuffer") \
    X(DS_SETCOOP, "IDirectSound::SetCooperativeLevel") X(DS_COMPACT, "IDirectSound::Compact") X(DS_GETSPEAKER, "IDirectSound::GetSpeakerConfig") \
    X(DS_SETSPEAKER, "IDirectSound::SetSpeakerConfig") X(DS_INITIALIZE, "IDirectSound::Initialize") X(DS_VERIFYCERT, "IDirectSound8::VerifyCertification") \
    X(B_QI, "IDirectSoundBuffer::QueryInterface") X(B_ADDREF, "IDirectSoundBuffer::AddRef") X(B_RELEASE, "IDirectSoundBuffer::Release") X(B_GETCAPS, "IDirectSoundBuffer::GetCaps") \
    X(B_GETCURPOS, "IDirectSoundBuffer::GetCurrentPosition") X(B_GETFORMAT, "IDirectSoundBuffer::GetFormat") X(B_GETVOLUME, "IDirectSoundBuffer::GetVolume") \
    X(B_GETPAN, "IDirectSoundBuffer::GetPan") X(B_GETFREQ, "IDirectSoundBuffer::GetFrequency") X(B_GETSTATUS, "IDirectSoundBuffer::GetStatus") \
    X(B_INITIALIZE, "IDirectSoundBuffer::Initialize") X(B_LOCK, "IDirectSoundBuffer::Lock") X(B_PLAY, "IDirectSoundBuffer::Play") \
    X(B_SETCURPOS, "IDirectSoundBuffer::SetCurrentPosition") X(B_SETFORMAT, "IDirectSoundBuffer::SetFormat") X(B_SETVOLUME, "IDirectSoundBuffer::SetVolume") \
    X(B_SETPAN, "IDirectSoundBuffer::SetPan") X(B_SETFREQ, "IDirectSoundBuffer::SetFrequency") X(B_STOP, "IDirectSoundBuffer::Stop") X(B_UNLOCK, "IDirectSoundBuffer::Unlock") \
    X(B_RESTORE, "IDirectSoundBuffer::Restore") X(B_SETFX, "IDirectSoundBuffer8::SetFX") X(B_ACQUIRE, "IDirectSoundBuffer8::AcquireResources") X(B_GETOBJ, "IDirectSoundBuffer8::GetObjectInPath") \
    X(N_QI, "IDirectSoundNotify::QueryInterface") X(N_ADDREF, "IDirectSoundNotify::AddRef") X(N_RELEASE, "IDirectSoundNotify::Release") \
    X(N_SETPOS, "IDirectSoundNotify::SetNotificationPositions")

enum {
#define X(id, name) M_##id,
    METHODS(X)
#undef X
    M_COUNT
};
static const char *const mname[M_COUNT] = {
#define X(id, name) name,
    METHODS(X)
#undef X
};

/* vtable slot counts: IDirectSound 11 (+1 for IDirectSound8), IDirectSoundBuffer 21 (+3 for IDirectSoundBuffer8), IDirectSoundNotify 4 */
#define VT_DS_N  12u
#define VT_B_N   24u
#define VT_N_N   4u
static uint64_t vt_ds[VT_DS_N], vt_ds_v1[VT_DS_N], vt_b[VT_B_N], vt_n[VT_N_N];
static int vt_ready;

static uint64_t slot_va(unsigned id) { return FLE_COM_SLOT_BASE + FLE_COM_SLOT_STEP * (uint64_t)id; }

static void vt_init(void)
{
    static const unsigned ds[VT_DS_N] = { M_DS_QI, M_DS_ADDREF, M_DS_RELEASE, M_DS_CREATESOUNDBUFFER, M_DS_GETCAPS, M_DS_DUPLICATE, M_DS_SETCOOP, M_DS_COMPACT, M_DS_GETSPEAKER, M_DS_SETSPEAKER, M_DS_INITIALIZE, M_DS_VERIFYCERT };
    static const unsigned b[VT_B_N] = { M_B_QI, M_B_ADDREF, M_B_RELEASE, M_B_GETCAPS, M_B_GETCURPOS, M_B_GETFORMAT, M_B_GETVOLUME, M_B_GETPAN, M_B_GETFREQ, M_B_GETSTATUS, M_B_INITIALIZE, M_B_LOCK, M_B_PLAY,
                                        M_B_SETCURPOS, M_B_SETFORMAT, M_B_SETVOLUME, M_B_SETPAN, M_B_SETFREQ, M_B_STOP, M_B_UNLOCK, M_B_RESTORE, M_B_SETFX, M_B_ACQUIRE, M_B_GETOBJ };
    static const unsigned n[VT_N_N] = { M_N_QI, M_N_ADDREF, M_N_RELEASE, M_N_SETPOS };
    unsigned i;

    if (vt_ready) return;
    for (i = 0; i < VT_DS_N; i++) vt_ds[i] = slot_va(ds[i]);
    for (i = 0; i < VT_DS_N; i++) vt_ds_v1[i] = slot_va(ds[i]);
    vt_ds_v1[11] = slot_va(M_DS_VERIFYCERT);   /* the slot exists in the table of an IDirectSound too (the guest never reads it); calling it stops by name */
    for (i = 0; i < VT_B_N; i++) vt_b[i] = slot_va(b[i]);
    for (i = 0; i < VT_N_N; i++) vt_n[i] = slot_va(n[i]);
    vt_ready = 1;
}

/* ---- objects ---- */
typedef struct {
    uint32_t tag, channels, rate, bits, align;   /* tag: 1 PCM, 3 IEEE float (an EXTENSIBLE format is reduced to the tag of its SubFormat) */
    uint8_t raw[18 + 64];                        /* the format as given (WAVEFORMATEX + extra bytes up to 64), returned by GetFormat */
    uint32_t raw_len;
} wfmt;

typedef struct { uint32_t off; uint64_t h; } ds_notify_pos;

typedef struct ds_dev ds_dev;
typedef struct ds_buf ds_buf;

struct ds_dev {
    uint64_t vtbl;           /* first: the guest reads it */
    uint32_t refs;
    int live;
    int ds8;
    uint32_t coop;           /* 0 = SetCooperativeLevel not called */
    uint64_t hwnd;
    ds_buf *primary;
    wfmt fmt;                /* the primary format = the device format */
};

struct ds_buf {
    uint64_t vtbl;           /* first: the IDirectSoundBuffer interface pointer is the object pointer */
    uint64_t nvtbl;          /* the embedded IDirectSoundNotify interface: its pointer is &nvtbl, its vtable pointer is this field */
    uint32_t refs;
    int live;
    ds_dev *dev;
    int primary, ds8;
    uint32_t flags, size;    /* DSBCAPS; bytes (0 for the primary buffer) */
    uint8_t *data;
    wfmt fmt;
    int locked;
    uint32_t lk_off, lk_n1, lk_n2;
    int playing, looping, silent_until_coop;
    uint32_t cursor;         /* frames: the play cursor while stopped */
    uint64_t base;           /* device frame index at which start_pos applies */
    uint32_t start_pos;      /* frames */
    uint64_t pulled;         /* device frames handed to the output so far: index of the first frame not handed yet */
    uint64_t prev_abs;       /* last play position seen by the notification check, unrolled, in frames */
    int32_t vol, pan;
    uint32_t freq;
    uint32_t nnotify;
    ds_notify_pos *notify;
    uint64_t unlocks, unlock_bytes;
};

static ds_dev *devs[MAX_DEV];
static ds_buf *bufs[MAX_BUF];
static dsound_stats stats;

const dsound_stats *fle_dsound_stats(void) { return &stats; }

static void ret(uint32_t hr) { R[0] = hr; }

static int guid_eq(uint64_t g, const uint8_t want[16])
{
    unsigned i;

    for (i = 0; i < 16; i++)
        if ((uint8_t)fle_ld(g + i, 1) != want[i]) return 0;
    return 1;
}


/* ---- the audio engine of the device: mixing and notification ---- */
static unsigned play_count(void)
{
    unsigned i, n = 0;

    for (i = 0; i < MAX_BUF; i++)
        if (bufs[i] && bufs[i]->live && bufs[i]->playing) n++;
    return n;
}

static uint32_t size_frames(const ds_buf *b) { return b->size / b->fmt.align; }

static float sample_at(const ds_buf *b, uint32_t frame, unsigned ch)
{
    const uint8_t *p = b->data + (size_t)frame * b->fmt.align;
    unsigned c = b->fmt.channels == 1 ? 0u : ch;

    if (b->fmt.tag == WAVE_FORMAT_IEEE_FLOAT_) {
        float f;

        memcpy(&f, p + 4u * c, 4);
        return f;
    }
    if (b->fmt.bits == 8) return ((float)p[c] - 128.0f) / 128.0f;
    {
        int16_t s = (int16_t)(p[2u * c] | (p[2u * c + 1u] << 8));

        return (float)s / 32768.0f;   /* S-ds-pcm16-scale: 16-bit PCM is divided by 32768 (a full-scale -32768 is -1.0) */
    }
}

static void gains(const ds_buf *b, float *gl, float *gr)
{
    float v = b->silent_until_coop ? 0.0f : powf(10.0f, (float)b->vol / 2000.0f);   /* Learn SetVolume: attenuation in hundredths of a dB */
    float l = v, r = v;

    if (b->pan > 0) l *= powf(10.0f, -(float)b->pan / 2000.0f);   /* Learn SetPan: DSBPAN_RIGHT means the left channel is attenuated by 100 dB */
    if (b->pan < 0) r *= powf(10.0f, (float)b->pan / 2000.0f);
    if (b->vol <= DSBVOLUME_MIN) l = r = 0.0f;
    if (b->pan >= DSBPAN_RIGHT) l = 0.0f;
    if (b->pan <= -DSBPAN_RIGHT) r = 0.0f;
    *gl = l;
    *gr = r;
}

/* Mixes the playing buffers for the device frames [i0, i0 + n) into out (interleaved L R). A buffer that has not started (index < base) adds nothing; a buffer that is not looping and reaches its end stops there. */
static void mix_chunk(uint64_t i0, uint32_t n, float *out)
{
    unsigned bi;
    uint32_t j;

    memset(out, 0, (size_t)n * 2u * sizeof(float));
    for (bi = 0; bi < MAX_BUF; bi++) {
        ds_buf *b = bufs[bi];
        float gl, gr;
        uint32_t sf;

        if (!b || !b->live || !b->playing || b->primary) continue;
        sf = size_frames(b);
        gains(b, &gl, &gr);
        for (j = 0; j < n; j++) {
            uint64_t i = i0 + j, pos;

            if (i < b->base) continue;
            pos = (uint64_t)b->start_pos + (i - b->base);
            if (pos >= sf) {
                if (!b->looping) continue;
                pos %= sf;
            }
            out[2u * j] += gl * sample_at(b, (uint32_t)pos, 0);
            out[2u * j + 1u] += gr * sample_at(b, (uint32_t)pos, 1);
        }
    }
}

/* The unrolled play position of a playing buffer at device frame index i (frames since the start of the buffer's data, not reduced modulo the size). */
static uint64_t abs_pos_at(const ds_buf *b, uint64_t i)
{
    return (uint64_t)b->start_pos + (i > b->base ? i - b->base : 0u);
}

/* S-ds-late-write (decision 0241, PROVISIONAL). The device hands the frames up to consumed + lead to the output ahead of time (audio_sync), reading the buffers as they are then. Learn (Play and Write Cursors): the data between the
 * play cursor and the write cursor "may already have been queued for playback and should not be touched" - what happens to a write there is not defined. With an external consumer (the AudioWorklet) a write that came after the
 * hand-over used to be lost for the frames already handed over although the output had not read them yet (they played as the old content). Here such a write takes effect while it is still in time: after an Unlock of a playing
 * buffer, the frames that were handed over before it (up to `wf0`) and are at least the guard ahead of the output's read position are mixed again from the buffers (their present content) and put over the ones handed over.
 * The frames of the written region that are nearer than the guard cannot be changed any more and are counted as lost. `wf0` is the end of the hand-over before this Unlock's own synchronisation (the frames that synchronisation adds are made from the new content).
 * A frame is the region's if the buffer position that the output reads at that frame lies in the region (a looping buffer wraps, a buffer that plays once ends). */
static void rewrite_unread(const ds_buf *b, uint32_t off_f, uint32_t n1_f, uint32_t n2_f, uint64_t wf0)
{
    uint64_t c = fle_audio_consumed(), guard = fle_audio_rewrite_guard_frames(), sf = size_frames(b), pos0 = abs_pos_at(b, c) % (b->looping ? sf : (uint64_t)1 << 62), d, from, to;
    uint32_t applied = 0, lost = 0;

    if (wf0 <= c || sf == 0) return;
    for (d = 0; d < wf0 - c; d++) {
        uint64_t q = pos0 + d;

        if (b->looping) q %= sf;
        else if (q >= sf) break;
        if (!((q >= off_f && q < (uint64_t)off_f + n1_f) || q < n2_f)) continue;
        if (d < guard) lost++;
        else applied++;
    }
    if (applied == 0 && lost == 0) return;
    fle_audio_rewrite_note(applied, lost);
    stats.late_writes++;
    stats.late_write_applied += applied;
    stats.late_write_lost += lost;
    if (applied == 0) return;
    from = c + guard;
    to = wf0;
    while (from < to) {
        float tmp[2 * 1024];
        uint32_t n = (uint32_t)(to - from < 1024 ? to - from : 1024);

        mix_chunk(from, n, tmp);
        fle_audio_ring_rewrite(tmp, n, from);
        from += n;
    }
}

static void fire_notifications(ds_buf *b, uint64_t from_abs, uint64_t to_abs)
{
    uint32_t sf = size_frames(b), k;
    uint64_t w, w_from, w_to;

    if (!b->nnotify || to_abs <= from_abs || sf == 0) return;
    w_from = from_abs / sf;
    w_to = to_abs / sf;
    if (w_to - w_from > 64) w_from = w_to - 64;   /* many wraps in one step (the poll was very late): the events coalesce anyway */
    for (w = w_from; w <= w_to; w++) {
        /* the offsets of one wrap in the order of the play cursor */
        uint32_t last = 0;
        int started = 0;

        for (;;) {
            uint32_t best = 0;
            int have = 0;

            for (k = 0; k < b->nnotify; k++) {
                uint32_t off = b->notify[k].off;

                if (off == DSBPN_OFFSETSTOP) continue;
                off /= b->fmt.align;
                if (started && off <= last) continue;
                if (!have || off < best) {
                    best = off;
                    have = 1;
                }
            }
            if (!have) break;
            {
                uint64_t a = w * sf + best;

                if (a > from_abs && a <= to_abs)
                    for (k = 0; k < b->nnotify; k++)
                        if (b->notify[k].off != DSBPN_OFFSETSTOP && b->notify[k].off / b->fmt.align == best) {
                            if (fle_event_host_set(b->notify[k].h)) stats.notifications++;
                        }
            }
            last = best;
            started = 1;
        }
    }
}

/* One synchronisation of the device with the output: hands frames to the ring up to consumed + lead, fires the notifications the play cursor has crossed, finishes buffers that reached their end. */
static void audio_sync(void)
{
    uint64_t c;
    unsigned bi;
    uint32_t lead;
    fle_audio_ring_t *r;

    if (play_count() == 0) return;
    r = fle_audio_ring_get();
    fle_audio_ring_sync_consumer();
    c = fle_audio_consumed();
    lead = fle_audio_lead_frames();
    r->lead_frames = lead;
    if (r->write_frames < c) {
        /* The poll is later than the frames it should have handed over. Virtual clock: the clock moves only inside the scheduler's waits, while no guest code runs, so the buffers did not change in the gap: the frames are
         * handed over now, from the current content (what a real device would have read). External consumer (the AudioWorklet): it has already played silence for the gap, so the frames are lost; skip to the consumed
         * position and count them (late_frames). A gap of more than 8 rings in virtual mode is skipped too (an environment limit). */
        if (fle_audio_external() || c - r->write_frames > 8u * r->capacity) {
            stats.late_frames += c - r->write_frames;
            r->write_frames = c;
        }
    }
    {
        uint64_t target = c + lead;

        while (r->write_frames < target) {
            float tmp[2 * 1024];
            uint32_t n = (uint32_t)(target - r->write_frames < 1024 ? target - r->write_frames : 1024), took;

            mix_chunk(r->write_frames, n, tmp);
            took = fle_audio_ring_append(tmp, n);
            if (took == 0) break;
        }
    }
    for (bi = 0; bi < MAX_BUF; bi++) {
        ds_buf *b = bufs[bi];
        uint64_t now_abs, sf;

        if (!b || !b->live || !b->playing || b->primary) continue;
        b->pulled = r->write_frames;
        now_abs = abs_pos_at(b, c);
        sf = size_frames(b);
        fire_notifications(b, b->prev_abs, now_abs);
        b->prev_abs = now_abs;
        if (!b->looping && now_abs >= sf) {   /* Learn Play: "If this value is not set, the buffer will stop when it reaches the end of the sound data." */
            b->playing = 0;
            b->cursor = 0;
            b->start_pos = 0;
            for (uint32_t k = 0; k < b->nnotify; k++)
                if (b->notify[k].off == DSBPN_OFFSETSTOP) fle_event_host_set(b->notify[k].h);
        }
    }
    if (play_count() == 0) fle_audio_ring_set_state(FLE_AUDIO_OPEN);
}

void fle_audio_poll(void) { audio_sync(); }

int fle_audio_next_tick(uint64_t *out)
{
    unsigned bi;
    int found = 0;
    uint64_t c;

    if (play_count() == 0) return 0;
    c = fle_audio_consumed();
    for (bi = 0; bi < MAX_BUF; bi++) {
        ds_buf *b = bufs[bi];
        uint32_t sf, k;
        uint64_t cur, best = 0;
        int have = 0;

        if (!b || !b->live || !b->playing || b->primary || !b->nnotify) continue;
        sf = size_frames(b);
        cur = abs_pos_at(b, c);
        for (k = 0; k < b->nnotify; k++) {
            uint64_t off, a;

            if (b->notify[k].off == DSBPN_OFFSETSTOP) continue;
            off = b->notify[k].off / b->fmt.align;
            a = (cur / sf) * sf + off;
            if (a <= cur) a += sf;
            if (!have || a < best) {
                best = a;
                have = 1;
            }
        }
        if (have) {
            uint64_t i_now = c > b->base ? c : b->base;
            uint64_t t = fle_audio_tick_of_frame(i_now + (best - cur));

            if (t != 0 && (!found || t < *out)) {
                *out = t;
                found = 1;
            }
        }
    }
    return found;
}

/* ---- format checks ---- */
static int read_format(uint64_t p, wfmt *f)
{
    uint32_t tag, cb = 0, bits, i;
    uint32_t total;

    if (p == 0) return 0;
    tag = (uint32_t)fle_ld(p, 2);
    f->channels = (uint32_t)fle_ld(p + 2, 2);
    f->rate = (uint32_t)fle_ld(p + 4, 4);
    f->align = (uint32_t)fle_ld(p + 12, 2);
    f->bits = bits = (uint32_t)fle_ld(p + 14, 2);
    if (tag != WAVE_FORMAT_PCM_) cb = (uint32_t)fle_ld(p + 16, 2);
    if (cb > 64u) return -1;
    total = tag == WAVE_FORMAT_PCM_ ? 16u : 18u + cb;
    memset(f->raw, 0, sizeof f->raw);
    for (i = 0; i < total; i++) f->raw[i] = (uint8_t)fle_ld(p + i, 1);
    f->raw_len = tag == WAVE_FORMAT_PCM_ ? 18u : total;   /* GetFormat returns a WAVEFORMATEX with cbSize (0 for PCM) */
    f->tag = tag;
    if (tag == WAVE_FORMAT_EXTENSIBLE_) {
        if (cb < 22u) return -1;
        if (guid_eq(p + 24, SUB_PCM)) f->tag = WAVE_FORMAT_PCM_;
        else if (guid_eq(p + 24, SUB_FLOAT)) f->tag = WAVE_FORMAT_IEEE_FLOAT_;
        else return -1;
    }
    return 1;
}

/* 1 when the format is one the device plays; the bad ones are DSERR_BADFORMAT (S-ds-formats: PCM 8/16-bit and IEEE float 32-bit, one or two channels). The Learn WAVEFORMATEX page: nBlockAlign "must be equal" to
 * nChannels * wBitsPerSample / 8 for PCM and EXTENSIBLE; nAvgBytesPerSec "should be" rate * block (S-ds-avgbytes: not checked). */
static int format_ok(const wfmt *f)
{
    if (f->channels != 1 && f->channels != 2) return 0;
    if (f->rate < DSBFREQUENCY_MIN || f->rate > DSBFREQUENCY_MAX) return 0;
    if (f->tag == WAVE_FORMAT_PCM_) {
        if (f->bits != 8 && f->bits != 16) return 0;
    } else if (f->tag == WAVE_FORMAT_IEEE_FLOAT_) {
        if (f->bits != 32) return 0;
    } else {
        return 0;
    }
    return f->align == f->channels * f->bits / 8u;
}

/* ---- object bookkeeping ---- */
static ds_dev *dev_of(uint64_t p)
{
    unsigned i;

    for (i = 0; i < MAX_DEV; i++)
        if (devs[i] && (uint64_t)(uintptr_t)devs[i] == p) return devs[i];
    return NULL;
}

static ds_buf *buf_of(uint64_t p, int *via_notify)
{
    unsigned i;

    for (i = 0; i < MAX_BUF; i++) {
        if (!bufs[i]) continue;
        if ((uint64_t)(uintptr_t)bufs[i] == p) {
            if (via_notify) *via_notify = 0;
            return bufs[i];
        }
        if ((uint64_t)(uintptr_t)&bufs[i]->nvtbl == p) {
            if (via_notify) *via_notify = 1;
            return bufs[i];
        }
    }
    return NULL;
}

static ds_dev *this_dev(unsigned id)
{
    ds_dev *d = dev_of(R[1]);

    if (!d) fle_stop(mname[id], "`this` is not a DirectSound object that this runtime made");
    if (!d->live) fle_stop(mname[id], "the object was released (use after the last Release)");
    return d;
}

static ds_buf *this_buf(unsigned id, int notify_ok)
{
    int vn = 0;
    ds_buf *b = buf_of(R[1], &vn);

    if (!b) fle_stop(mname[id], "`this` is not a DirectSound buffer that this runtime made");
    if (!b->live) fle_stop(mname[id], "the object was released (use after the last Release)");
    if (vn != notify_ok && notify_ok >= 0) fle_stop(mname[id], "the interface pointer is not the one this method belongs to");
    return b;
}

/* ---- the IUnknown methods (Learn: IUnknown::AddRef/Release return the new count; QueryInterface returns E_NOINTERFACE and a NULL pointer for an interface the object does not have) ---- */
static int iid_is(uint64_t iid, const uint8_t want[16]) { return guid_eq(iid, want); }

static void m_ds_qi(void)
{
    ds_dev *d = this_dev(M_DS_QI);
    uint64_t iid = R[2], pp = R[8];

    if (iid == 0 || pp == 0) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (iid_is(iid, IID_IUnknown_) || iid_is(iid, IID_IDirectSound_) || (d->ds8 && iid_is(iid, IID_IDirectSound8_))) {
        d->refs++;
        fle_st(pp, 8, (uint64_t)(uintptr_t)d);
        ret(HR_OK);
        return;
    }
    fle_st(pp, 8, 0);
    ret(HR_NOINTERFACE);
}

static void m_ds_addref(void)
{
    ds_dev *d = this_dev(M_DS_ADDREF);

    d->refs++;
    R[0] = d->refs;
}

static void buf_destroy(ds_buf *b)
{
    b->live = 0;
    b->playing = 0;
    free(b->data);
    b->data = NULL;
    free(b->notify);
    b->notify = NULL;
    if (play_count() == 0) fle_audio_ring_set_state(FLE_AUDIO_OPEN);
}

static void m_ds_release(void)
{
    ds_dev *d = this_dev(M_DS_RELEASE);
    unsigned i;

    if (d->refs == 0) fle_stop(mname[M_DS_RELEASE], "no reference outstanding");
    d->refs--;
    if (d->refs == 0) {
        d->live = 0;
        for (i = 0; i < MAX_BUF; i++)
            if (bufs[i] && bufs[i]->live && bufs[i]->dev == d) buf_destroy(bufs[i]);
    }
    R[0] = d->refs;
}

static ds_buf *new_buf(ds_dev *d)
{
    unsigned i;
    ds_buf *b;

    for (i = 0; i < MAX_BUF; i++)
        if (!bufs[i]) break;
    if (i == MAX_BUF) return NULL;
    b = calloc(1, sizeof *b);
    if (!b) return NULL;
    vt_init();
    b->vtbl = (uint64_t)(uintptr_t)vt_b;
    b->nvtbl = (uint64_t)(uintptr_t)vt_n;
    b->refs = 1;
    b->live = 1;
    b->dev = d;
    b->vol = 0;
    b->pan = 0;
    bufs[i] = b;
    return b;
}

/* IDirectSound::CreateSoundBuffer(pcDSBufferDesc rdx, ppDSBuffer r8, pUnkOuter r9) */
static void m_ds_createsoundbuffer(void)
{
    ds_dev *d = this_dev(M_DS_CREATESOUNDBUFFER);
    uint64_t desc = R[2], pp = R[8], outer = R[9];
    uint32_t size, flags, bytes;
    uint64_t pfmt;
    ds_buf *b;
    int rf;

    if (outer != 0) {   /* Learn: pUnkOuter "Must be NULL"; DSERR_NOAGGREGATION is in the list */
        ret(HR_NOAGGREGATION);
        return;
    }
    if (desc == 0 || pp == 0) {
        ret(HR_INVALIDPARAM);
        return;
    }
    fle_st(pp, 8, 0);
    size = (uint32_t)fle_ld(desc, 4);
    flags = (uint32_t)fle_ld(desc + 4, 4);
    bytes = (uint32_t)fle_ld(desc + 8, 4);
    pfmt = fle_ld(desc + 16, 8);
    if (size != DSBUFFERDESC_SIZE && size != DSBUFFERDESC1_SIZE) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (flags & ~DSBCAPS_KNOWN) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (flags & DSBCAPS_CTRL3D) fle_stop(mname[M_DS_CREATESOUNDBUFFER], "DSBCAPS_CTRL3D (3D buffers) is not modeled (S-ds-no-3d)");
    if (flags & DSBCAPS_CTRLFX) fle_stop(mname[M_DS_CREATESOUNDBUFFER], "DSBCAPS_CTRLFX (effects) is not modeled (S-ds-no-fx)");
    if (flags & (DSBCAPS_LOCDEFER | DSBCAPS_STATIC)) fle_stop(mname[M_DS_CREATESOUNDBUFFER], "DSBCAPS_LOCDEFER / DSBCAPS_STATIC are not modeled (S-ds-no-voice-manager)");
    if ((flags & DSBCAPS_LOCHARDWARE) && (flags & DSBCAPS_LOCSOFTWARE)) {   /* Learn DSBUFFERDESC: "mutually exclusive" */
        ret(HR_INVALIDPARAM);
        return;
    }
    if (flags & DSBCAPS_LOCHARDWARE) {   /* Learn CreateSoundBuffer: with no hardware acceleration "fails with either DSERR_CONTROLUNAVAIL or DSERR_INVALIDCALL" (S-ds-hw: INVALIDCALL, the virtual machine has no hardware mixing) */
        ret(HR_INVALIDCALL);
        return;
    }
    if (size == DSBUFFERDESC_SIZE && !(flags & DSBCAPS_CTRL3D)) {
        /* Learn DSBUFFERDESC: "If DSBCAPS_CTRL3D is not set in dwFlags, this member must be GUID_NULL". FLEngine leaves the member as the stack had it (audio-m5.md 3.3): the model does NOT read it (S-ds-guid3d-unread). */
    }
    if (flags & DSBCAPS_PRIMARYBUFFER) {
        if (bytes != 0 || pfmt != 0) {   /* Learn DSBUFFERDESC: dwBufferBytes "must be 0", lpwfxFormat "must be NULL" for the primary buffer */
            ret(HR_INVALIDPARAM);
            return;
        }
        if (d->primary && d->primary->live) fle_stop(mname[M_DS_CREATESOUNDBUFFER], "a second primary buffer on one device (no page defines it: S-ds-primary-twice)");
        b = new_buf(d);
        if (!b) {
            ret(HR_OUTOFMEMORY);
            return;
        }
        b->primary = 1;
        b->ds8 = 0;   /* Learn CreateSoundBuffer: "IDirectSoundBuffer8 is not available for the primary buffer" */
        b->flags = flags;
        b->fmt = d->fmt;
        d->primary = b;
        fle_st(pp, 8, (uint64_t)(uintptr_t)b);
        ret(HR_OK);
        return;
    }
    if (pfmt == 0) {
        ret(HR_INVALIDPARAM);
        return;
    }
    {
        wfmt f;

        rf = read_format(pfmt, &f);
        if (rf < 0 || (rf > 0 && !format_ok(&f))) {
            ret(HR_BADFORMAT);
            return;
        }
        if (bytes < DSBSIZE_MIN || bytes % f.align != 0) {   /* Learn DSBUFFERDESC: "DSBSIZE_MIN and DSBSIZE_MAX" bound the size; S-ds-size-multiple: a whole number of blocks (WAVEFORMATEX: data starts at a block boundary) */
            ret(HR_INVALIDPARAM);
            return;
        }
        if (bytes > MAX_BYTES) {
            ret(HR_OUTOFMEMORY);
            return;
        }
        if (f.rate != d->fmt.rate) fle_stop(mname[M_DS_CREATESOUNDBUFFER], "a secondary buffer whose rate differs from the device's: resampling is not modeled (S-ds-no-resample)");
        b = new_buf(d);
        if (!b) {
            ret(HR_OUTOFMEMORY);
            return;
        }
        b->data = calloc(1, bytes);   /* S-ds-buffer-init: Learn "DirectSound does not initialize the contents"; the model starts with zeros */
        if (!b->data) {
            unsigned q;

            for (q = 0; q < MAX_BUF; q++)
                if (bufs[q] == b) bufs[q] = NULL;
            free(b);
            ret(HR_OUTOFMEMORY);
            return;
        }
        b->fmt = f;
        b->size = bytes;
        b->flags = flags;
        b->ds8 = d->ds8;
        b->freq = DSBFREQUENCY_ORIGINAL;
        fle_st(pp, 8, (uint64_t)(uintptr_t)b);
        stats.secondary_created++;
        ret(HR_OK);
    }
}

/* IDirectSound::GetCaps(pDSCaps rdx). S-ds-caps: a software mixing device with continuous rates; the page lists the members and the flag names, not the values of a device. */
static void m_ds_getcaps(void)
{
    ds_dev *d = this_dev(M_DS_GETCAPS);
    uint64_t p = R[2];
    unsigned i;

    (void)d;
    if (p == 0 || (uint32_t)fle_ld(p, 4) != DSCAPS_SIZE) {
        ret(HR_INVALIDPARAM);
        return;
    }
    for (i = 4; i < DSCAPS_SIZE; i += 4) fle_st(p + i, 4, 0);
    fle_st(p + 4, 4, DSCAPS_PRIMARYMONO | DSCAPS_PRIMARYSTEREO | DSCAPS_PRIMARY8BIT | DSCAPS_PRIMARY16BIT | DSCAPS_CONTINUOUSRATE);
    fle_st(p + 8, 4, DSBFREQUENCY_MIN);
    fle_st(p + 12, 4, DSBFREQUENCY_MAX);
    fle_st(p + 16, 4, 1);   /* Learn: dwPrimaryBuffers "will always be 1" */
    ret(HR_OK);
}

/* IDirectSound::SetCooperativeLevel(hwnd rdx, dwLevel r8d) */
static void m_ds_setcoop(void)
{
    ds_dev *d = this_dev(M_DS_SETCOOP);
    uint64_t hwnd = R[2];
    uint32_t level = (uint32_t)R[8];
    fle_win_view v;
    unsigned i;

    if (level < DSSCL_NORMAL || level > DSSCL_WRITEPRIMARY) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (level == DSSCL_WRITEPRIMARY) fle_stop(mname[M_DS_SETCOOP], "DSSCL_WRITEPRIMARY (the application writes the primary buffer; no secondary buffer can play) is not modeled (S-ds-no-writeprimary)");
    if (hwnd == 0 || (fle_win_get && !(fle_win_get(hwnd, &v) && v.live))) {   /* S-ds-hwnd: a window of the virtual machine is required (the page: "Handle to the application window") */
        ret(HR_INVALIDPARAM);
        return;
    }
    for (i = 0; i < MAX_BUF; i++)
        if (bufs[i] && bufs[i]->live && bufs[i]->dev == d && bufs[i]->locked) {   /* Learn: "Do not call this method if any buffers are locked." (S-ds-coop-locked: DSERR_INVALIDCALL) */
            ret(HR_INVALIDCALL);
            return;
        }
    d->coop = level;
    d->hwnd = hwnd;
    for (i = 0; i < MAX_BUF; i++)
        if (bufs[i] && bufs[i]->live && bufs[i]->dev == d) bufs[i]->silent_until_coop = 0;   /* Learn Play: "no sound will be produced until a cooperative level has been set" */
    ret(HR_OK);
}

/* ---- IDirectSoundBuffer ---- */
static void m_b_qi(void)
{
    int vn = 0;
    ds_buf *b = buf_of(R[1], &vn);
    uint64_t iid = R[2], pp = R[8];

    if (!b || !b->live) fle_stop(mname[M_B_QI], "`this` is not a live DirectSound buffer");
    if (iid == 0 || pp == 0) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (iid_is(iid, IID_IUnknown_) || iid_is(iid, IID_IDirectSoundBuffer_)) {
        b->refs++;
        fle_st(pp, 8, (uint64_t)(uintptr_t)b);
        ret(HR_OK);
        return;
    }
    if (iid_is(iid, IID_IDirectSoundBuffer8_) && b->ds8 && !b->primary) {   /* Learn CreateSoundBuffer: "Use QueryInterface to obtain IDirectSoundBuffer8. IDirectSoundBuffer8 is not available for the primary buffer." */
        b->refs++;
        fle_st(pp, 8, (uint64_t)(uintptr_t)b);
        ret(HR_OK);
        return;
    }
    if (iid_is(iid, IID_IDirectSoundNotify_) && (b->flags & DSBCAPS_CTRLPOSITIONNOTIFY) && !b->primary) {   /* the notify interface exists only for a buffer made with DSBCAPS_CTRLPOSITIONNOTIFY (Learn DSBUFFERDESC) */
        b->refs++;
        fle_st(pp, 8, (uint64_t)(uintptr_t)&b->nvtbl);
        ret(HR_OK);
        return;
    }
    fle_st(pp, 8, 0);
    ret(HR_NOINTERFACE);
}

static void m_b_addref(void)
{
    ds_buf *b = buf_of(R[1], NULL);

    if (!b || !b->live) fle_stop(mname[M_B_ADDREF], "`this` is not a live DirectSound buffer");
    b->refs++;
    R[0] = b->refs;
}

static void m_b_release(void)
{
    ds_buf *b = buf_of(R[1], NULL);

    if (!b || !b->live) fle_stop(mname[M_B_RELEASE], "`this` is not a live DirectSound buffer");
    if (b->refs == 0) fle_stop(mname[M_B_RELEASE], "no reference outstanding");
    b->refs--;
    if (b->refs == 0) buf_destroy(b);
    R[0] = b->refs;
}

/* S-ds-write-cursor (0295): the write cursor that the guest sees leads the play cursor by DS_WRITE_CURSOR_MS (Learn, IDirectSoundBuffer::GetCurrentPosition: "The write cursor always leads the play cursor, typically by about 15 milliseconds' worth of audio data"). It is NOT the
 * hand-over lead (FLE_AUDIO_LEAD_MS, S-audio-lead, 60 ms: how far audio_sync fills the output ahead of the play cursor); the two do not share a variable. A write between the two is still applied: rewrite_unread (S-ds-late-write) accepts a write down to a guard of 256 frames before the output's read position. */
#define DS_WRITE_CURSOR_MS 15u

/* the play and write cursors in bytes after a synchronisation */
static void cursors(ds_buf *b, uint32_t *play, uint32_t *write)
{
    uint32_t sf;
    uint64_t c, pl, wr;

    if (b->primary) {   /* S-ds-primary-cursors: the primary buffer has no data; both cursors are 0 */
        *play = *write = 0;
        return;
    }
    sf = size_frames(b);
    if (!b->playing) {   /* S-ds-stopped-write: a stopped buffer has committed nothing: the write cursor is the play cursor */
        *play = *write = b->cursor * b->fmt.align;
        return;
    }
    audio_sync();
    c = fle_audio_consumed();
    pl = abs_pos_at(b, c);
    {
        uint64_t vis = (uint64_t)b->fmt.rate * DS_WRITE_CURSOR_MS / 1000u, handed = b->pulled > c ? b->pulled - c : 0;   /* never beyond what was handed to the output */
        wr = abs_pos_at(b, c + (vis < handed ? vis : handed));
    }
    if (b->looping) {
        pl %= sf;
        wr %= sf;
    } else {
        if (pl > sf) pl = sf;
        if (wr > sf) wr = sf;
    }
    *play = (uint32_t)pl * b->fmt.align;
    *write = (uint32_t)wr * b->fmt.align;
}

static void read_tick(void)
{
    /* S-ds-pos-read-tick: reading the position moves the virtual clock by FLE_CLOCK_READ_TICKS, as every read of the clock does (decision 0017), so that a loop that only polls the position ends. */
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + FLE_CLOCK_READ_TICKS);
}

/* IDirectSoundBuffer::GetCurrentPosition(pdwPlay rdx, pdwWrite r8): both pointers may be NULL (Learn). */
static void m_b_getcurpos(void)
{
    ds_buf *b = this_buf(M_B_GETCURPOS, 0);
    uint32_t pl, wr;

    cursors(b, &pl, &wr);
    if (R[2]) fle_st(R[2], 4, pl);
    if (R[8]) fle_st(R[8], 4, wr);
    stats.getpos++;
    read_tick();
    ret(HR_OK);
}

/* IDirectSoundBuffer::GetFormat(pwfx rdx, dwSizeAllocated r8d, pdwSizeWritten r9): Learn: a NULL pwfx returns the needed size; a smaller allocation than the format is DSERR_INVALIDPARAM */
static void m_b_getformat(void)
{
    ds_buf *b = this_buf(M_B_GETFORMAT, 0);
    uint64_t p = R[2], pw = R[9];
    uint32_t alloc = (uint32_t)R[8], n = b->fmt.raw_len, i;

    if (p == 0) {
        if (pw == 0) {
            ret(HR_INVALIDPARAM);
            return;
        }
        fle_st(pw, 4, n);
        ret(HR_OK);
        return;
    }
    if (alloc < n) {
        ret(HR_INVALIDPARAM);
        return;
    }
    for (i = 0; i < n; i++) fle_st(p + i, 1, b->fmt.raw[i]);
    if (pw) fle_st(pw, 4, n);
    ret(HR_OK);
}

static void m_b_getstatus(void)
{
    ds_buf *b = this_buf(M_B_GETSTATUS, 0);
    uint64_t p = R[2];
    uint32_t st = 0;

    if (p == 0) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (b->playing) audio_sync();
    if (b->playing) st = DSBSTATUS_PLAYING | (b->looping ? DSBSTATUS_LOOPING : 0u);
    fle_st(p, 4, st);
    ret(HR_OK);
}

static void m_b_getvolume(void)
{
    ds_buf *b = this_buf(M_B_GETVOLUME, 0);

    if (R[2] == 0) { ret(HR_INVALIDPARAM); return; }
    if (!(b->flags & DSBCAPS_CTRLVOLUME) && !b->primary) { ret(0x8878001Eu); return; }   /* DSERR_CONTROLUNAVAIL */
    fle_st(R[2], 4, (uint32_t)b->vol);
    ret(HR_OK);
}

static void m_b_getpan(void)
{
    ds_buf *b = this_buf(M_B_GETPAN, 0);

    if (R[2] == 0) { ret(HR_INVALIDPARAM); return; }
    if (!(b->flags & DSBCAPS_CTRLPAN) && !b->primary) { ret(0x8878001Eu); return; }
    fle_st(R[2], 4, (uint32_t)b->pan);
    ret(HR_OK);
}

static void m_b_getfreq(void)
{
    ds_buf *b = this_buf(M_B_GETFREQ, 0);

    if (R[2] == 0) { ret(HR_INVALIDPARAM); return; }
    if (!(b->flags & DSBCAPS_CTRLFREQUENCY) || b->primary) { ret(0x8878001Eu); return; }
    fle_st(R[2], 4, b->freq ? b->freq : b->fmt.rate);
    ret(HR_OK);
}

static void m_b_setvolume(void)
{
    ds_buf *b = this_buf(M_B_SETVOLUME, 0);
    int32_t v = (int32_t)(uint32_t)R[2];

    if (!(b->flags & DSBCAPS_CTRLVOLUME)) { ret(0x8878001Eu); return; }   /* Learn: DSERR_CONTROLUNAVAIL; the control must be asked for when the buffer is made (DSERR_CONTROLUNAVAIL text) */
    if (v > 0 || v < DSBVOLUME_MIN) { ret(HR_INVALIDPARAM); return; }     /* Learn SetVolume: allowable values are DSBVOLUME_MAX (0) to DSBVOLUME_MIN (-10000) */
    b->vol = v;
    ret(HR_OK);
}

static void m_b_setpan(void)
{
    ds_buf *b = this_buf(M_B_SETPAN, 0);
    int32_t v = (int32_t)(uint32_t)R[2];

    if (!(b->flags & DSBCAPS_CTRLPAN)) { ret(0x8878001Eu); return; }
    if (v > DSBPAN_RIGHT || v < -DSBPAN_RIGHT) { ret(HR_INVALIDPARAM); return; }   /* Learn SetPan: DSBPAN_LEFT (-10000) to DSBPAN_RIGHT (10000) */
    b->pan = v;
    ret(HR_OK);
}

static void m_b_setfreq(void)
{
    ds_buf *b = this_buf(M_B_SETFREQ, 0);
    uint32_t f = (uint32_t)R[2];

    if (b->primary) { ret(HR_INVALIDCALL); return; }                       /* Learn SetFrequency: "not valid for the primary buffer" (S-ds-primary-call: DSERR_INVALIDCALL) */
    if (!(b->flags & DSBCAPS_CTRLFREQUENCY)) { ret(0x8878001Eu); return; }
    if (f != DSBFREQUENCY_ORIGINAL && (f < DSBFREQUENCY_MIN || f > DSBFREQUENCY_MAX)) { ret(HR_INVALIDPARAM); return; }
    if (f != DSBFREQUENCY_ORIGINAL && f != b->fmt.rate) fle_stop(mname[M_B_SETFREQ], "a frequency other than the buffer's own: resampling is not modeled (S-ds-no-resample)");
    b->freq = f;
    ret(HR_OK);
}

/* IDirectSoundBuffer::Lock(this, dwOffset edx, dwBytes r8d, ppvAudioPtr1 r9, pdwAudioBytes1 [5], ppvAudioPtr2 [6], pdwAudioBytes2 [7], dwFlags [8]) */
static void m_b_lock(void)
{
    ds_buf *b = this_buf(M_B_LOCK, 0);
    uint32_t off = (uint32_t)R[2], bytes = (uint32_t)R[8], flags = (uint32_t)fle_arg(8);
    uint64_t pp1 = R[9], pn1 = fle_arg(5), pp2 = fle_arg(6), pn2 = fle_arg(7);
    uint32_t n1, n2 = 0;
    uint64_t p2 = 0;

    if (b->primary) {   /* S-ds-primary-lock: the cooperative level DSSCL_WRITEPRIMARY (not modeled) is the only one at which the primary buffer is locked */
        ret(HR_INVALIDCALL);
        return;
    }
    if (pp1 == 0 || pn1 == 0 || (flags & ~(DSBLOCK_FROMWRITECURSOR | DSBLOCK_ENTIREBUFFER))) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (b->locked) {   /* S-ds-lock-twice */
        ret(HR_INVALIDCALL);
        return;
    }
    if (flags & DSBLOCK_ENTIREBUFFER) {
        off = 0;
        bytes = b->size;
    } else {
        if (flags & DSBLOCK_FROMWRITECURSOR) {   /* Learn Lock: "Start the lock at the write cursor. The dwOffset parameter is ignored." */
            uint32_t pl, wr;

            cursors(b, &pl, &wr);
            off = wr;
        }
        if (off >= b->size || bytes == 0 || bytes > b->size) {   /* S-ds-lock-range: an offset past the end, no bytes, or more than the buffer is DSERR_INVALIDPARAM (Learn: "the buffer is conceptually circular", bytes may exceed the distance to the end) */
            ret(HR_INVALIDPARAM);
            return;
        }
    }
    n1 = bytes < b->size - off ? bytes : b->size - off;
    if (n1 < bytes && pp2 != 0 && pn2 != 0) {   /* Learn: "If the application passes NULL for the ppvAudioPtr2 and pdwAudioBytes2 parameters, the lock extends no further than the end of the buffer and does not wrap." */
        p2 = (uint64_t)(uintptr_t)b->data;
        n2 = bytes - n1;
    }
    fle_st(pp1, 8, (uint64_t)(uintptr_t)(b->data + off));
    fle_st(pn1, 4, n1);
    if (pp2) fle_st(pp2, 8, p2);
    if (pn2) fle_st(pn2, 4, n2);
    b->locked = 1;
    b->lk_off = off;
    b->lk_n1 = n1;
    b->lk_n2 = n2;
    stats.locks++;
    ret(HR_OK);
}

/* IDirectSoundBuffer::Unlock(this, pvAudioPtr1 rdx, dwAudioBytes1 r8d, pvAudioPtr2 r9, dwAudioBytes2 [5]) */
static void m_b_unlock(void)
{
    ds_buf *b = this_buf(M_B_UNLOCK, 0);
    uint64_t p1 = R[2], p2 = R[9];
    uint32_t n1 = (uint32_t)R[8], n2 = (uint32_t)fle_arg(5);

    if (!b->locked) {
        ret(HR_INVALIDCALL);
        return;
    }
    /* Learn Unlock: both pointers of the Lock must be passed back; the byte counts "must specify the number of bytes actually written", which may be less than the lock (S-ds-unlock-check: more than the lock, or other pointers, is DSERR_INVALIDPARAM) */
    if (p1 != (uint64_t)(uintptr_t)(b->data + b->lk_off) || n1 > b->lk_n1 || (b->lk_n2 ? (p2 != (uint64_t)(uintptr_t)b->data || n2 > b->lk_n2) : (p2 != 0 || n2 != 0))) {
        ret(HR_INVALIDPARAM);
        return;
    }
    b->locked = 0;
    b->unlocks++;
    b->unlock_bytes += (uint64_t)n1 + n2;
    stats.unlocks++;
    stats.unlock_bytes += (uint64_t)n1 + n2;
    if (fle_audio_external() && b->playing) {   /* the page measures the delay from this write to the read of the region by the output: the first frame of the region is read when the consumed count reaches c + its distance ahead of the play cursor */
        uint32_t pl, wr;
        uint64_t sf = size_frames(b), c = fle_audio_consumed(), d, off_f = b->lk_off / b->fmt.align, wf0 = fle_audio_ring_get()->write_frames;

        cursors(b, &pl, &wr);
        d = (off_f + sf - (pl / b->fmt.align) % sf) % sf;
        fle_audio_unlock_note((double)(c + d), (n1 + n2) / b->fmt.align);
        rewrite_unread(b, (uint32_t)off_f, n1 / b->fmt.align, n2 / b->fmt.align, wf0);
    }
    ret(HR_OK);
}

static void restart_geometry(ds_buf *b, uint32_t pos_frames)
{
    fle_audio_ring_t *r = fle_audio_ring_get();
    uint64_t c = fle_audio_consumed();

    b->start_pos = pos_frames;
    b->base = r->write_frames > c ? r->write_frames : c;   /* frames already handed to the output are not changed */
    b->pulled = b->base;
    b->prev_abs = pos_frames;
}

/* IDirectSoundBuffer::Play(this, dwReserved1 edx, dwPriority r8d, dwFlags r9d) */
static void m_b_play(void)
{
    ds_buf *b = this_buf(M_B_PLAY, 0);
    uint32_t res = (uint32_t)R[2], prio = (uint32_t)R[8], flags = (uint32_t)R[9];

    if (res != 0 || (flags & ~(DSBPLAY_LOOPING | DSBPLAY_LOCHARDWARE | DSBPLAY_LOCSOFTWARE | DSBPLAY_TERMINATEBY_TIME | DSBPLAY_TERMINATEBY_DISTANCE | DSBPLAY_TERMINATEBY_PRIORITY))) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (prio != 0 || (flags & ~DSBPLAY_LOOPING)) {   /* Learn Play: dwPriority "must be 0" and the voice flags fail with DSERR_INVALIDPARAM unless the buffer was made with DSBCAPS_LOCDEFER (not modeled) */
        ret(HR_INVALIDPARAM);
        return;
    }
    if (b->primary) {
        if (!(flags & DSBPLAY_LOOPING)) {   /* Learn: "Primary buffers must be played with the DSBPLAY_LOOPING flag set." */
            ret(HR_INVALIDPARAM);
            return;
        }
        ret(HR_OK);   /* nothing to start: "the primary buffer plays silence if no secondary buffers are playing" */
        return;
    }
    if (b->playing) {   /* Learn: "the call succeeds and the buffer continues to play. However, the flags defined in the most recent call supersede" */
        b->looping = (flags & DSBPLAY_LOOPING) != 0;
        ret(HR_OK);
        return;
    }
    fle_audio_clock_start();
    if (play_count() == 0) {   /* the device was idle: the output clock ran on, silent, and the ring has nothing for that time (S-ds-idle-gap) */
        fle_audio_ring_t *rg = fle_audio_ring_get();
        uint64_t c0 = fle_audio_consumed();

        if (rg->write_frames < c0) rg->write_frames = c0;
    }
    b->looping = (flags & DSBPLAY_LOOPING) != 0;
    b->silent_until_coop = b->dev->coop == 0;
    b->playing = 1;
    restart_geometry(b, b->cursor);
    fle_audio_ring_set_state(FLE_AUDIO_PLAYING);
    stats.plays++;
    audio_sync();
    ret(HR_OK);
}

static void m_b_setcurpos(void)
{
    ds_buf *b = this_buf(M_B_SETCURPOS, 0);
    uint32_t pos = (uint32_t)R[2];

    if (b->primary) {   /* Learn SetCurrentPosition: "This method cannot be called on the primary buffer." (S-ds-primary-call: DSERR_INVALIDCALL) */
        ret(HR_INVALIDCALL);
        return;
    }
    if (pos >= b->size || pos % b->fmt.align) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (b->playing) {   /* Learn: "If the buffer is playing, the cursor immediately moves to the new position and play continues from that point." */
        audio_sync();
        restart_geometry(b, pos / b->fmt.align);
    } else {
        b->cursor = pos / b->fmt.align;
    }
    ret(HR_OK);
}

/* IDirectSoundBuffer::SetFormat(pcfxFormat rdx): the primary buffer only (Learn); needs DSSCL_PRIORITY or higher (Learn: it fails with DSSCL_NORMAL; S-ds-format-nolevel: no level yet is the same) */
static void m_b_setformat(void)
{
    ds_buf *b = this_buf(M_B_SETFORMAT, 0);
    wfmt f;
    int rf;

    if (!b->primary) {   /* Learn: "This method is not available for secondary sound buffers." (S-ds-secondary-setformat: DSERR_INVALIDCALL) */
        ret(HR_INVALIDCALL);
        return;
    }
    if (b->dev->coop < DSSCL_PRIORITY) {
        ret(HR_PRIOLEVELNEEDED);
        return;
    }
    rf = read_format(R[2], &f);
    if (rf == 0) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (rf < 0 || !format_ok(&f)) {
        ret(HR_BADFORMAT);
        return;
    }
    if (play_count() != 0 && f.rate != b->dev->fmt.rate) fle_stop(mname[M_B_SETFORMAT], "the device rate is changed while a buffer plays (S-ds-rate-change)");
    b->dev->fmt = f;
    b->fmt = f;
    fle_audio_ring_open(f.rate);
    ret(HR_OK);
}

static void m_b_stop(void)
{
    ds_buf *b = this_buf(M_B_STOP, 0);
    unsigned k;

    if (b->primary) {   /* Learn Stop: for the primary buffer without DSSCL_WRITEPRIMARY "simply reverses the effects of Play" */
        ret(HR_OK);
        return;
    }
    if (b->playing) {
        uint64_t sf = size_frames(b), at;

        audio_sync();
        /* S-ds-stop-drain: the frames already handed to the output (the lead) are played; the play cursor after Stop is the position that follows the last of them, so that a later Play neither repeats nor skips a frame
         * (Learn Stop: "sets the play cursor to the sample that follows the last sample played"). */
        at = abs_pos_at(b, b->pulled);
        b->cursor = (uint32_t)(b->looping ? at % sf : (at > sf ? sf : at));
        b->playing = 0;
        for (k = 0; k < b->nnotify; k++)
            if (b->notify[k].off == DSBPN_OFFSETSTOP) fle_event_host_set(b->notify[k].h);   /* Learn DSBPOSITIONNOTIFY: DSBPN_OFFSETSTOP signals when Stop is called */
        if (play_count() == 0) fle_audio_ring_set_state(FLE_AUDIO_OPEN);
        stats.stops++;
    }
    ret(HR_OK);
}

static void m_b_restore(void)
{
    this_buf(M_B_RESTORE, 0);
    ret(HR_OK);   /* S-ds-restore: no buffer is ever lost here, so there is nothing to restore (Learn: DSERR_INVALIDCALL is in the list; not used) */
}

static void m_b_getcaps(void)
{
    ds_buf *b = this_buf(M_B_GETCAPS, 0);
    uint64_t p = R[2];

    if (p == 0 || (uint32_t)fle_ld(p, 4) != 16u) {   /* DSBCAPS: dwSize, dwFlags, dwBufferBytes, dwUnlockTransferRate, dwPlayCpuOverhead (20 bytes in dsound.h) */
        if (p == 0 || (uint32_t)fle_ld(p, 4) != 20u) {
            ret(HR_INVALIDPARAM);
            return;
        }
    }
    fle_st(p + 4, 4, b->flags | DSBCAPS_LOCSOFTWARE);
    fle_st(p + 8, 4, b->size);
    fle_st(p + 12, 4, 0);
    fle_st(p + 16, 4, 0);
    ret(HR_OK);
}

/* ---- IDirectSoundNotify ---- */
static void m_n_qi(void)
{
    m_b_qi();
}

/* IDirectSoundNotify::SetNotificationPositions(this, dwPositionNotifies edx, pcPositionNotifies r8): Learn: "The buffer must be stopped when this method is called"; "If a position-notify array has already been set, the method replaces the previous array";
 * DSBPN_OFFSETSTOP "should be the last item"; failures are DSERR_INVALIDPARAM / DSERR_OUTOFMEMORY (the "stopped" violation is DSERR_INVALIDCALL: S-ds-notify-playing) */
static void m_n_setpos(void)
{
    ds_buf *b = this_buf(M_N_SETPOS, 1);
    uint32_t n = (uint32_t)R[2], k;
    uint64_t p = R[8];
    ds_notify_pos *a = NULL;

    if (b->playing) {
        ret(HR_INVALIDCALL);
        return;
    }
    if (n > DSBNOTIFICATIONS_MAX || (n != 0 && p == 0)) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (n) {
        a = calloc(n, sizeof *a);
        if (!a) {
            ret(HR_OUTOFMEMORY);
            return;
        }
    }
    for (k = 0; k < n; k++) {
        a[k].off = (uint32_t)fle_ld(p + 16u * k, 4);
        a[k].h = fle_ld(p + 16u * k + 8, 8);
        if ((a[k].off != DSBPN_OFFSETSTOP && (a[k].off >= b->size || a[k].off % b->fmt.align)) || fle_event_state(a[k].h) < 0 || (a[k].off == DSBPN_OFFSETSTOP && k + 1 != n)) {
            free(a);
            ret(HR_INVALIDPARAM);
            return;
        }
    }
    free(b->notify);
    b->notify = a;
    b->nnotify = n;
    ret(HR_OK);
}

/* ---- methods not modeled: stop by name ---- */
static void m_stop(unsigned id)
{
    char why[200];

    snprintf(why, sizeof why, "%s is not modeled (decision 0139: the engine does not call it; a call is a finding)", mname[id]);
    fle_stop(mname[id], why);
}

static void dispatch_slot(unsigned id)
{
    switch (id) {
    case M_DS_QI: m_ds_qi(); break;
    case M_DS_ADDREF: m_ds_addref(); break;
    case M_DS_RELEASE: m_ds_release(); break;
    case M_DS_CREATESOUNDBUFFER: m_ds_createsoundbuffer(); break;
    case M_DS_GETCAPS: m_ds_getcaps(); break;
    case M_DS_SETCOOP: m_ds_setcoop(); break;
    case M_B_QI: m_b_qi(); break;
    case M_B_ADDREF: m_b_addref(); break;
    case M_B_RELEASE: m_b_release(); break;
    case M_B_GETCAPS: m_b_getcaps(); break;
    case M_B_GETCURPOS: m_b_getcurpos(); break;
    case M_B_GETFORMAT: m_b_getformat(); break;
    case M_B_GETVOLUME: m_b_getvolume(); break;
    case M_B_GETPAN: m_b_getpan(); break;
    case M_B_GETFREQ: m_b_getfreq(); break;
    case M_B_GETSTATUS: m_b_getstatus(); break;
    case M_B_LOCK: m_b_lock(); break;
    case M_B_PLAY: m_b_play(); break;
    case M_B_SETCURPOS: m_b_setcurpos(); break;
    case M_B_SETFORMAT: m_b_setformat(); break;
    case M_B_SETVOLUME: m_b_setvolume(); break;
    case M_B_SETPAN: m_b_setpan(); break;
    case M_B_SETFREQ: m_b_setfreq(); break;
    case M_B_STOP: m_b_stop(); break;
    case M_B_UNLOCK: m_b_unlock(); break;
    case M_B_RESTORE: m_b_restore(); break;
    case M_N_QI: m_n_qi(); break;
    case M_N_ADDREF: m_b_addref(); break;
    case M_N_RELEASE: m_b_release(); break;
    case M_N_SETPOS: m_n_setpos(); break;
    default: m_stop(id); break;
    }
}

int fle_com_try_slot(uint64_t va)
{
    if (va < FLE_COM_SLOT_BASE || va >= FLE_COM_SLOT_BASE + FLE_COM_SLOT_STEP * (uint64_t)M_COUNT || ((va - FLE_COM_SLOT_BASE) % FLE_COM_SLOT_STEP) != 0) return 0;
    dispatch_slot((unsigned)((va - FLE_COM_SLOT_BASE) / FLE_COM_SLOT_STEP));
    return 1;
}

/* ---- the entry points of dsound.dll ---- */
/* dsound!DirectSoundEnumerateA / W: HRESULT f(LPDSENUMCALLBACK rcx, LPVOID rdx). Learn DirectSoundEnumerate: DS_OK, or DSERR_INVALIDPARAM (a NULL callback: the only failure the page names). The primary device is enumerated twice, first
 * with a NULL GUID (Learn DSEnumCallback); the callback returns TRUE to continue, FALSE to stop. */
static void enumerate(const char *api, int wide)
{
    uint64_t cb = R[1], ctx = R[2], guid = (uint64_t)(uintptr_t)DEV_GUID, d1, d2, m;
    uint64_t r;

    if (cb == 0) {
        ret(HR_INVALIDPARAM);
        return;
    }
    if (fle_call_guest == NULL) fle_stop(api, "the guest callback mechanism is not linked: the callback cannot be called");
    d1 = (uint64_t)(uintptr_t)(wide ? (const void *)DESC_PRIMARY_W : (const void *)DESC_PRIMARY_A);
    d2 = (uint64_t)(uintptr_t)(wide ? (const void *)DESC_DEVICE_W : (const void *)DESC_DEVICE_A);
    m = (uint64_t)(uintptr_t)(wide ? (const void *)MODULE_W : (const void *)MODULE_A);
    stats.enumerations++;
    r = fle_call_guest(cb, 0, d1, m, ctx);
    if ((uint32_t)r != 0) fle_call_guest(cb, guid, d2, m, ctx);
    ret(HR_OK);
}

void fle_imp_DirectSoundEnumerateA(void) { enumerate("DirectSoundEnumerateA", 0); }
void fle_imp_DirectSoundEnumerateW(void) { enumerate("DirectSoundEnumerateW", 1); }

/* dsound!DirectSoundCreate / DirectSoundCreate8: HRESULT f(LPCGUID rcx, LPDIRECTSOUND* rdx, LPUNKNOWN r8). Learn DirectSoundCreate8: the GUID is one the enumeration gave, NULL, DSDEVID_DefaultPlayback (= NULL) or
 * DSDEVID_DefaultVoicePlayback; errors DSERR_ALLOCATED, INVALIDPARAM, NOAGGREGATION, NODRIVER ("or the given GUID is not a valid DirectSound device ID"), OUTOFMEMORY; pUnkOuter "Must be NULL, because aggregation is not supported". */
static void create(const char *api, int ds8)
{
    uint64_t g = R[1], pp = R[2], outer = R[8];
    ds_dev *d;
    unsigned i;

    if (pp == 0) {
        ret(HR_INVALIDPARAM);
        return;
    }
    fle_st(pp, 8, 0);   /* S-ds-fail-null: *ppDS is NULL on failure (the COM convention for an out pointer) */
    if (outer != 0) {
        ret(HR_NOAGGREGATION);
        return;
    }
    if (g != 0 && !guid_eq(g, DEV_GUID) && !guid_eq(g, DEV_DEFAULT) && !guid_eq(g, DEV_VOICE)) {
        ret(HR_NODRIVER);
        return;
    }
    for (i = 0; i < MAX_DEV; i++)
        if (!devs[i]) break;
    if (i == MAX_DEV) fle_stop(api, "more DirectSound objects than the model holds (8)");
    d = calloc(1, sizeof *d);
    if (!d) {
        ret(HR_OUTOFMEMORY);
        return;
    }
    vt_init();
    d->vtbl = (uint64_t)(uintptr_t)(ds8 ? vt_ds : vt_ds_v1);
    d->refs = 1;
    d->live = 1;
    d->ds8 = ds8;
    /* S-ds-primary-default: the device format before any SetFormat is 44100 Hz, 16 bit, stereo PCM (Learn SetCooperativeLevel: under DSSCL_NORMAL the output is "the default 8-bit format"; the value is a choice) */
    d->fmt.tag = WAVE_FORMAT_PCM_;
    d->fmt.channels = 2;
    d->fmt.rate = 44100;
    d->fmt.bits = 16;
    d->fmt.align = 4;
    d->fmt.raw_len = 18;
    d->fmt.raw[0] = 1; d->fmt.raw[2] = 2; d->fmt.raw[4] = 0x44; d->fmt.raw[5] = 0xAC; d->fmt.raw[8] = 0x10; d->fmt.raw[9] = 0xB1; d->fmt.raw[12] = 4; d->fmt.raw[14] = 16;
    devs[i] = d;
    fle_audio_ring_open(d->fmt.rate);
    fle_st(pp, 8, (uint64_t)(uintptr_t)d);
    stats.devices_created++;
    ret(HR_OK);
}

void fle_imp_DirectSoundCreate(void) { create("DirectSoundCreate", 0); }
void fle_imp_DirectSoundCreate8(void) { create("DirectSoundCreate8", 1); }
