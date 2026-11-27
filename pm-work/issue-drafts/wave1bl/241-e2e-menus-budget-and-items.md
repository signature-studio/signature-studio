---
draft: "241"
title: "E2E の menus: 予算で 246 本が飛ばされ、項目の判定が 73 本落ちる(shard の割り方・項目の指定・閉じない箱)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify", "topbar-menu"]
size: L
priority: High
project: signature-studio/3
---

## 目的

`menus`(417 本)は、E2E の新しい領域。全シナリオの E2E(`e2e-full` の最新の run。commit 24aeee99 の版)で、次の 2 つの理由で job が落ちた。(1) 行の予算(35 分)に、1 shard の 134〜135 本が入らず、246 本が `skipped`(予算切れ)になった。(2) 残りの 73 本の印の無い失敗のうち、項目を選んでも効果が出ない・箱が閉じないもの。この Issue は、(1) の shard の割り方を直して、全本が予算の中で走るようにし、(2) の項目の指定と判定を確かめて直す。製品(`app/`・`lift/`)は、原因が製品と分かるまで変えない。

## 背景

- 予算(実測): `.github/workflows/e2e-full.yml` は menus を 3 shard に割り、`--budget 35m` で走らせる。1 shard の行は 134 本・134 本・135 本。menus 1/3 は 34.5 分の時点で 50 本目まで走り、残り 84 本が skip。menus 2/3 は skip 75 本、menus 3/3 は skip 87 本。
- 項目の判定 73 本(実測。notes の文言で分類):
  - 項目を選んでも効果が出ない 38 本: `windows: none; submenus open: 0; changed: none; title: same`(例: `menus/add-3-choose`・`menus/add-5-4-choose`・`menus/file-7-again`)。続きの `no window opened: no dialog to answer` が 23 本(例: `menus/add-5-1-cancel`・`menus/file-7-esc`)。
  - 箱が閉じない 8 本: `dialog TSpeechForm[Speech properties] is still there after ok`(`menus/add-1-ok`・`menus/add-1-cancel`・`menus/add-1-esc`)、`TInAppShopForm[Buy FL Studio]`(`menus/file-5-ok`・`menus/file-4-ok`・`menus/file-6-ok`)、`TMissingPluginsForm[Problems loading the project]`(`menus/file-1-esc`・`menus/file-2-5-esc`)。
  - ドロップダウンが開かない 2 本: `menus/file-8-submenu-escape`・`menus/file-9-10-submenu-escape`。同じ仕組みで、`file` 領域の 5 本も `drop-downs shown 0` か `new windows []`(`file/export-submenu`・`file/menu-new-from-template-submenu`・`file/exit-not-chosen`・`file/export-all-playlist-tracks`・`file/export-master`)。
  - その他 1 本: `menus/file-2-2-again`。停止 1 本(`menus/edit-3-choose`)は、停止として #{{243}} で扱う。
- 道具の状態(実測): 項目は `Down x N` で指定し、名前は `index`(位置)で付けている。項目の名前は、選ぶ前に確かめていない。
- 推定: 効果が出ない項目の一部は、指定の位置がずれている(区切り線・無効な項目)。残りは、製品の未対応か、確認できない。
- 閉じた Issue の印(実測): `menus/add-14-choose`(#1411)・`menus/file-9-11-choose`(#1415)・`menus/view-11-1-choose`(#1416)。3 本とも、印の番号は閉じている。

## 読む物

- `.github/workflows/e2e-full.yml`(menus の shard の数と、`--budget 35m`)
- `tools/e2e/run.mjs`・`tools/e2e/lib/plan.mjs`(予算・skip の扱い)
- `tools/e2e/lib/menus.mjs`(`chooseItem`・`withNote`・項目の指定)・`tools/e2e/lib/common.mjs`(`openMenu`・`menuBarRect`・`POPUP`・`newWindows`)
- `tools/e2e/scenarios/menus/_generate.mjs`(生成器。`STOP` の表が印の番号を持つ)と、生成された `menus/*.mjs`(手で編集しない。生成し直す)
- `tools/e2e/features/generated/menus.tsv`・`tools/e2e/features/menus.tsv`(項目の表)
- `tools/e2e/scenarios/file/export-submenu.mjs` など、file 領域の `drop-downs` の 5 本

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 予算: `.github/workflows/e2e-full.yml` の menus の shard の数を、1 shard が 35 分に収まる数に増やす(目安は 8 shard。実測の所要に合わせて決め、決めた理由を報告に書く)。menus の全本が、予算の中で走る(skip が 0)ことを確かめる。
2. 項目の指定: 項目を選ぶ前に、項目の名前を、メニューの窓の表(`TQuickPopupMenuWindow` の項目)かエンジンのメモリから読み、指定と一致することを確かめる。一致しなければ、効果の判定より前に、指定の誤りとして失敗させる。
3. 効果の判定: 2 の後で、効果が出ない項目を数える。製品の未対応か、確認できないかを、項目ごとに報告に書く。未対応の項目は、印の番号を付けない(`new` のまま、一覧を報告に書く)。
4. 箱が閉じない: OK・Cancel・Esc で閉じない窓(`TSpeechForm`・`TInAppShopForm`・`TMissingPluginsForm`)の判定を、答えの後の窓の表で確かめる。道具の座標の誤りなら直す。製品の欠陥なら、報告に書く。
5. 閉じた Issue の印(#1411・#1415・#1416)を、現在の状態に合わせる。印を外すか、開いている Issue の番号に変える。生成器の `STOP` の表も直し、`node tools/e2e/scenarios/menus/_generate.mjs` で生成し直す。
6. 受け入れを通す。通らない時は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && ! grep -rqE "'#(1411|1415|1416)'" tools/e2e/scenarios/menus && node tools/e2e/run.mjs --area menus --shard 1/8 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/m18.jsonl" && node tools/e2e/run.mjs --area menus --shard 8/8 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/m88.jsonl" && node -e 'let s=0,n=0,e=0;for(const p of process.argv.slice(1)){for(const r of require("fs").readFileSync(p,"utf8").trim().split("\n").map((l)=>JSON.parse(l))){if(r.error!==null)e++;for(const x of r.scenarios){n++;if(x.skipped===true)s++}}}process.exit(e===0&&s===0&&n>0?0:1)' "$RUNNER_TEMP/m18.jsonl" "$RUNNER_TEMP/m88.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる(shard 1/8 と 8/8 で、skip が 0、error が無い)。
- `.github/workflows/e2e-full.yml` の menus の shard の数が、予算の中に全本が入る数になっている(報告に、1 shard の所要の実測を書く)。
- 閉じた Issue の印(#1411・#1415・#1416)が、menus の生成された本文に無い。
- 報告に: (a) shard の数の決めた理由と、実測の所要、(b) 項目の指定の確かめの結果(一致した件数・一致しなかった件数と、その ID)、(c) 効果の出ない項目の一覧(製品の未対応か、確認できないかの区別つき)、(d) 箱が閉じない 8 本の原因、(e) 閉じた印の扱い、(f) 2 つの shard の出力の行。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜6 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。`.github/`・`tools/e2e/` は直してよい。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- ビルドや E2E の長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- ビルドは 1 回にする。tools だけを直した後は、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせて直す。
- 1 回で終わらない前提で、段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。`git add` は 1 つずつ、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- #1507(windows・settings の位置)・#1508(plugins)・#1509(topbar・page・resilience・perf)とは、領域が違うので、ファイルは重ならない。
- 製品の停止(`menus/edit-3-choose` を含む 4 系統)は、#{{243}} で扱う。この Issue では、停止を報告に書くだけにする。
