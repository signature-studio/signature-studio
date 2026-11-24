---
draft: "141"
title: "FL のテーマを切り替えると engine が止まる(crash)を直す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-24。原文): 「テーマを切り替えるとクラッシュします。」

この Issue の目標は 2 つ。
1. 停止を再現し、原因を特定して直すこと。停止の理由は、推測ではなく、engine のログ(`fle:` の行・`== stop:` の行・戻り番地)で確かめる。
2. 配信されている全てのテーマ(インストーラの `Artwork\Themes\` の `.flstheme` の全件)に切り替えても止まらず、画面が切り替わること。

## 背景

- 症状は実機の利用で起きた。実機の停止の記録は、理由が取れていない: `pm-work/findings/real-user-exits-2026-11-24.md`(停止 `stopped / 134` の 1 件。理由の項目は無い)。理由を記録に残す項目は #{{136}} が足す。この Issue は、停止を手元で再現してから直す。
- テーマの配信: インストーラの一覧 `inputs/fl-installer-files.tsv` の `Artwork\Themes\` の下は 39 件(ファイルのみ。一覧にフォルダの行は無い)。そのうち `.flstheme` は 13 件(直下 8 件と、フォルダ `DEDZII\`・`P.Priime\`・`Veela\` の下の 5 件)。#{{105}} で、これらが起動の前に取る物として seed 表 `lift/harness/seeds/vfs-seed-idle1.tsv` に入った(`resources/Artwork/Themes/...` の行)。
- 起動の記録(`pm-work/findings/missing-paths.md`): 起動の間に FL が要求して、仮想ボリュームに無かったテーマの物は次の 3 件。`Artwork\Themes\__current.flstheme`(`GetFileAttributesW`、ERROR_PATH_NOT_FOUND 3)、`C:\Users\user\Documents\Image-Line\FL Studio\Settings\Themes\__current.flstheme`(ERROR_FILE_NOT_FOUND 2)、そのフォルダ `…\Settings\Themes`(2)。推定(未確認): テーマを選ぶと FL は選んだテーマを `__current.flstheme` として書き、フォルダ `Settings\Themes` を作る。書き込みと作成の経路で止まる可能性がある。
- 仮想ボリュームの作成・書き込み(`lift/runtime/fle_vfs.c`): `CreateFileW`・`CreateDirectoryW`・`WriteFile`。この file は、停止の条件が多い(例: 1 ファイル 256 MiB を超える・パスの成分が 255 字を超える・予約名・ハンドルの領域の使い切り。`fle_stop` の呼び出しが 139 箇所)。どれが該当するかは、再現で分かる。
- 画像の読み込み: テーマは `.jpg`・`.png` の画像を持つ(`Artwork\Themes\DEDZII\` など)。画像を読む経路は未確認(`lift/runtime/fle_loadimage.c`、`lift/runtime/fle_load.c` を読む)。
- 設定窓(F10)の Theme 頁: 頁の一覧に在る(`pm-work/findings/hidpi-fl-scaling.md` の 10.1)。テーマを切り替える操作は、この頁で行うと推定する(未確認。再現の手順で確かめる)。
- 原因は、まだ分かっていない。この節の推定は、再現の前の材料であり、結論ではない。

## 読む物

- `lift/runtime/fle_vfs.c`・`lift/runtime/fle_vfs.h`(`CreateFileW`・`CreateDirectoryW`・`GetFileAttributesW`・`fle_stop` の条件)
- `lift/runtime/fle_loadimage.c`・`lift/runtime/fle_load.c`(画像と DLL の読み込み)
- `lift/out-app/gen/wasm-func-names.tsv`(`fe-build engine` が作る。停止の戻り番地を関数名に引く)
- `tools/smoke/welcome-flow.mjs`(起動と、Welcome 窓を閉じる手順。流れ D の「I'm in」)
- `tools/feat-c1-hidpi/measure-size.mjs` の `settingsPhase`(F10 で設定窓を開き、頁を押す手順。#{{117}})
- `app/src/engine/fruity.ts`(`fleIdle`)、`app/src/engine/worker.ts`(`desk-peek`・`guest-text`)、`app/src/main.ts`(`window.__app`)
- `pm-work/findings/missing-paths.md`(起動の要求の表。テーマの行)、`pm-work/findings/real-user-exits-2026-11-24.md`(停止の記録)

## やること

0. 作業ツリーの確認。`.prior-work.md` があれば最初に読む。
1. 再現の道具 `tools/theme-switch/measure.mjs`(新規)を作る。`tools/smoke/welcome-flow.mjs` の起動と、#{{117}} の `settingsPhase` の F10 の手順を再利用する。
   - 起動し、Welcome 窓と確認ダイアログを閉じる(流れ D の「I'm in」か、流れ A の「No thanks」と Escape。どちらでも、窓の表から位置を取る)。
   - F10 で設定窓を開き、Theme 頁を押す(頁の位置は窓の表の帯から取る。固定の座標を使わない)。
   - 配信の 13 件の `.flstheme` を、1 件ずつ選ぶ。選んだら 2 秒待たずに描画を続け、2 秒の間の提示の回数(`window.__app.deskStats.presents` の増え)と、画面(canvas の画素)を読む。
   - 停止したら、停止の全文(engine のログの末尾 200 行。`fle:` の行・`== stop:` の行・戻り番地)と、停止した時のテーマの名前と順番を、JSON と `pm-work/findings/theme-switch-crash.md` に書く。
2. 原因の特定と修正。停止の理由から、未実装の API・欠けたファイルやフォルダ・メモリの不足・画像の読みのどれかを特定し、既定の動作として直す(オプトインを作らない)。戻り番地は `lift/out-app/gen/wasm-func-names.tsv` で関数名に引き、どの guest の処理が切り替えの時に呼んだかを書く。推定と実測を分けて報告に書く。
3. 全てのテーマ(13 件)に切り替えても止まらないことを、道具で確かめる。切り替えの前後で、画面が変わったこと(画素が異なる)と、主窓の背景色が変わったこと(テーマごとに色が異なる)を、道具の JSON に出す。
4. 報告に、停止の全文、特定した原因の commit、直した方式、13 件の切り替えの表(テーマ名・提示の前後の数・画面の変化)を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/theme-switch/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/theme.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const n=fs.readFileSync("inputs/fl-installer-files.tsv","utf8").replace(/\r/g,"").split("\n").filter((l)=>/^Artwork\\Themes\\.*\.flstheme\t/i.test(l)).length;const ok=(r)=>r.error===null&&r.stopped===false&&r.themes===n&&r.switched===n&&r.items.length===n&&r.elapsedMs<=12500&&r.preBootBytes<=314368711&&r.items.every((x)=>x.presentsAfter>x.presentsBefore&&x.screenChanged===true);process.exit(n===13&&L.length===2&&L.every(ok)?0:1)' "$RUNNER_TEMP/theme.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 道具の出力は 2 行(DPR 1 の 1 行と、乱数の DPR 1.01〜3.00 の 1 行)。各行で、`error` が null、`stopped` が false、`themes`(配信の `.flstheme` の件数。一覧から数えた 13)と `switched` が一致する。
- 各テーマ(`items`)で、`presentsAfter` が `presentsBefore` より大きい、かつ `screenChanged` が true(切り替えの前後で、画素が異なる)。
- `elapsedMs` が 12,500 以下。`preBootBytes` が 314,368,711 以下(起動の前に取る量を増やさない)。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)が通る(#{{116}} の判定のまま)。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 再現の停止の全文(戻り番地と関数名を含む)、原因の特定(どの API・どのファイルか)、直した方式と理由、13 件の切り替えの結果、流れ A〜D の結果。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- テーマの名前・個数・色の値を、runtime・app・seed・テストに、定数・表・分岐として入れない(配信の一覧から数える)。
- 測定の道具(`tools/theme-switch/`)だけが、切り替えの操作を行う。製品の既定の動作は、利用者の操作だけで変える。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して待つ。背景で起動して待たない。
- 1 回の試行で、ビルドは数回まで。再現・記録・実験の計測は、まとめて 1 回のビルドで行う。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- 配信の一覧は #{{105}} で入った。この Issue は、配信された物を切り替えた時の停止だけを扱う。
- #{{136}}(停止の理由を記録に残す)は、実機の停止の理由を取る。この Issue は、手元の再現の理由で直す。
- #{{145}}(ユーザーのフォルダの保存)と、`Settings\Themes\` の書き込みが重なる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
