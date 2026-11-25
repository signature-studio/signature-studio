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

1. `fle_input_host_mouse` で、移動(`FLE_IN_MOVE`)の時、待ち行列の最後の件が移動なら、その件の座標・`mk`・`ticks` を新しい値にする。件数は増やさない。`note_changebits` と `fle_thread_wake_check` は、今のとおり呼ぶ。
2. ボタン・ホイール・キーは、今のとおり新しい件として積む(統合しない)。
3. 満杯の時でも、最後の件が移動なら、置き換えて受理する。置き換えられない時だけ、今のとおり捨てる。
4. 試験を足す: `g3_scenarios.c` に、移動 3 件が 1 件になること・ボタンを挟むと統合しないこと・満杯で置き換えられること。`wall_fr14.rs` に、その試験を登録する。

## 受け入れ

```accept
rm -rf .tmp-base && mkdir .tmp-base && git archive "$(git merge-base HEAD origin/main)" | tar -x -C .tmp-base && (cd .tmp-base && npm ci --prefix app && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle) && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && cargo test --release --locked --manifest-path lift/Cargo.toml -p fe-lift --test wall_fr14 && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-182-boot.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-182-boot.json","utf8"));process.exit(r.error===null&&r.state==="engine-idle"&&r.standIn===false?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-182-flow.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-182-flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' && node tools/ab/run.mjs p2 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > .tmp-182-ab-p2.json && node -e 'const s=JSON.parse(require("fs").readFileSync(".tmp-182-ab-p2.json","utf8"));process.exit(s.pass===true&&s.medians.inputDropped.after<=s.medians.inputDropped.base?0:1)' && node tools/ab/run.mjs p3 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > .tmp-182-ab-p3.json && node -e 'const s=JSON.parse(require("fs").readFileSync(".tmp-182-ab-p3.json","utf8"));process.exit(s.pass===true?0:1)'
```

判定の基準:
- `wall_fr14`(移動の統合・ボタンを挟む・満杯の試験を含む)が全て通る。
- 起動の確認と流れ A〜D が全て合格(受け入れのとおり)。
- `tools/ab/run.mjs` の `p2` と `p3` が、回帰なしで `pass` が true。
- 効果の判定(残す条件): 基準の `inputDropped` の中央値が 0 より大きく、変更後の中央値がそれより小さい。届かないなら(基準が 0 の時を含む)、この変更を取り消す。取り消した後も、受け入れのコマンドは通る(基準と変更後が同じになる)。測定の表だけを `pm-work/findings/perf-windows-browser/182-result.md` に書いて commit する。
- 報告に書く: 6 回の run ごとの `inputDropped`・`inputToPresentMsP95`・`inputRateOk`・`minWindowFps`、中央値、判定(残す・取り消す)。

## 作業の進め方

- engine のビルドは約 35 分。基準(変更前)と変更後の 2 回の engine のビルドが要る(受け入れのコマンドに入っている)。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して、直接実行する。背景で起動して、終わるのを待たない。
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
