---
draft: "121"
title: "demo の曲 DEDZII を File > Open で開くと、Song info の窓の後に engine が止まる(LockResource: no PE image is mapped)を直す"
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

インストーラの demo の曲 `DEDZII - NIGHT TERROR.flp`(playlist 3,850 項目)を File > Open で開くと、Song info の窓(`TInfoForm`)が出た直後に engine が止まる(`fle: LockResource: no PE image is mapped at 0x400000 (no MZ signature)`)。

この Issue(段階 1)の目標: DEDZII と Blocks(テンプレート)を File > Open で開いて、LockResource の停止が出ず、engine が idle のまま再生の quantum が進むこと。

同じ demo の Exoplanet(playlist 4,440 項目)は、LockResource の停止の後に、別の停止(Fruity Balance の ExitProcess。終了コード 0)が出る。それは段階 2 の #{{128}} に分けた。

P3 の測定(#{{004}})は、音の出る demo を入力に使う。この曲は開けないことが P3 の入力を狭めている(P3 に関わるので High)。

## 背景

- 発生(run 45253143948。#{{004}} の作業者が、道具で開いて確かめた。実測): DEDZII と Exoplanet は、Song info の窓が出た直後に、ログ末尾が `fle: LockResource: no PE image is mapped at 0x400000 (no MZ signature)` になり、停止する。Blocks は止まらない(サンプル不足の窓は出るが、engine は止まらない。#{{004}} の 3 回の測定)。
- 原因(前回の試行で特定。実測と推定。下の「前回の試行で分かったこと」): `LockResource` は、exe の木で見つからない資源を、実モジュールの表の順に `img_open` していた。#{{101}} 以降は、読み込み(LoadLibrary)の前の像が写されていないため、その像の MZ の検査で止まる。停止文言の `0x400000` は固定の文字列で、実際の基準ではない。
- 関係: #{{019}}(Fruity Filter の起動後の取得)と #{{102}}(プラグインのコードを分ける)が、同じ経路(`fle_res.c`・`fle_realmod.c`)を変える。作業の最初に、`git log --oneline -- lift/runtime lift/harness app/src/engine` で、近い変更を確かめる。
- 道具: `tools/p3-audio-probe/measure.mjs` の File > Open の手順は、前回の作業の差分(下の 6)に入っている。main の版は古い。

## 前回の試行で分かったこと(3 回目。run 45346925057。`failed`)

- 判定: 受け入れは通った(判定の表示は「判定は通った」)。落ちたのは main への push だけ: `CONFLICT (content): Merge conflict in tools/p3-audio-probe/measure.mjs`(他の Issue が、同じ道具を変えたため)。作業は保存された(差分は取り込みの段で作業ツリーに載る)。作業者の commit `dd2bc0bc` は作業者のツリーの commit。main には無い。
- 直した内容(作業者の実装):
  - `lift/runtime/fle_res.c`(`LockResource`): 写されていない実モジュールの像を読まずに飛ばす。`img_open` の停止文言の基準を、固定の `0x400000` から実際の基準に直した。
  - `lift/runtime/fle_realmod.c`・`fle_realmod.h`: `fle_realmod_image_mapped(idx)` を足した。写す hook(`fle_realmod_map_image`)の無い link(census の harness など)では常に真。hook の在る link(製品の経路)では、`LoadLibrary` で headers を読んだ後に真。hook の無い link の動きは従来どおり。
  - `tools/p3-audio-probe/measure.mjs`: 1 回目の全体の実行で DEDZII が「40 秒後に開く窓が無い」で落ちた。原因は、窓の表を取る dump が「絵を表示中」で拒否され(`ask again`)、`Problems loading the project` の窓を見つけられなかったこと。前面の窓の題名(`pageTitle()`)でも、この窓を判定するようにした。成功時も engine のログ末尾 400 行を出す。
- 作業者の実行の値(runner の値。nproc 4、loadavg 1.1〜1.9。マージ後の木・最後の commit `dd2bc0bc`):
  ```
  boot:    {"error":null,"state":"engine-idle","standIn":false}
  DEDZII:  {"error":null,"state":"engine-idle","standIn":false,"keyAtGuest":true,"quantaWhilePlaying":9158,"quantaDelta":3448,"underrunDelta":0,"endTitle":"NIGHT TERROR - FL Studio 2026"}
  BLOCKS:  {"error":null,"state":"engine-idle","standIn":false,"keyAtGuest":true,"quantaWhilePlaying":9147,"quantaDelta":3448,"underrunDelta":0,"endTitle":"blocks.flp - FL Studio 2026"}
  ACCEPT_EXIT=0
  ```
  DEDZII と Blocks のどちらも、engine のログ末尾 400 行に `LockResource` の行は 0 件。
- 経過: 1 回目の全体(`8ee1ca3b` まで)は、DEDZII が上記の道具の問題で `ACCEPT_EXIT=1`。道具を直して単独で通し、main(`fa5642e3`)を取り込んだ後に、全体を再実行して `ACCEPT_EXIT=0`。その後、hook の無い link の判定を直し(`dd2bc0bc`)、もう一度全体で `ACCEPT_EXIT=0`。
- 次の試行でやること: 差分を取り込み、`tools/p3-audio-probe/measure.mjs` の衝突を直す。`measure.mjs` は、他の Issue も変えている。衝突したら、main の版を取り、この Issue の変更(前面の窓の題名での Problems の窓の判定・成功時のログ末尾の出力)だけを取り直す。両方の変更を残す。その後、受け入れを通す(main が動いているので、結果は変わりうる)。
- 未確認: Exoplanet は測っていない(#1372)。

## 2 回目の試行で分かったこと

run 45315619741(2 回目。作業は保存された: `work.diff`。変えたのは `lift/runtime/fle_res.c`・`lift/runtime/fle_realmod.c`・`lift/runtime/fle_realmod.h`・`tools/p3-audio-probe/measure.mjs`。判定は差し戻し)。

1. 原因(実測と推定): LockResource の修正前は、exe の木で見つからない `hResData` を、実モジュールの表の順に `img_open` していた。写されていない像(遅延の読み込みの対象。#{{101}})を読むと、MZ の検査で停止する。実測: `FindResourceW` の `h 0x7000000` から取れた資源を、LockResource が `found 1` で返す(0x7000000 は Fruity Filter の像)。
2. 直した方式: `fle_realmod_image_mapped(idx)` を足した(像が写されたかを、`headers` で判定する)。`LockResource` は、写されていない実モジュールの像を読まずに飛ばす。NULL・見つからない時の停止・MZ の検査は残した。`img_open` の停止文言の `0x400000` は、実際の基準に直した。
3. 修正後の実測(runner の値。nproc 4。loadavg 0.31 0.88 1.49): DEDZII は `error` null、`state` engine-idle、`quantaWhilePlaying` 9255、`quantaDelta` 3452、`underrunDelta` 0、`keyAtGuest` true。Blocks は `error` null、engine-idle、`quantaWhilePlaying` 9251、`quantaDelta` 3449、`underrunDelta` 0。LockResource の停止は、この 2 件では出なくなった(受理)。
4. Exoplanet は別の停止: `[err] fle: ExitProcess: the program asked to end the process (exit code 0); stopped here, not exited`(Fruity Balance の読み込みの直後)。段階 2(#{{128}})へ分けた。
5. 判定の段で落ちた理由: 受け入れの `fe-build engine` の blend2d の取得で、取得先への接続が 133 秒で失敗した(取得の失敗の表示)。通信の一時的な失敗で、作業者の実行では、この段は通っていた。次の試行は、取り直しで通る見込み(推定)。
6. 測定の道具の直し(`tools/p3-audio-probe/measure.mjs`。差分に入っている): Welcome は Escape で閉じる。既定のプロジェクトの「Problems loading the project」は Enter で閉じる。File > Open は Ctrl+O で、ファイル窓の `#fx-input-files` に渡し、`#fx-ok` を押す。開いたかは、メインの窓の題名が変わったことで判定する(`pageTitle()` は前面の窓の題名なので、Song info の間は別の題名になる)。失敗時は、状態と engine のログの末尾 400 行を JSON に出す。
7. 次の試行で必要なこと: 差分(上の 2・6)が載っていれば、それを使う。受け入れを通す(DEDZII と Blocks)。Exoplanet は、この Issue では測らない。

## 読む物

- `lift/runtime/fle_res.c`・`fle_realmod.c`・`fle_realmod.h`(`LockResource` と、写された像の判定)。`fle_seh.c` も同じ関数を使うが、LockResource の経路ではない。
- `tools/p3-audio-probe/measure.mjs`(開く手順。失敗時のログ末尾)
- `tools/smoke/welcome-flow.mjs`(Welcome の窓の閉じ方)

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば読む。`git status` と `git diff --stat` で、前回の差分(3 回目の作業)が載っているかを確かめる。
2. 差分が載っていなければ、3 回目の変更(`lift/runtime/fle_res.c`・`fle_realmod.c`・`fle_realmod.h`・`tools/p3-audio-probe/measure.mjs`)を、同じ内容で作る(推定ではなく、実測の値に合わせる)。`measure.mjs` が main と衝突したら、main の版を取り、この Issue の変更だけを取り直す(両方の変更を残す。上の 3 回目の節)。
3. 受け入れを通す(DEDZII と Blocks。Exoplanet は測らない)。
4. commit する。受け入れが通らなくても、作業の最後に作業ツリーを commit する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-121 && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-121/dedzii.flp && cp "lift/out-app/host/resources/Data/Templates/Other/Blocks/Blocks.flp" .tmp-121/blocks.flp && for f in dedzii blocks; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-121/$f.flp" > "$RUNNER_TEMP/open-$f.json"; done && node -e 'const fs=require("fs");const L=["dedzii","blocks"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.quantaWhilePlaying>0)?0:1)'
```

判定の基準: DEDZII と Blocks の 2 件とも、`error` が null、`state` が engine-idle、再生の quantum が 1 以上(engine が止まらず、再生まで進むこと)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 2 件の JSON の行、LockResource の停止が消えたことの証拠(ログ末尾)、直した方式と理由、変えたパス(`app/`・`lift/` を含む。`app/` と `lift/` を変えると main の deploy が動く)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 展開したプロジェクトは、リポジトリに入れない(`.tmp-*` に置き、消す)。
- 依頼者の個人のプロジェクトは使わない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは約 35 分(直近は約 48 分)。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。`sleep`・`timeout` をコマンドの先頭に書かない。

## 他のタスクとの関係

- #{{128}}(段階 2: Exoplanet の ExitProcess)は、この Issue が閉じてから始まる(blocked by)。
- #{{129}}(demo の素材の配信)は、この Issue と #{{122}} が閉じてから始まる。
- #{{004}}(P3 の demo の測定)が、この Issue の結果を使う。
- #{{019}}・#{{102}}(Fruity Filter と Fruity Balance の、起動後の取得)が、同じプラグインの読みの経路を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
