/* Maps the PE image at its ImageBase from the file's section table. Each IAT
 * slot is filled with its own slot VA (the pseudo-VA of fe-pe's iat_va), so a
 * call through an import reaches fle_dispatch_miss with that VA. */
#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

#include "boot.h"

#define SYM_MAX 8192

static harness_sym imports[SYM_MAX];
static harness_sym exports[SYM_MAX];
static size_t nimports;
static size_t nexports;

static uint16_t rd16(const uint8_t *p) { uint16_t v; memcpy(&v, p, sizeof v); return v; }
static uint32_t rd32(const uint8_t *p) { uint32_t v; memcpy(&v, p, sizeof v); return v; }
static uint64_t rd64(const uint8_t *p) { uint64_t v; memcpy(&v, p, sizeof v); return v; }

static int fail(char *err, size_t errlen, const char *msg) {
    snprintf(err, errlen, "%s", msg);
    return -1;
}

static const uint8_t *at(const uint8_t *mem, uint64_t size, uint64_t rva, uint64_t len) {
    if (rva > size || len > size - rva) return NULL;
    return mem + rva;
}

static const char *lookup(const harness_sym *t, size_t n, uint64_t va) {
    for (size_t i = 0; i < n; i++) {
        if (t[i].va == va) return t[i].name;
    }
    return NULL;
}

const char *image_import_name(uint64_t va) { return lookup(imports, nimports, va); }
const char *image_export_name(uint64_t va) { return lookup(exports, nexports, va); }

static int add_sym(harness_sym *t, size_t *n, uint64_t va, const char *name) {
    char *copy;
    if (*n >= SYM_MAX) return -1;
    copy = strdup(name);
    if (!copy) return -1;
    t[*n].va = va;
    t[*n].name = copy;
    (*n)++;
    return 0;
}

int image_load(const char *path, harness_image *img, char *err, size_t errlen) {
    static uint8_t hdr[0x1000];
    const uint8_t *pe, *opt, *dd;
    uint8_t *mem;
    uint64_t ibase, size;
    uint32_t lfanew, entry_rva, size_of_image, size_of_headers, imp_rva, exp_rva;
    uint16_t nsec, optsz;
    ssize_t n;
    void *m;
    int fd;

    fd = open(path, O_RDONLY);
    if (fd < 0) return fail(err, errlen, "open failed");
    n = pread(fd, hdr, sizeof hdr, 0);
    if (n < 0x400) return fail(err, errlen, "file too small");
    if (memcmp(hdr, "MZ", 2) != 0) return fail(err, errlen, "no MZ signature");
    lfanew = rd32(hdr + 0x3c);
    if ((uint64_t)lfanew + 24 + 0xf0 > (uint64_t)n) return fail(err, errlen, "optional header beyond first 4 KiB");
    pe = hdr + lfanew;
    if (memcmp(pe, "PE\0\0", 4) != 0) return fail(err, errlen, "no PE signature");
    if (rd16(pe + 4) != 0x8664) return fail(err, errlen, "not AMD64");
    nsec = rd16(pe + 6);
    optsz = rd16(pe + 20);
    opt = pe + 24;
    if (rd16(opt) != 0x20b) return fail(err, errlen, "not PE32+");
    entry_rva = rd32(opt + 16);
    ibase = rd64(opt + 24);
    size_of_image = rd32(opt + 56);
    size_of_headers = rd32(opt + 60);
    dd = opt + 112;
    if (size_of_headers > sizeof hdr) return fail(err, errlen, "headers larger than 4 KiB");
    if ((uint64_t)lfanew + 24 + optsz + 40ull * nsec > (uint64_t)n) return fail(err, errlen, "section table beyond first 4 KiB");
    if (ibase % 0x1000 || size_of_image == 0) return fail(err, errlen, "bad ImageBase or SizeOfImage");
    size = size_of_image;

#ifdef __EMSCRIPTEN__
    /* wasm (lift/harness/wasm, decision 0045): no mmap. The image lies at its ImageBase inside the linear memory,
     * below the module's data (the link sets -sGLOBAL_BASE above ImageBase + SizeOfImage). Memory is zero at start. */
    {
        extern char __global_base;
        if (ibase + size > (uint64_t)(uintptr_t)&__global_base) return fail(err, errlen, "image does not fit below GLOBAL_BASE");
    }
    m = (void *)(uintptr_t)ibase;
    memset(m, 0, size);
#else
    m = mmap((void *)(uintptr_t)ibase, size, PROT_READ | PROT_WRITE | PROT_EXEC,
             MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    if (m == MAP_FAILED || (uint64_t)(uintptr_t)m != ibase) return fail(err, errlen, "mmap at ImageBase failed");
#endif
    mem = m;
    memcpy(mem, hdr, size_of_headers);

    for (uint16_t i = 0; i < nsec; i++) {
        const uint8_t *s = pe + 24 + optsz + 40 * (size_t)i;
        uint32_t vsize = rd32(s + 8), va = rd32(s + 12), rawsz = rd32(s + 16), rawptr = rd32(s + 20);
        uint32_t span = vsize ? vsize : rawsz;
        uint32_t copy = span < rawsz ? span : rawsz;
        if ((uint64_t)va + span > size) return fail(err, errlen, "section beyond SizeOfImage");
        if (copy && pread(fd, mem + va, copy, rawptr) != (ssize_t)copy) return fail(err, errlen, "section read failed");
    }
    close(fd);

    imp_rva = rd32(dd + 8);
    if (imp_rva) {
        for (uint64_t off = 0;; off += 20) {
            const uint8_t *d = at(mem, size, imp_rva + off, 20);
            uint32_t oft, namerva, ft, ilt;
            char dll[64], sym[224], full[300];
            if (!d) return fail(err, errlen, "import descriptor beyond image");
            oft = rd32(d);
            namerva = rd32(d + 12);
            ft = rd32(d + 16);
            if (!oft && !namerva && !ft) break;
            ilt = oft ? oft : ft;
            snprintf(dll, sizeof dll, "%.60s", (const char *)at(mem, size, namerva, 1));
            for (uint64_t k = 0;; k++) {
                const uint8_t *tp = at(mem, size, ilt + 8 * k, 8);
                uint64_t th, slot;
                if (!tp) return fail(err, errlen, "import thunk beyond image");
                th = rd64(tp);
                if (!th) break;
                if (th >> 63) {
                    snprintf(sym, sizeof sym, "ord%llu", (unsigned long long)(th & 0xffff));
                } else {
                    const uint8_t *np = at(mem, size, (th & 0x7fffffffu) + 2, 1);
                    if (!np) return fail(err, errlen, "import name beyond image");
                    snprintf(sym, sizeof sym, "%.200s", (const char *)np);
                }
                snprintf(full, sizeof full, "%s!%s", dll, sym);
                slot = ibase + ft + 8 * k;
                if (!at(mem, size, ft + 8 * k, 8)) return fail(err, errlen, "IAT slot beyond image");
                if (add_sym(imports, &nimports, slot, full) != 0) return fail(err, errlen, "too many imports");
                memcpy(mem + ft + 8 * k, &slot, sizeof slot);
            }
        }
    }

    exp_rva = rd32(dd + 0);
    if (exp_rva) {
        const uint8_t *e = at(mem, size, exp_rva, 40);
        uint32_t nnames, afunc, anames, aord;
        if (!e) return fail(err, errlen, "export directory beyond image");
        nnames = rd32(e + 24);
        afunc = rd32(e + 28);
        anames = rd32(e + 32);
        aord = rd32(e + 36);
        for (uint32_t i = 0; i < nnames; i++) {
            const uint8_t *np = at(mem, size, anames + 4ull * i, 4);
            const uint8_t *op = at(mem, size, aord + 2ull * i, 2);
            uint16_t ord;
            const uint8_t *fp, *nm;
            uint32_t frva;
            if (!np || !op) return fail(err, errlen, "export name table beyond image");
            ord = rd16(op);
            fp = at(mem, size, afunc + 4ull * ord, 4);
            if (!fp) return fail(err, errlen, "export function table beyond image");
            frva = rd32(fp);
            nm = at(mem, size, rd32(np), 1);
            if (!nm) return fail(err, errlen, "export name beyond image");
            {
                char name[256];
                snprintf(name, sizeof name, "%.200s", (const char *)nm);
                if (add_sym(exports, &nexports, ibase + frva, name) != 0) return fail(err, errlen, "too many exports");
            }
        }
    }

    img->base = ibase;
    img->size = size;
    img->entry = ibase + entry_rva;
    img->nimports = nimports;
    img->nexports = nexports;
    return 0;
}
