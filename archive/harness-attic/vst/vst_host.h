/* Host side of a VST3 session (decision 0156): calling guest functions from C, and the host objects (IHostApplication, IComponentHandler, IParameterChanges,
 * IParamValueQueue, IEventList) that are handed to the plugin. Harness only. */
#ifndef FLE_VST_HOST_H
#define FLE_VST_HOST_H

#include <stdint.h>

#define VST_HM_VA_BASE UINT64_C(0x7FFC00000000)
#define VST_HM_VA(k) (VST_HM_VA_BASE + UINT64_C(0x10) * ((uint64_t)(k) + 1))
enum {
#define HM(n) VST_HM_##n,
#include "vst_hm.def"
#undef HM
    VST_HM_COUNT
};

#define K_RESULT_OK 0u
#define K_RESULT_FALSE 1u
#define K_NO_INTERFACE 0x80004002u
#define K_INVALID_ARG 0x80070057u
#define K_NOT_IMPL 0x80004001u

typedef uint8_t vst_iid[16];

void vst_iid_make(vst_iid out, uint32_t l1, uint32_t l2, uint32_t l3, uint32_t l4);
extern vst_iid IID_FUnknown, IID_IPluginFactory, IID_IPluginFactory2, IID_IPluginFactory3, IID_IComponent, IID_IAudioProcessor, IID_IEditController,
    IID_IHostApplication, IID_IComponentHandler, IID_IParameterChanges, IID_IParamValueQueue, IID_IEventList, IID_IPluginBase;
void vst_iids_init(void);

/* Guest calls. gcall: Windows x64 convention, up to six integer arguments (the 5th and 6th on the stack) and up to four double arguments (argument slot i in xmm_i).
 * vcall(obj, slot, ...) = gcall(vtable[slot], obj, ...). The result is rax; gret_d is xmm0 after the call. */
uint64_t gcall(uint64_t va, const uint64_t a[6], const double d[4]);
uint64_t vcall(uint64_t obj, unsigned slot, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5);
uint64_t vcall_d(uint64_t obj, unsigned slot, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, const double d[4]);
double gret_d(void);
void gcall_set_stack(uint64_t sp);

/* Host objects. */
typedef struct {
    uint64_t host_app;        /* IHostApplication* (as FUnknown*) */
    uint64_t comp_handler;    /* IComponentHandler* */
    uint64_t param_changes;   /* IParameterChanges* (input of the next process call) */
    uint64_t event_list;      /* IEventList* (empty) */
    uint64_t out_changes;     /* IParameterChanges* (output; accepts and drops) */
} vst_host;
int vst_host_init(vst_host *h);
/* Input parameter changes for the next process call: replaces the list. */
void vst_pc_clear(void);
void vst_pc_add(uint32_t id, int32_t sample_offset, double value);
void vst_host_report(void);

#endif
