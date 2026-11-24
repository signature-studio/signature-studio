/* Host side of a VST3 session, shared by the vst3a harness (lift/harness/vst3a) and the plugin-worker wasm (lift/harness/web/mt1/plugin-worker).
 * The objects are static C structs whose method tables hold pseudo-VAs; the lifted plugin reaches them through fle_dispatch, and the generated
 * dispatch table maps each VST_HM_VA(...) to its vst_hm_* function (mkrows.sh writes those rows from vst_hm.def).
 * Interface IDs and HRESULTs: the VST3 SDK (vst3_pluginterfaces) and the Windows SDK. Not a copy of any vendor header. */
#pragma once

#include <stdint.h>

typedef uint8_t vst_iid[16];

typedef struct {
    uint64_t host_app;
    uint64_t comp_handler;
    uint64_t param_changes;
    uint64_t event_list;
    uint64_t out_changes;
} vst_host;

extern vst_iid IID_FUnknown, IID_IPluginBase, IID_IPluginFactory, IID_IPluginFactory2, IID_IPluginFactory3, IID_IComponent, IID_IAudioProcessor,
    IID_IEditController, IID_IHostApplication, IID_IComponentHandler, IID_IParameterChanges, IID_IParamValueQueue, IID_IEventList;

void vst_iid_make(vst_iid o, uint32_t l1, uint32_t l2, uint32_t l3, uint32_t l4);
void vst_iids_init(void);

#define K_RESULT_OK 0x00000000u
#define K_RESULT_FALSE 0x00000001u
#define K_NO_INTERFACE 0x80004002u
#define K_NOT_IMPL 0x80004001u
#define K_INVALID_ARG 0x80070057u

/* The method rows: VST_HM(n) for each line HM(n) of vst_hm.def, numbered from 0 in that order. The pseudo-VA of row k is VST_HM_BASE + 0x10 * (k + 1). */
#define VST_HM_BASE UINT64_C(0x7FFC10000000)
enum {
#define HM(n) VST_HM_##n,
#include "vst_hm.def"
#undef HM
    VST_HM_COUNT
};
#define VST_HM_VA(k) (VST_HM_BASE + 0x10u * ((uint64_t)(k) + 1u))

/* calling the guest: gcall runs a guest function at va; vcall calls slot `slot` of the vtable of obj (arguments after this); the result is R[0] */
uint64_t gcall(uint64_t va, const uint64_t a[6], const double d[4]);
uint64_t vcall(uint64_t obj, unsigned slot, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5);
uint64_t vcall_d(uint64_t obj, unsigned slot, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, const double d[4]);
double gret_d(void);

int vst_host_init(vst_host *h);
void vst_host_report(void);
void vst_pc_clear(void);
void vst_pc_add(uint32_t id, int32_t off, double value);
