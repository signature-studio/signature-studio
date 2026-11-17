# wall-r の測定記録（census の前後。診断であって A4 の進捗ではない）

**census の数は A4 の進捗ではない**（決定 0046、0060、0097）。放棄した unit は完了していない。ここの「ok」は「ゲストのループに戻った」だけで、unit の内部で fail-soft に進んだかは見ていない。決定 0098 の API 応答（すべて仮の決定）の前後で、同じ census（`lift/harness/census/run_census.sh`、決定 0097）を、同じ木（HEAD 5002d98 + census.diff、dsp 構成は FLEngine + 縮小 dsp_ippv2 基底 0x2000000）に応答だけを足した木で動かして比べた。

凡例: **実測** = census の TSV と stdout、テストの出力。**読み** = 逆アセンブル・生成 C を読んだ結果で、実行して確かめていない。**未確認** = そのとおり。

## 1. 内訳（実測）

| 構成 | 状態 | ok | stop | fault | hang | empty | noproc | 計 |
|---|---|---|---|---|---|---|---|---|
| dsp（FLEngine + 縮小 dsp_ippv2） | 前（`runs/before-1`） | 876 | 19 | 0 | 0 | 46 | 1 | 942 |
| dsp | 後（`runs/after-1`、`runs/after-2`） | **881** | **14** | 0 | 0 | 46 | 1 | 942 |
| 単独（FLEngine のみ） | 前 | 873 | 22 | 0 | 0 | 46 | 1 | 942 |
| 単独 | 後 | **877** | **18** | 0 | 0 | 46 | 1 | 942 |

- 「前」は census が報告した値（dsp 876/19、単独 873/22）と**完全に一致**した（自分のビルドで再測定。8 列までの TSV は census の `dsp-raw-1.tsv`、`single-raw-1.tsv` と同一）。
- 「後」を 2 回（`after-1`、ソース確定後に作り直したビルドの `after-2`）測り、経過時間の列を除いて**全 unit が同一**。
- 前後で結果が変わった unit は dsp 構成で 8 つ（下の表）。**それ以外の 934 unit は状態もメッセージも同じ**。
- 壁の種類（dsp）: 16 → **11**（未応答 import の分類 a は 6 → 0）。

## 2. 担当した 8 unit（dsp 構成。実測）

| unit | 壁（前） | 後 | 次の壁（同じ unit で次に見えたもの） |
|---|---|---|---|
| 170 | `msacm32.dll!acmMetrics` | **ok** | （単独構成では unit 144 が放棄されるため、同じ unit が `indirect call to 0`（`f_672600`）で止まる。二次の疑い。dsp 構成では ok） |
| 235 | `shell32.dll!SHGetMalloc` | **ok** | — |
| 246 | `user32.dll!EnumDisplaySettingsW` | **ok** | — |
| 272 | `user32.dll!GetClipCursor` | **ok** | — |
| 275 | `SystemParametersInfoW` uiAction 0x30 | **ok** | — |
| 276 | `GetSystemMetrics` index 94 | stop | `GetSystemMetrics: index 86 (SM_TABLETPC) is used by the guest but no value is decided (decision 0057)`。同じ関数 `f_80e360` が続けて 0x2003（SM_CONVERTIBLESLATEMODE）、0x2004（SM_SYSTEMDOCKED）を問う（読み）。0x2003 は 0 のときゲストがスレートモードの経路へ入るので、値は未決。86 だけ決めても unit は ok にならない |
| 283 | `gdi32.dll!EnumFontFamiliesExW` | stop | `CompareStringW: character other than an ASCII letter or digit in unequal strings`（決定 0072 の枠）。top `f_4231c0` ← `f_4399e0` ← `f_8650c0`（コールバック）。ゲストのコールバックがリストの "Default" と "Segoe UI" を比べ、空白が枠の外（読み）。**EnumFontFamiliesExW 自体は答えてコールバックまで進んだ**（実測） |
| 880 | `version.dll!GetFileVersionInfoSizeW` | stop | `indirect call to 0 is not a known function entry`。top `f_125da90` ← unit の proc `f_1263540`。読み: `f_125da90` が `LoadLibraryW(L"SHFolder.dll")`（仮想モジュール）→ `GetProcAddress(…, L"SHGetFolderPathW")`（応答表に行がなく NULL）→ NULL を呼ぶ（CSIDL 0x1a）。census は「二次の疑い」と印を付けたが、読みでは SHGetFolderPathW の未応答という**一次**の壁（`f_125dc16`〜`0x125dc55` を読んだ。実行は未確認） |

単独構成（実測）: 235、246、272、275 が ok（dsp 構成と同じ）。170 は unit 144（`LoadLibraryW "dsp_ippv2_x64.dll"`）が放棄された後の状態で `f_672600` の null 関数ポインタ呼び（`[0x15e0a48]` が NULL）になる。276、283、880 は dsp 構成と同じ次の壁。

## 3. 壁の一覧（dsp。後。実測）

| 分類 | 壁 | unit |
|---|---|---|
| c | `LoadLibraryW "blend2d.dll"` | 149、190 |
| d | `RaiseException`（0xc06d007e、0xc06d007f） | 245、265 |
| c | `LoadLibraryW "iltools_x64.dll"` | 280、659 |
| c | `LoadLibraryW "quickfontcache_x64.dll"` | 158 |
| c | `GetProcAddress` by ordinal | 238 |
| d | `GetSystemMetrics` index 86（SM_TABLETPC） | 276 |
| d | `CompareStringW`（ASCII の枠） | 283 |
| b | `indirect call to 0x477e73` | 296 |
| d | `VirtualProtect` | 314 |
| d | `RegOpenKeyExW` の空要素 | 830 |
| e | `call through a null function pointer`（`f_125da90`） | 880 |

（担当外の壁は並行して別の担当が直す。ここでは触れていない。）

## 4. census 無効時の a4（実測。不変であること）

census を有効にしないとき（`FLE_CENSUS` 未設定）、応答を足した木のビルドで `a4-survey.sh` を動かした:

| 構成 | init_count | last unit | functions | entries | 停止 |
|---|---|---|---|---|---|
| 単独 | **145/942** | 144（proc 0x65c540） | **821** | **15,685** | `LoadLibraryW "dsp_ippv2_x64.dll"` |
| dsp（FLEngine + 縮小 dsp_ippv2） | **150/942** | 149（proc 0x664260） | **943** | **26,585** | `LoadLibraryW "blend2d.dll"` |

census の報告した基準（単独 145/942、functions 821、entries 15,685、dsp 構成 150/942、functions 943、entries 26,585）と**完全に一致**。つまり、この 8 つの応答は census 無効の a4 の進み方を変えない（8 つの API はどれも、a4 が止まる unit 144、149 より後の unit にある）。

## 5. 生成 C（実測）

| 項目 | 旧 C（HEAD の `fe-lift`） | 新 C |
|---|---|---|
| SHA-256 | `feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f` | `9d237ca0291c679d1cbdcebfb99b61539d955f937d6ad14ac4e73c24b14a35ca` |
| バイト数 | 215,709,232 | 215,711,386 |
| functions、bad_nodes | 59,297、0 | 59,297、0 |
| unsupported_total | 525 | **519**（6 つの import thunk が答えられた） |
| fle_resp_count、dispatch_count | 268、59,565 | 278、59,575 |

`diff` の結果（`lift/out-wallR/gen-diff.txt`）: 変わった箇所は **11 か所**（65 行）。(1) 10 個の関数の宣言、(2) `fle_dispatch_table` の 10 行と個数、(3) `fle_resp_table` の 10 行と個数、(4) **6 つの import thunk の 1 命令**（`fle_unsupported(UINT64_C(0x…))` → `fle_imp_*(); return;`。0x423e00、0x4240e0、0x424ae0、0x424bd0、0x593340、0x6708d0）。**この 6 つの thunk 以外に、関数本体の変化は 0**。thunk の変化は「未応答の import に応答を付ける」という変更そのもので、応答表の外の変化だが、これは避けられない（thunk が応答表の関数を呼ぶ形に変わる）。

## 6. 限界

- 各 unit で見えるのは**最初の壁だけ**（同じ unit の次の壁は上の表のとおり）。
- ok の 5 unit は、放棄した unit の後の状態で「戻った」だけ（census の限界、決定 0097）。
- 次の壁の読み（276 の 0x2003、283 の空白、880 の SHGetFolderPathW）は逆アセンブルと census の top の読みで、実行して確かめていない。
- 実機 Windows との照合は無い。native のみ（wasm は使っていない）。
