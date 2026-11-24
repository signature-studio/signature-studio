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
- 一覧 `inputs/fl-installer-files.tsv` では、`Data\Patches\Packs\Legacy\Drums\Dance\` に `DNC_*.wav` が 21 件ある(その他に `Data\Patches\Misc\Used by demo projects\` の demo 用の素材もある)。前回の試行の 4 件は、この一覧に在る名前(確認済み。下の「前回の試行で分かったこと」)。
- 展開の木(`lift/out-app/host/`)には、この 4 件が無い(前回の報告の記述: `%FLStudioFactoryData%` の下)。
- 範囲: #{{105}}(言語・テーマ・テンプレート・デモの配信)は、`Data\Patches\`(17,782 件・約 657 MB。プリセット)を範囲外にした。そのため、Patches の中で、プロジェクトが使う物(サンプル)は、どの Issue にも入っていない。この Issue がその範囲を扱う。
- 内蔵プラグインの FLEX・Emphasizer は、既定のプロジェクトで読めない(「Problems loading」の窓の一部)。これは #{{021}}(内蔵 Effects の残り)と #{{022}}(内蔵インストゥルメント)の範囲で、この Issue の範囲外。
- 仕組み(既存の道具): `@lazy-file:` の印(決定 0314 の `@lazy:` の索引。`lift/harness/vfs_lazy.h`)と、`lift/harness/web/pack.args` の `--lazy-file`(起動の後の取得)。デモ(#{{105}})と同じ仕組みを使える(`lift/harness/vfs_seed.h`・`app/src/engine/lazy-file.ts`)。
- P1(起動 10 秒以内)の予算: 起動前に取る量を増やさない。サンプルは起動の後に取る。

## 前回の試行で分かったこと(run 45315603003。約 2 時間 23 分。`failed`)

- 作業は保存された(`work.diff` 62,686 B。artifact `agent-1366`)。判定が落ちた理由: 受け入れの `fe-build bundle` が、`lift/out-app/host/resources/Data/Patches/Packs/Legacy/Drums/Dance/DNC_Clap.wav` を開けなかった(ENOENT。exit 1)。
- 原因は構造的。21 件の実物は、インストーラからしか取れない。取り出すのは取り出しの段(`extract`)だけで、作業の段には取り出しの資格が無い。その取り出しの接頭辞(`ci/resources-prefixes.txt`)に `Data\Patches\` の下が無かったので、作業の木(`lift/out-app/host-extra/resources/`)に 21 件が無かった。
- 直した(main の 81e3b816): `ci/resources-prefixes.txt` に `Data\Patches\Packs\Legacy\Drums\Dance\` を 1 行足した。取り出しの接頭辞は `\` で終わるフォルダ単位で、ファイル単位の指定は無い。このフォルダは 33 件・642,458 B。
- そのうち DNC_*.wav が 21 件・193,734 B(この Issue の配信の範囲)。Blocks が要求するのは、そのうち 4 件・30,646 B。残りの 12 件(448,724 B: `Basic 808` ×4、`CB_` ×4、`* Basic` ×4)は、同じ取り出しで作業の木に入るが、bundle には載せない(`pack.args` に入るのは 21 件だけ)。
- 欠けていた物(Blocks が要求する 4 件。大きさと SHA-256 は、インストーラの実物から取った一覧の値):

| 相対パス(`Data\Patches\Packs\Legacy\Drums\Dance\` の下) | 大きさ (B) | SHA-256 |
|---|---|---|
| `DNC_Kick.wav` | 5,254 | `3b53ec2451d0d3ecddf4ab968467e87170910b094f6590a4ff9947069eece38b` |
| `DNC_Clap_5.wav` | 14,448 | `7b4bf3b93cb017c33d1358ba7902f71167ee35248aa8bc5571404dd375c29cf1` |
| `DNC_Hat.wav` | 4,856 | `b427aaa6ae50f1d5c3bd37a07d12a388fd9f4b3995397050174e71935b107383` |
| `DNC_Snare_4.wav` | 6,088 | `5bd8a48ca10addd1cd706d7f89ec028308318a200a646b888d9c28d96b3b1d03` |

- 作業の差分(`work.diff`)は、次の試行の取り込みで入る(`git apply --check` は main に対して通る)。中身:
  - `lift/harness/seeds/vfs-seed-idle1.tsv`: 21 行を `@lazy-file:` で足した(SHA-256 は一覧の値)。
  - `lift/harness/web/pack.args`: 21 組の `--lazy-file` を足した。
  - `lift/crates/fe-build/tests/plan/bundle.txt`: 同じ 21 組。
  - `tools/p3-audio-probe/measure.mjs`: 開いた後の窓を `projectDialogs` に、欠けた `Data\Patches\` の `.wav` の要求を `projectMissing` に記録する。Welcome の窓は Escape で閉じ、プロジェクトは Ctrl+O で開く(`tools/delivery-split/measure.mjs` と同じ経路)。
  - `pm-work/findings/blocks-samples-1366.md`: 測定の記録。
- 測定(変更の前。runner の値。この機械とは別枠。nproc 4):
  - Blocks を開く: `error` null。`projectDialogs` 1 件(「Problems loading the project」、691×287)。4 件の要求は、フォルダが無い(`GetFileAttributesW` の code 3)で失敗した。
  - 起動前の量(`delivery-split`、3 回): `preBootBytes` 310,588,073 B(3 回とも同じ)。`elapsedMs` 10,642.1・10,683.5・10,798.2 ms。
  - 起動確認(変更の前): engine-idle・standIn false・error null。`elapsedMs` 12,492(loadavg 1.59 1.31 1.86)。
  - 変更の後の測定は無い(bundle が作れなかったため)。
- 道具: `pm-work/findings/p3-probe-1335.patch` は、上の `measure.mjs` と別の版(同じファイルを変える)。上の差分に当てると 7 hunk のうち 4 が失敗する。**当てない。**
- ビルド: `lift/target/release/fe-build engine` は、この run で 2 回とも約 32〜33 分。

## 実物の置き場

作業ツリーの `lift/out-app/host-extra/resources/` に、接頭辞 `Data\Patches\Packs\Legacy\Drums\Dance\` の 33 件の実物が在る(この 21 件を含む)。ラベル `resources` の取り出しの段が、`ci/resources-prefixes.txt` の接頭辞で取り出し、作業の段が `lift/out-app/host-extra/resources/` に置く。この run から。接頭辞の変更(81e3b816)は、この Issue の前に main に入った。

`fe-build bundle` の `pack-seed.mjs` は `lift/out-app/host/resources/` を読むため、受け入れの前に、この木を `lift/out-app/host/resources/` へ写す。受け入れのコマンドの先頭で行う(#{{105}} と同じ書き方)。Blocks の `Blocks.flp`(57,737 B)も、`lift/out-app/host-extra/resources/Data/Templates/Other/Blocks/Blocks.flp` から `.tmp-122/blocks.flp` へ写す(測定の入力。受け入れのコマンドの中で行う)。

## 読む物

- `pm-work/findings/p3-blocks-1335.md`(前回の報告。窓の内容と、無音の JSON)
- `inputs/fl-installer-files.tsv`(`Data\Patches\Packs\Legacy\` の行)
- `lift/harness/vfs_lazy.h`(決定 0314)、`lift/harness/vfs_seed.h`、`lift/harness/web/pack.args`、`app/src/engine/lazy-file.ts`、`app/scripts/extract-fl.mjs`、`app/scripts/pack-seed.mjs`
- `tools/missing-paths/measure.mjs`(起動の時に要求されて無かった物。サンプルの要求の記録に使える)
- `pm-work/issue-drafts/wave1d/105-resources-full-set.md`(範囲の決め方。`Data\Patches\` は範囲外)
- `tools/p3-audio-probe/measure.mjs`(Blocks を開いて、`projectDialogs` を見る。前回の差分で入った版。`pm-work/findings/p3-probe-1335.patch` の版ではない)

## やること

前回の試行で、下の 1〜5 は済んでいる(作業の差分に入っている)。次の試行は、差分が載っているかを確かめるところから始める。

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
2. 前回の差分(`work.diff`)が載っているかを、`git status` と `git diff --stat` で確かめる。載っていなければ、「前回の試行で分かったこと」の変更を、同じ内容で作る(`pm-work/findings/blocks-samples-1366.md` に記録がある)。
3. 欠けている物の特定: 済(4 件)。
4. 配信の範囲: 済(接頭辞 `Data\Patches\Packs\Legacy\Drums\Dance\`。21 件を配信。残りの 12 件は bundle に載せない)。
5. 配信する: 済(seed 表・`pack.args`・`bundle.txt`)。起動前の量が増えないことは、下の「残りのやること」の 3 で測る。
6. 確かめる: Blocks を開いて、サンプルの窓が出ないこと(`projectDialogs` が空)。受け入れで行う。

## 残りのやること

1. 載った作業ツリーを確かめる: `.prior-work.md`、`git status`、`git diff --stat`。前回の差分が載っていて、`lift/out-app/host-extra/resources/` に 33 件が在る(`MANIFEST.tsv` が在る)。
2. 受け入れを通す(再ビルド): 受け入れのコマンドは、先頭で実物の写しを行い、エンジンと bundle を作り、起動を確かめ、Blocks を開く。
3. 通ったら、変更の後の起動前の量を測る(`tools/delivery-split/measure.mjs`、3 回。`preBootBytes`・`elapsedMs`)。上の前回の値と比べて報告に書く。
4. commit する(受け入れが通らなくても、作業の最後に commit する)。push はしない。
5. 報告に、欠けていた物と配信の範囲、変えたパス、測定の値、受け入れの出力を書く(「報告の規則」を参照)。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && mkdir -p .tmp-122 && cp lift/out-app/host-extra/resources/Data/Templates/Other/Blocks/Blocks.flp .tmp-122/blocks.flp && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && test -s .tmp-122/blocks.flp && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-122/blocks.flp" > "$RUNNER_TEMP/blocks.json" && node -e 'const fs=require("fs");const r=JSON.parse(fs.readFileSync(process.argv[1],"utf8").trim());process.exit(r.error===null&&Array.isArray(r.projectDialogs)&&r.projectDialogs.length===0?0:1)' "$RUNNER_TEMP/blocks.json"
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
- エンジンのビルドは 1 回約 32〜35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。

## 範囲外(この Issue では直さない)

- `cargo test -p fe-build` の `plans_match_recorded_texts` は、この Issue の前から、エンジンの計画(`engine.txt`)で失敗する(rt 91 と 90。f4cffe71 以来)。受け入れは通すが、直さない。
- delivery-split のシーン(NewStuff.flp を開く)は、前回の試行の 3 回とも `state exited after the scene` で終わった。原因は未確認。#{{102}} の受け入れでは通っていた経路なので、新しい回帰の可能性がある。作業者は、報告に、現象と、疑う commit を書く。残りのやること 3 が同じ現象で止まったら、その値は「確認できない」と書く。
- `Data\Patches\` の残り(#{{105}} の範囲外。接頭辞の外)。このうち DNC 2 件(`Data\Patches\Misc\Used by demo projects\`)と、`Data\Patches\Packs\Legacy\Instruments\Strings\` の 2 件は、この Issue の範囲外として残す。

## 他のタスクとの関係

- #{{004}}(P3 の demo の測定)と #{{109}}(実機の条件の測定)が、音の出る入力として、この Issue の結果を使う。
- #{{105}}(配信の全件)と #{{015}}・#{{101}}(起動後の取得の仕組み)と、同じ読みの経路(`lift/harness/vfs_seed.h`・`app/src/main.ts`)を使う。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- FLEX・Emphasizer の読み込みは、#{{021}}・#{{022}} の範囲(この Issue の外)。

