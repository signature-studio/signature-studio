---
draft: "004"
title: "インストーラの demo プロジェクトで P3(Song Mode の underrun)を測る"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["003", "040"]
labels: ["audio", "verify"]
size: S
priority: Urgent
project: signature-studio/3
---

## 目的

P3 の完成の条件: プロジェクトを Song Mode で再生して、音が途切れない。Song Mode の再生の窓(Space の後の 10 秒)で underrun が 0 であることを、runner で n=3 で実測する。音が出ていること(`nonSilentQuanta` が 0 より大きいこと)も条件に入れる。音が出ない再生では、途切れの数は P3 の判定にならない。この実測は、#{{040}}(音声への圧迫の解消)の後に行う。

## 背景

- 入力のプロジェクト: インストーラに含まれる demo・テンプレートのプロジェクト(`.flp`)のうち、Song Mode の playlist を持ち、**音が出る**物を、作業者が選ぶ。依頼者の個人のファイルは使わない。
- 選ぶ元は、#{{013}} の成果物 `inputs/fl-installer-files.tsv`(インストーラ全件の一覧)。`Data\` の下の `.flp` を候補にする。playlist の件数で、Song Mode の再生があるかを確かめる。
- 選んだ 1 件を `inputs/p3-project.tsv` に書いて commit する(列: `guest`・`bytes`・`sha256`・`playlist`。見出し行を 1 行残し、その下に 1 行。guest は、インストール先からの相対のパス)。インストール先の既定の値は書かない。
- 実行に使うファイルは、インストーラの展開の木(`lift/out-app/host/resources/`)から取る。受け入れの中で `.tmp-p3-project.flp` に写す(リポジトリには入れない)。
- 定義(旧 ci タスク 0005 の記述による。出典の文書 docs/ops/fidelity-and-performance.md は、リポジトリに無い): 所定のプロジェクトを Song Mode で再生して、AudioWorklet のリングの枯渇(underrun)が 0。
- 台本: `tools/verify-kit/tools/scn-p3-mixmove-300.json` を参考にしてよい。ただし、Song Mode の再生は #{{003}} の道具の中の手順で行う。
- 前提: #{{003}}(音声の道具)と #{{040}}(圧迫の解消)が終わっている(両方とも閉じている)。
- 音が出ない原因: 下の「前回の試行で分かったこと」を読む。

## 前回の試行で分かったこと

run 45315923203(判定は差し戻し。作業は保存された: `work.diff`。`inputs/p3-project.tsv` は見出し行のみ。`pm-work/findings/p3-sound-1335.md`(候補の表)と `tools/p3-audio-probe/measure.mjs`(道具の直し)を含む)。

1. 結果(実測。runner の値。nproc 4): 選んだ 1 件(Sacco - Exploding Chips。playlist 160 項目)で、n=3 の測定は 3 回とも `error` が null、`quantaWhilePlaying` が 11,063〜11,074、窓の `underrunDelta` が 0。ただし `nonSilentQuanta` は 3 回とも 0。受け入れは、この `nonSilentQuanta` の判定で落ちた。
2. 候補の全件: playlist を持つ 51 件のうち、開けた 30 件は、すべて `nonSilentQuanta` が 0。開けた 1 件(Sacco - Expanding universe)では、振幅(peak・rms)も 0。計数の道具は正しく動いている。開けない 21 件は、前面が Song info のまま閉じない。表は `pm-work/findings/p3-sound-1335.md`。
3. 無音の原因(推定。未確認): インストーラの demo は、`Data\Patches\Misc\Used by demo projects\<曲名>\` の素材を使う(曲ごとのフォルダ。623 件・約 82 MB。曲は 26 件)。このフォルダは、配信の接頭辞(`ci/resources-prefixes.txt`)にも、起動の seed にも無い(#{{122}} の範囲外として残した分)。テンプレート Blocks の「Problems loading」の窓と同じ症状。これを、新しい Issue #{{129}} で DEDZII と Mehran Abbasi - With You の 2 件分だけ配信する。
4. Song info の窓: 開けない 21 件、DEDZII、Exoplanet は、Song info の窓が閉じない(Escape では閉じない)。DEDZII は #{{121}}(段階 1)、Exoplanet は #{{128}}(段階 2)で扱う。
5. 道具(`tools/p3-audio-probe/measure.mjs`。差分に入っている): 題名の待ちは「題名が変わるまで」(OK の前の題名と比べ、Song info は除く)。ファイル名以外の題名になる物(例: Electric Loop)も、`error` が null で読めた。
6. 受け入れの欠陥(構造的。直した): 前回の受け入れは `test -s .tmp-p3-project.flp` で始まり、作業者が一時ファイルを消すと、判定の段で落ちた。今の受け入れは、`inputs/p3-project.tsv` の 2 行目から、展開の木の写しを作る形に直した(大きさと SHA-256 が TSV の値と一致しなければ落ちる)。
7. 起動(実測): `boot.json` は engine-idle・standIn false・error null・elapsedMs 8,166(runner)。

## 読む物

- `pm-work/findings/p3-sound-1335.md`(候補の表。前回の差分に入っている)
- `tools/p3-audio-probe/measure.mjs`(main の版は古い。前回の差分の版を使う)
- `inputs/fl-installer-files.tsv`(`Data\Demo projects\Demo songs\` と `Data\Patches\Misc\Used by demo projects\` の行)
- `app/src/audio/page-audio.ts`(42〜43 行: 音声の状態の項目)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば読む。`git status` と `git diff --stat` で、前回の差分(道具と TSV)が載っているかを確かめる。
1. 前提を確かめる: `git log --oneline -30` で、#{{121}}(段階 1)と #{{129}}(demo の素材)が main に入っているかを確かめる。入っていなければ、受け入れを実行せず、「前提未達」と報告に書いて終える。
2. 候補を出す: `inputs/fl-installer-files.tsv` の `Data\Demo projects\Demo songs\` の `.flp` のうち、playlist を持つ物を、件数の多い順に並べる。件数は、FLP のイベント 233 の大きさを 32 か 60 バイトで割って数える(前回の方法。`pm-work/findings/p3-sound-1335.md` に書いてある)。
3. 音が出る 1 件を選ぶ。同じビルドの engine で、候補を件数の多い順に `tools/p3-audio-probe/measure.mjs` で 1 件ずつ測り(1 件 1 分ほど。ビルドは 1 回だけ)、`nonSilentQuanta > 0` かつ `quantaWhilePlaying > 0` の物を選ぶ。開けない物は飛ばし、その事実を報告に書く。
4. 選んだ 1 件を `inputs/p3-project.tsv` に書いて commit する。見出し行 `guest` `bytes` `sha256` `playlist`(タブ区切り)の下に、1 行。guest は `Data\Demo projects\Demo songs\<名前>.flp` の形。
5. 受け入れを通す(n=3 を含む)。各回の道具の JSON の 1 行を、そのまま報告に貼る。
6. 各回で、窓の `underrunDelta`・`nonSilentQuanta`・`underrunSpans`(途切れの区間)を報告に書く。0 でない回は、その区間を書く。
7. 耳の確認は行わない。「音が途切れない」とは書かず、underrun の数と `nonSilentQuanta` で「受理」「差し戻し」「未確認」を判定する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && test -s inputs/p3-project.tsv && node -e 'const fs=require("fs"),c=require("crypto");const [g,b,h]=fs.readFileSync("inputs/p3-project.tsv","utf8").split("\n")[1].split("\t");const s="lift/out-app/host/resources/"+g.replace(/\\/g,"/");const d=fs.readFileSync(s);if(String(d.length)!==b||c.createHash("sha256").update(d).digest("hex")!==h)process.exit(1);fs.copyFileSync(s,".tmp-p3-project.flp")' && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-p3-project.flp" > "$RUNNER_TEMP/p3-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/p3-"+i+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.quantaWhilePlaying>0&&r.nonSilentQuanta>0&&r.underrunDelta===0)?0:1)'
```

判定の基準: 3 回すべてで、`error` が null、再生の quantum が 1 以上、`nonSilentQuanta` が 1 以上、窓の `underrunDelta` が 0。受け入れの 2 段目(TSV の行から、展開の木の写しを `.tmp-p3-project.flp` に作る)は、大きさと SHA-256 が TSV の値と一致しなければ落ちる。判定の段でも同じ写しが走るので、作業者は `.tmp-p3-project.flp` を消さない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」「音が途切れない」とは書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 展開の木に選んだ 1 件が無い場合は、「実エンジン: 実行なし(選んだ物が展開の木に無い)」と書いて止まる。
- 音が出る入力が見つからない場合は、「実エンジン: 実行あり。音の出る入力なし」と書き、候補ごとに開けたか・音が出たかを表にする。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 依頼者の個人のプロジェクトは無い。インストーラの demo を使う。展開したプロジェクトは、リポジトリに入れない(`.tmp-*` に置き、消す)。
- ベンダー名と Cloudflare の識別子を書かない。インストール先の既定の値を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 35〜46 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。`sleep`・`timeout` をコマンドの先頭に書かない。候補の測定は同じビルドで行い、ビルドを繰り返さない。

## 他のタスクとの関係

- blocked_by: #{{003}}(道具)と #{{040}}(圧迫の解消)。どちらも閉じている。
- 前提(まだ閉じていない。この Issue の再試行は、これらが閉じてから行う): #{{121}}(段階 1: DEDZII を開く)と #{{129}}(demo の素材の配信)。#{{128}}(段階 2: Exoplanet)が閉じると、候補が広がる。
- #{{031}}(P3 の再生中の命令の内訳)は、この Issue が commit する `inputs/p3-project.tsv` を使う。
- #{{008}}(HiDPI の後の実測)と #{{038}}(完成の検証)が、この Issue の結果を使う。
- #{{109}}(実機の条件の測定)は、この Issue が選ぶ `inputs/p3-project.tsv` を、段 2・3 の入力にする。
