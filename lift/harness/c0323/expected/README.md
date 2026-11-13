# harness/c0323/expected (decision 0323, ci1)

- `wine-10.0.out`: contract_0323.c (scenarios E P T H) built with mingw and run under Wine 10.0 (headless, a prefix of its own). UNOFFICIAL reference: Wine is not Windows.
- `runtime-HEAD-3969ad31.out`: the same file against the runtime of HEAD 3969ad31 (before decision 0323): 130 differing lines against Wine (E 20, P 16, T 51, H 31).
- `runtime-0323.out`: against the runtime with the 0323 diff: 48 differing lines against Wine (E 0, P 0, T 35, H 13). Scenarios E and P are identical to Wine.
- `runtime-0323-scenario-N.out`, `runtime-HEAD-3969ad31-scenario-N.out`: scenario N (runtime only: `contract_0323 N`; Wine's null driver has no window surface to read): the pixels of a window after the erase of the update rectangle.
- `regress.sh TREE WORKDIR` (after `build.sh native TREE WORKDIR`): the native outputs of api2/win_upd.c, win_seq.c and win_act.c, to be compared between two trees. HEAD and 0323 differ only by 2 WM_ERASEBKGND lines in win_seq (Wine has them too) and 0 lines in win_upd and win_act.

The 48 lines that remain in T and H (known divergences, each judged against the Learn page):
- T1: Wine 10.0 does not walk from a transparent top-level window to a sibling top-level window (its window_from_point looks only below the top-level window the server chose); Learn WM_NCHITTEST says "underlying windows in the same thread". The runtime follows Learn.
- T4, T5: S-disabled-drops of decision 0136 (not part of 0323): Wine skips a disabled child (the parent gets the click) and answers a disabled top-level window with HTERROR (WM_SETCURSOR hit=-2); the runtime drops the event.
- the trailing WM_NCHITTEST that Wine sends after a click (a refresh of the hit test), and the WM_SETCURSOR of a button-up, which the runtime does not send (decision 0136).
- H: the runtime sends WM_SETCURSOR / WM_MOUSEACTIVATE / WM_PARENTNOTIFY when it translates the event, i.e. before the hook is called; Wine sends them after the hook, and not at all for a message the hook discards. PM_NOREMOVE: Wine hit-tests at the peek.

Build: `bash build.sh native|wine|both TREE WORKDIR [E|P|T|H|N]` (clang -O0 at nice 19; Wine in a prefix of its own under WORKDIR; about 18 s for the first run because of wineboot, about 2 s after).
