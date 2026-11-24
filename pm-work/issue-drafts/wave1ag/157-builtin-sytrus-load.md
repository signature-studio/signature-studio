---
draft: "157"
title: "内蔵の音源 Sytrus を起動の後に 1 プラグイン 1 wasm で届け、読み込めるようにする(Riff machine のプリセットが使う。読めないと警告が出て、窓が出ない)"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "022"
blocked_by: ["128", "129", "125"]
labels: ["plugin-builtin"]
size: L
priority: High
project: signature-studio/3
---

## 目的

内蔵の音源 Sytrus(`Plugins\Fruity\Generators\Sytrus\Sytrus_x64.dll`)を、起動の後の取得で 1 プラグイン 1 wasm として届け、FL が読み込めるようにする。この Issue の目標は次の 2 つ。
1. Sytrus の DLL の読み込み(`LoadLibraryW`)が成功し、「The plugin Sytrus couldn't be found or opened.」の警告が出ないこと。
2. Sytrus を使う demo(`Synth_dfr - Overcome - Sytrus.flp`・`Olbaid - Sytrus Essentials.flp`)の 2 件を File > Open で開くと、止まらず engine-idle のまま、少なくとも 1 件で音が出る(`nonSilentQuanta` が 1 以上)こと。

この Issue は、#1387(Piano roll の Riff machine が窓を出さない)の前提。Riff machine の 8 本のプリセットは、すべて Sytrus を使う。Sytrus が読めないと、Riff machine の窓は作られない。

## 背景

- 実測(#1387 の作業。run 45394680999。DPR 1 と 2.65 の 2 点): Riff machine の窓の前に、Sytrus の読み込みに失敗する警告(非表示の窓 `TMsgForm`「Warning」)が出る。警告を閉じるまで、Riff machine の窓(`TPRScoreCreatorForm`)は作られない。警告の本文は、ゲストのメモリの文字から読めた: 「The plugin Sytrus couldn't be found or opened. If it has to be unlocked, please make sure that you unlocked it correctly.」と「Tools > Manage cloud plugins. It's free!」。ログには `LoadLibraryW 126 ""` の失敗が出る。
- 抽出: `ci/extract-dlls.tsv` の Sytrus の行(`Sytrus_x64.dll`・`plugins/Plugins/Fruity/Generators/Sytrus/Sytrus_x64.dll`)と、`ci/plugin-builtin-prefixes.txt` の `Plugins\Fruity\Generators\` で、内蔵のプラグインの DLL は取り出される(#1321 で揃えた)。取り出した物は、作業ツリーの `lift/out-app/host-extra/plugins/` に入る。
- 一覧: `inputs/plugin-classes.tsv`(Sytrus の行。Generators・6,269,440 B・sha256 `e1395e97…`)と `inputs/plugin-hooks.tsv`(hook 点 3 点の判定は「高」)。
- seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)には、Sytrus の DLL の行が無い(`Plugins\Fruity\Generators\` の行は、`GM CCs.txt` の 1 件だけ。DLL ではない)。
- 既存の型: Fruity Balance は `77a7550f`・`212eaf59`(#1342)で、Fruity Filter と Fruity Delay 2 は `b0183525`(#1319)で、1 プラグイン 1 wasm として起動の後に取る形になった。足した場所は、`lift/harness/hyb/hyb-args.tsv` の行・`lift/harness/hyb/hybrid.cmd` の `--separate`・`lift/harness/web/pack.args` の `--lazy-dll` と `--lazy-file` の組・`lift/crates/fe-build/tests/plan/bundle.txt`・seed 表の行。`git show 212eaf59 --stat` と `git show b0183525 -- lift/harness/web/pack.args` で、同じ書き方を見られる。
- 親: #1323(E 内蔵インストゥルメント)の 1 群。Generators の DLL を抽出の表に揃える(#1321)は済み。
- 注意: #1372(Fruity Balance の入口と続きが、別の側で lift されている件)は、同じ hybrid の分割が原因。#1369 の後は、共有の表(`hyb-args.tsv`・`pack.args`・`bundle.txt` など)は生成物になり、この Issue は表を編集しない。この Issue は、#1369・#1372・#1373 が閉じた後に起動する(blocked by)。

## 読む物

- `git show 212eaf59 --stat`・`git show 212eaf59 -- lift/harness/hyb/hyb-args.tsv lift/harness/hyb/hybrid.cmd`(Balance を足した差分。型を真似る)
- `git show b0183525 -- lift/harness/web/pack.args lift/crates/fe-build/tests/plan/bundle.txt`(Filter と Delay 2 を足した差分)
- `lift/harness/hyb/hyb-args.tsv`・`lift/harness/hyb/hybrid.cmd`・`lift/crates/fe-wasmgen/src/hybrid.rs`
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`Plugins\Fruity\Effects\` の行の書式)
- `inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv`(Sytrus の行)
- `tools/p3-audio-probe/measure.mjs`(demo を開いて `nonSilentQuanta` を出す道具)
- `pm-work/findings/exoplanet-exit-1372.md`(#1372 の調査の記録。取り込まれていれば。Balance の入口と続きの話)

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
2. Sytrus は、`lift/plugins/sytrus/plugin.tsv`(`inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv` の行から)で足す。`plugin.tsv` の列・形は、#1369 の本文の「設計」(1)に従う(`key`・`guest`・`role`・`sha256`・`size`・`hook`・`args`)。`hook` は、`inputs/plugin-hooks.tsv` の Sytrus の 3 点が在れば `rva:` で書く。**共有の表(`hyb-args.tsv`・`hybrid.cmd`・`lowered-all.txt`・`pack.args`・`bundle.txt`・`seed` の表など)を編集しない。** 生成の段が、それらを作る。#1369 の本文の手順に従う。
3. 内蔵のプラグインの DLL を、受け入れの入力の場所に写す段が要るかを、Balance の経路(`fe-build bundle` が読む場所)で確かめ、足す。
4. 起動の前に取る量を増やさない: Sytrus の wasm は、起動の後に取る(起動の前の seed に入れない)。`preBootBytes` の値を報告に書く。
5. 停止が出たら(`ExitProcess`・longjmp の登録の欠け・`LoadLibraryW` の失敗など)、`pm-work/findings/exoplanet-exit-1372.md` の型(入口と続きが別の側)で同じ原因か確かめる。原因が同じなら、#1372 の直しの後に入る形で書く。
6. 受け入れを通す。通らなければ、作業ツリーを commit して終える。
7. commit する。push はしない。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-157 && cp "lift/out-app/host/resources/Data/Demo projects/Product demos/Synth_dfr - Overcome - Sytrus.flp" .tmp-157/synth.flp && cp "lift/out-app/host/resources/Data/Demo projects/Product demos/Olbaid - Sytrus Essentials.flp" .tmp-157/olbaid.flp && for f in synth olbaid; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-157/$f.flp" > "$RUNNER_TEMP/open-$f.json"; done && node -e 'const fs=require("fs");const L=["synth","olbaid"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&!JSON.stringify(r).includes("couldn\u0027t be found"))&&L.some((r)=>r.nonSilentQuanta>0&&r.quantaWhilePlaying>0)?0:1)'
```

判定の基準:
- 起動 1 回で、engine-idle・standIn が false・error が null。
- demo 2 件は、2 件とも `error` が null と engine-idle。JSON の文字の中に「couldn't be found」が無い(警告が出ていない)。少なくとも 1 件で `nonSilentQuanta` と `quantaWhilePlaying` が 1 以上。
- 起動の前に取る量: `preBootBytes` の値を報告に書く(314,368,711 B 以下)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 追加した行の数と大きさ(DLL の 6,269,440 B を含む)、取り出しの場所、2 件の demo の JSON の行、警告の有無、`preBootBytes`、変えたパス(`lift/`・`app/` を含む場合は、main の deploy が動く旨)、#1387 への引き継ぎ(Riff machine が窓を出すか)。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。Sytrus を読めるようにするのは既定の動作にする。
- 警告を隠す・閉じる・既定の答えを返す、などで読み込みを偽らない。読み込みが成功する形にする。
- `ci/` を変えない(接頭辞と取り出しの一覧は、作業者は変えない)。
- 製品のコード(`app/`・`lift/`)に Python を入れない。OCR を使わない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35〜46 分。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit して報告を書いて終える。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- 親: #1323(E 内蔵インストゥルメント)。
- blocked_by: #1369(プラグインを `lift/plugins/<名前>/plugin.tsv` の追加だけで足す基盤。この Issue はその形で Sytrus を足す)、#1372(Balance の入口と続きの件)と #1373(demo の素材)。#1369 の前に、共有の表を編集しない。
- この Issue が閉じると、#1387(Riff machine の窓)が起動する。
