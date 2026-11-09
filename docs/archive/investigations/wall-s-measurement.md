# wall-s の測定記録（census の前後。診断。A4 の進捗ではない）

決定 0099（census の壁バッチ 2）の実装の前後で census を実行した記録。**census の数は A4 の進捗ではない**（決定 0046、0060、0097）。放棄した unit は完了していない。

凡例: **実測** = 実行した出力。**読み** = 逆アセンブル・生成 C を読んだ結果で、実行して確かめていない部分。**未確認** = 確認していない。

## 1. 条件

| 項目 | 内容 |
|---|---|
| 基準の木 | `git archive` の HEAD 5002d98 + `lift/out-census/patches/census.diff`（`lift/out-wallS/base`）。編集は `src`（複製）だけ。パッチは `patches/wallS.diff`（`diff -u -r -N base src`）。2026-10-27 に HEAD が f1bcba7 まで進んだので、f1bcba7 の `git archive` + census.diff に `patch -p1 --dry-run` で当たることを確認した（`logs/check-head-wallS.txt`） |
| 前（before） | `lift/out-census/runs/dsp-raw-1.tsv`、`single-raw-1.tsv`（census 3 回の 1 回目。3 回は経過時間を除いて同一）と `wall-census-2.*`。**census の木のビルド（生成 C の SHA `feb5b908…`）**。同じバイナリ `out-census/two/v-dsp/native/two-native`（読み取りだけ）を今回も実行した: census 無効で 2 回、entries 26,585（基準どおり）。census 有効で gdb の下で 1 回（`runs/gdb-abort.out`）、unit 238、245、265、314、830 の停止メッセージが `wall-census-2.tsv` と同じ。unit ごとの入口数の比較は、census 3 回のうちの 1 回目の生の TSV（`dsp-raw-1.tsv`）を使った（バイナリの再実行ではない） |
| 後（after） | `lift/out-wallS/two/v-dsp/native/two-native`（FLEngine + 縮小 dsp_ippv2 基底 0x2000000、種 452 個）、`lift/out-wallS/single/native/flengine-native`。生成 C の SHA `62146e10…`。同じ手順（`gen_two.sh`、`build_two_native.sh`、`build_native.sh`、`FLE_CENSUS=ファイル`、`census_report.py`） |
| 実行 | dsp: `runs/after1/`、`runs/after2/`（2 回。経過時間の列を除いて同一）。単独: `runs/after-single/`。報告: `runs/after-report.md`、`after-report-dsp.tsv`、`after-report-single.tsv`（`census_report.py` の出力） |

## 2. 壁ごとの前後（dsp 構成。実測）

| unit | 壁 | 前（状態、入口数） | 後（状態、入口数） | 後の最初の壁（同じ unit の**次の壁**） |
|---|---|---|---|---|
| 238 | shell32 の序数 `GetProcAddress`（W3） | stop、5 | **ok、131** | なし |
| 245 | 遅延 import `Shcore.dll!GetProcessDpiAwareness` → `RaiseException(0xc06d007e)`（W1） | stop、135 | stop、556 | `LoadLibraryW "quickfontcache_x64.dll"`（`f_423990` ← `f_44db40` ← `f_6539e0`、ret 0x44db70）。**遅延モジュール（決定 0096）の領域**。分類 c |
| 265 | 遅延 import `kernel32!GetSystemDefaultLocaleName` → `RaiseException(0xc06d007f)`（W2） | stop、248 | **ok、256** | なし |
| 314 | `VirtualProtect` がイメージの `.text`（W4。決定しない） | stop、787 | stop、787 | 変化なし（メッセージも同じ） |
| 830 | `RegOpenKeyExW` の末尾の `\`（W5） | stop、257 | stop、340 | `fle: indirect call to 0 is not a known function entry`（`f_10c7cd0` ← `f_10d50d0` ← `f_4120c0`、ret 0x10d5320）。下の 2.1 |

他に変わった unit（実測）: 145（ok、入口数 422 → 440。`shcore.dll` が仮想モジュールになり、unit 145 自身の `LoadLibraryW("SHCore.dll")` + `GetProcAddress("GetDpiForMonitor")` が成功の経路を通る。決定 0099 W1 の「副作用」）、239（ok、668 → 710。unit 238 が戻るようになった結果）。上の 7 unit 以外の 935 unit は、状態も入口数も同じ。

内訳（dsp。census_report の出力）:

| | ok | stop | fault | hang | empty | noproc | 計 |
|---|---|---|---|---|---|---|---|
| 前 | 876 | 19 | 0 | 0 | 46 | 1 | 942 |
| 後 | **878** | **17** | 0 | 0 | 46 | 1 | 942 |

単独構成: ok 873 → **875**、stop 22 → **20**（変わった unit は dsp 構成と同じ）。壁の種類（dsp）: 16 → **14**（`RaiseException`（2 unit で 1 種）、`GetProcAddress by ordinal`、`RegOpenKeyExW` の 3 種が消え、「null 関数ポインタ呼び」（unit 830）が 1 種加わった。`LoadLibraryW "quickfontcache_x64.dll"` は 1 → 2 unit（158、245））。InitUnits を通し終えた後の unit の外の停止（`unsupported instruction at 0x412752`）は前と同じ。

### 2.1 unit 830 の次の壁（実測。gdb）

`f_10c7cd0` の `0x10c8941`（`call [rbp+0x860]`）が 0 を呼ぶ。0 は `GetProcAddress(LoadLibraryW(L"SHFolder.dll"), "SHGetFolderPathW")` の戻り値（`0x10c8902` の `LoadLibraryW` → `0x10c891c` の `f_4235f0`（GetProcAddress の Delphi ラッパー。ANSI 名に直して `GetProcAddress` を呼ぶ）。gdb で `fle_imp_GetProcAddress` の `hModule = 0x7ff800110000`（`fle_modules[16]` = `shfolder.dll`）、名前 `SHGetFolderPathW` を確認）。**応答表に `shfolder.dll!SHGetFolderPathW` の行が無い**ので NULL と 127 になり、ゲストは NULL を検査せずに `(0, CSIDL 0x1a = APPDATA, NULL, 0, buf)` で呼ぶ（読み）。census は「e: null 呼び、二次の疑い」に分類したが、**一次**（未応答の API。レーン R の領域）。`SHGetFolderPathW` は FLEngine の通常の import（IAT thunk）でもあり、静的な目録（`unsupported-inventory-pdata.md`）に既出。

## 3. census 無効の a4（実測）

| 構成 | init_count | last unit | functions | entries | 停止 | 基準 |
|---|---|---|---|---|---|---|
| 単独 | **145/942** | 144 | **821** | **15,685** | `LoadLibraryW "dsp_ippv2_x64.dll"` | 一致（`a4-survey.sh`。`runs/survey-single-off.txt`） |
| FLEngine + 縮小 dsp_ippv2 | **150/942** | 149 | **943** | **26,603**（基準 26,585、+18） | `LoadLibraryW "blend2d.dll"` | init_count、last unit、functions、停止は一致。**entries だけ +18**（unit 145 の `SHCore.dll` の成功経路。上の 2 の「他に変わった unit」。最初の 256 エントリのログは一致。`runs/survey-two-off.txt`、`runs/loader-old.txt` と `loader-new.txt` の差は最後の `GetProcAddress(shcore, "GetDpiForMonitor")` 1 行） |

## 4. 参考実験: W4 を (B) にしたら（パッチに含めない）

`exp/src/fle_vmap_exp.c`（`fle_vmap.c` の複製。`VirtualProtect` が `[0x400000, 0x400000 + 0x18c0000)` に収まる範囲なら旧保護 `0x20` を書いて 1 を返す。何も記録しない）を、`src` の他のオブジェクトと再リンクした `exp/two-native-exp` で、dsp 構成の census を実行した（`runs/exp/`）。

| 項目 | 結果（実測） |
|---|---|
| census の内訳 | ok **879**、stop **16**（after: 878／17）。変わった unit は 314 だけ |
| unit 314 | **ok、入口数 890**（次の壁なし） |
| 有効化 | gdb で `f_8e0ef0`（フックの有効化）が unit 314 の中で 1 回呼ばれる。census の最後（終了フェーズの停止点）で、`0x41c820`、`0x43b130`、`0x43b150` の先頭が `e9 5b e5 4b 00`、`e9 3b fb 49 00`、`e9 8b fb 49 00`（それぞれ `0x8dad80`、`0x8dac70`、`0x8dace0` への `jmp`） |
| フック対象の呼び出し | 有効化の前に `f_41c820` が 63 回。有効化の後は、`f_41c820`／`f_43b130`／`f_43b150` とも 0 回（この census の範囲。放棄した unit が多いので下限） |
| 解釈 | リフトされた C は `.text` のバイトを実行しないので、書かれた `jmp` は効かない。census の範囲ではフックの対象が有効化の後に呼ばれないので、結果は変わらないが、**フックが効かないこと自体はこの測定では見えない**（読み） |

## 5. 遅延 import の同じ種類の壁（実測の表: `investigate/delay-callers.txt`）

FLEngine の遅延 import は 17 記述子・57 関数（`investigate/delayimp-flengine.txt`）。応答表に行があったのは 3 件（`GetLogicalProcessorInformation`、`IsWow64Process`、`GetNativeSystemInfo`）。この決定で 2 件（`GetSystemDefaultLocaleName`、`GetProcessDpiAwareness`）が加わり、**52 件が残る**。静的な直接 `call` がある関数は 46 件、無い関数が 6 件（propsys の 5 件と ole32 の `PropVariantClear`）。`call` の数は `.text` の線形逆アセンブルから数えた下限（間接 call、vtable 経由は数えていない）。表は `investigate/delay-callers.py` の出力（`delay-callers.txt` の `callsites` 列）から機械的に作った:

| DLL（遅延 import 記述子の名前、小文字） | 関数 | 応答済み（本決定の 2 件を含む） | 残り（call あり） | 残り（call 無し） | 呼ばれたときの例外 |
|---|---|---|---|---|---|
| kernel32.dll | 10 | 4 | 6 | 0 | 0xc06d007f |
| user32.dll | 15 | 0 | 15 | 0 | 0xc06d007f |
| wtsapi32.dll | 2 | 0 | 2 | 0 | 0xc06d007e（absent） |
| msimg32.dll | 2 | 0 | 2 | 0 | 0xc06d007e（absent） |
| advapi32.dll | 1 | 0 | 1 | 0 | 0xc06d007f |
| rpcrt4.dll | 1 | 0 | 1 | 0 | 0xc06d007e（absent） |
| propsys.dll | 5 | 0 | 0 | 5 | 0xc06d007e（absent） |
| ole32.dll | 1 | 0 | 0 | 1 | 0xc06d007f |
| shell32.dll | 3 | 0 | 3 | 0 | 0xc06d007f |
| windowscodecs.dll | 1 | 0 | 1 | 0 | 0xc06d007e（absent） |
| uxtheme.dll | 5 | 0 | 5 | 0 | 0xc06d007e（absent） |
| imm32.dll | 7 | 0 | 7 | 0 | 0xc06d007e（absent） |
| dwmapi.dll | 2 | 0 | 2 | 0 | 0xc06d007e（absent） |
| shcore.dll | 2 | 1 | 1 | 0 | 0xc06d007f（`GetDpiForMonitor`） |
| 合計 | 57 | 5 | 46 | 6 | |

（kernel32 の残り 6 件: `NormalizeString`、`GetFinalPathNameByHandleW`、`CreateSymbolicLinkW`、`LocaleNameToLCID`、`GetUserPreferredUILanguages`、`GetTimeZoneInformationForYear`。0xc06d007e になるのは absent 8 名の 25 関数、0xc06d007f になるのは残りの 27 関数。）

## 6. 実行時間とマシンの状態

dsp 構成の census 1 回は数秒〜十数秒。生成（`gen_two.sh`）と構築（`build_two_native.sh`、`build_native.sh`）は各数分。全体テストは約 1 時間（分割した 5 回の合計）。この作業の途中で `/` が 100% になった（他の作業の書き込み。この作業の成果物は 8 GB）。ディスクが満杯の間は、シェルの出力が空になり、テストが `No space left on device` で失敗した（決定 0099 の「検証」）。
