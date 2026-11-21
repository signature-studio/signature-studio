---
draft: "019"
title: "Fruity Filter と Fruity Delay 2 を、起動後の取得で届ける(1 プラグイン 1 wasm)"
type: task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["015"]
labels: ["task", "plugin-builtin", "delivery", "priority-p1"]
size: 4
needs_owner: false
---

## 目的

台帳 L-035 と L-034: 内蔵のエフェクトを、起動後にバックグラウンドで届ける(1 プラグイン 1 wasm)。Fruity Filter と Fruity Delay 2 を、015 で作った型に乗せる。

## 規模

規模: 大(想定 4 時間。ビルドを約 5 回回す規模)

## 背景

- Fruity Filter: `lift/harness/b2d2/modules.tsv` の 9 行(`fruity_filter_x64.dll`・base `0x7000000`・hook 点のファイルを使う)。DLL の大きさは 3,780,608 B(survey-c4 の実測)。seed の表は 246 行。
- Fruity Delay 2: `lift/harness/b2d2/modules.tsv` の 10 行(base `0x7800000`。hook 点のファイルを使う)。DLL の大きさは 3,731,968 B。seed の表は 247 行。`ci/extract-dlls.tsv` の 18 行(追加の記録は 17 行の注釈)。
- hook 点: `lift/harness/hooks/fruity-delphi-effect-hook-points.txt` の 4 行は、Fruity Delay 2 も同じ 3 つの RVA だと書く。この記述は、旧の作業では未検証(pm-work/findings/survey-c4-builtin-plugins.md の手順 2)。この Issue で確かめる。
- 015(Fruity Balance の型)の後に行う。

## 読む物

- `lift/harness/b2d2/modules.tsv`(9〜10 行)
- `lift/harness/hooks/fruity-delphi-effect-hook-points.txt`(2〜4 行、12〜14 行)
- `lift/harness/web/pack.args`(13〜14 行の `--args`)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(246・247 行)
- `ci/extract-dlls.tsv`(17・18 行)
- `tools/delivery-split/measure.mjs`(014 が作る道具)
- 015 の成果物(Fruity Balance の型)

## やること

1. Fruity Filter と Fruity Delay 2 の hook 点の RVA を、DLL の実物で確かめる(報告に書く。確かめられなければ「未確認」)。
2. 015 の型で、2 件を起動後の取得に移す。
3. 2 件それぞれが、起動後に届き、engine が読み込むことを、道具の出力で確かめる。
4. 大きさ(起動前の取得の合計)の変化を、報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/fx.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const has=(a,re)=>a.some((f)=>re.test(f.name));const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&!has(r.preStart,/fruity.?filter/i)&&!has(r.preStart,/fruity.?delay/i)&&has(r.lazy,/fruity.?filter/i)&&has(r.lazy,/fruity.?delay/i));process.exit(ok?0:1)' "$RUNNER_TEMP/fx.jsonl"
```

判定の基準: 3 回の起動で、2 件とも起動前の取得に無く、起動後の取得に在り、engine-idle に達する。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: hook 点の RVA の確認結果(Delay 2 の 4 行の記述の正誤)、変えたパス、起動前の合計の大きさの変化。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: 015(型)。
- 020(抽出の表の拡張)と同じ表(`ci/extract-dlls.tsv`、`lift/harness/b2d2/modules.tsv`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
