# テーマの切り替えの停止(Issue #1382、2026-11-24〜25。試行 3〜5)

## 000. 試行 6(2026-11-25 17:38〜19:10 UTC 頃。run 45621517553 の差分の取り込み): 切り替えは全項目が通った。起動確認が基準外(中央値 13,352 ms)

判定: **差し戻し**(起動確認の 1 項目だけ基準外。他は通った)。

実エンジン: `fe-build engine` と `fe-build bundle`(1 回。17:39〜18:31 UTC、exit 0)、`tools/boot-check/boot-bundle.mjs`(6 回。engine-idle・standIn false・error null)、`tools/theme-switch/measure.mjs`(無指定。DPR 1 と乱数の DPR 2.54。18:33〜18:47 UTC)、同 `--dpr 2.93`(18:51〜19:03 UTC)、`tools/smoke/welcome-flow.mjs`(流れ A〜D 4/4 通過。B・D の `welcomeClosed` true)、`npm test --prefix app`(74 files passed、1455 tests passed)。測定の値は runner の値。この機械とは別枠。nproc 4、loadavg 1.9〜3.5。

### 000.1 衝突の解消

- `lift/runtime/fle_dispatch.h`: 前回の側の `fle_dispatch_miss`(と、その中で呼ぶ `fle_dispatch_miss_stack` の宣言)を外した。main の `fle_dispatch_miss` は `fle_dispatch_stop` を呼び、呼び出し元の戻り番地・レジスタ・近い関数名を出す。前回の 8 語の表示は、この情報と同じだった。
- `lift/runtime/fle_env.c`: 前回の `fle_dispatch_miss_stack`(参照が無くなった)を外し、HEAD の版に戻した。

### 000.2 受け入れの結果

| 項目 | 基準 | 結果 | 判定 |
|---|---|---|---|
| 起動確認(DPR 1、3 回) | 中央値 ≤ 12,500、3 回とも engine-idle | 13,520 / 13,352 / 12,582(中央値 13,352) | 差し戻し |
| 切り替え(DPR 1) | 13/13、停止 0、提示が増え、画面が変わる | 13/13、停止 0、`elapsedMs` 12,270 | 受理 |
| 切り替え(乱数の DPR 2.54) | 同上。上限 12,500 × 2.54 | 13/13、停止 0、14,986 | 受理 |
| 切り替え(DPR 2.93。`--dpr`) | 同上(参考。上限 36,625) | 13/13、停止 0、18,053。キャンバス 5379×3094、WebGL の文脈は作れた | 受理(参考) |
| 流れ A〜D | 4/4 | 4/4 | 受理 |
| `npm test` | 通過 | 74 files passed、1455 tests passed | 受理 |
| `preBootBytes` | ≤ 314,368,711 | 313,191,290 | 受理 |

起動の追加の 3 回(4〜6 回目): 12,326 / 12,319 / 13,382。6 回の中央値は 12,967 ms。

### 000.3 切り替えの表(13 件。提示は `presentsBefore`→`presentsAfter`。背景は主窓の色)

| # | テーマ | DPR 1 提示 | DPR 2.54 提示 | DPR 2.93 提示 | 画面 | DPR 1 背景 | DPR 2.54 背景 | DPR 2.93 背景 |
|---|---|---|---|---|---|---|---|---|
| 1 | DEDZII Light | 260→273 | 84→90 | 78→83 | 変わった | #34444e→#b0aca1 | #3c454b→#bab9ae | #3c454b→#bab9ae |
| 2 | DEDZII | 396→410 | 131→138 | 121→127 | 変わった | #b0aca1→#d7e4f2 | #bab9ae→#d7e4f2 | #bab9ae→#d7e4f2 |
| 3 | PPRIIME Dark | 530→543 | 175→179 | 160→165 | 変わった | #d7e4f2→#f0f0f0 | #d7e4f2→#f0f0f0 | #d7e4f2→#f0f0f0 |
| 4 | PPRIIME Light | 674→687 | 218→224 | 198→203 | 変わった | #f0f0f0→#dad5b0 | #f0f0f0→#d5d2bc | #f0f0f0→#d5d2bc |
| 5 | Veela | 818→832 | 262→267 | 237→241 | 変わった | #dad5b0→#310c50 | #d5d2bc→#331f46 | #d5d2bc→#331f46 |
| 6 | Ash | 961→973 | 309→317 | 273→279 | 変わった | #310c50→#444444 | #331f46→#444444 | #331f46→#444444 |
| 7 | Dark orange | 1101→1115 | 361→366 | 311→316 | 変わった | #444444→#293134 | #444444→#2d3132 | #444444→#2d3132 |
| 8 | Dark | 1240→1252 | 408→414 | 355→360 | 変わった | #293134→#22272b | #2d3132→#25272a | #2d3132→#25272a |
| 9 | Light cherry | 1341→1354 | 459→465 | 399→404 | 変わった | #25272a→#e9e9e9 | #25272a→#e9e9e9 | #25272a→#e9e9e9 |
| 10 | Light pear | 1446→1457 | 507→514 | 438→444 | 変わった | #e9e9e9→#e9e9e9 | #e9e9e9→#e9e9e9 | #e9e9e9→#e9e9e9 |
| 11 | Light tangerine | 1548→1560 | 556→563 | 478→482 | 変わった | #e9e9e9→#efefef | #e9e9e9→#efefef | #e9e9e9→#efefef |
| 12 | Sytrus | 1648→1661 | 600→605 | 511→516 | 変わった | #efefef→#3399ff | #efefef→#3399ff | #efefef→#3399ff |
| 13 | Ultra green | 1763→1777 | 635→642 | 543→547 | 変わった | #3399ff→#222222 | #3399ff→#242424 | #3399ff→#242424 |

### 000.4 停止

- 39 件の切り替え(DPR 1 の 13、2.54 の 13、2.93 の 13)で停止は 0。mutex の再入の停止(試行 3〜4 の段階 3。戻り番地 `0x44633b`、mutex `0x10f5711c`)は出なかった。
- 差分の mutex の包み(`lift/runtime/fle_thread.c` の `__wrap_pthread_mutex_*`)は、再入を見つけたら戻り番地つきで止めるだけで、解放の経路は直していない。再現しなかったので、解放の経路の特定は未確認。
- DIB の保留の解放(`lift/runtime/fle_gdi.c` の `GF_DELETE_PENDING`)は入っている。
- 試行 4 の DPR 2.93 の「WebGL で表示されない」は、この run では出なかった(文脈は作れた)。原因は未確認。

### 000.5 起動が遅い理由(未確認)

- 試行 5(13:16〜16:45 UTC)の起動は 8,170・9,156・9,187 ms(中央値 9,156)。試行 6 の DPR 1 の起動は、中央値 13,352 ms(3 回)と 12,967 ms(6 回)。
- 試行 5 から今回までに main に入った変更がある(#1402 の上部バーの表示、#1416 の停止の表示、#1418 の返り値の続行、#1421 の間接呼び出し、`boot-bundle` の 2 回の起動 16a11357 など)。今回の差分(`CompareStringOrdinal`・`:` の判定・mutex の包み・4 GiB の上限・`missing-proc` の表示)との切り分けは、次の節の計測で行う。

## 00. 試行 5(2026-11-25 13:16〜16:45 UTC): 受け入れの全項目が通った(DPR 2.97 の停止を直した)

実エンジン: `fe-build engine` と `fe-build bundle`(4 回。13:20〜13:55 の版、14:24〜15:02 の診断版(上限 2 GiB)、15:18〜15:43 の失敗(00.3 の 2)、15:44〜16:22 の最終版。exit 0 は 3 回目以外)、`tools/boot-check/boot-bundle.mjs`(3 回。engine-idle・standIn false・error null。`elapsedMs` 8,170・9,156・9,187、中央値 9,156)、`tools/theme-switch/measure.mjs`(無指定。DPR 1 と乱数の DPR 1.26。16:32〜16:41 UTC)、`tools/theme-switch/measure.mjs --dpr 2.97`(16:22〜16:32 UTC)、`tools/smoke/welcome-flow.mjs`(流れ A〜D 4/4 通過)、`npm test --prefix app`(通過)。測定の値は runner の値。この機械とは別枠。nproc 4、loadavg 1.4〜2.0。

### 00.1 停止の全文(試行 5 の、上限 2 GiB の版で DPR 2.97 を測った時。切り替えの 3 件目 `PPRIIME Dark`)

```
Cannot enlarge memory, requested 2148593664 bytes, but the limit is 2147483648 bytes!
Cannot enlarge memory, requested 2148597760 bytes, but the limit is 2147483648 bytes!
fle: CreateDIBSection: out of memory (guest return address 0x6b9063): live handles 190; DIB 5 blocks 96204096 bytes (pending delete 0/0, selected 5/96204096); DDB 24 blocks 5472 bytes; window surfaces 3 blocks 108074504 bytes; malloc in use 1059249800 bytes, free 49438776 bytes
fruity: exit code 134
```

最後の `fle:` の行は、この試行で足した診断(`fle_gdi.c` の `mem_why`。停止の時だけ出る 1 行)。戻り番地 `0x6b9063` は `fle_imp_CreateDIBSection` の呼び出し元(`lift/out-app/gen/wasm-func-names.tsv` で引く。未確認)。

### 00.2 原因(実測と、そこからの推定)

- **実測**: 停止の時、解放の保留の DIB は 0 個(`pending delete 0/0`)。試行 4 の DIB の解放漏れは、この停止の原因ではない(試行 4 の直しで、保留は解放されていた)。
- **実測**: wasm のリニアメモリの上限は 2 GiB(`link.rsp` の `MAXIMUM_MEMORY`)で、停止の時に 2.14 GB に届いていた。内訳は malloc の使用 1.06 GB(VirtualAlloc の arena 512 MB(`fle_sys.c` の `FLE_VMEM_ARENA_BYTES`。1 回の `aligned_alloc`)、窓の面 108 MB、DIB 96 MB を含む)と、malloc の外の約 0.95 GB。
- **実測**: malloc の外は、`-sGLOBAL_BASE=989659136`(`link.rsp`)で、静的データとスタック(`__stack_pointer` 1,015,021,888)を約 992 MB より上に置いた分。それより下は、ゲストの固定アドレス(イメージ等)の領域として使う(`fle_lowregs.h` の注釈)。このため、ヒープに使えるのは約 1.1 GB しかない。
- **推定**: 試行 4 の DPR 2.85 の合格は、この余裕の中の余裕。DPR 2.93 と 2.97 は、同じ位置(切り替え 3 件目)で上限に届く。大きい面(画面の大きさ 5453×3136 ≈ 68 MB)と DIB が、断片化したヒープに入らない(空きは計 49 MB で、24.8 MB の連続した塊が取れない)。
- **未確認**: 切り替えごとの増加量。`selected 5/96204096`(DC に選ばれたままの DIB が 5 個)が、切り替えで増えるかは測っていない。

### 00.3 直した方式(既定の動作。ビルド変数は増やさない)

1. `lift/harness/web/link.rsp`: `-sMAXIMUM_MEMORY=2147483648` を `4294967296`(wasm32 の上限の 4 GiB)にする。
2. `lift/crates/fe-wasmgen/src/lib.rs`: 結合の時に取り込む memory の上限を 32768 → 65536 ページ(C 部分の上限と一致させる。一致しないと `wasm-merge` が `maximal size 65536 is larger than expected maximal size 32768` で失敗する。1 の変更だけの試行の失敗はこれ)。
3. `app/src/perf/stats.ts`: `WASM_MEMORY_LIMIT_BYTES` を 4096 MB にする(表示の割合の分母)。単体テスト `app/test/unit/perf-beacon.test.ts`・`perf-panel.test.ts` の値を合わせる。`app/src/engine/filedlg.ts` の注釈の「2 GiB」を「4 GiB」にする(384 MiB の上限は変えない)。
4. 停止の 1 行(`fle_gdi.c` の `mem_why`・`fle_raster.c` の `fle_raster_mem`)を残す。停止の時だけ出る。

理由: 2 GiB の中で、ゲストの固定の領域と VirtualAlloc の arena と画面サイズの面が、DPR 2.9 以上で上限を超える。上限の引き上げは、製品の設定の変更なので、報告で示す(依頼者の判断の対象)。

### 00.4 測定(試行 5 の最終版)

- **DPR 2.97(停止なし)**: `error` null、`stopped` false、13/13、`elapsedMs` 11,433、`preBootBytes` 312,236,070、キャンバス 5453×3136。全件で提示が増え、画面が変わった(表 00.5 の下)。
- **受け入れの行(DPR 1)**: 13/13、停止 0、`elapsedMs` 8,126。**乱数の DPR 1.26**: 13/13、停止 0、`elapsedMs` 8,378。どちらも全件で提示が増え、画面が変わった。
- 起動(`boot-bundle`、DPR 1 の 3 回): 8,170・9,156・9,187 ms(中央値 9,156)。流れ A〜D 4/4 通過。`npm test` 通過。

### 00.5 13 件の切り替え(DPR 1 の行。画面は全件で変わった)

| # | テーマ | 提示 前→後 | 画面 | 背景色 前→後 |
|---|---|---|---|---|
| 1 | DEDZII Light | 303→318 | 変わった | #34444e→#b0aca1 |
| 2 | DEDZII | 462→475 | 変わった | #b0aca1→#d7e4f2 |
| 3 | PPRIIME Dark | 618→631 | 変わった | #d7e4f2→#f0f0f0 |
| 4 | PPRIIME Light | 770→782 | 変わった | #f0f0f0→#dad5b0 |
| 5 | Veela | 927→940 | 変わった | #dad5b0→#310c50 |
| 6 | Ash | 1084→1099 | 変わった | #310c50→#444444 |
| 7 | Dark orange | 1237→1246 | 変わった | #444444→#293134 |
| 8 | Dark | 1381→1392 | 変わった | #293134→#22272b |
| 9 | Light cherry | 1487→1500 | 変わった | #25272a→#e9e9e9 |
| 10 | Light pear | 1598→1610 | 変わった | #e9e9e9→#e9e9e9(変わらない) |
| 11 | Light tangerine | 1710→1723 | 変わった | #e9e9e9→#efefef |
| 12 | Sytrus | 1824→1840 | 変わった | #efefef→#3399ff |
| 13 | Ultra green | 1941→1954 | 変わった | #3399ff→#222222 |

DPR 2.97 の行(停止なし。全件で提示が増え、画面が変わった。提示 前→後): 1 DEDZII Light 78→85、2 DEDZII 119→124、3 PPRIIME Dark 162→166、4 PPRIIME Light 201→205、5 Veela 245→249、6 Ash 286→290、7 Dark orange 328→333、8 Dark 370→374、9 Light cherry 411→417、10 Light pear 466→472、11 Light tangerine 516→522、12 Sytrus 550→556、13 Ultra green 585→589(背景色は、Light pear が変わらない以外は表 00.5 と同じ)。

### 00.6 未確認と、次にやること

1. 4 GiB の上限が、利用者のブラウザで使えるか(wasm32 の 4 GiB は Chrome・Firefox 系で確認の価値あり。Safari・iOS の実際の上限は未確認)。
2. 切り替えごとのメモリの増加(00.2 の未確認)。切り替えを多く(例: 30 回)繰り返した時の上限への近さを測る。
3. 乱数の DPR の上端(3.00 に近い値)は、2.97 の 1 回しか測っていない。
4. 試行 4 の「2.93 の停止は DIB のリーク」という推定は、この試行の実測(保留の DIB が 0)と合わない。停止の原因は上限の余裕の不足だった。

### 00.7 試行 5 の差分(報告の対象)

- `lift/harness/web/link.rsp`・`lift/crates/fe-wasmgen/src/lib.rs`: 上限の 4 GiB(`lift/` の変更は `deploy.yml` を動かす)。
- `app/src/perf/stats.ts`・`app/src/engine/filedlg.ts`・`app/test/unit/perf-beacon.test.ts`・`app/test/unit/perf-panel.test.ts`: 上限の数字と注釈と試験。
- `lift/runtime/fle_gdi.c`・`fle_raster.c`: 停止の 1 行の診断(`mem_why`・`fle_raster_mem`)。
- 測定の記録: `pm-work/findings/theme-switch-crash-run.jsonl` に、試行 5 の 3 行(DPR 1、1.26、2.97)を足した。

## 0. 試行 4(2026-11-25 08:02〜11:32 UTC): 判定は差し戻し

判定: **差し戻し**。受け入れの 2 行のうち、DPR 1 の行は全項目を満たした。乱数の DPR 2.85 の行は、停止 0・13/13 切り替え・全件で画面が変わった(前回の停止は消えた)が、`elapsedMs` が 17,807(上限 12,500)で落ちた。

実エンジン: `fe-build engine`(10:22〜11:12 UTC、約 50 分。exit 0)、`fe-build bundle`(exit 0)、`tools/boot-check/boot-bundle.mjs`(`engine-idle`、`standIn` false、`error` null、`elapsedMs` 13,564)、`tools/theme-switch/measure.mjs`(無指定。DPR 1 と乱数の DPR 2.85。11:12〜11:27 UTC)、`tools/smoke/welcome-flow.mjs`(流れ A〜D 4/4 通過、B・D の `welcomeClosed` true)、`npm test --prefix app`(exit 0)。測定の値は runner の値。この機械とは別枠。nproc 4、loadavg 1.6〜1.8。

### 0.1 停止の全文(試行 4 の前の、診断入りの DPR 2.93 の測定。切り替えの 4 番目 `PPRIIME Light`)

```
fle: DIB section made 24883200 bytes (live 7 blocks, 145006504 bytes; guest return address 0x6b072f)
Cannot enlarge memory, requested 2163200000 bytes, but the limit is 2147483648 bytes!
Cannot enlarge memory, requested 2163204096 bytes, but the limit is 2147483648 bytes!
fle: CreateDIBSection: out of memory
```

exit 134(`stopped 134`)。停止の時点の大きいブロック: DIB 7 個・145 MB、ウィンドウの面 3 個・106 MB(診断入りの版で数えた)。wasm の上限は `MAXIMUM_MEMORY=2147483648`(`lift/harness/web/link.rsp`)。

### 0.2 原因(実測と推定を分ける)

**実測**(診断入りの版の `fle: diag` 行と、大きいブロックの記録。DPR 1 と DPR 2.85 の両方で同じ):

- 切り替え `PPRIIME Dark`・`PPRIIME Light`・`Veela` の 3 つで、24,883,200 bytes(2880×2160×4)の DIB が、切り替えごとに 1 個、解放されずに残った。他のテーマでは増えない。DIB の増え方は DPR に依らない(テーマの画像の大きさ)。
- 残った DIB の解放の試み: `DeleteObject DIB ... (state 1, sel 1, ret 0x6aff79)`。`sel 1` は、DIB が DC に選ばれたまま削除を呼んだこと。`DeleteObject` は選択中のオブジェクトに 0 を返す(Learn の規則。`fle_imp_DeleteObject`)。戻り番地 `0x6aff79` は関数 `f_6aff60` の `call f_424090`(`DeleteObject`)の直後。`f_6aff60` は、`DeleteObject` の後で `f_424070` を呼び、その後でハンドルの欄を 0 にする(`lift/out-app/gen/src/fle-03.c` の `L_6aff74`〜`L_6affe9`)。
- 同じ DIB は、DC を `DeleteDC` で解放した後も残るため、後の `DeleteObject` も 0 を返すことはない(呼ばれない: ハンドルは捨てられている)。

**推定**(強い根拠があるが、未確認):

- `f_424070` は `DeleteDC`(DC の解放)。`f_6aff60` の順序は「ビットマップを先に削除し、DC を後で削除する」。Windows の規則では、選択中のビットマップの削除は失敗するので、実機の FL でも同じ DIB は残る(解放漏れ)。実機の FL で、この解放漏れが目立つかは未確認(メモリが大きいため)。
- wasm の上限 2 GiB と、初期の 1.39 GB(`INITIAL_MEMORY`)のため、この解放漏れは 2 GiB の上限に届く(DPR 2.93 で 4 切り替え目で届いた。DIB 145 MB と面 106 MB の他に、起動の後の heap が大きい)。

### 0.3 直し方(既定の動作。オプションは作らない)

`lift/runtime/fle_gdi.c`:

- `DeleteObject` の戻り値は Windows どおり 0 のまま(選択中)。ただし、選択中のビットマップには `GF_DELETE_PENDING` を付ける。
- その DC を `DeleteDC` で解放する時(`dc_destroy`)に、保留のビットマップがもう選択されていなければ、`bitmap_pending_free` で実体(画素)と枠を解放する。
- 解放は `DeleteDC` の時だけにした(`SelectObject` で外す時には解放しない)。外された古いハンドルは、呼び出し側が受け取るので、その時点で消すと意味が変わるため。

理由: ゲストが削除を試みた時点で、ハンドルは捨てられている(`f_6aff60` がハンドルの欄を 0 にする)。Windows ではこの実体が残るが、wasm のヒープには余裕がない。

### 0.4 測定(受け入れの 2 行)

| # | テーマ | DPR 1: 提示 前→後・背景 前→後・画面 | DPR 2.85: 提示 前→後・背景 前→後・画面 |
|---|---|---|---|
| 1 | DEDZII Light | 271→285・#34444e→#b0aca1・変わった | 78→84・#3c454b→#bab9ae・変わった |
| 2 | DEDZII | 400→413・#bab9ae→#d7e4f2・変わった | 118→123・#bab9ae→#d7e4f2・変わった |
| 3 | PPRIIME Dark | 531→545・#d7e4f2→#f0f0f0・変わった | 160→165・#d7e4f2→#f0f0f0・変わった |
| 4 | PPRIIME Light | 668→683・#f0f0f0→#dad5b0・変わった | 205→209・#f0f0f0→#d5d2bc・変わった |
| 5 | Veela | 818→832・#dad5b0→#310c50・変わった | 245→249・#d5d2bc→#331f46・変わった |
| 6 | Ash | 955→968・#310c50→#444444・変わった | 282→288・#331f46→#444444・変わった |
| 7 | Dark orange | 1092→1104・#444444→#293134・変わった | 327→332・#444444→#2d3132・変わった |
| 8 | Dark | 1224→1235・#293134→#22272b・変わった | 367→373・#2d3132→#25272a・変わった |
| 9 | Light cherry | 1322→1335・#25272a→#e9e9e9・変わった | 410→416・#25272a→#e9e9e9・変わった |
| 10 | Light pear | 1421→1430・#e9e9e9→#e9e9e9・画面は変わった | 452→457・#e9e9e9→#e9e9e9・画面は変わった |
| 11 | Light tangerine | 1515→1524・#e9e9e9→#efefef・変わった | 493→498・#e9e9e9→#efefef・変わった |
| 12 | Sytrus | 1614→1629・#efefef→#3399ff・変わった | 527→533・#efefef→#3399ff・変わった |
| 13 | Ultra green | 1720→1733・#3399ff→#222222・変わった | 565→570・#3399ff→#242424・変わった |

- DPR 1: `error` null、`stopped` false、`switched` 13/13、`elapsedMs` 11,691、`preBootBytes` 312,224,801。
- DPR 2.85: `error` null、`stopped` false、`switched` 13/13、`elapsedMs` **17,807**、`gl.canvas` 5233×3010、`gl.limits` は `maxTexture` 8192・`maxRenderbuffer` 8192・`maxViewport` [8192, 8192]、`contextAtCanvasSize` true(WebGL の文脈は作れる。黒い画面ではない)。

### 0.5 受け入れの判定(試行 4)

- 道具の 2 行: `error`・`stopped`・`themes`・`switched` は全部一致(DPR 1 と 2.85)。
- 各テーマの `presentsAfter > presentsBefore` と `screenChanged` は全件 true。
- `preBootBytes` ≤ 314,368,711: 通過(312,224,801)。
- **`elapsedMs` ≤ 12,500: DPR 1 は通過(11,691)、DPR 2.85 は落ちた(17,807)**。
- 流れ A〜D: 通過。`npm test`: 通過。`boot-bundle`: 通過(`elapsedMs` 13,564 は起動の計測。判定の対象ではない)。

**未確認(差し戻しの理由)**: 起動から `engine-idle` までの時間が、キャンバスの大きさに比例して延びる(DPR 1: 約 11.7 s、DPR 2.85: 17.8 s、DPR 2.93: 17.6 s)。停止とは別の問題で、今回は直していない。受け入れの上限 12.5 s を乱数の DPR(〜3.00)で守るかは、PM の判断が要る(P1 の 10 秒とも関係する)。

### 0.6 段階の整理(試行 4 まで)

- 段階 1(`CompareStringOrdinal`): 直した(試行 3)。
- 段階 2(途中の `:`): 直した(試行 3)。
- 段階 3(mutex の再入): 原因は試行 3 の `LocalFree`(無効な値の解放が dlmalloc の大域 lock を取ったまま戻る)。試行 4 で、wrapper の再入を「黙って数えて続行」から「停止(戻り番地つき)」に変えた。停止は、試行 4 の 4 回の測定(DPR 1 ×2、DPR 2.85、DPR 2.93)で出なかった。
- 段階 4(新): DIB の解放漏れ(`PPRIIME` 系の切り替え)が、DPR 2.85 以上で wasm の上限に届く。0.3 で直した。

### 0.7 製品の変更(試行 4)

- `lift/runtime/fle_gdi.c`: `GF_DELETE_PENDING`・`bitmap_pending_free`・`dc_destroy`・`DeleteObject`。
- `lift/runtime/fle_thread.c`: mutex の wrapper(再入は停止)。
- `lift/runtime/fle_env.c`・`fle_env.h`・`fle_raster.c`・`fle_imports.c`・`fle_vfs.c`・`fle_glob.*`・`fle_locale.*`・`fle_modules.c`・`fle_dispatch.h`(前回までの変更。診断の行は残る)。
- `lift/crates/fe-lift/src/imports_table.rs`(`CompareStringOrdinal` の行。衝突は両方の行を残して解消)、`lift/harness/web/link.rsp`(`--wrap` の 2 行)。
- `lift/` の変更は `deploy.yml` を動かす(報告に書く)。`app/` は変更なし。
- 測定の道具 `tools/theme-switch/measure.mjs`: タイルはスクロールのつまみのドラッグで押す(13 件とも押せた)。`gl`(キャンバスの大きさ・WebGL の上限)を JSON に出す。

### 0.8 次にやること

1. 起動の時間(高い DPR で `engine-idle` まで 17 s 超)。どの処理が時間を使うかを測る(キャンバスの面の確保・初回の描画・GL の転送)。PM が上限(12.5 s を DPR 上限で守るか)を決める。
2. 解放漏れの元(ゲストの `f_6aff60` の順序)は、Windows の FL にも同じ順序があるはず。実機の確認は未確認。
3. `FormatMessageW` の失敗の番号は、試行 4 の測定では確認していない(停止のない回は、エンジンのログを取っていない)。

## 1〜8. 試行 3(2026-11-25 04:34〜07:00 UTC)

判定: **受け入れの測定は通った(DPR 1 と乱数の DPR 1.08 の 2 行とも、13/13・停止 0・全件で画面が変わった。06:52〜07:00 UTC)**。ただし、DPR 2.93 の停止(wasm のメモリ不足)は未修正。乱数の DPR が大きい値になると、同じ停止で落ちる(2 の 2.93 の結果から推定)。

実エンジン: `fe-build engine`(04:34〜05:06 UTC、32 分。exit 0)、`fe-build bundle`(exit 0)、`tools/boot-check/boot-bundle.mjs`(`engine-idle`、`standIn` false、`error` null、`elapsedMs` 12,578)、`tools/theme-switch/measure.mjs --dpr 1`(06:46 UTC)・`--dpr 1.23`(06:23 UTC)・`--dpr 2.93`(06:40 UTC)、`tools/smoke/welcome-flow.mjs`(流れ A〜D すべて通過。06:48 UTC)、`npm test --prefix app`(71 files passed、1406 tests passed)。測定の値は runner の値。この機械とは別枠。nproc 4、loadavg 1.4〜2.6。

## 1. 停止の全文(前回の試行の最後の測定。DPR 1、テーマ `DEDZII Light`)

```
fle: pthread_mutex_lock 0x10f5711c taken again by the thread that holds it (re-entry 13, locks so far 2169560, taken through the wrapper)
fle:   first taken at: rsp 0x13135c88, the words at it: 0x44633b 0xffffff36ffffff36 0x40f97c 0x1
fle:   taken again at: rsp 0x13135c88, the words at it: 0x44633b 0xffffff36ffffff36 0x40f97c 0x2
```

前回の記録(`theme-switch-crash-run.jsonl` の旧版)では、同じ mutex が 16 回続けて再入し、その後に exit 134。

## 2. 原因の特定

**実測**(バイナリとソースを読んだ結果。この試行の実行で再現はしていない):

- 戻り番地 `0x44633b` は関数 `f_446290`(VA `0x446290`。`objdump -d` で確認)の中の `call 0x4239e0` の直後。`0x4239e0` は import の thunk(`jmp [IAT]`)。`f_446290` は、`FormatMessageW`(`dwFlags` `0x3300|0x800`、`FROM_SYSTEM | IGNORE_INSERTS | ALLOCATE_BUFFER`)を呼び、末尾の空白と `.` を除き、その後 `LocalFree(lpBuffer)` に当たる thunk を呼ぶ。`0x40f97c` は、この関数を呼ぶ `0x40f960` の中の戻り番地。
- runtime: `fle_imp_LocalFree` は `free((void *)R[1])` を呼ぶ(`lift/runtime/fle_imports.c`)。有効なブロックかどうかを見ない。
- dlmalloc(emscripten の `system/lib/dlmalloc.c`): `USE_LOCKS` の mutex は `pthread_mutex_t`(`MLOCK_T`)。release 版の `USAGE_ERROR_ACTION` は `ABORT` = `__builtin_unreachable()`。不正な解放は、解放の途中で戻り、mutex を解放しないまま進む。
- したがって、mutex `0x10f5711c` は **dlmalloc の大域 mutex**(libc の pthread。Win32 の CriticalSection や mutex ではない)。前回の決まりの 2(Win32 の再帰の規則)は、この停止には当てはまらない。

**推定**(強い根拠はあるが、この試行では未確認):

- `FormatMessageW` が失敗する経路(テーブルに無い message id)では、`lpBuffer` は触られない(`lift/runtime/fle_imports.c` の decision 0129 の注記)。ゲストは失敗後も `LocalFree(lpBuffer)` を呼ぶ。`lpBuffer` は、前回の呼び出しで解放済みの値(二重解放)か、無関係な値。
- その `free` が不正な解放になり、mutex を解放しないまま戻る。その後の `malloc`/`free` は、すべて同じ thread からの再入になる(`locks so far` が増えない。前回の記録)。最後に abort(134)。
- 失敗した message id は未確認(新しい診断行 `fle: FormatMessageW: message N is in none of the tables` は、この試行の停止の記録には出ていない。停止の末尾 200 行だけを保存しているため、出たかどうかは確認していない)。

## 3. 直した方式(既定の動作。オプトインは作らない)

1. **`LocalFree` は、有効な固定ブロックだけを解放する**(`lift/runtime/fle_glob.c`・`fle_glob.h`・`fle_imports.c`)。`LocalAlloc` と `FormatMessageW`(ALLOCATE_BUFFER)のブロックを、`GlobalAlloc` と同じ固定ブロックの表に登録する(Win32 では、ローカルとグローバルの固定ブロックは同じヒープ)。登録されていない値は解放せず、MSDN の規則どおり `hMem` を返す(失敗の値)。`GetLastError` は `ERROR_INVALID_HANDLE`(6)(ページが番号を書いていないので、推定)。
2. **mutex の wrapper は、取得の記録を常に動くコードに残さない**(`lift/runtime/fle_thread.c`)。取得ごとの記録(rsp の 4 語)と取得の数の集計を外した。同じ thread の再入は、数を上げるだけ。再入の表示は最初の 4 行まで(rsp と、その位置の戻り番地)。
3. `FormatMessageW` の失敗(テーブルに無い message id)は、最初の 8 回まで診断の行を出す(`fle_imports.c`)。失敗の値の扱いは変えない(decision 0129 のまま)。
4. 段階 1・2(前回の試行。`CompareStringOrdinal`、途中の成分の `:`)は、そのまま残す。診断の `fle_dispatch_miss_stack`・`missing-proc` も残す。

## 4. 13 件の切り替え(実測)

全件の JSON は `pm-work/findings/theme-switch-crash-run.jsonl`(DPR 1・1.23・2.93 の 3 行)。`presentsBefore→After` は、押した後の提示の数。`screen changed` は、押す前後の画素の hash が違うこと。

**DPR 1(06:46 UTC、`switched` 13/13、停止 0)**

| # | テーマ | 提示 前→後 | 画面 | 背景色 前→後 |
|---|---|---|---|---|
| 1 | DEDZII Light | 223→233 | 変わった | #3c454b→#b0aca1 |
| 2 | DEDZII | 357→370 | 変わった | #b0aca1→#d7e4f2 |
| 3 | PPRIIME Dark | 494→504 | 変わった | #d7e4f2→#f0f0f0 |
| 4 | PPRIIME Light | 627→640 | 変わった | #f0f0f0→#dad5b0 |
| 5 | Veela | 771→783 | 変わった | #dad5b0→#310c50 |
| 6 | Ash | 906→917 | 変わった | #331f46→#444444 |
| 7 | Dark orange | 1036→1047 | 変わった | #444444→#293134 |
| 8 | Dark | 1169→1180 | 変わった | #293134→#22272b |
| 9 | Light cherry | 1268→1280 | 変わった | #25272a→#e9e9e9 |
| 10 | Light pear | 1367→1378 | 変わった | #e9e9e9→#e9e9e9(**変わらない**) |
| 11 | Light tangerine | 1469→1478 | 変わった | #e9e9e9→#efefef |
| 12 | Sytrus | 1567→1579 | 変わった | #efefef→#3399ff |
| 13 | Ultra green | 1671→1683 | 変わった | #3399ff→#222222 |

**DPR 1.23(06:23 UTC、13/13、停止 0)**: 全件で画面が変わった。背景色は、Light pear(#e9e9e9→#e9e9e9)だけ変わらない。表は JSON を参照。

**DPR 2.93(06:40 UTC、13 件に押せた。停止 2 回)**: 停止は 2 回。1 回目は `PPRIIME Dark`(13 件のうち 3 件目)の後(`fle: CreateDIBSection: out of memory`)、2 回目は `Light tangerine`(11 件目)の後(`fle: GetDC: out of memory`)。停止の後は、新しいエンジンで続きを押した(セッション 0: 1〜3、セッション 1: 4〜11、セッション 2: 12〜13)。`Light tangerine` の画面は変わらない(停止と同時)。

## 5. DPR 2.93 の切り分け(受理ではない)

- **「the desktop is not shown with WebGL」は、測定の道具の問題**: 道具が `peek`(GL の画面の読み戻し)を使っていた。製品は WebGL2 で描いている(`gl` の probe: キャンバス 5379×3094 で WebGL2 の文脈が作れ、`MAX_TEXTURE_SIZE` 8192、`MAX_RENDERBUFFER_SIZE` 8192、`MAX_VIEWPORT_DIMS` 8192)。道具は、非 peek の dump に変えた(WebGL の無い表示方式でも読める)。
- **停止の原因は、製品のメモリ**(実測): `fle_gdi.c` の `CreateDIBSection` と `GetDC` の `calloc` が失敗して `fle_stop`。画面サイズのビットマップは 5379×3094×4 ≈ 66 MB。wasm の上限は 2 GiB(`link.rsp` の `MAXIMUM_MEMORY`)。テーマの切り替えで、画面サイズの DIB が解放されずに増える、またはヒープが断片化する(**未確認**。`DeleteObject` は選択中の DIB を解放しない。どの DIB が残るかは測っていない)。
- **未修正**。ブラウザの上限の問題ではない(文脈は作れている)。直し方は、次の決めが要る: (a) wasm の上限の引き上げ(ビルド変数の変更。製品の設定の変更)、(b) 残る DIB の解放(切り替えの経路の調査)、(c) 画面の大きさの上限(拡大してぼやける。PM の判断)。

## 6. 測定の道具(`tools/theme-switch/measure.mjs`)

- タイルの位置: ホイールは DPR 1.23 でグリッドに効かなかった(画素が変わらない。CDP の `deltaY` は、ブラウザ側で DPR で割られ、120 が 97.6 になる。製品の `WheelAccumulator`(100 px = 1 ノッチ、切り捨て)は、0.976 ノッチを捨てる。道具で 100×DPR を送っても効かなかった。**原因は未確認**。製品の `tracker.ts` の挙動の疑い。道具は直さず、ホイールを使わない)。
- 代わりに、**縦スクロールバーのつまみを中央からドラッグ**して上端・下端へ動かす(クリックと同じ入力経路。DPR に依らず効いた)。上端と下端の画面の差(`scrollShift`、重なり 40% 以上)で最大のずれを測る(DPR 1.23 で 306 px)。Sytrus と Ultra green も押せるようになった。
- 画面の読み(`dumpNow`): 非 peek。`ask again` は再試行する。
- 入力の後は、画面が落ち着く(同じ hash が 2 回続く)まで読み直す(描画が遅れるため)。
- `gl`(キャンバスの大きさ・WebGL2 の上限・文脈の可否)を、1 回の起動で JSON に出す。
- `--dpr R`(試験用。1 回だけ走らせる。受け入れの既定の動作は変えない)。

## 7. 未確認と、次の試行でやること

1. **DPR 2.93 のメモリ**: 上の 5 の (a)〜(c) を、PM に決めてもらう。先に、切り替えの前後で `CreateDIBSection` と `DeleteObject` のバイト数を集計する診断を入れて、残る DIB を特定する。
2. **`FormatMessageW` の失敗の message id**: 停止の記録の末尾 200 行では確認していない。診断の行(8 行まで)を、次の実行の記録で読む。失敗が多いなら、テキストを足す(Learn のページ)。
3. **ホイールが DPR≠1 で効かない原因**(製品の `tracker.ts` か、CDP の性質か)。未確認。
4. **背景色**: Light pear は Light cherry と同じ `#e9e9e9`(テーマの値の確認は未)。
5. 乱数の DPR の受け入れは、DPR 2.5 を超える値で、停止が起きる見込み(2 の 2.93 の結果から推定)。

## 8. 残りの受け入れ

- `tools/boot-check/boot-bundle.mjs`: `engine-idle`・`standIn` false・`error` null(通過)。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`): 4/4 通過。`B.welcomeClosed`・`D.welcomeClosed` true。
- `npm test --prefix app`: 1406 passed、10 skipped(通過)。
- テーマの切り替え(受け入れの 2 行): DPR 1 と乱数の DPR 1.08 で通過(`tools/theme-switch/measure.mjs` を受け入れのとおりに実行。elapsedMs 11,566 と 11,119。preBootBytes 311,146,147)。DPR 2.93 は停止あり(5 を参照)。
