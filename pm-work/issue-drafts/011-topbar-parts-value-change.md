---
draft: "011"
title: "Tempo・Song Position・Pattern Selector のメニューで項目を選ぶと、部品の値が変わる"
type: task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: ["010"]
labels: ["task", "topbar-menu", "priority-p2"]
size: 5
needs_owner: false
---

## 目的

台帳 L-033 の (6): Tempo・Song Position・Pattern Selector などのトップバーのコンテキストメニューが機能すること。3 つの部品それぞれで、メニューの項目を選ぶと、部品の表示する値が変わる。

## 規模

規模: 大(想定 5 時間。ビルドを約 5 回回す規模)

## 背景

- 010 で、右クリックのメニューが画面に出て、項目を選ぶと engine に届く(受理の後)。この Issue は、3 つの部品それぞれに対して、その先の動作を確かめる。
- 部品の位置は、起動後の画面からまだ読めていない(旧の測定。pm-work/findings/survey-c6-topbar-menus.md の 5-b)。部品の位置を読むために、画面の構成(確認ダイアログ、Welcome 窓、プロジェクトを開く操作)を確かめる必要がある。
- 旧の計画の受け入れ(案): 項目を選ぶと値が変わる(Tempo の値など)。

## 読む物

- `tools/feat-c6-popup-menus/measure.mjs`(010 が作る道具。部品の右クリックの手順の元)
- `lift/runtime/fle_crash1_menu.c`(600〜618 行。010 の変更の後)
- `app/src/input/pointer.ts`(68・104 行)
- `app/index.html`(23〜27 行: `#toolbar`)
- `pm-work/findings/survey-c6-topbar-menus.md`(画面の読み。部品が見えなかった記録)

## やること

1. 起動後の画面を PNG に保存し、3 つの部品の位置を読む。読めない場合は、どの操作で読めるか(確認ダイアログを閉じる、プロジェクトを開く等)を、測定の道具の中だけで試して報告に書く。
2. 3 つの部品それぞれで、右クリックを送り、メニューの項目を 1 つ選び、部品の表示する値(テンポ・小節の位置・パターン番号)の前後を読む。値は、画面の画素の比較か、engine の表示の文字で読む。読み方を報告に書く。
3. 測定の道具 `tools/topbar-parts/measure.mjs` を作る。JSON の 1 行だけを stdout に出す: `{"state","standIn","error","parts":[{"name","menuShown","valueBefore","valueAfter","changed"}, ...3]}`。
4. 項目の選び方(どの項目を選んだか)を、報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/topbar-parts/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/parts.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const names=r.parts.map((p)=>p.name).sort().join(",");const ok=r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.parts.length===3&&names==="Pattern Selector,Song Position,Tempo"&&r.parts.every((p)=>p.menuShown===true&&p.changed===true);process.exit(ok?0:1)' "$RUNNER_TEMP/parts.json"
```

判定の基準: 3 つの部品すべてで、メニューが出て(menuShown)、項目を選んだ後に値が変わった(changed)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 読めない部品がある場合は、その部品を「未確認」と書き、理由(位置を読めなかった等)を書く。
- 参考点(部品ではない場所)の右クリックの結果を、部品の結果として書かない。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: 010(メニューが出て、項目が engine に届くこと)。
- 038(完成の検証)が、この Issue の結果を使う。
