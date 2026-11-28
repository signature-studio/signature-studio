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
#include "fle_thread.h"

/* fle_vfs.c's file lookup (weak: a link without the virtual volume has no file). */
extern int fle_vfs_peek_file(const uint16_t *wpath, const uint8_t **data, uint64_t *size) __attribute__((weak));

/* The number of real modules: 192 (Issue #1379: the second wave of plugins, 160 DLLs, makes 170 rows of modules.tsv; 16 before, G9-c step; 8 since decision 0106 until the plugin Fruity Balance made the eighth; decision 0206 measured that a set of 11 images did not fit in 8; the handles of fle_loadimage.h, stride 0x1000 in a 1 MiB region, allow 255). */
#define RM_MAX 192u

/* No real module unless the link brings its own fle_realmod_table and fle_realmod_count. */
const fle_realmod_entry fle_realmod_table[] __attribute__((weak)) = { { NULL, 0 } };
const uint64_t fle_realmod_count __attribute__((weak)) = 0;

enum { ST_NOT_LOADED = 0, ST_LOADING = 1, ST_LOADED = 2 };

typedef struct {
    uint64_t entry;     /* VA of AddressOfEntryPoint (DllMain) */
    uint64_t exp_rva;   /* export directory */
    uint64_t exp_size;
    uint32_t size;      /* SizeOfImage */
    unsigned state;
    uint32_t refs;
    int headers;        /* PE headers read */
    int program;        /* Issue #1499: an .exe image (no IMAGE_FILE_DLL): started by ShellExecute, never by LoadLibrary */
    char *file;         /* Issue #1499: a program's full path (ASCII), the file it was started from; made at its first start */
} rm_t;

static rm_t mods[RM_MAX];
static int table_ready;

/* Checks the table once, on the first lookup: a wrong table is a build error, so it stops the run by name. */
static void table_check(const char *api)
{
    uint64_t i, j, n = fle_realmod_count;
    char why[512]; /* the messages hold a name of 255 characters (Issue #1523) */

    if (table_ready) return;
    if (n > RM_MAX) fle_stop(api, "fle_realmod_count is larger than the 192 real modules this runtime holds");
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

/* The path-table entry whose suffix the path u[0..n) ends with (the rule of fle_realmod_find_path), or -1. */
static int path_entry_of(const uint16_t *u, size_t n)
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
        return (int)e;
    }
    return -1;
}

int fle_realmod_find_path(const uint16_t *u, size_t n)
{
    int e = path_entry_of(u, n);
    return e < 0 ? -1 : fle_realmod_find(fle_realmod_path_table[e].name);
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

/* Issue #1341: the image of a real module is mapped when the guest first asks for the module (LoadLibrary), not at start. The link may define this (harness/two/main_two.c): it reads the module's file and maps the
 * PE image at the table's base, before the headers are read below. A link without it maps every image at start, as before. */
void fle_realmod_map_image(const char *api, int idx) __attribute__((weak));

/* 1 when the image of real module idx is in memory: always, when the link maps every image at start (no fle_realmod_map_image); otherwise once its headers are read (fle_realmod_load, the first LoadLibrary). */
int fle_realmod_image_mapped(int idx)
{
    table_check("fle_realmod_image_mapped");
    return idx >= 0 && (uint64_t)idx < fle_realmod_count && (!fle_realmod_map_image || mods[idx].headers);
}

/* The PE headers of the image that the harness mapped at the base: DOS and NT signatures, PE32+, entry point and export directory. */
static void read_headers(const char *api, int idx)
{
    rm_t *m = &mods[idx];
    uint64_t base = fle_realmod_table[idx].base, lfanew, opt, rva;
    char why[512]; /* the message holds a name of 255 characters (Issue #1523) */

    if (m->headers) return;
    if (fle_realmod_map_image) fle_realmod_map_image(api, idx);
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
    m->size = (uint32_t)fle_ld(opt + 56, 4);
    m->program = (fle_ld(base + lfanew + 22, 2) & 0x2000u) == 0;   /* FileHeader.Characteristics: IMAGE_FILE_DLL 0x2000 is clear for an .exe */
    m->headers = 1;
}

/* 1 when va lies in a section of the image at base that has IMAGE_SCN_MEM_EXECUTE (0x20000000). The section table follows the optional header (PE format). */
static int in_code_section(uint64_t base, uint64_t va)
{
    uint64_t nt = base + fle_ld(base + 0x3c, 4), sh, rva = va - base, n, k;

    n = fle_ld(nt + 6, 2);
    sh = nt + 24 + fle_ld(nt + 20, 2);
    for (k = 0; k < n; k++, sh += 40) {
        uint64_t vsize = fle_ld(sh + 8, 4), vaddr = fle_ld(sh + 12, 4);
        if (rva >= vaddr && rva - vaddr < vsize && (fle_ld(sh + 36, 4) & 0x20000000u)) return 1;
    }
    return 0;
}

/* Issue #1421: a `jmp [rip+disp32]` (FF 25, or 48 FF 25 as MSVC writes it in a thunk) in an executable section of a real module's image is the jump
 * through its slot, which is what the lifted code of that instruction does. The lifter makes function entries of the code that a guest reaches by a
 * call; a thunk that follows the padding of another function and is taken only by its address (a `lea`, then an indirect call or a jump through a
 * stored pointer) is not one, and an indirect call to it misses the table (fle_dispatch_miss, which asks fle_win_try_thunk). 1 and *target = the slot's
 * value when va is such a thunk, else 0 (the caller then stops by name, as before). */
int fle_realmod_jmp_thunk(uint64_t va, uint64_t *target)
{
    uint64_t i, p;

    table_check("fle_realmod_jmp_thunk");
    for (i = 0; i < fle_realmod_count; i++) {
        const rm_t *m = &mods[i];
        uint64_t base = fle_realmod_table[i].base;

        if (!m->headers || va < base || va - base >= m->size) continue;
        if (!in_code_section(base, va)) return 0;
        p = va + ((fle_ld(va, 1) & 0xF0u) == 0x40u ? 1 : 0);
        if (fle_ld(p, 1) != 0xFFu || fle_ld(p + 1, 1) != 0x25u) return 0;
        *target = fle_ld(p + 6 + (uint64_t)(int64_t)(int32_t)(uint32_t)fle_ld(p + 2, 4), 8);
        return 1;
    }
    return 0;
}

/* DllMain(hinstDLL = base, reason, lpvReserved = NULL): lpvReserved is NULL for a dynamic load and for the call that follows a failed
 * load (Learn, DllMain). fle_call_guest needs R[4] to be the stack pointer of the running API, which it is inside LoadLibrary*. */
/* Decision 0183 (wrp1, PROVISIONAL): the loader's work that precedes the first DllMain(DLL_PROCESS_ATTACH) of a module with a TLS directory (the TLS callbacks run first, Learn: PE format, "TLS Callback Functions").
 * The link defines fle_realmod_pre_attach (harness/two/wrp1_vst3.c, Issue #1326), which gets it called with the module's base and table index; the weak reference only keeps a link without that file building. */
void fle_realmod_pre_attach(uint64_t base, int idx) __attribute__((weak));

/* Decision 0200 (wrp3): the loader's per-thread work for a module with implicit TLS (the TLS block of the starting thread; the TLS callbacks with DLL_THREAD_ATTACH / DLL_THREAD_DETACH) is done by the link's
 * fle_realmod_pre_thread (harness/two/wrp1_vst3.c, Issue #1326), called in the context of the thread that starts (reason 2) or ends (reason 3), before the DllMain notifications and whatever the load state of the modules is
 * (a thread that started before the module was loaded also has a block: Learn, PE format, the .tls section, Windows Vista and later). */
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
    if (m->program) fle_stop(api, "LoadLibrary of a program (an .exe image of the table): only ShellExecute starts it (fle_realmod_shell_program); loading it as a DLL is not modelled");
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

int fle_realmod_loaded(int idx)
{
    return mods[idx].state != ST_NOT_LOADED;
}

uint64_t fle_realmod_getproc(const char *api, int idx, const char *name)
{
    rm_t *m = &mods[idx];
    uint64_t base = fle_realmod_table[idx].base, e, nfunc, nnames, afunc, anames, aord, i;

    if (m->state == ST_NOT_LOADED) return 0;   /* Issue #1418: the caller answers a module that is not loaded (fle_modules.c, ERROR_MOD_NOT_FOUND) */
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

/* ---- Issue #1499 (段 B): a program of the table (PluginManager.exe) started by ShellExecute ----
 * The start is a new cooperative thread (fle_thread_spawn) whose body calls the image's entry point (AddressOfEntryPoint) with no argument: the start-up code of an
 * .exe takes none. The thread is the program's process here: GetModuleHandle(NULL) on it gives the program's base (fle_realmod_running_base), GetModuleFileNameW of
 * the base is the path it was started from (fle_realmod_program_file), GetCommandLineW/A on it is the quoted path and the parameters of ShellExecute
 * (fle_realmod_running_cmdline_w/a), and ExitProcess on it ends only that thread (fle_realmod_program_end); the engine's threads are not touched.
 * Choices without a source (S-ia2-program-*): the directory of ShellExecute is not passed on (the program's current directory is the engine's), and at most
 * PROG_MAX programs run at once. */
#define PROG_MAX 8u
#define PROG_STR_MAX 1024u   /* a path, verb or parameter string of ShellExecute at or above this length is not a program's (the call is answered as before) */
#define PROG_CMD_MAX (2u * PROG_STR_MAX + 8u)   /* the quoted path, a space and the parameters, with the NUL */

typedef struct {
    int used;                /* the slot is taken: from the start until the thread ends */
    int started;             /* the thread has begun its body: tidx is its index */
    unsigned tidx;
    int idx;                 /* the program's real module */
    uint32_t exit_code;
    uint16_t cw[PROG_CMD_MAX];   /* the command line of the program (GetCommandLineW) */
    char ca[PROG_CMD_MAX];       /* the same, ASCII (GetCommandLineA) */
} prog_t;
static prog_t progs[PROG_MAX];

/* The slot of the program whose thread is the running one, or -1. */
static int program_slot_running(void)
{
    unsigned s;
    for (s = 0; s < PROG_MAX; s++)
        if (progs[s].used && progs[s].started && progs[s].tidx == fle_thread_current()) return (int)s;
    return -1;
}

static void program_body(uint64_t slot)
{
    prog_t *p = &progs[slot];

    p->tidx = fle_thread_current();
    p->started = 1;
    p->exit_code = (uint32_t)fle_call_guest(mods[p->idx].entry, 0, 0, 0, 0);
    p->used = 0;
    p->started = 0;
}

/* Copies the NUL-terminated UTF-16 string at the guest address p to out (at most cap - 1 units). Returns its length; cap - 1 when it is not shorter than that. */
static size_t guest_wide(uint64_t p, uint16_t *out, size_t cap)
{
    size_t i;
    for (i = 0; i + 1 < cap; i++) {
        uint16_t c = (uint16_t)fle_ld(p + 2u * i, 2);
        if (c == 0) break;
        out[i] = c;
    }
    out[i] = 0;
    return i;
}

static void ascii16(char *out, size_t cap, const uint16_t *u, size_t n)
{
    size_t i;
    for (i = 0; i < n && i + 1 < cap; i++) out[i] = (char)(u[i] >= 0x20 && u[i] < 0x7F ? u[i] : '?');
    out[i] = 0;
}

/* The verb "open" (ASCII case folded). */
static int verb_is_open(const uint16_t *v, size_t n)
{
    static const char open[] = "open";
    size_t i;
    if (n != 4) return 0;
    for (i = 0; i < 4; i++) {
        uint16_t c = v[i];
        if (c >= 'A' && c <= 'Z') c = (uint16_t)(c - 'A' + 'a');
        if (c != (uint16_t)(unsigned char)open[i]) return 0;
    }
    return 1;
}

/* The real module of the program that the path u[0..n) names (a path-table entry with the suffix ".exe"), or -1. */
static int program_of_path(const uint16_t *u, size_t n)
{
    int e = path_entry_of(u, n);
    const char *suf;
    size_t sl;

    if (e < 0) return -1;
    suf = fle_realmod_path_table[e].suffix;
    sl = strlen(suf);
    if (sl < 4 || strcmp(suf + sl - 4, ".exe") != 0) return -1;
    return fle_realmod_find(fle_realmod_path_table[e].name);
}

int fle_realmod_shell_program(const char *api, uint64_t verb, uint64_t file, uint64_t params, uint64_t *thread)
{
    uint16_t fu[PROG_STR_MAX], vu[PROG_STR_MAX], pu[PROG_STR_MAX];
    char shown[PROG_STR_MAX], shown_par[PROG_STR_MAX];
    const uint8_t *data;
    uint64_t sz, h;
    size_t n, vn = 0, pn;
    int idx, s;

    if (file == 0 || !fle_thread_spawn) return 0;
    n = guest_wide(file, fu, PROG_STR_MAX);
    if (n >= PROG_STR_MAX - 1) return 0;
    if (verb != 0) {
        vn = guest_wide(verb, vu, PROG_STR_MAX);
        if (!verb_is_open(vu, vn)) return 0;
    }
    idx = program_of_path(fu, n);
    if (idx < 0 || !fle_vfs_peek_file || !fle_vfs_peek_file(fu, &data, &sz)) return 0;
    pn = params != 0 ? guest_wide(params, pu, PROG_STR_MAX) : 0;
    read_headers(api, idx);
    if (!mods[idx].program) fle_stop(api, "the path's file of the real module table is a DLL, not a program (the table and the file disagree)");
    if (mods[idx].file == NULL) {
        mods[idx].file = malloc(n + 1);
        if (mods[idx].file == NULL) fle_stop(api, "out of memory for the program's file name");
        ascii16(mods[idx].file, n + 1, fu, n);
    }
    for (s = 0; s < (int)PROG_MAX && progs[s].used; s++) {}
    if (s == (int)PROG_MAX) fle_stop(api, "more programs are running at once than the engine holds (PROG_MAX)");
    progs[s].used = 1;
    progs[s].started = 0;
    progs[s].tidx = 0;
    progs[s].idx = idx;
    progs[s].exit_code = 0;
    {   /* the command line: the quoted path, then the parameters (a space between them unless the parameters begin with one) */
        size_t k = 0, j;
        progs[s].cw[k++] = '"';
        for (j = 0; j < n; j++) progs[s].cw[k++] = fu[j];
        progs[s].cw[k++] = '"';
        if (pn > 0) {
            if (pu[0] != ' ') progs[s].cw[k++] = ' ';
            for (j = 0; j < pn; j++) progs[s].cw[k++] = pu[j];
        }
        progs[s].cw[k] = 0;
        ascii16(progs[s].ca, sizeof progs[s].ca, progs[s].cw, k);
    }
    h = fle_thread_spawn(program_body, (uint64_t)s);
    if (h == 0) {
        progs[s].used = 0;
        fle_stop(api, "no thread could be made for the program (the thread table is full)");
    }
    ascii16(shown, sizeof shown, fu, n);
    ascii16(shown_par, sizeof shown_par, pu, pn);
    fprintf(stderr, "fle: %s starts the program \"%s\" as a thread of this engine (verb \"%s\"; parameters \"%s\" are its command line's; the directory is not passed on; S-ia2-program-start)\n",
            api, shown, verb != 0 ? "open" : "(none)", shown_par);
    *thread = h;
    return 1;
}

void fle_realmod_program_end(uint32_t code)
{
    int s = program_slot_running();

    if (s < 0) return;
    progs[s].exit_code = code;
    progs[s].used = 0;
    progs[s].started = 0;
    fle_thread_end_running(code);
}

uint64_t fle_realmod_running_base(void)
{
    int s = program_slot_running();

    return s < 0 ? 0 : fle_realmod_table[progs[s].idx].base;
}

const uint16_t *fle_realmod_running_cmdline_w(void)
{
    int s = program_slot_running();

    return s < 0 ? NULL : progs[s].cw;
}

const char *fle_realmod_running_cmdline_a(void)
{
    int s = program_slot_running();

    return s < 0 ? NULL : progs[s].ca;
}

/* Issue #1499 (段 B): when addr..addr+size lies in a program's image, the protection of that page (PAGE_EXECUTE_READ 0x20 in an executable section, else PAGE_READWRITE 0x04); 0 otherwise. */
uint32_t fle_realmod_program_protect(uint64_t addr, uint64_t size)
{
    uint64_t i;

    for (i = 0; i < fle_realmod_count; i++) {
        const rm_t *m = &mods[i];
        uint64_t base = fle_realmod_table[i].base;

        if (!m->program || addr < base || addr - base >= m->size) continue;
        if (size > m->size || addr - base > m->size - size) return 0;
        return in_code_section(base, addr) ? 0x20u : 0x04u;
    }
    return 0;
}

/* Issue #1501 (段 B, S-version-info-program-resource): the base of the program whose path u[0..n) names, when that program's image is in memory (it was started), else 0. */
uint64_t fle_realmod_program_image_base(const uint16_t *u, size_t n)
{
    int idx = program_of_path(u, n);

    if (idx < 0 || !mods[idx].program || !mods[idx].headers) return 0;
    return fle_realmod_table[idx].base;
}

int fle_realmod_program_file(int idx, char *out, unsigned n)
{
    size_t len;

    if (idx < 0 || (uint64_t)idx >= fle_realmod_count || !mods[idx].program || mods[idx].file == NULL) return -1;
    len = strlen(mods[idx].file);
    if (len + 1 > n) return -1;
    memcpy(out, mods[idx].file, len + 1);
    return (int)len;
}
