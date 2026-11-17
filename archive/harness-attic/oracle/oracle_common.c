/* A5 oracle: case-file driver shared by both sides. Usage: <driver> CASES OUT PE
 * CASES: one case per line,
 *   CASE id fn_hex fill seed_hex nstk [stk_hex...] gpr0..gpr15 (hex) xmm0lo xmm0hi .. xmm15hi (hex)
 * OUT: one JSON object per case. */
#define _GNU_SOURCE
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <ucontext.h>
#include <unistd.h>

#include "../boot.h"
#include "oracle_common.h"

char oracle_last_msg[256];
oracle_cpu oracle_ret_cpu;

typedef struct {
    uint64_t n, hash;
    uint32_t nlist;
    uint64_t list[OR_LIST][3]; /* addr, before, after */
} region_diff;

typedef struct {
    volatile int32_t status;
    int32_t sig;
    uint64_t fault_addr, fault_rip;
    char msg[256];
    oracle_cpu out;
    region_diff reg[3]; /* image (writable hull), scratch, stack */
} oracle_res;

static oracle_res *res;
static uint8_t *snap[3];
static uint64_t reg_lo[3], reg_sz[3];

static uint16_t rd16(const uint8_t *p) { uint16_t v; memcpy(&v, p, sizeof v); return v; }
static uint32_t rd32(const uint8_t *p) { uint32_t v; memcpy(&v, p, sizeof v); return v; }

static int pe_parse(const char *path, oracle_pe *pe, char *err, size_t errn) {
    uint8_t hdr[0x1000];
    int fd = open(path, O_RDONLY);
    ssize_t n;
    uint32_t lfanew;
    const uint8_t *h, *opt, *sh;
    uint16_t nsec, optsz;
    if (fd < 0) { snprintf(err, errn, "open %s failed", path); return -1; }
    n = pread(fd, hdr, sizeof hdr, 0);
    close(fd);
    if (n < 0x400 || memcmp(hdr, "MZ", 2) != 0) { snprintf(err, errn, "not a PE"); return -1; }
    lfanew = rd32(hdr + 0x3c);
    if (lfanew + 24 + 0xf0 > (uint64_t)n) { snprintf(err, errn, "header beyond 4 KiB"); return -1; }
    h = hdr + lfanew;
    nsec = rd16(h + 6);
    optsz = rd16(h + 20);
    opt = h + 24;
    if (nsec > OR_MAX_SEC) { snprintf(err, errn, "too many sections"); return -1; }
    memset(pe, 0, sizeof *pe);
    memcpy(&pe->base, opt + 24, 8);
    pe->size = rd32(opt + 56);
    pe->wr_lo = UINT64_MAX;
    sh = h + 24 + optsz;
    for (int i = 0; i < nsec; i++) {
        const uint8_t *s = sh + 40 * i;
        uint32_t vs = rd32(s + 8), va = rd32(s + 12), rs = rd32(s + 16), ch = rd32(s + 36);
        uint64_t span = vs > rs ? vs : rs;
        uint64_t lo = pe->base + va, hi = (lo + span + 0xfff) & ~UINT64_C(0xfff);
        pe->sec[i].va = lo;
        pe->sec[i].size = hi - lo;
        pe->sec[i].w = (ch & 0x80000000u) != 0;
        pe->sec[i].x = (ch & 0x20000000u) != 0;
        if (pe->sec[i].x && !pe->text_hi) { pe->text_lo = lo; pe->text_hi = hi; }
        if (pe->sec[i].w) {
            if (lo < pe->wr_lo) pe->wr_lo = lo;
            if (hi > pe->wr_hi) pe->wr_hi = hi;
        }
    }
    pe->nsec = nsec;
    if (pe->wr_lo == UINT64_MAX) pe->wr_lo = pe->wr_hi = 0;
    return 0;
}

static void protect_sections(const oracle_pe *pe) {
    mprotect((void *)(uintptr_t)pe->base, 0x1000, PROT_READ);
    for (int i = 0; i < pe->nsec; i++) {
        int prot = pe->sec[i].w ? PROT_READ | PROT_WRITE : pe->sec[i].x ? PROT_READ | PROT_EXEC : PROT_READ;
        mprotect((void *)(uintptr_t)pe->sec[i].va, pe->sec[i].size, prot);
    }
}

static uint64_t rng_next(uint64_t *s) {
    uint64_t x = *s;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *s = x;
    return x * UINT64_C(0x2545F4914F6CDD1D);
}

static void fill_scratch(uint32_t mode, uint64_t seed) {
    uint64_t s = seed * 2 + 1;
    uint64_t *p = (uint64_t *)(uintptr_t)OR_SCRATCH;
    for (uint64_t i = 0; i < OR_SCRATCH_SZ / 8; i++) {
        uint64_t r = rng_next(&s);
        switch (mode) {
        case 0: p[i] = r; break;
        case 1: {
            uint64_t k = r & 3;
            uint64_t v = rng_next(&s);
            p[i] = k < 2 ? OR_SCRATCH + ((v % (OR_SCRATCH_SZ - 0x100)) & ~UINT64_C(7)) : k == 2 ? (v & 0xff) : v;
            break;
        }
        case 2: p[i] = 0; break;
        case 4: {
            static const uint64_t b[] = {0, 1, 0x7f, 0x80, 0xff, 0x7fff, 0x8000, 0xffff, 0x7fffffff, 0x80000000,
                                         0xffffffff, UINT64_C(0x7fffffffffffffff), UINT64_C(0x8000000000000000),
                                         UINT64_MAX, UINT64_C(0x3ff0000000000000), UINT64_C(0x7ff8000000000000)};
            p[i] = b[r & 15];
            break;
        }
        default: p[i] = UINT64_MAX; break;
        }
    }
}

static void fill_stack(const oracle_case *c) {
    uint64_t s = (c->seed ^ UINT64_C(0x9e3779b97f4a7c15)) * 2 + 1;
    uint64_t *p = (uint64_t *)(uintptr_t)OR_STACK;
    for (uint64_t i = 0; i < OR_STACK_SZ / 8; i++) p[i] = rng_next(&s);
    for (uint64_t i = 0; i < c->nstk && i < OR_MAX_STK; i++) {
        uint64_t a = OR_STACK_TOP + 32 + 8 * i;
        memcpy((void *)(uintptr_t)a, &c->stk[i], 8);
    }
}

static void diff_region(int r) {
    const uint64_t *now = (const uint64_t *)(uintptr_t)reg_lo[r];
    const uint64_t *old = (const uint64_t *)snap[r];
    region_diff *d = &res->reg[r];
    uint64_t h = UINT64_C(0xcbf29ce484222325);
    d->n = 0;
    d->nlist = 0;
    for (uint64_t i = 0; i < reg_sz[r] / 8; i++) {
        if (now[i] == old[i]) continue;
        uint64_t a = reg_lo[r] + 8 * i;
        h = (h ^ a) * UINT64_C(0x100000001b3);
        h = (h ^ now[i]) * UINT64_C(0x100000001b3);
        h ^= h >> 29;
        if (d->nlist < OR_LIST) {
            d->list[d->nlist][0] = a;
            d->list[d->nlist][1] = old[i];
            d->list[d->nlist][2] = now[i];
            d->nlist++;
        }
        d->n++;
    }
    d->hash = h;
}

static void on_signal(int sig, siginfo_t *si, void *uc) {
    ucontext_t *c = uc;
    res->sig = sig;
    res->fault_addr = (uint64_t)(uintptr_t)si->si_addr;
    res->fault_rip = (uint64_t)c->uc_mcontext.gregs[REG_RIP];
    snprintf((char *)res->msg, sizeof res->msg, "%s", oracle_last_msg);
    res->status = sig == SIGALRM ? OR_TIMEOUT : sig == SIGABRT ? OR_ABORT : OR_FAULT;
    _exit(0);
}

static void install_handlers(void) {
    static uint8_t alt[1 << 16];
    stack_t ss = { .ss_sp = alt, .ss_size = sizeof alt, .ss_flags = 0 };
    struct sigaction sa;
    sigaltstack(&ss, NULL);
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = on_signal;
    sa.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGSEGV, &sa, NULL);
    sigaction(SIGBUS, &sa, NULL);
    sigaction(SIGILL, &sa, NULL);
    sigaction(SIGFPE, &sa, NULL);
    sigaction(SIGABRT, &sa, NULL);
    sigaction(SIGALRM, &sa, NULL);
}

static void child(const oracle_case *c, const oracle_pe *pe, oracle_exec_fn exec) {
    oracle_cpu out;
    install_handlers();
    alarm(OR_TIMEOUT_S);
    fill_scratch(c->fill, c->seed);
    fill_stack(c);
    memcpy(snap[0], (void *)(uintptr_t)reg_lo[0], reg_sz[0]);
    memcpy(snap[1], (void *)(uintptr_t)reg_lo[1], reg_sz[1]);
    memcpy(snap[2], (void *)(uintptr_t)reg_lo[2], reg_sz[2]);
    if (exec(c, pe, &out) != 0) {
        res->status = OR_NOENTRY;
        _exit(0);
    }
    alarm(0);
    memcpy((void *)(uintptr_t)(OR_STACK_TOP - 8), &(uint64_t){OR_SENTINEL}, 8);
    res->out = out;
    for (int r = 0; r < 3; r++) diff_region(r);
    res->status = OR_OK;
    _exit(0);
}

static int parse_case(char *line, oracle_case *c) {
    char *tok[128];
    int nt = 0;
    for (char *p = strtok(line, " \t\r\n"); p && nt < 128; p = strtok(NULL, " \t\r\n")) tok[nt++] = p;
    if (nt < 6 || strcmp(tok[0], "CASE") != 0) return -1;
    memset(c, 0, sizeof *c);
    snprintf(c->id, sizeof c->id, "%s", tok[1]);
    c->fn = strtoull(tok[2], NULL, 16);
    c->fill = (uint32_t)strtoul(tok[3], NULL, 10);
    c->seed = strtoull(tok[4], NULL, 16);
    c->nstk = strtoull(tok[5], NULL, 10);
    if (c->nstk > OR_MAX_STK || nt < (int)(6 + c->nstk + 16 + 32)) return -1;
    int k = 6;
    for (uint64_t i = 0; i < c->nstk; i++) c->stk[i] = strtoull(tok[k++], NULL, 16);
    for (int i = 0; i < 16; i++) c->in.gpr[i] = strtoull(tok[k++], NULL, 16);
    for (int i = 0; i < 16; i++) {
        c->in.xmm[i][0] = strtoull(tok[k++], NULL, 16);
        c->in.xmm[i][1] = strtoull(tok[k++], NULL, 16);
    }
    return 0;
}

static void emit(FILE *out, const oracle_case *c, const char *side) {
    static const char *names[] = {"ok", "fault", "abort", "timeout", "noentry", "died"};
    int st = res->status;
    fprintf(out, "{\"side\":\"%s\",\"id\":\"%s\",\"fn\":\"0x%llx\",\"status\":\"%s\"", side, c->id,
            (unsigned long long)c->fn, st >= 0 && st <= 5 ? names[st] : "died");
    if (st == OR_FAULT || st == OR_ABORT || st == OR_TIMEOUT) {
        char msg[300];
        size_t j = 0;
        for (size_t i = 0; res->msg[i] && j + 2 < sizeof msg; i++) {
            unsigned char ch = (unsigned char)res->msg[i];
            msg[j++] = ch < 0x20 || ch == '"' || ch == '\\' || ch > 0x7e ? '_' : (char)ch;
        }
        msg[j] = 0;
        fprintf(out, ",\"sig\":%d,\"fault_addr\":\"0x%llx\",\"fault_rip\":\"0x%llx\",\"msg\":\"%s\"", res->sig,
                (unsigned long long)res->fault_addr, (unsigned long long)res->fault_rip, msg);
    }
    if (st == OR_OK) {
        fprintf(out, ",\"gpr\":[");
        for (int i = 0; i < 16; i++) fprintf(out, "%s\"%llx\"", i ? "," : "", (unsigned long long)res->out.gpr[i]);
        fprintf(out, "],\"xmm\":[");
        for (int i = 0; i < 16; i++)
            fprintf(out, "%s\"%llx\",\"%llx\"", i ? "," : "", (unsigned long long)res->out.xmm[i][0],
                    (unsigned long long)res->out.xmm[i][1]);
        fprintf(out, "],\"mem\":{");
        static const char *rn[] = {"image", "scratch", "stack"};
        for (int r = 0; r < 3; r++) {
            const region_diff *d = &res->reg[r];
            fprintf(out, "%s\"%s\":{\"n\":%llu,\"hash\":\"%llx\",\"blocks\":[", r ? "," : "", rn[r],
                    (unsigned long long)d->n, (unsigned long long)d->hash);
            for (uint32_t i = 0; i < d->nlist; i++)
                fprintf(out, "%s[\"%llx\",\"%llx\",\"%llx\"]", i ? "," : "", (unsigned long long)d->list[i][0],
                        (unsigned long long)d->list[i][1], (unsigned long long)d->list[i][2]);
            fprintf(out, "]}");
        }
        fprintf(out, "}");
    }
    fprintf(out, "}\n");
}

int oracle_main(int argc, char **argv, const char *side, oracle_exec_fn exec) {
    harness_image img;
    oracle_pe pe;
    char err[256], line[8192];
    FILE *cf, *out;
    if (argc < 4) {
        fprintf(stderr, "usage: %s CASES OUT PE\n", argv[0]);
        return 2;
    }
    if (!getenv("ORACLE_NOASLR")) {
        setenv("ORACLE_NOASLR", "1", 1);
        execvp("setarch", (char *[]){"setarch", "x86_64", "-R", argv[0], argv[1], argv[2], argv[3], NULL});
        perror("setarch");
        return 3;
    }
    if (pe_parse(argv[3], &pe, err, sizeof err) != 0) {
        fprintf(stderr, "pe: %s\n", err);
        return 2;
    }
    if (image_load(argv[3], &img, err, sizeof err) != 0) {
        fprintf(stderr, "image: %s\n", err);
        return 2;
    }
    protect_sections(&pe);
    if (mmap((void *)(uintptr_t)OR_SCRATCH, OR_SCRATCH_SZ, PROT_READ | PROT_WRITE,
             MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0) == MAP_FAILED ||
        mmap((void *)(uintptr_t)OR_STACK, OR_STACK_SZ, PROT_READ | PROT_WRITE,
             MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0) == MAP_FAILED) {
        perror("mmap scratch/stack");
        return 2;
    }
    reg_lo[0] = pe.wr_lo;
    reg_sz[0] = pe.wr_hi - pe.wr_lo;
    reg_lo[1] = OR_SCRATCH;
    reg_sz[1] = OR_SCRATCH_SZ;
    reg_lo[2] = OR_STACK;
    reg_sz[2] = OR_STACK_SZ;
    for (int r = 0; r < 3; r++) {
        snap[r] = mmap(NULL, reg_sz[r], PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANONYMOUS, -1, 0);
        if (snap[r] == MAP_FAILED) { perror("mmap snap"); return 2; }
    }
    res = mmap(NULL, sizeof *res, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (res == MAP_FAILED) { perror("mmap res"); return 2; }
    cf = fopen(argv[1], "r");
    out = fopen(argv[2], "w");
    if (!cf || !out) { perror("open case/out"); return 2; }
    while (fgets(line, sizeof line, cf)) {
        oracle_case c;
        pid_t pid;
        int wst;
        if (line[0] == '#' || line[0] == '\n') continue;
        if (parse_case(line, &c) != 0) { fprintf(stderr, "bad case line\n"); return 2; }
        memset(res, 0, sizeof *res);
        res->status = OR_DIED;
        oracle_last_msg[0] = 0;
        fflush(NULL);
        pid = fork();
        if (pid < 0) { perror("fork"); return 2; }
        if (pid == 0) child(&c, &pe, exec);
        waitpid(pid, &wst, 0);
        if (res->status == OR_DIED && WIFSIGNALED(wst)) {
            res->sig = WTERMSIG(wst);
            snprintf((char *)res->msg, sizeof res->msg, "killed by signal %d", res->sig);
        }
        emit(out, &c, side);
    }
    fclose(cf);
    fclose(out);
    return 0;
}
