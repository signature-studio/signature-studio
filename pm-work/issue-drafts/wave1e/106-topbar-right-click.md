---
draft: "106"
title: "トップバー(Tempo・Song Position・Pattern Selector)の右クリックに FL が反応しない原因を特定し、メニューを出す"
type: Task
milestone: "M3 トップバーのコンテキストメニュー(P2)"
parent: "009"
blocked_by: ["010"]
labels: ["topbar-menu"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指示(課題 C-6): トップバーのコンテキストメニューが機能していない。トップバー(Tempo・Song Position・Pattern Selector)の右クリックの後に、FL のメニューが画面に出ること。原因を特定し、直す。

## 背景

- #{{010}} の測定(`tools/feat-c6-popup-menus/measure.mjs`。runner の値。この機械とは別枠)で、次が分かった。
  - ミキサーの Insert 1 の右クリックは、FL 自身の文脈メニューを画面に出す(陽性の対照。`menuBandChanged` が true)。`WM_RBUTTONDOWN` の時点で、`CreateWindowExW`(WS_POPUP)→ `SetWindowPos` → `ShowWindow` が呼ばれる。`WM_CONTEXTMENU` は出ない。
  - トップバーの 3 部品の右クリックでは、`WM_RBUTTONDOWN`・`WM_RBUTTONUP`・`WM_CONTEXTMENU` は出るが、窓の作成も `TrackPopupMenu` も出ない。Tempo のヒント「[Trial] Tempo」だけが出る。
  - `WM_CONTEXTMENU` の宛先は、押された子の窓ではなく、その親の窓になっている(Tempo は、押下が 0x7f990124、宛先が 0x7f9900cc)。`lift/runtime/fle_input.c` 983 行の `target = v.parent ? v.parent : hwnd` による。
- 推定(未確認): トップバーの部品(子の窓)の手続きに、`WM_CONTEXTMENU` が届いていない。ミキサーが動くのは、`WM_RBUTTONDOWN` の時点で窓を作るためかもしれない。
- 前提(`TrackPopupMenu` が出す)は成り立たない。FL は、ミキサーの文脈メニューを自前の窓で出す。`TrackPopupMenu` は、全体で 1 度も呼ばれていない。

## 読む物

- `lift/runtime/fle_input.c`(133 行: 入力の入口。538 行: `translate_mouse`。973〜994 行: 右ボタンの離しと `WM_CONTEXTMENU` の送信)
- `lift/runtime/fle_win.c`(904 行〜: `CreateWindowExW`。`ShowWindow`・`SetWindowPos`・`MoveWindow` の入口。窓の表)、`lift/runtime/fle_win.h`
- `lift/runtime/fle_crash1_menu.c`(602 行〜: `fle_imp_TrackPopupMenu`)
- `tools/feat-c6-popup-menus/measure.mjs`(#{{010}} が作る測定の道具。JSON の形と、4 部品の名前)
- `app/src/input/pointer.ts`(68・104 行)
- `pm-work/findings/survey-c6-topbar-menus.md`

## やること

1. **ミキサーとトップバーを、同じ道具で比べる。** `tools/feat-c6-popup-menus/measure.mjs` の記録(`inputLines`・`windowApiCalls`・`menuApiCalls`)を読み、ミキサーの `mixer-insert1` と、トップバーの `tempo` で、次の 4 点を比べる。足りない記録は、#{{010}} の記録に足してよい。
   - 子の窓への届き方: `WM_RBUTTONDOWN` と `WM_CONTEXTMENU` が、どの窓の手続きに届くか(宛先の親と、wParam の子)。
   - カーソルの下の窓の判定(`WindowFromPoint`・`GetCursorPos`)。
   - マウスのキャプチャ(`SetCapture`・`TrackMouseEvent`)。
   - `WM_SETCURSOR` と `WM_NCHITTEST` の結果。
2. **原因を特定し、`lift/runtime` を直す。** 直すのは、FL 本体の窓の手続き(Delphi の VCL)の挙動を、Windows の仕様に合わせること(エミュレーションの欠陥を直す)。製品の既定の動作を、測定のためだけに変えない。原因の説明を、実測と推定を分けて報告に書く。
3. **直した後の測定で、3 部品のうち 1 つ以上に、右クリックの後に `CreateWindowExW`(または `menu-api:`)の記録が出て、メニューが画面に出ることを確かめる。** 窓が作られても画面に出ない時は、その表示の経路を直す。
4. 項目の選択(`WM_COMMAND`)は、この Issue では確かめない。#{{011}} が扱う。
5. 実際の呼び出しに無い表示を、偽のメニューとして描かない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c6-popup-menus/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c6.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const top=["tempo","song-position","pattern-selector"];const m=r.parts.find((p)=>p.name==="mixer-insert1");const ok=r.state==="engine-idle"&&r.standIn===false&&r.error===null&&m!=null&&m.menuBandChanged===true&&r.parts.some((p)=>top.includes(p.name)&&p.menuBandChanged===true&&(p.menuApiCalls.length>0||p.windowApiCalls.some((l)=>/CreateWindowExW/.test(l))));process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c6.json"
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。3 部品のうち 1 つ以上で、右クリックの後に `CreateWindowExW`(または `menu-api:`)の記録があり、かつ `menuBandChanged` が true。ミキサーの陽性の対照(`mixer-insert1` の `menuBandChanged`)も true のままであること。項目の選択(`WM_COMMAND`・戻り値)は、判定に入れない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 報告に書く: 3 部品それぞれの結果(窓の作成の有無、`menuBandChanged`、記録の行)、ミキサーの陽性の対照の値、原因の説明(ミキサーとトップバーで何が違ったか)、変えたパス(lift/・tools/ を含む)、直した理由。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 製品の既定の動作を、測定のためだけに変えない。
- 実際の呼び出しに無い表示を、偽のメニューとして描かない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 依頼者の資格情報・設定に触れない。

## 他のタスクとの関係

- blocked_by: #{{010}}(右クリックの記録と測定の道具)。
- #{{011}}(3 部品の値の変化)は、この Issue が閉じた後に、メニューの項目を選ぶ(blocked_by)。
- #{{006}}(HiDPI)が描画の座標を変える場合、測定の座標を、#{{006}} の結果に合わせる。両方を変えるときは、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、受け入れをもう一度通す。
- #{{002}}(描画の分離)の Worker を通る経路を、描画の Worker が変える場合がある。その場合も、同じ受け入れで確かめる。
