---
draft: "162"
title: "FL の画面の操作を網羅的に自動で踏み、停止(abort・ExitProcess・trap・停滞)を全て集める(crash の探索の道具と、最初の全数の走査)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["verify", "audio"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(2026-11-25): 「まだ非常にクラッシュしやすい状態が続いています。積極的に是正してください。」

個別に報告された crash(#1382 テーマ切替・#1386 コード進行ツール・#1387 リフマシーン・#1365 評価用プロジェクト・#1384 General のスクロール)は、依頼者が踏んで報告するまで分からなかった。依頼者の報告を待たず、FL の画面の操作を網羅的に自動で踏み、停止を全て集める。

この Issue の目標は次の 3 つ。
1. FL の画面の操作を自動で踏む道具 `tools/crash-hunt/run.sh` を作る。停止(abort・`ExitProcess`・trap・停滞・ページの例外)を、操作の経路と一緒に記録し、停止の後も走査を続けられる。
2. 最初の全数の走査を、予算 50 分で 1 回行い、停止を指紋ごとにまとめた表を出す。停止が見つかったことは失敗ではない。見つかった停止は、PM が種類ごとに Issue にする。
3. 道具の頑健さを確かめる(停止の後の起動し直しが全て成功する・同じ経路を二重に踏まない・メニューの項目数が 2 回の走査で一致する・CDP の timeout を数える)。

## 背景

- 既知の停止の種類: `HostFilesUnsupported`(ホストのファイルの解放の後の読み。#1357)・`ExitProcess`(exit 0 で止まる。#1372)・`LockResource`(#1365)・wasm の trap・abort(`fle: <api>: <why>` の行の後に止まる)・停滞(操作の後に engine が idle に戻らない)。
- 停止の理由は、engine のログの `fle: ` の行と `== stop:` の行で読む。ビーコンの `engine.exit.stop`(#1380)と同じ事実を、道具は窓の表とログから取る。
- 窓の表は `window.__app.deskDumpBase64()`、ログは `window.__app.engineLog()`(`app/src/main.ts`)。
- 依頼者の実機の DPR は約 1.76。窓の大きさで操作の座標が変わるため、DPR を 2 通り(1 と、乱数の 1.01〜2.24)で走らせる。
- 既存の部品(操作の仕方の参考):
  - `tools/smoke/welcome-flow.mjs`: Welcome 窓を閉じる。窓の表から位置を取る。
  - `tools/feat-c6-popup-menus/measure.mjs`: 右クリックのメニューを開き、Escape で閉じる。
  - `tools/p3-audio-probe/measure.mjs`: `openProject`。
  - `tools/piano-roll-tools/measure.mjs`・`tools/clipboard/measure.mjs`・`tools/cursor/measure.mjs`・`tools/persist/measure.mjs`。
- 参照: `pm-work/findings/chord-tool-crash.md`(#1386 の記録)。
- 素材: `inputs/plugin-classes.tsv`(Effects 112 件・Generators 70 件)。

## 読む物

- 上の既存の道具(操作の仕方と、engine の状態の読み方)
- `app/src/main.ts`(`window.__app` の `engineLog`・`deskDumpBase64`・`windowApiTrace`)と `app/src/engine/worker.ts`(`desk-dump` の要求)
- `app/src/perf/exit-message.ts` と `app/src/perf/beacon.ts`(`engine.exit.stop` の項目と、個人の値の除き方)
- `inputs/plugin-classes.tsv`
- `tools/plugin-run/` が在れば(#1371 の道具。プラグインの instantiate の確かめに使う。無ければ、メニューの操作で代える)

テーマの切替と、F10 の Settings の操作の道具(`tools/theme-switch/`・`tools/settings-scroll/`)は、まだ無い。上の既存の道具と同じ方法で操作する。

## やること

0. 起動の確認。道具の作業の前に、`tools/smoke/welcome-flow.mjs` を 1 回走らせ、流れ A の行を取る(道具は 4 流れを順に走らせるので、流れ A の行だけ使う)。`trapped` が false、`elapsedMs` が 12500 以下、`preBootBytes` が 314368711 以下であることを確かめる。この 3 つの値は、走査の JSON の `startup` にも入れる。

1. 道具 `tools/crash-hunt/run.sh`(新規。`node tools/crash-hunt/hunt.mjs` を呼ぶ薄い段でよい)を作る。引数は次のとおり。
   - `--budget <時間>`(例 `50m`)・`--out <ディレクトリ>`・`--bundle <ディレクトリ>`(既定 `lift/out-app/bundle/raw/fl-Oz`)・`--seed <整数>`(既定 20251125。乱数の DPR を決める)・`--resume`(`<out>/crash-hunt-state.json` から続ける)。
   - Chrome の起動と CDP の接続は、`tools/smoke/welcome-flow.mjs` と同じ方法にする。

2. 走査の単位は「経路」。操作の経路(メニューの文字列の階層で書く。例: `Options > Settings > General`)と DPR の組を 1 つ決め、操作し、結果を 1 件の probe にする。操作は座標でなく、メニューの文字列・窓の表・ボタンの文字で指定する。

   操作の後、2〜3 秒待ってから次を読む: engine の状態(`engine-idle` か `running` か)・`error`・`engine.exit`・`engineLog()` の新しい行(`fle: ` と `== stop:`)・窓の表の新しい窓。結果は次のいずれか。
   - `ok`: 何も起きないか、出た窓・ダイアログを閉じられた。
   - `stopped`: `engine.exit` が出た。
   - `error`: ページの例外か、`error` が出た。
   - `hung`: 20 秒経っても engine が idle に戻らない。または、CDP の `Runtime.evaluate` が 2 回続けて timeout した。
   - `skipped`: 外部へ通信する項目(Web のヘルプ・アップデート・登録・アカウント)。踏まずに、理由と一緒に書く。

   出た新しい窓・ダイアログは、Escape・Enter・OK の順で閉じる。閉じられなければ、`stopped` と同じ扱いにする。

   停止(`stopped`・`error`・`hung`)の時は、probe の記録を取ってから、ページを読み込み直して engine を新しく起動し、次の経路から続ける。同じ経路は二重に踏まない。

   プロジェクトを失う確認(New project・Close・Open の上書きなど)は、確認のダイアログを、キャンセルで閉じる。FL を終える項目(`Exit`)は、走査の最後の 1 つとして踏む(それまでの経路を止めないため)。

   CDP の `Runtime.evaluate` が timeout(20〜30 秒)したら、1 回だけ再試行し、その数を `cdpRetries` に数える(隠さない)。

3. 走査する範囲。全ての項目を、1 つずつ踏む。
   - (a) メニュー: 上部のメニュー(File・Edit・Add・Patterns・View・Options・Tools・Help)の全ての項目。サブメニューは再帰的に開く。項目の数を `menuItemsTotal` に入れる。
   - (b) キー: 窓を開くか再生を行う既定のキー(F5〜F12・Alt+F8・Ctrl+O・Ctrl+E・Ctrl+P・Ctrl+F・Ctrl+I・Space)を、1 つずつ。
   - (c) 右クリック: 主窓の上部バーの全部品(窓の表と部品の表に在る物。Tempo・Song Position・Pattern Selector を含む)・Channel Rack のチャンネル・Mixer のインサート・Playlist のトラック・Piano roll の鍵盤・Browser の項目。各メニューの全項目。
   - (d) F10 の Settings: 全てのタブ(General・Audio・MIDI・File・Debug・Info ほか)の、全ての選択欄・チェック・ボタン。Language は配信の全言語、Theme は `Artwork\Themes\` の全テーマ(13 件)に、1 つずつ切り替える。切り替えの後は、既定(English・既定のテーマ)に戻してから次へ進む。
   - (e) Piano roll の Tools メニューの全項目(コード進行・Riff machine・各スクリプト)と、Browser の全てのタブ(Sounds・Plugin database・Packs ほか)。
   - (f) 内蔵プラグイン: ミキサーの FX スロットに、`inputs/plugin-classes.tsv` の Effects の全件を 1 つずつ足す。Channel Rack の Add から、Generators の全件を 1 つずつ足す。足したら、そのスロット・チャンネルを消してから次へ進む。

   DPR 1 の全項目を先に走らせ、次に乱数の DPR(1.01〜2.24。`--seed` から決める)の全項目を走らせる。予算が尽きたら途中で止め、残りの経路を `remaining` に書く。

4. 出力。
   - `pm-work/findings/crash-hunt-<UTC の日付>.json`: `startup`(流れ A の 3 つの値と `ok`)・`dprs`(使った DPR)・`budgetMs`・`elapsedMs`・`complete`(全経路を踏み終えたか)・`menuItemsTotal`・`cdpRetries`・`restartFailures`(停止の後の起動し直しで失敗した数)・`skipped`(経路と理由)・`remaining`(残りの経路)・`probes`(経路・DPR・グループ(a〜f)・結果・`stop`(`lastFle`・`stopLine`・`abortKind`・`memoryBytes`。無ければ null)・`logTail`(最後の 20 行)・`elapsedMs`)。
   - `pm-work/findings/crash-hunt-<UTC の日付>.md`: 概要(probe の件数を DPR とグループ別に・結果の内訳・予算・完了の有無)と、指紋ごとの表(件数・結果・DPR・代表の経路 3 件)。指紋は `stop.lastFle` の先頭 80 字(数字と 16 進の番地を `#` に、パスを `<path>` に置き換えた形)。`lastFle` が無い停止は `(fle 無し)`、停滞は `(停滞)`。
   - `<out>/crash-hunt-state.json`: 完了した経路の一覧(`--resume` で続けるため)。

   ログのパスは `<path>` に置き換えてから書く。個人の値を findings に書かない。

5. 頑健さの確認。1 回目の走査の後、同じ道具・同じ `--seed` で、予算 50 分の走査を、別のディレクトリ(`.tmp-crash-hunt-2`)に 1 回行う。2 回の `menuItemsTotal` を比べる。2 回目の後は、`.tmp-crash-hunt-2` を消す。時間が足りなければ省き、報告に省いたと書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && bash tools/crash-hunt/run.sh --budget 50m --out pm-work/findings && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));const ok=Array.isArray(r.probes)&&r.probes.length>=200&&r.restartFailures===0&&r.startup&&r.startup.ok===true&&typeof r.cdpRetries==="number"&&Number.isFinite(r.menuItemsTotal);process.exit(ok?0:1)' "pm-work/findings/crash-hunt-$(date -u +%F).json" && bash guard/scan.sh "pm-work/findings/crash-hunt-$(date -u +%F).md" "pm-work/findings/crash-hunt-$(date -u +%F).json"
```

判定の基準:
- 受け入れの 1 行が通る。
- `probes` が 200 件以上(DPR 1 と乱数の DPR の合計)。
- `restartFailures` が 0(停止の後の起動し直しが、全て成功した)。
- `startup.ok` が true(流れ A: `trapped` が false・`elapsedMs` が 12500 以下・`preBootBytes` が 314368711 以下)。
- `cdpRetries` と `menuItemsTotal` の値が在る。2 回目の走査の `menuItemsTotal` が 1 回目と一致する(一致しなければ、報告に 2 回の値を書く)。
- 同じ経路・同じ DPR の組が、`probes` に 2 回現れない。
- 停止が見つかったこと自体は、失敗ではない。見つかった停止の件数と経路を報告に書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 報告に書く: `probes` の件数(DPR 別・グループ別)、`ok`・`stopped`・`error`・`hung`・`skipped` の件数、指紋ごとの件数と代表の経路 3 件(上位 10 件)、`restartFailures`・`cdpRetries`・`menuItemsTotal`(1 回目と 2 回目)、`startup` の値、`complete` と `remaining` の数。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 停止の原因は、報告で推定しない。指紋ごとの `lastFle` の先頭 80 字と件数だけを書く(原因の特定は PM が、種類ごとに Issue にして行う)。

## 制約

- 製品(`app/`・`lift/`・`ci/`)は変えない。道具は `tools/crash-hunt/` に、findings は `pm-work/findings/` に置く。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 画面の文字は OCR で読まない。FL の画面の文字は、ゲストのメモリの走査(#1358 の方式)か、エンジンのログと窓の表で読む。
- Python は使わない(調査と検証にも使わない。道具は node と bash で書く)。
- ベンダー名と Cloudflare の識別子を書かない。ストレージの名前・アカウント ID・ホスト名を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 個人の値(パス・ユーザー名)を findings に書かない。ログのパスは `<path>` に置き換える。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35 分。受け入れのコマンドが、ビルドと 1 回目の走査を含む。ビルドは、このコマンドの 1 回で足りる。道具を直すために engine を作り直すのは、必要な時だけにする。
- 走査は予算 50 分。シェルツールの `timeout` 引数に 5400000 を渡し、フォアグラウンドで直接実行する。背景で起動して、終わるのを待たない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- この Issue は、停止を集めるところまで。個々の停止の直し(#1382・#1386・#1387・#1365・#1384 など)は、別の Issue で行う。この Issue は、それらを直さない。
- プラグインの instantiate(f)は、#1371 の道具が在れば、それを使う。無ければ、メニューの操作で行う。
