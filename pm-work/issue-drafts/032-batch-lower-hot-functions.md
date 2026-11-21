---
draft: "032"
title: "P3 の再生中に命令数の多い未直接の関数を、一括で直接 wasm にする"
type: Task
milestone: "M7 エンジンの性能(P3 のホットパス)"
parent: "030"
blocked_by: ["031"]
labels: ["perf"]
size: L
priority: Medium
project: signature-studio/3
---

## 目的

031 で採った P3 の再生中の内訳で、直接 wasm でない関数のうち命令数の多い物を、一括で直接 wasm にする。割合を上げて、P3 の余裕を増やす。

## 規模

規模: 大(想定 8 時間。1 回の受け入れのビルドが 25〜43 分。数回回す規模)

## 背景

- 031 の成果物: `inputs/p3-profile.tsv`(P3 の再生中の関数ごとの命令数)と、基準の割合 `baselinePct`。
- 直接 wasm にする手順の先例: 過去のバッチ(lowerb-0105・0107・0118・0126。main の履歴に在る。1 バッチ 40 関数。`lift/harness/hyb/lists/lowered-<module>.txt` と `lift/harness/hyb/lowered-all.txt` の末尾に足す)。その後、関数の塊を一括で足す方式を、この Issue で使う。1 関数 1 タスクには戻さない。
- 除外: atomic の関数(`lift/harness/seeds/mt4-atomic-functions.tsv`)。直接 wasm にして起動が通らなかった関数(`pm-work/inputs/lower-rejected.tsv`。理由は `listed but not emitted` など)。
- 判定: 起動が通ること(engine-idle・standIn が false・error が null)。

## 読む物

- `inputs/p3-profile.tsv`(031 の成果物)
- `lift/harness/hyb/lists/`(モジュール別の一覧。8 件)、`lift/harness/hyb/lowered-all.txt`
- `lift/harness/seeds/mt4-atomic-functions.tsv`(除外)
- `pm-work/inputs/lower-rejected.tsv`(落ちた関数の記録)
- `tools/instr-coverage/measure.mjs`(031 が作る道具)

## やること

1. 031 の内訳から、直接 wasm でない関数のうち、命令数の多い順に、除外を外した塊を選ぶ(塊の大きさは作業者が決め、理由を書く)。
2. 塊を、モジュール別の一覧と `lowered-all.txt` の末尾に足す。
3. ビルドと起動確認を行う。通らない場合は、落ちた関数を外し、`lower-rejected.tsv` に理由を書く。
4. 通った物を、1 つの commit にする。割合(`coveragePct`)が 99 以上で、かつ基準(`baselinePct`)以上であることを、道具の出力で確かめる。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/instr-coverage/measure.mjs inputs/p3-profile.tsv > "$RUNNER_TEMP/cov.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());process.exit(r.coveragePct>=99&&r.coveragePct>=r.baselinePct?0:1)' "$RUNNER_TEMP/cov.json"
```

判定の基準: 起動が通り、割合が 99% 以上で、基準以上であること。満たさない場合は「差し戻し」と書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 選んだ塊の大きさと理由、落ちた関数の数と理由、基準の割合と結果の割合、ビルドの回数、変えたパス(lift/ を含む)。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 1 タスク 1 関数に戻さない。
- atomic の関数を直接 wasm にしない(除外の一覧に従う)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の build を変えるので、main の deploy の仕組みが動く。報告に書く。

## 他のタスクとの関係

- blocked_by: 031。
- 034・035(起動の時間)と、同じ engine の build を使う。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、受け入れをもう一度通す。
