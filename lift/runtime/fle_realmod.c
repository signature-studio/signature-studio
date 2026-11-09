/* Real modules (see fle_realmod.h; docs/decisions/0082-two-module-load-semantics.md). The mechanism was measured on a synthetic PE in
 * docs/investigations/stage1-two-module.md; the behaviours below that are not in the decision are marked "not decided" and stop or
 * copy that measurement. Standard C only (native and wasm). */
#define FLE_REALMOD_IMPL
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_modules.h"
#include "fle_realmod.h"

/* The number of real modules: 8, as since decision 0106 (decision 0206 measured that a set of 11 images does not fit; the handles of fle_loadimage.h, stride 0x1000 in a 1 MiB region, allow 256). */
#define RM_MAX 8u

/* No real module unless the link brings its own fle_realmod_table and fle_realmod_count. */
const fle_realmod_entry fle_realmod_table[] __attribute__((weak)) = { { NULL, 0 } };
const uint64_t fle_realmod_count __attribute__((weak)) = 0;

enum { ST_NOT_LOADED = 0, ST_LOADING = 1, ST_LOADED = 2 };

typedef struct {
    uint64_t entry;     /* VA of AddressOfEntryPoint (DllMain) */
    uint64_t exp_rva;   /* export directory */
    uint64_t exp_size;
    unsigned state;
    uint32_t refs;
    int headers;        /* PE headers read */
} rm_t;

static rm_t mods[RM_MAX];
static int table_ready;

/* Checks the table once, on the first lookup: a wrong table is a build error, so it stops the run by name. */
static void table_check(const char *api)
{
    uint64_t i, j, n = fle_realmod_count;
    char why[200];

    if (table_ready) return;
    if (n > RM_MAX) fle_stop(api, "fle_realmod_count is larger than the 8 real modules this runtime holds");
    for (i = 0; i < n; i++) {
        const char *name = fle_realmod_table[i].name;
        uint64_t base = fle_realmod_table[i].base;
        const char *p;
        if (name == NULL || name[0] == 0) fle_stop(api, "a real module has no name");
        for (p = name; *p; p++)
            if (*p >= 'A' && *p <= 'Z') fle_stop(api, "a real module name is not lower-case");
        if (strchr(name, '\\') || strchr(name, '/') || strchr(name, ':')) fle_stop(api, "a real module name has a path");
        for (j = 0; j < fle_module_count(); j++) {
            /* a deferred module (decision 0096) is not in the way: a real module of its name wins over it */
            if (strcmp(name, fle_module_name(j)) == 0 && !fle_module_is_deferred(j)) {
                snprintf(why, sizeof why, "real module \"%s\" has the name of a virtual module", name);
                fle_stop(api, why);
            }
        }
        if (base == 0 || (base & 0xFFFFu) != 0 || base == FLE_EXE_HANDLE || base >= FLE_MODULE_HANDLE_BASE) {
            snprintf(why, sizeof why, "real module \"%s\": base %#llx is not usable (a nonzero multiple of 64 K, not 0x400000, below the virtual module handles)",
                     name, (unsigned long long)base);
            fle_stop(api, why);
        }
        for (j = 0; j < i; j++) {
            if (strcmp(name, fle_realmod_table[j].name) == 0 || base == fle_realmod_table[j].base) {
                snprintf(why, sizeof why, "real module \"%s\" repeats a name or a base of an earlier entry", name);
                fle_stop(api, why);
            }
        }
    }
    table_ready = 1;
}

/* Decision 0183 (wrp1, PROVISIONAL): see fle_realmod.h. No entry unless the link brings its own table. */
const fle_realmod_path_entry fle_realmod_path_table[] __attribute__((weak)) = { { NULL, NULL } };
const uint64_t fle_realmod_path_count __attribute__((weak)) = 0;

int fle_realmod_find_path(const uint16_t *u, size_t n)
{
    char buf[520];
    size_t i, k;
    uint64_t e;
    if (n == 0 || n >= sizeof buf) return -1;
    for (i = 0; i < n; i++) {
        uint16_t c = u[i];
        if (c > 0x7f) return -1;
        if (c >= 'A' && c <= 'Z') c = (uint16_t)(c - 'A' + 'a');
        if (c == '/') c = '\\';
        buf[i] = (char)c;
    }
    buf[n] = 0;
    for (e = 0; e < fle_realmod_path_count; e++) {
        const char *suf = fle_realmod_path_table[e].suffix;
        size_t sl = strlen(suf);
        if (sl > n) continue;
        k = n - sl;
        if (strcmp(buf + k, suf) != 0) continue;
        if (k > 0 && buf[k - 1] != '\\') continue;
        return fle_realmod_find(fle_realmod_path_table[e].name);
    }
    return -1;
}

int fle_realmod_find(const char *name)
{
    uint64_t i;
    table_check("fle_realmod_find");
    for (i = 0; i < fle_realmod_count; i++)
        if (strcmp(fle_realmod_table[i].name, name) == 0) return (int)i;
    return -1;
}

uint64_t fle_realmod_base_of(int idx)
{
    table_check("fle_realmod_base_of");
    return idx >= 0 && (uint64_t)idx < fle_realmod_count ? fle_realmod_table[idx].base : 0;
}

uint64_t fle_realmod_total(void)
{
    table_check("fle_realmod_total");
    return fle_realmod_count;
}

const char *fle_realmod_name(int idx)
{
    table_check("fle_realmod_name");
    return fle_realmod_table[idx].name;
}

uint64_t fle_realmod_base(int idx)
{
    table_check("fle_realmod_base");
    return fle_realmod_table[idx].base;
}

int fle_realmod_find_handle(uint64_t h)
{
    uint64_t i;
    table_check("fle_realmod_find_handle");
    for (i = 0; i < fle_realmod_count; i++)
        if (fle_realmod_table[i].base == h) return (int)i;
    return -1;
}

/* The PE headers of the image that the harness mapped at the base: DOS and NT signatures, PE32+, entry point and export directory. */
static void read_headers(const char *api, int idx)
{
    rm_t *m = &mods[idx];
    uint64_t base = fle_realmod_table[idx].base, lfanew, opt, rva;
    char why[200];

    if (m->headers) return;
    if ((uint16_t)fle_ld(base, 2) != 0x5a4d || (lfanew = fle_ld(base + 0x3c, 4)) < 0x40 || lfanew > 0x1000 ||
        (uint32_t)fle_ld(base + lfanew, 4) != 0x4550) {
        snprintf(why, sizeof why, "real module \"%s\": no PE image is mapped at %#llx", fle_realmod_table[idx].name, (unsigned long long)base);
        fle_stop(api, why);
    }
    opt = base + lfanew + 24;
    if ((uint16_t)fle_ld(opt, 2) != 0x20b) fle_stop(api, "a real module is not a PE32+ image");
    rva = fle_ld(opt + 16, 4);
    if (rva == 0) fle_stop(api, "a real module has no entry point (a DLL without DllMain is not modelled)");
    m->entry = base + rva;
    m->exp_rva = fle_ld(opt + 112, 4);
    m->exp_size = fle_ld(opt + 116, 4);
    m->headers = 1;
}

/* DllMain(hinstDLL = base, reason, lpvReserved = NULL): lpvReserved is NULL for a dynamic load and for the call that follows a failed
 * load (Learn, DllMain). fle_call_guest needs R[4] to be the stack pointer of the running API, which it is inside LoadLibrary*. */
/* Decision 0183 (wrp1, PROVISIONAL, OPT-IN): the loader's work that precedes the first DllMain(DLL_PROCESS_ATTACH) of a module with a TLS directory (the TLS callbacks run first, Learn: PE format, "TLS Callback Functions").
 * A link that defines fle_realmod_pre_attach (harness/two/wrp1_vst3.c) gets it called with the module's base and table index; without it nothing happens. */
void fle_realmod_pre_attach(uint64_t base, int idx) __attribute__((weak));

/* Decision 0200 (wrp3, OPT-IN): the loader's per-thread work for a module with implicit TLS (the TLS block of the starting thread; the TLS callbacks with DLL_THREAD_ATTACH / DLL_THREAD_DETACH) is done by a link that
 * defines fle_realmod_pre_thread (harness/two/wrp1_vst3.c), called in the context of the thread that starts (reason 2) or ends (reason 3), before the DllMain notifications and whatever the load state of the modules is
 * (a thread that started before the module was loaded also has a block: Learn, PE format, the .tls section, Windows Vista and later). Without it nothing happens. */
void fle_realmod_pre_thread(uint32_t reason) __attribute__((weak));

static uint32_t call_dllmain(int idx, uint32_t reason)
{
    return (uint32_t)fle_call_guest(mods[idx].entry, fle_realmod_table[idx].base, reason, 0, 0);
}

uint64_t fle_realmod_load(const char *api, int idx)
{
    rm_t *m = &mods[idx];
    uint64_t base = fle_realmod_table[idx].base;

    if (m->state == ST_LOADED) {
        m->refs++;
        return base;
    }
    if (m->state == ST_LOADING) {
        /* Not decided (decision 0082: the count of a same-name load from DllMain is open). The measured choice of the stage 1 prototype:
         * one more reference, no second DllMain entry. */
        m->refs++;
        return base;
    }
    read_headers(api, idx);
    m->state = ST_LOADING;
    m->refs = 1;
    if (fle_realmod_pre_attach) fle_realmod_pre_attach(base, idx);
    if (call_dllmain(idx, 1) == 0) {
        call_dllmain(idx, 0);
        m->state = ST_NOT_LOADED;
        m->refs = 0;
        return 0;
    }
    m->state = ST_LOADED;
    return base;
}

/* Decision 0122 (PM item 6): DLL_THREAD_ATTACH / DLL_THREAD_DETACH to the real modules, in the context of the running thread. Learn (DllMain): DLL_THREAD_ATTACH "The current process is
 * creating a new thread. When this occurs, the system calls the entry-point function of all DLLs currently attached to the process. The call is made in the context of the new thread";
 * DLL_THREAD_DETACH is the same for a thread that exits ("The entry-point function of all attached dynamic-link libraries (DLLs) is invoked with a value indicating that the thread is
 * detaching from the DLL", ExitThread). A module that is being loaded (ST_LOADING) or not loaded is not attached. The order (load order for attach, the reverse for detach) is a
 * choice (Learn gives none). lpvReserved is NULL (the page defines it for PROCESS_ATTACH and PROCESS_DETACH only). The return value of DllMain for these reasons is ignored (the page:
 * "The return value is ignored" is for THREAD notifications: it says the return value matters for DLL_PROCESS_ATTACH only). The virtual modules have no DllMain and receive nothing. */
void fle_realmod_thread_notify(uint32_t reason)
{
    if (fle_realmod_pre_thread) fle_realmod_pre_thread(reason);
    int i;

    table_check("fle_realmod_thread_notify");
    if (reason == 2) {
        for (i = 0; i < (int)fle_realmod_count; i++)
            if (mods[i].state == ST_LOADED && mods[i].headers) (void)call_dllmain(i, 2);
    } else {
        for (i = (int)fle_realmod_count - 1; i >= 0; i--)
            if (mods[i].state == ST_LOADED && mods[i].headers) (void)call_dllmain(i, 3);
    }
}

uint64_t fle_realmod_free(const char *api, int idx)
{
    rm_t *m = &mods[idx];

    if (m->state != ST_LOADED) fle_stop(api, "FreeLibrary of a real module that is not loaded (not modelled)");
    /* Decision 0206 (dec1, PROVISIONAL, OPT-IN: -DFLE_REALMOD_FREE_RESIDENT; default = the stop below, decision 0082 D11): FreeLibrary of the last reference keeps the module mapped and loaded (refs stays 1, DllMain is not
     * called with DLL_PROCESS_DETACH, a later LoadLibraryW returns the same handle without a new DllMain) and returns TRUE. The engine frees DelZip64.dll after each DZ_Exec call; the module keeps no state between calls (S-dec1-free). */
    if (m->refs <= 1) return 1;
    m->refs--;
    return 1;
}

uint64_t fle_realmod_module_handle(const char *api, int idx)
{
    (void)api;
    /* Only a fully attached module is found; during its own DllMain (ST_LOADING) the answer is "not loaded", as in the stage 1 prototype
     * (not decided). */
    return mods[idx].state == ST_LOADED ? fle_realmod_table[idx].base : 0;
}

int fle_realmod_file_name(const char *api, int idx, char *out, unsigned n)
{
    rm_t *m = &mods[idx];
    uint64_t base = fle_realmod_table[idx].base, np;
    unsigned k;

    if (m->state == ST_NOT_LOADED) fle_stop(api, "the file name of a real module that is not loaded");
    if (m->exp_rva == 0 || m->exp_size < 16) return -1;
    np = base + fle_ld(base + m->exp_rva + 12, 4);
    for (k = 0; k + 1 < n; k++) {
        uint8_t c = (uint8_t)fle_ld(np + k, 1);
        if (c >= 0x80) return -1;
        out[k] = (char)c;
        if (c == 0) return (int)k;
    }
    return -1;
}

uint64_t fle_realmod_getproc(const char *api, int idx, const char *name)
{
    rm_t *m = &mods[idx];
    uint64_t base = fle_realmod_table[idx].base, e, nfunc, nnames, afunc, anames, aord, i;

    if (m->state == ST_NOT_LOADED) fle_stop(api, "GetProcAddress on a real module that is not loaded");
    if (m->exp_rva == 0) return 0;
    if (m->exp_size < 40) fle_stop(api, "the export directory of a real module is too small");
    e = base + m->exp_rva;
    nfunc = fle_ld(e + 20, 4);
    nnames = fle_ld(e + 24, 4);
    afunc = base + fle_ld(e + 28, 4);
    anames = base + fle_ld(e + 32, 4);
    aord = base + fle_ld(e + 36, 4);
    for (i = 0; i < nnames; i++) {
        uint64_t np = base + fle_ld(anames + 4 * i, 4), k;
        for (k = 0;; k++) {
            uint8_t c = (uint8_t)fle_ld(np + k, 1);
            if (c != (uint8_t)name[k]) break;
            if (c == 0) {
                uint64_t ord = fle_ld(aord + 2 * i, 2), frva;
                if (ord >= nfunc) fle_stop(api, "an export name of a real module points past its function table");
                frva = fle_ld(afunc + 4 * ord, 4);
                if (frva == 0) fle_stop(api, "an export of a real module has a zero RVA");
                if (frva >= m->exp_rva && frva < m->exp_rva + m->exp_size) fle_stop(api, "an export forwarder of a real module is not modelled");
                return base + frva;
            }
        }
    }
    return 0;
}
