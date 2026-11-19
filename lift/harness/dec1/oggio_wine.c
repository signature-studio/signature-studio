/* Decision 0206 (dec1, opt-in test tool): calls the REAL oggio_x64.dll (Image-Line's, from the installer) under Wine and prints, per file, the decoded result
 * of OggioLoadFileW as the engine uses it (FLEngine f_675660): sample rate, channels, frames and an FNV-1a hash of the float planes of each channel.
 * Build: x86_64-w64-mingw32-gcc -O2 -o oggio_wine.exe oggio_wine.c      Run (oggio_x64.dll next to the exe): wine oggio_wine.exe FILE... [-d DUMPPREFIX]
 * TOggioLoadFileInfo layout: READ from the engine's use at f_675660 (info = 0x94 bytes at rbp+0x7c; +0x00 int rate, +0x04 int channels, +0x0c float** planes, +0x14 / +0x1c optional
 * pointers [smpl-like data], +0x24 input from the engine's settings, +0x2c u64 frames); the engine passes cb = a function that polls an abort flag and user = 0. */
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
typedef int (*loadw_t)(const wchar_t *, void *, void *, void *);
typedef void (*free_t)(void *);
static uint64_t fnv(const void *p, size_t n, uint64_t h) {
    const unsigned char *b = (const unsigned char *)p;
    for (size_t i = 0; i < n; i++) { h ^= b[i]; h *= 1099511628211ULL; }
    return h;
}
static int cb_calls;
static void __cdecl cb(void *user, unsigned long long a, unsigned long long b) { (void)user; (void)a; (void)b; cb_calls++; }
int main(int argc, char **argv) {
    HMODULE h = LoadLibraryA(getenv("DEC1_OGGIO") ? getenv("DEC1_OGGIO") : "oggio_x64.dll");
    const char *dump = NULL;
    if (!h) { fprintf(stderr, "no oggio_x64.dll (%lu)\n", GetLastError()); return 3; }
    loadw_t load = (loadw_t)GetProcAddress(h, "OggioLoadFileW");
    free_t fr = (free_t)GetProcAddress(h, "OggioFreeInfo");
    if (!load || !fr) { fprintf(stderr, "no export\n"); return 3; }
    for (int a = 1; a < argc; a++) {
        if (!strcmp(argv[a], "-d") && a + 1 < argc) { dump = argv[++a]; continue; }
        wchar_t wpath[1024];
        MultiByteToWideChar(CP_UTF8, 0, argv[a], -1, wpath, 1024);
        unsigned char info[0x100]; memset(info, 0, sizeof info);
        *(uint32_t *)(info + 0x24) = 0;
        cb_calls = 0;
        int rc = load(wpath, info, getenv("DEC1_NOCB") ? NULL : (void *)cb, NULL);
        int rate = *(int *)(info + 0), ch = *(int *)(info + 4);
        uint64_t frames = *(uint64_t *)(info + 0x2c);
        float **pl = *(float ***)(info + 0x0c);
        uint64_t hh = 14695981039346656037ULL;
        printf("file %s rc=%d rate=%d ch=%d frames=%llu", argv[a], rc, rate, ch, (unsigned long long)frames);
        if (rc == 0 && ch > 0 && ch <= 8 && pl) {
            for (int c = 0; c < ch; c++) {
                uint64_t hc = fnv(pl[c], (size_t)frames * 4, 14695981039346656037ULL);
                printf(" ch%d=%016llx", c, (unsigned long long)hc);
                hh = fnv(&hc, 8, hh);
                if (dump) { char fn[512]; snprintf(fn, sizeof fn, "%s.ch%d.f32", dump, c); FILE *f = fopen(fn, "wb"); if (f) { fwrite(pl[c], 4, (size_t)frames, f); fclose(f); } }
            }
            printf(" all=%016llx extra14=%p extra1c=%p", (unsigned long long)hh, *(void **)(info + 0x14), *(void **)(info + 0x1c));
            for (int k = 0; k < 2; k++) { const unsigned char *q = *(const unsigned char **)(info + (k ? 0x1c : 0x14)); printf(" s%s=\"", k ? "1c" : "14"); if (q) for (int j = 0; j < 160 && q[j]; j++) { if (q[j] >= 0x20 && q[j] < 0x7f && q[j] != 0x22 && q[j] != 0x5c) putchar(q[j]); else printf("\\x%02x", q[j]); } printf("\""); }
            fr(info);
        }
        printf(" cb=%d\n", cb_calls);
        fflush(stdout);
    }
    return 0;
}
