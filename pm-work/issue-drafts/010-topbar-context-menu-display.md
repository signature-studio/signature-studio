---
draft: "010"
title: "右クリックでメニューを画面に出し、項目の選択を engine に返す"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: []
labels: ["topbar-menu"]
size: M
priority: High
project: signature-studio/3
---

## 目的

課題 C-6: トップバーのコンテキストメニューを機能させる。右クリックの後に、メニューが画面に出て、項目を選ぶと engine に届くこと。

## 背景

- 旧の測定(pm-work/findings/survey-c6-topbar-menus.md。runner の値。変更の前の版): 右クリックは、ページの `contextmenu` を止めた上で engine に届いた(入力欄に `button R up` が出て、`defaultPrevented` が true)。engine のログは増えなかった(322 → 322 行)。メニューらしい物は見えなかった。
- 原因(読みによる推定。実測で確かめる): `lift/runtime/fle_crash1_menu.c` の `fle_imp_TrackPopupMenu`(600〜618 行)は、「何も表示せず、選ばれず、窓へ何も送らない」(注釈 600〜601 行、`fle_crash1_menu.h` 5 行)。戻り値は、`TPM_RETURNCMD`(0x100)なら 0、それ以外なら 1(617 行)。
- 関係する経路: `app/src/input/pointer.ts` 68 行(`onContext` が既定の動作を止める)と 104 行(`contextmenu` の登録)。`lift/runtime/fle_input.c` 975〜992 行(右ボタンを離した時の `WM_CONTEXTMENU` の送信)。
- 起動直後の画面には、「使用状況の送信への同意」の確認ダイアログが出る(旧の測定。「No thanks」で閉じられる。測定の道具の中だけで「No thanks」で閉じる。製品の既定の動作は変えない)。

## 読む物

- `lift/runtime/fle_crash1_menu.c`(600〜618 行: `fle_imp_TrackPopupMenu`)
- `lift/runtime/fle_crash1_menu.h`(先頭の注釈。5 行)
- `lift/runtime/fle_input.c`(133 行: 入力の入口。538 行: `translate_mouse`。973〜994 行: `WM_CONTEXTMENU` の送信)
- `lift/runtime/fle_input.h`(97 行付近: `FLE_WM_RBUTTONUP`)
- `lift/runtime/fle_win.c`(窓の表。`fle_win_get` の使い方)
- `lift/runtime/fle_desk.h`(窓を描く仕組みの注釈)
- `app/src/input/pointer.ts`(68・104 行)
- `app/src/engine/worker.ts`(`pushLog`。engine のログの経路)、`app/src/main.ts`(engineLog の公開)
- `app/index.html`(23〜27 行: `#toolbar`)
- `tools/boot-check/boot-bundle.mjs`、`tools/boot-check/cdp.mjs`
- `pm-work/findings/survey-c6-topbar-menus.md`(測定の手順と結果)

## やること

1. 変更の前に、起動後の画面を PNG に保存し、部品の位置を読めるかを確かめる(報告に画像の読みを書く)。
2. 実装: `TrackPopupMenu` で、メニューの項目(メニューの表)を、既存の窓の描き方で画面に描く。項目を選んだら WM_COMMAND(0x0111。wParam の下位 16 ビットが項目の ID。hwnd へ送る)を送る。`TPM_RETURNCMD`(0x100)のときは WM_COMMAND を送らず、選んだ ID を戻り値にする(選ばなければ 0)。
3. 表示した時と項目を選んだ時に、engine のログに、WM_COMMAND か TrackPopupMenu の語を含む行を出す(既存の pushLog の経路)。
4. 測定の道具 `tools/feat-c6-popup-menus/measure.mjs` を作る。boot-bundle と同じ起動で、engine-idle の後、(a) 測定の道具の中だけで確認ダイアログを「No thanks」で閉じる、(b) 右クリックの前の画面を保存、(c) 窓の内側の 1 点に右ボタンの press と release を送る、(d) 1 秒待って画面を保存、(e) クリック位置の近く(半径 200 px の正方形)で、前後の差のある画素数を数える、(f) 出たメニューの項目を 1 つクリックする、(g) engine のログの増分を取る。JSON の 1 行だけを stdout に出す: `{"state","standIn","error","changedNearClick","changedTotal","logAdded":[...]}`。
5. 3 つの部品(Tempo・Song Position・Pattern Selector)の位置を、起動後の画面から読む。読めた物は右クリックで試す。読めない物は「未確認」と書く(受け入れには入れない。#{{011}} で扱う)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c6-popup-menus/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c6.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const ok=r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.changedNearClick>0&&r.logAdded.some((l)=>/WM_COMMAND|TrackPopupMenu/.test(l));process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c6.json"
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。右クリックの近くに画面の変化があり、項目を選んだ時の engine のログに WM_COMMAND か TrackPopupMenu の語がある。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 変えたパス(lift/・app/ を含む)、メニューの描き方の選択と理由、項目を選んだ時の戻り値と WM_COMMAND の ID、3 つの部品の位置が読めたか(読めた物は、位置と右クリックの結果を書く)。
- 確認ダイアログを「No thanks」で閉じたのは測定の道具の中だけであることを書く。製品の既定の動作は変えていない。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない(起動時の確認ダイアログを含む)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者の資格情報・設定に触れない。

## 他のタスクとの関係

- #{{011}}(3 部品の値の変化)が、この Issue を待つ(blocked_by)。
- #{{006}}(HiDPI)が描画の座標を変える場合、この Issue の測定の座標を、#{{006}} の結果に合わせる。両方を変えるときは、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、受け入れをもう一度通す。
- #{{002}}(描画の分離)の Worker を通る経路を、描画の Worker が変える場合がある。その場合も、同じ受け入れで確かめる。
