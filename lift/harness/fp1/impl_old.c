/* The header of HEAD ac5a790 before decision 0143, verbatim (ref/fle_mxcsr_flags_old.h): the reference. It needs
 * only fle_xmm, fle_ld and FL; fle_sse.h is not included (it would include the new header). */
#include "fle_runtime.h"
#include "mxfp_common.h"
typedef union {
    uint64_t u64[2];
    uint32_t u32[4];
    float f32[4];
    double f64[2];
} fle_xmm;
_Static_assert(sizeof(fle_xmm) == 16, "fle_xmm is 128 bits");
#include "fle_mxcsr_flags_old.h"
#define PFX fle_mx_
#define MPFX FLE_MX_
#define TAG old
#define MXFP_HAS_HADDPD 1
#include "mxfp_ops.h"
