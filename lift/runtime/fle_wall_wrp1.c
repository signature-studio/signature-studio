/* Walls that the Fruity Wrapper reached inside the engine (decision 0183, PROVISIONAL). See fle_wall_wrp1.h. */
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_vfs.h"
#include "fle_wall_wrp1.h"

#define ERROR_INVALID_HANDLE 6u
#define ERROR_ALREADY_EXISTS 183u   /* winerror.h; the CreateJobObjectW page names the symbol, not the number */
#define MAX_PATH_UNITS 260u         /* the CreateJobObjectW page: "The name is limited to MAX_PATH characters" */

typedef struct {
    int live;           /* the job object exists */
    uint32_t handles;   /* open handles */
    uint16_t *name;     /* NULL for an unnamed job */
    size_t nlen;
    uint32_t limit_flags;
    int have_limits;
} job_ent;

static job_ent *jobs;
static uint64_t jobs_n, jobs_cap, jobs_live;
/* one entry per ISSUED HANDLE: handle i (0-based) refers to job obj[i] */
static uint64_t *hobj;
static uint8_t *hlive;
static uint64_t h_n, h_cap;
static int hook_registered;

unsigned fle_job_live(void) { return (unsigned)jobs_live; }

static int in_region(uint64_t h) { return h > FLE_JOB_HANDLE_BASE && h < FLE_JOB_HANDLE_BASE + FLE_JOB_REGION_SIZE; }

static int64_t h_index(uint64_t h)
{
    uint64_t off, i;
    if (!in_region(h)) return -1;
    off = h - FLE_JOB_HANDLE_BASE;
    if (off % 4u) return -1;
    i = off / 4u - 1u;
    return i < h_n ? (int64_t)i : -1;
}

/* CloseHandle of a job handle (through fle_vfs_add_close_hook). Learn (CreateJobObjectW): "The job is destroyed when its last handle has been closed and all associated processes have exited";
 * no process can be assigned here, so the job is destroyed with its last handle. A closed handle fails with 0 and ERROR_INVALID_HANDLE (the page names no code: inference, as fle_event.c). */
static int job_close_hook(uint64_t h)
{
    int64_t i;
    if (!in_region(h)) return 0;
    i = h_index(h);
    if (i < 0) fle_stop("CloseHandle", "job handle value was not issued by this runtime");
    if (!hlive[i]) {
        fle_set_last_error(ERROR_INVALID_HANDLE);
        R[0] = 0;
        return 1;
    }
    hlive[i] = 0;
    if (--jobs[hobj[i]].handles == 0) {
        job_ent *j = &jobs[hobj[i]];
        j->live = 0;
        free(j->name);
        j->name = NULL;
        jobs_live--;
    }
    R[0] = 1;
    return 1;
}

static uint64_t new_handle(const char *api, uint64_t obj)
{
    if (h_n == h_cap) {
        uint64_t nc = h_cap ? h_cap * 2 : 16;
        uint64_t *no = realloc(hobj, nc * sizeof *no);
        uint8_t *nl;
        if (!no) fle_stop(api, "out of memory");
        hobj = no;
        nl = realloc(hlive, nc);
        if (!nl) fle_stop(api, "out of memory");
        hlive = nl;
        h_cap = nc;
    }
    if (h_n + 2 >= FLE_JOB_REGION_SIZE / 4u) fle_stop(api, "job handle region exhausted");
    hobj[h_n] = obj;
    hlive[h_n] = 1;
    jobs[obj].handles++;
    h_n++;
    return FLE_JOB_HANDLE_BASE + 4u * h_n;
}

/* kernel32!CreateJobObjectW. Learn: "Creates or opens a job object"; lpJobAttributes NULL = default security descriptor and a non-inheritable handle; lpName: limited to MAX_PATH characters, NULL = no name,
 * "Name comparison is case-sensitive"; success returns a handle with JOB_OBJECT_ALL_ACCESS; "If the object existed before the function call, the function returns a handle to the existing job object and
 * GetLastError returns ERROR_ALREADY_EXISTS"; a new job has zero accounting, no active limits and no processes. Implemented: NULL security attributes (a structure stops by name: security descriptors and
 * inheritance are not modelled), unnamed and named jobs among the jobs of this runtime. NOT modelled (the page says it, the runtime has no such objects): a name equal to an event / semaphore / mutex /
 * waitable timer / file mapping gives ERROR_INVALID_HANDLE on Windows (the page); this runtime has no cross-kind namespace (named events are not implemented, fle_event.c), so such a clash is not detected
 * (S-wrp1-job-namespace). The last error is left alone on a plain success (the page does not say it is cleared). */
void fle_imp_CreateJobObjectW(void)
{
    static const char api[] = "CreateJobObjectW";
    uint64_t attrs = R[1], name = R[2], obj = 0;
    uint16_t *nm = NULL;
    size_t nlen = 0;
    int existing = 0;

    if (attrs != 0) fle_stop(api, "lpJobAttributes is not NULL (security descriptors and inheritance are not modelled)");
    if (name != 0) {
        nm = fle_w_read_alloc(api, name, MAX_PATH_UNITS + 1u, &nlen);
        if (nlen > MAX_PATH_UNITS) fle_stop(api, "a name longer than MAX_PATH characters: the page names no result");
        for (obj = 0; obj < jobs_n; obj++)
            if (jobs[obj].live && jobs[obj].name && jobs[obj].nlen == nlen && memcmp(jobs[obj].name, nm, nlen * 2u) == 0) { existing = 1; break; }
    }
    if (!hook_registered) {
        fle_vfs_add_close_hook(job_close_hook);
        hook_registered = 1;
    }
    if (!existing) {
        if (jobs_n == jobs_cap) {
            uint64_t nc = jobs_cap ? jobs_cap * 2 : 8;
            job_ent *n = realloc(jobs, nc * sizeof *n);
            if (!n) fle_stop(api, "out of memory");
            jobs = n;
            jobs_cap = nc;
        }
        obj = jobs_n++;
        memset(&jobs[obj], 0, sizeof jobs[obj]);
        jobs[obj].live = 1;
        jobs[obj].name = nm;
        jobs[obj].nlen = nlen;
        jobs_live++;
        nm = NULL;
    }
    free(nm);
    R[0] = new_handle(api, obj);
    if (existing) fle_set_last_error(ERROR_ALREADY_EXISTS);
}

/* kernel32!SetInformationJobObject. Learn: "Sets limits for a job object"; hJob must have JOB_OBJECT_SET_ATTRIBUTES (handles from CreateJobObjectW have JOB_OBJECT_ALL_ACCESS); JobObjectExtendedLimitInformation
 * = 9 takes a JOBOBJECT_EXTENDED_LIMIT_INFORMATION whose first member is JOBOBJECT_BASIC_LIMIT_INFORMATION {LARGE_INTEGER PerProcessUserTimeLimit, LARGE_INTEGER PerJobUserTimeLimit, DWORD LimitFlags, ...}
 * (member order of the structure pages; LimitFlags is at offset 16 by natural alignment); success is nonzero; failure is 0 with the last error set. The page names no error for a wrong
 * cbJobObjectInformationLength: the wrapper passes 0x6c (108) for class 9 while sizeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION) on x64 is larger (the SDK header, not the page) and does not look at the result.
 * S-wrp1-job-length: the length is NOT validated (only that the 20 bytes up to and including LimitFlags are inside it); the limits are recorded and nothing is enforced (no process can be assigned). An invalid or
 * closed handle: 0 and ERROR_INVALID_HANDLE (inference). Other information classes stop by name. */
void fle_imp_SetInformationJobObject(void)
{
    static const char api[] = "SetInformationJobObject";
    uint64_t h = R[1], info = R[8];
    uint32_t cls = (uint32_t)R[2], cb = (uint32_t)R[9];
    int64_t i;

    if (h == 0) {
        fle_set_last_error(ERROR_INVALID_HANDLE);
        R[0] = 0;
        return;
    }
    i = h_index(h);
    if (i < 0) fle_stop(api, "handle was not issued as a job object by this runtime");
    if (!hlive[i]) {
        fle_set_last_error(ERROR_INVALID_HANDLE);
        R[0] = 0;
        return;
    }
    if (cls != 9u) fle_stop(api, "only JobObjectExtendedLimitInformation (9) is modelled");
    if (info == 0 || cb < 20u) fle_stop(api, "the information buffer does not reach LimitFlags: the page names no result");
    jobs[hobj[i]].limit_flags = (uint32_t)fle_ld(info + 16u, 4);
    jobs[hobj[i]].have_limits = 1;
    R[0] = 1;
}
