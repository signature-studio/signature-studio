/* Types shared by the differential test files of decision 0143. */
#ifndef MXFP_COMMON_H
#define MXFP_COMMON_H
#include <stdint.h>
#include <string.h>

typedef union {
    uint64_t u64[2];
    uint32_t u32[4];
} xmm_t;

struct MxfpOp {
    const char *name;
    int sk;    /* 0 f32 lanes, 1 f64 lanes, 2 i32 lanes, 3 GPR source */
    int sid;   /* id of the fast-path statistics of the lane class, -1 if none */
    int lanes; /* lanes per instruction */
    int comi;
    void (*fn)(xmm_t *, const xmm_t *, uint64_t *);
};

extern const struct MxfpOp MXFP_OPS_new[], MXFP_OPS_nofast[], MXFP_OPS_old[], MXFP_OPS_host[];
extern const unsigned MXFP_N_new, MXFP_N_nofast, MXFP_N_old, MXFP_N_host;
extern uint64_t fle_mx_stat[64][2];
#endif
