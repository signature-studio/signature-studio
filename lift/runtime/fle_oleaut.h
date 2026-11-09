/* oleaut32 answers: BSTR, VARIANT, SAFEARRAY, GetErrorInfo (docs/decisions/0037-apis-oleaut-ole.md; the pages read are
 * listed in docs/investigations/apis-oleaut-ole-run.md). Same convention as fle_imports.h: arguments in R[1] (rcx),
 * R[2] (rdx), R[8] (r8), R[9] (r9); the return value in R[0] (an HRESULT in eax, zero-extended); the guest return
 * address stays on the stack for the caller's ret. Single guest thread only. A case the Microsoft pages do not define,
 * and every case that would need a call back into guest code (IUnknown::AddRef/Release, IRecordInfo), stops the run
 * through fle_stop (fle_env.h). */
#ifndef FLE_OLEAUT_H
#define FLE_OLEAUT_H

#include <stdint.h>

#include "fle_com.h"

/* The most characters one BSTR may hold here. A BSTR carries a 4-byte byte count, so 2^31 - 1 characters is the
 * format's own ceiling; this smaller limit is a limit of the virtual machine (a longer request stops the run). */
#define FLE_BSTR_MAX_CHARS 0x4000000u

/* oleaut32!SysAllocStringLen: BSTR f(const OLECHAR *strIn, UINT ui). SysFreeString: void f(BSTR).
 * SysReAllocStringLen: INT f(BSTR *pbstr, const OLECHAR *psz, unsigned int len). */
void fle_imp_SysAllocStringLen(void);
void fle_imp_SysFreeString(void);
void fle_imp_SysReAllocStringLen(void);

/* oleaut32!VariantInit: void f(VARIANTARG *). VariantClear: HRESULT f(VARIANTARG *).
 * VariantCopy: HRESULT f(VARIANTARG *dest, const VARIANTARG *src). VariantCopyInd: HRESULT f(VARIANT *dest,
 * const VARIANTARG *src). VariantChangeType: HRESULT f(VARIANTARG *dest, const VARIANTARG *src, USHORT wFlags,
 * VARTYPE vt). */
void fle_imp_VariantInit(void);
void fle_imp_VariantClear(void);
void fle_imp_VariantCopy(void);
void fle_imp_VariantCopyInd(void);
void fle_imp_VariantChangeType(void);

/* oleaut32!SafeArrayCreate: SAFEARRAY *f(VARTYPE vt, UINT cDims, SAFEARRAYBOUND *rgsabound).
 * SafeArrayGetLBound / SafeArrayGetUBound: HRESULT f(SAFEARRAY *, UINT nDim, LONG *). SafeArrayAccessData:
 * HRESULT f(SAFEARRAY *, void **). SafeArrayUnaccessData: HRESULT f(SAFEARRAY *). SafeArrayGetElement /
 * SafeArrayPutElement: HRESULT f(SAFEARRAY *, LONG *rgIndices, void *pv). SafeArrayPtrOfIndex: HRESULT
 * f(SAFEARRAY *, LONG *rgIndices, void **). SafeArrayGetElemsize: UINT f(SAFEARRAY *). */
void fle_imp_SafeArrayCreate(void);
void fle_imp_SafeArrayGetLBound(void);
void fle_imp_SafeArrayGetUBound(void);
void fle_imp_SafeArrayAccessData(void);
void fle_imp_SafeArrayUnaccessData(void);
void fle_imp_SafeArrayGetElement(void);
void fle_imp_SafeArrayPutElement(void);
void fle_imp_SafeArrayPtrOfIndex(void);
void fle_imp_SafeArrayGetElemsize(void);

/* oleaut32!GetErrorInfo: HRESULT f(ULONG dwReserved, IErrorInfo **pperrinfo). */
void fle_imp_GetErrorInfo(void);

/* Number of live BSTRs and of live safe arrays (for tests and tools). */
uint64_t fle_oleaut_bstr_live(void);
uint64_t fle_oleaut_array_live(void);

#endif
