/* Decision 0144 (PROVISIONAL): the G8 probe (files and audio). A harness-side observation of the guest's file reads, for the stage record of "open a project" (G8a-0 ... G8c). Linked ONLY by a build that passes
 * -Wl,--wrap=fle_imp_CreateFileW -Wl,--wrap=fle_imp_ReadFile -Wl,--wrap=fle_imp_CreateFileMappingW -Wl,--wrap=fle_imp_MapViewOfFile (lift/harness/g8/link-flags.txt); no default build contains it.
 * Run time: FLE_G8_FILETRACE=1 prints one "g8-file:" line per open, read, mapping and view on stderr and the per-file totals at exit; unset or 0 = the wrappers call the real function and print nothing.
 * The audio part (-Wl,--wrap=fle_audio_ring_append): FLE_G8_AUDIOTRACE=1 prints one "g8-audio:" line per append to the audio ring (decision 0139: the DirectSound device's pull) with its first frame, the virtual
 * time, the largest |sample|, the number of frames with a non-zero sample and the SHA-256 of the float bytes, and a summary at exit; FLE_G8_AUDIODUMP=<file> also writes the appended frames (interleaved stereo float32).
 * The wrappers read the arguments from R[] before and the result after the real call (fle_imports.h convention) and change nothing the guest sees. Single host thread (decision 0122): no lock. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_audio_ring.h"
#include "fle_runtime.h"
#include "fle_sys.h"

extern uint64_t trace_call_count(void) __attribute__((weak));

void __real_fle_imp_CreateFileW(void);
void __real_fle_imp_ReadFile(void);
void __real_fle_imp_CreateFileMappingW(void);
void __real_fle_imp_MapViewOfFile(void);

#define G8_MAX 1024
typedef struct {
    uint64_t handle;
    char path[260];
    uint64_t opens, reads, read_bytes, maps, mapped_bytes;
} g8_file;

static g8_file files[G8_MAX];
static unsigned nfiles;
static int on = -1;
static unsigned seq;
static uint64_t mapping_h[G8_MAX], mapping_file[G8_MAX];
static unsigned nmappings;

static void totals(void)
{
    uint64_t rb = 0, mb = 0;
    unsigned i;

    for (i = 0; i < nfiles; i++) {
        rb += files[i].read_bytes;
        mb += files[i].mapped_bytes;
        fprintf(stderr, "g8-file-total: opens %" PRIu64 " reads %" PRIu64 " read_bytes %" PRIu64 " maps %" PRIu64 " mapped_bytes %" PRIu64 " path %s\n", files[i].opens, files[i].reads, files[i].read_bytes,
                files[i].maps, files[i].mapped_bytes, files[i].path);
    }
    fprintf(stderr, "g8-file-sum: files %u read_bytes %" PRIu64 " mapped_bytes %" PRIu64 "\n", nfiles, rb, mb);
}

static int enabled(void)
{
    if (on < 0) {
        const char *e = getenv("FLE_G8_FILETRACE");
        on = e && e[0] && e[0] != '0';
        if (on) atexit(totals);
    }
    return on;
}

static g8_file *by_handle(uint64_t h)
{
    unsigned i;

    for (i = nfiles; i-- > 0;)
        if (files[i].handle == h) return &files[i];
    return NULL;
}

static g8_file *by_path(const char *p)
{
    unsigned i;

    for (i = 0; i < nfiles; i++)
        if (!strcmp(files[i].path, p)) return &files[i];
    return NULL;
}

static uint64_t entries(void) { return trace_call_count ? trace_call_count() : 0; }
static uint64_t now_ms(void) { return fle_sys_clock_ticks() / FLE_TICKS_PER_MS; }

void __wrap_fle_imp_CreateFileW(void)
{
    uint64_t name = R[1], access = R[2];
    char path[260];
    unsigned i;

    if (!enabled()) {
        __real_fle_imp_CreateFileW();
        return;
    }
    for (i = 0; i < sizeof path - 1; i++) {
        uint16_t c = (uint16_t)fle_ld(name + 2u * i, 2);
        if (!c) break;
        path[i] = c < 0x80 ? (char)c : '?';
    }
    path[i] = 0;
    __real_fle_imp_CreateFileW();
    {
        uint64_t h = R[0];
        g8_file *f = by_path(path);

        if (!f && nfiles < G8_MAX) {
            f = &files[nfiles++];
            memset(f, 0, sizeof *f);
            snprintf(f->path, sizeof f->path, "%s", path);
        }
        if (f && h != UINT64_MAX) {
            f->handle = h;
            f->opens++;
        }
        fprintf(stderr, "g8-file: #%u open clock_ms %" PRIu64 " entries %" PRIu64 " access %#" PRIx64 " handle %#" PRIx64 " path %s\n", ++seq, now_ms(), entries(), access, h, path);
    }
}

void __wrap_fle_imp_ReadFile(void)
{
    uint64_t h = R[1], nread_ptr = R[9], want = R[8] & 0xFFFFFFFFu;

    __real_fle_imp_ReadFile();
    if (!enabled()) return;
    {
        g8_file *f = by_handle(h);
        uint64_t got = nread_ptr ? fle_ld(nread_ptr, 4) : 0;

        if (!f) return;
        f->reads++;
        f->read_bytes += got;
        fprintf(stderr, "g8-file: #%u read clock_ms %" PRIu64 " entries %" PRIu64 " handle %#" PRIx64 " want %" PRIu64 " got %" PRIu64 " ok %u path %s\n", ++seq, now_ms(), entries(), h, want, got,
                (unsigned)(R[0] & 0xFFFFFFFFu), f->path);
    }
}

void __wrap_fle_imp_CreateFileMappingW(void)
{
    uint64_t hfile = R[1];

    __real_fle_imp_CreateFileMappingW();
    if (!enabled()) return;
    {
        g8_file *f = by_handle(hfile);

        if (!f || !R[0]) return;
        if (nmappings < G8_MAX) {
            mapping_h[nmappings] = R[0];
            mapping_file[nmappings] = hfile;
            nmappings++;
        }
        fprintf(stderr, "g8-file: #%u mapping clock_ms %" PRIu64 " entries %" PRIu64 " file %#" PRIx64 " mapping %#" PRIx64 " path %s\n", ++seq, now_ms(), entries(), hfile, R[0], f->path);
    }
}

void __wrap_fle_imp_MapViewOfFile(void)
{
    uint64_t hmap = R[1], nbytes = fle_arg(5);   /* dwNumberOfBytesToMap */

    __real_fle_imp_MapViewOfFile();
    if (!enabled()) return;
    {
        unsigned i;
        g8_file *f = NULL;

        for (i = nmappings; i-- > 0;)
            if (mapping_h[i] == hmap) {
                f = by_handle(mapping_file[i]);
                break;
            }
        if (!f) return;
        f->maps++;
        f->mapped_bytes += nbytes;
        fprintf(stderr, "g8-file: #%u view clock_ms %" PRIu64 " entries %" PRIu64 " mapping %#" PRIx64 " bytes %" PRIu64 " at %#" PRIx64 " path %s\n", ++seq, now_ms(), entries(), hmap, nbytes, R[0], f->path);
    }
}

/* ---- audio ---- */
void g3_sha256_hex(const void *p, size_t n, char hex[65]);
uint32_t __real_fle_audio_ring_append(const float *lr, uint32_t frames);

static int aon = -1;
static FILE *adump;
static uint64_t ablocks, anonzero_blocks, aframes, afirst_nz_frame = UINT64_MAX, afirst_nz_ms, aframes_nz;
static float amax_all;

static void asummary(void)
{
    fprintf(stderr, "g8-audio-sum: blocks %" PRIu64 " frames %" PRIu64 " nonzero_blocks %" PRIu64 " nonzero_frames %" PRIu64 " max_abs %.9g first_nonzero_frame %lld first_nonzero_ms %" PRIu64 "\n", ablocks, aframes,
            anonzero_blocks, aframes_nz, (double)amax_all, afirst_nz_frame == UINT64_MAX ? -1LL : (long long)afirst_nz_frame, afirst_nz_ms);
}

uint32_t __wrap_fle_audio_ring_append(const float *lr, uint32_t frames)
{
    if (aon < 0) {
        const char *e = getenv("FLE_G8_AUDIOTRACE"), *d = getenv("FLE_G8_AUDIODUMP");

        aon = e && e[0] && e[0] != '0';
        if (d && d[0]) adump = fopen(d, "wb");
        if (aon) atexit(asummary);
    }
    if (aon && lr && frames) {
        uint64_t first = fle_audio_ring_get()->write_frames, nz = 0;
        float mx = 0;
        uint32_t k;
        char hex[65];

        for (k = 0; k < frames; k++) {
            float l = lr[2u * k], r = lr[2u * k + 1u], a = l < 0 ? -l : l, b = r < 0 ? -r : r;

            if (a > mx) mx = a;
            if (b > mx) mx = b;
            if (l != 0.0f || r != 0.0f) nz++;
        }
        g3_sha256_hex(lr, (size_t)frames * 8u, hex);
        ablocks++;
        aframes += frames;
        aframes_nz += nz;
        if (mx > amax_all) amax_all = mx;
        if (nz) {
            anonzero_blocks++;
            if (afirst_nz_frame == UINT64_MAX) {
                afirst_nz_frame = first;
                afirst_nz_ms = now_ms();
            }
        }
        fprintf(stderr, "g8-audio: block #%" PRIu64 " first_frame %" PRIu64 " frames %u clock_ms %" PRIu64 " consumed %" PRIu64 " max_abs %.9g nonzero_frames %" PRIu64 " sha256 %s\n", ablocks, first, frames, now_ms(),
                fle_audio_consumed(), (double)mx, nz, hex);
        if (adump) fwrite(lr, 8, frames, adump);
    }
    return __real_fle_audio_ring_append(lr, frames);
}

/* ---- GetProcAddress (FLE_G8_GPATRACE=1: one "g8-gpa:" line per call with the name, the module handle and the result; the guest's lazy binding of API pointers) ---- */
void __real_fle_imp_GetProcAddress(void);

void __wrap_fle_imp_GetProcAddress(void)
{
    static int gon = -1;
    uint64_t hmod = R[1], name = R[2];
    char nm[96];
    unsigned i;

    if (gon < 0) {
        const char *e = getenv("FLE_G8_GPATRACE");
        gon = e && e[0] && e[0] != '0';
    }
    if (!gon) {
        __real_fle_imp_GetProcAddress();
        return;
    }
    if (name < 0x10000u) snprintf(nm, sizeof nm, "#ordinal %llu", (unsigned long long)name);
    else {
        for (i = 0; i < sizeof nm - 1; i++) {
            unsigned char c = (unsigned char)fle_ld(name + i, 1);
            if (!c) break;
            nm[i] = c >= 32 && c < 127 ? (char)c : '?';
        }
        nm[i] = 0;
    }
    __real_fle_imp_GetProcAddress();
    fprintf(stderr, "g8-gpa: clock_ms %" PRIu64 " entries %" PRIu64 " hmod %#" PRIx64 " %s -> %#" PRIx64 "\n", now_ms(), entries(), hmod, nm, R[0]);
}
