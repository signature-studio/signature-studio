# DPR を実行中に変えると engine が止まる(page/dpr-change-keeps-drawing)の読み

日付: 2026-11-28(UTC 02:11 に開始)。読むだけの作業。製品のビルドと E2E は、この機械では行っていない。

対象: `#1509` の作業者の報告(`.tmp-art2/1509/report.md` の (f) と、page 2/3 の記述の 19 行目)に出た `page/dpr-change-keeps-drawing`(「DPR 1.8 でエンジンが exit code 5 で止まる(製品の候補、未確認)」)。

## 結論

- **製品か E2E の操作か: 確定できない。推定では製品側(engine の停止)。** E2E の操作(CDP で DPR を変え、物理の窓の大きさを保つ)に、誤りの根拠は見つからなかった。同じシナリオは、記録に残っている別の run で、基準 DPR 1・1.84・2.89 から 1.75 へ変えて ok になっている(実測。下)。
- **「exit code 5」の意味**: `lift/harness/fruity.c:652` の `if (in_report) _exit(5);` が出所。停止の報告(`report()`)の最中に、もう一度 abort が起きると 5 で終わる(推定の範囲で、コードの読みとしては確か)。つまり 5 は、一度目の停止の原因の番号ではなく、報告中の再入の値。一度目の原因は、ログが無いので確認できない。
- **「DPR 1.8」の出所**: 4k-random の行の基準 DPR(乱数で 1.80)と読める(推定。判定の出力は `DPR 1.84` の形で基準 DPR を書く。`judge.txt` 16000 行)。この行のログは、ワークツリーに無い(確認できない)。
- **再現の注意(確かめた事実)**: このシナリオの宣言は既定値 `['4k-dpr1', '4k-random']`(`tools/e2e/lib/scenario.mjs:4` と `:86`)。`--displays` は宣言を広げない(`tools/e2e/run.mjs` の 17 行目の注釈。`lib/plan.mjs` の `rowsFor` が `sc.displays.includes(name)` で絞る)。よって、報告のとおり `--displays 4k-dpr2` や `dpr:1.8` を指定しても、宣言に無い行では走らない。1.8 の行で再現するには、宣言に `dpr:1.8` を足す必要がある。

## 読んだ物(行つき)

E2E の操作
1. `tools/e2e/scenarios/page/dpr-change-keeps-drawing.mjs:9` `return dprMoveCase(ctx, 1.75);`。宣言なし(既定の 2 行)。
2. `tools/e2e/lib/page.mjs`:
   - 107〜111 `setRatio`: `Emulation.setDeviceMetricsOverride` で、物理の大きさ(`inner × 旧 DPR`)を保ったまま、CSS の窓を `物理 ÷ 新 DPR` にする。
   - 126〜140 `waitSettled`: 20 秒まで、canvas の CSS の大きさ・guest の画面・部屋の画素を確かめる。
   - 230〜238 `menuChangesPicture`: File メニューを開いて閉じ、画の変化を見る。
   - 288〜302 `dprMoveCase`: 変更 → settle → crisp → メニュー → 基準へ戻す → settle。`ok = s.ok && r.ok && drawn`。
3. `tools/e2e/lib/plan.mjs`: 59〜73 `displayRows`(4k-random は 1.01〜3.00 の 2 つの乱数、小数 2 桁)、76〜79 `fixedDpr`、92〜95 `rowsFor`。

DPR の変化が engine に届く経路(製品)
4. `app/src/main.ts`:
   - 305・308 `measureRoom` / `applyDisplay`: 部屋の CSS の大きさと DPR を読む。
   - 338 `resize` → `applyDisplay`。342 `ResizeObserver`。
   - 357〜365 `onDevicePixelRatio`: DPR が変わると、`applyDisplay`・`applyGuestCursor`・`roomFollower.push`。
   - 367〜373 `watchDevicePixelRatio`(matchMedia)・`resize`・250 ms のタイマー。
   - 391〜404 `followRoom`: `chooseGuestScreen(room)` の大きさが、engine に渡した大きさと違えば `sendRaw({ kind: IN.DISPLAY, width, height })`。
5. `app/src/ui/screen-fit.ts:47〜54` `chooseGuestScreen`: 部屋(CSS)× DPR を、engine の上下限で丸める。
6. `lift/runtime/fle_input.c`:
   - 191〜206 `fle_input_host_display`: `FLE_IN_DISPLAY` の入力を queue に積む。
   - 777〜791 `apply_display`: `fle_screen_set` の後、全トップレベル窓へ `WM_DISPLAYCHANGE` と `WM_SETTINGCHANGE`、続けて `fle_win_input_remaximize`。DPI は変えない(`:786` の注釈)。
   - 893 の呼び出し。
7. `lift/runtime/fle_screen.h:84〜89` `fle_screen_set`: 数値を変えるだけ(確保も解放もしない)。

停止の型と終了コード
8. `lift/runtime/fle_thread.c:939〜980` `__wrap_pthread_mutex_lock`: 同じ guest スレッドが保持している libc の mutex を取ろうとすると、`the mutex is held by this guest thread ... a release was skipped` で `fle_stop`。`:979` がその文面。
9. `lift/runtime/fle_env.c:66〜70` `fle_stop`: `fle: <api>: <why>` を標準エラーに出し、`abort()`。
10. `lift/harness/fruity.c`:
    - 645〜652 `report()`: 先頭で `if (in_report) _exit(5);`。
    - 770 の行: 終了コード一覧(134 SIGABRT、139 SIGSEGV、…、6 = 名前つき停止。5 の記載は無い)。
    - 787〜790 `on_abort_wasm`: `report(..., 134)`。
11. `lift/harness/web/link.rsp:26` `MAXIMUM_MEMORY=4294967296`(4 GiB)、`:36` `--wrap=abort`、`:37` `--wrap=pthread_mutex_lock`。
12. `lift/runtime/fle_imports.c:446〜470` `LocalFree`(#1382): 生きていないハンドルは解放せず、戻り値で返す。

## 実測(記録の中。出典つき)

- `.tmp-art2/1509/judge.txt`(作業者の判定の出力。どの run の出力かは、ファイルからは確認できない):
  - 15956 行: `page/dpr-change-keeps-drawing (4k-dpr1, DPR 1): ok true, engine engine-idle, errors 0/0/0, stuck 0`
  - 15978 行: 同 `(4k-random, DPR 2.89): ok true, engine engine-idle`
  - 16000 行: 同 `(4k-random, DPR 1.84): ok true, engine engine-idle`
  - 16024〜16027 行: `page/zoom-*(dpr:1)` が ok(同じ `setRatio` の経路)。
  - 同じ page 行で、報告の page 2/3 の後追いの失敗 8 本(first-frame-drawn・focus-engine・fullscreen-request・history-hash・resize-picture-kept・resize-window-table・visibility-engine・webgl-gl-presenter)は出ていない。よって、judge.txt は報告の page 2/3 の run とは別の run と読むのが妥当(推定)。
- `.tmp-art2/1509/report.md`:
  - 19 行目: page 2/3 は差し戻し。4k-dpr1 は 0 本、4k-random は 9 本。`page/dpr-change-keeps-drawing` で engine が exit code 5 で止まり、残りの 8 本は同じ共有頁の後追い。
  - 62 行目 (f): `DPR 1.8 でエンジンが exit code 5 で止まる(製品の候補、未確認)`。
  - 停止の報告(`fle:` の行)と、engine のログは、このワークツリーに保存されていない(`--engine-logs` を付けていない)。
- `pm-work/findings/e2e-full-2.md:156`(系統 D): `file/export-last-format`(印 `new`)が終了 5・`fle: pthread_mutex_lock: the mutex is held by this guest thread ... libc's mutexes are not recursive`。DPR の変更とは無関係の場面で、同じ型の停止が出ている(実測)。

## 推定

- 停止の型は、`pthread_mutex_lock` の wrapper が見つけた「同じスレッドが保持する libc の mutex」(`fle_thread.c:979`)。libc の mutex(dlmalloc の大域 lock など)を、解放しないまま戻った経路がある、という読み。系統 D と同じ型なので、DPR の変更に固有の原因とは限らない。
- 報告中の再入の可能性: 一度目の停止のあと、報告(`report()`)が `malloc` 等で、同じ lock を取ろうとして、二度目の停止になり、5 で終わった(`fruity.c:652`)。この場合、`lastFle` の mutex の行は報告中の再入の行で、一度目の原因ではない。
- DPR 変更で engine に届く差は、物理の大きさが保たれるため、guest 画面の大きさの変化は丸めの分(1 px 程度)と見込まれる(未測定)。その差でも `apply_display` の `WM_DISPLAYCHANGE` が全窓に届く。その後の窓の再描画・割り当て・解放の経路で、lock を解放しないまま戻った、という経路が候補(未確認)。
- メモリの上限(theme-switch-crash の 2 GiB の系統)は、main では 4 GiB(`link.rsp:26`)に上げてあるので、1.8 の停止の候補としては弱い(推定)。

## 確認できない

- 報告の 1.8 の停止のログ(一度目の `fle:` の行、戻り番地、停止直前の呼び出し)。
- 1.8 での再現(`judge.txt` の基準 1.84 では止まっていない。1.8 の行は、このシナリオの宣言に無いので、`--displays` では走らない)。
- E2E の操作が、実機の拡大率の変更と同じ入力を engine に与えるか。CDP の `Emulation` は `screen.*` を変えない。engine の部屋の大きさは `devicePixelRatio` と部屋の CSS の大きさだけで決まる(`app/src/main.ts:305・308` と `:391〜404`)。実機の移動との差は、この読みでは判断できない。

## 次の一手(Issue の内容)

1. シナリオの宣言に `dpr:1.8` を足し、1.8 の行で `--repeat` を付けて再現する。`--engine-logs` でログを残す。
2. ログの `fle:` の行を、順序どおりに全部見る。一度目の停止(原因の行)と、報告中の再入(二度目)を分ける。
3. 原因の場所で直す。E2E の操作が原因だと分かったら、E2E を直す(根拠を報告に書く)。

起票: 下書き 255(`pm-work/issue-drafts/wave1br/255-dpr-change-engine-stops.md`)を #1541 として作った(Task・High・M・M2 HiDPI と P2 の測定(P2)・親 #1305・labels hidpi・verify・blocked_by なし)。`pm-hold` は起票の最後の段で外された(起動は駆動の設計に任せる。起動の確認は、この機械からは行っていない)。
