# census の結果（診断。A4 の進捗ではない）

生成: `census_report.py`。census は決定 0097 の診断ツールで、**数は A4 の進捗ではない**（決定 0046、0060）。放棄した unit は完了していない。

## 内訳: dsp

| ok | stop | fault | hang | empty | noproc | not_reached | 計 |
|---|---|---|---|---|---|---|---|
| 876 | 19 | 0 | 0 | 46 | 1 | 0 | 942 |

- InitUnits を通し終えた後、**unit の外**で停止（stop outside a unit）: `fle: unsupported instruction at 0x412752`（分類 b: unsupported Jmp at 0x412752）。top f_4126f0 ← f_412770 ← f_4128b0。
- 最初の放棄 unit: 149

## 内訳: single

| ok | stop | fault | hang | empty | noproc | not_reached | 計 |
|---|---|---|---|---|---|---|---|
| 873 | 22 | 0 | 0 | 46 | 1 | 0 | 942 |

- InitUnits を通し終えた後、**unit の外**で停止（stop outside a unit）: `fle: unsupported instruction at 0x412752`（分類 b: unsupported Jmp at 0x412752）。top f_4126f0 ← f_412770 ← f_4128b0。
- 最初の放棄 unit: 144

## 分類別件数（dsp）

| 分類 | 名前 | 放棄 unit 数 | 壁の種類数 |
|---|---|---|---|
| a | 未応答 import | 6 | 6 |
| b | 未対応命令・未解決 jmp・lift の意味の穴 | 1 | 1 |
| c | LoadLibrary／GetProcAddress の名前 | 6 | 4 |
| d | 仕様が無く停止（その他の名前付き停止） | 6 | 5 |
| e | メモリ違反・null 呼び | 0 | 0 |
| f | hang | 0 | 0 |
| g | その他のシグナル | 0 | 0 |

## 上位 20 の壁（dsp。件数の多い順、同数は最初の unit 番号順）

| # | 分類 | 壁（正規化したメッセージ） | 件数 | unit | 代表の呼び出し元（最初の unit） | 一次／二次 |
|---|---|---|---|---|---|---|
| 1 | c | LoadLibraryW "blend2d.dll" | 2 | 149,190 | f_423990 ← f_44db40 ← f_6539e0 (ret 0x44db70) | 一次 |
| 2 | d | RaiseException: exception code 0x…, flags 0, 1 argument(s) raised; no structured exception handling is modelled, stopped here | 2 | 245,265 | f_41cfa0 ← f_41d9a0 ← f_41cfc0 (ret 0x41dc6f) | 一次 |
| 3 | c | LoadLibraryW "iltools_x64.dll" | 2 | 280,659 | f_423990 ← f_44db40 ← f_6539e0 (ret 0x44db70) | 一次 |
| 4 | c | LoadLibraryW "quickfontcache_x64.dll" | 1 | 158 | f_423990 ← f_44db40 ← f_6539e0 (ret 0x44db70) | 一次 |
| 5 | a | msacm32.dll!acmMetrics | 1 | 170 | f_6708d0 ← f_672600 ← f_675290 (ret 0x672621) | 一次 |
| 6 | a | shell32.dll!SHGetMalloc | 1 | 235 | f_593340 ← f_708260 ← f_4120c0 (ret 0x7082b4) | 一次 |
| 7 | c | GetProcAddress by ordinal (lpProcName 0x…) is not supported | 1 | 238 | f_4235e0 ← f_4235f0 ← f_712080 (ret 0x423623) | 一次 |
| 8 | a | user32.dll!EnumDisplaySettingsW | 1 | 246 | f_424ae0 ← f_666d50 ← f_741880 (ret 0x666df3) | 一次 |
| 9 | a | user32.dll!GetClipCursor | 1 | 272 | f_424bd0 ← f_7f5350 ← f_4120c0 (ret 0x7f538d) | 一次 |
| 10 | d | SystemParametersInfoW: uiAction 0x… (SPI_GETWORKAREA) is used by the guest but no value is decided (decision 0058) | 1 | 275 | f_4255f0 ← f_867390 ← f_80e040 (ret 0x8673aa) | 一次 |
| 11 | d | GetSystemMetrics: index 94 (SM_DIGITIZER) is used by the guest but no value is decided (decision 0057) | 1 | 276 | f_424ee0 ← f_80e360 ← f_80e3f0 (ret 0x80e37c) | 一次 |
| 12 | a | gdi32.dll!EnumFontFamiliesExW | 1 | 283 | f_4240e0 ← f_866d80 ← f_818560 (ret 0x866e2a) | 一次 |
| 13 | b | indirect call to 0x477e73 (not a known function entry) | 1 | 296 | f_534b00 ← f_534eb0 ← f_4fa2a0 (ret 0x534edf) | 一次 |
| 14 | d | VirtualProtect: address is not in a VirtualAlloc region (the runtime does not track other memory) | 1 | 314 | f_423d00 ← f_8e0de0 ← f_8e0c60 (ret 0x8e0eb2) | 一次 |
| 15 | d | RegOpenKeyExW: an empty component (leading, trailing or doubled backslash) in a key path | 1 | 830 | f_4230d0 ← f_5b0cd0 ← f_5b1820 (ret 0x5b0d58) | 一次 |
| 16 | a | version.dll!GetFileVersionInfoSizeW | 1 | 880 | f_423e00 ← f_651ce0 ← f_6520b0 (ret 0x651d32) | 一次 |

壁の種類は全部で 16。一次／二次はヒューリスティック（確実でない。下の注）。

## 分類別件数（single）

| 分類 | 名前 | 放棄 unit 数 | 壁の種類数 |
|---|---|---|---|
| a | 未応答 import | 6 | 6 |
| b | 未対応命令・未解決 jmp・lift の意味の穴 | 1 | 1 |
| c | LoadLibrary／GetProcAddress の名前 | 7 | 5 |
| d | 仕様が無く停止（その他の名前付き停止） | 6 | 5 |
| e | メモリ違反・null 呼び | 2 | 1 |
| f | hang | 0 | 0 |
| g | その他のシグナル | 0 | 0 |

## 上位 20 の壁（single。件数の多い順、同数は最初の unit 番号順）

| # | 分類 | 壁（正規化したメッセージ） | 件数 | unit | 代表の呼び出し元（最初の unit） | 一次／二次 |
|---|---|---|---|---|---|---|
| 1 | c | LoadLibraryW "blend2d.dll" | 2 | 149,190 | f_423990 ← f_44db40 ← f_6539e0 (ret 0x44db70) | 一次 |
| 2 | d | RaiseException: exception code 0x…, flags 0, 1 argument(s) raised; no structured exception handling is modelled, stopped here | 2 | 245,265 | f_41cfa0 ← f_41d9a0 ← f_41cfc0 (ret 0x41dc6f) | 一次 |
| 3 | c | LoadLibraryW "iltools_x64.dll" | 2 | 280,659 | f_423990 ← f_44db40 ← f_6539e0 (ret 0x44db70) | 一次 |
| 4 | e | call through a null function pointer | 2 | 833,935 | f_10d79f0 ← f_4120c0 ← f_412160 (ret 0x412129) | 二次の疑い |
| 5 | c | LoadLibraryW "dsp_ippv2_x64.dll" | 1 | 144 | f_423990 ← f_44db40 ← f_6539e0 (ret 0x44db70) | 一次 |
| 6 | c | LoadLibraryW "quickfontcache_x64.dll" | 1 | 158 | f_423990 ← f_44db40 ← f_6539e0 (ret 0x44db70) | 一次 |
| 7 | a | msacm32.dll!acmMetrics | 1 | 170 | f_6708d0 ← f_672600 ← f_675290 (ret 0x672621) | 一次 |
| 8 | a | shell32.dll!SHGetMalloc | 1 | 235 | f_593340 ← f_708260 ← f_4120c0 (ret 0x7082b4) | 一次 |
| 9 | c | GetProcAddress by ordinal (lpProcName 0x…) is not supported | 1 | 238 | f_4235e0 ← f_4235f0 ← f_712080 (ret 0x423623) | 一次 |
| 10 | a | user32.dll!EnumDisplaySettingsW | 1 | 246 | f_424ae0 ← f_666d50 ← f_741880 (ret 0x666df3) | 一次 |
| 11 | a | user32.dll!GetClipCursor | 1 | 272 | f_424bd0 ← f_7f5350 ← f_4120c0 (ret 0x7f538d) | 一次 |
| 12 | d | SystemParametersInfoW: uiAction 0x… (SPI_GETWORKAREA) is used by the guest but no value is decided (decision 0058) | 1 | 275 | f_4255f0 ← f_867390 ← f_80e040 (ret 0x8673aa) | 一次 |
| 13 | d | GetSystemMetrics: index 94 (SM_DIGITIZER) is used by the guest but no value is decided (decision 0057) | 1 | 276 | f_424ee0 ← f_80e360 ← f_80e3f0 (ret 0x80e37c) | 一次 |
| 14 | a | gdi32.dll!EnumFontFamiliesExW | 1 | 283 | f_4240e0 ← f_866d80 ← f_818560 (ret 0x866e2a) | 一次 |
| 15 | b | indirect call to 0x477e73 (not a known function entry) | 1 | 296 | f_534b00 ← f_534eb0 ← f_4fa2a0 (ret 0x534edf) | 一次 |
| 16 | d | VirtualProtect: address is not in a VirtualAlloc region (the runtime does not track other memory) | 1 | 314 | f_423d00 ← f_8e0de0 ← f_8e0c60 (ret 0x8e0eb2) | 一次 |
| 17 | d | RegOpenKeyExW: an empty component (leading, trailing or doubled backslash) in a key path | 1 | 830 | f_4230d0 ← f_5b0cd0 ← f_5b1820 (ret 0x5b0d58) | 一次 |
| 18 | a | version.dll!GetFileVersionInfoSizeW | 1 | 880 | f_423e00 ← f_651ce0 ← f_6520b0 (ret 0x651d32) | 一次 |

壁の種類は全部で 18。一次／二次はヒューリスティック（確実でない。下の注）。

## 比較: dsp と single

- 同じ unit で同じ壁: 19 件。違い: 3 件。

| unit | 関係 |
|---|---|
| 144 | dsp では ok、single では放棄（LoadLibraryW "dsp_ippv2_x64.dll"） |
| 833 | dsp では ok、single では放棄（call through a null function pointer） |
| 935 | dsp では ok、single では放棄（call through a null function pointer） |

- **測定した依存**: 一方の構成でだけ放棄される unit は、その unit 自身の壁ではなく、**構成の違い**（実在モジュールの有無と、それに伴う前の unit の完了）に依存した失敗である（二次被害の実測の証拠）。ただし 2 つの実行は実在モジュールの有無だけでなく前の unit の完了も変えるので、どの変化が原因かはこの比較だけでは切り分けていない。

## 二次被害のヒューリスティック（確実でない）

- dsp: 該当なし（最初の放棄 unit より後で、null 呼び・null 参照・NULL／0x0 ハンドルの手掛かりがある停止は無かった）。
- single:

| unit | 壁 | 理由 |
|---|---|---|
| 833 | call through a null function pointer | after unit 144; cue: call through a null function pointer |
| 935 | call through a null function pointer | after unit 144; cue: call through a null function pointer |


定義: 最初に放棄された unit より後に失敗し、メッセージに「前の unit が設定すべき値が無い」手掛かり（null 関数ポインタ呼び、null ページへのアクセス、`NULL`、`hModule 0x0`、`is 0`）がある unit に印を付ける。手掛かりが無い unit は「一次」と書くが、それは**前の放棄の結果ではないことの証明ではない**（放棄した unit が書くはずだったメモリ・ランタイムの表は戻していない）。

