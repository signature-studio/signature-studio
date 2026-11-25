---
draft: "145"
title: "FL のユーザーのファイル(保存したプロジェクト・バックアップ・設定)を、ページの再読込をまたいで保持する(段階 1)"
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

この Issue は段階 1: 保存したプロジェクト・バックアップの持ち越しと、0 バイトの扱い。Recent の保持は段階 2(#{{168}})に分けた(FL が Recent を書く操作が、まだ分かっていないため)。

## 前回の試行で分かったこと(3 回目。run 45458518648)

- 前回: run 45458518648(判定は `failed`。差し戻し)。作業の commit は `f66c479d`(main には無い。差分は作業ツリーに載る)。詳細は `pm-work/findings/persist-files-1385.md`(前回の記録)。
- 実装したもの(`f66c479d`): 保存の対象は `C:\Users\user\` の下で FL が書いたファイルとディレクトリ。閉じた時と flush の時(内容が変わった時だけ)に、削除・移動・作成・コピーを記録する。保存の形式は既存の journal(`/fl/state/hkcu.log`。#{{142}})に、キーがパスの 3 種類の記録(5・6・7)を足した。読み戻しは `fle_reg_restore_saved` の中(seed の後)。
- 測定(runner の値): 保存の領域は再読込の後で 11,027 B(上限 100 MiB は #{{142}} と共通)。`preBootBytes` は 311,126,361 B(起動の前の量は増えていない)。`languageKept`・`restartPrompted` は true。`elapsedMs` は 11,336。流れ A〜D は 4 回とも engine-idle・error null・trapped false・hostErrors 0・opened true・welcomeClosed true(elapsedMs 11,403〜12,233)。`npm test`: 1392 件通過。型検査は通った。
- 判定で落ちたのは Recent だけ: `recentKept` が false(`recentWhere` は unknown)。
- Recent の保存先の実測: レジストリにもファイルにも無い。プロジェクトを開いた後の `RegSetValueExW` は起動時の設定の 11 件だけで、ファイル名を含む値は無い。`Settings\Browser\Recent files.scr` は、プロジェクトを開いた後も、120 秒待った後も開かれなかった(`vfs-open` の行で確認。開いたのは、プロジェクトの読み取り、起動ログ、書き込みの確認用の一時ファイルだけ)。
- 推定(未確認): FL は Recent を engine の終了の時に書く。再読込では終了の処理が走らないので残らない。Alt+F4 で終了を試したが、Chrome がタブを閉じてしまい、確かめられなかった。
- ログ: FL は起動時に `Startup <日時>.log` を、`CREATE_ALWAYS` で作って閉じ、その後は `OPEN_EXISTING` の書き込みハンドルを開いたまま書き続ける。このため保存されるのは作った時の 0 バイトの版だけになる。再読込のたびに 0 バイトの版が 1 件ずつ増えた。
- 停止の回: 試行の中で engine が `exit code 134`(SIGABRT)で終わった回があり、その回は保存を捨てた(#{{142}} の暫定)。journal が 0 バイトになり、ユーザーのファイルも失われた(#{{154}} が直るまでの暫定)。
- 開けた demo: `nucleon - Housework.flp`(`Demo projects` の下)と Sacco の demo は開けた(engine は idle のまま)。`--project` の path は `Demo projects` の下で、`Demo songs` の下ではない。道具は、起動に失敗した時は次の候補を試す。

## 前回の試行で分かったこと(4 回目。run 45499845007)

- 前回: run 45499845007(判定は `failed`。差し戻し。落ちたのは `recentKept` だけ)。作業の commit は `39181b4c`(main には無い。差分は次の試行の作業ツリーに取り込まれ、続きから行う。記録は `pm-work/findings/persist-files-1385.md`)。
- 段階 1 の判定で通った物(runner の値。nproc 4・loadavg 1.5〜2.6): 言語(`languageKept`・`restartPrompted` が true)。`elapsedMs` 9,274。`preBootBytes` 311,127,136 B。流れ A〜D は 4 回とも engine-idle・error null・trapped false・hostErrors 0・opened true・welcomeClosed true(8,970〜9,462 ms)。`npm test --prefix app` は 1,405 件通過。`guard/scan.sh` は 0 件。
- 0 バイトの扱い(受理): 閉じた時に中身が 0 バイトのパスは、保存の記録から取り除く(`fle_vfs.c` の `kept_file`)。Startup ログと `testpermissions.tmp` は保存されず、`logFilesBefore` と `logFilesAfter` は 0 と 0。
- 保存の持ち越し(受理): ページを離れる時(`pagehide`)に FL へ閉じる要求を出すと、保存したプロジェクト(`Downloads\<名前>.flp` 123,823 B)とバックアップ(`Projects\Backup\…flp` 81,072 B)が保存の記録に載り、再読込の後に戻る(`savedFiles` 2。保存の領域 340,845 B)。要求を外した同じ run では `savedFiles` が 0。
- 停止まで: 要求から engine の停止まで 258 ms(前の build では 254 ms。exit code 0)。保存が無い時の要求では、その間の書き込みは 0(`bytesBefore` 10,211 → `bytesAfter` 10,211)。
- Recent は FL が書いていない(差し戻し): `Settings\Browser\Recent files.scr` は起動の時に探され(`GetFileAttributesW` → 2。無い)、プロジェクトを開いた後も、保存(Ctrl+S)の後も作られない。レジストリに Recent の値は無い(保存の記録に、ファイル名を含む値は無い)。「終了の時に書く」は、上の 258 ms の間の書き込みが 0 なので、この VM では否定。Ctrl+N の試しは engine の行が出ず、結論が出なかった。→ 段階 2 として #{{168}} に移す。
- 判定の式の欠陥: `recentKept` は、名前が FL のゲストのメモリに在るかを見る。保存したプロジェクトが戻ると、名前がメモリに出るため、Recent の記録が無くても true になる(保存の run で `recentKept` は true だが、`recentFiles` は [])。判定から外した。JSON には出し続ける(`recentKept`・`recentWhere`・`recentFiles`)。
- 作業者の判断(PM が採る): `visibilitychange` は使わず `pagehide` だけにした(タブの切り替えで FL が閉じるのを避ける)。1.5 秒の待ちは入れていない(ページは unload を待てない)。pagehide の中で保存が書き終わる時間は未確認。
- 言語の段: 言語は無作為に 1 つ選ぶ(7 言語)。es を選んだ回は、再読込の後に engine が `missing-ilsvg` で止まり、2 回目で回復した(#{{154}} の範囲)。この止まりは判定に入れない(PM の決定。JSON の `engineStoppedAfterReload`・`stopReason` に出す)。
- ログ: Startup ログの件数は、FL がそのログを閉じるかどうかで決まる。この run では閉じず、件数は 0 のまま。FL が終了の時にログを閉じると、内容が保存されて件数が増える(未確認)。
- 段階 1 の範囲: 「やること」の 1・3・5・6・7 は前回の差分で入った(受け入れの判定で確かめる)。2 は #{{168}} に移した。4 は未確認のまま。受け入れの判定は、下のとおり変えた(`--save-after-open` を足す。`savedFiles`・0 バイトの判定を足し、`recentKept` を外す)。

## 背景

- 前提: #{{142}} が入った後に行う。保存の仕組み(ページの OPFS へ書く方式と、起動の前の読み戻し)は #{{142}} の物を使う。
- 仮想ボリュームはメモリの上だけにある。`lift/runtime/fle_vfs.h` の先頭の注釈どおり、ホストのファイルシステムには触れず、木は空の `C:\` から始まり、ゲストが作った物だけが在る。ページを再読込すると、ユーザーのフォルダの中身は消える。
- ユーザーのフォルダの場所(仮想。`lift/runtime/fle_env.h`・`fle_env.c`・`fle_filedlg.c`): `C:\Users\user`(`FLE_VIRTUAL_USER_PROFILE_DIR`)。Documents は `C:\Users\user\Documents`。`CSIDL_APPDATA` は `C:\Users\user\AppData\Roaming`。
- 記録に出ている FL のユーザーのフォルダ(`pm-work/findings/missing-paths.md`): `C:\Users\user\Documents\Image-Line\FL Studio\` の下(`Settings\`・`Support\Logs\Startup\`・`Settings\Themes\`・`Settings\Languages\`)と、`C:\Users\user\Documents\Image-Line\Downloads\`。共有の既定の場所は `C:\Users\user\Documents\Image-Line\`(seed `lift/harness/seeds/reg-seed-app.tsv` の `Shared data`)。
- 保存の経路が既にあるもの: 開く・保存の窓の橋(`app/src/files/select.ts` の 62〜65 行の注釈。選んだファイルは仮想ボリュームの `OPEN_FOLDER\<OPFS のパス>` へコピーされ、保存の先は engine が書いた後で OPFS へ戻される。decision 0193 の `saved-file` の橋)。ただし、FL が自分で書く設定・一覧のファイルは、この橋を通らない。
- Recent の一覧の保存先: 前回の試行で、レジストリにもファイル(`Settings\Browser\Recent files.scr`)にも書かれないと分かった(上の節)。FL が書くのは engine の終了の時と推定する(未確認)。
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

0. 作業ツリーの確認。`.prior-work.md` があれば最初に読む。前回の差分(`f66c479d`・`pm-work/findings/persist-files-1385.md`)が在るかを確かめる。#{{142}} の保存の経路が main に在ることを確かめる。
1. **0 バイトの版を保存しない**: 閉じた時に中身が 0 バイトのファイルは保存しない。場所(パス)の定数は runtime に入れず、中身の有無で決める。実際に中身が書かれたファイル(Recent・設定・プリセット・プロジェクト)は保存する。
2. **Recent の書き込みの時期と場所を実測する**: FL が Recent をいつ・どこに書くかを、`tools/persist/measure.mjs` の files の段で確かめる。推定は「engine の終了の時」。書く時期が、終了の時でなく、プロジェクトを開いた時なら、その時期に合わせる。
3. **終了の要求(推定が正しい時)**: ページが `visibilitychange`(`hidden`)と `pagehide` で、engine に「終了の要求」を出す。engine は FL の主窓に `WM_CLOSE`・`WM_QUERYENDSESSION`・`WM_ENDSESSION` を、Windows のセッションの終了と同じ順で送る。FL が終了の処理(Recent・設定の書き込み)を行い、その書き込みが journal に flush されるのを、最大 1.5 秒待つ。保存されていないプロジェクトの確認(「Save changes?」)の窓が出る時は、待たずに、そのまま閉じる(窓は FL の画面に出るが、ページは閉じる)。
4. **書き終わるかの実測**: headless の測定で、`page.close({ runBeforeUnload: true })` と、再読込の前の `visibilitychange` の発火で、この方式が `pagehide` の短い時間で書き終わるかを確かめる。書き終わる時間(ms)と、書かれた件数を報告に書く。書き終わらない時は、その事実を報告に書く(推測で待ち時間を延ばさない)。
5. **停止の回の保存**: engine が異常に終わった回は、保存を全て捨てる。この扱いは変えない(#{{154}} が直るまでの暫定)。
6. **ログの数**: 測定の道具に、再読込の前後の `Support\Logs` の下の Startup ログの件数(`logFilesBefore`・`logFilesAfter`)を出す。再読込のたびに 0 バイトの版が増えないことを、受け入れで確かめる。
7. **上限と起動**: 保存の領域は合計 100 MiB 以下(#{{142}} と同じ)。起動の前に取る量(`preBootBytes`)を増やさない。
8. **受け入れ**: 下の「受け入れ」のコマンドを、そのまま通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/persist/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --save-after-open > "$RUNNER_TEMP/persist.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const lang=L.find((r)=>"language" in r);const f=L.find((r)=>r.step==="files");process.exit(lang&&f&&lang.defaultReloadState==="engine-idle"&&lang.registryKept===true&&f.error===null&&f.savedFiles>=1&&f.savedPaths.every((p)=>p.bytes>0)&&f.bytes<=104857600&&Number.isInteger(f.logFilesBefore)&&f.logFilesBefore===f.logFilesAfter&&lang.elapsedMs<=12500&&lang.preBootBytes<=314368711?0:1)' "$RUNNER_TEMP/persist.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

`--save-after-open` は前回の差分(run 45499845007 の `tools/persist/measure.mjs`)に在る道具の機能で、次の試行は `.prior-work.md` でその差分を作業ツリーに載せてから行う。

判定の基準:
- ファイルの段(`step` が `files` の行): `error` が null。`savedFiles` が 1 以上(`--save-after-open` の run で、保存したプロジェクトとバックアップが、再読込の後の保存の記録に在る)。`savedPaths` の全ての `bytes` が 0 より大きい(0 バイトは保存しない)。`bytes` が 104,857,600 以下。
- `recentKept`・`recentWhere`・`recentFiles` は JSON に出し続けるが、判定には使わない(上の 4 回目の節。Recent は #{{168}} が扱う)。
- ログの件数: 同じ行の `logFilesBefore` と `logFilesAfter`(`Support\Logs` の下の Startup ログの件数。再読込の前と後)が、同じ整数。0 バイトの版が再読込のたびに増えないことを確かめる。
- 言語の段(`step` が `files` ではない行): 英語(既定)の再読込(`defaultReloadState`)が `engine-idle` に届き、選んだ言語の値が再読込の後の起動で読まれている(`registryKept` が true)。言語は無作為に 1 つ選ぶ。非英語の再読込の後に engine が止まったかは、`engineStoppedAfterReload`・`stopReason` として JSON に出すだけで、判定に入れない(#{{154}} の範囲)。`restartPrompted` も判定に入れない(JSON には出し続ける)。
- 起動の段(言語の行): `elapsedMs` が 12,500 以下。`preBootBytes` が 314,368,711 以下。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)が通る。単体テスト(`npm test --prefix app`)が通る。

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
- 段階 2(#{{168}}: Recent の保持)は、この Issue の後に行う。Recent の保持は、そちらで扱う。
- #{{141}}(テーマの切り替えの停止)は、`Settings\Themes\` の書き込みで重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #{{004}} の openProject の手順は、道具の中の関数を読むだけ(この Issue は、その道具を書き換えない)。
