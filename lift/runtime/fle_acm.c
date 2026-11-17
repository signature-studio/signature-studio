/* Decision 0229 (ref1, PROVISIONAL, OPT-IN through the rows of harness/seeds/ref1-imports.txt; without them nothing here is reached): msacm32, the Audio Compression Manager.
 *
 * What is modelled: ONE installed ACM driver, an MPEG Layer-3 DECODER (MPEGLAYER3WAVEFORMAT, tag 0x55 -> 16-bit PCM of the same channel count and sample rate), which is what a Windows machine with
 * l3codeca.acm / l3codecp.acm offers to an application that opens an .mp3 file (SmpLoader_MP3 of FLEngine: acmDriverEnum -> acmDriverOpen -> acmFormatSuggest -> acmStreamOpen -> acmStreamSize ->
 * acmStreamPrepareHeader -> acmStreamConvert ...). Other installed drivers of a real machine (the PCM converter, ADPCM, GSM, G.711 ...) are NOT modelled: a conversion they would do is ACMERR_NOTPOSSIBLE here
 * (S-ref1-acm-one-driver). The decoder is minimp3 (CC0, runtime/third_party/minimp3, not modified).
 *
 * Contracts (Microsoft Learn, MicrosoftDocs/sdk-api msacm/nf-msacm-acm*.md, fetched 2026-11-04): acmDriverEnum (callback BOOL(HACMDRIVERID, DWORD_PTR, DWORD fdwSupport); no drivers: returns 0, no callback), acmDriverOpen,
 * acmDriverClose, acmFormatSuggest (ACM_FORMATSUGGESTF_* say which members of the destination are valid; ACMERR_NOTPOSSIBLE when the driver cannot satisfy them), acmStreamOpen (ACMERR_NOTPOSSIBLE,
 * MMSYSERR_INVALFLAG / INVALHANDLE / INVALPARAM / NOMEM; ACM_STREAMOPENF_QUERY opens nothing), acmStreamSize (the recommended size, usually an over-estimate, never zero on success; zero on failure),
 * acmStreamPrepareHeader / acmStreamUnprepareHeader (ACMSTREAMHEADER_STATUSF_PREPARED; preparing a prepared header returns 0), acmStreamConvert (ACMERR_UNPREPARED, ACMERR_BUSY; cbSrcLengthUsed and
 * cbDstLengthUsed; START re-initialises the instance data), acmStreamClose. What the pages do not define is a named stop (fle_stop), never an invented answer (acmStreamMessage, a non-MP3 source in a call that
 * has no failure code on its page).
 * The numbers (the pages name codes, not values; the values are those of the SDK headers, read in mingw-w64's mmsyscom.h and msacm.h): MMSYSERR_BASE 0 + INVALHANDLE 5, NOMEM 7, INVALFLAG 10, INVALPARAM 11; ACMERR_BASE 512 + NOTPOSSIBLE 0,
 * BUSY 1, UNPREPARED 2; ACM_STREAMSIZEF_SOURCE 0 / DESTINATION 1 / QUERYMASK 0xF; ACMSTREAMHEADER_STATUSF_DONE 0x10000 / PREPARED 0x20000 / INQUEUE 0x100000; ACM_FORMATSUGGESTF_* 0x10000..0x80000; ACM_DRIVERENUMF_NOLOCAL 0x40000000 / DISABLED 0x80000000. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_acm.h"

#define MINIMP3_IMPLEMENTATION
#define MINIMP3_ONLY_MP3
#define MINIMP3_NO_SIMD
#include "third_party/minimp3/minimp3.h"

#define MMSYSERR_INVALHANDLE 5u
#define MMSYSERR_NOMEM 7u
#define MMSYSERR_INVALFLAG 10u
#define MMSYSERR_INVALPARAM 11u
#define ACMERR_NOTPOSSIBLE 512u
#define ACMERR_BUSY 513u
#define ACMERR_UNPREPARED 514u

#define WAVE_FORMAT_PCM 1u
#define WAVE_FORMAT_MPEGLAYER3 0x55u

#define ACM_STREAMOPENF_QUERY 0x00000001u
#define ACM_STREAMOPENF_ASYNC 0x00000002u
#define ACM_STREAMOPENF_NONREALTIME 0x00000004u
#define ACM_STREAMCONVERTF_BLOCKALIGN 0x00000004u
#define ACM_STREAMCONVERTF_START 0x00000010u
#define ACM_STREAMCONVERTF_END 0x00000020u
#define ACMSTREAMHEADER_STATUSF_DONE 0x00010000u
#define ACMSTREAMHEADER_STATUSF_PREPARED 0x00020000u
#define ACMSTREAMHEADER_STATUSF_INQUEUE 0x00100000u
#define ACMDRIVERDETAILS_SUPPORTF_CODEC 0x00000001u

#define H_DRIVERID 0x41434D01u
#define H_DRIVER0 0x41434D10u
#define H_STREAM0 0x41434D80u
#define MAX_DRIVER_OPEN 16
#define MAX_STREAMS 64

typedef struct {
    int used;
    unsigned channels, rate, spf;   /* spf = samples per frame and channel: 1152 (MPEG-1), 576 (MPEG-2 / 2.5) */
    mp3dec_t dec;
    uint64_t converted_src, converted_dst;
} stream_t;

static int drv_open[MAX_DRIVER_OPEN];
static stream_t streams[MAX_STREAMS];

static void *gp(uint64_t a) { return (void *)(uintptr_t)a; }

/* WAVEFORMATEX at guest address a: tag(2) channels(2) rate(4) avgBytes(4) blockAlign(2) bits(2) cbSize(2) */
typedef struct { uint32_t tag, ch, rate, avg, align, bits, cb; } wfx_t;
static void rd_wfx(uint64_t a, wfx_t *w, unsigned avail_ok_cb)
{
    w->tag = (uint32_t)fle_ld(a, 2); w->ch = (uint32_t)fle_ld(a + 2, 2); w->rate = (uint32_t)fle_ld(a + 4, 4); w->avg = (uint32_t)fle_ld(a + 8, 4);
    w->align = (uint32_t)fle_ld(a + 12, 2); w->bits = (uint32_t)fle_ld(a + 14, 2);
    w->cb = avail_ok_cb ? (uint32_t)fle_ld(a + 16, 2) : 0;
}
static int rate_ok(uint32_t r) { return r == 8000 || r == 11025 || r == 12000 || r == 16000 || r == 22050 || r == 24000 || r == 32000 || r == 44100 || r == 48000; }
static unsigned spf_of(uint32_t rate) { return rate >= 32000 ? 1152u : 576u; }

/* the one conversion the driver offers: MPEG Layer-3 (tag 0x55, WAVEFORMATEX + 12 bytes of MPEGLAYER3WAVEFORMAT) -> 16-bit PCM, same channels and rate */
static int src_is_mp3(uint64_t pwfx)
{
    wfx_t w;
    if (!pwfx) return 0;
    rd_wfx(pwfx, &w, 1);
    return w.tag == WAVE_FORMAT_MPEGLAYER3 && (w.ch == 1 || w.ch == 2) && rate_ok(w.rate) && w.cb >= 12;
}
static int dst_matches(uint64_t psrc, uint64_t pdst)
{
    wfx_t s, d;
    if (!pdst) return 0;
    rd_wfx(psrc, &s, 1);
    rd_wfx(pdst, &d, 0);
    return d.tag == WAVE_FORMAT_PCM && d.ch == s.ch && d.rate == s.rate && d.bits == 16 && d.align == d.ch * 2u && d.avg == d.rate * d.align;
}

static int had_ok(uint64_t had) { return had >= H_DRIVER0 && had < H_DRIVER0 + MAX_DRIVER_OPEN && drv_open[had - H_DRIVER0]; }

void fle_imp_acm_DriverEnum(void)
{
    uint64_t fn = R[1], inst = R[2];
    uint32_t fl = (uint32_t)R[8];
    if (fl & ~0xC0000000u) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (fn == 0) { R[0] = MMSYSERR_INVALPARAM; return; }
    (void)fle_call_guest(fn, H_DRIVERID, inst, ACMDRIVERDETAILS_SUPPORTF_CODEC, 0);
    R[0] = 0;
}

void fle_imp_acm_DriverOpen(void)
{
    uint64_t phad = R[1], hadid = R[2];
    uint32_t fl = (uint32_t)R[8];
    int i;
    if (fl) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!phad) { R[0] = MMSYSERR_INVALPARAM; return; }
    if (hadid != H_DRIVERID) { R[0] = MMSYSERR_INVALHANDLE; return; }
    for (i = 0; i < MAX_DRIVER_OPEN; i++) if (!drv_open[i]) break;
    if (i == MAX_DRIVER_OPEN) { R[0] = MMSYSERR_NOMEM; return; }
    drv_open[i] = 1;
    fle_st(phad, 8, H_DRIVER0 + (uint64_t)i);
    R[0] = 0;
}

void fle_imp_acm_DriverClose(void)
{
    uint64_t had = R[1];
    uint32_t fl = (uint32_t)R[2];
    if (fl) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!had_ok(had)) { R[0] = MMSYSERR_INVALHANDLE; return; }
    drv_open[had - H_DRIVER0] = 0;
    R[0] = 0;
}

void fle_imp_acm_FormatSuggest(void)
{
    static const char api[] = "acmFormatSuggest";
    uint64_t had = R[1], psrc = R[2], pdst = R[8], cbdst = R[9];
    uint32_t fl = (uint32_t)fle_arg(5);
    wfx_t s, d;
    if (fl & ~0x00FF0000u) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!psrc || !pdst || (uint32_t)cbdst < 16) { R[0] = MMSYSERR_INVALPARAM; return; }
    if (had != 0 && !had_ok(had)) { R[0] = MMSYSERR_INVALHANDLE; return; }
    if (!src_is_mp3(psrc)) { R[0] = ACMERR_NOTPOSSIBLE; return; }
    rd_wfx(psrc, &s, 1);
    memset(&d, 0, sizeof d);
    rd_wfx(pdst, &d, (uint32_t)cbdst >= 18);
    /* ACM_FORMATSUGGESTF_*: WFORMATTAG 0x10000, NCHANNELS 0x20000, NSAMPLESPERSEC 0x40000, WBITSPERSAMPLE 0x80000: the flagged members of the destination are valid and must be satisfiable */
    if ((fl & 0x10000u) && d.tag != WAVE_FORMAT_PCM) { R[0] = ACMERR_NOTPOSSIBLE; return; }
    if ((fl & 0x20000u) && d.ch != s.ch) { R[0] = ACMERR_NOTPOSSIBLE; return; }
    if ((fl & 0x40000u) && d.rate != s.rate) { R[0] = ACMERR_NOTPOSSIBLE; return; }
    if ((fl & 0x80000u) && d.bits != 16) { R[0] = ACMERR_NOTPOSSIBLE; return; }
    fle_st(pdst, 2, WAVE_FORMAT_PCM); fle_st(pdst + 2, 2, s.ch); fle_st(pdst + 4, 4, s.rate); fle_st(pdst + 8, 4, s.rate * s.ch * 2u);
    fle_st(pdst + 12, 2, s.ch * 2u); fle_st(pdst + 14, 2, 16);
    if ((uint32_t)cbdst >= 18) fle_st(pdst + 16, 2, 0);
    (void)api;
    R[0] = 0;
}

void fle_imp_acm_StreamOpen(void)
{
    uint64_t phas = R[1], had = R[2], psrc = R[8], pdst = R[9], pfltr = fle_arg(5);
    uint32_t fl = (uint32_t)fle_arg(8);
    int i;
    if (fl & ~(ACM_STREAMOPENF_QUERY | ACM_STREAMOPENF_ASYNC | ACM_STREAMOPENF_NONREALTIME | 0x00070000u)) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!psrc || !pdst || (!phas && !(fl & ACM_STREAMOPENF_QUERY))) { R[0] = MMSYSERR_INVALPARAM; return; }
    if (had != 0 && !had_ok(had)) { R[0] = MMSYSERR_INVALHANDLE; return; }
    if (pfltr || (fl & ACM_STREAMOPENF_ASYNC)) { R[0] = ACMERR_NOTPOSSIBLE; return; }   /* the decoder has no filter and is synchronous (S-ref1-acm-one-driver) */
    if (!src_is_mp3(psrc) || !dst_matches(psrc, pdst)) { R[0] = ACMERR_NOTPOSSIBLE; return; }
    if (fl & ACM_STREAMOPENF_QUERY) { R[0] = 0; return; }
    for (i = 0; i < MAX_STREAMS; i++) if (!streams[i].used) break;
    if (i == MAX_STREAMS) { R[0] = MMSYSERR_NOMEM; return; }
    {
        wfx_t s;
        rd_wfx(psrc, &s, 1);
        memset(&streams[i], 0, sizeof streams[i]);
        streams[i].used = 1; streams[i].channels = s.ch; streams[i].rate = s.rate; streams[i].spf = spf_of(s.rate);
        mp3dec_init(&streams[i].dec);
    }
    fle_st(phas, 8, H_STREAM0 + (uint64_t)i);
    R[0] = 0;
}

static stream_t *get_stream(uint64_t has) { return (has >= H_STREAM0 && has < H_STREAM0 + MAX_STREAMS && streams[has - H_STREAM0].used) ? &streams[has - H_STREAM0] : NULL; }

void fle_imp_acm_StreamClose(void)
{
    uint64_t has = R[1];
    uint32_t fl = (uint32_t)R[2];
    stream_t *st = get_stream(has);
    if (fl) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!st) { R[0] = MMSYSERR_INVALHANDLE; return; }
    st->used = 0;
    R[0] = 0;
}

/* The recommended sizes. A Layer-3 frame (1152 samples per channel at 32/44.1/48 kHz, 576 below) decodes to spf*channels*2 bytes. From a source size: the number of frames the source can hold at the SMALLEST frame the
 * stream's sample rate allows (the lowest bit rate of the format: 32 kbit/s MPEG-1, 8 kbit/s MPEG-2 and 2.5: 144000*32/rate resp. 72*8000/rate bytes), rounded up: an over-estimate, as the page says. From a
 * destination size: the whole frames that fit, times the LARGEST frame (320 kbit/s MPEG-1: 144*320000/rate + 1; 160 kbit/s MPEG-2 and 2.5: 72*160000/rate + 1), i.e. the source that is certain to decode into the
 * destination (S-ref1-acm-size-bound: the pages give no formula; the bounds are the format's, the rounding is a choice). */
void fle_imp_acm_StreamSize(void)
{
    uint64_t has = R[1], cb = R[2], pout = R[8];
    uint32_t fl = (uint32_t)R[9];
    stream_t *st = get_stream(has);
    uint64_t frame_pcm, out;
    if (fl & ~0x0000000Fu) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!pout || (uint32_t)cb == 0) { R[0] = MMSYSERR_INVALPARAM; return; }
    if (!st) { R[0] = MMSYSERR_INVALHANDLE; return; }
    frame_pcm = (uint64_t)st->spf * st->channels * 2u;
    if ((fl & 0xFu) == 0x00000000u) {   /* ACM_STREAMSIZEF_SOURCE (0): cbInput is a source size, the output the destination size */
        uint64_t minf = st->rate >= 32000 ? 144ull * 32000u / st->rate : 72ull * 8000u / st->rate;
        uint64_t frames = (cb + minf - 1u) / minf;
        out = frames * frame_pcm;
    } else if ((fl & 0xFu) == 0x00000001u) {   /* ACM_STREAMSIZEF_DESTINATION (1): cbInput is a destination size, the output the source size */
        uint64_t frames = cb / frame_pcm;
        out = frames * ((st->rate >= 32000 ? 144ull * 320000u : 72ull * 160000u) / st->rate + 1u);
        if (frames == 0) { fle_st(pout, 4, 0); R[0] = ACMERR_NOTPOSSIBLE; return; }
    } else { R[0] = MMSYSERR_INVALFLAG; return; }
    if (out > 0xFFFFFFFFu) out = 0xFFFFFFFFu;
    fle_st(pout, 4, out);
    R[0] = 0;
}

/* ACMSTREAMHEADER (x64): cbStruct u32 +0, fdwStatus u32 +4, dwUser +8, pbSrc +16, cbSrcLength u32 +24, cbSrcLengthUsed u32 +28, dwSrcUser +32, pbDst +40, cbDstLength u32 +48, cbDstLengthUsed u32 +52, dwDstUser +56 */
#define SH_MIN 104u   /* sizeof(ACMSTREAMHEADER) on x64: 64 + 10 DWORDs of dwReservedDriver (mingw-w64 msacm.h, _DRVRESERVED) */

void fle_imp_acm_StreamPrepareHeader(void)
{
    uint64_t has = R[1], pash = R[2];
    uint32_t fl = (uint32_t)R[8], st_;
    stream_t *st = get_stream(has);
    if (fl) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!pash || (uint32_t)fle_ld(pash, 4) < SH_MIN) { R[0] = MMSYSERR_INVALPARAM; return; }
    if (!st) { R[0] = MMSYSERR_INVALHANDLE; return; }
    st_ = (uint32_t)fle_ld(pash + 4, 4);
    if (st_ & ACMSTREAMHEADER_STATUSF_PREPARED) { R[0] = 0; return; }
    if (!fle_ld(pash + 16, 8) || !fle_ld(pash + 40, 8) || !(uint32_t)fle_ld(pash + 24, 4) || !(uint32_t)fle_ld(pash + 48, 4)) { R[0] = MMSYSERR_INVALPARAM; return; }
    fle_st(pash + 4, 4, st_ | ACMSTREAMHEADER_STATUSF_PREPARED);
    R[0] = 0;
}

void fle_imp_acm_StreamUnprepareHeader(void)
{
    uint64_t has = R[1], pash = R[2];
    uint32_t fl = (uint32_t)R[8], st_;
    stream_t *st = get_stream(has);
    if (fl) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!pash || (uint32_t)fle_ld(pash, 4) < SH_MIN) { R[0] = MMSYSERR_INVALPARAM; return; }
    if (!st) { R[0] = MMSYSERR_INVALHANDLE; return; }
    st_ = (uint32_t)fle_ld(pash + 4, 4);
    if (st_ & ACMSTREAMHEADER_STATUSF_INQUEUE) { R[0] = ACMERR_BUSY; return; }
    if (!(st_ & ACMSTREAMHEADER_STATUSF_PREPARED)) { R[0] = ACMERR_UNPREPARED; return; }
    fle_st(pash + 4, 4, st_ & ~(ACMSTREAMHEADER_STATUSF_PREPARED | ACMSTREAMHEADER_STATUSF_DONE));
    R[0] = 0;
}

/* header of a Layer-3 frame at p (n bytes available): returns the frame length (with padding), 0 when p is no valid Layer-3 header or the bit rate is free-format / bad */
static unsigned l3_frame_len(const uint8_t *p, size_t n, unsigned *rate_out, unsigned *ch_out)
{
    static const uint16_t br1[16] = { 0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0 };
    static const uint16_t br2[16] = { 0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0 };
    static const uint32_t sr1[4] = { 44100, 48000, 32000, 0 };
    unsigned ver, layer, bi, si, pad, br, rate;
    if (n < 4 || p[0] != 0xFF || (p[1] & 0xE0) != 0xE0) return 0;
    ver = (p[1] >> 3) & 3u; layer = (p[1] >> 1) & 3u;
    if (ver == 1 || layer != 1) return 0;   /* 01 reserved; layer bits 01 = Layer III */
    bi = p[2] >> 4; si = (p[2] >> 2) & 3u; pad = (p[2] >> 1) & 1u;
    if (si == 3 || bi == 0 || bi == 15) return 0;
    rate = sr1[si]; if (ver == 2) rate /= 2; else if (ver == 0) rate /= 4;
    *rate_out = rate; *ch_out = ((p[3] >> 6) == 3) ? 1u : 2u;
    br = (ver == 3 ? br1[bi] : br2[bi]) * 1000u;
    return ver == 3 ? (144u * br / rate + pad) : (72u * br / rate + pad);
}

void fle_imp_acm_StreamConvert(void)
{
    static const char api[] = "acmStreamConvert";
    uint64_t has = R[1], pash = R[2];
    uint32_t fl = (uint32_t)R[8], st_;
    stream_t *st = get_stream(has);
    uint64_t psrc, pdst;
    uint32_t slen, dlen, used_s = 0, used_d = 0;
    if (fl & ~(ACM_STREAMCONVERTF_BLOCKALIGN | ACM_STREAMCONVERTF_START | ACM_STREAMCONVERTF_END)) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!pash || (uint32_t)fle_ld(pash, 4) < SH_MIN) { R[0] = MMSYSERR_INVALPARAM; return; }
    if (!st) { R[0] = MMSYSERR_INVALHANDLE; return; }
    st_ = (uint32_t)fle_ld(pash + 4, 4);
    if (!(st_ & ACMSTREAMHEADER_STATUSF_PREPARED)) { R[0] = ACMERR_UNPREPARED; return; }
    if (st_ & ACMSTREAMHEADER_STATUSF_INQUEUE) { R[0] = ACMERR_BUSY; return; }
    psrc = fle_ld(pash + 16, 8); slen = (uint32_t)fle_ld(pash + 24, 4);
    pdst = fle_ld(pash + 40, 8); dlen = (uint32_t)fle_ld(pash + 48, 4);
    if (fl & ACM_STREAMCONVERTF_START) mp3dec_init(&st->dec);
    while (used_s < slen) {
        const uint8_t *p = (const uint8_t *)gp(psrc) + used_s;
        unsigned frate = 0, fch = 0, flen = 0;
        size_t left = slen - used_s;
        short pcm[MINIMP3_MAX_SAMPLES_PER_FRAME];
        mp3dec_frame_info_t info;
        int ns;
        uint32_t need = st->spf * st->channels * 2u;
        flen = l3_frame_len(p, left, &frate, &fch);
        if (!flen) { used_s++; continue; }   /* not at a frame header (junk, a tag): the bytes are consumed without output */
        if (flen > left) break;   /* a partial frame at the end of the source: not consumed */
        if (dlen - used_d < need) break;
        if (frate != st->rate) fle_stop(api, "a Layer-3 frame has a sample rate other than the one of the stream's source format: the pages define no conversion");
        if (fch != st->channels) fle_stop(api, "a Layer-3 frame has a channel mode other than the one of the stream's source format (mono vs stereo): the pages define no conversion");
        memset(&info, 0, sizeof info);
        ns = mp3dec_decode_frame(&st->dec, p, (int)flen, pcm, &info);
        if (ns > 0) {
            if ((unsigned)ns != st->spf) fle_stop(api, "minimp3 returned a sample count that is not the frame's");
            memcpy((uint8_t *)gp(pdst) + used_d, pcm, need);
        } else {
            memset((uint8_t *)gp(pdst) + used_d, 0, need);   /* S-ref1-acm-reservoir-silence: a frame that cannot be decoded (its bit reservoir is not there yet) gives a frame of silence, one frame in = one frame out */
        }
        used_d += need; used_s += flen;
    }
    fle_st(pash + 28, 4, used_s);
    fle_st(pash + 52, 4, used_d);
    fle_st(pash + 4, 4, st_ | ACMSTREAMHEADER_STATUSF_DONE);
    st->converted_src += used_s; st->converted_dst += used_d;
    R[0] = 0;
}

/* acmDriverDetailsA (Learn: nf-msacm-acmdriverdetails.md, ns-msacm-acmdriverdetails.md). ACMDRIVERDETAILSA on x64: cbStruct u32 +0, fccType +4, fccComp +8, wMid u16 +12, wPid u16 +14, vdwACM +16, vdwDriver +20, fdwSupport +24,
 * cFormatTags +28, cFilterTags +32, hicon (8, aligned) +40, szShortName[32] +48, szLongName[128] +80, szCopyright[80] +208, szLicensing[128] +288, szFeatures[512] +416; 928 bytes (field sizes: mingw-w64 msacm.h).
 * The values describe THIS environment's one driver. wMid 172 (MM_FRAUNHOFER_IIS) and wPid 9 (MM_FHGIIS_MPEGLAYER3_DECODE) are mmreg.h's; the version numbers, the strings and cFormatTags are CHOICES (S-ref1-acm-details): a real
 * machine's l3codeca.acm has its own strings and versions, which this work could not read. "the returned information will never exceed the requested size": only the first cbStruct bytes are written. */
#define DD_SIZE 928u
void fle_imp_acm_DriverDetailsA(void)
{
    uint64_t hadid = R[1], padd = R[2];
    uint32_t fl = (uint32_t)R[8], cb;
    uint8_t buf[DD_SIZE];
    if (fl) { R[0] = MMSYSERR_INVALFLAG; return; }
    if (!padd) { R[0] = MMSYSERR_INVALPARAM; return; }
    if (hadid != H_DRIVERID) { R[0] = MMSYSERR_INVALHANDLE; return; }
    cb = (uint32_t)fle_ld(padd, 4);
    if (cb < 4) { R[0] = MMSYSERR_INVALPARAM; return; }
    if (cb > DD_SIZE) cb = DD_SIZE;
    memset(buf, 0, sizeof buf);
#define PUT(off, n, v) do { uint64_t v_ = (v); memcpy(buf + (off), &v_, (n)); } while (0)
    PUT(0, 4, cb); PUT(4, 4, 0x63647561u /* 'audc' = ACMDRIVERDETAILS_FCCTYPE_AUDIOCODEC (mmioFOURCC) */); PUT(8, 4, 0); PUT(12, 2, 172); PUT(14, 2, 9);
    PUT(16, 4, 0x04030000u); PUT(20, 4, 0x01000000u); PUT(24, 4, ACMDRIVERDETAILS_SUPPORTF_CODEC); PUT(28, 4, 2); PUT(32, 4, 0); PUT(40, 8, 0);
    snprintf((char *)buf + 48, 32, "%s", "MPEG Layer-3");
    snprintf((char *)buf + 80, 128, "%s", "Fraunhofer IIS MPEG Layer-3 Codec (decode only)");
    snprintf((char *)buf + 208, 80, "%s", "Fraunhofer IIS");
    memcpy(gp(padd), buf, cb);
    R[0] = 0;
}

/* acmFormatChooseW shows a modal dialog (ACMFORMATCHOOSEW): the dialog is not modelled; the engine's export of a compressed format would reach it. */
void fle_imp_acm_FormatChooseW(void)
{
    fle_stop("acmFormatChooseW", "the format-choice dialog of the Audio Compression Manager is not modelled (S-ref1-acm-no-dialog)");
}

void fle_imp_acm_StreamMessage(void)
{
    fle_stop("acmStreamMessage", "the page: 'returns the value returned by the ACM device driver'; this environment's one driver (an MPEG Layer-3 decoder) defines no driver-specific message");
}
