---
draft: "116"
title: "回帰: Welcome 窓を閉じる(No thanks・Escape・Default の選択)と engine が trap する。ホストのファイルの解放の後の読みを直す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "render", "verify"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

最優先の回帰。利用者が Welcome 窓を閉じると、engine が trap し、FL が使えなくなる可能性がある(本番。推定。本番での実測は無い)。

この Issue の目標は次の 2 つ。
1. Welcome 窓を閉じる全ての流れ(確認ダイアログの「No thanks」・Escape・Welcome の Default の選択・確認ダイアログの「I'm in」)で、engine が trap せず、プロジェクトが開くこと。
2. 起動の時間と、起動前に取る量を、増やさないこと。

## 背景

- 事実 1(`#1353` の試行。run 45148253436。報告の「4. Debug ログ・F10 が届かない理由」): Welcome 窓を閉じる操作が、engine を trap させた。
  - 「No thanks」(DPR 1 の座標 (993, 548)): `HostFilesUnsupported: ... pread on descriptor 20, which was never opened or has been closed`。
  - Escape: `HostFilesUnsupported: ... opening .../Data/Templates/Minimal/Basic with limiter/Basic with limiter.flp after the host files were released`。
- 事実 2(同じ試行): Welcome 窓が開いている間は、F10 で何も開かない(モーダル)。
- 事実 3(`#1311` の測定。02:25Z の実行。`tools/feat-c6-popup-menus/measure.mjs`): 「No thanks」の後に Welcome の Default の行をダブルクリックすると、プロジェクトが開いた。その時の main では動いた。
- 推定(実測で確かめる): 事実 3 の後に main に入った変更のどれかが、ホストのファイルを解放した(`host_files_released`。起動の最初の idle の直後)後に読む経路を作った。候補は、`#1345`(`6698a7bd`・`dc21310c`。seed の取り込みを、本体の初回の読みまで遅らせる形)、`#1341`(`ecb300bb`。像の読みを要求の時まで遅らせた)、`#1347` の変更のどれか。
- 仕組み(コードの読み): `app/src/engine/hostfiles.ts` の `release()` は、永続(persistent)でないファイルの source を閉じ、その descriptor を閉じる(`closeWhere`)。解放の後に、そのパスを open すると `HostFilesUnsupported`(`knownPaths`)、閉じた descriptor を pread すると `HostFilesUnsupported` になる。`app/src/engine/fruity.ts` の `fleIdle`(n === 1)が、最初の idle で `release()` を呼ぶ。設計の前提は、読みは全て最初の idle の前に終わる(decision 0221。`hostfiles.ts` の注釈)。
- 永続のファイル(`addPersistent`。`#1314`・decision 0314)は、解放の対象から外れる。`@lazy-file:` の仕組み(`app/src/engine/lazy-file.ts`・`lazy-tree.ts`)は、起動の後の取得に使える。

## 読む物

- `app/src/engine/hostfiles.ts`(`release()`・`knownPaths`・`addPersistent`・`overrides` の `owned()`・`openat`・`closeWhere`)
- `app/src/engine/fruity.ts`(`fleIdle` と `host_files_released`、`preRun` の `mountPack`)
- `app/src/engine/lazy-file.ts`・`app/src/engine/lazy-tree.ts`(起動の後の取得)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(`Data\Templates\Minimal\Basic with limiter\Basic with limiter.flp` の行)
- `tools/feat-c6-popup-menus/measure.mjs`(Welcome の「No thanks」と Default の行の操作。固定の座標で書かれているので、窓の表から取る形に直す)
- `tools/boot-check/boot-bundle.mjs`・`tools/boot-check/cdp.mjs`
- `git log --oneline 2b528414..HEAD -- app/src lift/runtime lift/harness`(`#1311` の後の変更)、`git show 6698a7bd dc21310c ecb300bb`
- `pm-work/findings/topbar-parts-1311.md`(`#1311` の測定の記録)

## やること

0. 再現と測定の道具 `tools/smoke/welcome-flow.mjs`(新規)。
   - 起動して、DPR 1(部屋 1836×1056 CSS px)で、流れごとに起動し直す(各流れは、起動の最初の idle から始める)。座標は固定にせず、窓の表(`window.__app` の窓の表。lid・位置・題名)と、確認ダイアログの表示から取る。
   - 流れ A: 確認ダイアログ「Help us improve …」の「No thanks」→ Welcome の Default の行をダブルクリックして、プロジェクトを開く。
   - 流れ B: Escape で Welcome 窓を閉じる。
   - 流れ C: 確認ダイアログを「No thanks」で閉じた後、Welcome の Default の行を 1 回クリックして開く。
   - 流れ D: 確認ダイアログの「I'm in」で閉じる(Welcome 窓が閉じ、プロジェクトが開くまで)。
   - 各流れは、engine が idle に戻る(最長 120 秒)まで待ち、次の値を 1 行の JSON にする(stdout に、流れ A・B・C・D の順で 4 行)。
     - `flow`("A" など)・`error`(`engine-idle` に戻った後の error。無ければ null)・`state`・`standIn`
     - `trapped`: engine が idle に戻らない、または log に trap・abort の行がある時 true
     - `hostErrors`: 流れの間の `HostFilesUnsupported` の行の数
     - `welcomeClosed`: 流れの後、Welcome 窓が窓の表から消えた時 true
     - `opened`: 窓の題名が、Welcome の時の題名から変わった時 true
     - `elapsedMs`: 起動(navigation)から最初の `engine-idle` まで(`#1355` の道具の定義と同じ)
     - `preBootBytes`: `window.__app.assetsReport().items` の bytes の合計
   - 流れ D の操作の順序は、窓の表で確かめ、報告に書く。
1. 原因の特定。`HostFilesUnsupported` の 2 つの行(descriptor の番号・open のパス)を engine の log から出し、その descriptor が何のファイルか、どの呼び出し元(FL の手続き)が読んだかを、trace で特定する。`git show` で、解放の後に読みを作った変更を確かめる。実測と推定を分けて報告に書く。
2. 直す。解放の後に読まれる物は、次のどちらかにする。製品の既定の動作として直す(オプトインを作らない)。
   - (a) 解放の前に、VFS の中(メモリ)へ写し、解放の対象から外す(永続にする)。
   - (b) 起動の後の取得(`@lazy-file:` の仕組み)で、guest の読みの時に取る。取得の間は guest を待たせる(`Atomics.wait` の形)。
   - ホストのファイルの descriptor を、解放の後に使わない。閉じない descriptor を解放の対象から外すか、解放の後の読みを正しく応答する。
   - どちらを選んだかと、その理由を報告に書く。
   - 起動前に取る合計(`preBootBytes`)は、基準に 2 MB(2,097,152 B)を足した値を超えない。起動の時間(`elapsedMs`)は、基準に 500 ms を足した値を超えない。基準は、`#1355` の測定の値(`elapsedMs` 7751、`preBootBytes` 314,326,602。runner の値)。`elapsedMs` は負荷で揺れる(`#1355` の 2 回目は 7797、負荷の高い回は 10716)ので、判定は 4 流れの `elapsedMs` の中央値で行う。
3. 測定の道具 `tools/smoke/welcome-flow.mjs` で、流れ A〜D を全て測る。直す前と後の値を、同じ道具で報告に書く。
4. 報告に、特定した原因の変更の commit、直した方式、流れ A〜D の JSON の 4 行(そのまま貼る)、`elapsedMs` の 4 回の値と中央値、`preBootBytes` の値を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=8251&&pb<=316423754?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(DPR 1。流れ A〜D の全て):
- 4 行が在る(流れ A・B・C・D)。各行が、`error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。
- 流れ B と D は、`welcomeClosed` が true。
- 4 流れの `elapsedMs` の中央値が 8251 以下(基準 7751 + 500 ms)。
- `preBootBytes` の最大が 316,423,754 以下(基準 314,326,602 + 2 MB)。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 原因の特定(descriptor・パスと、どの呼び出し元が読んだか、どの変更が解放の後の読みを作ったか)、直した方式と理由、流れ A〜D の JSON の 4 行、`elapsedMs` と `preBootBytes` の変更の前後の値。
- 「直った」と書かず、流れ A〜D の結果で書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 直しは、既定の動作として入れる。測定のためだけに既定の動作を変えない(`tools/smoke/welcome-flow.mjs` の測定は、製品の既定の動作を変えない)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35 分。1 回の試行で、ビルドは数回まで。実験は、記録と trace をまとめた 1 回のビルドで行う。
- ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- `#{{117}}`(F10 の General settings)は、この Issue の後に行う(Welcome 窓が閉じられないと、F10 が届かない)。
- `#{{113}}`(拡大率の測定)と、`app/src/engine/` の変更が重なりうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
