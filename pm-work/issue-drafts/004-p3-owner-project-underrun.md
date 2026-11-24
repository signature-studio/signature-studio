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
- 選んだ 1 件を `inputs/p3-project.tsv` に書いて commit する(列: `guest`・`bytes`・`sha256`・`playlist`。見出し行を 1 行残す)。インストール先の既定の値は書かない。
- 実行に使うファイルは、インストーラの展開の木から取り、`.tmp-p3-project.flp` に置く。受け入れの後に消す。リポジトリには入れない。展開の木に無い場合は、ここで止めて報告する。
- 定義(旧 ci タスク 0005 の記述による。出典の文書 docs/ops/fidelity-and-performance.md は、リポジトリに無い): 所定のプロジェクトを Song Mode で再生し、AudioWorklet のリングの枯渇(underrun)が 0。
- 台本: `tools/verify-kit/tools/scn-p3-mixmove-300.json` を参考にしてよい。ただし、Song Mode の再生は#{{003}} の道具の中の手順で行う。
- 前提: #{{003}}(音声の道具)と#{{040}}(音声への圧迫の解消)が終わっている(両方とも閉じている)。
- 音が出ない原因(前回の試行で分かった。#{{122}}・#{{121}}): テンプレート Blocks はサンプル 4 件が読めず、demo の 2 件は File > Open で engine が止まる。音の出る入力が見つからない場合は、その事実を報告に書き、判定は「差し戻し」にする。

## 前回の試行で分かったこと

run 45253143948(2026-11-23 19:52Z 起動)の作業。runner の値(nproc 4)。判定は差し戻し。報告の原文は `pm-work/findings/p3-blocks-1335.md`、差分(道具の直し)は `pm-work/findings/p3-probe-1335.patch`。

1. 結果: 3 回とも `underrunQuanta`(AudioContext の開始からの累計)が 1,015・1,081・1,041 で、受け入れの 0 を満たさなかった。Space の後の 10 秒の窓の `underrunDelta` は 3 回とも 0。途切れは窓より前の 2 区間(AudioContext の開始からの ms): 1 回目 29〜4,469 と 12,359〜13,243、2 回目 36〜4,476 と 12,106〜12,949、3 回目 76〜4,357 と 12,096〜12,976。
2. 音: 3 回とも `nonSilentQuanta` が 0(音が出ていない)。`quantaWhilePlaying` は 11,266・11,373・11,373 で、再生は進んでいる。Blocks の 4 件のサンプル(`DNC_*.wav`)が読めず、「Problems loading the project」の窓(691×287)が出る。内蔵プラグインの FLEX・Emphasizer は、既定のプロジェクトで読めない(#{{021}}・#{{022}} の範囲)。
3. 候補の選び直し: playlist の項目数は、FLP のイベント 233(`DATA + 25`。1 件 32 または 60 バイト)を数えた。展開の木の 80 件のうち、項目を持つのは 48 件。項目数が多い demo の `Eric Kauffmann & NrgMind - Exoplanet.flp`(4,440 項目)と `DEDZII - NIGHT TERROR.flp`(3,850 項目)は、File > Open で Song info の窓が出た後に engine が止まる(exit 134。ログ `LockResource: no PE image is mapped at 0x400000 (no MZ signature)`)。→ #{{121}}。テンプレート `Data\Templates\Other\Blocks\Blocks.flp`(260 項目)は開けた。
4. 測定の道具の欠陥(直す。`tools/p3-audio-probe/measure.mjs`):
   - (a) Welcome 窓は X が画面の外(窓 874×573 が、780×480 の画面に収まらない)。Escape で閉じる。
   - (b) 既定のプロジェクトが読まれると、「Problems loading the project」(`TMissingPluginsForm`)が前面に出て、File メニューに届かない。その Ok(窓の左上から +368、窓の下端から −29)を押す。窓の高さは内容で変わる(359 と 287 を見た)。幅 691 の窓だけ押す。
   - (c) File > Open は、今のアプリでは OPFS のファイル管理の窓(`host.select`。決定 0216)。`#filedlg-input` の経路は無い。前回の道具は、この手順で開けた(下の「やること」2)。
   - (d) 例外の説明文を省かない。JSON に `welcome`・`projectDialogs`・`underrunSpans`(AudioContext の開始からの途切れの区間)を足した。
   - 前回の commit(`d5696f7a`・`fd55ddb5`)は push されず、このクローンにも無い。差分は `pm-work/findings/p3-probe-1335.patch` に残した。`spec/guard.test.sh` と `spec/workflow.test.sh` は、この差分で通った。
5. 起動: `boot.json` は engine-idle・standIn false・error null。`elapsedMs` は 11,216〜11,522(runner)。P1(10 秒)は超えている(#{{112}} の領域)。
6. 途切れの原因(推定): 前者の区間は、AudioContext の立ち上がり直後と思われる(起動直後にエンジンが出力に追いつけていない。#{{112}} と同じ種類)。後者は Space の前後と思われる(Space の時刻は記録していない)。AudioContext をプロジェクトの読み込みの後に開く版も試したが、累計は 977 で、ほぼ同じだった。
7. 受け入れの式を直した(理由): 累計は AudioContext の開始からの数で、Space の前の起動の途切れ(#{{112}} の領域)を含む。P3 の判定は再生の窓で行うべきなので、判定を窓の `underrunDelta` に変えた。音の条件 `nonSilentQuanta > 0` を加えた(音が出ない再生の途切れは P3 の判定にならない。#{{109}} の段 2・3 と同じ条件)。

## 読む物

- `tools/p3-audio-probe/measure.mjs`(#{{003}} が作る道具。前回の差分を当てた版を使う。`pm-work/findings/p3-probe-1335.patch`)
- `pm-work/findings/p3-blocks-1335.md`(前回の報告の原文)
- `inputs/fl-installer-files.tsv`(#{{013}} の成果物)
- `app/src/audio/page-audio.ts`(42〜43 行: 音声の状態の項目)

## やること

0. 作業の最初に、道具の直しを当てる: `git apply pm-work/findings/p3-probe-1335.patch`。main に既に入っている場合は、当てない。当たらない場合は、上の「前回の試行」の 4 の (a)〜(d) と同じ内容を手で直す。
1. 候補を出す: `inputs/fl-installer-files.tsv` の `Data\` の `.flp` のうち、playlist を持つ物を、件数の多い順に並べる(件数の数え方は前回の 3 のとおり)。
2. File > Open の手順(道具の `openProject`): (1) Welcome を Escape で閉じる。(2) 既定のプロジェクトの「Problems loading」の窓が出たら、その Ok を押す。(3) ファイル管理(`#files-btn`)を開き、`#files-overlay #fx-input-files` に `DOM.setFileInputFiles` で `.flp` を渡す。(4) `#files-overlay #fx-list` に行が出るまで待ち、Files を閉じる。(5) File > Open を開き、`#filedlg #fx-list` の行を選び、`#filedlg #fx-ok` を押す。(6) 題名が変わるまで待つ。
3. 音が出る 1 件を選ぶ。同じビルドの engine で、候補を件数の多い順に `tools/p3-audio-probe/measure.mjs` で 1 件ずつ測り(1 件 1 分ほど。ビルドは 1 回だけ)、`nonSilentQuanta > 0` かつ `quantaWhilePlaying > 0` の物を選ぶ。開けない物(engine が止まる・窓が閉じない)は飛ばし、その事実を報告に書く。前回の Blocks は、音が出ないので外してよい。
4. 選んだ 1 件を展開の木から `.tmp-p3-project.flp` に置き、guest パス・大きさ・SHA-256・playlist の件数を `inputs/p3-project.tsv` に書いて commit する。
5. 同じ道具で、n=3 を実行する。各回の道具の JSON の 1 行を、そのまま報告に貼る。
6. 各回で、窓の `underrunDelta`・`nonSilentQuanta`・`underrunSpans`(途切れの区間)を報告に書く。0 でない回は、その区間を書く。
7. 耳の確認は行わない。「音が途切れない」とは書かず、underrun の数と `nonSilentQuanta` で「受理」「差し戻し」「未確認」を判定する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && test -s .tmp-p3-project.flp && test -s inputs/p3-project.tsv && for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-p3-project.flp" > "$RUNNER_TEMP/p3-$i.json" || exit 1; done && node -e 'const fs=require("fs");const L=[1,2,3].map((i)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/p3-"+i+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.quantaWhilePlaying>0&&r.nonSilentQuanta>0&&r.underrunDelta===0)?0:1)'
```

判定の基準: 3 回すべてで、`error` が null、再生の quantum が 1 以上、`nonSilentQuanta` が 1 以上、窓の `underrunDelta` が 0。

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
- エンジンのビルドは 1 回約 35〜46 分。候補の測定は同じビルドで行い、ビルドを繰り返さない。

## 他のタスクとの関係

- blocked_by: #{{003}}(道具)と #{{040}}(圧迫の解消)。
- #{{121}}(demo の 2 件が開けない)と #{{122}}(サンプルが読めない)が閉じると、音の出る候補が広がる。
- #{{031}}(P3 の再生中の命令の内訳)は、この Issue が commit する `inputs/p3-project.tsv` を使う。
- #{{008}}(HiDPI の後の実測)と #{{038}}(完成の検証)が、この Issue の結果を使う。
- #{{109}}(実機の条件の測定)は、この Issue が選ぶ `inputs/p3-project.tsv` を、段 2・3 の入力にする。
