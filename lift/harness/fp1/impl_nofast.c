/* The runtime's fle_mxcsr_flags.h with -DFLE_MX_NOFAST: the integer path only. */
#include "fle_runtime.h"
#include "fle_sse.h"
#include "mxfp_common.h"
#define PFX fle_mx_
#define MPFX FLE_MX_
#define TAG nofast
#define MXFP_HAS_HADDPD 1
#include "mxfp_ops.h"
