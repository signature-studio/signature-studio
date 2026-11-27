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
