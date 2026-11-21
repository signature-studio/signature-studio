---
draft: "017"
title: "言語・テーマ・プロジェクトテンプレートを、インストーラの全件と揃える"
type: task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["013", "016"]
labels: ["task", "resources", "priority-p1"]
size: 5
---

## 目的

課題 C-3: 言語パック・テーマ・プロジェクトテンプレートなど、FL Studio 自体が必要とする物を揃える。今の抽出の木は、seed の表と同じ範囲(238 件)しか持たない。インストーラの全件の一覧(013)と、見つからなかったファイルの記録(016)を使って、足りない物を入れる。

## 規模

規模: 大(想定 5 時間。ビルドを約 5 回回す規模)

## 背景

- 旧の測定(pm-work/findings/survey-c3-fl-resources.md。runner の値): 抽出の木(238 件)と seed の表(ファイル 227 件)は、件数も名前も一致した(missing 0)。分類: 言語 `System\Languages\` は 1 件(`en.svg`、899 B)。テーマ `Artwork\Skins\` は 3 件(343,555 B)。テンプレート `Data\Templates\` はディレクトリの行だけでファイルは 0 件。デモ `Data\Demo projects\` は 1 件(313,356 B)。既定のプロジェクト `Data\System\Empty.flp` は 1 件(46,361 B)。
- 「全件」は、インストーラの一覧(013)が出るまで分からない。インストーラの言語とテーマの全件の数は、未確認。
- seed の表: `lift/harness/seeds/vfs-seed-idle1.tsv`(247 行。guest のパス、抽出先、SHA-256)。seed の pack の作り: `app/scripts/pack-seed.mjs`。抽出: `app/scripts/extract-fl.mjs`(seed の行を抽出先で引き、無ければ終了コード 2 で止まる)。
- 起動前の取得に入る: seed pack は、起動前の取得(`app/src/main.ts` 714〜717 行)の中。言語・テーマ・テンプレートは、起動前に取る物の区分に入る。

## 読む物

- `lift/harness/seeds/vfs-seed-idle1.tsv`(件数と形式)、`lift/harness/seeds/vfs-seed-fonts.tsv`
- `app/scripts/extract-fl.mjs`(69〜98 行: インストール木の引き方と seed の行)
- `app/scripts/pack-seed.mjs`(1〜8 行の注釈)
- `inputs/fl-installer-files.tsv`(013 の成果物。全件の一覧)
- `pm-work/findings/missing-paths.md`(016 の成果物。見つからなかったパス)
- `pm-work/findings/survey-c3-fl-resources.md`(旧の分類の数字)

## やること

1. 013 の一覧から、言語(`System\Languages\`)・テーマ(`Artwork\Skins\`)・テンプレート(`Data\Templates\`)・プロジェクト(`Data\System\`・`Data\Demo projects\`)の件数を出す。seed の表の件数と比べ、差の一覧を報告に書く。
2. 差の物を、抽出と seed に加える。加える方法は、既存の抽出の仕組み(extract の段と seed の表)に従う。新しい経路は作らない。
3. 起動の確認で、016 の記録が空であること(見つからなかったパスが 0 件)を確かめる。
4. 増えた件数と、増えた大きさ(起動前の取得の合計)を、変更の前後で報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/missing-paths/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/missing.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const inv=fs.readFileSync("inputs/fl-installer-files.tsv","utf8").trim().split("\n").slice(1).map((l)=>l.split("\t"));const n=(p)=>inv.filter((r)=>r[0].toLowerCase().startsWith(p.toLowerCase())&&r[3]!=="dir").length;const ok=L.length===3&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.missing.length===0)&&n("System\\Languages\\")>=1&&n("Artwork\\Skins\\")>=3;process.exit(ok?0:1)' "$RUNNER_TEMP/missing.jsonl"
```

判定の基準: 3 回の起動で `missing` が 0 件。言語とテーマの件数が、013 の一覧で 1 以上・3 以上(一覧にある分)。一覧と seed の件数の差は、報告に一覧で書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 一覧の件数の内訳(言語・テーマ・テンプレート・プロジェクト)、seed の件数との差、増えた大きさ、見つからなかったパスの件数、変えたパス(app/・lift/ を含む)。
- 一覧が無い、または件数が合わない場合は、「未確認」と書く。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 作業者に依頼者の個人の資格を置かない。インストーラの取得は、R2 の鍵を持つ段だけ。
- ベンダー名と Cloudflare の識別子を書かない。インストール先の既定の値を、表にも報告にも書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: 013(一覧)、016(記録)。
- 015(Fruity Balance)と同じ manifest(起動前の取得の区分)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
