/* A small open-addressing map from a live guest pointer (key >= 2) to a value, used by fle_oleaut.c and fle_ole.c to
 * know which blocks they handed out (the fx_* set in fle_glob.c does the same for GlobalAlloc). Key 0 marks an empty
 * slot and key 1 a deleted one. Internal to the runtime (docs/decisions/0037-apis-oleaut-ole.md). */
#ifndef FLE_PTRMAP_H
#define FLE_PTRMAP_H

#include <stdint.h>
#include <stdlib.h>

#include "fle_env.h"

typedef struct {
    uint64_t *key;
    uint64_t *val;
    uint64_t cap, used, live;
} fle_pmap;

static inline uint64_t fle_pm_hash(uint64_t k)
{
    uint64_t h = (k >> 3) * UINT64_C(0x9E3779B97F4A7C15);
    return h ^ (h >> 29);
}

/* 1 and *v set when k is present. */
static inline int fle_pm_get(const fle_pmap *m, uint64_t k, uint64_t *v)
{
    uint64_t i, n;

    if (!m->cap || k < 2) return 0;
    for (n = 0, i = fle_pm_hash(k) & (m->cap - 1); n < m->cap; n++, i = (i + 1) & (m->cap - 1)) {
        if (m->key[i] == 0) return 0;
        if (m->key[i] == k) {
            if (v) *v = m->val[i];
            return 1;
        }
    }
    return 0;
}

static inline void fle_pm_put_raw(fle_pmap *m, uint64_t k, uint64_t v)
{
    uint64_t i = fle_pm_hash(k) & (m->cap - 1);

    while (m->key[i] > 1) i = (i + 1) & (m->cap - 1);
    if (m->key[i] == 0) m->used++;
    m->key[i] = k;
    m->val[i] = v;
    m->live++;
}

/* Adds k, which must not be present. */
static inline void fle_pm_put(fle_pmap *m, const char *api, uint64_t k, uint64_t v)
{
    if ((m->used + 1) * 2 > m->cap) {
        uint64_t *ok = m->key, *ov = m->val, oc = m->cap, i;
        uint64_t nc = m->cap ? m->cap * 2 : 256;

        m->key = calloc(nc, sizeof *m->key);
        m->val = calloc(nc, sizeof *m->val);
        if (!m->key || !m->val) fle_stop(api, "out of memory");
        m->cap = nc;
        m->used = m->live = 0;
        for (i = 0; i < oc; i++)
            if (ok[i] > 1) fle_pm_put_raw(m, ok[i], ov[i]);
        free(ok);
        free(ov);
    }
    fle_pm_put_raw(m, k, v);
}

/* 1 when k was present and is now removed. */
static inline int fle_pm_del(fle_pmap *m, uint64_t k)
{
    uint64_t i, n;

    if (!m->cap || k < 2) return 0;
    for (n = 0, i = fle_pm_hash(k) & (m->cap - 1); n < m->cap; n++, i = (i + 1) & (m->cap - 1)) {
        if (m->key[i] == 0) return 0;
        if (m->key[i] == k) {
            m->key[i] = 1;
            m->live--;
            return 1;
        }
    }
    return 0;
}

#endif
