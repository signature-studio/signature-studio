/* Host side of a VST3 session (decision 0156). See vst_host.h. Every value that is the harness's own choice (not the SDK's, not FL Studio's) is marked S-vst-... and
 * listed in docs/decisions/0156-first-vst3-plugin.md. */
#include "vst_host.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "boot.h"
#include "fle_dispatch.h"
#include "fle_runtime.h"
#include "fle_sse.h"

vst_iid IID_FUnknown, IID_IPluginFactory, IID_IPluginFactory2, IID_IPluginFactory3, IID_IComponent, IID_IAudioProcessor, IID_IEditController,
    IID_IHostApplication, IID_IComponentHandler, IID_IParameterChanges, IID_IParamValueQueue, IID_IEventList, IID_IPluginBase;

/* INLINE_UID with COM_COMPATIBLE = 1 (the Windows build): base/funknown.h of the public SDK, https://github.com/steinbergmedia/vst3_pluginterfaces */
void vst_iid_make(vst_iid o, uint32_t l1, uint32_t l2, uint32_t l3, uint32_t l4)
{
    o[0] = (uint8_t)l1; o[1] = (uint8_t)(l1 >> 8); o[2] = (uint8_t)(l1 >> 16); o[3] = (uint8_t)(l1 >> 24);
    o[4] = (uint8_t)(l2 >> 16); o[5] = (uint8_t)(l2 >> 24); o[6] = (uint8_t)l2; o[7] = (uint8_t)(l2 >> 8);
    o[8] = (uint8_t)(l3 >> 24); o[9] = (uint8_t)(l3 >> 16); o[10] = (uint8_t)(l3 >> 8); o[11] = (uint8_t)l3;
    o[12] = (uint8_t)(l4 >> 24); o[13] = (uint8_t)(l4 >> 16); o[14] = (uint8_t)(l4 >> 8); o[15] = (uint8_t)l4;
}

void vst_iids_init(void)
{
    vst_iid_make(IID_FUnknown, 0x00000000, 0x00000000, 0xC0000000, 0x00000046);
    vst_iid_make(IID_IPluginBase, 0x22888DDB, 0x156E45AE, 0x8358B348, 0x08190625);
    vst_iid_make(IID_IPluginFactory, 0x7A4D811C, 0x52114A1F, 0xAED9D2EE, 0x0B43BF9F);
    vst_iid_make(IID_IPluginFactory2, 0x0007B650, 0xF24B4C0B, 0xA464EDB9, 0xF00B2ABB);
    vst_iid_make(IID_IPluginFactory3, 0x4555A2AB, 0xC1234E57, 0x9B122910, 0x36878931);
    vst_iid_make(IID_IComponent, 0xE831FF31, 0xF2D54301, 0x928EBBEE, 0x25697802);
    vst_iid_make(IID_IAudioProcessor, 0x42043F99, 0xB7DA453C, 0xA569E79D, 0x9AAEC33D);
    vst_iid_make(IID_IEditController, 0xDCD7BBE3, 0x7742448D, 0xA874AACC, 0x979C759E);
    vst_iid_make(IID_IHostApplication, 0x58E595CC, 0xDB2D4969, 0x8B6AAF8C, 0x36A664E5);
    vst_iid_make(IID_IComponentHandler, 0x93A0BEA3, 0x0BD045DB, 0x8E890B0C, 0xC1E46AC6);
    vst_iid_make(IID_IParameterChanges, 0xA4779663, 0x0BB64A56, 0xB44384A8, 0x466FEB9D);
    vst_iid_make(IID_IParamValueQueue, 0x01263A18, 0xED074F6F, 0x98C9D356, 0x4686F9BA);
    vst_iid_make(IID_IEventList, 0x3A2C4214, 0x346349FE, 0xB2C4F397, 0xB9695A44);
}

/* ---- calling the guest ---- */
uint64_t gcall(uint64_t va, const uint64_t a[6], const double d[4])
{
    uint64_t entry_sp = R[4];
    uint64_t call_sp = (entry_sp - 0x30) & ~UINT64_C(15);
    uint64_t r;
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    fle_st(call_sp + 0x20, 8, a[4]);
    fle_st(call_sp + 0x28, 8, a[5]);
    if (d) {
        for (int i = 0; i < 4; i++) { X[i].f64[0] = d[i]; X[i].u64[1] = 0; }
    }
    R[4] = call_sp - 8;
    fle_st(R[4], 8, HARNESS_SENTINEL_RET);
    fle_dispatch(va);
    if (R[4] != call_sp - 8) { fprintf(stderr, "vst: guest call %#llx returned with an unbalanced stack\n", (unsigned long long)va); abort(); }
    r = R[0];
    R[4] = entry_sp;
    return r;
}

uint64_t vcall_d(uint64_t obj, unsigned slot, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, const double d[4])
{
    uint64_t vt = fle_ld(obj, 8);
    uint64_t fn = fle_ld(vt + 8u * slot, 8);
    uint64_t a[6] = { obj, a2, a3, a4, a5, 0 };
    return gcall(fn, a, d);
}

uint64_t vcall(uint64_t obj, unsigned slot, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5)
{
    return vcall_d(obj, slot, a2, a3, a4, a5, NULL);
}

double gret_d(void) { return X[0].f64[0]; }

/* ---- host objects ---- */
typedef struct vq { uint64_t vtbl; uint32_t id; int32_t n; int32_t off[8]; double val[8]; } vq;
#define PC_MAX 8
typedef struct pcs { uint64_t vtbl; int32_t count; int32_t pad; vq q[PC_MAX]; } pcs;
typedef struct { uint64_t vtbl; } plain;

static uint64_t vt_ha[5], vt_ch[7], vt_pc[6], vt_vq[7], vt_el[6];
static plain o_ha, o_ch, o_el;
static pcs o_pc_in, o_pc_out;
static struct { uint64_t obj; const uint8_t *iid; const char *name; } reg[8];
static int nreg;
static uint64_t hm_calls[VST_HM_COUNT];
static const char *const hm_names[] = {
#define HM(n) #n,
#include "vst_hm.def"
#undef HM
};
static int g_trace;

static void reg_add(uint64_t obj, const uint8_t *iid, const char *name) { reg[nreg].obj = obj; reg[nreg].iid = iid; reg[nreg].name = name; nreg++; }

int vst_host_init(vst_host *h)
{
    vst_iids_init();
    g_trace = getenv("VST_TRACE") != NULL;
    vt_ha[0] = VST_HM_VA(VST_HM_qi); vt_ha[1] = VST_HM_VA(VST_HM_addref); vt_ha[2] = VST_HM_VA(VST_HM_release);
    vt_ha[3] = VST_HM_VA(VST_HM_ha_getName); vt_ha[4] = VST_HM_VA(VST_HM_ha_createInstance);
    vt_ch[0] = VST_HM_VA(VST_HM_qi); vt_ch[1] = VST_HM_VA(VST_HM_addref); vt_ch[2] = VST_HM_VA(VST_HM_release);
    vt_ch[3] = VST_HM_VA(VST_HM_ch_beginEdit); vt_ch[4] = VST_HM_VA(VST_HM_ch_performEdit); vt_ch[5] = VST_HM_VA(VST_HM_ch_endEdit); vt_ch[6] = VST_HM_VA(VST_HM_ch_restart);
    vt_pc[0] = VST_HM_VA(VST_HM_qi); vt_pc[1] = VST_HM_VA(VST_HM_addref); vt_pc[2] = VST_HM_VA(VST_HM_release);
    vt_pc[3] = VST_HM_VA(VST_HM_pc_count); vt_pc[4] = VST_HM_VA(VST_HM_pc_data); vt_pc[5] = VST_HM_VA(VST_HM_pc_add);
    vt_vq[0] = VST_HM_VA(VST_HM_qi); vt_vq[1] = VST_HM_VA(VST_HM_addref); vt_vq[2] = VST_HM_VA(VST_HM_release);
    vt_vq[3] = VST_HM_VA(VST_HM_vq_id); vt_vq[4] = VST_HM_VA(VST_HM_vq_npoints); vt_vq[5] = VST_HM_VA(VST_HM_vq_getPoint); vt_vq[6] = VST_HM_VA(VST_HM_vq_addPoint);
    vt_el[0] = VST_HM_VA(VST_HM_qi); vt_el[1] = VST_HM_VA(VST_HM_addref); vt_el[2] = VST_HM_VA(VST_HM_release);
    vt_el[3] = VST_HM_VA(VST_HM_el_count); vt_el[4] = VST_HM_VA(VST_HM_el_get); vt_el[5] = VST_HM_VA(VST_HM_el_add);
    o_ha.vtbl = (uint64_t)(uintptr_t)vt_ha;
    o_ch.vtbl = (uint64_t)(uintptr_t)vt_ch;
    o_el.vtbl = (uint64_t)(uintptr_t)vt_el;
    o_pc_in.vtbl = (uint64_t)(uintptr_t)vt_pc;
    o_pc_out.vtbl = (uint64_t)(uintptr_t)vt_pc;
    for (int i = 0; i < PC_MAX; i++) { o_pc_in.q[i].vtbl = (uint64_t)(uintptr_t)vt_vq; o_pc_out.q[i].vtbl = (uint64_t)(uintptr_t)vt_vq; }
    reg_add((uint64_t)(uintptr_t)&o_ha, IID_IHostApplication, "IHostApplication");
    reg_add((uint64_t)(uintptr_t)&o_ch, IID_IComponentHandler, "IComponentHandler");
    reg_add((uint64_t)(uintptr_t)&o_el, IID_IEventList, "IEventList");
    reg_add((uint64_t)(uintptr_t)&o_pc_in, IID_IParameterChanges, "IParameterChanges");
    reg_add((uint64_t)(uintptr_t)&o_pc_out, IID_IParameterChanges, "IParameterChanges(out)");
    for (int i = 0; i < PC_MAX; i++) {
        reg_add((uint64_t)(uintptr_t)&o_pc_in.q[i], IID_IParamValueQueue, "IParamValueQueue");
        if (nreg >= 8) break;
    }
    h->host_app = (uint64_t)(uintptr_t)&o_ha;
    h->comp_handler = (uint64_t)(uintptr_t)&o_ch;
    h->param_changes = (uint64_t)(uintptr_t)&o_pc_in;
    h->event_list = (uint64_t)(uintptr_t)&o_el;
    h->out_changes = (uint64_t)(uintptr_t)&o_pc_out;
    return 0;
}

void vst_pc_clear(void) { o_pc_in.count = 0; for (int i = 0; i < PC_MAX; i++) o_pc_in.q[i].n = 0; }

void vst_pc_add(uint32_t id, int32_t off, double value)
{
    int k;
    for (k = 0; k < o_pc_in.count; k++) if (o_pc_in.q[k].id == id) break;
    if (k == o_pc_in.count) {
        if (k >= PC_MAX) { fprintf(stderr, "vst: more than %d parameter queues\n", PC_MAX); abort(); }
        o_pc_in.count++; o_pc_in.q[k].id = id; o_pc_in.q[k].n = 0;
    }
    if (o_pc_in.q[k].n >= 8) { fprintf(stderr, "vst: more than 8 points in a queue\n"); abort(); }
    o_pc_in.q[k].off[o_pc_in.q[k].n] = off;
    o_pc_in.q[k].val[o_pc_in.q[k].n] = value;
    o_pc_in.q[k].n++;
}

static void hm_note(int k, const char *extra)
{
    hm_calls[k]++;
    if (g_trace) fprintf(stderr, "vst: host %s%s\n", hm_names[k], extra ? extra : "");
}

void vst_host_report(void)
{
    printf("host calls:");
    for (int k = 0; k < VST_HM_COUNT; k++) if (hm_calls[k]) printf(" %s=%llu", hm_names[k], (unsigned long long)hm_calls[k]);
    printf("\n");
}

/* FUnknown for every host object: queryInterface answers the object itself for FUnknown and for the interface the object was made for (S-vst-qi: no other interface,
 * in particular no IComponentHandler2, IPlugFrame, IUnitHandler: FL Studio's host objects do offer some of them, which the RTTI of FLEngine shows, but the harness has no
 * code behind them); addRef/release keep no count (the objects are static and never freed). */
void vst_hm_qi(void)
{
    uint64_t self = R[1], iid = R[2], pobj = R[8];
    uint8_t b[16];
    hm_note(VST_HM_qi, NULL);
    if (!iid || !pobj) { R[0] = K_INVALID_ARG; return; }
    for (int i = 0; i < 16; i++) b[i] = (uint8_t)fle_ld(iid + i, 1);
    if (memcmp(b, IID_FUnknown, 16) == 0) { fle_st(pobj, 8, self); R[0] = K_RESULT_OK; return; }
    for (int i = 0; i < nreg; i++) {
        if (reg[i].obj == self && memcmp(b, reg[i].iid, 16) == 0) { fle_st(pobj, 8, self); R[0] = K_RESULT_OK; return; }
    }
    if (g_trace) { char s[64]; for (int i = 0; i < 16; i++) snprintf(s + 2 * i, 3, "%02x", b[i]); fprintf(stderr, "vst: host queryInterface of %s refused\n", s); }
    fle_st(pobj, 8, 0);
    R[0] = K_NO_INTERFACE;
}
void vst_hm_addref(void) { hm_note(VST_HM_addref, NULL); R[0] = 2; }
void vst_hm_release(void) { hm_note(VST_HM_release, NULL); R[0] = 1; }

/* IHostApplication::getName(String128): S-vst-hostname: "FL Studio" (the product that hosts the plugin in the reference project; the string FL passes is not known). */
void vst_hm_ha_getName(void)
{
    uint64_t p = R[2];
    const char *s = "FL Studio";
    size_t i;
    hm_note(VST_HM_ha_getName, NULL);
    if (!p) { R[0] = K_INVALID_ARG; return; }
    for (i = 0; s[i]; i++) fle_st(p + 2 * i, 2, (unsigned char)s[i]);
    fle_st(p + 2 * i, 2, 0);
    R[0] = K_RESULT_OK;
}
/* IHostApplication::createInstance(TUID cid, TUID iid, void **obj): S-vst-hostmsg: the host offers no IMessage/IAttributeList (FL Studio does, TVST3Message/TVST3AttributeList), so
 * the answer is kNotImplemented with *obj = NULL; a plugin that needs a message to connect its component and controller stops here and is reported. */
void vst_hm_ha_createInstance(void)
{
    uint64_t pobj = R[9];
    hm_note(VST_HM_ha_createInstance, NULL);
    if (pobj) fle_st(pobj, 8, 0);
    R[0] = K_NOT_IMPL;
}
void vst_hm_ch_beginEdit(void) { hm_note(VST_HM_ch_beginEdit, NULL); R[0] = K_RESULT_OK; }
void vst_hm_ch_performEdit(void) { hm_note(VST_HM_ch_performEdit, NULL); R[0] = K_RESULT_OK; }
void vst_hm_ch_endEdit(void) { hm_note(VST_HM_ch_endEdit, NULL); R[0] = K_RESULT_OK; }
void vst_hm_ch_restart(void)
{
    char s[40];
    snprintf(s, sizeof s, " flags=%#x", (unsigned)R[2]);
    hm_note(VST_HM_ch_restart, s);
    R[0] = K_RESULT_OK;
}
static pcs *pc_of(uint64_t o) { return (pcs *)(uintptr_t)o; }
void vst_hm_pc_count(void) { hm_note(VST_HM_pc_count, NULL); R[0] = (uint32_t)pc_of(R[1])->count; }
void vst_hm_pc_data(void)
{
    pcs *p = pc_of(R[1]);
    int32_t i = (int32_t)R[2];
    hm_note(VST_HM_pc_data, NULL);
    R[0] = (i >= 0 && i < p->count) ? (uint64_t)(uintptr_t)&p->q[i] : 0;
}
/* addParameterData(const ParamID &id, int32 &index): output side. The harness keeps the first queues and drops nothing silently: the points that the plugin adds are counted. */
void vst_hm_pc_add(void)
{
    pcs *p = pc_of(R[1]);
    uint32_t id = (uint32_t)fle_ld(R[2], 4);
    int k;
    hm_note(VST_HM_pc_add, NULL);
    for (k = 0; k < p->count; k++) if (p->q[k].id == id) break;
    if (k == p->count) {
        if (k >= PC_MAX) { R[0] = 0; return; }
        p->count++; p->q[k].id = id; p->q[k].n = 0;
    }
    if (R[8]) fle_st(R[8], 4, (uint32_t)k);
    R[0] = (uint64_t)(uintptr_t)&p->q[k];
}
static vq *vq_of(uint64_t o) { return (vq *)(uintptr_t)o; }
void vst_hm_vq_id(void) { hm_note(VST_HM_vq_id, NULL); R[0] = vq_of(R[1])->id; }
void vst_hm_vq_npoints(void) { hm_note(VST_HM_vq_npoints, NULL); R[0] = (uint32_t)vq_of(R[1])->n; }
void vst_hm_vq_getPoint(void)
{
    vq *q = vq_of(R[1]);
    int32_t i = (int32_t)R[2];
    hm_note(VST_HM_vq_getPoint, NULL);
    if (i < 0 || i >= q->n) { R[0] = K_RESULT_FALSE; return; }
    fle_st(R[8], 4, (uint32_t)q->off[i]);
    memcpy((void *)(uintptr_t)R[9], &q->val[i], 8);
    R[0] = K_RESULT_OK;
}
void vst_hm_vq_addPoint(void)
{
    vq *q = vq_of(R[1]);
    hm_note(VST_HM_vq_addPoint, NULL);
    if (q->n >= 8) { R[0] = K_RESULT_FALSE; return; }
    q->off[q->n] = (int32_t)R[2];
    q->val[q->n] = X[2].f64[0];
    if (R[9]) fle_st(R[9], 4, (uint32_t)q->n);
    q->n++;
    R[0] = K_RESULT_OK;
}
void vst_hm_el_count(void) { hm_note(VST_HM_el_count, NULL); R[0] = 0; }
void vst_hm_el_get(void) { hm_note(VST_HM_el_get, NULL); R[0] = K_RESULT_FALSE; }
void vst_hm_el_add(void) { hm_note(VST_HM_el_add, NULL); R[0] = K_RESULT_OK; }
