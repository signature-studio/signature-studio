---
draft: "145"
title: "FL のユーザーのファイル(Recent・Presets・Projects など)を、ページの再読込をまたいで保持する"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["142"]
labels: ["resources", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

依頼者の指摘(2026-11-24。原文): 「ページを再読込すると Recent が消えるなど、一部のデータは保持されていないように見えます。」

この Issue の目標は、FL が**ファイル**として書くユーザーのデータ(Recent の一覧が、ファイルに書かれる場合を含む。Presets・Projects・設定のファイル)を、ページの再読込をまたいで保持すること。レジストリの値(言語など)は #{{142}} が扱う。

## 背景

- 前提: #{{142}} が入った後に行う。保存の仕組み(ページの OPFS へ書く方式と、起動の前の読み戻し)は #{{142}} の物を使う。
- 仮想ボリュームはメモリの上だけにある。`lift/runtime/fle_vfs.h` の先頭の注釈どおり、ホストのファイルシステムには触れず、木は空の `C:\` から始まり、ゲストが作った物だけが在る。ページを再読込すると、ユーザーのフォルダの中身は消える。
- ユーザーのフォルダの場所(仮想。`lift/runtime/fle_env.h`・`fle_env.c`・`fle_filedlg.c`): `C:\Users\user`(`FLE_VIRTUAL_USER_PROFILE_DIR`)。Documents は `C:\Users\user\Documents`。`CSIDL_APPDATA` は `C:\Users\user\AppData\Roaming`。
- 記録に出ている FL のユーザーのフォルダ(`pm-work/findings/missing-paths.md`): `C:\Users\user\Documents\Image-Line\FL Studio\` の下(`Settings\`・`Support\Logs\Startup\`・`Settings\Themes\`・`Settings\Languages\`)と、`C:\Users\user\Documents\Image-Line\Downloads\`。共有の既定の場所は `C:\Users\user\Documents\Image-Line\`(seed `lift/harness/seeds/reg-seed-app.tsv` の `Shared data`)。
- 保存の経路が既にあるもの: 開く・保存の窓の橋(`app/src/files/select.ts` の 62〜65 行の注釈。選んだファイルは仮想ボリュームの `OPEN_FOLDER\<OPFS のパス>` へコピーされ、保存の先は engine が書いた後で OPFS へ戻される。decision 0193 の `saved-file` の橋)。ただし、FL が自分で書く設定・一覧のファイルは、この橋を通らない。
- Recent の一覧の保存先は、まだ分からない(未確認)。レジストリに書かれるなら #{{142}} の保存で足りる。ファイルに書かれるなら、この Issue が保存の対象に入れる。どちらかは、この Issue の再現で確かめ、報告に書く。
- 起動の記録で、FL は起動のたびに `Support\Logs\Startup\Startup <日時>.log` を作る(`pm-work/findings/hidpi-fl-scaling.md` の 10.2)。ログの大きさは、保存の上限の見積もりに入れる。
- 推測しない。上の「未確認」の項目は、再現と測定で埋める。

## 読む物

- `lift/runtime/fle_vfs.c`・`lift/runtime/fle_vfs.h`(仮想ボリュームの作成・書き込み・列挙。`CreateFileW`・`WriteFile`・`FindFirstFileW`)
- `lift/harness/vfs_seed.h`(起動の前に、表の物を仮想ボリュームへ入れる経路。`vs_run`)、`lift/harness/seeds/vfs-seed-idle1.tsv`
- `app/src/files/select.ts`(開く・保存の橋)、`app/src/files/ops.ts`・`app/src/files/opfs.ts`
- #{{142}} の保存の経路(`app/src/engine/worker.ts` と、保存の領域の名前)
- `tools/p3-audio-probe/measure.mjs` の `openProject`(File > Open で OPFS のファイルを開く手順。#{{004}})
- `tools/persist/measure.mjs`(#{{142}} が作る)

## やること

0. 作業ツリーの確認。`.prior-work.md` があれば最初に読む。#{{142}} の保存の経路が main に在ることを確かめる。
1. Recent の保存先を確かめる。`tools/persist/measure.mjs` に、ファイルの段を足す。
   - 新しいプロファイルで起動し、`tools/p3-audio-probe/measure.mjs` の `openProject` の手順で、OPFS のプロジェクトのファイル(`.flp`)を File > Open で開く。
   - Recent の一覧に、そのファイル名が出るかを確かめる(窓の文字は `guestText` の読み。#{{117}} の方式)。
   - ページを再読込し、engine が idle に戻るまで待つ。Recent の一覧に、同じファイル名が残るかを確かめる(`recentKept`)。
   - 残らなければ、消えたファイルの名前と場所(仮想ボリュームの木の、`Documents\Image-Line\FL Studio\` の下の物を、列挙で数える)を報告に書く。
   - 道具の出力は、#{{142}} の行(`language` の行)の後に、もう 1 行: `{"step":"files","recentKept":…,"recentWhere":"registry|file|unknown","bytes":…,"error":null}`。
2. Recent がレジストリに在ると分かったら、この Issue は道具の確認だけにし、#{{142}} の判定で保持を確かめる(報告に書く)。ファイルに在る場合は次の 3 を行う。
3. 保存の対象を決める(既定の動作として入れる。オプトインを作らない)。
   - 対象: ユーザーのフォルダの下の、FL が書いた物(`C:\Users\user\` の下)。ただし、`Support\Logs\` の下のログは、大きさを測って除くかを決め、理由を書く。
   - 保存の時期と方式は #{{142}} と同じ(書いた都度。ページの OPFS へ。同じ Worker から)。
   - 起動の前に、保存された物を仮想ボリュームへ入れる(`lift/harness/vfs_seed.h` の経路を使うか、同等の経路を作るかは作業者が決め、報告に書く)。起動の前に取る量(`preBootBytes`)を増やさない。
4. 上限: 保存の領域は合計 100 MB 以下(#{{142}} と同じ)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/persist/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/persist.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const lang=L.find((r)=>"language" in r);const f=L.find((r)=>r.step==="files");process.exit(lang&&f&&lang.error===null&&lang.languageKept===true&&lang.restartPrompted===true&&f.error===null&&f.recentKept===true&&f.bytes<=104857600&&lang.elapsedMs<=12500&&lang.preBootBytes<=314368711?0:1)' "$RUNNER_TEMP/persist.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- ファイルの段(`step` が `files` の行): `error` が null、`recentKept` が true(再読込の後も、Recent の一覧に同じファイル名が在る)、`bytes` が 104,857,600 以下。
- 言語の段(#{{142}} の判定のまま): `languageKept` が true、`restartPrompted` が true。
- `elapsedMs` が 12,500 以下。`preBootBytes` が 314,368,711 以下。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)が通る。単体テスト(`npm test --prefix app`)が通る。
- Recent がレジストリに在ると分かった場合は、ファイルの段の `recentKept` は、#{{142}} の保存で true になることを、同じ道具で確かめる(判定の式は変えない)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: Recent の保存先(レジストリかファイルか。根拠の出力)、保存の対象とログの扱い(大きさと理由)、起動の前の読み戻しの経路、保存の領域の大きさ、流れ A〜D の結果。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。保存は既定の動作にする(オプトインを作らない)。
- ファイルの名前・場所の値を、runtime・app・seed・テストに、定数・表・分岐として入れない(FL が書いた物の木を、そのまま写す)。
- 開く・保存の窓の橋(`app/src/files/select.ts`)の動作を変えない。
- 保存の領域(OPFS)が、ファイル管理の窓の一覧に出る場合は、報告に書く(見せるかどうかは、#{{142}} の決めた方式に従う)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して待つ。背景で起動して待たない。
- 1 回の試行で、ビルドは数回まで。Recent の保存先の確認・道具・直しは、まとめて 1 回のビルドで行う。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- #{{142}}(レジストリの保持)の後に行う。保存の経路と道具(`tools/persist/measure.mjs`)を共有する。
- #{{141}}(テーマの切り替えの停止)は、`Settings\Themes\` の書き込みで重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #{{004}} の openProject の手順は、道具の中の関数を読むだけ(この Issue は、その道具を書き換えない)。
