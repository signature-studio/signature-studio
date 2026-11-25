# persist-files: FL のユーザーのファイルの保存と、Recent の保存先(#1385)

この文書は、`tools/persist/measure.mjs` の files の段と、その道具の試し(`--exit-before-reload`・`--save-after-open`)の実行の記録です。値は runner の値(この機械とは別枠。nproc 4、loadavg 1.4〜2.6)。根拠の無い点は「未確認」と書きます。

## 6 回目の試行(段階 1 の受け入れ: 全段が通った。runner の値)

- 判定: **受理**(`--save-after-open` の受け入れの 4 段が、この run で全部 0 で終わった)。`npm run typecheck`・`guard/scan.sh`・`guard/flags.sh`・`spec/guard.test.sh`(67 ok)・`spec/workflow.test.sh`(460 ok)も通った。runner の値(nproc 4、loadavg 1.9〜2.9。この機械とは別枠)。
- 作業の最初: `.prior-work.md` の衝突(`app/scripts/lib/plugin-rows.mjs`・`spec/plugin-rows.test.mjs`)を解消した。main(`4d757064`)が、鍵の名前の host の先を既に入れていたので、HEAD の版を採った(取り込み側は同じ意味で、コメントと変数の違いだけ)。`node spec/plugin-rows.test.mjs` は全部通った。
- ビルド: `npm run build`・`cargo build`・`fe-build engine`・`fe-build bundle`(約 50 分)。起動確認(`boot.json`)は `state` engine-idle、`error` null、`standIn` false。

| 項目 | 値 | 判定 |
|---|---|---|
| 言語の段(選んだ言語 de) | `registryKept` true、`languageKept` true、`restartPrompted` true、`defaultReloadState` engine-idle | OK |
| 起動の時間 `elapsedMs` | 11,448(上限 12,500) | OK |
| 起動の前の量 `preBootBytes` | 311,146,277(上限 314,368,711。前回の 5 回目は 311,127,136。言語が違う run で、差の原因は未確認) | OK |
| 言語の再読込の停止 | `engineStoppedAfterReload` true、`stopReason` compare-string(JSON のみ。判定に入れない。#1395 の範囲) | 判定に入れない |
| ファイルの段の `error` | null | OK |
| `savedFiles` | 2(`Downloads\persist-check-6582-0.flp` 123,823 B、`Projects\Backup\…flp` 81,072 B。どちらも 0 より大きい) | OK |
| `bytes`(保存の領域) | 340,833(上限 104,857,600) | OK |
| `logFilesBefore` / `logFilesAfter` | 0 / 0 | OK |
| `journalParsed` | true(`journalBytes` 340,833 = 全体の大きさ) | OK |
| `recentKept` / `recentWhere` / `recentFiles` | true / unknown / []。判定に入れない(段階 2・#1409) | 判定外 |
| 流れ A〜D | 4 回とも engine-idle・error null・trapped false・hostErrors 0・opened true・welcomeClosed true(10,949〜11,550 ms) | OK |
| `npm test --prefix app` | 71 files passed (1 skipped)、1,410 tests passed (10 skipped) | OK |

- 終了の要求の時間(`--save-after-open --exit-before-reload`。runner の値): 要求から engine の停止まで **253 ms**(state `exited`)。要求の前の `bytesBefore` 10,211 → 停止の後の `bytesAfter` 340,007。停止までの行で、`vfs-kept` が `Downloads\…flp~` 123,823 B・Backup 81,072 B・`Downloads\…flp` 123,823 B を記録し、`vfs-gone` が `…flp~`(0 B)と元の `…flp` を消した(FL の保存の順)。`RegSetValueExW` は `LastSavedBackup` の 1 件。その後の再読込で保存の 2 件が戻った(`savedFiles` 2)。これは、生きたページでの要求の時間で、再読込(pagehide の中)の書き終わりの ms は未確認。
- 保存の対象の確認: 起動の時に `Startup 2021-01-01_000000.log`(`CREATE_ALWAYS`)と `testpermissions.tmp` は 0 バイトの版(`vfs-empty`)のため保存されない。`logFilesBefore` / `logFilesAfter` は 0 / 0。
- Recent: レジストリの書き込みは起動時の設定の 11 件だけ(`Shared\General` の `Fancy animations` など)で、Recent の値は無い。保存の記録にも Recent は無い。段階 2(#1409)のまま。
- 同意の箱の「No thanks」は、この run では押さなかった(ゲストの画面が 1836x1056。道具の測った 780x480 と違う。`files.steps` に記録)。ファイルは Ctrl+O で開けた(`opened` true)。
- 作業の判断: `visibilitychange` は使わない(前回と同じ。タブの切り替えでも FL が閉じるため)。`pagehide` だけ。依頼者の指示の「visibilitychange(hidden)と pagehide」からは外れる。判断を仰ぐ。
- ファイル管理の窓の一覧に出るか: 未確認(見ていない)。コード上は、`/fl/state` の名前の変更・削除・切り取りに確認を出す(`app/src/files/explorer.ts` の 587・603・627 行)。

## 5 回目の試行(段階 1 の受け入れ: 全段が通った)

- 判定: **受理**(`--save-after-open` の受け入れコマンドが、最後まで 0 で終わった。`ACCEPT-EXIT=0`。runner の値。nproc 4、loadavg 1.7〜2.6。この機械とは別枠)。
- 前提の直し(作業の最初): `fe-build engine` の plugin の段が、`lift/out-app/host/lift/out-plugins/effects.fruity-balance.fruity_balance_x64.dll` を探して止まった(`No such file`)。原因: #1379(`f933a299`)で `plugins.rs` は抽出先と seed の名前を **鍵(key)** から作る形にしたが、`app/scripts/lib/plugin-rows.mjs`(抽出の表)は旧い **ファイル名** のまま。抽出の木の鍵は `app/scripts/lib/*.mjs` を含むので、直すと抽出が走り直し、鍵の名前の木になる。直した: `plugin-rows.mjs` の host の先と seed の名前を鍵にし、ファイル名の一意の検査を外した(#1379 の `validate` と同じ)。`spec/plugin-rows.test.mjs` の期待を鍵の名前に直し、同じファイル名でも通る試験を足した(`node spec/plugin-rows.test.mjs` 全部通った)。
- ローカルの木(`lift/out-app/host`)は、直した表の出力と同じ名前に、バイト列を変えずに移した(sha256 `a399cbe0…` を確かめた後の `mv`)。CI では、鍵が変わるので抽出の段が木を作り直す。
- 段階 1 の測定(runner の値。`--save-after-open` の受け入れの run):

| 項目 | 値 | 判定 |
|---|---|---|
| 言語の段(選んだ言語 fr) | `registryKept` true、`defaultReloadState` engine-idle、`elapsedMs` 11,423(上限 12,500)、`preBootBytes` 311,134,812(上限 314,368,711) | OK |
| 言語の再読込の停止 | `stopReason` missing-ilsvg(`engineStoppedAfterReload` true。JSON のみ。#1395 の範囲) | 判定に入れない |
| ファイルの段 `error` | null | OK |
| `savedFiles` | 2(`Downloads\persist-check-….flp` 123,823 B、`Projects\Backup\…flp` 81,072 B) | OK(1 以上・0 より大) |
| `bytes`(保存の領域) | 340,833 B(上限 104,857,600) | OK |
| `logFilesBefore` / `logFilesAfter` | 0 / 0 | OK |
| 流れ A〜D | 4 回とも engine-idle・error null・trapped false・hostErrors 0・opened true・welcomeClosed true(11,184〜11,475 ms) | OK |
| `npm test --prefix app` | 71 files passed(1 skipped)、1,408 tests passed(10 skipped) | OK |
| `npm run typecheck` | 通った | OK |
| `spec/plugin-rows.test.mjs`・`spec/guard.test.sh`・`spec/workflow.test.sh` | 全部通った(workflow 444 件の ok) | OK |

- `recentKept` は true、`recentFiles` は []、`recentWhere` は unknown。前回までの結論どおり、名前が FL のメモリに在るだけを見るので、Recent の記録の証拠にならない。判定に入れない(段階 2・#1409)。
- 終了の要求の実測(道具の `--files-only --save-after-open --exit-before-reload`。保存の段を終了の要求の前に移した): 要求から engine の停止まで **251 ms**(state exited、exit code は run の記録に無い)。要求の後の行に、保存の記録(`.flp~` の作成と削除、Backup の記録、`HKCU\...\General\LastSavedBackup` の値、`Downloads\…flp` の記録)が出た。保存後・要求前の `bytes` は 340,019 B で、受け入れの run の「再読込の前の `bytesBefore`」10,211 B と**食い違う**。保存の記録が要求より前に書かれたのか、要求の後に書かれて行の読みが遅れたのか、この測定では決められない(未確認)。
- 書き終わる時間(step 4): 生きたページでの要求から停止まで 251 ms。再読込(pagehide の中)の時間は測れない(新しいページからは、古いページの unload の時刻が見えない)。再読込の後に保存の 2 件が戻ったので、pagehide の中で書き終わった件数は 2(受け入れの run)。
- 0 バイトは保存しない: `vfs-empty` の行。`Startup <日時>.log` は閉じない(0 B のまま)ので、件数は 0 と 0。
- 開く・保存の窓の橋(`select.ts`)は変えていない。`/fl/state` は保護されたパスのまま(ファイル管理の窓の一覧に出るかは未確認)。
- 同意の箱の「No thanks」は押せなかった(ゲストの画面が 1836x1056。道具の測った 780x480 と違う)。開く・保存は通った(`opened` は true)。

## 4 回目までの結論(受理ではない)

- 判定: **差し戻し**。受け入れの判定の `recentKept`(再読込の後、Recent に同じファイル名が在る)が false。Recent は、FL がファイルにもレジストリにも書いていない(下の「Recent の保存先と時期」)。
- 他の判定は通る: 言語の段、流れ A〜D、単体テスト、保存の領域の大きさ、起動の時間、起動の前の量、ログの件数(0 と 0)。
- ユーザーのファイルの保存は動く: FL が保存したプロジェクトと、そのバックアップは、ページを離れる時に FL を閉じると書き出され、再読込の後に戻る(`savedPaths` の 2 件)。FL は保存したファイルのハンドルを閉じないので、この終了の要求が要る。
- `recentKept` の判定の式は、Recent の記録ではなく「名前がゲストのメモリに在る」を見る。保存したプロジェクトのファイルが戻ると、名前は in-memory に出て true になる(下の「判定の式について」)。判定の式は変えていない。

## 受け入れの値(final build。runner の値)

| 項目 | 値 | 判定 |
|---|---|---|
| language.error | null | OK |
| languageKept / restartPrompted | true / true | OK |
| language.elapsedMs | 9,274 (上限 12,500) | OK |
| language.preBootBytes | 311,127,136 (上限 314,368,711) | OK |
| files.error | null | OK |
| files.recentKept | false(`recentWhere` unknown・`recentFiles` []) | **NG** |
| files.bytes | 11,037 (上限 104,857,600) | OK |
| files.logFilesBefore / logFilesAfter | 0 / 0 | OK |
| 流れ A〜D | 4/4 engine-idle・error null・trapped false・hostErrors 0・opened true・welcomeClosed true(8,970〜9,462 ms) | OK |
| `npm test --prefix app` | 71 files passed (1 skipped)、1,405 tests passed (10 skipped) | OK |
| `npm run typecheck` | 通った | OK |
| `guard/scan.sh` | 0 finding | OK |

## Recent の保存先と時期(実測)

- 保存先: ファイルにもレジストリにも書かれない。保存の記録(`savedPaths`)に Recent の記録は無く(`recentFiles` []、`recentWhere` unknown)、レジストリの値に名前は無い。
- FL は起動の時に `Settings\Browser\Recent files.scr` を `GetFileAttributesW` で探す(戻り値 2 = 無い)。`System\Config\Browser\Recent files.scr` も同じ(`pm-work/findings/missing-paths.md`)。`Settings\Browser` のフォルダは FL が起動の時に作る(`vfs-dir`)ので、フォルダが無いことは原因ではない。
- 開いた時(前回の試行の実測): 開いた後 120 秒待っても、Recent は開かれも書かれもしない。
- 閉じた時(`--exit-before-reload`、この run): 終了の要求から engine の停止まで **258 ms**(`state` exited。前の build の run では、ページのログに `exit finished code 0` が出た)。要求の後の engine の行 61 件に、ファイルへの書き込み・レジストリの書き込みは無い(`closeLines` []、`bytesBefore` 10,211 → `bytesAfter` 10,211)。推定の「終了の時に書く」は、この VM では否定された。
- 保存した時(`--save-after-open`。Ctrl+S): FL は保存したプロジェクト(`Downloads\<name>.flp` 123,823 B)とバックアップ(`Projects\Backup\...flp` 81,072 B)を書いたが、Recent は書かなかった(`recentFiles` [])。
- 書く時期と場所(Recent そのもの): **未確認**。開く・閉じる・保存のどれでも書かれなかった。Ctrl+N(プロジェクトを閉じる)の試しは、エンジンの行が 0 件で、キーが FL に届いたか判らないので結論を出せない(道具から外した)。

## 判定の式について(recentKept)

- 保存したプロジェクトを持つ run(`--save-after-open`、終了の要求あり)では `recentKept` が **true** になる。しかし `recentFiles` は [] で、名前を含む保存ファイルは無い。再読込の後のゲストのメモリに出た行は、`persist-check-<n>.flp` だけの 2 行(タイトルの「- FL Studio」を含まない)で、戻った保存プロジェクトのファイルの名前から来ている。
- つまり、`recentKept` は「名前が FL のメモリに在る」を見るため、Recent の記録が無くても通りうる。Recent を保持したと言える判定は、`recentFiles`(保存したファイルに名前が在る)の方になる。判定の式を変えるのは依頼者の判断。

## 保存の対象・ログ・読み戻し

- 0 バイトの版は保存しない(`lift/runtime/fle_vfs.c` `kept_file`): 閉じた時に中身が空のファイルは、そのパスの保存済みの記録を取り除く(`fle_kept_file_del`。削除は保存しない)。道具の行は `vfs-empty`。場所の定数は入れていない(中身の有無で決める)。
- Startup ログ(`Support\Logs\Startup\Startup <日時>.log`): FL は作って閉じた(0 B)後、同じファイルを開いたまま書き続ける。閉じないので、内容は保存されない。`logFilesBefore` / `logFilesAfter` は、ページが保存している Startup ログの件数(保存の記録の `op 5` で `Support\Logs\Startup\` の下のもの)。0 と 0 で、再読込のたびに増えない。
- 懸念(未確認): FL が終了の時にログを閉じると、ログの内容が保存され、件数は再読込のたびに増える。この run では閉じられなかった(`vfs-empty` のみ。`kept` は無い)。
- 除外の規則(パスでログを除く)は作らなかった。runtime に場所の定数を入れない制約のため。ログの扱いは依頼者の判断が要る。
- 読み戻し: `fle_reg_restore_saved`(seed の後)が `fle_vfs_restore_put` / `fle_vfs_restore_mkdir` / `fle_vfs_restore_drop` で戻す(記録はしない)。道具の行は `vfs-restored`(save run: `Downloads\...flp` 123,823 B と、`Projects\Backup\...` 81,072 B の 2 件)。

## 保存の領域

- 大きさ: 11,037 B(開くだけの run)、340,845 B(保存した run)。上限 100 MiB(`REG_STATE_LIMIT`。#1383 と共通)。
- 起動の前の量(`preBootBytes`): 311,127,136 B(前回 311,126,361 B。増えていない)。
- ファイル管理の窓の一覧に出るか: **未確認**(見ていない)。`/fl/state` は保護されたパス(`app/src/engine/regstore.ts` の注釈)。

## 終了の要求(pagehide)

- 実装: `pagehide` で Worker に `exit-request` を送り、Worker が FL の主窓(`fle_g3_main_window`: 題名が `FL Studio` を含む、作られた順で最初の task bar のボタン)に SC_CLOSE を入れる。task bar の経路(decision 0267)と同じ。
- 理由: FL は保存したファイルのハンドルを閉じない(save run の、要求を外したビルドでは `savedFiles` 0。要求を入れたビルドでは 2)。閉じた時に記録されるので、ページを離れる時に FL を閉じると、その内容が保存される。
- `visibilitychange` は使わない: タブの切り替えでも起きるので、FL が閉じてしまう。依頼者の指示の「visibilitychange(hidden) と pagehide」からは外れる。判断を仰ぐ。
- 書き終わる時間: 生きたページでの要求から engine の停止まで 258 ms。再読込(pagehide の中)での書き終わりの ms は測っていない(未確認)。再読込の後、保存した 2 件が戻った(書き終わった)。
- 1.5 秒の待ち: 実装しない。ページは unload を待てない(同期の待ちは、ページの破棄を止められない)。

## 流れ A〜D・言語の段の停止

- 流れ A〜D: 4 回とも engine-idle、error null、trapped false、hostErrors 0、opened true、welcomeClosed true。
- 言語の段: 選んだ言語(es)の再読込で engine が `missing-ilsvg` で止まり、2 回目の起動で回復(`recoveredIdle` true)。#1395 の範囲(英語以外の再読込の停止。#1383 の判定から外してある)。

## 試し(道具の動作の確認)

- 前回の Alt+F4 の試しは、Chrome がタブを閉じて確かめられなかった。この run では Alt+F4 を使わず、`window.__app.exitRequest()`(Worker の `exit-request` と同じ経路)で要求した。
- `--save-after-open`(Ctrl+S)と `--exit-before-reload`(時間を測る)は、道具の試し。受け入れの run では使わない。

## 未確認

- Recent の書き込みの時期と場所(FL が Recent を書く条件)。開く・閉じる・保存では書かれなかった。
- 再読込の pagehide の中で書き終わる時間(ms)。
- FL が終了の時にログを閉じた時の内容の扱い(この run では閉じられなかった)。
- 保存の領域がファイル管理の窓の一覧に出るか。
- 保存の確認の窓(「保存されていません」)が出る時の終了の要求(この run では出なかった)。
