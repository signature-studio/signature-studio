---
draft: "020"
title: "内蔵のエフェクトとインストゥルメントの DLL を、抽出の表に揃える(一覧と SHA-256 が一致)"
type: task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["013", "023"]
labels: ["task", "plugin-builtin", "inventory", "priority-p1"]
size: 6
needs_owner: false
---

## 目的

台帳 L-034: 内蔵のエフェクト(Effects 約 90 件)とインストゥルメント(Generators 約 51 件)を存在させる。その前段として、抽出の表(DLL の一覧と SHA-256)に、インストーラの一覧(013)の内蔵の DLL を、SHA-256 つきで揃える。OpenGL の VFX 系(023)は除く。実際の lift は、群ごとの Sub-Issue(021・022)が行う。

## 規模

規模: 大(想定 6 時間。ビルドを約 5 回回す規模)

## 背景

- 現在の抽出の表: `ci/extract-dlls.tsv` の有効な行は 11 件(FLEngine 1 件とエンジンのライブラリ 7 件と Fruity 3 件。pm-work/findings/survey-c4-builtin-plugins.md の手順 1・4)。インストーラの内蔵の DLL は、ここには無い。
- Generators の DLL は、今の抽出の表に 0 件。Effects の DLL の追加(Fruity Filter・Delay 2)は、子 019 で扱う。
- 表の 1 行の形: `ci/extract-dlls.tsv` の各行は、インストーラ内の DLL のパス、抽出先、SHA-256 などを持つ(現在の表の形に従う。形は作業者が読んで確かめる)。
- 1 DLL を足す手数(推定。survey-c4 の手順 8): 表の行、`lift/harness/b2d2/modules.tsv` の行、seed の行、`lift/harness/web/pack.args` の `--args`。hook 点のファイルが必要な Delphi の DLL もある。
- 一覧の入力: `inputs/fl-installer-files.tsv`(013 の成果物。列: guest・bytes・sha256・kind・imports)。
- 後回しの VFX 系の一覧: `inputs/vfx-deferred.tsv`(023 の成果物)。

## 読む物

- `ci/extract-dlls.tsv`(全行。17・18 行の注釈)
- `inputs/fl-installer-files.tsv`(013 の成果物)
- `inputs/vfx-deferred.tsv`(023 の成果物)
- `app/scripts/extract-fl.mjs`(52〜68 行: DLL の表。SHA-256 が一致した物を抽出先に置く)
- `lift/harness/b2d2/modules.tsv`、`lift/harness/web/pack.args`、`lift/harness/seeds/vfs-seed-idle1.tsv`(表の 4 つの箇所)
- `lift/harness/hooks/fruity-delphi-effect-hook-points.txt`(Delphi の DLL の hook 点の形)

## やること

1. 一覧(013)から、`Plugins\Fruity\Effects\`・`Plugins\Fruity\Generators\` の DLL のうち、後回し(023)を除いた物を取り出す。件数を報告に書く。
2. 取り出した各 DLL について、`ci/extract-dlls.tsv` の行を、一覧の SHA-256 と一致させて加える。各行は、抽出の仕組み(extract-fl.mjs の DLL 表)で検査できる形にする。
3. Generators の DLL は、seed の表に無いので、そのままでは起動の経路に入らない。この Issue では表だけを揃え、lift と起動の経路は群の Sub-Issue(022)に任せる。報告に、どの表が未完かを書く。
4. 追加の手数(表の行数・触った箇所の数)を、報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node -e 'const fs=require("fs");const rows=fs.readFileSync("ci/extract-dlls.tsv","utf8").split("\n").filter((l)=>l.trim()!==""&&!l.startsWith("#"));const inv=fs.readFileSync("inputs/fl-installer-files.tsv","utf8").trim().split("\n").slice(1).map((l)=>l.split("\t"));const def=new Set(fs.readFileSync("inputs/vfx-deferred.tsv","utf8").trim().split("\n").slice(1).map((l)=>l.split("\t")[0].toLowerCase()));const want=inv.filter((r)=>/^plugins\\fruity\\(effects|generators)\\/i.test(r[0])&&r[3]==="dll"&&!def.has(r[0].toLowerCase()));const ok=want.every((r)=>rows.some((l)=>l.toLowerCase().includes(r[2].toLowerCase())))&&rows.length>=11+want.length;process.exit(ok?0:1)'
```

判定の基準: 一覧の内蔵の DLL(後回しを除く)の SHA-256 が、表の行に全て在り、表の有効な行が 11 件に、その件数を足した数以上あること。判定の道具は、`ci/extract-dlls.tsv` の SHA-256 の列を一覧と照合する(列の位置は作業者が確かめて報告に書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 一覧の件数(Effects・Generators・後回しを除いた件数)、表の追加行数、表の未完の箇所、1 DLL 当たりの手数の実測。インストール先の既定の値を書かない。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。内蔵の一覧を、オプションで切り替えない。
- 作業者に依頼者の個人の資格を置かない(L-011)。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。インストーラの既定の会社名のフォルダを、表に書かない。
- 抽出の段(extract の仕組み)の鍵を変えると、全件の作業が抽出し直しになる。鍵を変える場合は、報告に書く。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: 013(一覧)、023(後回しの一覧)。
- 019(Fruity Filter と Delay 2)と同じ表を変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直す。
- 021・022 の群の Sub-Issue は、この Issue の表を前提にする。
