/* Virtual environment block, command line and startup info (docs/decisions/0018-virtual-fs-reg-env.md,
 * docs/investigations/import-vfs-spec.md for the sources). The host environment is never read. */
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"

void fle_stop(const char *api, const char *why)
{
    fprintf(stderr, "fle: %s: %s\n", api, why);
    abort();
}

size_t fle_w_len(const uint16_t *s)
{
    size_t n = 0;
    while (s[n]) n++;
    return n;
}

uint16_t fle_w_fold(uint16_t c)
{
    return (c >= 'A' && c <= 'Z') ? (uint16_t)(c + 32) : c;
}

int fle_w_icmp(const uint16_t *a, size_t an, const uint16_t *b, size_t bn)
{
    size_t i, n = an < bn ? an : bn;
    for (i = 0; i < n; i++) {
        uint16_t x = fle_w_fold(a[i]), y = fle_w_fold(b[i]);
        if (x != y) return x < y ? -1 : 1;
    }
    if (an == bn) return 0;
    return an < bn ? -1 : 1;
}

size_t fle_w_read(const char *api, uint64_t guest, uint16_t *buf, size_t cap)
{
    size_t n;
    if (guest == 0) fle_stop(api, "NULL string pointer");
    for (n = 0;; n++) {
        if (n >= cap) fle_stop(api, "string longer than the supported limit");
        buf[n] = (uint16_t)fle_ld(guest + 2u * n, 2);
        if (buf[n] == 0) return n;
    }
}

uint16_t *fle_w_read_alloc(const char *api, uint64_t guest, size_t maxlen, size_t *len)
{
    size_t n = 0, i;
    uint16_t *p;
    if (guest == 0) fle_stop(api, "NULL string pointer");
    while (fle_ld(guest + 2u * n, 2) != 0) {
        n++;
        if (n > maxlen) fle_stop(api, "string longer than the supported limit");
    }
    p = malloc((n + 1) * sizeof *p);
    if (!p) fle_stop(api, "out of memory");
    for (i = 0; i <= n; i++) p[i] = (uint16_t)fle_ld(guest + 2u * i, 2);
    *len = n;
    return p;
}

void fle_w_write(uint64_t dst, const uint16_t *src, size_t units)
{
    memcpy((void *)fle_addr(dst, (unsigned)(units * sizeof *src), 1), src, units * sizeof *src);
}

uint16_t *fle_w_from_ascii(const char *s)
{
    size_t n = strlen(s), i;
    uint16_t *p = malloc((n + 1) * sizeof *p);
    if (!p) fle_stop("runtime", "out of memory");
    for (i = 0; i <= n; i++) p[i] = (uint8_t)s[i];
    return p;
}

/* ---- environment block ---- */

#define FLE_ENV_VALUE_MAX 32767u

typedef struct {
    uint16_t *name;
    size_t nlen;
    uint16_t *value;
} fle_env_entry;

static fle_env_entry *env_v;
static size_t env_n, env_cap;

static long env_find(const uint16_t *name, size_t nlen)
{
    size_t i;
    for (i = 0; i < env_n; i++)
        if (fle_w_icmp(env_v[i].name, env_v[i].nlen, name, nlen) == 0) return (long)i;
    return -1;
}

const uint16_t *fle_env_get(const uint16_t *name)
{
    long i = env_find(name, fle_w_len(name));
    return i < 0 ? NULL : env_v[i].value;
}

/* The name rules come from MSDN (Environment Variables: the name cannot include '='). A NULL, empty or
 * '='-containing name is not defined by the GetEnvironmentVariableW / SetEnvironmentVariable pages and stops
 * the run. */
static uint16_t *env_name(const char *api, uint64_t p, size_t *len)
{
    size_t i;
    uint16_t *n = fle_w_read_alloc(api, p, FLE_ENV_VALUE_MAX, len);
    if (*len == 0) fle_stop(api, "empty variable name is not defined");
    for (i = 0; i < *len; i++)
        if (n[i] == '=') fle_stop(api, "variable name containing '=' is not defined");
    return n;
}

/* kernel32!GetEnvironmentVariableW (MSDN: GetEnvironmentVariableW). lpName rcx, lpBuffer rdx, nSize r8.
 * Not found: return 0 and last error ERROR_ENVVAR_NOT_FOUND (203). Found: when nSize is larger than the length
 * (room for the NUL) the value and NUL are stored and the length without NUL is returned; otherwise the
 * required size including the NUL is returned and the buffer is left alone (the page calls it undefined).
 * A found variable with an empty value returns 0 and does not touch the last error (the page is silent). */
void fle_imp_GetEnvironmentVariableW(void)
{
    static const char api[] = "GetEnvironmentVariableW";
    uint64_t pb = R[2];
    uint32_t nsize = (uint32_t)R[8];
    size_t nl, vl;
    uint16_t *name = env_name(api, R[1], &nl);
    long i = env_find(name, nl);

    free(name);
    if (i < 0) {
        fle_set_last_error(FLE_W32_ERR_ENVVAR_NOT_FOUND);
        R[0] = 0;
        return;
    }
    vl = fle_w_len(env_v[i].value);
    if (nsize > vl) {
        if (pb == 0) fle_stop(api, "lpBuffer is NULL but nSize leaves room for the value");
        fle_w_write(pb, env_v[i].value, vl + 1);
        R[0] = (uint32_t)vl;
        return;
    }
    R[0] = (uint32_t)(vl + 1);
}

/* kernel32!SetEnvironmentVariableW (MSDN: SetEnvironmentVariable). lpName rcx, lpValue rdx. A NULL value deletes
 * the variable. Deleting a variable that is not there is not defined by the page and stops the run. A value
 * above 32,767 characters (the documented maximum) stops. Success returns 1. */
void fle_imp_SetEnvironmentVariableW(void)
{
    static const char api[] = "SetEnvironmentVariableW";
    size_t nl, vl = 0;
    uint16_t *name = env_name(api, R[1], &nl);
    uint16_t *val = NULL;
    long i = env_find(name, nl);

    if (R[2] != 0) {
        val = fle_w_read_alloc(api, R[2], FLE_ENV_VALUE_MAX, &vl);
    }
    if (val == NULL) {
        if (i < 0) fle_stop(api, "deleting a variable that does not exist is not defined");
        free(env_v[i].name);
        free(env_v[i].value);
        memmove(&env_v[i], &env_v[i + 1], (env_n - (size_t)i - 1) * sizeof env_v[0]);
        env_n--;
        free(name);
        R[0] = 1;
        return;
    }
    if (i >= 0) {
        free(env_v[i].value);
        env_v[i].value = val;
        free(name);
    } else {
        if (env_n == env_cap) {
            size_t nc = env_cap ? env_cap * 2 : 16;
            fle_env_entry *nv = realloc(env_v, nc * sizeof *nv);
            if (!nv) fle_stop(api, "out of memory");
            env_v = nv;
            env_cap = nc;
        }
        env_v[env_n].name = name;
        env_v[env_n].nlen = nl;
        env_v[env_n].value = val;
        env_n++;
    }
    R[0] = 1;
}

/* kernel32!ExpandEnvironmentStringsW (MSDN: ExpandEnvironmentStringsW). lpSrc rcx, lpDst rdx, nSize r8.
 * %name% is replaced by the value (case ignored); a name that is not found stays as written (page). The page
 * does not define: a '%' without a closing '%', an empty %%, lpSrc == lpDst (the page forbids it), a NULL lpDst
 * with nSize > 0. Those stop the run. After a name that is not found, scanning continues after its closing '%'
 * (inference; the page is silent). Return: the size including the NUL; when nSize is smaller than that, the
 * required size is returned and lpDst is left alone. */
#define FLE_EXPAND_MAX 0x100000u

void fle_imp_ExpandEnvironmentStringsW(void)
{
    static const char api[] = "ExpandEnvironmentStringsW";
    uint64_t src = R[1], dst = R[2];
    uint32_t nsize = (uint32_t)R[8];
    size_t sl, i = 0, on = 0, ocap = 256;
    uint16_t *s, *out;

    if (src == 0) fle_stop(api, "lpSrc is NULL");
    if (dst != 0 && dst == src) fle_stop(api, "lpDst equals lpSrc (the page forbids it)");
    if (dst == 0 && nsize > 0) fle_stop(api, "lpDst is NULL with nSize > 0");
    s = fle_w_read_alloc(api, src, FLE_ENV_VALUE_MAX * 2, &sl);
    out = malloc(ocap * sizeof *out);
    if (!out) fle_stop(api, "out of memory");

    while (i < sl) {
        size_t j, span, add;
        const uint16_t *piece;
        long e = -1;
        if (s[i] != '%') {
            span = 1;
            piece = &s[i];
            add = 1;
        } else {
            for (j = i + 1; j < sl && s[j] != '%'; j++) {}
            if (j >= sl) fle_stop(api, "'%' without a closing '%' is not defined");
            if (j == i + 1) fle_stop(api, "empty %% is not defined");
            e = env_find(&s[i + 1], j - i - 1);
            span = j - i + 1;
            if (e >= 0) {
                piece = env_v[e].value;
                add = fle_w_len(piece);
            } else {
                piece = &s[i];
                add = span;
            }
        }
        if (on + add + 1 > FLE_EXPAND_MAX) fle_stop(api, "expansion longer than the supported limit");
        while (on + add + 1 > ocap) {
            uint16_t *no;
            ocap *= 2;
            no = realloc(out, ocap * sizeof *out);
            if (!no) fle_stop(api, "out of memory");
            out = no;
        }
        memcpy(out + on, piece, add * sizeof *out);
        on += add;
        i += span;
    }
    out[on] = 0;
    if (dst != 0 && nsize >= on + 1) fle_w_write(dst, out, on + 1);
    R[0] = (uint32_t)(on + 1);
    free(out);
    free(s);
}

/* ---- command line and startup info ---- */

/* Decision 0144 (PROVISIONAL): FLE_CMDLINE (fle_appfile.c, weak: a program that does not link it is as before) replaces the value of GetCommandLineW / GetCommandLineA; unset or empty = the value below. */
const uint16_t *fle_appfile_cmdline_w(void) __attribute__((weak));
const char *fle_appfile_cmdline_a(void) __attribute__((weak));

/* kernel32!GetCommandLineW (MSDN: GetCommandLineW): a pointer to the process command line, owned by the system
 * (the guest must not free or modify it). The process here is the lifted DLL (GetModuleHandleW(NULL) returns it,
 * decision 0015), so the command line is its virtual path in quotes, no arguments. The page says the OS may
 * prepend a fully qualified path to the executable name: that is what the quoted path is. The value is a
 * decision of 0018, not a Windows rule. */
void fle_imp_GetCommandLineW(void)
{
    static uint16_t *cmd;
    if (cmd == NULL && fle_appfile_cmdline_w) cmd = (uint16_t *)(uintptr_t)fle_appfile_cmdline_w();
    if (cmd == NULL) {
        static const char q[] = "\"" FLE_VIRTUAL_DLL_PATH "\"";
        cmd = fle_w_from_ascii(q);
    }
    R[0] = (uint64_t)(uintptr_t)cmd;
}

/* kernel32!GetCommandLineA (MSDN: GetCommandLineA, decision 0078): "a pointer to the command-line string for the current process",
 * managed by the system; "a conversion of the Unicode command line to the 8-bit process code page". The Unicode command line is the
 * one of GetCommandLineW above, every character of which is ASCII, and the process code page of this environment (fle_locale.c: 1252)
 * keeps ASCII as it is (general knowledge for 1252, as the code in fle_locale.c already assumes), so the bytes are the same ASCII text. */
void fle_imp_GetCommandLineA(void)
{
    static char *cmd;
    if (cmd == NULL && fle_appfile_cmdline_a) {
        const char *t = fle_appfile_cmdline_a();
        if (t) {
            cmd = malloc(strlen(t) + 1);
            if (!cmd) fle_stop("GetCommandLineA", "out of memory");
            memcpy(cmd, t, strlen(t) + 1);
        }
    }
    if (cmd == NULL) {
        static const char q[] = "\"" FLE_VIRTUAL_DLL_PATH "\"";
        cmd = malloc(sizeof q);
        if (!cmd) fle_stop("GetCommandLineA", "out of memory");
        memcpy(cmd, q, sizeof q);
    }
    R[0] = (uint64_t)(uintptr_t)cmd;
}

/* kernel32!GetEnvironmentStringsW / FreeEnvironmentStringsW (MSDN, decision 0078). The page's block format is
 * "Var1=Value1\0 Var2=Value2\0 ... VarN=ValueN\0\0": each variable NUL-terminated and one more NUL at the end. A copy of the virtual
 * environment (it starts empty, so the first block is the single terminating NUL until SetEnvironmentVariableW adds one) is made on
 * every call, in the order of the table. "If the function fails, the return value is NULL" (a failed malloc). The block is read-only for
 * the guest and "should be freed by calling FreeEnvironmentStrings"; FreeEnvironmentStringsW takes "a pointer to a block of environment
 * strings ... obtained by calling the GetEnvironmentStringsW function": any other pointer is not defined by the page (it names no error
 * code for the failure) and stops the run. Success is nonzero. The last error is not touched. */
static uint16_t **env_blocks;
static size_t env_blocks_n, env_blocks_cap;

void fle_imp_GetEnvironmentStringsW(void)
{
    static const char api[] = "GetEnvironmentStringsW";
    size_t i, total = 1, at = 0;
    uint16_t *blk;

    for (i = 0; i < env_n; i++) total += env_v[i].nlen + 1 + fle_w_len(env_v[i].value) + 1;
    blk = malloc(total * sizeof *blk);
    if (!blk) {
        R[0] = 0;
        return;
    }
    for (i = 0; i < env_n; i++) {
        size_t vl = fle_w_len(env_v[i].value);
        memcpy(blk + at, env_v[i].name, env_v[i].nlen * sizeof *blk);
        at += env_v[i].nlen;
        blk[at++] = '=';
        memcpy(blk + at, env_v[i].value, vl * sizeof *blk);
        at += vl;
        blk[at++] = 0;
    }
    blk[at++] = 0;
    if (env_blocks_n == env_blocks_cap) {
        size_t nc = env_blocks_cap ? env_blocks_cap * 2 : 8;
        uint16_t **nb = realloc(env_blocks, nc * sizeof *nb);
        if (!nb) fle_stop(api, "out of memory");
        env_blocks = nb;
        env_blocks_cap = nc;
    }
    env_blocks[env_blocks_n++] = blk;
    R[0] = (uint64_t)(uintptr_t)blk;
}

void fle_imp_FreeEnvironmentStringsW(void)
{
    static const char api[] = "FreeEnvironmentStringsW";
    size_t i;
    uint64_t p = R[1];

    for (i = 0; i < env_blocks_n; i++) {
        if ((uint64_t)(uintptr_t)env_blocks[i] == p) {
            free(env_blocks[i]);
            env_blocks[i] = env_blocks[--env_blocks_n];
            R[0] = 1;
            return;
        }
    }
    fle_stop(api, "a pointer that GetEnvironmentStringsW did not return (or that was already freed); the page names no error code");
}

/* kernel32!GetStartupInfoW (MSDN: GetStartupInfoW, STARTUPINFOW). lpStartupInfo rcx. The page gives the members
 * and says the structure was specified by the creating process; there is none here, so every member is zero
 * (no flags, no window position or show state, no standard handles, NULL desktop and title) and cb is set to the
 * structure size. The size 0x68 follows from the member types on x64 (and is the value FLEngine stores in cb at
 * 0x40738d). cbReserved2 is 0 and lpReserved2 is NULL (no inherited handles). The page makes lpStartupInfo an [out]
 * parameter and states no rule for the incoming cb, so the incoming cb is not read: whatever it holds (the CRT of
 * dsp_ippv2_x64.dll passes an uninitialised buffer), all 0x68 bytes are zeroed and cb is set (decision 0083, provisional:
 * whether a real kernel32 reads the incoming cb is not in the primary sources). NULL has no rule on the page and stops. */
#define FLE_STARTUPINFOW_SIZE 0x68u

void fle_imp_GetStartupInfoW(void)
{
    static const char api[] = "GetStartupInfoW";
    uint64_t p = R[1];
    if (p == 0) fle_stop(api, "lpStartupInfo is NULL");
    memset((void *)fle_addr(p, FLE_STARTUPINFOW_SIZE, 1), 0, FLE_STARTUPINFOW_SIZE);
    fle_st(p, 4, FLE_STARTUPINFOW_SIZE);
}

/* ---- ANSI forms reached by a statically linked MSVC CRT start-up (decision 0102 W7, provisional) ---- */

/* kernel32!GetStartupInfoA (MSDN: GetStartupInfoA): STARTUPINFOA has the members of STARTUPINFOW with LPSTR for LPWSTR (8 bytes each on
 * x64), so it is the same 0x68 bytes at the same offsets; the fill is that of GetStartupInfoW (decision 0083). */
void fle_imp_GetStartupInfoA(void)
{
    static const char api[] = "GetStartupInfoA";
    uint64_t p = R[1];
    if (p == 0) fle_stop(api, "lpStartupInfo is NULL");
    memset((void *)fle_addr(p, FLE_STARTUPINFOW_SIZE, 1), 0, FLE_STARTUPINFOW_SIZE);
    fle_st(p, 4, FLE_STARTUPINFOW_SIZE);
}

/* kernel32!GetEnvironmentStrings, the ANSI export (MSDN: GetEnvironmentStringsW / FreeEnvironmentStrings; the ANSI name is that of the
 * same function, reading): the block of GetEnvironmentStringsW in the 8-bit process code page. The virtual environment is converted
 * character by character; a character at or above 0x80 stops the run (the code page conversion of decisions 0084/0091 is not applied
 * here). The block is freed by FreeEnvironmentStringsA, which accepts only a pointer this function returned. */
static char **envA_blocks;
static size_t envA_blocks_n, envA_blocks_cap;

void fle_imp_GetEnvironmentStrings(void)
{
    static const char api[] = "GetEnvironmentStrings";
    size_t i, j, total = 1, at = 0;
    char *blk;

    for (i = 0; i < env_n; i++) total += env_v[i].nlen + 1 + fle_w_len(env_v[i].value) + 1;
    blk = malloc(total);
    if (!blk) {
        R[0] = 0;
        return;
    }
    for (i = 0; i < env_n; i++) {
        size_t vl = fle_w_len(env_v[i].value);
        for (j = 0; j < env_v[i].nlen; j++) {
            if (env_v[i].name[j] >= 0x80) fle_stop(api, "an environment variable name has a character at or above 0x80 (the ANSI conversion is not modelled)");
            blk[at++] = (char)env_v[i].name[j];
        }
        blk[at++] = '=';
        for (j = 0; j < vl; j++) {
            if (env_v[i].value[j] >= 0x80) fle_stop(api, "an environment variable value has a character at or above 0x80 (the ANSI conversion is not modelled)");
            blk[at++] = (char)env_v[i].value[j];
        }
        blk[at++] = 0;
    }
    blk[at++] = 0;
    if (envA_blocks_n == envA_blocks_cap) {
        size_t nc = envA_blocks_cap ? envA_blocks_cap * 2 : 8;
        char **nb = realloc(envA_blocks, nc * sizeof *nb);
        if (!nb) fle_stop(api, "out of memory");
        envA_blocks = nb;
        envA_blocks_cap = nc;
    }
    envA_blocks[envA_blocks_n++] = blk;
    R[0] = (uint64_t)(uintptr_t)blk;
}

void fle_imp_FreeEnvironmentStringsA(void)
{
    static const char api[] = "FreeEnvironmentStringsA";
    size_t i;
    uint64_t p = R[1];

    for (i = 0; i < envA_blocks_n; i++) {
        if ((uint64_t)(uintptr_t)envA_blocks[i] == p) {
            free(envA_blocks[i]);
            envA_blocks[i] = envA_blocks[--envA_blocks_n];
            R[0] = 1;
            return;
        }
    }
    fle_stop(api, "a pointer that GetEnvironmentStrings did not return (or that was already freed); the page names no error code");
}

/* shell32!SHGetFolderPathW and shfolder!SHGetFolderPathW (MSDN: SHGetFolderPathW, CSIDL; decisions 0103 and 0107, PROVISIONAL). hwnd rcx
 * ("Reserved"), csidl edx, hToken r8, dwFlags r9d, pszPath the fifth argument at [rsp+0x28] on entry (the first stack slot after the
 * 32-byte shadow space and the return address; R[4] points at the return address here). Two csidl values are answered, nothing else:
 *  - CSIDL_FONTS (0x0014, from shlobj.h: the Learn CSIDL page lists the constants without numbers; decision 0103): the page says
 *    "CSIDL_FONTS: A virtual folder that contains fonts. A typical path is C:\Windows\Fonts", and SHGetFolderPathW says "Only real folders
 *    are valid. If a virtual folder is specified, this function fails": the contradiction is settled by the PM's instruction for that wall
 *    (answer the font folder). Path FLE_VIRTUAL_WINDOWS_DIR + "\\Fonts" = C:\Windows\Fonts.
 *  - CSIDL_APPDATA (0x001a, shlobj.h; decision 0107): "the file system directory that serves as a common repository for application-specific
 *    data", a real folder (the page's list of supported values has CSIDL_APPDATA). Path FLE_VIRTUAL_USER_PROFILE_DIR + "\\AppData\\Roaming" =
 *    C:\Users\user\AppData\Roaming (the Windows Vista and later default of FOLDERID_RoamingAppData, which the CSIDL maps to; the user name is
 *    a choice of this environment, there is no other user name in it).
 * Both: S_OK (0), no trailing backslash (page: "The returned path does not include a trailing backslash"), never a host path. hToken must be
 * NULL (the "usually NULL" case; any token needs an access-token model that does not exist here) and dwFlags must be SHGFP_TYPE_CURRENT (0)
 * or SHGFP_TYPE_DEFAULT (1), which give the same path because nothing is redirected. A folder that does not exist yields S_FALSE on a real
 * system; the virtual volume holds no system or profile folder (as GetWindowsDirectoryW says), so existence is not tested. Every other csidl
 * (CSIDL_FLAG_CREATE and CSIDL_FLAG_DONT_VERIFY included), flag, token or a NULL pszPath stops the run by name. The buffer is "of length
 * MAX_PATH"; the path and the NUL are written. The last error is not touched (HRESULT function).
 * shfolder.dll: the response table has a second row, ("shfolder.dll", "SHGetFolderPathW"), for the same function. The Windows 2000
 * Application Specification says the redistributable SHFOLDER.DLL is a protected system file of Windows 2000 and later, and the old MSDN text
 * of SHGetFolderPath says that SHFolder.dll "always calls the current platform's version of this function" (see decision 0107 for what was and
 * was not read); FLEngine imports SHFolder.dll!SHGetFolderPathW itself (IAT 0x1971820; thunk 0x4262e0) and looks it up with LoadLibraryW +
 * GetProcAddress at 25 call sites (24 copies of the strings). */
#define FLE_CSIDL_FONTS 0x0014
#define FLE_CSIDL_APPDATA 0x001a
/* Decision 0126 (PROVISIONAL), wall 59: CSIDL_WINDOWS (0x0024, shlobj.h: UNOFFICIAL; the page learn/CSIDL.txt lists the name with FOLDERID_Windows and no number): "The Windows directory or SYSROOT. This corresponds to the %windir% or
 * %SYSTEMROOT% environment variables. A typical path is C:\Windows." A real folder (the page's list). Path FLE_VIRTUAL_WINDOWS_DIR, the directory GetWindowsDirectoryW answers (decision 0103), no trailing backslash. */
#define FLE_CSIDL_WINDOWS 0x0024
/* Decision 0126 (PROVISIONAL), wall 60: CSIDL_LOCAL_APPDATA (0x001c, shlobj.h: UNOFFICIAL; FOLDERID_LocalAppData on the page): "The file system directory that serves as a data repository for local (nonroaming) applications. A typical
 * path is C:\Documents and Settings\username\Local Settings\Application Data." That typical path is the one of Windows XP; the path of this environment is FLE_VIRTUAL_USER_PROFILE_DIR + "\AppData\Local" (the Windows Vista
 * and later default of FOLDERID_LocalAppData, the same reading as CSIDL_APPDATA above): S-localappdata-path, a choice without a source (the page gives the XP path only). A real folder (the page's list). */
#define FLE_CSIDL_LOCAL_APPDATA 0x001c
/* numbers from shlobj.h (UNOFFICIAL; the page lists names): */
#define FLE_CSIDL_PERSONAL 0x0005
#define FLE_CSIDL_COMMON_APPDATA 0x0023
#define FLE_CSIDL_PROGRAM_FILES 0x0026
#define FLE_CSIDL_PROFILE 0x0028
#define FLE_CSIDL_PROGRAM_FILESX86 0x002a
#define FLE_CSIDL_PROGRAM_FILES_COMMON 0x002b
#define FLE_CSIDL_PROGRAM_FILES_COMMONX86 0x002c
#define FLE_LOCALAPPDATA_SUBDIR "\\AppData\\Local"
#define FLE_FONTS_SUBDIR "\\Fonts"
#define FLE_APPDATA_SUBDIR "\\AppData\\Roaming"

void fle_imp_SHGetFolderPathW(void)
{
    static const char api[] = "SHGetFolderPathW";
    uint32_t csidl = (uint32_t)R[2], flags = (uint32_t)R[9];
    uint64_t token = R[8], path = fle_ld(R[4] + 0x28u, 8);
    const char *text;
    uint16_t *w;
    size_t len;
    char why[260];

    if (csidl == FLE_CSIDL_FONTS) {
        text = FLE_VIRTUAL_WINDOWS_DIR FLE_FONTS_SUBDIR;
    } else if (csidl == FLE_CSIDL_APPDATA) {
        text = FLE_VIRTUAL_USER_PROFILE_DIR FLE_APPDATA_SUBDIR;
    } else if (csidl == FLE_CSIDL_WINDOWS) {
        text = FLE_VIRTUAL_WINDOWS_DIR;
    } else if (csidl == FLE_CSIDL_LOCAL_APPDATA) {
        text = FLE_VIRTUAL_USER_PROFILE_DIR FLE_LOCALAPPDATA_SUBDIR;
    } else if (csidl == FLE_CSIDL_PROGRAM_FILES || csidl == FLE_CSIDL_PROGRAM_FILESX86) {
        /* Decision 0126 (PROVISIONAL), wall 80 (S-programfiles): CSIDL_PROGRAM_FILES: "The Program Files folder. A typical path is C:\Program Files" (learn/CSIDL.txt). CSIDL_PROGRAM_FILESX86 (0x2a) is on the page with FOLDERID_ProgramFilesX86
         * and NO typical path: this machine is a 64-bit environment whose product lives under "C:\Program Files" (FLE_VIRTUAL_DLL_PATH), and has no 32-bit Program Files folder; the guest (a 64-bit program asking for
         * the 32-bit folder, to look for 32-bit plugins) gets the same path as CSIDL_PROGRAM_FILES: a CHOICE WITHOUT A SOURCE (the real answer is "C:\Program Files (x86)", which the page does not give). */
        text = "C:\\Program Files";
    } else if (csidl == FLE_CSIDL_PROFILE || csidl == FLE_CSIDL_PERSONAL) {
        /* Decision 0126 (S-profile-personal): CSIDL_PROFILE (0x28): "The user's profile folder. A typical path is C:\Users\username" (the page): FLE_VIRTUAL_USER_PROFILE_DIR. CSIDL_PERSONAL (0x05): the page lists CSIDL_PERSONAL
         * with CSIDL_MYDOCUMENTS and no typical path of its own; the guest's own "Documents\Image-Line\" base (reg seed [Shared data] C:\Users\user\Documents\Image-Line\) shows that Documents is under the profile:
         * FLE_VIRTUAL_USER_PROFILE_DIR + "\\Documents" (a choice). */
        text = csidl == FLE_CSIDL_PROFILE ? FLE_VIRTUAL_USER_PROFILE_DIR : FLE_VIRTUAL_USER_PROFILE_DIR "\\Documents";
    } else if (csidl == FLE_CSIDL_PROGRAM_FILES_COMMON || csidl == FLE_CSIDL_PROGRAM_FILES_COMMONX86) {
        /* Decision 0126 (S-programfiles-common, wall 80b): CSIDL_PROGRAM_FILES_COMMON (0x2b): "A folder for components that are shared across applications. A typical path is C:\Program Files\Common" (the page; it adds "Valid
         * only for Windows XP"); CSIDL_PROGRAM_FILES_COMMONX86 (0x2c) has no typical path on the page. The real path is "C:\Program Files\Common Files": a choice without a source. Both answer it. */
        text = "C:\\Program Files\\Common Files";
    } else if (csidl == FLE_CSIDL_COMMON_APPDATA) {
        /* Decision 0126 (S-commonappdata): CSIDL_COMMON_APPDATA (0x23, FOLDERID_ProgramData): "application data for all users. A typical path is C:\Documents and Settings\All Users\Application Data" (Windows XP's); the Vista and
         * later path of FOLDERID_ProgramData is "C:\ProgramData": a choice without a source (the page gives the XP path only). */
        text = "C:\\ProgramData";
    } else {
        snprintf(why, sizeof why, "csidl %#x is not answered (answered: 0x05 PERSONAL, 0x14 FONTS, 0x1a APPDATA, 0x1c LOCAL_APPDATA, 0x23 COMMON_APPDATA, 0x24 WINDOWS, 0x26 PROGRAM_FILES, 0x28 PROFILE, 0x2a PROGRAM_FILESX86, 0x2b/0x2c PROGRAM_FILES_COMMON[X86])", csidl);
        fle_stop(api, why);
    }
    if (token != 0) fle_stop(api, "hToken is not NULL: access tokens are not modelled");
    if (flags > 1) fle_stop(api, "dwFlags is neither SHGFP_TYPE_CURRENT (0) nor SHGFP_TYPE_DEFAULT (1)");
    if (path == 0) fle_stop(api, "pszPath is NULL: the page does not define it");
    w = fle_w_from_ascii(text);
    len = fle_w_len(w);
    fle_w_write(path, w, len + 1);
    free(w);
    R[0] = 0;
}

/* ---- computer name and user name (decision 0112, PROVISIONAL) ---- */

_Static_assert(sizeof(FLE_VIRTUAL_COMPUTER_NAME) - 1 <= FLE_COMPUTERNAME_NETBIOS_MAX, "the NetBIOS name has at most MAX_COMPUTERNAME_LENGTH characters");

/* The text for a COMPUTER_NAME_FORMAT value (Learn, COMPUTER_NAME_FORMAT: the enumerators in the order ComputerNameNetBIOS, ComputerNameDnsHostname,
 * ComputerNameDnsDomain, ComputerNameDnsFullyQualified, ComputerNamePhysicalNetBIOS, ComputerNamePhysicalDnsHostname, ComputerNamePhysicalDnsDomain,
 * ComputerNamePhysicalDnsFullyQualified, ComputerNameMax, that is 0 to 8; the GetComputerNameExW example indexes its description table with the value).
 * Answered: the NetBIOS name (0) and its Physical form (4), and the DNS host name (1) and its Physical form (5). The Physical forms differ from the others only
 * "if the local computer is a node in a cluster" (GetComputerNameExW), and this one is not. The NetBIOS name is "the OEM representation of the DNS host name up to
 * MAX_COMPUTERNAME_LENGTH characters" (Learn, Computer Names) and Windows uses upper-case NetBIOS names (Learn, Naming conventions in Active Directory): the one
 * name is upper case and short enough, so all four values are the same string (which case a DNS host name has is the choice of this environment).
 * Stopped: the DNS domain (2, 6) and the fully qualified names (3, 7): the pages do not say what a computer that is not joined to a domain returns (an empty
 * string and success, or a failure), and "If the local computer is not configured to use DNS names, GetComputerNameEx will not return DNS information" does not
 * say which. ComputerNameMax (8) is "Not used" and no error is documented for it. Any other value is outside the enumeration. */
static const char *computer_name_for_format(const char *api, uint32_t fmt)
{
    char why[200];
    switch (fmt) {
    case 0:
    case 1:
    case 4:
    case 5:
        return FLE_VIRTUAL_COMPUTER_NAME;
    case 2:
    case 3:
    case 6:
    case 7:
        snprintf(why, sizeof why, "NameType %u (a DNS domain or a fully qualified name): the pages do not say what a computer that is not joined to a domain returns", fmt);
        fle_stop(api, why);
    case 8:
        fle_stop(api, "NameType 8 (ComputerNameMax) is \"Not used\" on the COMPUTER_NAME_FORMAT page and no error is documented for it");
    default:
        snprintf(why, sizeof why, "NameType %u is outside the COMPUTER_NAME_FORMAT enumeration (0 to 8)", fmt);
        fle_stop(api, why);
    }
}

static void put_name(uint64_t buf, const char *text, int wide)
{
    size_t i, n = strlen(text);
    for (i = 0; i <= n; i++) {
        if (wide) fle_st(buf + 2u * i, 2, (uint8_t)text[i]);
        else fle_st(buf + i, 1, (uint8_t)text[i]);
    }
}

/* The in/out size protocol of the four calls (Learn: GetComputerNameExW / GetComputerNameW / GetUserNameW, parameter nSize / lpnSize / pcbBuffer): "On input,
 * specifies the size of the buffer, in TCHARs. On output, receives the number of TCHARs copied to the destination buffer, not including the terminating null
 * character" (the two computer-name calls) or "including the terminating null character" (GetUserNameW, count_nul). Too small: the call fails with `err` and the
 * size receives "the size of the buffer required, including the terminating null character"; the buffer is not written (the pages do not say; inference from
 * "the function fails"). A size of 0 is always too small, so a NULL buffer with size 0 gets the required size (GetComputerNameExW: "set this parameter to NULL and
 * use the required buffer size returned in the lpnSize parameter"). Stops: a NULL size pointer; for GetComputerNameExW (is_ex) a NULL buffer with a non-zero size
 * ("If lpBuffer is NULL, this parameter must be zero"); a NULL buffer with a size that would hold the name (the pages define no such call). The last error is not
 * touched on success. TCHAR is the 16-bit character of the W forms and the byte of the A form. */
static void name_answer(const char *api, const char *text, int wide, uint64_t buf, uint64_t psize, uint32_t err, int is_ex, int count_nul)
{
    size_t need = strlen(text) + 1;
    uint32_t cap;

    if (psize == 0) fle_stop(api, "the size pointer is NULL: the page does not define it");
    cap = (uint32_t)fle_ld(psize, 4);
    if (is_ex && buf == 0 && cap != 0) fle_stop(api, "lpBuffer is NULL but *nSize is not zero: the page says it must be zero");
    if (cap < need) {
        fle_st(psize, 4, (uint32_t)need);
        fle_set_last_error(err);
        R[0] = 0;
        return;
    }
    if (buf == 0) fle_stop(api, "lpBuffer is NULL and the size would hold the name: the page defines no such call");
    put_name(buf, text, wide);
    fle_st(psize, 4, (uint32_t)(count_nul ? need : need - 1));
    R[0] = 1;
}

/* kernel32!GetComputerNameExW (Learn: GetComputerNameExW; decision 0112). NameType ecx, lpBuffer rdx, nSize r8. All 26 call sites of FLEngine pass NameType 0
 * (ComputerNameNetBIOS) and a buffer of 0x2000 TCHARs (25 sites) or 0x3ff (one site), and 25 of them ignore the return value and read the buffer up to its NUL
 * (so a failure that leaves the buffer alone would make them read what the buffer held before). Too small: ERROR_MORE_DATA (234). */
void fle_imp_GetComputerNameExW(void)
{
    static const char api[] = "GetComputerNameExW";
    name_answer(api, computer_name_for_format(api, (uint32_t)R[1]), 1, R[2], R[8], FLE_W32_ERR_MORE_DATA, 1, 0);
}

/* kernel32!GetComputerNameExA: the 8-bit form (Learn: the page is that of GetComputerNameExW with "(ANSI)"). The name is ASCII, so the bytes are the characters
 * (the process code page 1252 keeps ASCII, as fle_locale.c assumes). FLEngine and the bundled DLLs do not import it; the row is here because decision 0112
 * asks for the pair. */
void fle_imp_GetComputerNameExA(void)
{
    static const char api[] = "GetComputerNameExA";
    name_answer(api, computer_name_for_format(api, (uint32_t)R[1]), 0, R[2], R[8], FLE_W32_ERR_MORE_DATA, 1, 0);
}

/* kernel32!GetComputerNameW (Learn: GetComputerNameW): "retrieves only the NetBIOS name"; lpBuffer rcx, nSize rdx. "The buffer size should be large enough to
 * contain MAX_COMPUTERNAME_LENGTH + 1 characters" (the single FLEngine site, 0x5df888, passes the size 32 while its buffer, at rbp+0x64, lies 32 bytes before the
 * size variable at rbp+0x84: 16 TCHARs, reading of the frame; the 15-character name and its NUL fit).
 * Too small: ERROR_BUFFER_OVERFLOW (111; this page names it, GetComputerNameExW names ERROR_MORE_DATA). */
void fle_imp_GetComputerNameW(void)
{
    name_answer("GetComputerNameW", FLE_VIRTUAL_COMPUTER_NAME, 1, R[1], R[2], FLE_W32_ERR_BUFFER_OVERFLOW, 0, 0);
}

/* advapi32!GetUserNameW (Learn: GetUserNameW): lpBuffer rcx, pcbBuffer rdx. The user name is FLE_VIRTUAL_USER_NAME (decision 0107 chose "user" for the profile
 * directory). "A buffer size of (UNLEN + 1) characters will hold the maximum length user name"; on success the out size counts the terminating NUL; too small:
 * ERROR_INSUFFICIENT_BUFFER (122). Neither FLEngine nor the bundled DLLs import it; the row is here because decision 0112 asks for it. */
void fle_imp_GetUserNameW(void)
{
    name_answer("GetUserNameW", FLE_VIRTUAL_USER_NAME, 1, R[1], R[2], FLE_W32_ERR_INSUFFICIENT_BUFFER, 0, 1);
}
