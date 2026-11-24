---
draft: "129"
title: "demo 2 件(DEDZII・Mehran Abbasi)の素材を、起動の後の取得で配信し、P3 の入力に音が出る demo があるかを確かめる"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["121", "122", "128"]
labels: ["resources", "audio"]
size: M
priority: High
project: signature-studio/3
---

## 目的

P3 の測定(#1335)の入力に、音が出る demo が無い。前回の測定(51 件の候補のうち開けた 30 件)は、すべて `nonSilentQuanta` が 0 だった。原因の一部は、demo の素材が配信されていないこと。

この Issue の目標(PM の決定。前回の目標の「`nonSilentQuanta` が 1 以上」は、この Issue から外した。音が出る demo の選定は #1401 が行う):
1. インストーラに在る素材を、全て配信する。demo 2 件(DEDZII - NIGHT TERROR・Mehran Abbasi - With You)を開いたとき、ゲストが要求して見つからなかった素材のうち、インストーラの一覧(`inputs/fl-installer-files.tsv`)に在る物が 0 件(`missingInInventory` = 0)。
2. 見つからなかった残りの素材の内訳を出す: インストーラの一覧に無い物の件数(`notInInstaller`)と、先頭 10 件(JSON と報告に出す)。
3. DEDZII が開く(`error` が null で `engine-idle`)。Mehran Abbasi - With You は、#1372 の後に開く(#1372 が open の間は、この Issue は起動しない)。

## 背景

- インストーラの素材は、`Data\Patches\Misc\Used by demo projects\<曲名>\` に、曲ごとのフォルダで置かれている(`inputs/fl-installer-files.tsv`。623 件・約 82 MB。曲のフォルダは 26 件)。
- 2 フォルダを選んだ理由: 一覧で小さい(DEDZII 4 件・877,543 B。Mehran Abbasi - With You 5 件・694,532 B。合計約 1.57 MB)。DEDZII は、#1365 の段階 1 で、File > Open で開けて再生が進むことが確かめられた。
- 配信は 2 段。(a) 取り出し: `ci/resources-prefixes.txt` の接頭辞の下の物を、ビルドの前に `lift/out-app/host-extra/resources/` に取り出す。(b) 配信: 起動の後に取る物には、seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)の `@lazy-file:` の行と、`lift/harness/web/pack.args`・`lift/crates/fe-build/tests/plan/bundle.txt` の `--lazy-file` の組が要る。(a) だけでは、ゲストの仮想のファイルシステムに載らず、見つからない(`pm-work/workflow-design.md` の「接頭辞で取った物は」)。
- 欠けた素材の記録: エンジンのログの `missing-path: <api> <err> "<path>"` の行。パスは `C:\Program Files\Image-Line\FL Studio 21\`(または `%FLStudioFactoryData%\`。同じ場所)の下にある。一覧との照合は、この root を外した相対パスで、大文字小文字を無視して行う。
- 配信の仕組みは #1366 と同じ。

## 前回までの試行で分かったこと

- 1 回目: run 45392098052。`fe-build bundle` の seed の段で止まった(9 件が作業ツリーに無かった。接頭辞の一覧に 2 行が無かったため。main の c43f05ee で足した)。
- 2 回目: run 45436255034(16:23〜19:02 UTC)。判定は「差し戻し」(受け入れの後段が Mehran の測定で exit 1)。push していない。commit は `bfb28a16`(作業者の環境)。runner の値。nproc 4、loadavg 1.6〜1.9。
  - 9 件は配信された(seed 表の 9 行が `@lazy-file:`)。DEDZII の `projectMissing` に 9 件は一件も出ない。
  - 前回の差分(`work.diff`)には、Mehran の `.ogg` の修正が在る(`app/scripts/pack-assets.mjs`・`app/test/unit/pack-assets-fruity.test.ts`)。manifest の URL に生の `#` が入り、`#` より後ろが fragment として切られて読めなかった。`#` `%` `?` を `%XX` にする。この修正は取り込んで残す。
  - DEDZII: 開いた(`error` null・`engine-idle`・`quantaWhilePlaying` 5765・`underrunQuanta` 366)。`nonSilentQuanta` は 0 で、無音。
  - Mehran: 開けない。`.ogg` を読んだ後、遅延読み込みの `fruity_balance`(`fruity_balance_x64.6800000.dll` と `.wasm`)の後で `ExitProcess`(exit 0)。窓の題が「Song info」のまま止まる。#1372 の範囲。
  - DEDZII のゲストの欠落は 205 件(distinct)。報告の内訳: 22 件はインストーラの一覧に在るが配信されていない(例: `Data\Patches\Packs\Legacy\Drums\Dance\Basic 808 Kick.wav`)。183 件は一覧に無いとされた。**この 183 件の分類は確かめられていない**: 報告の例 `%FLStudioFactoryData%\Data\Patches\Packs\Drums (ModeAudio)\...` の相対パスは、一覧に在る(`Data\Patches\Packs\Drums (ModeAudio)\Hi Hats\MA TapDance Hat.wv`)。照合の読み替えの誤りの可能性がある。22 件の一覧は、保存された記録に無い。
  - 一覧の読み(読むだけ。この Issue の前に確かめた): `Data\Patches\Packs\Legacy\Drums\Dance\` は接頭辞の一覧に在り、その下の 33 件のうち 21 件(`DNC_*.wav`)は seed 行を持つ。残りの 12 件は seed 行が無く、届かない(`Basic 808 Clap・HiHat・Kick・Snare.wav`、`CB_Clap・Hat・Kick・Snare.wav`、`Clap Basic.wav`・`Hat Basic.wav`・`Kick Basic.wav`・`Snare Basic.wav`。計 448,724 B)。22 件の一部と見てよいが、未確認。
- 9 件が無音の原因では無かったことは、配信の後も DEDZII が無音だったことで分かった。無音の原因は、インストーラに無い素材か、seed 行の無い素材のどちらか(未確認。下の「やること」の 3 で分ける)。

## 読む物

- `tools/missing-paths/measure.mjs`(`missing-path` の行の読み方。3 回の起動で行を取る道具)
- `tools/p3-audio-probe/measure.mjs`(開いて `nonSilentQuanta`・`projectMissing` を出す道具。ここに内訳を足す)
- `inputs/fl-installer-files.tsv`(guest 列・bytes 列・sha256 列)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`@lazy-file:` の行の形。`DNC_*.wav` の行が手本)
- `lift/harness/web/pack.args`、`lift/crates/fe-build/tests/plan/bundle.txt`
- `ci/resources-prefixes.txt`(16 行。Misc の 2 行を含む)
- `pm-work/workflow-design.md`(「抽出物の接頭辞の一覧は」の段落)

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
2. 取り込まれた前回の差分(seed 表の 9 行・`pack.args`・`bundle.txt`・`app/scripts/pack-assets.mjs` と その試験)が、二重になっていないことを確かめる(`git diff HEAD --stat`・`grep -c`)。衝突の印が在れば解消する。`ci/resources-prefixes.txt` は、main の版の 16 行のまま。
3. 道具 `tools/p3-audio-probe/measure.mjs` に、欠落の内訳を足す(既存の項目は残す)。エンジンのログの `missing-path` の行の全て(`.wav` に限らない)を読み、重複を除いた件数を `missingDistinct` に出す。各パスの root を外し、大文字小文字を無視して `inputs/fl-installer-files.tsv` の guest 列と照合する。在る物の数を `missingInInventory`、無い物の数を `notInInstaller`、無い物の先頭 10 件を `notInInstallerFirst10` に出す。
4. 一覧に在って届いていない物を、届ける。各パスについて: (a) 接頭辞の外に在るなら、その folder(`\` で終わる)を `ci/resources-prefixes.txt` に足す(合計 30 MB 以下。`spec/workflow.test.sh` の接頭辞の検査を、足した数に合わせる。先に red にする)。(b) seed 表に `@lazy-file:` の行を足す(行の形は `DNC_*.wav` の行と同じ)。(c) `lift/harness/web/pack.args` と `lift/crates/fe-build/tests/plan/bundle.txt` に、同じ数の組を足す。大きさと SHA-256 は一覧の値。空の代替(空ファイル・0 埋め)は置かない。最初の候補は、上の 12 件(Dance の seed 行の無い物)。残りは、道具の出力から取る。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。
6. commit する。push はしない(ジョブの最後の判定が通ったときだけ push される)。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-129 && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-129/dedzii.flp && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/Mehran Abbasi - With You.flp" .tmp-129/mehran.flp && for f in dedzii mehran; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-129/$f.flp" > "$RUNNER_TEMP/open-$f.json"; done && node -e 'const fs=require("fs");const L=["dedzii","mehran"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));process.exit(L[0].error===null&&L[0].state==="engine-idle"&&L.every((r)=>r.missingInInventory===0&&typeof r.notInInstaller==="number"&&Array.isArray(r.notInInstallerFirst10))?0:1)'
```

判定の基準:
- 起動 1 回で、engine-idle・standIn が false・error が null。
- DEDZII と Mehran の両方で、`missingInInventory` が 0(数値)。`notInInstaller` が数値で、`notInInstallerFirst10` が配列。
- DEDZII の `error` が null で、`state` が engine-idle。Mehran の `error` は判定に入れない(#1372 の範囲)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 追加した物の相対パス・大きさ・SHA-256(一覧の値)、変えた接頭辞(無ければ「無し」)、DEDZII と Mehran の `missingDistinct`・`missingInInventory`・`notInInstaller`・`notInInstallerFirst10`、DEDZII の `nonSilentQuanta`(参考。判定には入れない)、取り込みの衝突の有無、`app/` を変えたこと(push すると `deploy.yml` が動く)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。遅延は既定の動作にする(オプトインを作らない)。
- 空の代替(空ファイル・0 埋め)を置かない。SHA-256 と大きさは、一覧(インストーラの実物から取った値)と照合したものだけを置く。
- `inputs/fl-installer-files.tsv` は読むだけ。
- インストーラに無い素材を、どこからも取らない(配信の対象外)。
- 依頼者の個人のプロジェクトは使わない。
- ベンダー名・Cloudflare の識別子・インストール先の既定の値を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 32〜46 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。`sleep`・`timeout` をコマンドの先頭に書かない。

## 範囲外(この Issue では直さない)

- インストーラに無い素材(FL がオンラインで取る素材)。その件数と先頭 10 件を出すまで。配信はしない。
- 他の曲の素材(`Used by demo projects` の残りの 24 曲)。音が出る demo の選定は #1401。
- Mehran の `fruity_balance` の遅延読み込み(#1372)。
- FLEX・Emphasizer の読み込み(#1322・#1323 の範囲)。

## 実物の置き場

- 取り出した実物は `lift/out-app/host-extra/resources/` に在る。受け入れの先頭で、その写しを `lift/out-app/host/resources/` に置く(`cp -a`)。前回の 1 回目の止まりは、`host/resources` の下の `Misc` の 9 件が取り出されていなかったための ENOENT だった(接頭辞の一覧の 2 行が無かった。上の「前回までの試行」)。

## 他のタスクとの関係

- blocked_by: #1365・#1366(いずれも閉じている)。**#1372(Balance の入口の setjmp の登録。Mehran の open に必要。open の間は、この Issue は起動しない)**。
- 後続: #1401(音が出る demo を決める。この Issue の `notInInstaller` の結果を使う)。#1335(P3 の測定)は #1401 を待つ。
- #1372(Exoplanet)・#1387(Riff machine)・#1400(Sytrus)も、`pack.args`・`bundle.txt`・seed 表を変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
