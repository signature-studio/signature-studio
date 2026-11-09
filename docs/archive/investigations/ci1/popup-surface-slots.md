# File menu popup disappears after about 30 openings (the surface table of lift/harness/fruity_surface.c has 32 slots that are never given back)

Observed 2026-11-04 on the bundle that production got (main 15540a0, F2xH2) and also on `fl-engine-ship2-F2xH2` (the page of 15540a0 in both): after the welcome wizard is closed, the File menu (click at (18,20)) is opened and closed with Escape again and again.
The first 5 openings show the menu (`TQuickPopupMenuWindow` 208x298 at (4,36), a canvas in the page, the list of items in the screenshot). After 40 openings the window table still lists the popup as visible, but the page has no canvas for it:
`window.__app.surfaces()` has 2 entries (the main form, the wizard) and the screenshot shows only the highlighted "FILE" header. The engine does not stop.

Cause (read, then confirmed by the fix below): `note_write()` in `lift/harness/fruity_surface.c` gives every new drawing surface the next entry of a table with `CAP 32` entries (`k = used++`), `refresh()` marks the entry of a destroyed window dead (`flags = 0`) but nothing ever gives the entry back, and `if (used == CAP) return;` silently drops every surface after the 32nd. A menu, a dialog or a tooltip is a new window (a new lid, a new surface view) each time it opens, so after 32 surfaces over the whole session (the main form, the wizard, the popups opened so far, ...) no new window is shown.
CONTRACT.md section 2 says that a free entry is recognised by `flags & 1` (not by lid) and that lids are not reused, which is what a reuse needs.
A second, smaller defect of the same code: the dead entry kept `view_of[k]`, so a new view allocated at the freed address would have found the dead entry (`find_entry`) and every write to it would have been dropped by `if (!(s->flags & F_LIVE)) return;`.

Not FL specific: the table is the harness's registry of surfaces for the page. No class name, no window kind.
Not a regression: the same sequence on the SHIP2 bundle (old runtime) gives the same result.

Fix (`ci1-popup.diff`: `lift/harness/fruity_surface.c`, the native test `lift/harness/web/present/test_fruity_surface_slots.c`, this note): a new view first takes a free entry (the lowest with `flags & 1 == 0`), then grows the table; the reused entry is reset (lid, hwnd, flags, geometry, hints, dirty rectangle, layer words; `lseq` is bumped around the change, `gen`/`layout_gen` keep going up) so that the existing "new buffer" branch makes it all dirty; `view_of[k] = NULL` when an entry dies; `refresh()` runs before a free entry is looked for (the idle hook does it too, but a menu can close and another window open within one idle).
Left as it is: `LID_SCAN_MAX 4096` (a window with a lid above 4095 is recorded with lid 0xFFFFFFFF; each popup uses a new lid, so a session with ~4,000 windows would reach it), and the table still has 32 entries for 32 windows alive at the same time.

Test (`test_fruity_surface_slots.c`, same style as `test_fruity_surface_layer.c`: fruity_surface.c included with the stub `<emscripten.h>` and fakes for the runtime; usage in its header):
250 windows opened and closed next to a main window that stays (every one recorded, the table stays at 2 entries, live count 2, lseq even, gen/layout_gen only up, nothing of the old occupant left, whole new buffer dirty);
a new view at a dead view's address is recorded; 32 windows alive at once are all recorded, the 33rd is not and is as soon as one is gone, the others keep their entries; `fle_surface_take_dirty` / `fle_surface_info` for the new lid.
Result: the original `fruity_surface.c` (164e81a) fails 11 of 20 checks (the first at the 33rd window: "no entry for lid 33 (used 32, live 6)"); the fixed file passes 20 of 20; the existing `test_fruity_surface_layer.c` still passes 27 of 27 with the fixed file.

desk1 (`lift/out-git/desk1-results/desk1.diff`, `desk1-remove.diff`) does not touch `fruity_surface.c` or `web/present/`; `desk1.diff` changes `fle_raster.c` only under `#ifdef FLE_DESKTOP` (a call after the write, `fle_desk_wrote`), so `fle_raster_wrote` / `fle_raster_surface_peek` are what this file relies on. `desk1-remove.diff` deletes the page side that reads this table (`surface-events.ts`, `surface-info.ts`, `surface-sync.ts`, `surface-table.ts`): if it is adopted this table and this fix are no longer read by the page.

Limits that remain (not fixed): (1) 32 windows alive at the same time (`CAP 32`); the 33rd gets no entry and is not shown. (2) `LID_SCAN_MAX 4096`: a window whose lid is above 4095 is recorded with lid 0xFFFFFFFF and the page cannot tell two of them apart; a popup uses a new lid at every opening, so a session with about 4,000 windows (opening a menu about 4,000 times) reaches it.
