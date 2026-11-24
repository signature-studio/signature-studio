---
draft: "129"
title: "demo 2 件(DEDZII・Mehran Abbasi)の素材を、起動の後の取得で配信し、P3 の入力に音が出る demo があるかを確かめる"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["121", "122"]
labels: ["resources", "audio"]
size: M
priority: High
project: signature-studio/3
---

## 目的

P3 の測定(#{{004}}・#1335)の入力に、音が出る demo が無い。前回の測定(51 件の候補のうち開けた 30 件)は、すべて `nonSilentQuanta` が 0 だった。原因の 1 つは、demo の素材が配信されていないこと(推定。下の背景)。

この Issue の目標: demo の素材 2 件分(DEDZII・Mehran Abbasi - With You)が、起動の後の取得で読めること。その 2 件を File > Open で開いて、少なくとも 1 件で `nonSilentQuanta` が 1 以上になること。

## 背景

- インストーラの素材は、`Data\Patches\Misc\Used by demo projects\<曲名>\` に、曲ごとのフォルダで置かれている(`inputs/fl-installer-files.tsv`。623 件・約 82 MB。曲のフォルダは 26 件)。
- この下の素材は、配信の接頭辞(`ci/resources-prefixes.txt`)にも、起動の seed(`lift/harness/seeds/vfs-seed-idle1.tsv`)にも無い。#{{122}} の範囲外として残した分。
- 推定: demo の .flp は、この素材を参照する。素材が無いチャンネルは無音になる(テンプレート Blocks の「Problems loading the project」の窓と同じ症状。#{{122}} の前回の試行)。
- 2 フォルダを選んだ理由: 一覧で小さい(DEDZII 4 件・877,543 B。Mehran Abbasi - With You 5 件・694,532 B。合計約 1.6 MB)。DEDZII は、#{{121}} の段階 1 で、File > Open で開けて再生が進むことが確かめられた(前回の試行の実測)。
- 未確認: 2 件とも、素材が無いことで無音になるか。欠けた物は、窓の内容か、道具の `projectMissing`(#{{122}} の差分の道具が記録する版)で確かめる。Mehran Abbasi - With You が開けるかも未確認。
- 配信の仕組みは #{{122}} と同じ(`@lazy-file:` の印、`lift/harness/web/pack.args` と `lift/crates/fe-build/tests/plan/bundle.txt` の `--lazy-file`、接頭辞は `\` で終わるフォルダ単位)。#{{122}} の作業の結果(main に入った版)を読んでから始める。

## 読む物

- `pm-work/issue-drafts/wave1m/122-builtin-samples-delivery.md` と、#{{122}} の作業の記録(`pm-work/findings/blocks-samples-1366.md`。main に入っている場合)
- `inputs/fl-installer-files.tsv`(`Data\Patches\Misc\Used by demo projects\DEDZII - NIGHT TERROR\` と `Data\Patches\Misc\Used by demo projects\Mehran Abbasi - With You\` の行)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`@lazy-file:` の行の形。demo の .flp は、既に `@lazy-file:resources/Data/Demo projects/Demo songs/...` の行で入っている)
- `lift/harness/web/pack.args`、`lift/crates/fe-build/tests/plan/bundle.txt`
- `tools/p3-audio-probe/measure.mjs`(開いて `nonSilentQuanta` を出す道具)

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
2. `ci/resources-prefixes.txt` に 2 行を足す(`\` で終わるフォルダ単位): `Data\Patches\Misc\Used by demo projects\DEDZII - NIGHT TERROR\` と `Data\Patches\Misc\Used by demo projects\Mehran Abbasi - With You\`。(`ci/` の変更は main の deploy を動かす。報告に書く)
3. 2 フォルダの 9 件を、`lift/harness/seeds/vfs-seed-idle1.tsv` に `@lazy-file:` の行で足す。大きさと SHA-256 は `inputs/fl-installer-files.tsv` の値。空の代替(空ファイル・0 埋め)は置かない。
4. `lift/harness/web/pack.args` と `lift/crates/fe-build/tests/plan/bundle.txt` に、同じ 9 組の `--lazy-file` を足す(#{{122}} と同じ書き方)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。
6. commit する。push はしない。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node -e 'const fs=require("fs");const n=fs.readFileSync("lift/harness/seeds/vfs-seed-idle1.tsv","utf8").split("\n").filter((l)=>l.includes("Used by demo projects"));process.exit(n.length===9&&n.every((l)=>l.split("\t")[1].startsWith("@lazy-file:"))?0:1)' && mkdir -p .tmp-129 && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-129/dedzii.flp && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/Mehran Abbasi - With You.flp" .tmp-129/mehran.flp && for f in dedzii mehran; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-129/$f.flp" > "$RUNNER_TEMP/open-$f.json"; done && node -e 'const fs=require("fs");const L=["dedzii","mehran"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle")&&L.some((r)=>r.nonSilentQuanta>0&&r.quantaWhilePlaying>0)?0:1)'
```

判定の基準: 起動 1 回で、engine-idle・standIn が false・error が null。demo 2 件を開いた結果は、2 件とも `error` が null と engine-idle で、そのうち 1 件以上で `nonSilentQuanta` と `quantaWhilePlaying` が 1 以上。新しい seed の行は 9 件すべて `@lazy-file:` で始まること(起動の前に取る量を増やさない)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 追加した 9 件の相対パス(曲のフォルダごと)・大きさ・SHA-256(一覧の値)、2 件の測定の JSON の行(`nonSilentQuanta` を含む)、欠けた物の一覧(測れた場合)、変えたパス(`ci/`・`lift/`・`app/` を含む場合は、その旨)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。遅延は既定の動作にする(オプトインを作らない)。
- 空の代替(空ファイル・0 埋め)を置かない。SHA-256 と大きさは、一覧(インストーラの実物から取った値)と照合したものだけを置く。
- `inputs/fl-installer-files.tsv` は読むだけ。
- 依頼者の個人のプロジェクトは使わない。
- ベンダー名・Cloudflare の識別子・インストール先の既定の値を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 32〜46 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。`sleep`・`timeout` をコマンドの先頭に書かない。

## 範囲外(この Issue では直さない)

- 他の曲の素材(`Used by demo projects` の残りの 24 曲。約 80 MB)。P3 の入力として必要になったら、新しい Issue にする。
- FLEX・Emphasizer の読み込み(#{{021}}・#{{022}} の範囲)。DEDZII の FLEX の音は、この Issue では確かめない。

## 他のタスクとの関係

- blocked_by: #{{121}}(demo を File > Open で開く。段階 1)と #{{122}}(サンプルの配信の仕組み。同じ seed 表・`pack.args`・`bundle.txt` を変える)。
- #{{004}}・#1335(P3 の測定。この Issue の結果を、入力の候補に使う)。
