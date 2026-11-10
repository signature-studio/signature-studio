# 0201: `?dedupe`(同一矩形の省略)を既定 ON にする条件の検証 — (a) 2D canvas の contextlost/contextrestored の後の再同期を実装、(b) Save 系ダイアログの区間と (c) 長時間で `?dedupe=check` の差分 0、既定 ON の差分 `dd1-default.diff`。遅延の改善は『ビーコン ON のページでは出ない』

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0201、決定 0187 の『既定 ON の条件』)。実装・測定・文書は作業者 dd1(2026-11-04 00:10〜)。作業は HEAD **26eb04d** の `git archive`(`lift/out-dd1/src/`。作業中に HEAD が 7f024d7 に進んだが `app/` は同じ)。`dd1.diff` は `app/` の 11 ファイルだけ、`dd1-default.diff` は `app/` の 5 ファイルだけを変える(`lift/`・wasm・生成 C・runtime・既定の入口・`lift/out-websrv/` は変えていない)。成果物は `lift/out-git/dd1-results/`。

区分の凡例は決定 0152 と同じ。**実測**＝このセッションのコマンドの出力を見た事実。**読み**＝解釈で、確かめていない。**未確認**。**出典なしの選択**＝`S-dd1-…`(§8)。環境は『この機械』= 16 コアの Linux VM、隔離 headless Chrome 156(SwiftShader)、load 7〜45(他担当と共有)。**依頼者の機械ではない。実機 Windows との照合は 0 件。入力はすべて CDP の模擬で、人の操作ではない。**

## 冒頭の三点(正直さ)

1. **何を実測したか(実行した本物と範囲)**: 実エンジン(`FLEngine_x64.dll` と同梱 DLL を x86-64 → C → wasm にした物)を隔離 headless Chrome 156 で動かし、`?dedupe=check`(shadow が『不変』と判定した矩形だけ参照 canvas に描かず、実 canvas と全窓・全画素を比べる自己検証)で、(c) **配信中の既定の入口 SHIPH**(パラメータ無しと同じ内容)の 5 セッション・合計 62.1 分(うち 1 本は 29.6 分)・2,437 回の比較、(b) **テストバンドル F1xH + ia2**(自分でビルド。混成なし)の Save as/Save/Save new version/Ctrl+S 系のダイアログと『Buy FL Studio』窓を含む 4 セッション・677 回の比較、を取った。(a) の実 Chrome の GPU クラッシュ(`Browser.crashGpuProcess`)と、canvas を空にして `contextlost`/`contextrestored` を dispatch する模擬を、stand-in と実エンジンで。遅延(§6)は lat1 の測定器で、既定の入口と `?dedupe=0`、さらに `beacon=0` の対。**代役・モック・画像の再生・事前生成は使っていない**(stand-in の e2e 場面は『stand-in の STAND-IN の帯つき』で、進捗には数えない)。
2. **結論**: (a) **再同期を実装しテストした**(contextlost/restored のどちらでもページが Worker に通知 → 全窓を全面送信 + shadow を消去。単体 +22、e2e +54 チェック〔全体 1071/1071〕、変異 9/9)。(b) **Save as/Save/Save new version などのダイアログの区間で差分 0**(テストバンドル、677 回の比較)。(c) **長時間(再生 300 s 超・窓の開閉・F8/F10・ダイアログ・右クリックメニュー)で差分 0**(有効 5 セッションの合計 62.1 分、2,437 回)。→ 条件 (a)(b)(c) を上の範囲で満たしたので、**既定 ON の差分 `dd1-default.diff` を別ファイルに作った**(`?dedupe=0` で OFF。vitest 1225、e2e 1071/1071、変異 5/5)。**ただし 既定 ON の『遅延が下がる』根拠は、既定のページ(ビーコン ON)では再現しなかった**: 既定 ON と `?dedupe=0` の event→次の rAF の中央値は同じ範囲(§6.2)。PM の指摘(cbeacon2)どおり、ビーコンが待機中も rAF を回してページがフレームを作り続けると利点が消える。`?beacon=0` の対(各 2 組、n=24 前後)では、クリック 24.2 → 7.1、ホバー 30.2 → 15.2、ホイール 46.4 → 30.2 ms などで**決定 0194 の改善が再現した**(メニューを閉じる・Space の停止は差なし/逆。§6.3)。**既定 ON の根拠として確実に言えるのは『画素が同一(差分 0)』と『ページ主スレッド CPU の減少』(アイドル: ビーコン ON で 0.113〜0.158 → 0.035〜0.056 コア、ビーコン OFF で 0.168〜0.169 → 0.020〜0.023 コア。CPU 秒、全 7 セル・2 組で同じ向き)**。遅延の利点はビーコン OFF でだけ出るので、ビーコン ON の既定のページでは cbeacon2 の修正後に再確認が要る。
3. **未確認**: (i) **実 Chrome での『復元後の画素』**(この Chrome は GPU クラッシュの後 `contextrestored` を出さず、元の canvas は戻らない。復元後は合成イベント + 空の canvas の模擬だけ)。復元されない canvas の差し替えは未実装。(ii) 実機の GPU・ディスプレイ・120 Hz 以上、依頼者の機械、所定のプロジェクト。(iii) `?dedupe=check` の比較は 0.7〜1.5 s 間隔(フレームの間だけ現れて消える差は見えない)、方式 B のみ。(iv) テストバンドルは SHIPH と同一の構成ではない(混成なし、`ia2-imports.txt` は exp1 の 6 行を含まない)。出荷の SHIPH には ia2 の行がないので、SHIPH で File>Save 系を押すと GetKeyNameTextW で停止する(未修正)。(v) File>Open のダイアログ区間(ブラウザのファイル橋のない構成)、Save as template を閉じる操作(`SendMessageA` の壁で停止)。(vi) ビーコン ON のページの遅延(cbeacon2 の修正後に再確認)。(vii) `LatencyTracker`/ビーコンの数字への既定 ON の影響(読み)。

## 1. 方法

- **ワークツリー**: HEAD 26eb04d の `git archive`(`lift/out-dd1/src/`)。作業中に HEAD が 7f024d7(lat1 の取り込み)まで進んだが、`app/`・`lift/runtime`・`lift/crates` は 26eb04d と同じ(`git diff --stat 26eb04d HEAD -- app lift/runtime lift/crates` が空)。`dd1.diff` は最後に最新の `git archive HEAD` に `patch -p1 --dry-run` が当たることを確認した(`logs/patch-dry-run.txt`)。テストバンドルの `bundle.description` が `HEAD 7f024d7` と書くのは、`build-engine-bundle.sh` が親リポジトリの HEAD を読むため(木の中身は 26eb04d の archive)。
- **テストバンドル(b 用)**: `app/scripts/build-engine-bundle.sh` を `SHIP_PRESET=F1xH`(= 出荷の SHIPH の C 経路:F1x〔prune S + Blend2D 置換〕+ 層 1〜3)で、`EXTRA_IMPORTS=lift/harness/seeds/ia2-imports.txt`(139 行 = `dlg1-imports.txt`〔層 1〜3〕+ ia2 の層 4 の行〔`CompareStringW::ia2-sort-weights` の marker 行を含む〕。**exp1 の層 4 の行のうち `SetThreadExecutionState` と `IFileSaveDialog::*` 5 行は `ia2-imports.txt` に入っていない**〔依頼文は『dlg1 + exp1 + ia2』と書いたが、ファイルは ia2 の MERGE-NOTES どおり dlg1 + ia2 だけ〕。= ブラウザの保存ダイアログの COM モデルは入っていない)に替えて作った(`tools/bb.sh`)。**混成 p1000r は入っていない**(SHIPH ではなく F1xH + ia2 の構成)。`lift/out-dd1/B/bundle/fl-engine-dd1-F1xH-ia2/`(wasm 177,182,569 B、SHIPH は 171,302,606 B、+5.9 MB。ia2felift の SSE3 命令と ia2 の runtime)。圧縮は gzip(ローカルの vite preview 用)。**配信中の `lift/out-websrv/assets/` には置かず、触っていない**。上流 Blend2D の wasm ライブラリ(-O2、SIMD なし)は `out-b2d1/up6` の tarball からオフラインで作り直した(出来た `libblend2d.a` の md5 は `out-b2d1/up5・up6` の物と同じ)。ビルド: cargo 19 s、gen 112 s、wasm の単位コンパイル 1,672 s(他担当の負荷の中)、wasm-opt 588 s。
- **実エンジンの走行**: 隔離 headless Chrome 156(`app/test/e2e/browser.mjs`、プロファイル使い捨て)、自前の `vite preview`(ポート 4610〜、`--outDir` はこの木の `vite build`)。(c) は**配信中の既定の入口と同じ内容**(`lift/out-websrv/assets/manifest.json` → SHIPH を、`lift/out-dd1/serve/assets/` の読み取り専用の symlink で)、URL は `?pace=catchup&dedupe=check` だけ(`pace=catchup` は idle1 の check-long と同じ)。(b) はテストバンドルを `?bundle=` で指す。入力はすべて **CDP の模擬**(`Input.dispatchMouseEvent`/`dispatchKeyEvent`)で人の操作ではない。ツール: `tools/dd-check.mjs`(idle1 の `check-long.mjs` の発展)。
- **自己検証 `?dedupe=check`**(決定 0187 §6): 方式 B で、全矩形を従来どおり送って描き、shadow が『不変』と判定した矩形だけ**参照 canvas** に描かない。`window.__app.dedupeCheck()` が窓ごとに canvas と参照 canvas の違う画素数を返す。(c) は 1.5 s ごと、(b) は 0.7 s ごとに**全窓・全画素**を比べる。差が出たら最初の差の窓・画素・外接矩形・crop と直前 40 件の入力を `first-diff-*.json` に保存する(`dedupeCheck()` に足した)。1 セッションのメニューは 28 個まで(runtime の hook 表の累計 64 個の限界、決定 0194)。
- **限界**: 比較は『ある時点の canvas の状態』(フレームの間だけ現れて消える差は見えない)。方式 B のみ。この機械は 16 コアの SwiftShader で load 7〜45(他担当と共有)。依頼者の機械ではない。実機 Windows との照合は 0 件。

## 2. (a) Chrome の 2D canvas の contextlost / contextrestored の後の再同期(app/ のみ、`dd1.diff`)

**問題**: 2D canvas がコンテキストを失う(GPU プロセスのクラッシュ・リセット)と、canvas は描けなくなり、戻ったときは**空**。`?dedupe=1` の shadow は『ページにはこれが出ている』と信じたままなので、エンジンが同じ画素を書き直しても省略し続け、**空の canvas が戻らない**。(決定 0187 §8 は『従来はアイドルの毎フレーム再描画で自然に直っていた』と書いたが、再描画は主窓の 1 画素幅だけなので、dedupe なしでも失われた canvas の大部分は戻らない。= dedupe の有無に関わらず直す価値がある。)

**実装**(`app/src/engine/`):

| 場所 | 内容 |
|---|---|
| `ctx-resync.ts`(新) | `watchContextLoss(target, {lost, restored})`: `contextlost` を **`preventDefault()`**(HTML の規則は WebGL と同じ: 取り消すと復元が許される。取り消さない場合の動きは未テスト)して `lost()`、`contextrestored` で `restored()`。HTMLCanvasElement と OffscreenCanvas の両方に使える |
| `client.ts` | 方式 B(実エンジンの既定): ページの各 canvas(`?dedupe=check` では参照 canvas も)を監視。どちらのイベントでも LayerPainter のステージング canvas を作り直し、Worker に `{cmd:'resync', why}` を送り、ページのログに warn を出す |
| `worker.ts` | 方式 A: OffscreenCanvas は Worker 内なので `doBind` でリスナーを付ける。`{cmd:'resync'}` と合わせて `resync(why)`: `SurfaceSync.resync()` + 全シンクの LayerPainter を作り直し + ログ + その場で `pumpAll()` |
| `surface-sync.ts` / `surface-table.ts` | `SurfaceSync.resync()` = `Presenter.forgetSeen()`(`seen` を消す = 次の pump で全シンクが `Sink.resize`〔canvas の消去と dedupe shadow の `resetShadow`〕+ 窓全体の矩形)+ table stamp の無効化(変化がなくても次の look が完全になる) |
| `protocol.ts` | `ToWorker` に `{ cmd: 'resync'; why: string }` |
| `client.ts` の `dedupeCheck()` | (c) の調査用に、差のある窓に『最初の差の画素・外接矩形・canvas と参照 canvas の 256×256 以内の PNG crop』を足した(差がなければ従来と同じ) |

**テスト**:
- 単体 `ctx-resync.test.ts` 4 件(`preventDefault` の有無、EngineClient の各 canvas 監視と `resync` 送信とログ、`Presenter.forgetSeen`、`SurfaceSync.resync` が stamp の skip を破る)。`dedupe-presenter.test.ts` +18(方式 A/B × 〔陰性対照 1 + ランダム 1,500 手 8 シード〕: contextlost 中の描画は無視され、復元で canvas が空になっても、resync の後は canvas == エンジンの表面。陰性対照: resync なしでは shadow が『省略』と答えて canvas は空のまま)。
- e2e(stand-in、`dedupe-standin-contextloss-B`〔dedupe あり〕/`-B-nodedupe`、`dedupe-standin-gpucrash-B/-A`、`dedupe-standin-check-detects`): 結果は §2.2。
- 変異検査 9 件(`test/mutation/run.mjs`): contextlost を取り消さない、contextrestored を無視、forgetSeen が忘れない、stamp を無効化しない、client が監視しない、client が送らない、Worker がメッセージを無視、Worker が pump しない、`dedupeCheck` の最初の画素が最後になる。

### 2.1 実測した結果(テスト)

| 項目 | 結果 |
|---|---|
| `npm run typecheck` | 通過(`logs/dd1-typecheck.txt`) |
| `npx vitest run`(`dd1.diff` の木) | **1224 passed + 7 skipped**(基準 HEAD: 1209 のうち 7 skipped → +22 件〔`ctx-resync.test.ts` 4、`dedupe-presenter.test.ts` +18〕。`logs/dd1-vitest.txt`) |
| `npm run test:worker` | 43/43 |
| `npm run test:e2e`(dev・preview・worker の 3 target) | **1071/1071 checks passed**(基準 1017 + 54 = 新しい 3 場面 × 3 target × 6 チェック ほか。`logs/e2e-full-dd1.txt.gz`、新規分だけは `tables/e2e-new-checks.txt`) |
| 変異検査(`test/mutation/run.mjs` に 9 件追加) | **9/9 検出**(contextlost を取り消さない、contextrestored を無視、forgetSeen が忘れない、stamp を無効化しない、client が監視しない、client が送らない、Worker がメッセージを無視、Worker が pump しない〔`logs/dd1-mutation-ctx-resync.txt`: 8/8〕、`dedupeCheck` の最初の画素が最後になる〔`logs/dd1-mutation-extra.txt`: 1/1〕) |

### 2.2 実 Chrome の context loss(隔離 headless Chrome 156、`--disable-gpu` = SwiftShader、この機械)

CDP の `Browser.crashGpuProcess` を使った(`tools/ctxloss-probe*.mjs`、`logs/`)。

- **`contextlost` は届く**: ページの HTMLCanvasElement(`getContext('2d')` の既定の canvas も `willReadFrequently:true` の canvas も)と、Worker に移した OffscreenCanvas の両方に。`preventDefault()` しても同じ。
- **`contextrestored` は来なかった**(最長 約 19 s 待った。探索の段階で probe を 10 回ほど回し〔最初の 10 回 poll の版は 20 s 待った。そのログは保存していない〕、最後に 3 本を再実行して `logs/chrome-contextloss-probes.txt` に保存した。`isContextLost()` は true のまま。同じ canvas に `getContext('2d')` し直しても同じ context で lost のまま、`canvas.width = canvas.width` でも直らない。**新しく作った canvas は使える**)。= この Chrome・この環境では、GPU クラッシュの後の元の canvas は戻らない。復元されたときの動き(contextrestored の後の画素)は**実 Chrome では検証できていない**(読み: Chromium は取り消された contextlost の後に数回だけ復元を試みて諦める。未確認)。
- 実エンジンでの実 loss(`gpucrash` 走行、`runs/c-gpucrash/`): `crashGpuProcess` の後、ページ canvas(`l8`・`l14`・`l48` とその参照 canvas)で『2D context lost; the Worker is asked to send every window whole again』がログに出て、Worker が『every window is sent whole again (the dedupe shadows are cleared)』を返し、**例外なし**、エンジンは動き続け(その後 Edit メニューを開いて idle 10 s)。canvas は 10 s 後も `isContextLost() == true`(上と同じ)。
- 実エンジンでの**模擬の loss**(`ctxloss` 走行〔canvas を空にして `contextlost`/`contextrestored` を dispatch〕、`runs/c-ctxloss/`): 窓 2 つ(主窓 `l14` と `l48`)を空にして resync 後 4 s で読み戻すと、**1 回目は前と画素まで同一**(`differingPixels` 0)、2 回目は主窓の 13 画素(x=1042、y=12〜24 の 1 画素幅の縦線)だけ違った。同じ canvas を loss なしで 4.2 s 間隔で読んだ対照でも**同じ 13 画素**が変わった(エンジンの主窓が自分で書き換えている縦線。メーター様)。= resync は主窓の 1930x1092 全体を元どおりに戻した。その後の `?dedupe=check` の比較は 0。

## 3. (b) File > Save / Save as / Save new version のダイアログの区間(テストバンドル F1xH + ia2、実エンジン、`?dedupe=check`)

テストバンドル(§1)を自前の vite preview(ポート 4610〜)で `?bundle=/assets/fl-engine-dd1-F1xH-ia2/manifest.json&pace=catchup&dedupe=check`。空のプロジェクト(ウィザードを Enter で閉じただけ)。入力は CDP の模擬(座標は決定 0191 の表の仮想画面座標)。比較は 0.7 s ごとの全窓・全画素。**偽装なし**: 『Buy FL Studio』窓(`TInAppShopForm`)はエンジン自身の窓で、Trial 状態の Save が開く(決定 0191 §6 と同じ)。表の全文は `tables/dd-check-b-dialogs.md`。

| セッション | 操作(順) | 見えた窓(エンジンの窓一覧の className) | 比較回数 | 差分ピクセル最大 | 最大窓数 | メニュー |
|---|---|---|---|---|---|---|
| save-1 | File>Save as → ボタン上のホバー → Esc → File>Save as → **Skip** → File>Save → ダイアログの **Save** → Buy 窓 6 s → Buy 窓上のホバー → Esc | `TNewProjForm`(Save as ダイアログ、485x404)、`TInAppShopForm`(Buy FL Studio) | 151 | **0** | 10 | 3 |
| save-2 | File>Save new version → Esc → File>Save new version → ダイアログの Save → Buy 窓 → Esc → Ctrl+S・Ctrl+Shift+S・Ctrl+N(修飾キーは**フラグだけ**: ダイアログは出ず)→ File>Open(何も開かない: このバンドルにブラウザのファイル橋はない)→ Esc | `TNewProjForm`、`TInAppShopForm` | 196 | **0** | 9 | 3 |
| save-3 | File>Export の 4 項目、Ctrl+R、File>Import の 4 項目(ia2 の CompareStringW の重みで**停止しない**。決定 0187 §11 では最初の項目で停止)、File>Save as template → Esc | `TNameEditForm`(Save as template の名前窓)。**Esc の後にエンジンが停止**(`fle: unsupported instruction at 0x4253a0` = `user32!SendMessageA`、ia2 の残った壁 W32、exit 134)。停止までの比較は差分 0 | 171 | **0** | 21 | 17 |
| save-4 | 修飾キーを先に押す正しい和音: Ctrl+Shift+S → Esc → Ctrl+S → ダイアログの Save → Buy 窓 → Esc → Ctrl+S(2 回目: **ダイアログなしで Buy 窓**)→ Esc → Ctrl+N → Ctrl+R | `TNewProjForm`(Ctrl+Shift+S と最初の Ctrl+S)、`TInAppShopForm` | 159 | **0** | 7 | 0 |

**合計: 4 セッション、677 回の比較(各回すべての窓の全画素。1 回に比べた窓は最大 7〜21)で、違う画素は最大 0**。dedupe の Worker の数(セッション終了時): 省略した矩形 4,454 / 6,124 / 6,009 / 5,219、送った 1,072 / 918 / 1,746 / 922(= 77〜87% が省略)。比較の対象になった区間: ダイアログの出現・ホバー・消去、Buy 窓(別の `TInAppShopForm`、484x636 の灰色)の出現・ホバー・消去、閉じた後の主窓の再描画(窓ごとの canvas を全画素)。

**範囲(正直に)**: (1) テストバンドルは SHIPH ではなく F1xH + ia2(混成なし)。(2) Ctrl+S 系は save-2 では修飾キーがフラグだけでダイアログが出なかった(そのセッションの Ctrl 系の比較は『何も起きない区間』の比較にすぎない)。save-4 で直した版を別セッションで取った。(3) Open は何も開かないので『Open のダイアログの区間』は**見ていない**(このバンドルにはブラウザのファイル橋 `-DFLE_FILEDLG_BROWSER` がない)。(4) Export の 4 項目はエンジンの窓を開かなかった(メニューの閉じ方だけ)。(5) 比較は 0.7 s 間隔。

## 4. (c) 長時間の自己検証(既定の入口 SHIPH、`?pace=catchup&dedupe=check`、実エンジン、隔離 headless Chrome 156)

配信中の既定の入口と同じ内容(`/assets/manifest.json` → SHIPH。`lift/out-dd1/serve/assets/` の読み取り専用 symlink)を、自前の vite preview(ポート 4610〜)で。1 セッション = 1 回のページ読み込み = 1 回のエンジン起動(メニューの上限 28 個のため。各セッションは 8〜10 個)。比較は 1.5 s ごと、全窓・全画素。表の全文は `tables/dd-check-c-long.md`、1 行ずつは `tables/dd-check-runs-oneline.md`。

| セッション | 中身 | 時間 | 比較回数 | 差分ピクセル最大 | 最大窓数 |
|---|---|---|---|---|---|
| c-long-1 | ウィザード → Enter → idle 10 s → メニュー 7(Edit..Help)+ 右クリックのメニュー 3(チャンネル・ミキサー・プレイリスト)→ F7 開閉 → F5/F6/F9 を開いて閉じる → **F8(全画面のプラグイン一覧 `TPlugListForm`)開閉**・ホバー → **F10(設定窓 `TMIDIForm`)開閉**・窓内クリック → チャンネルのダブルクリック → ミキサー/プレイリストのクリック → ポインタ掃引 → **再生 310 s**(再生中に 25 s ごとに F9 開閉・ポインタ掃引・F7 開閉を計 10 回)→ 停止 → idle | 505 s | 333 | **0** | 14 |
| c-long-2 | F8 開閉 → F7・F5/F6/F9 → メニュー 5 + 右クリック 3 → F8 → F10 設定窓 → **模擬の context loss**(§2.2)→ ダブルクリック・クリック・掃引 → **再生 150 s** → F7・F5/F6/F9 → 再生 60 s → idle | 458 s | 290 | **0**(模擬 loss の区間は比較を止めた。下記) | 14 |
| c-long-3 | **再生 310 s を最初に**(ウィザード直後)→ 停止 → メニュー 7 + 右クリック 3 → F5/F6/F9 → F8 開閉 → F10 設定窓 → クリック・掃引 → idle | 494 s | 323 | **0** | 14 |
| **c-long-4(1 本で 29.6 分)** | 4 周(周ごとにメニュー 2 + 右クリック 3、F7・F5/F6/F9 の開閉、F8 開閉 4 回・F10 設定窓 4 回の合計、クリック・掃引、再生 300 s〔再生中に F9/F7/掃引〕、idle)。再生は開始時・終了時とも idle 率 > 1,500/s。メニュー 20 個 | 1774 s(29.6 分) | 1169 | **0** | 8 |
| long-1(最初の版のスクリプト) | c-long-1 と同じ操作。ただし F5/F6/F9 の後に Welcome wizard が再び出て F8 を受けた(F8 の窓は開かなかった。F10 なし)。再生 320 s | 493 s | 322 | **0** | 14 |

**合計: 有効な 5 セッション、約 62.1 分(3,725 s)、2,437 回の比較で、違う画素は最大 0**。そのうち **`c-long-4` は 1 本で 1,774 s(29.6 分。『30 分以上』に 26 s 足りない)のセッション**(同じ engine を 1 回起動したまま 4 周: 周ごとにメニュー 5 個〔計 20 個、上限 28〕、窓の開閉、F8 開閉、F10 設定窓、クリック、再生 300 s〔再生中に窓の開閉・掃引・F7〕、idle。比較 1,169 回で差分 0)。他の 4 本はメニューの上限のためのセッション分割。有効 5 セッションの合計は 62.1 分。再生は 5 セッションとも開始時・終了時の両方でエンジンの idle 率 > 1,500/s(再生中の判定。決定 0182)を確認(再生の区間の合計 約 2,470 s: 318 + 246〔150 s と 60 s の 2 回と窓の開閉〕+ 312 + 324〔long-1〕+ 318×4〔long-4〕= 2,472 s)。**F8 の全画面一覧(`TPlugListForm`)の開閉は計 8 回〔c-long-1: 1、c-long-2: 2、c-long-3: 1、c-long-4: 4〕、F10 の設定窓(`TMIDIForm`)の開閉は計 7 回〔1+1+1+4〕、Welcome wizard の再出現、最大 14 窓**が同時に見えた区間を含む。dedupe の Worker の数(セッション終了時): 省略 18,044 / 1,475 / 2,705 / 17,905 に対し、変化があったため送った 4,910 / 3,710 / 917 / 4,729。**再生中は省略が少ない**(c-long-3: 再生 309 s で省略 1,955・送信 17,209。c-long-1: 再生 315 s で省略 14,285・送信 3,903。**読み、未確認**: 再生ヘッドなどの更新と 1 画素幅の無効化が同じ pump の dirty 矩形に合流すると『不変の矩形』にならない。c-long-1 は F5/F6/F9 を開閉した後に再生したので画面の更新が少ないのかもしれない)。

**差分が出たときの保存**(`first-diff-*.json` + crop)の経路は用意し、1 つの自己検知テスト(§2)で動作を確かめた。**本番の run では保存すべき差は出なかった**(出た 1 件は下)。

**除外した 1 件(自分のスクリプトの人工物)**: 最初の版の `long-2`(`runs/long-2/`)は、『模擬 context loss』の段階で 2,106,690 画素(主窓 1930x1092 全体)の差を記録した。原因は私のスクリプト: ページの canvas を空にした(`canvas.width = canvas.width`)のに参照 canvas(DOM にない)は空にしていないので、その間の比較は構造的に違う。以後の版は loss の区間は比較を止める。その run の他の段階はすべて 0 で、loss の後 resync が済むと比較は 0 に戻った。`first-diff-long-2.json` はこの人工物の記録で、**実際の差ではない**。

## 5. 既定を ON にする差分(`dd1-default.diff`。`dd1.diff` の**後に**当てる)

条件 (a)(b)(c) を上の範囲で満たしたので、既定 ON の差分を別ファイルに分けた。**取り込みは PM の判断**(§7)。`app/` の 5 ファイルだけ:

| ファイル | 変更 |
|---|---|
| `src/engine/dedupe.ts` | `resolveDedupeParam(v)` を追加: 値が無い・`1`・`1/0/check` 以外 → ON(`true`)、`0` → `false`、`check` → `'check'`。冒頭の説明を『既定 ON(決定 0201)』に |
| `src/main.ts` | `const dedupe = resolveDedupeParam(dedupeParam)`(以前は `check` 以外は `=== true`)。ページのログ: 値が無いとき『unchanged window rectangles are not sent (the default since decision 0201; ?dedupe=0 sends every one)』、`?dedupe=0` は『every rectangle is sent』、未知の値は警告つきで ON |
| `test/unit/dedupe.test.ts` | `resolveDedupeParam` のテスト(値なし・1・0・check・未知の値 8 通り) |
| `test/e2e/run.mjs` | 4c の期待値: `dedupe-standin-default` と `dedupe-standin-bad`(`?dedupe=2`)は ON(Worker が省略/送信の数をログに出す)、`dedupe-standin-zero` だけ『Worker が dedupe について何も言わない』。context loss の『dedupe なし』場面は `&dedupe=0` を付ける |
| `test/mutation/run.mjs` | `worker-dedupe-flag-ignored` の対象場面を `dedupe-standin-zero` に。新規 2 件: 既定を OFF に戻す、`0` を無視する |

**動作**: `?dedupe=0` で従来(全矩形を送る)に戻る。`?dedupe=check` は従来どおり。ランタイム・生成 C・wasm・バンドルは変わらない(`app/` のみ)。Worker 自身の既定(`dedupeOn = false`)と `EngineClient.init` の引数(`dedupe` を送るのは `true`/`'check'` のときだけ)は変えていない = ページが決める。

**『`-` 行』**: `dd1-default.diff` の `-` 行 15 本をすべて読んだ: dedupe.ts のコメント 2、main.ts の 5 本(import・コメント・`dedupeMode`・`dedupe` の定義・旧ログの 1 行)、e2e の 6 本(4c の説明 2 行・場面の表・『Worker が言わない』のチェック・ログのチェック・context loss の場面の表の 1 行)、mutation の 1 行(e2eOnly)、unit の import 1 行。すべて意図した書き換え。

**この木での結果**(`dd1.diff` + `dd1-default.diff`):

| 項目 | 結果 |
|---|---|
| 当て方 | 最新の `git archive HEAD`(26eb04d と 7f024d7 は `app/` が同じ)に `dd1.diff` → `dd1-default.diff` の順に当てた木がワークツリー `src-default/` と一致(`diff -r` が空) |
| typecheck | 通過(`logs/default-typecheck.txt`) |
| vitest | **1225 passed + 7 skipped**(`dd1.diff` の木 1224 + `resolveDedupeParam` の 1 件。`logs/default-vitest.txt`) |
| e2e(dev・preview・worker) | **1071/1071**(`logs/e2e-full-default.txt.gz`)。最初の版では、私が足した context loss の『dedupe なし』場面のチェック 3 件(各 target に 1 件)が落ちた(既定が ON になったので `&dedupe=0` が要った。上の e2e の行で直した)。**既存の 1017 チェックのうち期待値を変えたのは 4c の `default`・`bad` 場面のチェックだけ**で、他は変更なしで通る |
| 変異検査(既定の部分) | **5/5 検出**(`logs/default-mutation.txt`: 既定を OFF に戻す、`0` を無視、未知の値を受け入れる、`check` を失う、Worker の flag を無視) |
| 最新の HEAD(48cec69)に当てた木 | `dd1.diff` → `dd1-default.diff` の順に `patch -p1` で当てた木: typecheck 通過、vitest **1235 passed + 7 skipped**(HEAD 側のテストが増えている)、e2e **1098/1098**(`logs/combined-on-head-48cec69.txt`、`logs/e2e-full-head48cec69-dd1-plus-default.txt.gz`) |

## 6. 既定 ON で『入力 → 次の rAF』の遅延は下がるか(決定 0194 の表との照合)

**方法**: lat1 の測定器(`lift/harness/web/lat1/` の写し。`tools-lat1/`。構成 `dedupe0`・`d1b0`・`d0b0` と `DD_DIST` を足した)を、`dd1.diff` + `dd1-default.diff` の木の `vite build` に対して。待機(idle)の 6 セル(クリック・ホバー・ホイール・FILE メニュー・右クリックメニュー・Space)、1 セル 12 反復(ホバーは 24、メニューは 7×2)。起点は `event.timeStamp`、終点は変化の描画の次の rAF の開始。1 度に Chrome は 1 つ(同時に (c) の long-4 の Chrome が別に 1 つ動いていた時間がある)。**構成**:

| 構成 | URL | 意味 |
|---|---|---|
| `SHIPH` | パラメータ無し(既定の入口) | この木では dedupe 既定 ON、**ビーコン ON(既定)**。アイドルの窓の更新 0.82/s |
| `dedupe0` | `?dedupe=0` | dedupe OFF、ビーコン ON。アイドルの窓の更新 62.4/s |
| `d1b0` | `?dedupe=1&beacon=0` | dedupe ON、**ビーコン OFF** |
| `d0b0` | `?dedupe=0&beacon=0` | dedupe OFF、ビーコン OFF |

それぞれ 2 組(順序を替えて交互: `SHIPH,dedupe0` → `dedupe0,SHIPH`、`d1b0,d0b0` → `d0b0,d1b0`)。**この機械は load 7〜35(他担当と共有。run ごとの load は `tables/lat-cpu-seconds.md` と各 run の JSON)なので、ms は速度の主張に使わない**(参考値。load ≤ 8 の run は `d1b0`/`d0b0` の一部だけ)。主張できるのは CPU 秒(§6.1)と、同じ時間帯の対での向き(§6.3)。

### 6.1 CPU 秒(load に比較的よらない。ビーコン ON の 2 組、全 7 セルで同じ向き)

ページの主スレッドの CPU(セルの CPU 秒 ÷ 壁の秒 = コア):

| セル | `SHIPH`(既定 = dedupe ON) | `dedupe0`(OFF) |
|---|---|---|
| アイドル(背景、6 s) | 0.035、0.056 | 0.113、0.158 |
| クリック | 0.072、0.096 | 0.151、0.186 |
| ホバー | 0.055、0.052 | 0.132、0.173 |
| ホイール | 0.075、0.071 | 0.155、0.185 |
| FILE メニュー | 0.093、0.087 | 0.157、0.185 |
| 右クリックメニュー | 0.067、0.062 | 0.149、0.169 |
| Space 開始/停止(再生を含む) | 0.219、0.213 | 0.290、0.277 |

**ページ主スレッドは ON で約半分以下**(アイドル 0.113〜0.158 → 0.035〜0.056 コア)。Worker の CPU はほぼ同じ(アイドル 0.160/0.257 vs 0.156/0.197 コア。ON の 2 組目は load 22 で高め)。= 決定 0187 §7.1(ページ主スレッド 0.073 → 0.016、Worker 変わらず)と決定 0194 §1 の 2(ページ主スレッド 0.106 → 0.02)の**CPU の結論は再現**。**ビーコン OFF の対(§6.3)では差がさらに大きい**: アイドルのページ主スレッド `d1b0` 0.020、0.023 vs `d0b0` 0.168、0.169 コア(`tables/lat-b0-cpu-seconds.md`。ビーコン ON の差が小さいのは、ビーコンの rAF の分が両方に乗るため〔読み〕)。

### 6.2 ビーコン ON(既定のページ): event.timeStamp → 次の rAF(ms、中央値、参考。load 10〜35)— **改善は出なかった**

`tables/lat-event-to-raf-tables.md`。2 組・各 n=24(ホバー 48、メニュー 14)、`SHIPH`(ON)/ `dedupe0`(OFF):

| input | SHIPH (p50 / p95 ms, n, per-round p50) | dedupe0 |
|---|---|---|
| left click (press) | 27.5 / 34.5 (n 24/24) [28.5, 27.3] | 24.3 / 29.5 (n 24/24) [25.9, 21.5] |
| hover (one move) | 30.8 / 33.5 (n 48/48) [27.5, 32.3] | 26.8 / 34.3 (n 48/48) [26.8, 25.0] |
| wheel (tempo field) | 47.6 / 60.6 (n 24/24) [49.0, 46.8] | 58.2 / 76.6 (n 24/24) [57.1, 58.2] |
| Space (start) | 28.6 / 33.4 (n 24/24) [28.9, 24.6] | 26.8 / 41.6 (n 24/24) [24.9, 29.2] |
| Space (stop) | 23.6 / 36.2 (n 24/24) [23.0, 24.7] | 26.9 / 58.6 (n 24/24) [27.8, 25.9] |
| close the FILE menu (Escape) | 28.8 / 33.7 (n 14/14) [28.8, 29.3] | 26.6 / 31.5 (n 14/14) [28.9, 25.3] |
| open the FILE menu | 28.6 / 45.4 (n 14/14) [28.7, 26.8] | 27.3 / 78.5 (n 14/14) [26.9, 31.9] |
| close the context menu (Escape) | 23.2 / 30.4 (n 14/14) [27.3, 22.6] | 26.4 / 31.3 (n 14/14) [26.4, 26.9] |
| open the context menu | 28.9 / 44.2 (n 14/14) [28.9, 30.2] | 28.9 / 142 (n 14/14) [28.9, 30.9] |

決定 0194 の表(ホバー 30.0 → 15.5、クリック 18.1 → 10.2、ホイール 48.5 → 34.2、メニューを閉じる 19.9 → 10.6 ms)のような差は**出なかった**: ON と OFF の中央値は組によって入れ替わる範囲(±数 ms)にあり、ホイールだけ ON が約 10 ms 短い。**原因(PM から:cbeacon2 の報告)**: 決定 0192(cbeacon1)の画面測定は、ビーコンが有効だと待機中も `requestAnimationFrame` を回し続ける。ページがフレームを作り続けると Chrome はポインタ移動の配信を次のフレームまで持つので、dedupe で窓の更新をなくしても利点が消える。cbeacon2 の対照(PM の報告、本作業の測定ではない): `?dedupe=1` ビーコンあり クリック/ホバー p50 24.8, 26.3、`?dedupe=1&beacon=0` は 9.7, 12.1 / 15.6, 15.2(lat1 の 10.2/15.5 と一致)。cbeacon2 が rAF 測定を短いバーストに直す(別の diff)。**= 本作業の既定 ON のページ(ビーコン ON)は、dedupe の遅延の利点を失っていた**(私の測定 §6.3 と PM の対照が一致)。

### 6.3 ビーコン OFF(`beacon=0`): dedupe ON と OFF の対(同じ手順、2 組・交互)— **クリック・ホバー・ホイール・Space 開始・FILE を開くで利点が出る。メニューを閉じる・Space の停止では出ない**

| input | d1b0 (p50 / p95 ms, n, per-round p50) | d0b0 |
|---|---|---|
| left click (press) | 7.1 / 17.9 (n 24/24) [5.8, 8.9] | 24.2 / 26.9 (n 24/24) [24.3, 17.8] |
| hover (one move) | 15.2 / 25.3 (n 48/48) [15.2, 13.9] | 30.2 / 32.5 (n 48/48) [30.2, 30.0] |
| wheel (tempo field) | 30.2 / 45.2 (n 24/24) [30.5, 28.3] | 46.4 / 57.6 (n 24/24) [46.6, 46.4] |
| Space (start) | 12.2 / 26.6 (n 24/24) [10.5, 17.2] | 24.0 / 29.5 (n 24/24) [23.0, 24.2] |
| Space (stop) | 22.2 / 28.4 (n 24/24) [21.5, 23.6] | 21.0 / 39.7 (n 24/24) [25.3, 19.6] |
| close the FILE menu (Escape) | 13.3 / 21.5 (n 14/14) [13.5, 12.1] | 14.0 / 28.8 (n 14/14) [14.2, 12.9] |
| open the FILE menu | 19.2 / 35.5 (n 14/14) [22.1, 19.2] | 28.4 / 54.4 (n 14/14) [28.8, 27.6] |
| close the context menu (Escape) | 18.2 / 30.3 (n 14/14) [20.4, 16.4] | 13.6 / 30.6 (n 14/14) [12.4, 21.8] |
| open the context menu | 22.1 / 29.4 (n 14/14) [20.9, 26.2] | 23.4 / 31.7 (n 14/14) [20.4, 26.7] |

2 組・各 n=24(ホバー 48、メニュー 14)の中央値、ON = `d1b0`、OFF = `d0b0`。**1 組目は両方 load 6.5〜8 で取れた**(`d1b0` r1: 6.5〜7.3、`d0b0` r1: 6.7〜8.0)、2 組目は load 9〜22(`d1b0` r2 は 17〜22)。

**効果が出た入力**(ON が短い。対の順序を替えても同じ向き。ms は参考): 左クリック 24.2 → 7.1、ホバー 30.2 → 15.2、ホイール 46.4 → 30.2、Space 開始 24.0 → 12.2、FILE メニューを開く 28.4 → 19.2。= **決定 0194 §1 の 2(クリック 18.1 → 10.2、ホバー 30.0 → 15.5、ホイール 48.5 → 34.2)とほぼ同じ向きと大きさで再現**(ホバーは 15.2 と 15.5、ホイールは 30.2 と 34.2)。ビーコン ON(§6.2)の同じ cell は ON/OFF ともに 27〜30 ms 付近で、利点はなかった。

**効果が出なかった/逆だった入力**(正直に): Space の停止 22.2 vs 21.0(同じ。決定 0194 の例外は『ON の方が遅い』だったが、ここでは差なし)、メニューを閉じる(FILE)13.3 vs 14.0(同じ。決定 0194 の表の 19.9 → 10.6 は再現せず。OFF 側が今回は短い: 14.0)、右クリックのメニューを閉じる 18.2 vs 13.6(**ON の方が長い**、n=14、1 組目 20.4 vs 12.4、2 組目 16.4 vs 21.8 と組で逆転)、右クリックのメニューを開く 22.1 vs 23.4(同じ)。= メニューを閉じる系は、この n と load では ON の利点は言えない。

**範囲**: 待機(idle)のみ、1 セル n=24 前後、2 組、load 6.5〜22、依頼者の機械でも実画面でもない(ヘッドレス Chrome は rAF が 60 Hz)。120 fps については何も言えない。CPU(`tables/lat-b0-cpu-seconds.md`)はビーコン OFF でも ON の方が主スレッドが少ない向き。

## 7. PM の判断点

1. **`?dedupe` を既定 ON にするか**(`dd1-default.diff`)。**既定 ON の根拠は『画素が同一(差分 0)』と『ページ主スレッド CPU 約半分』に限る**。遅延の改善は、ビーコン OFF(`?beacon=0`)の測定で確認(§6.3)、**ビーコン ON(既定のページ)は PM の指摘〔cbeacon2: ビーコンが待機中も rAF を回してページがフレームを作り続け、Chrome がポインタ移動の配信を次のフレームまで持つ〕のとおり利点を失っていて、cbeacon2 の修正後に再確認**。条件 (a)(b)(c) の充足: (a) は実装・単体・e2e・変異まで済み。ただし**実 Chrome での『復元後の画素』は検証できていない**(この Chrome は `contextrestored` を出さず、合成イベント + canvas 消去の模擬で見た)。(b)(c) は上の範囲で差分 0。既定 ON の差分は `?dedupe=0` で OFF に戻せる。`dd1-default.diff` は cbeacon2 の統合後に当てる想定で、作り直していない(PM の指示)。残るリスク = 依頼者の GPU・実 Chrome での復元後、フレームの間だけ現れる差(比較は 0.7〜1.5 s 間隔)、120 Hz 以上の画面。**取り込みの判断は PM**。
2. **復元されない canvas の扱い**(S-dd1-no-replace)。この Chrome は GPU クラッシュの後 `contextlost` を出したきり `contextrestored` を出さず(最長 約 19 s 観測)、元の canvas は永久に描けない(新しい canvas は使える)。= dedupe の有無に関わらず、そうなるとその窓は空のまま。ページが canvas を作り直す(`DesktopView` に差し替えの API を足す、約 30 行)かは未実装。実機でこの状況が起きるかも未確認。
3. **出荷バンドルに ia2 の行を載せるか**。SHIPH(配信中の既定の入口)には ia2 の行がなく、`File > Save / Save as / Save new version` は `user32!GetKeyNameTextW` で停止する(決定 0187 §11 の W18)。本作業のテストバンドル(F1xH + ia2、+5.9 MB)では、Chrome で Save as ダイアログ、Save(『Buy FL Studio』窓)、Ctrl+S、Ctrl+Shift+S、Import の 4 項目が**停止せずに**動いた(ia2 の native 観測と一致。Chrome での再現は初めて)。残る壁: `File > Save as template` の名前窓を Esc で閉じると `user32!SendMessageA`(0x4253a0、ia2 の W32)で停止。ia2 を SHIPH に載せる(行の追加・再生成・wasm 化・サイズ測定)は別の作業。
4. **Space の停止の遅れ**(決定 0194 §4.5 の例外: dedupe1 の方が 17.8 → 24.8 ms)。§6.3 の再測定(ビーコン OFF の対)では Space の停止は ON 22.2 vs OFF 21.0 ms で差なし、右クリックのメニューを閉じるは ON 18.2 vs OFF 13.6 ms で ON が長い(n=14、組で逆転)。『メニューを閉じる/Space の停止では dedupe の利点が出ない』が、この n では『逆効果』とも言えない。
5. **既定 ON はアプリ自身の遅延の測定(`LatencyTracker`、決定 0192 のビーコン)の意味を変える**(決定 0187 §7.3 の L2 と同じ現象。読み、この作業では再測定していない): 画素を変えない入力(ボタンの押下だけで見た目が変わらないクリックなど)には『次の更新』が来ないので、`input → 次のフレーム` は次の無関係な更新まで伸びる。応答が遅くなったのではなく、毎フレームの同一矩形が『フレーム』として数えられていた人工的な短さが消える。依頼者のブラウザから届くビーコン(`perf-log.jsonl`)の p95 は既定 ON の後で大きく見えうる。P2 の『遅延』は `event.timeStamp` から『変化の描画の次の rAF』までで測る(決定 0194 の勧告)。
6. **`?dedupe=check` は方式 B のみ**(`?present=A` ではページが canvas を持たない)。実エンジンの既定は B。

## 8. 出典のない選択(S-dd1-…)

| 番号 | 選択 | 理由・代替・撤回条件 |
|---|---|---|
| S-dd1-both-events | `contextlost` と `contextrestored` の**両方**で再同期を要求する | 失われている間に送ったフレームは描かれない(canvas は描画を無視する)ので、lost の時点で shadow を信用しないこと、restored(空の canvas)で全面を送り直すことの両方が要る。代替: restored だけ(lost の間の送信が shadow に残るが restored の再同期で消える。結果は同じで、lost の間の全面送信を 1 回省ける)。撤回: 全面送信(主窓 8.4 MB)の費用が問題になったとき |
| S-dd1-always | 再同期は `?dedupe=` の値に関係なく働く(dedupe OFF でも全窓を送り直す) | OFF でも失われた canvas は(1 画素幅のアイドル再描画では)戻らないため。既存の既定動作はイベントが来た時だけ変わる。代替: dedupe のときだけ |
| S-dd1-all-windows | 全窓を送り直す(影響を受けた canvas だけではない) | GPU のリセットは通常すべての canvas に及ぶ。窓ごとの再同期は代替(メッセージに key を持たせる) |
| S-dd1-cancel-lost | `contextlost` を常に `preventDefault()` | HTML の規則(WebGL と同じ): 取り消すと復元が許される。この Chrome では取り消しても復元されなかった。取り消さない場合は試していない。代替: 取り消さない |
| S-dd1-no-replace | 復元されない canvas の差し替えは実装しない | この Chrome は `contextrestored` を出さなかった。差し替えは `ui/desktop.ts` の canvas 配置(card・DOM 位置)に触る。代替: `ClientEvents.replaceCanvas(key)` を足し、lost の後 N 秒で新しい canvas に差し替えて resync。撤回: 実機で復元されない例が出たとき |
| S-dd1-pump-now | `resync()` はメッセージを受けた場で `pumpAll()` も呼ぶ | エンジンが idle を出さない間も全面送信を始めるため。代替: 次の idle の pump に任せる |
| S-dd1-check-crop | `dedupeCheck()` が差のある窓に出す物: 最初の画素、外接矩形、256×256 以内の crop | 調査用の情報。本番の経路では使わない |
| S-dd1-menu-budget | 1 セッションのメニューは 28 個まで(スクリプトが超えると止まる) | runtime の hook 表の累計 64 個・メニュー 31 個目で停止(決定 0194、tbl1 が是正中)の手前。代替: 他の値 |
| S-dd1-poll | 比較の間隔: (c) は 1.5 s、(b) は 0.7 s | 決定 0187 は 1.5 s。ダイアログの区間は短くした。フレーム単位ではなく、フレームの間だけ現れて消える差は見えない |
| S-dd1-test-bundle | (b) のテストバンドルは F1xH + `ia2-imports.txt`(混成 p1000r なし)、配信用の圧縮は gzip(ローカルの vite preview 用。brotli q11 は遅いので使わない) | 混成の手順(2 段リンク・wasm-opt 2 回目・merge)は別で時間がかかる。混成は 16 スナップ 82 行で F1xH と一致している(決定 0189)ので、描画の列は同じはず(読み)。**SHIPH 同一の構成ではない**。代替: 混成も作る |
| S-dd1-default-invalid | 既定 ON の版では、1/0/check 以外の値(`?dedupe=2`)は無視して(警告つき)ON | 以前は OFF だった。『壊れた URL でも既定に従う』を選んだ。代替: 警告して OFF |
| S-dd1-sim-loss | e2e と実エンジンの『復元後の画素』は、canvas を空にして `contextlost`/`contextrestored` を dispatch する**模擬**で見る | この Chrome は実際の GPU クラッシュで `contextrestored` を出さない。実機 GPU・実 Chrome での復元後は未確認 |
| S-dd1-lat-cells | 遅延は待機(idle)の 6 セル(クリック・ホバー・ホイール・FILE メニュー・右クリックメニュー・Space)× 12 反復 × 2 組。F8・ドラッグ・再生中は測らない | 時間(総計 3.5 時間)のため。決定 0194 の表の主要な 4 入力 + Space。n が小さく、p95 は粗い。代替: 全セル・3 組以上 |
| S-dd1-no-wait-load | 遅延の run は load ≤ 8 になるのを 1〜3 分しか待たず、下がらなければそのまま走らせた(load は各セルの JSON に記録) | load は他担当で 7〜35 を行き来し、待っても ≤ 8 が続く保証がない。速度は主張せず、CPU 秒と『同じ時間帯の対の向き』で書いた |
| S-dd1-long-4 | 30 分に近い 1 本(`long-4`、実際は 29.6 分)は 4 周(各周: メニュー 2 + 右クリック 3、窓の開閉、F8、F10、クリック、再生 300 s + 窓の開閉/掃引/F7、idle)。メニューは 1 周 5 個 × 4 = 20(上限 28) | PM の『30 分以上の run』。5 周(約 37 分)にすると時間が足りないと見て 4 周にしたところ、29.6 分で 30 分に 26 s 足りなかった |

## 9. 自己申告(規則違反・事故・誤り)

- **存在しない `bash` ツールを 3 回呼んだ**(いずれも『shell として実行した』と返った)。**`sleep` を 2 回試し**(`sleep 1`、`sleep 0`)、拒否された。待ちは出力ファイルの読み直しにした。
- **e2e と mutation の作業場所の既定が、他担当と共有の `/tmp/opencode/app1` だった**。最初の e2e 4 場面の確認と ctx-resync の変異検査 8 件はそこで走らせた(`APP_WORK` で分けたのはその後)。他担当の `--only default-manifest` の e2e が同時に走っていたが、干渉の形跡は見ていない(確認はしていない)。
- **重い処理の同時数が 2 を超えた時間帯がある**: wasm の単位コンパイル(`-j3`)+ 全 e2e + Chrome の長時間走行が重なった(00:52〜00:58 頃)。load は 25〜45(他担当の分も含む)。速度は主張していない。
- **私のスクリプトの人工物を『差分』として記録した**: 最初の版の `long-2`(§4 の除外した 1 件)。気づいてスクリプトを直し(loss の区間は比較を止める)、`c-long-2` で取り直した。`runs/long-2/` と `first-diff-long-2.json` は残してある(実際の差ではない)。
- **最初の版の run は古いビルドで走った**: `long-1`・`long-2`(最初の版)・`smoke-idle` は `dedupeCheck()` に『最初の差の画素・外接矩形』を足す前の `dist-dd1` を使った(差が出なかったので、結果には影響しない。`long-2` の人工物の記録に位置が無いのはそのため)。以後は `dist-dd1b`。
- **F8 が効かなかった run**: `long-1`(最初の版)は F5/F6/F9 の後の Welcome wizard が F8 を受けて、F8 の窓が開かなかった。スクリプトを直して(エンジンが idle になるのを待って wizard を閉じる)から `c-long-1` などで F8(`TPlugListForm`)の開閉を確認した。`long-1` は『F8 なし』の run として数えている(§4 の表に注記)。
- **save-2 の Ctrl+S 系の入力は修飾キーを『フラグ』としてだけ送ったので、ダイアログが出なかった**。save-4 で修飾キーのキーイベントを先に送る形に直して取り直した。save-2 のその段階は『何も起きない区間の比較』にすぎない(§3 に注記)。
- **テストバンドルは SHIPH と同一の構成ではない**(混成 p1000r なし、`ia2-imports.txt` に exp1 の行のうち 6 行が入っていない)。依頼文は『SHIPH の構成に ia2 の行を足す』と書いたが、混成の手順は別で、このセッションでは作っていない。
- **稼働中の run（`c-long-4`）のプロファイルと netlog を、整理のとき誤って消した**(`runs/*/profile-*` を一括で消した)。Chrome は消えたファイルを開いたまま動き続け、run は最後まで走って結果が出た(1,169 回の比較、例外なし)が、その run の `netlog` は無い。
- **`tools/mkdiff.sh` の最初の版は基準を『最新の HEAD』にしていた**ため、HEAD が進んだ(48cec69)後に作った `dd1.diff` が他担当の変更の逆を含んでいた。気づいて基準を 26eb04d に固定して作り直した(dry-run は最新の HEAD に対して)。出した `dd1.diff` は直した後の物。
- **`python` の出力を大きく画面に出した**(セルの `draws` の配列、約 100 KB)。
- **遅延の run に別の Chrome(long-4)を並走させた時間がある**(`d1b0`/`d0b0` の run。load は各セルに記録)。ms は速度の主張に使っていない。
- **遅延の原因の説明は PM から**(cbeacon2 の報告: ビーコンが待機中も rAF を回す)。私の測定は §6.2(ビーコン ON で改善なし)と §6.3(ビーコン OFF で改善あり)でそれと整合したが、原因そのもの(rAF の有無)は私は切り分けていない。
- 第 1 報(00:47)の『e2e 新規 dev で 18/18』は dev だけの数。全体は 1071/1071(§2.1)。
- 作業ディレクトリ `lift/out-dd1/` は一時 3.5 GiB(上限 5 GiB 以内)。終了時に 500 MB 以内へ整理した(§10)。
- 実リポジトリの tracked ファイルは変更していない(`git status` で開始時と同じ untracked 2 件だけ)。配信中の `lift/out-websrv/` は読み取りのみ(symlink 経由で SHIPH を配信させただけ。manifest は変えていない)。`lift/out-lat1/` はツールの写しを読んだだけ。

## 10. 成果物

`lift/out-git/dd1-results/`(3 MB 以内): `dd1.diff`(a の実装・テスト・変異 + `dedupeCheck` の詳細。既定は変えない)、`dd1-default.diff`(既定を ON にする。`dd1.diff` の後に当てる)、`MERGE-NOTES.md`、`docs/0201-dedupe-default.md`(本書)、`docs/README-0201-line.txt`、`tables/`(check 走行の表・遅延・新規 e2e)、`logs/`(テストの出力、バンドルのビルド、patch の dry-run)、`tools/`(走行・集計・差分作成のスクリプト)、`tools-lat1/`(遅延の測定。`lift/harness/web/lat1/` の写しに `dedupe0` の構成と `DD_DIST` を足した物)。

作業ディレクトリ `lift/out-dd1/`: `src/`(`dd1.diff` の木)、`B/bundle/fl-engine-dd1-F1xH-ia2/`(**テストバンドル**、gzip。配信中の `lift/out-websrv/assets/` には置いていない)、`runs/`(各走行の JSON・first-diff・crop。netlog は消した)、`serve/assets/`(symlink だけ)。生成 C・オブジェクト・wasm の中間物・cargo-target・emcache・`src-default/`・`dist-*` は終了時に消した。

## PM の判断と取り込み(2026-11-04)

取り込み: `dd1.diff` だけを HEAD 48cec69 に当てた(`patch -p1 --dry-run` は offset 2 行のみ、app/ の 11 ファイル、`-` の行 7 本をすべて読んだ: `dedupeCheck()` の定義の書き換え〔比較を窓ごとの画素数に拡張〕と `ModelSink` の1行の拡張で、意図した変更)。**`dd1-default.diff`(既定 ON)は今回は当てない。** PM の測定(`dd1.diff` 適用後の HEAD の木): typecheck 通過、vitest 1,234 passed + 7 skipped、Worker 43/43、e2e の結果は次のコミットメッセージに記載。lift/ には触れていないので全体テストと a4 は再走していない。

判断:

1. **`?dedupe` は引き続き opt-in**(`?dedupe=1`/`check`)。既定 ON(`dd1-default.diff`)は、cbeacon2 の修正(画面測定の rAF を連続からバーストへ)を統合し、ビーコン ON でも遅延の利点が戻ることを交互の測定で確かめた後に、PM が当てるかを決める。既定 ON の根拠は「画素同一(差分 0、既定の入口 SHIPH で 62.1 分・2,437 回の全窓全画素比較)」と「ページ主スレッド CPU の減少(アイドルで 0.113〜0.169 → 0.020〜0.056 コア)」であり、遅延の改善は `?beacon=0` の測定でだけ確認された(クリック 24.2 → 7.1、ホバー 30.2 → 15.2、ホイール 46.4 → 30.2 ms、参考値)。
2. **contextlost/contextrestored の再同期は採用**(§2、既定の挙動を変えない)。実 Chrome は `contextrestored` を出さない場合があり(最長約 19 s 観測)、復元後の画素は模擬(canvas を空にして dispatch)でしか確認できていない。canvas の差し替え(S-dd1-no-replace、約 30 行)は未実装で、実機で起きるかも未確認。
3. 出荷バンドルへ ia2 の行を載せる件(§7.3)は ship2 で扱う。`ia2-imports.txt` には exp1 の 6 行(`IFileSaveDialog::` ほか)が入っていない点も ship2 の EXTRA_IMPORTS の中身で確認する。
4. 依頼者のビーコンの p95 は、既定 ON の後で大きく見えうる(§7.5、読み): 画素を変えない入力には「次の更新」が来ないため。P2 の遅延は `event.timeStamp` から変化の次の rAF までで読む(決定 0194、0202)。
5. 作業者の自己申告(long-4 の netlog を整理で誤って消した、最初の長時間 run の差分 2,106,690 画素は自分のスクリプトの人工物、`mkdiff.sh` の基準を取り違えて作り直した、重い処理が2本超、存在しない bash ツール3回、sleep 2回〔拒否〕)は記録する。「30 分の1本」は 29.6 分(26 s 不足)。

## PM の判断: 既定 ON に変更(2026-11-04、決定 0205 の測定に基づく)

`dd1-default.diff`(`app/src/engine/dedupe.ts`、`main.ts`、e2e、変異検査、単体テストの 5 ファイル)を取り込み、**`?dedupe` の既定を ON にした。`?dedupe=0` で無効、`?dedupe=check` は従来どおり自己検証**。PM の測定: typecheck rc=0、vitest 1,362 passed + 7 skipped、e2e 1,109/1,109。根拠(決定 0205 §、担当の実測。この機械、模擬入力): 外側の p50 が 7 種類すべてで ON ≤ OFF、ページ主スレッドの idle CPU 約 0.14 → 0.025 コア、`?dedupe=check` の全窓全画素比較(2,437 + 629 回)で差 0、contextlost の再同期は決定 0201 で実装済み。弊害として記録: ノブを押すだけ(ドラッグなし)は ON が遅い向き(p50 241 対 207 ms、4 run、load 48〜56。原因未調査)。依頼者の Firefox での確認は 0 件。戻し方: `?dedupe=0`、またはこのコミットの revert。
