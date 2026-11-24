# コード進行ツールの Start creating で engine が止まる(#1386): 再現と原因の記録

実エンジン: `tools/piano-roll-tools/measure.mjs` で、main の版の engine(`fe-build engine` と `fe-build bundle` を作り直したもの)を起動し、Start creating を押して、engine が exit 134 で止まるのを観測した。Python の読み込みの要求(`LoadLibraryW 126 "Python\Python312.dll"`)と、停止の行 `fle: guest address 0x520000077c ... [fle_addr_hi32]` を、道具の出力で確かめた。

判定: **差し戻し**(直していない)。理由は「直し方」の節。道具 `tools/piano-roll-tools/measure.mjs` は作って、動作を確かめた(受理)。

runner の値。この機械とは別枠。nproc 4、loadavg は各行の `loadavg` の値(下の JSON)。

## 1. 測った値(道具の JSON。直す前)

受け入れの形(DPR 1 と乱数の DPR)。`missingPaths` は 590 件を超えるので省く。全文は道具の出力(`--dprs` の 1 行ずつ)。

```
{"tool":"chord-progression","dpr":1,"error":null,"engine":"exited","stopped":true,"presentedDelta":0,"notes":null,"elapsedMs":6745,"preBootBytes":311109738,"nproc":4,"loadavg":[1.69,1.48,1.27],"room":{"width":1836,"height":1056},"windowNew":[{"cls":"TPRChordToolForm","w":714,"h":458,"painted":true,"paintedShare":0.993}]}
{"tool":"chord-progression","dpr":1.79,"error":null,"engine":"exited","stopped":true,"presentedDelta":0,"notes":null,"elapsedMs":6675,"preBootBytes":311109738,"nproc":4,"loadavg":[1.7,1.52,1.29],"room":{"width":1836,"height":1056},"windowNew":[{"cls":"TPRChordToolForm","w":714,"h":458,"painted":true,"paintedShare":0.993}]}
```

- `notes`: null。この道具は、Piano roll のノートを数えない(ゲストのメモリの読み方を作っていない)。受け入れの `notes >= 1` は、この道具では確かめられない。
- DPR 1.79 で `room` が 1836x1056 のまま。DPR 1.37 の試しでは 2515x1447 になった。1.79 の違いの原因は未確認。
- 直す前と後の比較のための、直した後の行は無い(直していない)。

参考(Riff machine。#1387 の材料): DPR 1 で `engine-idle`、`stopped` false、`presentedDelta` 27、新しい窓は無し。停止の代わりに次の行が出る。

```
fle: MessageBoxW NOT SHOWN, answered by the virtual machine with the default button (id 1; S-ia2-msgbox-default), type 0x10, caller 0x86aadc, caption "Error", text "Some required files are missing."
missing-path: FindFirstFileW 3 "C:\Program Files\Image-Line\FL Studio 21\Data\System\Riff machine\*.*"
```

## 2. 停止の全文(再現の操作: TOOLS → Chord progression tool → Start creating)

道具の `steps`: `room set`, `Escape (Welcome window)`, `Escape (Problems box)`, `project open: title "FL Studio 2026"`, `Piano roll: lid 154 "Piano roll -"`, `TOOLS clicked at guest (244, 19)`, `TOOLS drop-down: lid 163, 437x464`, `5 Down, Enter`, `tool window: lid 167 "Piano roll - generate chord progression" at 561,299 714x458`, `Start creating clicked at guest (918, 687)`。

停止の行(engine のログの順。`fle: act1-net` の 4 行は起動の時の通信で、停止と関係しない):

```
== stop: returned from entry            (起動で毎回出る。停止ではない)
runtime message: (none)                 (同上)
fruity: == stop: SIGABRT (runtime abort, see runtime message)
fruity: runtime message: first non-background pixel on a drawing surface (decision 0126): written by UpdateLayeredWindowIndirect into the surface of top-level window lid 8; function entries since the program began: 0
fruity: exit code 134 (the codes of harness/wasm/trace_native.c: 134 SIGABRT, ...)
fle: guest address 0x520000077c (8-byte load) has bits above 32 set: not representable in wasm32 (a native run faults here) [fle_addr_hi32]
```

- `runtime message` の行は、停止の原因ではない(ハーネスが最後に出す既定の文)。
- 0xeedfade の例外(#1366 の 4.3)は、この再現では出ない。止まり方は `fle_addr_hi32`。

Start creating の前後で、Python に関係する行(`missing-path`、順序は道具の `missingPaths` の並び。ページの log は stdout と stderr を連結するので、`fruity:` 行との前後は保証されない):

```
missing-path: GetFileAttributesW 3 "C:\Program Files\Image-Line\FL Studio 21\Python\Python312.dll"
missing-path: GetFileAttributesW 3 "Python\Python312.dll"
missing-path: GetFileAttributesW 3 "C:\Program Files\Image-Line\FL Studio 21\Shared\Python\Python312.dll"
missing-path: GetFileAttributesW 3 "C:\Program Files\Image-Line\FL Studio 21\Python\Python312.dll"
missing-path: LoadLibraryW 126 "Python\Python312.dll"
```

Chord progression のスクリプト(`System\Tools\Chord progression\`)の探索は、この操作の中に 1 件も出ない。最初に要るのは Python の DLL である。

戻り番地: 停止の行に出ない(engine は呼び出し元を出さない)。`lift/out-app/gen/wasm-func-names.tsv` で引く元の番地が無いので、関数名は未確認。

## 3. 原因(確かめたこと / 推定)

確かめたこと(実測):
1. 窓(`TPRChordToolForm`)は止まらずに開く。止まるのは Start creating の押下の時だけ(押下の前の状態は `engine-idle`)。Enter(窓を閉じる)では止まらない。
2. Start creating の後で、Python の DLL を 4 か所で探し(`GetFileAttributesW 3` = 無い)、続けて `LoadLibraryW 126 "Python\Python312.dll"`。
3. seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)に `Shared\Python\`・`System\Tools\`・`Riff machine` の行は 0 件。resources(`lift/out-app/host-extra/resources/Shared/Python/`)には実物がある。つまり、seed 表に行が無いので VFS に載らない(`ci/resources-prefixes.txt` には接頭辞が在る)。
4. インストーラの一覧(`inputs/fl-installer-files.tsv` の 26519 行)に `Shared\Python\python312.dll`(7,003,928 B)がある。`lift/runtime/fle_wall_ia2.c` の注の「インストーラに Python312.dll が無い」は、この一覧と食い違う(Issue の事実 4 と同じ)。
5. `python312.dll`(取り出した実物を Python で PE import 表として読んだ。調査の目的だけ)は、19 個の DLL から 358 個の関数を import する。runtime のソースに名前が出るのは約 174 件(名前の一致による粗い数。コメントの一致も含む)。残りの約 184 件は未実装と見られる。
6. 停止は exit 134。停止の直前の並びに `LoadLibraryW 126` がある(順序は確定しない)。

推定(未確認):
- `LoadLibraryW` が NULL を返した後、呼び出し側がそれを使い、不正なアドレスを読んで止まる(`fle_addr_hi32`)。戻り番地が無いので、呼び出し側の関数は確かめていない。
- 126 の答えは `fle_wall_ia2.c` の `fle_ia2_path_absent` の choice(記録の上の選択)。これを外しても、DLL は lift されていないので、ロードは「ファイルはあるが、このビルドのモジュールではない」の停止(`stop_file_not_a_module`、decision 0233)になる見込み。試していない。

## 4. 直し方の検討(この Issue の範囲)

- (a) 配信(seed 表に `Shared\Python\` 37 件と `System\Tools\Chord progression\` 59 件を `@lazy-file:` で足す): 起動前の量は増えない。ただし lazy の件数の上限(`app/src/bundle/manifest.ts` の `lazy` の `maxItems` 128。#1366 の記録では現在 75 件)を超えるので、app/ の変更と deploy が要る。単独では、止まる場所が変わるだけで、止まること自体は直らない。
- (b) 実行(python312.dll を real module として lift する): `ci/extract-dlls.tsv` に行を足す必要がある(作業者は ci/ を変えない)。加えて、約 184 件の import の実装、realmod の表の生成、thread や file mapping など。この Issue の範囲を超える。
- choice(`fle_ia2_path_absent`)だけを外す: 試していない。`fle_wall_ia2.c` の注(ia2-W21)によると、同じ choice はチャンネルラックの設定アイコンの要求(`Python\Python312.dll`)にも効いている。外すと、その画面の動きが変わる。
- (c) 止めないだけの修正: 受け入れ(ノートが作られる)を満たさない。行わない。

→ 製品のコード(`app/`・`lift/`・`ci/`)は変えていない。判断は監査・受付へ: Python312.dll を real module として動かすか(大きい作業)、コード進行ツールを別の方式にするか。

## 5. 起動の確認(この Issue では製品を変えていない)

- `tools/smoke/welcome-flow.mjs`(2 回。受け入れの流れ A〜D):
  - 1 回目: A 合格(elapsedMs 7353、preBootBytes 311,109,738)、B 合格(7339)、C 失敗(`no engine-idle within 180000 ms (state running)`)、D 失敗(同)。
  - 2 回目: A 失敗(同)、B 合格(7105)、C 合格(7192)、D 合格(7034)。
  - 失敗は、起動の前に viewport を `Emulation.setDeviceMetricsOverride` で変える起動(welcome-flow の `VIEW_START`)で観測。viewport を変えない起動(`bootBundle` の既定)は 4 回続けて成功(elapsedMs 7106〜7188)。原因は未確認。
- `npm test --prefix app`: 合格(Test Files 69 passed、Tests 1351 passed、10 skipped)。

## 6. 道具の注意

- `tools/piano-roll-tools/measure.mjs` は、TOOLS のメニューの位置をメニューバーの窓の矩形の比率で、Start creating の位置を Chord 窓の矩形の比率で決める。どちらも DPR 1 の画面の読みから取った比率。DPR を変えた時の位置の精度は、停止の前に確かめた(DPR 1.37 で窓の表示と押下は通った)。
- 起動の前に viewport を変える方式は使わず、起動の後(afterIdle)で room と DPR を変えている(上の 5 の問題を避けるため)。
