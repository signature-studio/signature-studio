---
draft: "184"
title: "DirectSound の通知位置をバイト単位で受け、再生の状態は音声の準備の後に公開する(fle_dsound.c)"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["180"]
labels: ["perf", "audio"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

P3(Song Mode で音が途切れない)。DirectSound の仕様では、通知位置(`SetNotificationPositions`)のバイト位置は、サンプルの境界でなくてよい。main の今の `fle_dsound.c` は、サンプル境界でない通知位置を拒否し、サンプルへ切り捨てて扱う。また、再生の状態(`PLAYING`)を、音声の準備(`audio_sync`)より先に公開する。この Issue で、両方を仕様どおりに直し、P3 の指標で効果を測る。

## 背景

- main の今の版(`lift/runtime/fle_dsound.c`):
  - `m_n_setpos`: `a[k].off % b->fmt.align` のとき `HR_INVALIDPARAM` を返す(サンプル境界だけ受理)。
  - `fire_notifications` と `fle_audio_next_tick`: 通知位置を `off / b->fmt.align` と切り捨てて扱う。
  - `m_b_play`: `fle_audio_ring_set_state(FLE_AUDIO_PLAYING)` の後で `audio_sync()` を呼ぶ。再生の状態が先に出る。
- ブランチの版(`pm-work/findings/perf-windows-browser/patches/02-audio-unlock-dsound-notify-play-order.patch` の `fle_dsound.c` と `ds_scenarios.c`): バイト位置は `(off + align - 1) / align`(切り上げ)でサンプルに直す。`m_n_setpos` は、バッファの大きさの範囲なら受理する。`m_b_play` は、`audio_sync()` を先に行い、その後で状態を `PLAYING` にする。試験 `sc_notify_bytes`(通知位置 1 と 3527 バイト)を、`ds_scenarios.c` に足す。
- 効果の根拠: 推定。ブランチでは、指定の評価用プロジェクトでこの API が効いたかは未確認(報告の記述)。ネイティブの試験は通っている(25 件)。音声の準備の後に状態を出す点は、起動直後の音声の欠落(4 quanta)との関係が推定されるだけで、実測は無い。

## 読む物

- `lift/runtime/fle_dsound.c`(`fire_notifications`・`fle_audio_next_tick`・`m_n_setpos`・`m_b_play`・`audio_sync`・`fle_audio_ring_set_state`)
- `lift/harness/audio/ds_scenarios.c`(`sc_notify`・`sc_notify_alone`・`S[]` の表)、`lift/crates/fe-lift/tests/wall_fr15.rs`(この試験を動かす)
- `pm-work/findings/perf-windows-browser/patches/02-audio-unlock-dsound-notify-play-order.patch`(ブランチの版。参考。このうち、unlock の情報の部分は取り込まない)

## やること

1. `fire_notifications`・`fle_audio_next_tick`・`fle_dsound.c` の該当箇所で、通知位置を `(off + align - 1) / align` でサンプルに直す。`m_n_setpos` の、サンプル境界の検査を外し、範囲(`off < size`)だけを検査する。
2. `m_b_play` で、`audio_sync()` を `fle_audio_ring_set_state(FLE_AUDIO_PLAYING)` より先に呼ぶ。
3. 試験 `sc_notify_bytes` を `ds_scenarios.c` に足し、`S[]` の表に登録する。`wall_fr15.rs` で、この試験を動かす(既存の試験の期待値は変えない)。
4. 計測用の unlock の情報(`fle_audio_unlock_note` の引数の追加)は、この Issue では入れない。

## 受け入れ

```accept
node tools/ab/base.mjs "$(git merge-base HEAD origin/main)" .tmp-base && (cd .tmp-base && npm ci --prefix app && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle) && node tools/ab/base.mjs --verify .tmp-base && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && cargo test --release --locked --manifest-path lift/Cargo.toml -p fe-lift --test wall_fr14 --test wall_fr15 && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-184-boot.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-184-boot.json","utf8"));process.exit(r.error===null&&r.state==="engine-idle"&&r.standIn===false?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-184-flow.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-184-flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' && node tools/ab/run.mjs p3 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > .tmp-184-ab-p3.json && node -e 'const s=JSON.parse(require("fs").readFileSync(".tmp-184-ab-p3.json","utf8"));process.exit(s.pass===true?0:1)' && node tools/ab/run.mjs p2 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > .tmp-184-ab-p2.json && node -e 'const s=JSON.parse(require("fs").readFileSync(".tmp-184-ab-p2.json","utf8"));process.exit(s.pass===true?0:1)'
```

判定の基準:
- p3 の判定(`tools/ab/gates.mjs`): 変更後の全 run で `error` が null・`playing` が true・`last60sUnderruns` が 0・`inputRateOk` が true・`nonSilentDelta` が 0 より大きい・`pcmRepeat8192` が null か 0.01 未満。加えて、`underrunDelta` の変更後の中央値が基準の中央値以下(絶対の 0 は求めない。起動後の途切れは、別の規則で見る)。
- `wall_fr14` と `wall_fr15`(`sc_notify_bytes` を含む)が通る。既存の試験の期待値は変えていない。
- 起動の確認と流れ A〜D が全て合格(受け入れのとおり)。
- `tools/ab/run.mjs` の `p3` と `p2` が、回帰なしで `pass` が true。
- 残す条件: `wall_fr15` の `sc_notify_bytes` が、通知の位置をバイト単位で正しく返すことを示す(仕様の修正。効果が測れなくても残す)。加えて、`p3` の `pcmRepeat8192` の基準と変更後の中央値が数値で、変更後の中央値が基準の中央値 + 0.01 以下。`underrunDelta` は、変更後の中央値が基準の中央値以下(回帰の確認)。どちらかが満たされないなら、この変更を取り消し、測定の表だけを `pm-work/findings/perf-windows-browser/184-result.md` に書いて commit する。取り消した後も、受け入れのコマンドは通る。
- 報告に書く: 6 回の run ごとの `pcmRepeat8192`・`pcmEligible`・`underrunDelta`・`nonSilentDelta`・`minWindowFps`、中央値、判定(残す・取り消す)。「達成」と書かず、「受理」か「差し戻し」を、出力の行で書く。

## 前回の試行で分かったこと

- 基準の準備が通らなかった: `git archive` の複製には `lift/out-app/host` が無く、基準の `fe-build engine` が `binaries/win64/FLEngine_x64.dll` で止まった。`tools/ab/base.mjs` で持ち込むようにした。
- 効果は測れなかった: `pcmRepeat8192` は基準と変更後がともに 0。`underrunDelta` は、基準と変更後で同じ。
- 判断: 仕様の修正として残す(`sc_notify_bytes` で、通知の位置をバイト単位で仕様どおり返す)。
- 前回の変更は main に入っていない。この Issue で、下の「やること」から作り直す。

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
- `lift/runtime/fle_dsound.c` と `lift/harness/audio/ds_scenarios.c` は、今の open の Issue とは重ならない(#{{109}}・#{{112}}・#{{131}} は閉じている)。
- #{{183}}(スレッドの切り替え)とは、`fle_dsound.c` と `fle_thread.c` の別の箇所を触る。
