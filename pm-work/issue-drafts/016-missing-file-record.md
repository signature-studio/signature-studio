---
draft: "016"
title: "起動時にホストに無かったファイルを、engine のログに記録する"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources"]
size: M
priority: Medium
project: signature-studio/3
---

## 目的

課題 C-3: FL Studio が必要とする物を揃える。今の記録の仕組みでは、engine がファイルを要求して無かった場合が見えない。このため、無かったパスを記録する仕組みを足し、どのファイルが足りないかを実測で確かめられるようにする。

## 背景

- 旧の測定(pm-work/findings/survey-c3-fl-resources.md の 5 節。runner の値): 起動の engine ログは 322 行。「見つから」「not found」「ENOENT」「no such」「not exist」「CreateFile」「FindFirst」「GetFileAttributes」は 0 件。ただし、欠落の証明にはならない(記録の仕組みが無いので、ファイルを開いた記録も出ない)。
- 失敗らしい行は 3 種類だけ: `InternetOpenUrlW` の失敗 2 件(エラー 12007)、描画の破棄 1 件(`first drawing call discarded ...`)、entry からの戻り 1 件。
- ログの経路: `app/src/engine/worker.ts` の `pushLog`(engine のログの公開は `app/src/main.ts`。`window.__app.engineLog()`)。
- VFS の失敗の扱い: `lift/runtime/fle_vfs.c`(ファイルの open・stat・read)、`lift/runtime/fle_load.c`(LoadLibrary の失敗)。

## 読む物

- `lift/runtime/fle_vfs.c`(ファイルの open・stat・読み書きの失敗の扱い)
- `lift/runtime/fle_load.c`(LoadLibrary の失敗の扱い)
- `app/src/engine/worker.ts`(`pushLog`。engine のログの経路)
- `app/src/main.ts`(`engineLog` の公開)
- `pm-work/findings/survey-c3-fl-resources.md`(5 節。旧の記録)
- `tools/boot-check/boot-bundle.mjs`、`tools/boot-check/cdp.mjs`

## やること

1. `lift/runtime/fle_vfs.c` を読み、ファイルの要求が失敗した箇所を全て洗い出す。洗い出した箇所の一覧を、報告に書く。
2. 失敗した時に、その guest のパスを 1 行で engine のログに出す。ログの行は、既存の行と混ざらない形にする(例: 先頭に固定の語を付ける)。新しい環境変数・#ifdef を足さない。
3. 測定の道具 `tools/missing-paths/measure.mjs` を作る。boot-bundle と同じ起動で、engine-idle の後、3 回起動し、各回の JSON の 1 行を出す: `{"state","standIn","error","elapsedMs","missing":[guest のパス]}`。`missing` は engine のログの記録から集める。
4. 見つからなかったパスの一覧を、`pm-work/findings/missing-paths.md` に書いて commit する。パス、回数、どの起動で出たかを書く。0 件なら、0 件の事実と、確かめ方を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/missing-paths/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/missing.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&Array.isArray(r.missing))?0:1)' "$RUNNER_TEMP/missing.jsonl" && test -s pm-work/findings/missing-paths.md
```

判定の基準: 道具の 3 行の生の値が valid で、`missing` が配列であること。`pm-work/findings/missing-paths.md` が空でないファイルとして commit されていること。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 失敗箇所の一覧(fle_vfs.c・fle_load.c の何行)、見つからなかったパスの要点、ログの行の形、変えたパス(lift/・app/ を含む)、elapsedMs の 3 回の値。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。記録の有無を切り替える仕組みを作らない(オプトインを作らない)。
- 製品の既定の動作を変えない(記録を足すだけ)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- #{{017}}(言語・テーマ・テンプレートの全件)が、この Issue の記録を使う。
- `lift/runtime/` を変える。`lift/` の変更は main の deploy の仕組みを動かすので、報告に書く。
