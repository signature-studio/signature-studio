/* A read-only IBStream for the host side (decision 0165, PROVISIONAL): hands a saved plugin state (the bytes of a file) to IComponent::setState / IEditController::setComponentState /
 * setState. Interface of vst3_pluginterfaces/base/ibstream.h (fetched 2026-11-03): FUnknown (slots 0-2), read (3), write (4), seek (5), tell (6); kIBSeekSet 0, kIBSeekCur 1,
 * kIBSeekEnd 2; IID {C3BF6EA2-3099-4752-9B6B-F9901EE33E9B} (INLINE_UID 0xC3BF6EA2, 0x30994752, 0x9B6BF990, 0x1EE33E9B). The methods are C functions reached through the dispatch table
 * by rows (vst3a_stream_rows.txt, pseudo-VAs 0x7FFC20000000 + 0x10 * (k + 1)). read answers kResultOk with the number of bytes copied (short or 0 at the end; S-vst3a-stream-read: the
 * SDK header gives no value for a short read); write answers kNotImplemented. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "vst_host.h"
#include "vst3a.h"

typedef struct { uint64_t vtbl; uint8_t *data; int64_t size, pos; } mstream;
static uint64_t vt_st[7];
static vst_iid IID_IBStream;
static unsigned long long st_calls[7];

#define STREAM_VA(k) (UINT64_C(0x7FFC20000000) + UINT64_C(0x10) * ((uint64_t)(k) + 1))

uint64_t vst3a_stream_new(const void *data, size_t n)
{
    mstream *s = calloc(1, sizeof *s);
    for (int k = 0; k < 7; k++) vt_st[k] = STREAM_VA(k);
    vst_iid_make(IID_IBStream, 0xC3BF6EA2, 0x30994752, 0x9B6BF990, 0x1EE33E9B);
    s->vtbl = (uint64_t)(uintptr_t)vt_st;
    s->data = malloc(n ? n : 1);
    memcpy(s->data, data, n);
    s->size = (int64_t)n;
    return (uint64_t)(uintptr_t)s;
}
void vst3a_stream_report(void)
{
    static const char *const nm[7] = { "qi", "addref", "release", "read", "write", "seek", "tell" };
    printf("host IBStream calls:");
    for (int k = 0; k < 7; k++) if (st_calls[k]) printf(" %s=%llu", nm[k], st_calls[k]);
    printf("\n");
}
static mstream *me(void) { return (mstream *)(uintptr_t)R[1]; }
void vst3a_stream_qi(void)
{
    uint8_t b[16];
    st_calls[0]++;
    if (!R[2] || !R[8]) { R[0] = K_INVALID_ARG; return; }
    for (int i = 0; i < 16; i++) b[i] = (uint8_t)fle_ld(R[2] + i, 1);
    if (!memcmp(b, IID_FUnknown, 16) || !memcmp(b, IID_IBStream, 16)) { fle_st(R[8], 8, R[1]); R[0] = K_RESULT_OK; return; }
    fle_st(R[8], 8, 0);
    R[0] = K_NO_INTERFACE;
}
void vst3a_stream_addref(void) { st_calls[1]++; R[0] = 2; }
void vst3a_stream_release(void) { st_calls[2]++; R[0] = 1; }
void vst3a_stream_read(void)
{
    mstream *s = me();
    int32_t want = (int32_t)R[8];
    int64_t n;
    st_calls[3]++;
    if (!R[2] || want < 0) { R[0] = K_INVALID_ARG; return; }
    n = s->size - s->pos;
    if (n < 0) n = 0;
    if (n > want) n = want;
    memcpy((void *)(uintptr_t)R[2], s->data + s->pos, (size_t)n);
    s->pos += n;
    if (R[9]) fle_st(R[9], 4, (uint32_t)n);
    R[0] = K_RESULT_OK;
}
void vst3a_stream_write(void) { st_calls[4]++; if (R[9]) fle_st(R[9], 4, 0); R[0] = K_NOT_IMPL; }
void vst3a_stream_seek(void)
{
    mstream *s = me();
    int64_t pos = (int64_t)R[2], np;
    int32_t mode = (int32_t)R[8];
    st_calls[5]++;
    if (mode == 0) np = pos; else if (mode == 1) np = s->pos + pos; else if (mode == 2) np = s->size + pos; else { R[0] = K_INVALID_ARG; return; }
    if (np < 0) { R[0] = K_RESULT_FALSE; return; }
    s->pos = np;
    if (R[9]) fle_st(R[9], 8, (uint64_t)np);
    R[0] = K_RESULT_OK;
}
void vst3a_stream_tell(void)
{
    st_calls[6]++;
    if (!R[2]) { R[0] = K_INVALID_ARG; return; }
    fle_st(R[2], 8, (uint64_t)me()->pos);
    R[0] = K_RESULT_OK;
}
