# hidpi-fl-scaling: Issue #1347 の測定と結果(受理ではない。未確認と差し戻し)

測定値は runner の値。この機械とは別枠。nproc = 4。loadavg は測定の開始時 1.74(0.62 / 0.22)。実行の途中では取っていない。

## 判定

- 受け入れ(Issue #1347 の `accept`)は **通らない**。理由:
  - DPR 2.5 以上(DPI 240 以上)で engine-idle に届かない。guest の再帰で停止する(節 2〜4)。乱数の DPR は 1.00〜3.00 なので、この停止が出れば受け入れは通らない。
  - `flScalingPercent`・`parts`・`edgeWidth`・`debugLog` は測っていない(null / `captured: false`。節 6)。
- 実測で確かめたこと: 起動の時点から、窓の API と DPI の応答の記録が取れる(節 1・2)。FL が決める DPI の値は、DPR から式(`MulDiv(96, round(DPR × 100), 100)`)どおり(DPR 1 = 96、DPR 2.25 = 216、DPR 2.5 = 240)。
- FL64.exe の manifest は `dpiAware True/PM`(節 0)。DPI の認識の値 2 は、この宣言から決まる。

## 0. 起動用の実行ファイルのマニフェスト(実測)

`lift/out-app/host-extra/exe/` の 2 つの PE から、RT_MANIFEST(資源 24/1/1033)を `tools/feat-c1-hidpi/read-manifest.mjs`(node だけで読む道具。製品には入れない。`node tools/feat-c1-hidpi/read-manifest.mjs <file.exe>`)で取り出した。

| 項目 | `FL64.exe` | `FL64 (scaled).exe` |
|---|---|---|
| 大きさ・sha256 | 597064 B・`4c839f24…520c65` | 597064 B・`0ab91f0f…a2e979` |
| `assemblyIdentity` | ImageLine.FLStudio.FLExe 26.1.7.0 amd64 | 同じ |
| `dpiAware` | `True/PM` | (宣言なし) |
| `dpiAwareness` | (宣言なし) | (宣言なし) |
| `gdiScaling` | (宣言なし) | (宣言なし) |
| `longPathAware` | true | true |
| `requestedExecutionLevel` | (宣言なし) | asInvoker |

読み: `dpiAware=True/PM` は per-monitor DPI 対応(PROCESS_PER_MONITOR_DPI_AWARE = 2)の宣言。`FL64.exe` は DPI を自分で扱う。`FL64 (scaled).exe` は DPI 非対応の宣言が無く、Windows が画面を拡大する(ぼやける)ための起動用と読める(これは推定。実機では未確認)。

製品の応答: `GetProcessDpiAwareness` は 2 を返す(trace の行 `window-api: GetProcessDpiAwareness -> 2`)。この値は上の宣言から決める。

## 1. 記録の仕組み(実装)

- 窓の API の記録(`fle_win_api_trace_set`、C-6 / #1310)を、起動の前から有効にできるようにした。`window.__app.windowApiTrace(true)` を、ページの起動の直前(`client.start()` の前)に送る。ワーカーは起動の前の指示を覚え、wasm の `preRun` で有効にする(`app/src/main.ts` の `apiTraceWanted`、`app/src/engine/fruity.ts` の `traceOn`)。既定は無効。
- 記録する行(`window-api:` の接頭辞。既存の道具がそのまま読める): `GetDeviceCaps index=N -> V from <呼び出し元の戻り番地>`、`GetSystemMetrics index=N -> V`、`GetDpiForMonitor type=T -> V`、`GetProcessDpiAwareness -> V`。
- 戻り番地は、guest のスタックの `[rsp]`(`fle_arg` と同じ規則)。

## 2. DPI と拡大率の応答(実測。engine のログ)

| DPR | LOGPIXELSY(90) | GetDpiForMonitor | SM_CXCURSOR(13) | SM_CXSMICON(49) | 式の値(MulDiv) |
|---|---|---|---|---|---|
| 1 | 96 | 96 | 32 | 16 | DPI 96 |
| 2.25 | 216 | 216 | 72 | 36 | DPI 216 |
| 2.5 | 240 | 240 | 80 | 40 | DPI 240 |

guest は LOGPIXELSY(90)だけを読む(LOGPIXELSX(88)は呼ばれない)。SM の値は `MulDiv(32, DPI, 96)` と一致する(72 = 32 × 216 / 96)。

`flScalingPercent`(FL が採用した拡大率)は測っていない。FL の UI の部品の大きさを、この機械の出力から取れていない(節 6)。

## 3. 停止の切り分け(実測)

DPR 2.5 の起動。切り替えは `.tmp-exp` の一時的な実験(製品には残していない)。各 start-up の全文は `.tmp-exp/log-*.txt`(commit しない)。

| 実験 | 切り替え | 結果 | engine-idle |
|---|---|---|---|
| 基準(製品の差分) | なし | 60 秒で届かない。`[js] fle thread 2 failed: RangeError: Maximum call stack size exceeded` | 届かない |
| (i) フォントの高さだけ DPI 96 | `FLE_EXP_FONT96=1` | 届かない | 届かない |
| (ii) GetDeviceCaps の LOGPIXELS だけ 96 | `FLE_EXP_LOGPIX96=1` | 届いた(idle 4 回・9.2 秒) | **届く** |
| (iii) GetDpiForMonitor だけ 96 | `FLE_EXP_MON96=1` | 届かない | 届かない |
| 仮説: 物理サイズを機器の密度(96 × DPR)から求める | `FLE_EXP_MMDPI=1` | DPR 2.5 も DPR 3.0 も届かない | 届かない |
| 基準 DPR 2.25(DPI 216) | なし | 届いた(idle 1 回・11.1 秒) | 届く |
| 基準 DPR 1(DPI 96) | なし | 届いた(idle 4 回・9.4 秒) | 届く |

読み(実測):
- 停止の原因は、guest が **LOGPIXELSY の値 240** を受け取ること。フォントの高さも GetDpiForMonitor も、単独では原因ではない。216 は通る。しきい値は 216 と 240 の間。
- 物理サイズの矛盾(96 DPI 固定の mm と、LOGPIXELS の 240 の食い違い)は原因ではない(仮説の実験で否定)。
- 停止は窓の作成の直後: `CreateWindowExW 0x…24 ex=0x80 style=0x80000000 parent=0`(隠れた `TPUtilWindow` 相当)の後、描画(`UpdateLayeredWindowIndirect`)より前。DPR 1 では、同じ位置の後に描画が進む。
- 窓の再配置の反復(`SetWindowPos`)は、停止までに無い。

## 4. 停止の位置の絞り込み(実測と推定)

- guest の LOGPIXELSY の読み出し元(trace の `from`): 5 か所。戻り番地が含まれる関数は、生成物のソースの定義から `f_<番地>` の名前で引ける(`lift/out-app/gen/src/`)。書き込みは `f_613df40`(戻り番地 `0x613df66`)で、値を guest の大域 `0x630e924` に置く。
- この大域を読むのは 7 か所。読み手の関数: `f_6135c50`・`f_6135e10`・`f_61365b0`・`f_6136750`・`f_6136d10`・`f_613e020`(font の高さを DPI から求める所。MulDiv 相当)。どれも再帰はしない(直接の呼び出しを辿ると、読み手から先は 97 関数で、閉路は無い)。
- 停止時のスタック(診断。`fle_thread_lib.js` の一時的な表示): 停止の直前の窓の作成の rsp から、下へ 32 MB を読むと、戻り番地らしい値の並びが **12 個の周期で繰り返す**(`410002 518ac7 725b9f 51b687 51b687 519acc 410e00 5199f5 5193db 5273b2 4d78a8 519707`)。`51b687` が 2 回続くので、ある関数の自己呼び出しを含む。
- ただし、この番地の多くは生成物の中に命令の位置として見つからない(`/* 0x<番地> ` の検索)。よって、関数の名前への対応は確定していない。閉路は直接の `call` ではなく、仮想呼び出し(関数ポインタ)を経由すると推定する(静的な呼び出しの辿りでは閉路が見えないため)。
- 結論(推定): LOGPIXELSY = 240 のとき、guest の font・寸法の計算の経路が、間接呼び出しを経由する閉路に入り、無限に再帰する。閉路の入口と、どの runtime の応答がそこへ導くかは、この作業では特定できていない。

## 5. 一時的な診断の扱い

(この節の実験の全文 log は commit していない。`.tmp-exp/` は作業の後に削除する。上の表の行は、その log から抜いた行。)

- 診断(`fle_g3_rsp_mark`・戻り番地の表示)と、実験の切り替え(`FLE_EXP_*`・`__fleExpEnv`)は、commit の前に外した(`grep -rn 'TEMP-EXP\|FLE_EXP_\|fle diag' lift app tools` は 0 件)。
- 窓の API の記録の行(節 1)は残した(既定では出ない。測定の道具だけが有効にする)。

## 6. 未測定(Issue の (a)〜(j))

| 項目 | 状態 | 理由 |
|---|---|---|
| (a) DPI・画面の API の記録 | 一部 | 節 1・2。RegQueryValueExW・SystemParametersInfoW の記録は未 |
| (b) seed | 確認済み | `lift/harness/seeds/reg-seed-app.tsv` と `reg-seed-nsis.tsv` は、HKCU の Shared Paths と、インストーラが書く値だけ。GUI の拡大の値は無い。数値の拡大率は seed に無いので、直す対象は無い |
| (c) F10 の Display の節 | 未 | 窓の操作を自動化していない |
| (d) FL の採用した拡大率 | 未 | UI の部品の大きさを、この道具では取れていない |
| (e) Windows の計算値との突き合わせ | 一部 | LOGPIXELSY・GetDpiForMonitor・SM の値は式どおり(節 2)。FL の採用値は未 |
| (g)〜(i) Debug ログ | 未 | ログの文字を窓の部品から読む経路を作っていない |
| (j) debugLog | captured false | 同上 |
| FL の Main GUI Scaling の読み | 未 | FL が設定を読む経路(レジストリ・設定ファイル)が trace に出ていない |
| parts・edgeWidth | 未 | トップバーの部品は窓の表(`fle_desk_describe`)に出ない(最上位の窓だけ)。FL が主窓の面に描く |

## 7. 最終の測定(commit する状態。`measure-size.mjs` の出力そのまま)

測定の道具の出力 7 行(`randomDprs`: seed 3567857356、values [1.77, 2.71])。

| DPR | 状態 | guest(w×h) | round(部屋×DPR) | dpi(LOGPIXELSY) | round(96×DPR) | GetSystemMetrics 13(カーソル)/ 49(小アイコン) | canvas / display | blackShare |
|---|---|---|---|---|---|---|---|---|
| 1 | engine-idle | 1836×1056 | 1836×1056 | 96 | 96 | 32 / 16 | 1836×1056 / 1836×1056 | 0.910 |
| 1.25 | engine-idle | 2295×1320 | 2295×1320 | 120 | 120 | 40 / 20 | 2295×1320 / 2295×1320 | 0.912 |
| 1.5 | engine-idle | 2754×1584 | 2754×1584 | 144 | 144 | 48 / 24 | 2754×1584 / 2754×1584 | 0.912 |
| 1.7647 | engine-idle | 3240×1864 | 3240×1863 | 169 | 169 | 56 / 28 | 3240×1864 / 3240×1864 | 0.905 |
| 2 | engine-idle | 3672×2112 | 3672×2112 | 192 | 192 | 64 / 32 | 3672×2112 / 3672×2112 | 0.914 |
| 1.77(乱数) | engine-idle | 3250×1869 | 3250×1869 | 170 | 170 | 57 / 28 | 3250×1869 / 3250×1869 | 0.903 |
| 2.71(乱数) | **停止**(idle に届かない) | — | — | — | 260 | — | — | — |

- 停止の行: `error`: `state running before the first engine-idle` ・ 最後のログ行 `[js] fle thread 2 failed: RangeError: Maximum call stack size exceeded`。窓の作成 `CreateWindowExW 0x…24` の直後(節 3)。DPI 260 は、既に確かめた DPI 240 の停止と同じ経路。
- `flScalingPercent`・`mainGuiScaling`・`parts`・`edgeWidth`・`debugLog` は null / false(節 6)。
- `blackShare` は DPR 1 で 0.910(閾値 0.5 を超える)。Browser の矩形(CSS の (0, 580) から 400×470)が、この版の DPR 1 でも大部分が黒い、または矩形が Browser の一覧の外にある。どちらかは未分析。

受け入れの判定(Issue の `accept` の node 部分を、この出力に当てた結果): `false`(上の null と停止と blackShare による)。源コードの検査(追加行に特定の拡大率・DPI・DPR の値が無いこと): 一致 0 件。
単体テスト(`npm test --prefix app`): 68 件のファイル、1333 件が通る。lift の `gdc`・`screen1`・`wall_s`: 合計 38 件が通る。

## 8. 次にやること

1. 閉路の入口を特定する: 停止の時点の guest のスタックを、関数の名前に結ぶ(生成物の命令の位置を、wasm の関数番号と突き合わせる、または guest の関数の名前の表を作る)。
2. 閉路を起こす runtime の応答を、Windows の値と突き合わせる(LOGPIXELSY 以外の、DPI に依る応答の全部)。
3. 修正の後、DPR 1〜3.0 の乱数で engine-idle に届くことを確かめる。
4. 部品の位置・縁の遷移・拡大率・Debug ログを、測定の道具に入れる。
