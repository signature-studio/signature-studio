/* The virtual input locale identifier (docs/decisions/0063-virtual-keyboard-layout.md). Stateless. idThread 0 is answered;
 * every other idThread stops the run through fle_stop (fle_env.h). Nothing here reads the host: no keyboard, no layout, no input
 * method, no display server, no registry, no clock, no thread. */
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_locale.h"
#include "fle_kbd.h"

uint64_t fle_kbd_hkl(void)
{
    return ((uint64_t)FLE_KBD_HKL_HIGH_WORD << 16) | (uint64_t)(fle_emulated_locale.lcid & 0xFFFFu);
}

/* user32!GetKeyboardLayout: HKL f(DWORD idThread ecx). Learn: idThread is "The identifier of the thread to query, or 0 for the
 * current thread"; the return value is "the input locale identifier for the thread. The low word contains a Language
 * Identifier for the input language and the high word contains a device handle to the physical layout of the keyboard." The
 * page says nothing about a failure, so no failure is made up. Only the low DWORD of rcx is the argument (DWORD). Only 0 is
 * answered (the guest's three call sites pass 0): this environment has one thread and no thread table (decisions 0017, 0056),
 * so a thread identifier is not resolved, the virtual GetCurrentThreadId value FLE_EMULATED_TID included. The value is the
 * 64-bit HKL of fle_kbd_hkl(). The last error is left alone. */
void fle_imp_GetKeyboardLayout(void)
{
    static const char api[] = "GetKeyboardLayout";
    uint32_t id = (uint32_t)R[1];
    char why[200];

    if (id == FLE_EMULATED_TID)
        fle_stop(api, "idThread is the virtual GetCurrentThreadId value: there is no thread table to resolve it, only idThread 0 is answered (decisions 0056, 0063)");
    if (id != 0) {
        snprintf(why, sizeof why, "idThread %u is not 0: there is no thread table, only idThread 0 (the current thread) is answered (decision 0063)", (unsigned)id);
        fle_stop(api, why);
    }
    R[0] = fle_kbd_hkl();
}
