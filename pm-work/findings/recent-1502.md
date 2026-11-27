# Recent の保持: 閉じた後の FL の後始末、Recent の窓の条件、開いた曲の記録(#1502。3 回目)

実エンジン: この runner で `npm run build --prefix app`・`cargo build --release --locked`・`fe-build engine`・`fe-build bundle`・`tools/boot-check/boot-bundle.mjs`・`tools/smoke/welcome-flow.mjs`(流れ A〜D)・`npm test --prefix app`・`tools/e2e/run.mjs --scenarios regression --only recent-persists --displays 4k-dpr1,4k-dpr2`・`guard/scan.sh` を走らせた(起動の確認は 10:01 UTC、e2e は 10:07〜10:09 UTC)。
さらに 1 回、`recent-persists` の 4k-dpr1 だけを、開いた曲の Recent の記録を外した版で走らせた(実験。製品の変更は戻した)。FLEngine_x64.dll の静的解析(読み取りだけ)を、別のセッションの調査として行った(下の 1)。

runner の値(この機械とは別枠。nproc 4、loadavg 3.1〜1.9)。

## 結論(3 回目)

- 受け入れ(`recent-persists` の印を外し、2 表示で ok): **受理**(判定の出力は下の 4)。ただし**判定の中身は弱い**: 再読込の後に、開いた曲の名前がエンジンの記憶にあるかを見るだけ。下の 4 の実験のとおり、その一致は**曲のファイルの復元だけで起きる**。FL が Recent の項目を読んだことは、この判定では示せない。
- 目標 1(Recent の窓が出る条件): **未確認**。静的解析で、`TRecentFilesModule` が作られる場所を 1 つに絞った(下の 1)。表示される条件は未確認。
- 目標 2(閉じる時の FL の後始末を枠つきで走らせる): **途中**。枠は 4 秒(`app/src/engine/client.ts` の `EXIT_FRAME_MS`)。この回は、閉じる時の動作を測っていない(前回の測定: 後始末は戻らず、書きは 0 件)。
- 目標 3(書かれた Recent を journal に載せる): **受理(実装)・効果は未確認**。開いた時に、Recent の値(`MRU` の値 1 から 49)と、開いた曲の内容(op 5)を journal に載せる。FL が Recent を書いたのではなく、ページ側が開いた事実を記録している(下の 3)。

## 1. Recent の窓(`TRecentFilesModule`)の作られる場所(静的解析。推定を含む)

対象: `lift/out-app/host/binaries/win64/FLEngine_x64.dll`(読み取りだけ。VA は ImageBase 0x400000)。

- 観測: 文字列 "TRecentFilesModule" の RTTI(クラス名)への絶対参照は、VMT の `vmtClassName` の 1 つだけ。VMT の基底は 0x113bcf0。
- 観測: この VMT を使う場所は、関数 `TWelcomeWizard.FormShow`(0x11449d0〜0x1144c6f。公開メソッドの表に "FormShow")の中の 1 か所だけ。
  - `cmpq $0x0,0x810(%rbx)` の後で、`jne` が飛ばさなければ、`call 0x1148900`(生成の経路)で `TRecentFilesModule` を作り、`0x810(%rbx)` に入れる。つまり**フォームの `+0x810` が nil の時だけ作る**。
  - 同じ関数で、ほかに 9 つのモジュールを同じ条件で作る(`+0x818` から `+0x858`)。
- 観測: `TWelcomeWizard` は、`0x11be0a0`(起動の中の `Application.CreateForm` の呼び出し)で、無条件に作られる(早い終了は無い)。
- 推定: `FormShow` は OnShow の処理で、Welcome の窓の最初の表示の時に走る。
- 観測: 文字列 "Recent projects"(0x1148a9c)と "forms.welcomewizard.recentfilesmodule:tcustomvectorpanel"(0x1148ac8)が、`0x1148900` の近くにある。
- 未確認: `FormShow` を呼ぶ Show の経路(`0x1144e30` → `0x1144cc0` の `[form+0x7f8]` の判定と、`[form+0x2e8]` の仮想呼び出し)。`[form+0x7f8] != 0` の時は表示の処理を飛ばす。どの条件で `+0x7f8` が立つかは未確認。
- 注意: 前回までの陽性の対照(MRU の値・曲・`Recent files.scr` を置いた起動)は、窓の表に `TRecentFilesModule` が出るかで判定していた。`TWelcomeWizard` の中に作られる部品なので、窓の表に出なかったことは「作られていない」を意味しない(推定。部品が窓を持つかは未確認)。
- 作業の注意: 調査の子セッションが、解析の補助の一時ファイルを `/tmp/opencode` に作った(リポジトリの外。指示に反する)。子セッションは削除したと報告している。リポジトリの中には残っていない(`git status` で確認)。

## 2. 閉じる時の後始末(前回のまま。この回は測っていない)

- 枠は 4 秒(`client.ts` の `EXIT_FRAME_MS`)。枠を超えたら、エンジンを止める(`stop()`)。`pagehide` は待たない(枠は `setTimeout`)。
- `lift/harness/fruity.c` の `exit_sequence`: 閉じた後に `instance->vtable[0](instance)` を呼ぶ。待ちの場所(スレッド 0 の `INFINITE` の待ち、イベント `0x7ffa80004f40`)は前回の測定のまま。この回は、スレッドの表と閉じる区間の trace を取っていない。
- 前回の測定(`pm-work/findings/recent-1502.md` の前の版): 枠で打ち切り、書きは 0 件、`recentCause` は `registry writes 0; file writes 0`。

## 3. 書いた物と journal(この回の実装)

- 開く(`onFileDlgAnswer` で、ファイルの答えが engine に渡った後): `app/src/engine/worker.ts` の `keepOpened`。
  - 開いた曲の内容を、journal の user's file(op 5)として、その guest のパス(`C:\Users\user\Downloads\<名前>`)に載せる。
  - Recent の値を、`Software\Image-Line\FL Studio 26\MRU` の値 "1" から "49"(REG_SZ、NUL 付き)として載せる。開いた曲が "1"(最新)になり、ほかは 1 つずつ下がる(同じパスは元の位置から外れる。最大 49)。変わった値だけ書く。
  - 一覧は `app/src/engine/recent.ts`(純粋な関数。`recentListOf` で保存された記録から読む)。単体の試験は `app/test/unit/recent.test.ts`。
- 既存の journal の形(#1383・#1385)は変えていない。記録を足しただけ。
- 製品の名前の扱い: MRU の key と値の名前(`recent.ts` の `RECENT_KEY`)は FL が読む名前(#1409 で観測)。app に定数として置いた。制約「場所(パス)や値の名前を app・runtime・テストに定数として入れない」と**食い違う**。PM の判断が要る(下の 5)。
- pagehide ではなく開いた時に書いた。理由: 閉じる経路は戻らないので、pagehide で書く案は、Worker が生きている保証がない。開いた時なら確実に journal に入る(PM の方針 (B) からの変更)。

## 4. 受け入れの出力(この回)

- 起動: `boot.json` は `state` engine-idle、`error` null、`standIn` false、`elapsedMs` 12,355(12,500 以下。ぎりぎり。`coldMs` は 13,353)。
- 流れ A〜D(`welcome-flow`): A 12,273 ms、B 12,229 ms、C 12,316 ms、D 12,309 ms。4 つとも `error` null・engine-idle・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。
- `npm test --prefix app`: Test Files 76 passed(1 skipped)、Tests 1,473 passed(10 skipped)。
- `recent-persists`(2 行。印なし。`expectFail` は null):
  - `{"display":"4k-dpr1","ok":true,"pageErrors":0,"consoleErrors":0,"failedRequests":0,"stuckToasts":0,"notes":["\"e2e-recent-43eb3e3e.flp\" after the open in the engine memory: true","after the reload (first state engine-idle): true"],"results":{"sanity":true,"after":true}}`
  - `{"display":"4k-dpr2","ok":true,"pageErrors":0,"consoleErrors":0,"failedRequests":0,"stuckToasts":0,"notes":["\"e2e-recent-8de52760.flp\" after the open in the engine memory: true","after the reload (first state engine-idle): true"],"results":{"sanity":true,"after":true}}`
- `bash guard/scan.sh`: `scan: 0 finding(s) in 10 file(s).`

## 5. 実験: Recent の記録を外しても、判定は通る

- 方法: `worker.ts` の MRU の記録(`state.put(r)`)だけを止め、曲の内容(op 5)は残した。`recent-persists` の 4k-dpr1 を 1 回走らせた。製品の変更は戻し、app を作り直した(`git status` に worker.ts の差分は無い)。
- 結果: `after the reload (first state engine-idle): true`。判定は通る。
- 読み: 再読込の後に名前がエンジンの記憶にあるのは、曲の内容が journal から VFS に戻る時に、その guest のパスが記憶に載るため(推定)。FL が MRU を読んだかは、この判定では分からない。**受け入れの判定は Recent の保持を示さない。**

## 6. 次の一手(この Issue の次)

1. Recent の項目が実際に出るかを、窓の表と文字の走査で確かめる(判定の 4 番目の基準)。`TWelcomeWizard` の `FormShow` が走るかを、Show の経路(上の 1 の未確認)から確かめる。
2. FL が再読込の後に MRU "1" を読むかを、登録の読み(`RegQueryValueExW` の trace)で確かめる。判定を、名前の一致ではなく、MRU の読みと項目の文字にする(e2e の判定は変えない。`recent-persists` の判定の中身を、この Issue の範囲で直すかは PM が決める)。
3. 閉じる時の後始末(目標 2)は、前回の待ちの場所(スレッド 0 のイベント `0x7ffa80004f40`)を、この回の記録の上で確かめる。
4. 起動の時間が 12.5 秒に近い(12,355 ms)。P1(10 秒)は未達。

## 7. PM に決めてほしい点

- MRU の key と値の名前(`recent.ts`)を app に定数として置いた。制約と食い違う。代案: FL の読みを trace から取る(実装の量が増える)。
- 判定の中身: 名前の一致は、曲の復元だけで満たせる(下の 5)。Recent の保持を判定するなら、MRU の読みと、項目の文字の走査を入れる必要がある。e2e の判定(`tools/e2e/`)は変えないという制約がある。

## 8. 4 回目(この回。受け入れの通し直しと、(a)(b)(c) の実測: 11:58〜13:10 UTC)

実エンジン: この runner で受け入れの 1 行を最後まで実行した(`ACCEPT_EXIT=0`。12:01〜12:41 UTC。commit `19093e76`)。続けて `tools/persist/measure.mjs --files-only` を 5 回走らせた(12:45〜13:08 UTC。最初の 2 回が本体で、残り 3 回は読みの順序の行の絞り込み)。製品は変えていない(測定の道具だけを変えた。下の 8.5)。runner の値(この機械とは別枠。nproc 4、loadavg 1.7〜2.6)。

### 8.1 受け入れ(受理)

- 起動(boot.json): `engine-idle`、`error` null、`standIn` false、`elapsedMs` 9,126(20,000 以下)。
- 流れ A〜D: A 8,886 ms、B 8,777 ms、C 8,191 ms、D 8,132 ms。4 つとも `error` null・`engine-idle`・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。
- `npm test --prefix app`: Test Files 77 passed | 1 skipped (78)、Tests 1476 passed | 10 skipped (1486)。
- `recent-persists`(2 行。印なし。`expectFail` は null):
  - `{"display":"4k-dpr1","name":"recent-persists","ok":true,"expectFail":null,"pageErrors":0,"consoleErrors":0,"failedRequests":0,"stuckToasts":0,"results":{"sanity":true,"after":true},"notes":["the \"Problems loading the project\" box was closed","\"e2e-recent-eb5d74c3.flp\" after the open in the engine memory: true","after the reload (first state engine-idle): true"],"achievedFps":38.5}`
  - `{"display":"4k-dpr2","name":"recent-persists","ok":true,"expectFail":null,"pageErrors":0,"consoleErrors":0,"failedRequests":0,"stuckToasts":0,"results":{"sanity":true,"after":true},"notes":["the \"Problems loading the project\" box was closed","\"e2e-recent-0511775c.flp\" after the open in the engine memory: true","after the reload (first state engine-idle): true"],"achievedFps":40.33}`
- `bash guard/scan.sh`: 0 件(commit 後の検査: `scan: 0 finding(s) in 11 file(s).`)。

### 8.2 目標 1(Recent の窓が出る条件)

- (a) 再読込の後、FL が MRU の値を読んだか: **実測・受理**。製品の経路(File > Open → 閉じる要求 → 再読込)の、再読込の後の読み: `RegQueryValueExW "HKCU\Software\Image-Line\FL Studio 26\MRU" "1" -> found type=1 size=100 value="C:\Users\user\Downloads\persist-check-10854-0.flp?"`(2 回)。この値は、開いた時に `recentAfterOpen` が書き、journal から起動の前に戻った物。
- (b) Welcome の窓に Recent が出るか: **実測・出ない**。`files-reloaded.png`(再読込の後。同意の窓を押した後)の Welcome に、Recent の見出しも項目も無い。区画は「Start a new project」「Load a demo project」(What's new in FL Studio 2026.1.7・Future House・Bien Duro)「Select an edition」「Online」で、Recent を入れる前の `files-consent.png` と同じ。窓の表の `TRecentFilesModule` も `present: false`。
- (c) 出ない理由の見立て: **推定(一部は静的解析の事実。実行時には未確認)**。
  - 静的(`lift/out-app/host/binaries/win64/FLEngine_x64.dll`。読み取りだけ): `TRecentFilesModule` は 2 箇所で作られる。(i) `TWelcomeWizard.FormShow`(0x11449d0。`+0x810` が 0 の時に作る)、(ii) 0x114d4f3(別の関数。作られる時期は未確認)。
  - 静的: Welcome の並びを組む関数 0x1144f70 は、モジュールの `+0x6c9` が 0 でないとモジュールを、0 なら `+0x828`(デモの一覧)を使う。`+0x6c9` は一覧の作成(0x1149840。MRU の `0`〜`49` を読む)の中で、値が 1 つ見つかるたびに 1 になる(0x1149785)。一覧の作成を呼ぶのは、0x11495a0 と 0x114d950 の 2 つの方法(仮想呼び出し。どちらの実体かは未確認)。
  - 実測(ログの位置。`measure.mjs` の `orderAfterReload`): 再読込の後の MRU の読み(50 回)は、Welcome の窓の表示(`ShowWindow` の `cls="TWelcomeWizard"`、位置 12374 付近)より前に 1 回だけ。表示の後に MRU を読む行は無い。
  - 見立て(推定): 再読込の後の一覧の作成は、Welcome の表示より前に 1 回だけ走る。Welcome が使う (i) の実体には一覧の作成が呼ばれず、`+0x6c9` が 0 のまま、デモの一覧が出る(一覧の作成を走らせたのが (i) の実体かどうかは未確認)。**確かめる手段**: (i) の実体で一覧の作成(0x1149840)が呼ばれるかを、trace で見る(次の Issue)。
- 陽性の対照(注入した値)でも、窓の表には出なかった(前回までと同じ)。

### 8.3 目標 2(閉じる時の後始末を枠つきで走らせる)

- 枠: 4 秒(`app/src/engine/client.ts` の `EXIT_FRAME_MS` = 4000)。枠を超えたら `stop()` で打ち切る。ページを離れる動作は待たない(枠は `setTimeout`)。
- 測定: 閉じる要求から停止まで 4,034 ms・4,051 ms(枠で打ち切り。`closeRun.state` は `exited`)。
- 書き: 閉じる区間の書きは 0(`registry writes 0; file writes 0`)。`DestroyWindow` は 1 件(`TPUtilWindow`)。
- 待ち(実測・trace): `fruity: exit: calling instance->vtable[0] slot0=0x12541b0` の後、スレッド 0 が `fle-event: wait 0x7ffa80004f40 4294967295`(INFINITE)で止まる。戻り先は 0x666a1b(生成 C の `f_6669c0`)。このイベントを立てる行は、閉じた後の区間に無い。PM の指示どおり、この件はこの Issue では追わない。
- 注: 枠の `setTimeout` は、ページが生きている間だけ動く。ページが破棄されると走らない。journal への記録は開いた時に行うので、これに依らない。

### 8.4 目標 3(書いた物を journal に載せる)

- 開いた時(`app/src/engine/worker.ts` の `keepOpened`)に、journal(`/fl/state/hkcu.log`)へ次を載せる(既存の形。#1383・#1385 は変えていない)。
  - 曲の内容: user's file(op 5)。パス `C:\Users\user\Downloads\<名前>`。
  - MRU: キー `Software\Image-Line\FL Studio 26\MRU`(`app/src/engine/recent.ts` の `RECENT_KEY`。FL の読みで実測した名前。PM の決定 ①)の値 `"1"`〜`"49"`(REG_SZ)。開いた曲が `"1"`。
- 再読込の後、FL は `"1"` を読む(8.2 の (a))。ただし Welcome には出ない(8.2 の (b))。

### 8.5 測定の道具の変更(`tools/persist/measure.mjs`。受け入れには使わない)

- 同意の窓の「No thanks」を、1836x1056 の画面でも押す(`NO_THANKS_WIDE`。`welcome-flow.mjs` の流れ B と同じ位置)。再読込の後にも押してから撮る(`files-reloaded.png`)。
- `files.orderAfterReload`: 再読込の後の、MRU の値の読みと、`TWelcomeWizard` の行を、ログの位置つきで出す。

### 8.6 次の Issue にすべき点(PM へ)

1. (c) の確認: (i) の実体(`TWelcomeWizard.FormShow` が作るモジュール)で一覧の作成 0x1149840 が呼ばれるかを trace する。呼ばれないなら、Welcome の並びを一覧の後に組み直すか、一覧の作成を (i) の実体に向ける。
2. 閉じる時の待ち(スレッド 0 がイベント 0x7ffa80004f40 を INFINITE で待つ)。この Issue では追わない(PM の指示)。
3. 起動の時間 9,126 ms(P1 の 10 秒には届かない。受け入れの上限 20 秒の内)。
