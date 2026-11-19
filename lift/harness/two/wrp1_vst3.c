/* Decision 0183 (wrp1, PROVISIONAL, OPT-IN: linked only into the wrp1 native build; nothing else defines the two hooks): the loader's work for a third-party VST3 (Serum2.vst3, ImageBase 0x8000000) as a real module of the engine.
 * Ported from harness/vst3a/main_vst3a.c (decision 0165): (1) the IMPLICIT TLS of the module (S-vst-tls), (2) the IAT slots of the imports that have a row are set to the row's pseudo-VA, because Serum 2's code loads an
 * IAT slot into a register and calls it later (S-vst3a-iatptr; harness/image.c leaves each slot at its own VA), (3) the module's TLS callbacks run before the first DllMain (fle_realmod_pre_attach).
 * Only the module at WRP1_VST3_BASE is touched. The ordinal imports are not given pseudo-VAs (the 17 ordinal imports of Serum2.vst3 stay `fle_unsupported`: a named stop if reached). */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_realmod.h"
#include "fle_callback.h"
#include "fle_thread.h"

#ifndef WRP1_VST3_BASE
#define WRP1_VST3_BASE UINT64_C(0x8000000)
#endif
#define ULL(x) ((unsigned long long)(x))

extern uint64_t fle_tls_values[];
/* the ordinal imports of the VST3 module (generated, serum2/gen/ord.c by tools/mk-ord-c.py); absent = no ordinal gets a pseudo-VA */
extern const struct { const char *dll; int ordinal; uint64_t va; } wrp1_ord_table[] __attribute__((weak));
extern const unsigned wrp1_ord_count __attribute__((weak));
void fle_imp_TlsAlloc(void);

/* Decision 0200 (wrp3, wall W23): a minimal PEB for the TEB field ProcessEnvironmentBlock (TEB+0x60; Learn, TEB structure, winternl.h). Serum2's UCRT thread start (f_8a05724 -> f_8a057cc) reads
 * gs:[0x30] -> [+0x60] -> [PEB+0x20] (ProcessParameters; Learn PEB structure) -> [+8] and tests bit 31: the TEB of every thread but the first held 0, so the first Serum 2 worker thread stored through NULL + 0x20.
 * The PEB and the process parameters are zeroed host blocks (bit 31 of the flags word clear = a desktop process, the answer of docs/investigations/teb-peb-model.md section 3.1; S-wrp3-peb-minimal). */
extern uint8_t fle_teb[];
extern uint8_t *fle_teb_cur __attribute__((weak));
static uint64_t peb_va;
static uint64_t peb_get(void)
{
    if (!peb_va) {
        uint8_t *peb = (uint8_t *)calloc(1, 0x800), *params = (uint8_t *)calloc(1, 0x800);
        uint64_t pv = (uint64_t)(uintptr_t)params;
        memcpy(peb + 0x20, &pv, 8);
        peb_va = (uint64_t)(uintptr_t)peb;
    }
    return peb_va;
}
static void teb_peb(uint8_t *teb)
{
    uint64_t cur;
    memcpy(&cur, teb + 0x60, 8);
    if (!cur) { uint64_t p = peb_get(); memcpy(teb + 0x60, &p, 8); }
}

static uint64_t tls_cb[8];
static int ntls_cb;
/* decision 0200 (wrp3): what the loader needs again for every thread other than the first (the template of the module's TLS directory, the index, the load state) */
static uint64_t tls_idx_g, tls_start_g, tls_base_g;
static size_t tls_n_g, tls_raw_g;
static int tls_have, vst_loaded;

static int tls_setup(uint64_t base, uint64_t size)
{
    const uint8_t *m = (const uint8_t *)(uintptr_t)base;
    uint32_t lfa, rva, sz, zero;
    uint64_t start, end, idxva, cbva, block, idx;
    size_t n;
    memcpy(&lfa, m + 0x3c, 4);
    memcpy(&rva, m + lfa + 24 + 112 + 9 * 8, 4);
    memcpy(&sz, m + lfa + 24 + 112 + 9 * 8 + 4, 4);
    if (!rva || sz < 0x28) return 0;
    memcpy(&start, m + rva, 8); memcpy(&end, m + rva + 8, 8); memcpy(&idxva, m + rva + 16, 8); memcpy(&cbva, m + rva + 24, 8); memcpy(&zero, m + rva + 32, 4);
    if (end < start || start < base || end > base + size) { fprintf(stderr, "wrp1: bad TLS directory\n"); exit(2); }
    n = (size_t)(end - start) + zero;
    block = (uint64_t)(uintptr_t)aligned_alloc(64, (n + 63) & ~(size_t)63);
    memset((void *)(uintptr_t)block, 0, (n + 63) & ~(size_t)63);
    memcpy((void *)(uintptr_t)block, (void *)(uintptr_t)start, (size_t)(end - start));
    fle_imp_TlsAlloc();
    idx = (uint32_t)R[0];
    fle_tls_values[idx] = block;
    tls_idx_g = idx; tls_start_g = start; tls_base_g = base; tls_n_g = n; tls_raw_g = (size_t)(end - start); tls_have = 1;
    fle_st(idxva, 4, idx);
    if (cbva) for (int k = 0; k < 8 && fle_ld(cbva + 8u * k, 8); k++) tls_cb[ntls_cb++] = fle_ld(cbva + 8u * k, 8);
    printf("wrp1: VST3 TLS callbacks: %d; tls raw %llu B + zero %u -> index %llu\n", ntls_cb, ULL(end - start), zero, ULL(idx));
    return 1;
}

static int iat_fixup(uint64_t base)
{
    const uint8_t *m = (const uint8_t *)(uintptr_t)base;
    uint32_t lfa, imp_rva;
    int nfix = 0, nmiss = 0;
    memcpy(&lfa, m + 0x3c, 4);
    memcpy(&imp_rva, m + lfa + 24 + 112 + 8, 4);
    if (!imp_rva) return 0;
    for (uint32_t off = imp_rva;; off += 20) {
        uint32_t oft, namerva, ft;
        char dll[64];
        memcpy(&oft, m + off, 4); memcpy(&namerva, m + off + 12, 4); memcpy(&ft, m + off + 16, 4);
        if (!oft && !namerva && !ft) break;
        { size_t i; for (i = 0; i < 63 && m[namerva + i]; i++) dll[i] = (char)((m[namerva + i] >= 'A' && m[namerva + i] <= 'Z') ? m[namerva + i] + 32 : m[namerva + i]); dll[i] = 0; }
        for (uint32_t k = 0;; k++) {
            uint64_t th, j;
            const char *nm;
            memcpy(&th, m + (oft ? oft : ft) + 8u * k, 8);
            if (!th) break;
            if (th >> 63) {
                unsigned o = 0, noc = (&wrp1_ord_count != NULL) ? wrp1_ord_count : 0;
                for (o = 0; o < noc; o++)
                    if (!strcmp(wrp1_ord_table[o].dll, dll) && wrp1_ord_table[o].ordinal == (int)(th & 0xffff)) break;
                if (o < noc) { fle_st(base + ft + 8u * k, 8, wrp1_ord_table[o].va); nfix++; } else nmiss++;
                continue;
            }
            nm = (const char *)m + (th & 0x7fffffffu) + 2;
            for (j = 0; j < fle_resp_count; j++)
                if (!strcmp(fle_resp_table[j].dll, dll) && !strcmp(fle_resp_table[j].name, nm)) break;
            if (j == fle_resp_count) { nmiss++; continue; }
            fle_st(base + ft + 8u * k, 8, fle_resp_table[j].va);
            nfix++;
        }
    }
    printf("wrp1: VST3 IAT slots set to the row's pseudo-VA: %d (without a row or by ordinal: %d)\n", nfix, nmiss);
    return nfix;
}

static uint64_t vst_base, vst_size;
int wrp1_vst3_range(uint64_t *base, uint64_t *size) { *base = vst_base; *size = vst_size; return vst_base != 0; }
void wrp1_after_image_load(uint64_t base, uint64_t size)
{
    if (base != WRP1_VST3_BASE) return;
    vst_base = base; vst_size = size;
    (void)tls_setup(base, size);
    teb_peb(fle_teb);
    (void)iat_fixup(base);
}

void fle_realmod_pre_attach(uint64_t base, int idx)
{
    (void)idx;
    if (base != WRP1_VST3_BASE) return;
    for (int k = 0; k < ntls_cb; k++) {
        printf("wrp1: VST3 TLS callback %#llx(DLL_PROCESS_ATTACH)\n", ULL(tls_cb[k]));
        (void)fle_call_guest(tls_cb[k], base, 1, 0, 0);
    }
    vst_loaded = 1;
}

/* Decision 0200 (wrp3, OPT-IN; S-wrp3-tls-per-thread): the loader's per-thread TLS work for the VST3 module, called by fle_realmod_thread_notify (runtime/fle_realmod.c) in the context of the thread that starts (reason 2) or ends (3).
 * Learn (PE format, the .tls section): "The system copies all of this data each time a thread is created"; the TLS array is per thread and the module's slot holds its block; since Windows Vista the loader also allocates the
 * slot for a DLL that is loaded dynamically ("allocating TLS slots for such DLLs at load time"). So a thread has the block whether it started before or after the module was loaded: here the block is made when the thread starts,
 * whatever the load state. The TLS callbacks run with DLL_THREAD_ATTACH / DLL_THREAD_DETACH ("sent for all but the first thread") only while the module is loaded. The first thread keeps the block of tls_setup (fle_tls_values). */
void fle_realmod_pre_thread(uint32_t reason)
{
    uint64_t *tbl;

    if (fle_teb_cur) teb_peb(fle_teb_cur);   /* a thread's TEB: the PEB pointer (wall W23) */
    if (!tls_have) return;
    tbl = fle_thread_tls_values ? fle_thread_tls_values() : NULL;
    if (!tbl) return;
    if (reason == 2) {
        if (!tbl[tls_idx_g]) {
            uint64_t blk = (uint64_t)(uintptr_t)aligned_alloc(64, (tls_n_g + 63) & ~(size_t)63);
            memset((void *)(uintptr_t)blk, 0, (tls_n_g + 63) & ~(size_t)63);
            memcpy((void *)(uintptr_t)blk, (void *)(uintptr_t)tls_start_g, tls_raw_g);
            tbl[tls_idx_g] = blk;
            fprintf(stderr, "wrp3: TLS block %#llx for slot %llu of guest thread %u (table %p), callbacks %d\n", ULL(blk), ULL(tls_idx_g), fle_thread_current(), (void *)tbl, vst_loaded ? ntls_cb : 0);
        }
    }
    if (vst_loaded && (reason == 2 || reason == 3))
        for (int k = 0; k < ntls_cb; k++) (void)fle_call_guest(tls_cb[k], tls_base_g, reason, 0, 0);
}

/* ---- answers for the walls that decision 0165 met in the plug-in's own image (ported; see harness/vst3a/vst_env3a.c for the sources and the S-vst3a-* / S-vst-* names) ---- */

/* S-vst3a-vprotect: VirtualProtect on an address inside the plug-in's image (its CRT marks the 0x100 bytes of its .fptable read-only after DllMain starts) is a success; the old protection is the
 * section's (Characteristics: write -> PAGE_READWRITE 4, execute -> PAGE_EXECUTE_READ 0x20, else PAGE_READONLY 2) or the last one set on the same page; nothing is enforced (decision 0045).
 * Called from the --wrap of fle_imp_VirtualProtect (wrp1_wraps.c) before the runtime; returns 1 when it answered. */
static struct { uint64_t page; uint32_t prot; } vp_pages[64];
static int vp_n;

static uint32_t image_section_prot(uint64_t a)
{
    const uint8_t *m = (const uint8_t *)(uintptr_t)vst_base;
    uint32_t lfa, rva = (uint32_t)(a - vst_base);
    uint16_t nsec, osz;
    memcpy(&lfa, m + 0x3c, 4); memcpy(&nsec, m + lfa + 6, 2); memcpy(&osz, m + lfa + 20, 2);
    for (unsigned i = 0; i < nsec; i++) {
        const uint8_t *s = m + lfa + 24 + osz + 40u * i;
        uint32_t vs, va, ch;
        memcpy(&vs, s + 8, 4); memcpy(&va, s + 12, 4); memcpy(&ch, s + 36, 4);
        if (rva >= va && rva < va + vs) return (ch & 0x80000000u) ? 0x04u : (ch & 0x20000000u) ? 0x20u : 0x02u;
    }
    return 0x02u;
}

int wrp1_vst3_virtualprotect(void)
{
    uint64_t addr = R[1], size = R[2], pold = R[9];
    uint32_t np = (uint32_t)R[8], old;
    if (!vst_base || !(addr >= vst_base && addr + size <= vst_base + vst_size && size)) return 0;
    {
        uint64_t pg = addr & ~UINT64_C(0xfff);
        int k;
        if (!pold) { R[0] = 0; return 1; }
        old = image_section_prot(addr);
        for (k = 0; k < vp_n; k++) if (vp_pages[k].page == pg) old = vp_pages[k].prot;
        fle_st(pold, 4, old);
        for (uint64_t p = pg; p < addr + size; p += 0x1000) {
            for (k = 0; k < vp_n; k++) if (vp_pages[k].page == p) break;
            if (k == vp_n) { if (vp_n >= 64) { fprintf(stderr, "wrp1: more than 64 pages of the VST3 image changed\n"); exit(98); } vp_pages[vp_n++].page = p; }
            vp_pages[k].prot = np;
        }
        R[0] = 1;
    }
    return 1;
}

/* S-vst-lockresource: LockResource(h) for a value inside the plug-in's image: the image is mapped, nothing is locked, so the answer is the value itself (the runtime reads the resource tree of the exe image first). */
void __real_fle_imp_LockResource(void);
void __wrap_fle_imp_LockResource(void)
{
    uint64_t h = R[1];
    if (vst_base && h >= vst_base && h < vst_base + vst_size) { R[0] = h; return; }
    __real_fle_imp_LockResource();
}

/* S-vst-fmtmsg-lang: FormatMessageW with dwLanguageId 0x0400 (MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT)) is treated as 0, the default order (the runtime accepts 0 only). */
void __real_fle_imp_FormatMessageW(void);
void __wrap_fle_imp_FormatMessageW(void)
{
    if ((uint32_t)R[9] == 0x400u) R[9] = 0;
    __real_fle_imp_FormatMessageW();
}
