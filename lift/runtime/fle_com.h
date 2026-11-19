/* HRESULT values shared by fle_oleaut.c and fle_ole.c (docs/decisions/0037-apis-oleaut-ole.md). Every value is the
 * one printed on a Microsoft Learn page: S_OK, E_INVALIDARG, E_OUTOFMEMORY, E_UNEXPECTED (Common HRESULT Values),
 * S_FALSE (Error Handling in COM), the DISP_E_* values (COM Error Codes, Dispatch), RPC_E_CHANGED_MODE (COM Error
 * Codes, RPC), CO_E_CLASSSTRING and REGDB_E_CLASSNOTREG (COM Error Codes, Generic). NOERROR, which two pages name as
 * a return code, is S_OK (0). */
#ifndef FLE_COM_H
#define FLE_COM_H

#define FLE_HR_S_OK                 0x00000000u
#define FLE_HR_S_FALSE              0x00000001u
#define FLE_HR_E_UNEXPECTED         0x8000FFFFu
#define FLE_HR_E_INVALIDARG         0x80070057u
#define FLE_HR_E_OUTOFMEMORY        0x8007000Eu
#define FLE_HR_RPC_E_CHANGED_MODE   0x80010106u
#define FLE_HR_CO_E_CLASSSTRING     0x800401F3u
#define FLE_HR_REGDB_E_CLASSNOTREG  0x80040154u
#define FLE_HR_DISP_E_TYPEMISMATCH  0x80020005u
#define FLE_HR_DISP_E_BADVARTYPE    0x80020008u
#define FLE_HR_DISP_E_OVERFLOW      0x8002000Au
#define FLE_HR_DISP_E_BADINDEX      0x8002000Bu
#define FLE_HR_DISP_E_ARRAYISLOCKED 0x8002000Du

#endif
