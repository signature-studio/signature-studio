# wall-census-2 の読み方と壁ごとの読み（診断。A4 の進捗ではない）

**census の数は A4 の進捗ではない**（決定 0046、0060、0097）。放棄した 19 unit（dsp 構成）は完了していない。本書は `wall-census-2.md`（`census_report.py` の出力）と `wall-census-2.tsv`（FLEngine + 縮小 dsp_ippv2）、`wall-census-2-single.tsv`（FLEngine 単独）の補足で、何も決めない。

- `wall-census-1.md`／`.tsv` は、ツールが動いた直後に先に保存した**粗い第 1 版**（分類前の生の TSV）。分類済みの版が `wall-census-2.*`（同じ測定の 2 回目以降。3 回の実行は経過時間を除いて同一）。
- 区分: **実測** = census の TSV と stdout。**読み** = 逆アセンブル・生成 C・既存の調査文書を読んだ結果で、実行して確かめていない。**未確認**はそのとおり。

## 1. 何が分かったか（実測）

| 構成 | ok | stop | fault | hang | empty | noproc | 計 |
|---|---|---|---|---|---|---|---|
| FLEngine + 縮小 dsp_ippv2（基底 0x2000000） | 876 | 19 | 0 | 0 | 46 | 1 | 942 |
| FLEngine 単独 | 873 | 22 | 0 | 0 | 46 | 1 | 942 |

- runnable 895 = ok + stop。**fault と hang は 0 件**（メモリ違反も暴走も、この 2 構成では起きなかった）。
- 壁は dsp 構成で **16 種**（a 未応答 import 6、b 1、c 4、d 5）。単独構成は 18 種（unit 144 の `LoadLibraryW "dsp_ippv2_x64.dll"` と、unit 833、935 の null 関数ポインタ呼び 1 種が加わる）。
- 2 構成で同じ unit が同じ壁になるのは 19 件。違うのは 144、833、935 の 3 unit だけで、dsp 構成ではどれも ok。
- **unit 833、935（`indirect call to 0`、top `f_10d79f0`）は、unit 144 が放棄された単独構成にだけ出る**。dsp 構成（unit 144 が完了する）では ok。したがって、この 2 件は unit 144 の失敗の結果（二次被害）である疑いが強い（実測の比較。ただし dsp の有無と unit 144 の完了が同時に変わるので、どちらが原因かは切り分けていない）。`census_report.py` のヒューリスティックも同じ 2 件に `secondary?` を付けた。
- InitUnits を通し終えた後、guest 自身の終了フェーズで `fle: unsupported instruction at 0x412752`（`f_4126f0`。`f_4128b0` ← `f_1376820`（entry））で unit の外の停止（2 構成とも同じ）。0x412752 は `jmp qword ptr [rcx]`（バイト `ff 21`。逆アセンブルで確認）。直前でレジスタを `[rcx+…]` から復元している（読み: longjmp 風の関数。実行して確かめていない）。IAT の形（`jmp [rip+rel32]`）ではないので、生成器の分類は「Jmp: indirect target unresolved」の 1 件と思われる（FLEngine の `unsupported_total 525` のうち 524 が IAT thunk、1 が unresolved。`blend2d-srw.md` §4。この 1 件がこの jmp であることは**未確認**）。
- entry `f_1376820` は `f_41d8f0`（Init）の後に**無条件で** `f_4128b0`（終了処理）を呼ぶ形（生成 C を読んだ）。つまり A4 の「DllMain → InitUnits 完走」を満たしても、entry が戻るにはこの終了フェーズも通る必要がある。0046 の A4 の定義に含まれるかどうかは、この文書は判断しない。

## 2. 壁の一覧と読み（dsp 構成。件数の多い順）

分類は census の出力。「読み」は既存の文書との照合で、推測は推測と書いた。

| 分類 | 壁 | unit | 読み |
|---|---|---|---|
| c | `LoadLibraryW "blend2d.dll"` | 149、190 | 決定 0086（Blend2D を実在モジュールにする方向）の対象。**unit 190 が 2 つ目の呼び元**（149 で止まる直列の測定では見えなかった） |
| d | `RaiseException`（例外コード 0xc06d007e、0xc06d007f） | 245、265 | 遅延 import の API が NULL を返した後の `RaiseException(0xc06d007e)` に進む形は、既存の調査 `loadlibrary-names-classification.md` が読んだもの（読み）。0xc06d007f が同じ種類の例外かどうかは**未確認**（MSVC の delay-load の例外コードという私の知識による推測で、リポジトリでは確認していない）。runtime は SEH を持たず名前付き停止 |
| c | `LoadLibraryW "iltools_x64.dll"` | 280、659 | 同梱 DLL の 1 つ。`loadlibrary-names-classification.md` が「NULL を許容するが注記つき」と読んだ名前（読み）。実在モジュールにするかどうかは PM |
| c | `LoadLibraryW "quickfontcache_x64.dll"` | 158 | 同上（決定 0086 の方向の対象） |
| a | `msacm32.dll!acmMetrics` | 170 | IAT thunk に応答なし。静的な目録（`unsupported-inventory-pdata.md`）に既出 |
| a | `shell32.dll!SHGetMalloc` | 235 | 同上 |
| c | `GetProcAddress` by ordinal（`lpProcName 0x10`） | 238 | 序数での `GetProcAddress` は runtime が未対応（`fle_modules.c` の stop） |
| a | `user32.dll!EnumDisplaySettingsW` | 246 | 目録に既出 |
| a | `user32.dll!GetClipCursor` | 272 | 目録に既出 |
| d | `SystemParametersInfoW`: `uiAction 0x30 (SPI_GETWORKAREA)` | 275 | 決定 0058（値が決まっていない）。未決 |
| d | `GetSystemMetrics`: `index 94 (SM_DIGITIZER)` | 276 | 決定 0057（値が決まっていない）。未決 |
| a | `gdi32.dll!EnumFontFamiliesExW` | 283 | 目録に既出 |
| b | `indirect call to 0x477e73 (not a known function entry)` | 296 | **既存の文書に無い壁**。0x477e73 は命令の先頭（`add rcx,-0x10`。`objdump-text.txt`）だが、関数入口の表にない。コード上の関数ポインタか間接 call の行き先が、生成器の閉包に入っていない（読み）。top `f_534b00` ← `f_534eb0` |
| d | `VirtualProtect`: address is not in a VirtualAlloc region | 314 | VirtualAlloc の領域でないアドレスへの `VirtualProtect`（メッセージ: the runtime does not track other memory） |
| d | `RegOpenKeyExW`: an empty component in a key path | 830 | キーパスの空要素（先頭・末尾・二重のバックスラッシュ）。仕様の判断 |
| a | `version.dll!GetFileVersionInfoSizeW` | 880 | 目録に既出 |
| （unit の外） | `unsupported instruction at 0x412752` | — | §1 の終了フェーズ |

## 3. 並行して直せる単位（読み）

壁は API の領域ごとに独立しているので、分類別に別々に直せる。

- **a（6 件）**: import の応答の追加。API ごとに仕様（Learn）を読んで応答するか、名前付き停止のままにするかを決める。6 つとも別の DLL／API で、互いに依存しない。
- **c（4 種 6 件）**: 実在モジュールにするか（決定 0086）、`absent` と定義するか（決定 0015、0029）、序数の `GetProcAddress`（238）をどう扱うか。PM の方針の領域。
- **d（5 種 6 件）**: 値の決定が要るもの（SPI、SM。決定 0057、0058）、`RaiseException`（SEH を持たない runtime の方針）、`VirtualProtect` と `RegOpenKeyExW` の仕様の扱い。
- **b（1 件と終了フェーズの 1 件）**: 生成器の領域（関数入口の発見、間接 jmp の解決）。

## 4. 限界（実測の範囲）

- **各 unit で見えるのは最初の壁だけ**。上の 16 種は下限で、1 つ直すと同じ unit の次の壁が出ることがある。
- 放棄した unit の後の状態は不完全（メモリ、ランタイムの表を戻していない）。後続 unit の ok は「戻った」だけ。一次／二次の印はヒューリスティックで確実でない（`wall-census-2.md` の末尾、決定 0097）。
- fault と hang は 0 件だったので、この 2 つの経路の分類は実 DLL では試されていない（模擬ゲストのテスト 9 件で確認）。
- Blend2D と quickfontcache_x64 を実在モジュールにした構成の census は未実施。
- 実 Windows での挙動は未確認。native のみ（wasm は使っていない）。
