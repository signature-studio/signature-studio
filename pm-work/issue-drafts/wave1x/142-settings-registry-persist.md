---
draft: "142"
title: "FL の設定(言語など。レジストリの値)を、ページの再読込をまたいで保持する"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-24。原文): 「言語を切り替えると再起動を求められますが(これ自体は正常な挙動)、再起動しても設定が反映されていません。ページを再読込すると Recent が消えるなど、一部のデータは保持されていないように見えます。」

この Issue の目標は、FL が設定として書いた**レジストリの値**(言語など)を、ページの再読込(engine の作り直し)をまたいで保持すること。ユーザーのファイル(Recent の一覧・Presets・Projects など)は #{{145}} が扱う。

## 背景

- 症状の要点: 言語を変えると、FL は再起動を求める(正常)。再起動(ページの再読込)の後、言語が元に戻っている。
- 仮想レジストリはメモリの上だけにある。`lift/runtime/fle_reg.c` の先頭の注釈どおり、ホストのレジストリは読まず、五つの根の木はメモリにある。ページを再読込すると engine は作り直され、木は seed の値に戻る。`RegSetValueExW`(`fle_reg.c`)の書き込みは、engine の終了で消える。
- 製品の初期値の経路: レジストリの初期値は seed パックから入る(`lift/crates/fe-build/tests/plan/bundle.txt` の `app/scripts/pack-seed.mjs --extra lift/harness/seeds/reg-seed-nsis.tsv --extra lift/harness/seeds/reg-seed-app.tsv`)。`reg-seed-app.tsv` の値は `HKCU\Software\Image-Line\Shared\Paths` の `Shared data`(`C:\Users\user\Documents\Image-Line\`)。`reg-seed-nsis.tsv` は、インストーラの書く値。
- 仮想のユーザー: `C:\Users\user`(`lift/runtime/fle_env.h` の `FLE_VIRTUAL_USER_PROFILE_DIR`)。`SHGetFolderPathW` と `SHGetKnownFolderPath` は、これを基にフォルダを答える(`lift/runtime/fle_env.c`・`lift/runtime/fle_filedlg.c`)。
- ページ側の永続の仕組み: 製品に、engine の書き込みを保存する経路は無い。OPFS は、ファイル管理の窓(`app/src/files/opfs.ts`。`OpfsFs.open()` は OPFS のルートを開く)と、資産の保存(`app/src/bundle/assetstore.ts`。`/fl/cache/assets/`)にだけ使われる。
- 言語の選択を書く場所は、まだ分かっていない(未確認)。起動の要求の記録(`pm-work/findings/missing-paths.md`)では、FL は `C:\Users\user\Documents\Image-Line\FL Studio\Settings\Languages\en.moe` と `System\Languages\en.moe` を要求し、どちらも無かった。インストーラの言語の一覧(`inputs/fl-installer-files.tsv` の `System\Languages\`)は、de・es・fr・ja・ko・vi・zh の `.moe` と `.svg`、en は `.svg` のみ。言語の選択がレジストリに書かれるか、ファイルに書かれるかは、この Issue の再現で確かめる。
- 推測しない。上の「未確認」の項目は、再現と測定で埋める。

## 読む物

- `lift/runtime/fle_reg.c`(仮想レジストリ。`RegCreateKeyExW`・`RegSetValueExW`・`RegDeleteValueW`・`RegQueryValueExW` と、木の作り)、`lift/runtime/fle_reg.h`
- `lift/harness/reg_seed.h`(初期値の書き方の注釈)、`lift/harness/seeds/reg-seed-app.tsv`・`reg-seed-nsis.tsv`
- `app/scripts/pack-seed.mjs`(`--extra` の扱い)、`lift/crates/fe-build/tests/plan/bundle.txt`
- `app/src/engine/worker.ts`(起動の経路と、要求の `case`)、`app/src/engine/client.ts`、`app/src/engine/fruity.ts`(起動の前の読み)
- `app/src/files/opfs.ts`・`app/src/bundle/assetstore.ts`(OPFS の使い方の前例)
- `tools/smoke/welcome-flow.mjs`、`tools/feat-c1-hidpi/measure-size.mjs` の `settingsPhase`(F10 で設定窓を開き、General の頁を押す手順。#{{117}})、`guestText` の読み(`app/src/engine/worker.ts`。#{{117}} の方式)
- `pm-work/findings/missing-paths.md`(言語の要求の行)

## やること

0. 作業ツリーの確認。`.prior-work.md` があれば最初に読む。
1. 再現の道具 `tools/persist/measure.mjs`(新規)を作る。`tools/smoke/welcome-flow.mjs` の起動と、#{{117}} の F10 の手順を再利用する。
   - 新しいプロファイルで起動し、engine が idle になるまで待つ。
   - 英語以外の言語を 1 つ選ぶ(配信の言語 de・es・fr・ja・ko・vi・zh から乱数で。一覧から数える)。F10 の General 頁の Language の項目で、その言語を選ぶ。再起動の案内が出ることを確かめる(出ないなら `restartPrompted` が false)。
   - ページを再読込する(`Page.reload`)。engine が idle に戻るまで待つ。
   - 再読込の後に、F10 の General 頁の Language の項目を読み、選んだ言語が反映されているかを確かめる(`guestText` の文字列の読みか、エンジンの API の記録の読み。どちらを使うかは作業者が決め、報告に書く)。
   - 保存の領域の大きさ(バイト)を数える。
   - 1 行の JSON(`language`・`restartPrompted`・`languageKept`・`bytes`・`error`・`elapsedMs`・`preBootBytes`)を stdout に出す。
2. 設計: 次の要点で直す(実測で選ぶ。既定の動作として入れる)。
   - 対象: 仮想レジストリの `HKCU` の木の全て(値の名前・型・データ)。
   - 保存先: ページの OPFS(オリジン固有。ページを閉じても残る)の専用フォルダ(例: `/fl/state/`)。ファイル管理の窓が OPFS のルートを一覧にするため、この領域を一覧に見せるかを決め、理由を書く。
   - 保存の時期: 値を書いた都度(`RegSetValueExW`・`RegCreateKeyExW`・`RegDeleteValueW` の後)に、同じ Worker から書く。ページを閉じる前に書き終わることを、報告に確かめた方式で示す(同期のハンドル `createSyncAccessHandle` が使えるかは、作業者が実測で確かめる)。
   - 読み戻し: 起動の前に、保存された木を仮想レジストリへ入れる。seed の値の後に適用し、保存された値が seed の値を上書きする。
   - プライバシー: 保存は、利用者のブラウザの中だけ。外へ送らない。ビーコン(`app/src/perf/`)に値を入れない。
   - 上限: 保存の領域は合計 100 MB 以下。
3. 直す。言語の選択を保存する場所が、この Issue の時点で分かったら、その場所を保存の対象に入れる。分からなければ、`HKCU` の全てを対象にして、言語の値が含まれることを道具で確かめる。
4. 報告に、言語の選択の保存先(実測)、再読込の前後の言語の値、保存の領域の大きさ、直した方式と理由を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/persist/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/persist.json" && node -e 'const r=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l)).find((x)=>"language" in x);process.exit(!!r&&r.error===null&&r.restartPrompted===true&&r.languageKept===true&&r.bytes<=104857600&&r.elapsedMs<=12500&&r.preBootBytes<=314368711?0:1)' "$RUNNER_TEMP/persist.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- `restartPrompted` が true(言語を変えると、再起動の案内が出る。正常な挙動)。
- `languageKept` が true(再読込の後の言語が、選んだ言語)。
- `bytes` が 104,857,600(100 MB)以下。`error` が null。
- `elapsedMs` が 12,500 以下。`preBootBytes` が 314,368,711 以下(起動の前に取る量を増やさない。保存された値の読み戻しは、起動の前に取る物を増やさない)。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)が通る。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 言語の選択の保存先(レジストリ・ファイルのどちらか。実測の根拠)、再読込の前後の言語、保存の領域の大きさ、保存の時期と方式、直した方式と理由、流れ A〜D の結果。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。保存は既定の動作にする(オプトインを作らない)。
- 言語の名前や値を、runtime・app・seed・テストに、定数・表・分岐として入れない。保存は、言語に依らない仕組みにする。
- 保存先の書き込みは、ゲストの書き込みを写すだけ。ゲストに、保存の仕組みを見せない(ゲストの API の答えを変えない。読み戻した値だけが違う)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して待つ。背景で起動して待たない。
- 1 回の試行で、ビルドは数回まで。言語の保存先の確認・道具・直しは、まとめて 1 回のビルドで行う。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- #{{145}}(ユーザーのファイルの保持)は、この Issue の後に行う(保存の仕組みを共有する)。
- #{{141}}(テーマの切り替えの停止)と、仮想レジストリの書き込みが重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #{{117}}(F10 の設定窓の読み)の道具を使う。#{{117}} の判定の基準は変えない。
