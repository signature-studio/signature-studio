---
draft: "157"
title: "内蔵の音源 Sytrus を起動の後に 1 プラグイン 1 wasm で届け、読み込めるようにする(Riff machine のプリセットが使う。読めないと警告が出て、窓が出ない)"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "022"
blocked_by: ["128", "129", "125"]
labels: ["plugin-builtin", "resources"]
size: L
priority: High
project: signature-studio/3
---

## 目的

内蔵の音源 Sytrus(`Plugins\Fruity\Generators\Sytrus\Sytrus_x64.dll`)を、起動の後の取得で 1 プラグイン 1 wasm として届け、FL の `LoadLibraryW` が成功するようにする。この Issue は段階 1(読み込みまで)。目標は次の 2 つ。
1. Sytrus の DLL の読み込み(`LoadLibraryW`)が成功し、「The plugin Sytrus couldn't be found or opened.」の警告が出ないこと。
2. Sytrus の wasm が起動の後の取得で届き、エンジンが実モジュールの像を読んだ記録(`window.__app.imageReads` の Sytrus の行。`requested` が true)が在ること。

demo(`Synth_dfr - Overcome - Sytrus.flp`・`Olbaid - Sytrus Essentials.flp`)を最後まで開くことは、この Issue の受け入れに入れない。demo が止まる原因は、#1414(`Data\Default.dat` の空のパスと、Data の配信)と #1413(switch 表の飛び先)で直す。

この Issue が閉じると、#1413・#1414 が起動する。#1387(Riff machine の窓)は、#1413・#1414 が閉じた後に起動する。

## 背景

- 実測(#1387 の作業。run 45394680999。DPR 1 と 2.65 の 2 点): Riff machine の窓の前に、Sytrus の読み込みに失敗する警告(非表示の窓 `TMsgForm`「Warning」)が出る。警告を閉じるまで、Riff machine の窓(`TPRScoreCreatorForm`)は作られない。警告の本文は、ゲストのメモリの文字から読めた: 「The plugin Sytrus couldn't be found or opened. If it has to be unlocked, please make sure that you unlocked it correctly.」と「Tools > Manage cloud plugins. It's free!」。ログには `LoadLibraryW 126 ""` の失敗が出る。
- 抽出: `ci/extract-dlls.tsv` の Sytrus の行(`Sytrus_x64.dll`・`plugins/Plugins/Fruity/Generators/Sytrus/Sytrus_x64.dll`)と、`ci/plugin-builtin-prefixes.txt` の `Plugins\Fruity\Generators\` で、内蔵のプラグインの DLL は取り出される(#1321 で揃えた)。取り出した物は、作業ツリーの `lift/out-app/host-extra/plugins/` に入る。
- 一覧: `inputs/plugin-classes.tsv`(Sytrus の行。Generators・6,269,440 B・sha256 `e1395e97…`)と `inputs/plugin-hooks.tsv`(hook 点 3 点の判定は「高」)。
- seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)には、Sytrus の DLL の行が無い(`Plugins\Fruity\Generators\` の行は、`GM CCs.txt` の 1 件だけ。DLL ではない)。
- 既存の型: Fruity Balance は `77a7550f`・`212eaf59`(#1342)で、Fruity Filter と Fruity Delay 2 は `b0183525`(#1319)で、1 プラグイン 1 wasm として起動の後に取る形になった。足した場所は、`lift/harness/hyb/hyb-args.tsv` の行・`lift/harness/hyb/hybrid.cmd` の `--separate`・`lift/harness/web/pack.args` の `--lazy-dll` と `--lazy-file` の組・`lift/crates/fe-build/tests/plan/bundle.txt`・seed 表の行。`git show 212eaf59 --stat` と `git show b0183525 -- lift/harness/web/pack.args` で、同じ書き方を見られる。
- 親: #1323(E 内蔵インストゥルメント)の 1 群。Generators の DLL を抽出の表に揃える(#1321)は済み。
- 注意: #1372(Fruity Balance の入口と続きが、別の側で lift されている件)は、同じ hybrid の分割が原因。#1369 の後は、共有の表(`hyb-args.tsv`・`pack.args`・`bundle.txt` など)は生成物になり、この Issue は表を編集しない。この Issue は、#1369・#1372・#1373 が閉じた後に起動する(blocked by)。

## 前回の試行で分かったこと(2 回目の試行)

run 45518902053(00:37〜02:58 UTC。`failed`)の結果。数値は runner の値(nproc 4)。
- 受け入れの 1 段目で止まった: `cp: cannot stat 'lift/out-app/host-extra/resources/.'`。この Issue に `resources` のラベルが無く、`host-extra/resources` が作られなかった。ラベル `resources` を足した。
- 前回の差分は main に入っていない(push されなかった)。受理された部分は `pm-work/findings/sytrus-1400-attempt.diff` に残した。`lift/plugins/sytrus/plugin.tsv`・単位の段の直し(`lift/crates/fe-build/src/{main,plugins}.rs`・`tests/plan/engine.txt`)・`pm-work/findings/sytrus-1400.md` を含む。main に `git apply --3way` で当たる(確認済み)。
- 単位の段の直し: 静的な一覧(`lift/harness/two/plain-units.txt`・`ai-units.txt`)に無い `m<K>-<NN>.c` を gen の出力から求め、`compile-extra` でコンパイルし、link に足す。`cargo test`(fe-build 20 件ほか)が通った。engine で m11 の 7 単位がコンパイル・リンクされた。
- 一覧: `listed 18654, emitted ok 18654`。hook は `plugin-hooks.tsv` の 3 点(`rva:0x1b930,0x38b80,0x38ba0`)。omit `0x48cb70`(Sytrus の setjmp を呼ぶ関数。呼び出し元は 1 つ)。
- bundle の lazy: `Sytrus_x64.dll`(6,269,440 B)と `sytrus_x64.c580000.wasm`(21,486,577 B)。`preBootBytes` は 64,093,397 B(静的な値。上限 314,368,711 B 以内)。
- 起動 1 回: engine-idle・standIn false・error null。`elapsedMs` は 12,241 と 12,507 の 2 回(P1 の 10 秒は未達)。
- Sytrus の wasm は読まれた: `image-read: sytrus_x64.c580000.wasm requested=1`。JSON に「couldn't be found」は無い。`LoadLibraryW` の戻り値の記録は無く、未確認。
- demo(synth・olbaid。同じ停止): `fle: CreateFileW: empty path is not defined` で exit(`state: exited`)。180 秒の待ちで、題名は "Song info" のまま。原因は 2 つ。`Data\Default.dat` が無い(#1414)と、switch 表の飛び先 `0xc9cf273`(RVA `0x44f273`。基底 `0xc580000`)(#1413)。
- `Data` の 109 件を全部 seed に入れると、boot の段で `more than 512 seed files` で止まる。`--lazy` は 128 件まで(#1414)。
- `ci/extract-dlls.tsv` の 181 行目(Generators の Sytrus)は、残すと二重に取り出される(害は無かった)。この Issue で消す。

## 読む物

- `pm-work/findings/sytrus-1400-attempt.diff`(前回の差分。受理された部分)
- `git show 212eaf59 --stat`・`git show 212eaf59 -- lift/harness/hyb/hyb-args.tsv lift/harness/hyb/hybrid.cmd`(Balance を足した差分。型を真似る)
- `git show b0183525 -- lift/harness/web/pack.args lift/crates/fe-build/tests/plan/bundle.txt`(Filter と Delay 2 を足した差分)
- `lift/crates/fe-build/src/main.rs`・`lift/crates/fe-build/src/plugins.rs`(単位の段。`check_units`)
- `lift/harness/two/plain-units.txt`・`lift/harness/two/ai-units.txt`(静的な単位の一覧)
- `app/src/main.ts`(`imageReads`。実モジュールの像の読みの記録)
- `inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv`(Sytrus の行)
- `lift/plugins/fruity-balance/plugin.tsv`(`plugin.tsv` の列の形: key・guest・role・sha256・size・hook・args・omit)
- `app/scripts/lib/plugin-rows.mjs`(`plugin.tsv` の検査)
- `tools/p3-audio-probe/measure.mjs`(demo を開いて、`imageReads` と `engineLog` を出す道具)

## やること(段階 1)

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 前回の差分を当てる: `git apply --3way pm-work/findings/sytrus-1400-attempt.diff`。当たらない箇所は、同じ目的で書き直す。単位の段は、engine の作り直しで m11 の 7 単位がコンパイル・リンクされるまで確かめる。
2. `lift/plugins/sytrus/plugin.tsv` を、Sytrus の行(`inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv` の値。列は上の一覧の形)で足す。共有の表(`hyb-args.tsv`・`hybrid.cmd`・`lowered-all.txt`・`pack.args`・`bundle.txt`・seed の表など)は編集しない。生成の段が作る。
3. `ci/extract-dlls.tsv` の 181 行目(Generators の Sytrus の行)を消す。`plugin.tsv` と二重になるため。`app/scripts/lib/plugin-rows.mjs` は、取り出し表との重複を拒否しないので、消さなくても動く。消す理由は、二重の取り出しを残さないため。
4. 測定の道具(`tools/p3-audio-probe/measure.mjs`)の出力に、`window.__app.imageReads` の Sytrus の行(`name`・`requested`)を足す。`LoadLibraryW` の戻り値の記録が要るなら、エンジンの記録を 1 行足す(製品の変更。環境変数・`#ifdef` は足さない)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。
6. commit する。push はしない。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-sytrus && cp "lift/out-app/host/resources/Data/Demo projects/Product demos/Synth_dfr - Overcome - Sytrus.flp" .tmp-sytrus/synth.flp && cp "lift/out-app/host/resources/Data/Demo projects/Product demos/Olbaid - Sytrus Essentials.flp" .tmp-sytrus/olbaid.flp && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-sytrus/synth.flp" > "$RUNNER_TEMP/open-synth.json" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-sytrus/olbaid.flp" > "$RUNNER_TEMP/open-olbaid.json" && node -e 'const fs=require("fs");const L=["synth","olbaid"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));const read=(r)=>Array.isArray(r.imageReads)&&r.imageReads.some((x)=>/sytrus/i.test(x.name)&&x.requested===true);process.exit(L.every((r)=>read(r)&&!JSON.stringify(r).includes("couldn\u0027t be found"))?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 起動 1 回で、engine-idle・standIn が false・error が null(boot)。
- synth と olbaid の測定の出力で、`imageReads` に Sytrus の像が `requested: true` で在る。JSON の文字の中に「couldn't be found」が無い(警告が出ていない)。`LoadLibraryW` の戻り値を記録したなら、その値も報告に書く。
- 流れ A〜D(`welcome-flow.mjs`)の 4 行: 各行で `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。各行の `elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。
- demo が最後まで開くかどうかは、この Issue の判定に入れない(`error` と `state` は報告に書くだけ)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 前回の差分を当てたか(当たらなかった箇所)、追加した行の数と大きさ(DLL の 6,269,440 B を含む)、`ci/extract-dlls.tsv` の 181 行目を消したこと、2 件の demo の JSON の `imageReads` の行、警告の有無、`LoadLibraryW` の戻り値の記録の有無、demo の `error`・`state`(報告だけ)、`preBootBytes`、変えたパス(`lift/`・`app/` を含む場合は、main の deploy が動く旨)、#1387 への引き継ぎ(Riff machine が窓を出すか。ただし #1387 は #1413・#1414 の後)。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。Sytrus を読めるようにするのは既定の動作にする。
- 警告を隠す・閉じる・既定の答えを返す、などで読み込みを偽らない。読み込みが成功する形にする。
- `ci/` は、`ci/extract-dlls.tsv` の 181 行目を消す以外は変えない。接頭辞と取り出しの一覧は、作業者は足さない。
- 名前の規則(訂正): 禁止されているのは、AI のベンダー名(`guard/scan.sh` が検査する)と、Cloudflare の識別子だけ。FL の製品名・`Image-Line` のインストール先・`%FLStudioFactoryData%` などの変数名は、ファイル・Issue の本文に書いてよい(`lift/crates/fe-build/src/plugins.rs` などが既に書いている)。
- 製品のコード(`app/`・`lift/`)に Python を入れない。OCR を使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35〜46 分。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit して報告を書いて終える。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- 親: #1323(E 内蔵インストゥルメント)。
- blocked_by: #1369(プラグインを `lift/plugins/<名前>/plugin.tsv` の追加だけで足す基盤)、#1372(Balance の入口と続きの件)、#1373(demo の素材)。いずれも閉じている。
- この Issue が閉じると、#1413(switch 表の飛び先)と #1414(Sytrus の Data)が起動する。#1387(Riff machine の窓)は、#1413・#1414 が閉じた後に起動する。
