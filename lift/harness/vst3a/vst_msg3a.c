/* Host IMessage and IAttributeList (decision 0165, PROVISIONAL). Serum 2 asks the host for a message (IHostApplication::createInstance with the IMessage class id and interface id; two times at start)
 * and sends its controller an int64 attribute holding a pointer to its processor's data through IConnectionPoint::notify; SerumController::notify (0x1800b61b0) reads it with
 * IMessage::getAttributes()->getInt(id, v) and stores it at [controller + 0x22408]. Without a message the field stays NULL and the controller's setComponentState with a non-default state
 * dereferences it (state 5). Interfaces of vst3_pluginterfaces vst/ivstmessage.h and vst/ivstattributes.h (fetched 2026-11-03): IMessage: getMessageID (slot 3), setMessageID (4),
 * getAttributes (5); IAttributeList: setInt (3), getInt (4), setFloat (5), getFloat (6), setString (7), getString (8), setBinary (9), getBinary (10). IAttributeList IID 0x1E5F0AEB,
 * 0xCC7F4533, 0xA2544011, 0x38AD5EE4. The methods are dispatch-table rows 0x7FFC40000000 + 0x10 * (k + 1) (gen_vst3a.sh). S-vst3a-imessage: a small dictionary (32 keys) per list;
 * a key that is set twice is replaced; a string is kept as UTF-16 up to 255 units; binary data is copied. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_sse.h"
#include "vst_host.h"
#include "vst3a.h"

#define MSG_VA(k) (UINT64_C(0x7FFC40000000) + UINT64_C(0x10) * ((uint64_t)(k) + 1))
typedef struct { char key[64]; int type; int64_t i; double f; uint16_t s[256]; uint8_t *bin; uint32_t n; } attr;
typedef struct { uint64_t vtbl; attr a[32]; int na; } vattr;
typedef struct { uint64_t vtbl; char id[64]; vattr *attrs; } vmsg;
static uint64_t vt_msg[6], vt_attr[11];
static vst_iid IID_IMessage, IID_IAttributeList;
static unsigned long long m_calls[17];
static int m_inited;

static void m_init(void)
{
    if (m_inited) return;
    m_inited = 1;
    for (int k = 0; k < 6; k++) vt_msg[k] = MSG_VA(k);
    for (int k = 0; k < 11; k++) vt_attr[k] = MSG_VA(6 + k);
    vst_iid_make(IID_IMessage, 0x936F033B, 0xC6C047DB, 0xBB0882F8, 0x13C1E613);
    vst_iid_make(IID_IAttributeList, 0x1E5F0AEB, 0xCC7F4533, 0xA2544011, 0x38AD5EE4);
}
/* returns the guest address of a new IMessage, or 0 when (cid, iid) is not IMessage */
uint64_t vst3a_msg_try_create(uint64_t cid_p, uint64_t iid_p)
{
    uint8_t c[16], i[16];
    vmsg *m;
    m_init();
    for (int k = 0; k < 16; k++) { c[k] = (uint8_t)fle_ld(cid_p + k, 1); i[k] = (uint8_t)fle_ld(iid_p + k, 1); }
    if (memcmp(c, IID_IMessage, 16) || memcmp(i, IID_IMessage, 16)) return 0;
    m = calloc(1, sizeof *m);
    m->vtbl = (uint64_t)(uintptr_t)vt_msg;
    m->attrs = calloc(1, sizeof *m->attrs);
    m->attrs->vtbl = (uint64_t)(uintptr_t)vt_attr;
    return (uint64_t)(uintptr_t)m;
}
void vst3a_msg_report(void)
{
    static const char *const nm[17] = { "msg.qi", "msg.addref", "msg.release", "getMessageID", "setMessageID", "getAttributes", "attr.qi", "attr.addref", "attr.release", "setInt", "getInt", "setFloat", "getFloat", "setString", "getString", "setBinary", "getBinary" };
    printf("host IMessage/IAttributeList calls:");
    for (int k = 0; k < 17; k++) if (m_calls[k]) printf(" %s=%llu", nm[k], m_calls[k]);
    printf("\n");
}
static void qi_common(const uint8_t *self_iid)
{
    uint8_t b[16];
    if (!R[2] || !R[8]) { R[0] = K_INVALID_ARG; return; }
    for (int i = 0; i < 16; i++) b[i] = (uint8_t)fle_ld(R[2] + i, 1);
    if (!memcmp(b, IID_FUnknown, 16) || !memcmp(b, self_iid, 16)) { fle_st(R[8], 8, R[1]); R[0] = K_RESULT_OK; return; }
    fle_st(R[8], 8, 0);
    R[0] = K_NO_INTERFACE;
}
void vst3a_msg_qi(void) { m_calls[0]++; qi_common(IID_IMessage); }
void vst3a_msg_addref(void) { m_calls[1]++; R[0] = 2; }
void vst3a_msg_release(void) { m_calls[2]++; R[0] = 1; }
void vst3a_msg_getMessageID(void) { m_calls[3]++; R[0] = (uint64_t)(uintptr_t)((vmsg *)(uintptr_t)R[1])->id; }
void vst3a_msg_setMessageID(void)
{
    vmsg *m = (vmsg *)(uintptr_t)R[1];
    size_t i;
    m_calls[4]++;
    for (i = 0; i < sizeof m->id - 1; i++) { char c = (char)fle_ld(R[2] + i, 1); m->id[i] = c; if (!c) break; }
    m->id[sizeof m->id - 1] = 0;
    R[0] = 0;
}
void vst3a_msg_getAttributes(void) { m_calls[5]++; R[0] = (uint64_t)(uintptr_t)((vmsg *)(uintptr_t)R[1])->attrs; }
void vst3a_attr_qi(void) { m_calls[6]++; qi_common(IID_IAttributeList); }
void vst3a_attr_addref(void) { m_calls[7]++; R[0] = 2; }
void vst3a_attr_release(void) { m_calls[8]++; R[0] = 1; }
static attr *find(vattr *l, uint64_t id, int make)
{
    char k[64];
    size_t i;
    for (i = 0; i < sizeof k - 1; i++) { k[i] = (char)fle_ld(id + i, 1); if (!k[i]) break; }
    k[sizeof k - 1] = 0;
    for (int j = 0; j < l->na; j++) if (!strcmp(l->a[j].key, k)) return &l->a[j];
    if (!make) return NULL;
    if (l->na >= 32) vst3a_stop("IAttributeList", "more than 32 attributes in one list");
    memset(&l->a[l->na], 0, sizeof l->a[0]);
    strcpy(l->a[l->na].key, k);
    return &l->a[l->na++];
}
void vst3a_attr_setInt(void) { vattr *l = (vattr *)(uintptr_t)R[1]; attr *a = find(l, R[2], 1); m_calls[9]++; a->type = 1; a->i = (int64_t)R[8]; R[0] = 0; }
void vst3a_attr_getInt(void) { vattr *l = (vattr *)(uintptr_t)R[1]; attr *a = find(l, R[2], 0); m_calls[10]++; if (!a || a->type != 1 || !R[8]) { R[0] = K_RESULT_FALSE; return; } fle_st(R[8], 8, (uint64_t)a->i); R[0] = 0; }
void vst3a_attr_setFloat(void) { vattr *l = (vattr *)(uintptr_t)R[1]; attr *a = find(l, R[2], 1); m_calls[11]++; a->type = 2; a->f = X[2].f64[0]; R[0] = 0; }
void vst3a_attr_getFloat(void) { vattr *l = (vattr *)(uintptr_t)R[1]; attr *a = find(l, R[2], 0); m_calls[12]++; if (!a || a->type != 2 || !R[8]) { R[0] = K_RESULT_FALSE; return; } memcpy((void *)(uintptr_t)R[8], &a->f, 8); R[0] = 0; }
void vst3a_attr_setString(void)
{
    vattr *l = (vattr *)(uintptr_t)R[1];
    attr *a = find(l, R[2], 1);
    size_t i;
    m_calls[13]++;
    a->type = 3;
    for (i = 0; i < 255; i++) { a->s[i] = (uint16_t)fle_ld(R[8] + 2 * i, 2); if (!a->s[i]) break; }
    a->s[255] = 0;
    R[0] = 0;
}
void vst3a_attr_getString(void)
{
    vattr *l = (vattr *)(uintptr_t)R[1];
    attr *a = find(l, R[2], 0);
    uint32_t cap = (uint32_t)R[9] / 2;
    size_t n = 0;
    m_calls[14]++;
    if (!a || a->type != 3 || !R[8] || cap == 0) { R[0] = K_RESULT_FALSE; return; }
    while (a->s[n]) n++;
    if (n + 1 > cap) n = cap - 1;
    for (size_t i = 0; i < n; i++) fle_st(R[8] + 2 * i, 2, a->s[i]);
    fle_st(R[8] + 2 * n, 2, 0);
    R[0] = 0;
}
void vst3a_attr_setBinary(void)
{
    vattr *l = (vattr *)(uintptr_t)R[1];
    attr *a = find(l, R[2], 1);
    uint32_t n = (uint32_t)R[9];
    m_calls[15]++;
    a->type = 4;
    free(a->bin);
    a->bin = malloc(n ? n : 1);
    for (uint32_t i = 0; i < n; i++) a->bin[i] = (uint8_t)fle_ld(R[8] + i, 1);
    a->n = n;
    R[0] = 0;
}
void vst3a_attr_getBinary(void)
{
    vattr *l = (vattr *)(uintptr_t)R[1];
    attr *a = find(l, R[2], 0);
    m_calls[16]++;
    if (!a || a->type != 4 || !R[8] || !R[9]) { R[0] = K_RESULT_FALSE; return; }
    fle_st(R[8], 8, (uint64_t)(uintptr_t)a->bin);
    fle_st(R[9], 4, a->n);
    R[0] = 0;
}
