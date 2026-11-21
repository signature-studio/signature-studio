---
draft: "031"
title: "P3 の再生中の命令の内訳を採り、直接 wasm でない関数の割合を測る"
type: task
milestone: "M7 エンジンの性能(P3 のホットパス)"
parent: "030"
blocked_by: ["003"]
labels: ["task", "perf", "verify", "priority-p3", "needs-owner"]
size: 5
needs_owner: true
---

## 目的

P3 の再生中に実行される命令の内訳を、関数ごとに採る。直接 wasm でない関数の割合を、P3 の再生の中で実測する。032 の対象の選び方の根拠にする。

## 規模

規模: 大(想定 5 時間。採取の道具と、所定のプロジェクトでの採取)

## 背景

- 030 の読み: 既存の表(`pm-work/inputs/lift-function-classes.tsv`)の命令数の 99.65% は、直接 wasm の関数に入っている。表の出典(どの実行か)は未確認。このため、P3 の再生中の内訳を、この Issue で採り直す。
- 所定のプロジェクト(依頼者の個人のファイル)が要る。取得の経路は未確定(QUESTIONS 1)。このため、この Issue は依頼者の入力が揃うまで起動しない(needs-owner)。
- 命令数を関数ごとに採る道具が、リポジトリに在るかは未確認。無ければ作る。
- 子 003 の音声の道具(AudioContext の開始の方法)を使う。

## 読む物

- `tools/p3-audio-probe/measure.mjs`(子 003 の道具)
- `pm-work/inputs/lift-function-classes.tsv`(表の形。列: va・module・instructions)
- `lift/harness/hyb/lists/lowered-b2d.txt` ほか 7 件(直接 wasm の一覧。形は VA の 1 列)
- `lift/harness/seeds/mt4-atomic-functions.tsv`(直接 wasm にしない関数の一覧。atomic)
- `pm-work/inputs/lower-rejected.tsv`(直接 wasm にしたが起動が通らなかった関数)
- `tools/verify-kit/tools/scn-p3-mixmove-300.json`(P3 の台本の候補)

## やること

1. 入力が runner にあることを確かめる(報告に `test -f` の結果を書く)。無ければ、ここで止めて「実エンジン: 実行なし(入力の所定プロジェクトが runner に無い)」と報告する。
2. 関数ごとの命令数を、P3 の再生の中で採る。採り方が既存に無ければ、採り方を作る。採り方と限界を報告に書く。
3. 採った内訳を `inputs/p3-profile.tsv` にする(列: `va`・`module`・`instructions`。見出し行を 1 行残す)。
4. 直接 wasm でない関数の命令数の割合を、モジュール別に出す。計算の道具 `tools/instr-coverage/measure.mjs` を作り、JSON の 1 行を出す: `{"instrTotal","instrDirect","coveragePct","baselinePct"}`。`baselinePct` は、最初の計算の値を `inputs/p3-profile.tsv` の隣に記録する(032 の判定の基準)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && test -s inputs/p3-profile.tsv && node tools/instr-coverage/measure.mjs inputs/p3-profile.tsv > "$RUNNER_TEMP/cov.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.instrTotal>0&&typeof r.coveragePct==="number"&&typeof r.baselinePct==="number"?0:1)' "$RUNNER_TEMP/cov.json"
```

判定の基準: 採った内訳が空でなく、割合が計算でき、基準の値が記録されていること。割合の目標は QUESTIONS 9。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 採り方、採った時間、命令数の合計、モジュール別の割合(直接 wasm の関数の割合)、既存の表(030 の 99.65%)との違い。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。計測の切り替えを、製品に作らない(オプトイン)。
- 依頼者の個人のプロジェクトを、リポジトリに入れない。
- Python を製品に使わない(L-008)。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。
- sleep・timeout・nohup・末尾の `&` は使わない。

## 他のタスクとの関係

- blocked_by: 003(音声の道具。P3 の再生の開始の方法)。
- 032 の対象を決める。
