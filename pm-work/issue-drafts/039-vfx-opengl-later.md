---
draft: "039"
title: "OpenGL を使う VFX 系の内蔵プラグインを、全群の後に 1 プラグイン 1 wasm で入れる(後回し)"
type: task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["021", "022", "023"]
labels: ["task", "plugin-builtin", "needs-owner"]
size: 8
needs_owner: true
---

## 目的

台帳 L-034: OpenGL を使う VFX 系は後回しにしてよい。それ以外の内蔵プラグインは、全て lift の対象。この Issue は、後回しにした VFX 系(023 の一覧)を、全群の後に 1 プラグイン 1 wasm で入れる。着手は依頼者が決める(QUESTIONS 4)。

## 規模

規模: 大(想定 8 時間。OpenGL の描画の経路を含むため、ビルドを約 5 回回す規模)

## 背景

- 対象: `inputs/vfx-deferred.tsv`(023 の成果物)の全件。判定の規則は、import に `opengl32.dll` を含む DLL。
- 後回しの理由(台帳 L-034): OpenGL を使う描画の経路が、ブラウザでの WebGL の描画と合っているかを、この Issue で確かめる必要がある(推定)。
- 型: 015・019(1 プラグイン 1 wasm の起動後の届け方)と、群(021・022)の作り方を使う。
- 群の完了(021・022)の後に行うのは、依頼者が後回しを解く判断をしたとき。この Issue は、その判断が出るまで起動しない(`needs-owner`)。

## 読む物

- `inputs/vfx-deferred.tsv`(023 の成果物)
- `inputs/fl-installer-files.tsv`(013 の成果物)
- `lift/harness/b2d2/modules.tsv`、`lift/harness/web/pack.args`(表の形)
- `tools/delivery-split/measure.mjs`(014 が作る道具)
- `app/src/engine/desk-gl.ts`(WebGL の描画の経路。既存)

## やること

1. 依頼者の判断(QUESTIONS 4)が出ているかを、報告の先頭に書く。出ていない場合は、この Issue は起動しない。
2. 対象の DLL を、020 と同じ表の形で揃える。OpenGL の描画の経路が、ブラウザで動くかを、最初の 1 件で確かめる。動かないなら、理由を報告に書いて止まる。
3. 全件を、起動後の取得で届ける。
4. 起動確認と、取得の確認を行う。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/vfx.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const n=fs.readFileSync("inputs/vfx-deferred.tsv","utf8").trim().split("\n").length-1;process.exit(n>0&&L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.lazy.length>=n)?0:1)' "$RUNNER_TEMP/vfx.jsonl"
```

判定の基準: 3 回の起動で、engine-idle に達し、後回しの件数(`inputs/vfx-deferred.tsv` の件数)以上が起動後の取得に在る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 依頼者の判断が無い場合は、「実エンジン: 実行なし(依頼者の判断が無い)」と書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。
- 1 タスク 1 関数に戻さない(L-037)。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: 021・022(内蔵の残りの全群)、023(後回しの一覧)。
- 020 の表の形を使う。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
