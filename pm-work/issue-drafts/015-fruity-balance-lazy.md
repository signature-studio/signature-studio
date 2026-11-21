---
draft: "015"
title: "Fruity Balance を起動後の取得に移し、1 プラグイン 1 wasm で届ける(最初の 1 件)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["014"]
labels: ["delivery", "plugin-builtin"]
size: M
priority: High
project: signature-studio/3
---

## 目的

試行: 内蔵のプラグイン 1 件(Fruity Balance)を、起動後にバックグラウンドで届け、1 プラグイン 1 wasm の形を、最初の 1 件で確かめる。内蔵の 141 件の全件を入れる前の、型の確認。

## 規模

規模: 大(想定 4 時間。ビルドを約 5 回回す規模)

## 背景

- Fruity Balance は、内蔵のエフェクトの 1 件。DLL の大きさは 3,718,656 B(`lift/out-plugins/fruity_balance_x64.dll` の実測。pm-work/findings/survey-c4-builtin-plugins.md)。Fruity Filter(3,780,608 B)と Delay 2(3,731,968 B)より小さく、最初の 1 件に選んだ。
- 登録の場所: `lift/harness/b2d2/modules.tsv` の 8 行(`fruity_balance_x64.dll`・base `0x6800000`・`--hook-points lift/harness/hooks/fruity-delphi-effect-hook-points.txt`)。`lift/harness/web/pack.args` の `--args` の行(13〜14 行)。seed 表の 245 行(`Plugins\Fruity\Effects\Fruity Balance\...`)。`ci/extract-dlls.tsv` の 15 行(SHA-256 つき)。
- hook 点: `lift/harness/hooks/fruity-delphi-effect-hook-points.txt` の 12〜14 行(RVA 3 つ。Balance と Filter で同じ値)。
- 起動時の取得: 今は DLL pack(11 ファイル)に入り、起動の前に取られる(子 014 の結果による)。この Issue では、Fruity Balance を起動前の取得から外し、起動後の取得で届ける。
- 未確認: engine が起動時に Fruity Balance を読むかどうか(起動引数に名前がある)。この Issue で、`lift/runtime` の読みで確かめる。読み込まれないなら、起動の引数から外して、起動の記録で確かめる。

## 読む物

- `lift/harness/b2d2/modules.tsv`(8 行)、`lift/harness/web/pack.args`(13〜14 行)、`lift/harness/hooks/fruity-delphi-effect-hook-points.txt`(12〜14 行)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(245 行)
- `lift/runtime/fle_modules.c`(仮想モジュールの表)、`lift/runtime/fle_load.c`(LoadLibrary の解決)、`lift/runtime/fle_realmod.h`(1〜3 行)
- `app/src/bundle/loader.ts`、`app/src/engine/fruity.ts`(120〜140 行)
- 子 014 の出力(区分の作り)
- `tools/delivery-split/measure.mjs`(子 014 が作る道具)

## やること

1. Fruity Balance が、起動前の取得と起動後の取得のどちらに入っているかを、変更の前に確かめる(報告に書く)。
2. Fruity Balance を起動後の取得に移す。1 プラグイン 1 wasm の形で、起動後に届くようにする。届いた後に、engine がどう読み込むか(インスタンス化の場所、Worker の場所)を、報告に書く。
3. engine が起動時に Fruity Balance を要求する場合は、要求を満たす形(空の代替、遅延の読み込みの待ち、等)を決め、理由を報告に書く。
4. 道具の出力を確かめる。Fruity Balance が起動前の取得に無く、起動後の取得に在り、取得の開始が起動前の取得の完了以後であること。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/delivery-split/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-lazy.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const sum=(a)=>a.reduce((s,f)=>s+f.bytes,0);const fb=(a)=>a.some((f)=>/fruity.?balance/i.test(f.name));const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&!fb(r.preStart)&&fb(r.lazy)&&sum(r.lazy)>0&&r.fetches.some((f)=>/fruity.?balance/i.test(f.name)&&f.startMs>=r.preStartDoneMs));process.exit(ok?0:1)' "$RUNNER_TEMP/feat-lazy.jsonl"
```

判定の基準: 3 回の起動で、Fruity Balance が起動前の取得に無く、起動後の取得に在り、取得の開始が起動前の取得の完了以後であること。取得の開始が engine-idle より前にある場合は、その事実を報告に書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: engine が Fruity Balance を起動時に要求するかの読みと結果、変えたパス(lift/・app/ を含む)、起動前と起動後の合計の大きさ(変更の前後)、elapsedMs の 3 回の値。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者が言っていない制限(例: 他のプラグインの扱い)を、この作業の判断で増やさない。

## 他のタスクとの関係

- blocked_by: 014(区分の仕組み)。
- 019(Fruity Filter と Delay 2)、027〜029 が、この Issue の型を使う。
- 017(言語・テーマ・テンプレート)と同じ manifest を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
