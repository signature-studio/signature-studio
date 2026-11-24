---
draft: "158"
title: "P3 の測定に使う、音が出る demo を決める: インストーラの素材だけで、nonSilentQuanta が 0 でない物"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["129", "128"]
labels: ["audio", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

P3 の測定(#{{004}})の入力に、音が出る demo が無い。前回の測定(51 件の候補のうち開けた 30 件)は、すべて `nonSilentQuanta` が 0 だった。DEDZII は開けたが、配信の後も無音(#{{129}} の結果)。

この Issue の目標: インストーラに在る素材だけで、`nonSilentQuanta` が 0 でない demo を 1 件決め、`inputs/p3-project.tsv` に書く(#{{004}} の入力)。

## 背景

- 無音の原因は 2 通りある。(a) インストーラに在るが、配信されていない素材(#{{129}} で届ける)。(b) インストーラに無い素材(FL がオンラインで取る素材。配信できない)。(b) の件数が 0 の曲だけが、インストーラの素材だけで鳴る。
- 候補: インストーラの `Data\Demo projects\` の下の `.flp`(`inputs/fl-installer-files.tsv` の一覧では 46 件。Demo songs・その他の曲を含む)。
- 欠けた素材の数え方: エンジンのログの `missing-path` の行を、インストーラの一覧と照合する(#{{129}} の道具の `missingInInventory`・`notInInstaller`)。
- 未確認: 候補の多くが (b) を含むかもしれない。その時は、この Issue は「選べない」を報告に書く(下の「判定の基準」)。

## 前回までの試行で分かったこと

- 新規の Issue。前回の結果は #{{129}} の本文に在る(DEDZII は開くが無音。Mehran は #{{128}} の後に開く)。

## 読む物

- #{{129}} の本文(「前回までの試行で分かったこと」・「やること」の 3 と 4)
- `inputs/fl-installer-files.tsv`(`Data\Demo projects\` の行)
- `pm-work/findings/p3-blocks-1335.md`(前回の測定の表。51 件の候補)
- `tools/p3-audio-probe/measure.mjs`(開いて `nonSilentQuanta` を出す道具。#{{129}} で欠落の内訳が出る)

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
2. 候補の一覧を作る: `inputs/fl-installer-files.tsv` の `Data\Demo projects\` の下の `.flp` の全て(46 件)。
3. 道具 `tools/p3-audio-probe/scan-demos.mjs` を新しく作る(引数は bundle の場所だけ)。候補の全曲を 1 曲ずつ、1 回の起動ごとに開き(`measure.mjs` と同じ方式)、曲ごとに次を JSON に出す: `name`・`path`・`error`(null か文字列)・`notInInstaller`(件数)・`notInInstallerList`(パスの一覧)・`nonSilentQuanta`(測れた曲だけ)・`underrunDelta`(#{{004}} の定義と同じ)。stdout は JSON の 1 行で、`{"songs":[...]}`。
4. 選ぶ: `error` が null で、`notInInstaller` が 0 で、`nonSilentQuanta` が 0 より大きい曲のうち、`nonSilentQuanta` が最も大きい 1 件。その 1 件を `inputs/p3-project.tsv` に書く(1 行。列は曲名・パス。ヘッダの無いタブ区切り)。
5. 無い時: `inputs/p3-project.tsv` は作らない。報告に、各曲の `notInInstaller` の件数の表を書き、`error` が null の曲のうち件数が最も少ない曲の `notInInstallerList`(インストーラに無い素材の一覧)を出す。
6. 作業の最後に commit する(push は、ジョブの最後の判定が通ったときだけ行われる)。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/p3-audio-probe/scan-demos.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/scan.json" && node -e 'const fs=require("fs");const s=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));const cands=fs.readFileSync("inputs/fl-installer-files.tsv","utf8").split("\n").filter((l)=>l.startsWith("Data\\Demo projects\\")&&l.split("\t")[0].toLowerCase().endsWith(".flp")).length;const ok=Array.isArray(s.songs)&&s.songs.length===cands&&s.songs.every((x)=>typeof x.notInInstaller==="number"&&Array.isArray(x.notInInstallerList)&&(typeof x.nonSilentQuanta==="number"||typeof x.error==="string"));if(!ok)process.exit(1);const pick=s.songs.filter((x)=>x.error===null&&x.notInInstaller===0&&x.nonSilentQuanta>0).sort((a,b)=>b.nonSilentQuanta-a.nonSilentQuanta)[0];if(pick){process.exit(fs.readFileSync("inputs/p3-project.tsv","utf8").includes(pick.name)?0:1)}const min=[...s.songs].filter((x)=>x.error===null).sort((a,b)=>a.notInInstaller-b.notInInstaller)[0];process.exit(min&&min.notInInstallerList.length===min.notInInstaller?0:1)' "$RUNNER_TEMP/scan.json"
```

判定の基準:
- 道具の JSON(`songs`)に、候補の全曲(46 件)が在る。各曲に `notInInstaller`(数)・`notInInstallerList`(配列)が在る。`nonSilentQuanta` は数(測れた曲)か、`error` が文字列の曲では null でよい。
- 次のどちらか。(a) `error` が null で、`notInInstaller` が 0 で、`nonSilentQuanta` が 0 より大きい曲が 1 件以上。`inputs/p3-project.tsv` に、その 1 件(`nonSilentQuanta` が最大の曲)が在る。(b) 無い時は、表と最小の曲の一覧を報告に出す。`inputs/p3-project.tsv` は無くてよい。(b) でも受け入れは通る(表が出て、一覧が合う)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 候補の数、曲ごとの表(曲名・`notInInstaller`・`nonSilentQuanta`・`underrunDelta`)、選んだ 1 件か、最小の曲の欠けた素材の一覧。

## 制約

- `inputs/fl-installer-files.tsv` は読むだけ。`inputs/p3-project.tsv` は、この Issue だけが作る。
- インストーラに無い素材を、どこからも取らない(配信の対象外)。
- 依頼者の個人のプロジェクトは使わない。
- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python・OCR を使わない。ベンダー名・Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- エンジンのビルドは 1 回約 35 分。曲の走査は 1 曲ごとに起動し直すので、時間が掛かる。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 範囲外

- インストーラに無い素材の配信(FL がオンラインで取る素材)。
- P3 の測定そのもの(#{{004}})。この Issue は、その入力を決めるだけ。
- Mehran Abbasi - With You(#{{128}} の後に開く)。

## 他のタスクとの関係

- blocked_by: #{{129}}(欠けた素材の配信。この Issue の候補の無音の原因を、先に分ける)と #{{128}}(Balance の入口。Mehran を候補に入れるため)。
- #{{004}}(#1335・P3 の測定)は、この Issue の `inputs/p3-project.tsv` を入力に使う。#1335 の blocked_by に、この Issue を足す。
