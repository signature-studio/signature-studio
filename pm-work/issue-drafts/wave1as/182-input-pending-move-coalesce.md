---
draft: "182"
title: "保留中の連続したマウスの移動を 1 件にまとめ、入力の待ち行列が満杯の時に入力が落ちる数を減らす(fle_input.c)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["180"]
labels: ["perf", "input"]
size: S
priority: High
project: signature-studio/3
---

## 目的

1000 Hz 級のマウス入力の間、エンジンが一時的に止まると、入力の待ち行列(`fle_input.c` の `evq`、512 件)に移動が積もり、満杯になった後の入力は捨てられる。連続した移動を 1 件にまとめれば、待ち行列は短く保たれ、捨てられる入力が減る。この Issue で、それを直し、効果を測る。

## 背景

- main の今の版(`lift/runtime/fle_input.c` の `fle_input_host_mouse`): 移動・ボタン・ホイールを、必ず新しい 1 件として積む。`if (ev_n >= EV_MAX) return 0;` で、満杯の時は捨てる(`EV_MAX` は 512)。
- ブランチの版(`pm-work/findings/perf-windows-browser/patches/05-input-pending-move-coalesce.patch`): 待ち行列の最後が移動で、新しいのも移動の時は、最後の件の座標と時刻を置き換える(件数は増えない)。ボタン・ホイール・キーをまたいでは統合しない。満杯でも、置き換えられる時は捨てない。
- ブランチの試験: 移動 3 件が 1 件にまとまる。ボタンを挟むと統合しない。満杯の状態で移動を送ると、置き換えられる(`g3_scenarios.c` の `coalesce_boundaries`・`coalesce_full`)。
- 効果の根拠: 推定。ブランチでは実測していない。効果は、エンジンが止まって待ち行列が満杯になる時だけ出る。1200 Hz の入力(`tools/p2-frames/measure.mjs`)で、落ちた入力の数(`inputDropped`。#{{180}} で出す)が 0 なら、この変更は効果が無いので取り消す。

## 読む物

- `lift/runtime/fle_input.c`(`fle_input_host_mouse`・`evq`・`ev_head`・`ev_n`・`EV_MAX`・`note_changebits`・`fle_thread_wake_check`)
- `lift/harness/web/g3/g3_scenarios.c`(`sc_coalesce` と、その試験の登録)、`lift/crates/fe-lift/tests/wall_fr14.rs`(この試験を動かす)
- `pm-work/findings/perf-windows-browser/patches/05-input-pending-move-coalesce.patch`(ブランチの版。参考)
- `tools/p2-frames/measure.mjs`(`inputDropped`・`inputRateOk`・`inputToPresentMsP95`)

## やること

1. 変更を入れない。`lift/` と `app/` に差分を作らない(前回の試行で効果が出ず、捨てると判断した。下の「前回の試行で分かったこと」)。
2. 測定の表 `pm-work/findings/perf-windows-browser/182-result.md` を commit する(判定: 捨てる。値は下の節のとおり)。

## 受け入れ

```accept
test -s pm-work/findings/perf-windows-browser/182-result.md && grep -q '判定: 捨てる' pm-work/findings/perf-windows-browser/182-result.md && test -z "$(git diff --name-only "$(git merge-base HEAD origin/main)" -- lift app)"
```

判定の基準:
- `pm-work/findings/perf-windows-browser/182-result.md` が在り、`判定: 捨てる` と、`inputDropped` の基準と変更後の値を書いている。
- `lift` と `app` に、基準(`origin/main` との共通の祖先)からの差分が無い(変更を入れていない)。
- 報告に書く: 測定の表の値(`inputDropped`・`inputToPresentMsP95`・`minWindowFps`)、判定(捨てる)、表のパス。6 回の run ごとの値は前回の run のログに在り、この Issue では測り直さない。

## 前回の試行で分かったこと

- 基準の準備が通らなかった: `git archive` の複製には `lift/out-app/host` が無く、基準の `fe-build engine` が `binaries/win64/FLEngine_x64.dll` で止まった(判定の再実行でも同じ段で落ちた)。
- 効果が出なかった: `inputDropped` は、基準 58 から変更後 78 に増えた(減らない)。`inputToPresentMsP95`(84.4 → 67.0)と `minWindowFps`(14.5 → 15.9)は、変わらない範囲と判断した。
- 判断: 捨てる。変更は入れない。残す条件(変更後の `inputDropped` が基準より小さい)が満たされなかった。
- 測定の表: `pm-work/findings/perf-windows-browser/182-result.md`(判定: 捨てる)。

## 作業の進め方

- この Issue は engine を作り直さない(変更を入れないため)。
- 作業は開始から 4 時間 30 分以内に終える。最初に `date -u` で開始を記録する。その時刻を過ぎたら、受け入れが通っていなくても作業ツリーを commit して、報告を書いて終える。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python を使わない。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `lift/` の変更は、push で main の `deploy.yml` を動かす。報告に書く。

## 他のタスクとの関係

- blocked_by: #{{180}}(計測の道具)。
- `lift/runtime/fle_input.c` の他の変更(#{{106}}・#{{108}}・#{{109}}・#{{139}} は閉じている)とは、今は重ならない。
- 衝突: `lift/harness/web/g3/g3_scenarios.c` と `lift/crates/fe-lift/tests/wall_fr14.rs` を触る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、直して、受け入れをもう一度通す。
