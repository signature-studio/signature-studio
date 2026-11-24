---
draft: "122"
title: "テンプレート Blocks が使う内蔵のサンプル(Data\\Patches\\Packs\\Legacy の DNC_*.wav など)を、起動の後に取れるように配信する"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "audio"]
size: M
priority: High
project: signature-studio/3
---

## 目的

テンプレート `Data\Templates\Other\Blocks\Blocks.flp` を開くと、サンプル 4 件が読めず、「Problems loading the project」の窓が出る。そのチャンネルは無音になる。P3 の測定(#{{004}}・#{{109}})の入力は、音が出る物でなければ判定にならないので、音の素材になるサンプルを揃える。

この Issue の目標: Blocks が使うサンプルが、起動の後の取得で読めて、読み込みの窓(サンプルの不足)が出ないこと。起動前に取る量は増やさない。

## 背景

- 前回の試行(run 45253143948。#{{004}} の作業者が道具で開いて確かめた。実測): Blocks.flp(57,737 B。playlist 260 項目)を読むと、「Problems loading the project」の窓(691×287。Ok のボタンで閉じる)が出る。窓の内容は、見つからないサンプル 4 件(`DNC_*.wav`)。読み込みは進み、engine は止まらないが、その 4 件のチャンネルは無音になる。3 回の再生の `nonSilentQuanta` は 0 だった。
- 一覧 `inputs/fl-installer-files.tsv` では、`Data\Patches\Packs\Legacy\Drums\Dance\` に `DNC_*.wav` が 21 件ある(その他に `Data\Patches\Misc\Used by demo projects\` の demo 用の素材もある)。前回の試行の 4 件は、この一覧に在る名前(推定。作業者が名前で照合する)。
- 展開の木(`lift/out-app/host/`)には、この 4 件が無い(前回の報告の記述: `%FLStudioFactoryData%` の下)。
- 範囲: #{{105}}(言語・テーマ・テンプレート・デモの配信)は、`Data\Patches\`(17,782 件・約 657 MB。プリセット)を範囲外にした。そのため、Patches の中で、プロジェクトが使う物(サンプル)は、どの Issue にも入っていない。この Issue がその範囲を扱う。
- 内蔵プラグインの FLEX・Emphasizer は、既定のプロジェクトで読めない(「Problems loading」の窓の一部)。これは #{{021}}(内蔵 Effects の残り)と #{{022}}(内蔵インストゥルメント)の範囲で、この Issue の範囲外。
- 仕組み(既存の道具): `@lazy-file:` の印(決定 0314 の `@lazy:` の索引。`lift/harness/vfs_lazy.h`)と、`lift/harness/web/pack.args` の `--lazy-file`(起動の後の取得)。デモ(#{{105}})と同じ仕組みを使える(`lift/harness/vfs_seed.h`・`app/src/engine/lazy-file.ts`)。
- P1(起動 10 秒以内)の予算: 起動前に取る量を増やさない。サンプルは起動の後に取る。

## 読む物

- `pm-work/findings/p3-blocks-1335.md`(前回の報告。窓の内容と、無音の JSON)
- `inputs/fl-installer-files.tsv`(`Data\Patches\Packs\Legacy\` の行)
- `lift/harness/vfs_lazy.h`(決定 0314)、`lift/harness/vfs_seed.h`、`lift/harness/web/pack.args`、`app/src/engine/lazy-file.ts`、`app/scripts/extract-fl.mjs`、`app/scripts/pack-seed.mjs`
- `tools/missing-paths/measure.mjs`(起動の時に要求されて無かった物。サンプルの要求の記録に使える)
- `pm-work/issue-drafts/wave1d/105-resources-full-set.md`(範囲の決め方。`Data\Patches\` は範囲外)
- `tools/p3-audio-probe/measure.mjs`(Blocks を開いて、`projectDialogs` を見る。道具の版は `pm-work/findings/p3-probe-1335.patch`)

## やること

0. 作業の最初に `date -u` を記録する。道具の直し(File > Open の手順と、`projectDialogs` の記録)が main に無ければ、`git apply pm-work/findings/p3-probe-1335.patch` で当てる。
1. 欠けている物を特定する: Blocks を開いたときに要求される `DNC_*.wav` の guest パスを、道具か `missing-paths` の記録から出す(前回は 4 件)。
2. 一覧で照合する: その guest パス(大文字と小文字を区別しない)と、`inputs/fl-installer-files.tsv` の大きさと SHA-256 を取る。一覧に無い物は、報告に書いて止める(推測で SHA-256 を書かない)。
3. 配信の範囲を決める: 最低限は、Blocks が要求する 4 件。同じフォルダ(`Data\Patches\Packs\Legacy\Drums\Dance\`、21 件)の全件を入れるかは、大きさを見て決め、理由を報告に書く。
4. 配信する: 起動の後に取る(`@lazy-file:` の印、または `--lazy-file`)。`lift/harness/seed` の表に、起動前の物として入れない。起動前に取る量(`preBootBytes` と `elapsedMs`)が増えないことを、変更の前後で測って報告に書く。
5. 確かめる: Blocks を開いて、サンプルの窓が出ないこと(`projectDialogs` が空)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && test -s .tmp-122/blocks.flp && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-122/blocks.flp" > "$RUNNER_TEMP/blocks.json" && node -e 'const fs=require("fs");const r=JSON.parse(fs.readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&Array.isArray(r.projectDialogs)&&r.projectDialogs.length===0?0:1)' "$RUNNER_TEMP/blocks.json"
```

判定の基準: 起動 1 回で、engine-idle・standIn が false・error が null。Blocks を開いた 1 回で、`error` が null、`projectDialogs` が空(サンプルの窓が出ない)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 欠けていた物の guest パスと大きさ・SHA-256(一覧の値)、配信の範囲(件数と大きさ。フォルダごと)、起動前に取る量の変更前後(`preBootBytes`・`elapsedMs`。`tools/delivery-split/measure.mjs` の値)、Blocks の `projectDialogs` の結果、変えたパス(`app/`・`lift/`・`tools/` を含む。`app/` と `lift/` を変えると main の deploy が動く)。
- 配信の範囲の外として残したもの(`Data\Patches\` の残りなど)は、件数と理由を書く。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。遅延は既定の動作にする(オプトインを作らない)。
- 空の代替(空ファイル・0 埋め)を置かない。SHA-256 と大きさは、一覧(インストーラの実物から取った値)と照合したものだけを置く。
- `inputs/fl-installer-files.tsv` は読むだけ。
- ベンダー名と Cloudflare の識別子を書かない。インストール先の接頭辞を、本文と報告に書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者が言っていない制限を、この作業の判断で増やさない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 35〜46 分。

## 他のタスクとの関係

- #{{004}}(P3 の demo の測定)と #{{109}}(実機の条件の測定)が、音の出る入力として、この Issue の結果を使う。
- #{{105}}(配信の全件)と #{{015}}・#{{101}}(起動後の取得の仕組み)と、同じ読みの経路(`lift/harness/vfs_seed.h`・`app/src/main.ts`)を使う。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- FLEX・Emphasizer の読み込みは、#{{021}}・#{{022}} の範囲(この Issue の外)。
