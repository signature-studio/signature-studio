---
draft: "158"
title: "P3 の測定に使う、音が出る入力を、FL の操作で作る"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["129", "128"]
labels: ["audio", "verify", "resources"]
size: M
priority: High
project: signature-studio/3
---

## 目的

P3 の測定(#{{004}})の入力に、音が出る入力が無い。インストーラの demo を開いた前回の測定は、開けた 40 件のどれも `nonSilentQuanta` が 0 だった(無音)。曲を選ぶ方式は成り立たないと分かったので、目的を変える。

この Issue の目標は次の 2 つ。
1. FL の操作(クリックと再生)で、音が出る入力を作る。Welcome の Default(Default プロジェクト)の Channel Rack のステップを実際に押して、Play を押すと、`nonSilentQuanta` が 0 より大きいこと。
2. その手順を `inputs/p3-project.tsv` に書き、道具 `tools/p3-audio-probe/measure.mjs` に `--make-sound`(操作で音を作る)を足すこと。#{{004}} は、同じ手順で音の出る入力を再現する。

## 背景

- 前回(run 45608455537。job「作業 #1401」)の結果: インストーラの `Data\Demo projects\` の `.flp` 46 件のうち 40 件を開いた。40 件とも `nonSilentQuanta` が 0。5 件は開く途中でエンジンが停止した。1 件はインストーラの木に無い。表は `pm-work/findings/p3-demo-scan-1401.md`。
- 前回の判定の (a)「`notInInstaller` が 0 で、`nonSilentQuanta` が 0 より大きい曲」は、どの曲も満たさない。`notInInstaller` の件数は、素材の欠落ではなく、エンジンの既定の探索(`.ZGR` を `.wav` の前に探す・`.NFO`・Plugin database・`*.*`・ユーザーのフォルダ)を多く含むため。
- 最小の曲(`nucleon - Harmless Galactic`)は、DLL が 23 件欠け、「Problems loading the project」が出る。DLL の欠落は `notInInstaller` の数に入らない。無音の原因の一つは、DLL(プラグイン)の欠落と推定する(確かめていない)。
- この VM は音を出せる。実機の測定(ブランチ #1356。`pm-work/findings/perf-windows-browser/README.md`)では、別の評価プロジェクトで、非無音の quanta が 6,892 あった。無音の原因は、この VM ではなく、入力の側と推定する。
- PM の見立て(確かめる): Default プロジェクトの Channel Rack の Kick・Clap・Hat・Snare は、FL 内蔵の Sampler で、素材 `Data\Patches\Packs\Legacy\Drums\Dance\Basic 808 *.wav`(4 件)を使う。
- 配信: `ci/resources-prefixes.txt` の 8 行目 `Data\Patches\Packs\Legacy\Drums\Dance\` が、この 4 件を含む。接頭辞は既に在るので、`ci/` は変えない。4 件の大きさ(`inputs/fl-installer-files.tsv`): Clap 46,492 B・HiHat 28,180 B・Kick 74,468 B・Snare 32,992 B。
- 配信の後の届き方: `lift/harness/seeds/vfs-seed-idle1.tsv` と `lift/harness/web/pack.args` には、同じ Dance フォルダの `DNC_Clap` などの行は在るが、`Basic 808` の行は無い(`vfs-seed-g8.tsv` にだけ在る)。届いているかは、作業の最初に確かめる。

## 前回までの試行で分かったこと

(run 45608455537。job「作業 #1401」。判定: 未確認。`failed`。作業の差分(`scan-demos.mjs`・`measure.mjs` の 2 点)は work.diff に残り、push していない)

1. 受け入れの 1 段目が落ちた: `cp: cannot stat 'lift/out-app/host-extra/resources/.'`。`resources` のラベルが無いので、ジョブに `host-extra` が作られなかった。この版の本文で、ラベル `resources` を付けた。
2. 1 段目より後は、作業者が手で実行した: `npm run build --prefix app`・`cargo build`・`fe-build engine`(約 23 分)・`fe-build bundle`。起動確認は `engine-idle`・`standIn` false・`error` null・`elapsedMs` 8,472。
3. 差分: `tools/p3-audio-probe/scan-demos.mjs`(新規。1 曲ずつ `measure.mjs` を起動し、JSON の `songs` を出す)。`measure.mjs` の 2 点(`notInInstallerList` を全件にした。開く途中の停止で 180 秒を待たない)。
4. 走査は 28 分。46 件のうち 40 件を開いた。40 件とも `nonSilentQuanta` が 0。`underrunDelta` は、0 の曲と、3,133〜3,449 の曲がある。
5. 開く途中で停止した 5 件(`notInInstaller` の件数): DEDZII(66)・Mehran Abbasi - With You(18)・Olbaid - Sytrus Essentials(44)・Synth_dfr - Overcome(44)・Treben - Umziky(153)。エラーは「the engine is exited during the open」。
6. `notInInstaller` の最小は 23(`nucleon - Harmless Galactic`)。最大は 16,162(Astes - Bien Duro)。
7. 最小の曲の 23 件の内訳: `Basic 808` の `.ZGR`(4 件。実物は `.wav`)、Plugin database の `.NFO` 5 件・`.Plugins.ini`・`*.*`、`C:\Windows\Fonts\OpenSans-Regular.ttf`、ユーザーのフォルダの探索(Settings・Skins・Presets)。
8. 同じ曲で DLL が 23 件欠け、「Problems loading the project」(`TMissingPluginsForm`)が出た。DLL は `notInInstaller` の数に入らない。
9. 判定は、受け入れの (a)(曲が鳴る)も (b)(選べない)も、この数え方では決められなかった。PM は、目的を「曲を選ぶ」から「FL の操作で音の出る入力を作る」に変えた。
10. `inputs/p3-project.tsv` は作られていない(前回は正しい。音の出る曲が無かったため)。
11. 実機の参考: 非無音の quanta 6,892(#1356。別の評価プロジェクト)。この VM は音を出せる。
12. `Basic 808` の 4 件は、`ci/resources-prefixes.txt` の 8 行目の接頭辞の内に在る。`vfs-seed-idle1.tsv` と `pack.args` には行が無い(`DNC_Clap` の行は在る)。
13. 道具 `tools/p3-audio-probe/measure.mjs` の出力: `nonSilentQuanta`・`underrunDelta`(#{{004}} の定義と同じ)・`quantaWhilePlaying`・`underrunQuanta`・`projectMissing`・`notInInstaller`。
14. 受け入れの 2 段目以降は、前回の作業者が手で行った(build・bundle・起動確認・走査)。最後の判定の式は、「選べない」の分岐で終了 0 を確かめた(報告の記録)。
15. 前回の記録(表 46 件)は、`pm-work/findings/p3-demo-scan-1401.md` に写した。判定には入れない。
16. `underrunDelta` が 0 でない曲は 8 件(3,133〜3,449)。どれも `nonSilentQuanta` は 0 だった。音は出ていないのに、underrun だけが起きている(前回の観測)。
17. 開く途中で止まった 5 件は、`Song info` の窓が出た後に `exited` になった(error は「the engine is exited during the open」)。`measure.mjs` に早期の終了を入れた後は、止まる曲が約 30 秒で終わる。
18. `scan-demos.mjs` の出力は JSON の 1 行(`{"songs":[...]}`)。46 件のうち、`error` が null の曲は 40 件。
19. 前回の判定(受け入れの (a)・(b))は、どちらも決められなかった。(a) は 0 曲。(b) の最小の曲 23 件は、探索の数を多く含む(推定。素材の欠落ではない)。
20. 最小の曲 `nucleon - Harmless Galactic` の 23 件の `.ZGR` の 4 件は、`Basic 808` の実物(`.wav`)が在る。`.ZGR` の探索が先に来るため、欠落として数えられた。
21. 前回の run の 1 段目は `cp -a lift/out-app/host-extra/resources/.` で落ちた。その後の段は、作業者が手で同じ順に実行した(build・bundle・boot-check・走査)。
22. 前回の作業者は、探索の種類を分けて数え直す別の Issue を勧めた。PM は、その前に音の出る入力を作ることを先にする決定をした(この版の目的)。
23. 前回の表(46 件)の一覧は、`pm-work/findings/p3-demo-scan-1401.md` に在る。この Issue の判定には入れない。
24. 前回の走査は、1 曲ごとに `measure.mjs` を起動し直した。46 件で 28 分(runner の値)。
25. 前回の列の案(`inputs/p3-project.tsv` の曲名・パス)は、この版で「名前・手順」に変えた。手順は、FL の操作の順番を書く(曲のパスではない)。
26. `#1335` の前回の測定(51 件の候補のうち開けた 30 件)は、`pm-work/findings/p3-blocks-1335.md` に在る。今回の 46 件とは別の集合。
27. Welcome の確認ダイアログ(「Help us improve …」)は、流れ A と同じ「No thanks」で閉じる(`tools/smoke/welcome-flow.mjs`)。この Issue の `--make-sound` も、同じ方式で閉じる。
28. 前回の作業の差分(`scan-demos.mjs`・`measure.mjs` の 2 点)は push していない。この版の受け入れは `scan-demos.mjs` を使わず、`--make-sound` だけを使う。
29. 前回の作業者の報告の結論: 「選べない」(受け入れの (b))。`inputs/p3-project.tsv` は作らなかった。この版は、その結論を前提にして、目的を変えた。
30. 前回の 40 件の `nonSilentQuanta` 0 は、道具の 10 秒の測定の値。別の入力で鳴るかは、この測定からは判らない。

## 読む物

- `tools/p3-audio-probe/measure.mjs`(`nonSilentQuanta`・`underrunDelta`・`fromAudio`・`pickAudio`。`--make-sound` を足す所。引数の書式は先頭の注釈)
- `tools/smoke/welcome-flow.mjs`(Welcome の確認ダイアログ「No thanks」と、Welcome の Default の行の操作。窓の表から座標を取る方式)
- `tools/piano-roll-tools/measure.mjs`(窓の表(`windowNew`)と画素の読みの例)
- `ci/resources-prefixes.txt`(8 行目)・`inputs/fl-installer-files.tsv`(`Basic 808 *.wav` の 4 行)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`DNC_Clap` の行。書式の見本)・`lift/harness/web/pack.args`(`--lazy-file` の書式の見本)
- `pm-work/findings/p3-demo-scan-1401.md`(前回の表)・`pm-work/findings/p3-blocks-1335.md`(前回の測定の表)・`pm-work/findings/perf-windows-browser/README.md`(実機の非無音 quanta)

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
2. 素材の確かめ: 起動の後の VFS に、`Basic 808 *.wav` の 4 件が届くかを確かめる(`measure.mjs` の `missing-path` の行と、読んだ記録)。届かなければ、`lift/harness/seeds/vfs-seed-idle1.tsv` と `lift/harness/web/pack.args` に、`DNC_Clap` と同じ形の行を 4 件足す(`@lazy-file:`。大きさと sha256 は `inputs/fl-installer-files.tsv` の値)。`ci/resources-prefixes.txt` は変えない。
3. 操作の道具: `tools/p3-audio-probe/measure.mjs` に `--make-sound` を足す。Welcome の確認ダイアログを閉じ(流れ A と同じ方式)、Welcome の Default を開き、Channel Rack のステップを押し、Play を押し、10 秒後に `nonSilentQuanta` と `underrunDelta` を読む。道具の引数であり、製品の旗ではない。
4. ステップのボタンの位置は定数にしない。Channel Rack の窓の矩形(窓の表から取る)の中で、Kick の行の候補の点を、窓の幅と高さの比率で並べ、1 点ずつクリックする。その矩形の画素のハッシュが変わったら、点灯したとみなす。点灯した数を `makeSound.litSteps` に出す。OCR は使わない。
5. Play を押し、Space で再生して 10 秒測る。`nonSilentQuanta` が 0 のままなら、原因を切り分けて報告に書く: (a) 素材が読めたか(`vfs-open` の記録)、(b) チャンネルが有効か(行の点灯)、(c) Mixer のマスターが有効か、(d) `quantaWhilePlaying` は増えるのに `nonSilentQuanta` が 0 か(engine が 0 を書いている)。
6. `inputs/p3-project.tsv` を作る(タブ区切り。1 行。列: 名前・手順)。名前は `default-channel-rack-steps`。手順は、Welcome の Default → Channel Rack の Kick 行のステップの押す順 → Play の順に書く。#{{004}} は、この記録で同じ操作を再現する。
7. 受け入れを通す。通らなければ、作業ツリーを commit して終える。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --make-sound > "$RUNNER_TEMP/sound.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").pop());const ok=r.error===null&&r.state==="engine-idle"&&!!r.makeSound&&r.makeSound.litSteps>=1&&Number.isFinite(r.nonSilentQuanta)&&r.nonSilentQuanta>0&&Number.isFinite(r.underrunDelta);process.exit(ok?0:1)' "$RUNNER_TEMP/sound.json" && node -e 'const t=require("fs").readFileSync("inputs/p3-project.tsv","utf8");process.exit(/^default-channel-rack-steps\t/m.test(t)?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- `--make-sound` の JSON(1 行): `error` が null、`state` が engine-idle、`makeSound.litSteps` が 1 以上(押して点灯した Channel Rack のステップの数)、`nonSilentQuanta` が 1 以上、`underrunDelta` が数。
- `inputs/p3-project.tsv` に、名前 `default-channel-rack-steps` の行が在る(タブ区切り。手順の列が空でない)。
- `tools/smoke/welcome-flow.mjs` の流れ A〜D(4 行)。判定は #{{147}} と同じ(各行の `error`・`state`・`standIn`・`trapped`・`hostErrors`・`opened`、流れ B と D の `welcomeClosed`、4 流れの `elapsedMs` の中央値が 12,500 以下、`preBootBytes` の最大が 314,368,711 以下)。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 素材の確かめ(届いたか。足した行の数)、`--make-sound` の JSON の 1 行、点灯した数、`nonSilentQuanta` と `underrunDelta`、無音の時は切り分けの 4 点の結果、流れ A〜D の 4 行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。道具の引数(`--make-sound`)は製品の旗ではない。
- インストーラに無い素材を、どこからも取らない(配信の対象外)。
- 依頼者の個人のプロジェクトは使わない。
- Python・OCR を使わない。
- `inputs/fl-installer-files.tsv` と `ci/` は読むだけ。`lift/harness/seeds/`・`lift/harness/web/pack.args` は、素材の行を足す時だけ変える。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。

## 範囲外

- インストーラに無い素材の配信(FL がオンラインで取る素材)。
- P3 の測定そのもの(#{{004}})。この Issue は、音の出る入力と、その手順を決めるだけ。
- 曲(demo)を選ぶこと。前回の結果で、判定が成り立たないと分かった。

## 他のタスクとの関係

- blocked_by: #{{129}}(素材の配信。閉じた)と #{{128}}(Balance の入口。閉じた)。
- #{{004}}(#1335・P3 の測定)は、この Issue の `inputs/p3-project.tsv` の手順と、`--make-sound` を入力に使う。
