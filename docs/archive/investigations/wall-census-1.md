# wall-census-1 (粗い第 1 版。分類・要約は後続の版で置き換える)

**census の数は A4 の進捗ではない**（決定 0046、0097）。放棄した unit は完了していない。

構成: FLEngine 単独（census 実行 1 回目、`runs/single-raw-1.tsv` = `wall-census-1.tsv`）。

942 unit の内訳: ok 873 / stop 22 / fault 0 / hang 0 / empty 46 / noproc 1 / not_reached 0。
InitUnits を最後まで通した後、guest 自身の終了フェーズ（entry f_1376820 が f_41d8f0 の後に無条件で f_4128b0 を呼ぶ）で
`fle: unsupported instruction at 0x412752`（f_4126f0）が出て終了した（unit の外の停止）。

停止した 22 unit（unit: メッセージの要点）:

- 144: LoadLibraryW "dsp_ippv2_x64.dll" / 149: "blend2d.dll" / 158: "quickfontcache_x64.dll" / 190: "blend2d.dll" / 280, 659: "iltools_x64.dll"
- 170 msacm32!acmMetrics、235 shell32!SHGetMalloc、246 user32!EnumDisplaySettingsW、272 user32!GetClipCursor、283 gdi32!EnumFontFamiliesExW、880 version!GetFileVersionInfoSizeW（IAT thunk に応答なし）
- 238 GetProcAddress by ordinal 0x10、245/265 RaiseException（0xc06d007e / 0xc06d007f）
- 275 SystemParametersInfoW SPI_GETWORKAREA、276 GetSystemMetrics SM_DIGITIZER
- 296 indirect call to 0x477e73、833/935 indirect call to 0（null 関数ポインタ。二次被害の疑い）
- 314 VirtualProtect（VirtualAlloc 領域外）、830 RegOpenKeyExW（空の key 成分）

分類の整理は次の版（wall-census-2 以降）で行う。
