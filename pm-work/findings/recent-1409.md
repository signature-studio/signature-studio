# Recent(最近開いたファイル)の読み側・書き側(#1409)

実エンジン: この runner で `fe-build engine`・`fe-build bundle`・`tools/boot-check/boot-bundle.mjs`・`tools/persist/measure.mjs`(4 回目。曲のファイルつきの陽性の対照 3 形と、閉じる要求の列を含む)を走らせ、エンジンのログ(`window.__app.engineLog()`)の `wndproc:`・`window-api:`・`vfs-`・`fruity:` 行を観測した。

runner の値(この機械とは別枠。nproc 4、loadavg 1.5〜1.7)。実測と推定を分ける。

## 結論(4 回目)

- 判定: **受理**(受け入れの (c) の経路)。`positiveControl.fileInVfs` が true、`closeSequence` が配列、`recentCause` が空でない文字列。`recentKept` は false、`recentInSession` は null で、(a)・(b) には当たらない。
- 目標 1(再読込の後、Recent に開いた曲が出る): **未達**。
- 目標 2(FL が Recent を書く先の特定): **未確認**。書き側は、開く・保存・Save As・新規・閉じる要求のどれでも、Recent・MRU・`Recent files.scr` への書きを観測しなかった。
- 保持(journal への載せ): **作らなかった**。書き先が特定できず、Welcome の窓に `TRecentFilesModule` が出る条件も分からないため(受け入れの判定の基準どおり)。
- 製品の変更は trace の行だけ(動作は変えない。既定では出ない。window-api trace が有効な時だけ出る)。

## 1. 陽性の対照(曲のファイルつき。実測)

入れたもの: 曲のファイル `C:\Users\user\Downloads\persist-positive.flp`(demo の最小の曲 `Sacco - Pumping objects.flp`。81,072 バイト。op 5)と、一覧の記録の 3 形。

| 形 | 曲の復元(`vfs-restored`) | MRU の値 "1" | Recent files.scr の読み | Welcome の `TRecentFilesModule` |
|---|---|---|---|---|
| registry-mru(MRU "1" のみ) | あり(81,072 bytes) | found(×2) | 無し | **出ない** |
| scr-utf8(.scr の 1 行のみ) | あり | 無し | attr 0x80・open・read 46 bytes(全部) | **出ない** |
| registry-mru-and-scr | あり | found(×2) | 同上 | **出ない** |

- 前回の推定「曲のファイルが無いので一覧に出ない」は否定された。曲を実際に復元し、FL がその値を読んだ後も、モジュールは出ない。
- 出る条件は、この 3 形とは別にある(未確認)。

## 2. 読み側(実測。起動の中)

- `HKCU\Software\Image-Line\FL Studio 26\MRU`: `RegOpenKeyExW` は起動の中で 50 回。値は `"0"` が無く、`"1"` が found(type 1・90 バイト。NUL 込み。値は入れた曲のパス)、`"2"`〜`"22"` は無い。FL は番号の順に読む(`"1"` だけ見つかる)。
- `Settings\Browser\Recent files.scr`: `GetFileAttributesW`(0x80)→ 読み取りで開く → 46 バイトを全部読む(入れた UTF-8 の行の大きさ)。その後は読まれない。
- 起動で開かれる他のキー(`regLines`。抜粋): `Shared\Paths`・`Shared\General`・`FL Studio 26\General`・`Search paths`(無し)・`Favorite dirs`(無し)。Recent の一覧のキーは MRU のほかに見つからない。
- Browser の tabconfig(`Tab_*.tabconfig`・`UserTabs\*.tabconfig`)は、どれも無い(`GetFileAttributesW` 2、`FindFirstFileW` 2)。

## 3. 閉じる要求の列(実測。`closeSequence`・`closeLines`)

閉じる要求(`exitRequest`)から engine の停止まで 252 ms。その区間の 72 行のうち、閉じる に関わる行:

- `wndproc: in DefWindowProcW … lid=14 cls="TFruityLoopsMainForm" … WM_CLOSE` → **閉じる要求の WM_CLOSE は FL のメインの窓に届く**。
- その中で `wndproc: in SendMessageW … cls="TApplication" msg=0xb03f`(応答 0)、`window-api: GetFocus` ×6、`window-api: PostQuitMessage code=0`。
- `DestroyWindow` は 0 件(窓は破棄されない)。`RegSetValueExW` 0 件。`vfs-kept` 0 件。書き 0。
- 停止の行(harness の `fruity:`): `CreateFruityInstance RETURNED (eax=0)`、`FL64.exe would now call instance->vtable[0](instance) … NOT done in stage 1: recorded only`、`the process ended: visible top-level windows 2 before, 0 after`。

推定(未確認): 閉じる要求は FL のメインの窓の WM_CLOSE まで届き、FL は終了として PostQuitMessage を出す。しかし、この engine の stage 1 は、その後の FL64 の手続き(`instance->vtable[0]` の呼び出しと、その後)を走らせない(記録のみ)。Recent の書き先が、その後の手続きに在るなら、ここでは観測されない。

## 4. 書き側(操作ごと。`recentOps`)

| 操作 | 観測 | Recent・MRU・.scr への書き |
|---|---|---|
| 開く(Ctrl+O) | 曲の読み(vfs-open)、`SHAddToRecentDocs flags=3 path=…`(Windows の呼び出し。FL の一覧ではない) | なし |
| 保存(Ctrl+S) | `.flp~`・Backup・`LastSavedBackup`・`.flp` の書き。「Buy FL Studio」の窓(試用版)が出て、閉じた後に書かれる | なし |
| Save As(Ctrl+Alt+S) | 書き 8 件(保存と同じ。「Buy FL Studio」の窓) | なし |
| 新規(Ctrl+N) | engine の行 4 件、題名が変わらない(効いたかは未確認) | なし |
| 閉じる要求 | 72 行、書き 0、252 ms で停止 | なし |

## 5. Browser の木(描かれない理由の確認)

- 6 つのタブの木の画素は 1 種類(`tabs respond: false`)。Browser の窓は `TEdgeBrowser`(可視)で、WebView2 の面(#1355)。
- WebView2 の呼び出しは 62 件。曲の名前を含む呼び出しは 0。Browser の tabconfig は無い(上の 2)。
- 推定(未確認): Browser の一覧は WebView2 の面で描かれるため、この engine では木が出ない。#1483・#1484 の範囲。

## 6. 判定の道具(`tools/persist/measure.mjs`)

- `positiveControl`: `fileInVfs`(3 形すべてで曲を復元)・`moduleShown`・`variants[]`(形ごとの `fileInVfs`・`present`・`projectLines`・`regLines`)。
- files 段: `closeSequence`(閉じる要求の区間の、窓・書き・停止の行)・`closeLines`(その区間の全行。200 行まで)・`recentCause`(区間の全行から計数して作る 1 行の要約)。
- `recentKept`: 再読込の後に `TRecentFilesModule` があり、キャプションが 1 つ以上ある時だけ true(曲の名前では判定しない)。

## 7. 製品の変更(trace の行だけ。動作は変えない)

- `lift/runtime/fle_win.c`: 窓のメッセージの名前に WM_DESTROY・WM_CLOSE・WM_NCDESTROY を足す(trace の時だけ)。`DestroyWindow`・`PostQuitMessage` の trace 行。
- `lift/runtime/fle_vfs.c`(`vfs-read`・`vfs-attr`)、`fle_reg.c`(`RegOpenKeyExW`・`RegEnumValueW`)、`fle_appfile.c`・`fle_filedlg.c`(`SHAddToRecentDocs`・`SHGetKnownFolderPath`)(前回から。trace の行だけ)。

## 次の Issue にすべき点

1. **`TRecentFilesModule` を出す条件**: 曲の復元と、MRU・.scr の一覧を入れても出ない(上の 1)。Welcome の窓を作る経路で、このモジュールを作る条件を見る(VCL の生成の trace と、FL64 側の条件)。
2. **終了の後の FL64 の手続き**: 閉じる要求は WM_CLOSE → PostQuitMessage まで届く(上の 3)。`instance->vtable[0]` の呼び出しと、その後の手続きは、stage 1 の harness では走らせていない(記録のみ)。5 回目(下の 9)で、trace の時だけ呼ぶ probe を試したが、呼び出しは待ちに入って戻らず、positive control を壊したので取り消した。待ちの場所を特定してから、閉じた後の書きを見る。
3. **Browser の木**: WebView2 の面が描かれない理由(#1483・#1484 の範囲)。

## 8. 受け入れ(4 回目。最後の版で、受け入れのコマンドを全部通した: 18:12〜18:56 UTC)

- 起動(boot-bundle): `state` engine-idle、`error` null、`standIn` false、`elapsedMs` 8,150。
- 言語の段: 選ばれた言語は zh(無作為)。`error` null、`elapsedMs` 7,921(12,500 以下)、`preBootBytes` 313,187,106(314,368,711 以下)、`registryKept` true。再読込の後は `compare-string` で停止(CompareStringW が未モデル。#1395 の範囲。error ではない)。
- ファイルの段: `error` null、`recentKept` false、`recentInSession` null、`bytes` 713,404(104,857,600 以下)、`positiveControl.fileInVfs` true、`closeSequence` 配列(3 行)、`recentCause` あり → (c) の経路で受理。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`): 4 つとも `error` null、engine-idle、`opened` true、B と D は `welcomeClosed` true。
- `npm test --prefix app`: Test Files 74 passed(1 skipped)、Tests 1,455 passed(10 skipped)。

受理の理由: 受け入れの (c) の 3 条件(曲の復元・閉じる要求の列・要約)を、実測の出力で満たした。目標 1・2 は未達のまま(上の結論)。次の Issue は、上の「次の Issue にすべき点」の 1 と 2。

## 9. 5 回目(受け入れの通し直し: 21:33〜22:20 UTC。probe は取り消し)

実エンジン: この runner で `fe-build engine`・`fe-build bundle`・`tools/boot-check/boot-bundle.mjs`・`tools/persist/measure.mjs`・`tools/smoke/welcome-flow.mjs`・`npm test --prefix app` を通し、受け入れの 1 行を最後まで実行した(`ACCEPT_EXIT=0`)。runner の値(この機械とは別枠。nproc 4、loadavg 1.9〜2.0)。

受け入れの出力(抜粋):

- 起動: `BOOT {"state":"engine-idle","error":null,"standIn":false,"elapsedMs":9232}`
- 言語の段: 選ばれた言語は ja。`error` null、`elapsedMs` 9,131(12,500 以下)、`preBootBytes` 313,272,621(314,368,711 以下)、`registryKept` true。
- ファイルの段: `error` null、`recentKept` false、`recentInSession` null、`bytes` 714,049。`positiveControl.fileInVfs` true(3 形すべて)、`closeSequence` は配列(3 行)、`recentCause` = `close: WM_CLOSE reached a window 1 time(s); DestroyWindow 0; PostQuitMessage 1; registry writes 0; file writes 0; shell calls 0; the engine exited after 253 ms (72 lines)`。受け入れの (c) の経路で受理。
- 流れ A〜D: 4 つとも `error` null、`state` engine-idle、`opened` true、B と D は `welcomeClosed` true。
- `npm test --prefix app`: Test Files 74 passed(1 skipped)、Tests 1,455 passed(10 skipped)。

probe(取り消した。`lift/harness/fruity.c`。差分は `pm-work/findings/recent-1409-exit-probe.diff`): 閉じた後の FL の手続き(`instance->vtable[0]`)を、window-API trace が有効な時だけ呼んだ。

- 行: `fruity: exit probe: calling instance->vtable[0] 0x12541b0 (f_12541b0) with rcx=instance 0x3e847350`。その後、trace の行は出ず、30,121 ms `engine-idle` のまま戻らなかった(2 回。同じ結果)。
- 閉じる区間の行数は 20。レジストリ・ファイルの書きは 0。stdout(`fruity:`)と stderr(`window-api:`・`wndproc:`)は別の流れで、行の前後関係は確かめられない。
- idle の停止(`fle_thread_set_idle_stop(1)`)を probe の間だけ入れた試しも、停止の行は出ず、同じく 30 秒の `engine-idle` で止まった。
- 副作用: engine が終わらないため、後の positive control の 3 形が `NoModificationAllowedError: createWritable` で失敗し、`fileInVfs` が false になって、(c) が通らなくなった。
- 取り消した理由: この probe の形のままでは、受け入れが通らない。待ちの場所を見るには、測定の終わりで engine を止める手段と、thread の trace(`fle_thread_set_trace`。native の `FLE_FRUITY_THREAD_TRACE` と同じ物)が要る。

判定: **受理**(受け入れの (c) の経路)。目標 1(再読込の後の Recent の保持)は**未達**。目標 2(FL が Recent を書く先)は**未確認**。保持は作っていない(書き先が特定できないため)。

次の一手(推定を含む。未確認):
1. 待ちの場所: 上の probe を、測定の終わりで engine を止める形にして入れ、thread の trace を付けて、待ちの対象(スレッド・イベント・メッセージ)を見る。probe は trace が有効な時だけ、既定の動きは変えない。
2. 1 が分かったら、その待ちが済んだ後の書き(Recent・MRU・`Settings\`)を、操作ごとに見る。書きが見えたら、既存の journal(#1383・#1385)に載せる。
