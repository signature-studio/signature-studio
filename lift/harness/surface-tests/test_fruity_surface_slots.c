/* The slots of the surface table of lift/harness/fruity_surface.c (CAP entries): an entry whose window is gone is given back and reused, a new view is never matched with a dead entry, and what the limit is.
 * Found 2026-11-04 (docs/investigations/ci1/popup-surface-slots.md): the table never gave an entry back, so the 33rd surface of a session got no entry and the File menu (a new window at every opening) was
 * not shown after about 29 openings. Checked natively like test_fruity_surface_layer.c: fruity_surface.c is included with a stub <emscripten.h> (test-stubs/) and the runtime's answers are fakes below.
 * What is checked: (1) 250 windows opened and closed one after the other, with a main window that stays: every one gets an entry, the table stays at 2 entries, live count and fle_surface_count() are 2
 * after every opening, gen only goes up in a reused entry, lid is the new window's, nothing of the old occupant is left (geometry, hints, dirty rectangle, layer words), the new buffer is all dirty,
 * lseq is even; (2) a new view allocated at the address of a dead one is recorded (the dead entry is not found by address); (3) CAP windows alive at once are all recorded, the next one is not (the limit,
 * no crash, no entry of another window is damaged) and is recorded as soon as one entry is free; (4) fle_surface_take_dirty / fle_surface_info answer for the new lid and not for the old one; (5) the order
 * of the entries of fle_surface_info is the order of the table.
 * usage:  clang -std=gnu11 -O1 -Wall -Wextra -D__EMSCRIPTEN__=1 -I test-stubs -I ../../../runtime -o OUT test_fruity_surface_slots.c && OUT        (exit 0 = all passed) */
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include "../fruity_surface.c"

/* ---- the fakes ---- */
#define NLID 400
static fle_rsurf views[NLID + 1];
static uint32_t pix[NLID + 1][16];
static int alive[NLID + 1];
static const fle_rsurf *at_address[NLID + 1];   /* when set, the window's view is this address instead of its own struct (a freed view's address handed out again) */

static const fle_rsurf *view_for(uint32_t lid) { return at_address[lid] ? at_address[lid] : &views[lid]; }
const fle_rsurf *fle_raster_surface_peek(uint32_t lid) { return lid >= 1 && lid <= NLID && alive[lid] ? view_for(lid) : NULL; }
int fle_raster_evidence_seen(void) { return 0; }
void __real_fle_raster_wrote(const fle_rsurf *surf, fle_rrect r, const char *api) { (void)surf; (void)r; (void)api; }
uint64_t trace_call_count(void) { return 0; }
int fle_win_dc_query(uint64_t hwnd, fle_win_dcq *q) {
    uint32_t lid = (uint32_t)((hwnd - FLE_WIN_HANDLE_BASE) / 4);
    if (lid < 1 || lid > NLID || !alive[lid]) return 0;
    memset(q, 0, sizeof *q);
    q->lid = lid;
    q->visible = 1;
    return 1;
}
int fle_win_get(uint64_t hwnd, fle_win_view *v) {
    uint32_t lid = (uint32_t)((hwnd - FLE_WIN_HANDLE_BASE) / 4);
    if (lid < 1 || lid > NLID || !alive[lid]) return 0;
    memset(v, 0, sizeof *v);
    v->x = 4; v->y = 36; v->style = 0x90000000u; v->exstyle = 0x10008u;   /* a popup: topmost, not layered (what TQuickPopupMenuWindow has) */
    return 1;
}
int fle_layered_query(uint64_t hwnd, fle_layered_info *out) { (void)hwnd; (void)out; return 0; }

/* ---- the checks ---- */
static int fails, checks;
#define CHECK(c, ...) do { checks++; if (!(c)) { fails++; printf("FAIL line %d: ", __LINE__); printf(__VA_ARGS__); printf("\n"); } } while (0)

static const fle_surface_info_t *entry_of(uint32_t lid) {
    for (uint32_t k = 0; k < used; k++) if ((T.e[k].flags & F_LIVE) && T.e[k].lid == lid) return &T.e[k];
    return NULL;
}
static uint32_t live_entries(void) { uint32_t n = 0; for (uint32_t k = 0; k < used; k++) n += (T.e[k].flags & F_LIVE) ? 1u : 0u; return n; }
static void open_window(uint32_t lid) {
    memset(&views[lid], 0, sizeof views[lid]);
    views[lid].bits = (uint8_t *)pix[lid]; views[lid].width = 4; views[lid].height = 4; views[lid].stride = 16; views[lid].bpp = 32;
    alive[lid] = 1;
}
static void close_window(uint32_t lid) { alive[lid] = 0; }
static void write_to(uint32_t lid) { fle_rrect r = {0, 0, 2, 2}; __wrap_fle_raster_wrote(view_for(lid), r, "test"); }

int main(void) {
    (void)fle_surface_table();

    /* (1) a main window that stays and 250 popups that come and go */
    open_window(1); write_to(1);
    uint32_t gen_before[CAP] = {0}, lgen_before[CAP] = {0};
    int all_recorded = 1, small = 1, counts = 1, even = 1, fresh = 1, gen_up = 1, lid_ok = 1, dirty_all = 1;
    for (uint32_t lid = 2; lid <= 251; lid++) {
        open_window(lid); write_to(lid);
        const fle_surface_info_t *e = entry_of(lid);
        if (!e) { all_recorded = 0; printf("no entry for lid %u (used %u, live %u)\n", lid, used, T.live); break; }
        uint32_t k = (uint32_t)(e - T.e);
        if (used > 2u) small = 0;
        if (T.live != 2u || live_entries() != 2u || fle_surface_count() != 2u) counts = 0;
        if ((e->lseq & 1u) || (T.e[0].lseq & 1u) || (T.e[1].lseq & 1u)) even = 0;
        if (e->width != 4 || e->height != 4 || e->stride != 16 || e->hwnd != (uint32_t)(FLE_WIN_HANDLE_BASE + 4u * lid) || e->x != 4 || e->y != 36 || e->exstyle != 0x10008u ||
            (e->flags & ~(F_LIVE | F_VISIBLE)) || e->owner_lid || e->form_key || e->layer_mode || e->layer_alpha || e->layer_key || e->layer_flags) fresh = 0;
        if (!(e->dx0 == 0 && e->dy0 == 0 && e->dx1 == 4 && e->dy1 == 4)) dirty_all = 0;   /* a new buffer is all dirty (the 2x2 write is inside it) */
        if (e->gen <= gen_before[k] || e->layout_gen <= lgen_before[k]) gen_up = 0;
        gen_before[k] = e->gen; lgen_before[k] = e->layout_gen;
        if (e->lid != lid) lid_ok = 0;
        if (lid % 7 == 0) { fle_surface_take_dirty(lid, (uint32_t[5]){0}); }
        close_window(lid);
    }
    CHECK(all_recorded, "every one of 250 windows got an entry");
    CHECK(small, "the table stayed at 2 entries (used %u)", used);
    CHECK(counts, "live count == fle_surface_count() == number of live entries == 2 after every opening (T.live %u)", T.live);
    CHECK(even, "lseq is even after every opening");
    CHECK(fresh, "a reused entry has the new window's geometry/hwnd and nothing of the old occupant");
    CHECK(dirty_all, "a reused entry's dirty rectangle is the whole new buffer");
    CHECK(gen_up, "gen and layout_gen of a slot only go up across occupants");
    CHECK(lid_ok, "the entry carries the new window's lid");
    CHECK(entry_of(1) != NULL && entry_of(1) == &T.e[0], "the main window kept its entry (slot 0)");
    CHECK(fle_surface_count() == 1u, "after the last popup closed one surface is live (%u)", fle_surface_count());

    /* (4) the answers for the old lid and the new one */
    open_window(252); write_to(252);
    uint32_t out[5] = {0};
    CHECK(fle_surface_take_dirty(252, out) == 1 && out[3] - out[1] == 4u && out[4] - out[2] == 4u, "take_dirty of the new lid: nonempty, whole buffer");
    CHECK(fle_surface_take_dirty(251, out) == -1, "take_dirty of a gone lid: -1");
    fle_surface_info_t copy;
    CHECK(fle_surface_info(0, &copy) == 0 && copy.lid == 1 && fle_surface_info(1, &copy) == 0 && copy.lid == 252 && fle_surface_info(2, &copy) == -1, "fle_surface_info lists the live entries in table order");
    close_window(252); close_window(1);
    CHECK(fle_surface_count() == 0u, "everything closed: no live surface");

    /* (2) a new view at the address of a dead one (the allocator hands the freed address out again) */
    open_window(260); write_to(260);
    const fle_rsurf *freed = view_for(260);
    close_window(260); (void)fle_surface_count();   /* refresh(): the entry dies */
    open_window(261);
    at_address[261] = freed;                         /* lid 261's view lives at the freed address; its pixels are 261's */
    write_to(261);
    const fle_surface_info_t *e261 = entry_of(261);
    CHECK(e261 != NULL, "a new view at a freed view's address is recorded (the dead entry is not matched by address)");
    CHECK(e261 && e261->flags == (F_LIVE | F_VISIBLE) && e261->width == 4, "and it is live and sized");
    close_window(261); at_address[261] = NULL; (void)fle_surface_count();

    /* (3) CAP windows alive at once are recorded, the next is not, and it is as soon as an entry is free */
    for (uint32_t i = 0; i < CAP; i++) { open_window(300 + i); write_to(300 + i); }
    int all32 = 1;
    for (uint32_t i = 0; i < CAP; i++) if (!entry_of(300 + i)) all32 = 0;
    CHECK(all32 && T.live == CAP && used == CAP, "%u windows alive at once are all recorded (live %u, used %u)", (unsigned)CAP, T.live, used);
    open_window(340); write_to(340);
    CHECK(entry_of(340) == NULL && T.live == CAP, "the %uth window alive at once has no entry (the limit), nothing else changes", (unsigned)CAP + 1u);
    close_window(310);
    write_to(340);
    CHECK(entry_of(340) != NULL && T.live == CAP, "as soon as one window is gone the waiting window is recorded (live %u)", T.live);
    int others = 1;
    for (uint32_t i = 0; i < CAP; i++) if (300 + i != 310 && !entry_of(300 + i)) others = 0;
    CHECK(others, "the other %u windows kept their entries", (unsigned)CAP - 1u);

    printf("%s: %d checks, %d failed\n", fails ? "FAILED" : "ok", checks, fails);
    return fails ? 1 : 0;
}
