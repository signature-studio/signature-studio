#include <stdint.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_msctf.h"

/* The Learn page of SetInputScope gives one result (S_OK, "the method was successful") and one failure (E_INVALIDARG when
 * the window belongs to another thread). Both need to know the window: this runtime has no window table, so no value is
 * made up for either. The run stops and names the call. */
void fle_imp_SetInputScope(void)
{
    fle_stop("SetInputScope",
             "msctf.dll is a virtual module that exports it, but this runtime has no window table to attach an input scope "
             "to (the page's S_OK and E_INVALIDARG both depend on the window); no HRESULT is invented");
}
