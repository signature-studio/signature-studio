---
draft: "121"
title: "demo の曲 Exoplanet・DEDZII を File > Open で開くと、Song info の窓の後に engine が止まる(LockResource: no PE image is mapped)を直す"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["plugin-builtin"]
size: M
priority: High
project: signature-studio/3
---

## 目的

インストーラの demo の曲 `Eric Kauffmann & NrgMind - Exoplanet.flp`(playlist 4,440 項目)と `DEDZII - NIGHT TERROR.flp`(playlist 3,850 項目)を File > Open で開くと、Song info の窓(`TInfoForm`)が出た直後に engine が止まる(exit 134。SIGABRT 相当)。

この Issue の目標: この 2 件を File > Open で開いて、engine が止まらず idle の状態を保つこと(Song info の窓は閉じられる)。Blocks(テンプレート)と Default では起きない。

P3 の測定(#{{004}})は、音の出る demo を入力に使う。この 2 件は、playlist が長く、音の出る候補として一番に選ぶ物なので、開けないことが P3 の測定の入力を狭めている(P3 に関わるので High)。

## 背景

- 発生(前回の試行。run 45253143948。#{{004}} の作業者が、道具で開いて確かめた。実測): 2 件とも、Song info の窓が出た直後に、ログの末尾が `fle: LockResource: no PE image is mapped at 0x400000 (no MZ signature)` になり、停止する(exit 134)。停止の証拠として、画面に `first non-background pixel on a drawing surface (decision 0126): written by UpdateLayeredWindowIndirect` が出るが、これは記録であって停止の原因ではない(前回の報告の記述)。
- 前回の報告では、この行は「プラグイン(Fruity Filter の DLL)の読み込みの途中」に出ていた(Exoplanet のログ)。DEDZII も同じ行で止まった。
- Blocks.flp(テンプレート。260 項目)は、同じ道具で、読み込みが最後まで進んだ(サンプル不足の窓は出るが、engine は止まらなかった。#{{004}} の 3 回の測定)。
- 0x400000 は、実行ファイルの既定のイメージの基準アドレス(推定)。LockResource に渡されたモジュールのイメージが、engine の中で PE として見えていない(推定。原因は未確認)。
- 関係: #{{019}}(Fruity Filter の起動後の取得)と #{{102}}(プラグインのコードを分ける)が、同じ経路を変えうる。作業の最初に、`git log --oneline -- lift/runtime lift/harness app/src/engine` で、近い変更を確かめる。
- 道具: `tools/p3-audio-probe/measure.mjs` の File > Open の手順は、`pm-work/findings/p3-probe-1335.patch` の版が要る(main に無ければ `git apply` で当てる)。前回の作業の報告は `pm-work/findings/p3-blocks-1335.md`。

## 読む物

- `pm-work/findings/p3-blocks-1335.md`(前回の報告。停止の時刻・ログの行・画面)
- `lift/runtime/`・`lift/harness/` の `LockResource`(呼び出しの定義と、`no PE image is mapped` の出所)。`grep -rn "no PE image" lift app/src`
- `lift/harness/` の PE のイメージの割り当て(`0x400000` を使う箇所)
- `app/src/engine/`(ログの出力の窓口: `engineLog()`)
- `tools/p3-audio-probe/measure.mjs`(再現に使う。File > Open の手順は上の道具の版)

## やること

0. 作業の最初に `date -u` を記録する。道具の直し(File > Open の手順)が main に無ければ、`git apply pm-work/findings/p3-probe-1335.patch` で当てる。
1. 再現する: 2 件の `.flp` を展開の木から `.tmp-121/exoplanet.flp` と `.tmp-121/dedzii.flp` に置く(展開の木の場所は `inputs/fl-installer-files.tsv` の guest パスから引く。リポジトリには入れない)。同じ道具で開き、停止の行(`LockResource` の行)と、直前の呼び出しを出す。
2. 特定する: `LockResource` に渡されたモジュールのハンドル(0x400000)が何か(実行ファイル・DLL・Song info の窓の資源)、どの FL の手続き(Song info の窓の資源か、Fruity Filter の資源か)が呼んだか、どのイメージを engine が PE として見ていないか。結果を、確かめた事実と推定に分けて報告に書く。
3. 直す: 既定の動作として直す(オプトインを作らない)。Windows と同じ結果になるように、資源の読み(`LockResource`)が、読めるイメージを返すようにする。読めない資源を、空の代わりに返さない(検査を弱めない)。
4. 確かめる: 2 件を開いて、engine が止まらず、Song info の窓が閉じられ、音の再生(Space)で `quantaWhilePlaying` が増えること。Blocks も、同じ道具で開けること。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && test -s .tmp-121/exoplanet.flp && test -s .tmp-121/dedzii.flp && for f in exoplanet dedzii; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-121/$f.flp" > "$RUNNER_TEMP/open-$f.json" || exit 1; done && node -e 'const fs=require("fs");const L=["exoplanet","dedzii"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.quantaWhilePlaying>0)?0:1)'
```

判定の基準: 2 件とも、`error` が null、`state` が engine-idle、再生の quantum が 1 以上(engine が止まらず、再生まで進むこと)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 停止の再現(2 件の JSON の行)、`LockResource` の呼び出しの特定、直した方式と理由、変えたパス(`app/`・`lift/` を含む。`app/` と `lift/` を変えると main の deploy が動く)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 展開したプロジェクトは、リポジトリに入れない(`.tmp-*` に置き、消す)。
- 依頼者の個人のプロジェクトは使わない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 35〜46 分。

## 他のタスクとの関係

- #{{004}}(P3 の demo の測定)が、この 2 件を候補にしている。この Issue が閉じると、候補が広がる。
- #{{019}}・#{{102}}(Fruity Filter と Fruity Balance の、起動後の取得)が、同じプラグインの読みの経路を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
