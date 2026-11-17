/* Decision 0247b (alpha3b): the same scene as upd_region.c's uncover_scene on Windows API (build for Wine with x86_64-w64-mingw32-gcc, and for the runtime with harness/alpha3/build-wine2.sh-style linking of tools/apiw.h): top-level T, child P (CLIPSIBLINGS, with or without
 * CLIPCHILDREN), child C of P at P's right edge (CLIPCHILDREN), child M of T above P; M moves away from over P. Prints what P and C have to repaint. */
#include <windows.h>
#include <stdio.h>

static LRESULT CALLBACK wp(HWND h, UINT m, WPARAM w, LPARAM l) { return DefWindowProcW(h, m, w, l); }
static void pump(void) { MSG m; while (PeekMessageW(&m, NULL, 0, 0, PM_REMOVE)) DispatchMessageW(&m); }

static void scene(const char *what, int clip, int tx)
{
    HWND t = CreateWindowExW(0, L"unc", L"", WS_POPUP | WS_VISIBLE | (clip ? WS_CLIPCHILDREN : 0), tx, 300, 600, 400, NULL, NULL, NULL, NULL);
    HWND p = CreateWindowExW(0, L"unc", L"", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | (clip ? WS_CLIPCHILDREN : 0), 0, 0, 300, 300, t, NULL, NULL, NULL);
    HWND c = CreateWindowExW(0, L"unc", L"", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN, 250, 0, 50, 300, p, NULL, NULL, NULL);
    HWND m = CreateWindowExW(0, L"unc", L"", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS, 150, 0, 150, 150, t, NULL, NULL, NULL);
    HRGN rg = CreateRectRgn(0, 0, 0, 0);
    RECT rc;
    int r;

    SetWindowPos(m, HWND_TOP, 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE);
    pump();
    ValidateRect(t, NULL); ValidateRect(p, NULL); ValidateRect(c, NULL); ValidateRect(m, NULL);
    SetWindowPos(m, 0, 400, 0, 150, 150, SWP_NOZORDER | SWP_NOACTIVATE);
    r = GetUpdateRgn(p, rg, FALSE);
    printf("%s: P update region type %d beside-the-child(200,50)=%d over-the-child(275,50)=%d outside(50,50)=%d outside(100,200)=%d\n", what, r, PtInRegion(rg, 200, 50), PtInRegion(rg, 275, 50), PtInRegion(rg, 50, 50), PtInRegion(rg, 100, 200));
    r = GetUpdateRect(c, &rc, FALSE);
    printf("%s: C update rect %d (%ld,%ld,%ld,%ld)\n", what, r, rc.left, rc.top, rc.right, rc.bottom);
    r = GetUpdateRect(t, &rc, FALSE);
    printf("%s: T update rect %d (%ld,%ld,%ld,%ld)\n", what, r, rc.left, rc.top, rc.right, rc.bottom);
    DestroyWindow(t);
}

int main(void)
{
    WNDCLASSW wc = { 0 };

    wc.lpfnWndProc = wp;
    wc.lpszClassName = L"unc";
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    RegisterClassW(&wc);
    scene("no WS_CLIPCHILDREN on T and P", 0, 400);
    scene("WS_CLIPCHILDREN on T and P", 1, 1100);
    return 0;
}
