# #1400 内蔵の音源 Sytrus の読み込み(作業の記録)

実エンジン: 作業ツリー(runner の値。nproc 4。loadavg 1.1〜1.9。この機械とは別枠)で、`fe-build engine`・`fe-build bundle`・`boot-check`・`tools/p3-audio-probe/measure.mjs` を実行した。

## 追加したもの(受理)

- `lift/plugins/sytrus/plugin.tsv` の 1 行: `generators.sytrus.sytrus_x64.dll`、`Plugins\Fruity\Generators\Sytrus\Sytrus_x64.dll`、本体、6,269,440 B、SHA-256 `e1395e97…`、hook `rva:0x1b930,0x38b80,0x38ba0`(`inputs/plugin-hooks.tsv` の 3 点。先頭バイトは既存の Delphi の 3 点と同じ)、omit `0x48cb70`。
- omit の根拠: Sytrus の中の guest の setjmp(register 保存の形)を呼ぶ関数は 1 つだけ(`call` の並びを調べた。呼び出しは `0x48cb8b` → 中継 `0x11ab0`、続きは `0x48cb90`)。Balance の 0x2c13a0(呼び出し 0x2c13bb → 中継 0xe120、続き 0x2c13c0)と同じ形。
- 共有の表は編集していない。抽出と seed は `app/scripts/lib/plugin-rows.mjs` の行から、取り出しは `ci/extract-dlls.tsv` の既存の行(Generators の Sytrus は 181 行目に在る。`ci/` は変えていない)。

## 単位の段の直し(受理。設計 125 の (13))

- 症状: `fe-build engine` が `units` の段で止まった(`not listed ["m11-00.c" … "m11-06.c"]`)。単位の一覧は `lift/harness/two/plain-units.txt` の静的な行で、プラグインを足すと必ず共有の表の編集が要った。
- 直し: `units` の段は、一覧に無い `m<K>-<NN>.c` を gen の出力から求め(`plugins::check_units`)、`lift/out-app/plugins/extra-units.txt`・`extra-objs.txt` に書く。新しい段 `compile-extra` がそれをコンパイルし、`link` は `extra-objs.txt` の語を末尾に足す(`Step::RunWords`)。静的な一覧に無い単位がプラグインの形でない場合は止まる。
- 試験: `cargo test -p fe-build -p fe-wasmgen`(20 件など)が通る。`tests/plan/engine.txt` を新しい表示に合わせた。

## 実測(作業ツリーの値)

- engine: `list generators.sytrus.sytrus_x64.dll: emitted ok 18658, left out 4 (omit 1), listed 18654`。`units: 7 more than the lists: m11-00.c … m11-06.c`。`hybrid: 8 direct modules, 2 separate modules`。exit 0(35 分)。
- bundle: `lazy 6269440 B … Sytrus_x64.dll (after the engine starts)`、`lazy 21486577 B … sytrus_x64.c580000.wasm (after the engine starts)`。Balance の base(c1d0000)と重ならない(c580000)。
- boot-check(committed 状態): `state engine-idle`・`standIn false`・`error null`・`elapsedMs 12241`(P1 の 10 秒は未達。負荷 1.1〜1.9 の runner)。
- preBootBytes: 64,093,397 B(manifest・engine の br・glue・seed の pack・dll の pack の和。`tools/engine-cost/build-variant.mjs` と同じ式による静的な値。上限 314,368,711 B 以内)。
- 警告: `couldn't be found` は出ない(ログの `image-read: sytrus_x64.c580000.dll requested=1` が出る)。`LoadLibraryW` の失敗は `imm32.dll` のみ(Sytrus とは無関係)。

## demo 2 件(差し戻し)

- `Synth_dfr - Overcome - Sytrus.flp`・`Olbaid - Sytrus Essentials.flp` とも、読み込みの後に `fle: CreateFileW: empty path is not defined`(`lift/runtime/fle_vfs.c:388` の `fle_stop`)で engine が exit する。`error` は「the title of … (the title is \"Song info\")」の時間切れ(180 秒)。`end.state` は `exited`。
- 原因 1(確かめた): Sytrus の `Data\Default.dat` が VFS に無い。`Default.dat`(1,832 B)1 件だけを seed の表に足すと、この停止は消えた。
- 原因 2(確かめた。未修正): `indirect call to 0xc9cf273 is not a known function entry`。`0xc9cf273` は rebase の像の VA で、元の RVA は `0x44f273`。これは関数 `0x44f130` の途中の case の位置で、`mov eax,[rcx+rax*4+0x4520d0]; add rax,rcx; jmp rax`(MSVC の switch 表)の飛び先。lift の MSVC 表の認識(`fe-decode` の jumptab)は、ガード(`cmp/ja`)が無いため N を決められず(`msvc_rejected 0xc9cf271`)、間接の飛びは既知の関数の入口だけへの呼び出しになって止まる。
- 原因 3(確かめた。未対応): Sytrus の `Data` の 109 件(約 1.2 MB。`inputs/fl-installer-files.tsv`)を全部 seed に足すと、boot の段で `[err] vfs-seed: … more than 512 seed files` で止まる。`--lazy` 指定は pack-seed の上限 128 件(`must have <= 128 items`)。Data を入れる仕組み(plugin.tsv の行の種類)は未設計。

## 3 回目の試行(受理・段階 1 の受け入れを通した)

実エンジン: runner の値(nproc 4。boot の loadavg 1.35・1.26・1.78。この機械とは別枠)。`fe-build engine`(36 分)・`fe-build bundle`(14 分)・受け入れの残りを、作業ツリーで実行した。

- 前回の差分は、作業ツリーの index に残っていた(`pm-work/findings/sytrus-1400-attempt.diff` は無かった)。`git apply` は不要だった。
- 単位の段: `units: 7 more than the lists: m11-00.c … m11-06.c`、`compile-extra` と `link` が通った。`generators.sytrus.sytrus_x64.dll: listed 18654, emitted ok 18654`。
- `ci/extract-dlls.tsv` の Sytrus の行(Generators・181 行目)を消した。Vocodex の `Sytrus_x64.dll`(`Effects\Vocodex\Synthesizer`・別の SHA-256)は残す。
- 追加: `lift/plugins/sytrus/plugin.tsv` の 1 行(本体・6,269,440 B)。共有の表は触っていない。
- 試験の直し: `wave2-plugins.mjs` の第一波の除外に Sytrus を足した(`Plugins\Fruity\Generators\Sytrus`)。第二波の試験(`the_second_wave_is_160_dlls…`)は、Sytrus が lift/plugins に入ったため `the guest path is not unique` で落ちていた。新しい数は 130 folders・159 DLLs(131 本体・602,251,576 B)。wave の合計(748,158,976)は、lift/plugins と合わせた集合が同じなので変わらない。`cargo test -p fe-build -p fe-wasmgen` 全部通る。
- 測定の道具: `tools/p3-audio-probe/measure.mjs` の出力に `imageReads`(name・requested・atMs)を足した。さらに、ゲストがプロジェクトを開かない場合(`openProject` が失敗)は、`error` に理由を書き、終了コードを 0 にした。理由: 受け入れの `&&` の鎖が、デモの開かない失敗で止まり、Issue の「デモが開くかは判定に入れない」に反するため。それ以外の失敗は、終了コード 1 のまま。
- 受け入れの前提: 抽出の木(`lift/out-app/host`)は、ジョブの最初の HEAD の鍵で R2 から取る。plugin.tsv が HEAD に入るまで、Sytrus の DLL は木に無い。この run では、`host/plugins/Plugins/Fruity/Generators/Sytrus/Sytrus_x64.dll`(row 181 の取り出し。SHA-256 `e1395e97…`)の同じバイト列を、`host/lift/out-plugins/generators.sytrus.sytrus_x64.dll` と `host/lift/out-idle1/inputs/install/Plugins/Fruity/Generators/generators.sytrus.sytrus_x64.dll`(seed)に置いた。次の run は、鍵が変わるので、抽出が plugin.tsv から作る(この段は要らない)。
- boot-check: `engine-idle`・`standIn false`・`error null`・`elapsedMs 12303`(P1 の 10 秒は未達)。
- demo(測定の JSON): `synth`・`olbaid` とも `imageReads` に `generators.sytrus.sytrus_x64.c580000.dll` と `…c580000.wasm` が `requested: true`(約 19 s)。JSON に「couldn't be found」は無い。`error`: `timeout 180000 ms: the title of … (the title is "Song info")`。`state`: `exited`(`fle: CreateFileW: empty path` は #1414、switch 表は #1413)。
- ログの `missing-path` で Sytrus に関わるのは `GetFileAttributesW 2 "…\Sytrus\Fruityplug_x64.dll"`(存在の確認。読み込みの失敗ではない)。`LoadLibraryW` の成功は記録していない(成功の行は無い。読み込みの証拠は `image-read` の行)。
- 流れ A〜D(welcome-flow): 4 行とも `error` null・`engine-idle`・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。`elapsedMs` は 11,202〜11,421。`preBootBytes` は 312,207,408 B(上限 314,368,711 B 以内。前回の 64 MB は静的な見積もりだった)。
- `npm test --prefix app`: 71 files passed, 1406 tests passed。`spec/guard.test.sh`・`spec/workflow.test.sh` も通る。
- 変えたパス: `lift/`(plugins/sytrus・crates/fe-build の本体と試験)・`ci/`(extract-dlls.tsv の 1 行)・`tools/`(measure.mjs・wave2-plugins.mjs)・`pm-work/`。`lift/` と `ci/` に触れる push は main の `deploy.yml` を動かす。

## 次にやること(#1400 の残り。段階 1 の後)

1. 原因 2 の直し(#1413 へ): fe-decode の jumptab で、ガードの無い MSVC 表(`mov eX,[B+I*4+T]; add rX,B; jmp rX` で I と X が同じ)の N を、表の本体の終わり(既知の入口へ向かない所)で決める。あるいは、既知の入口に無い switch の飛び先を seed にする。どちらも lift 側の決めごと(decision 記録が要る)。
2. 原因 1・3(#1414 へ): Sytrus の Data を、plugin.tsv の行の種類(データ)で seed に足す。上限 512(boot の vfs-seed)と 128(`--lazy`)を確かめて、lazy の 128 件の上限を超えない形にする。
3. 受け入れの `cp -a lift/out-app/host-extra/resources/. …` は、`host-extra/resources` が在るときだけ通る(前回の失敗の原因。この run では木が在った)。木を置くのは `resources` のラベルの作業だけなので、ラベルの付け忘れに注意する。
4. 新しい plugin の最初の run は、木に DLL が無い(抽出の鍵が HEAD の鍵のため)。この件は PM に上げる: 受け入れの前に、plugin.tsv の DLL を木へ置く段を、ジョブに持たせるか、受け入れを直すか。
5. #1387(Riff machine の窓)は、#1413・#1414 が閉じた後に起動する。
