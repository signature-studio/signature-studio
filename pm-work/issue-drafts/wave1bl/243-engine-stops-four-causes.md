---
draft: "243"
title: "製品: エンジンが止まる 4 系統(外部の取得の失敗・Sytrus の demo 2 本・Edit > #3 とその乱操作・書き出しの形式)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "133"
blocked_by: []
labels: ["verify"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

全シナリオの E2E(`e2e-full` の最新の run。commit 24aeee99 の版)で、エンジンが止まった(終了コード 134 または 5)本が 8 本ある。原因は 4 系統で、どれも別の場所にある。この Issue は、4 系統をそれぞれ直して、同じ本が止まらずに走ることを確かめる。エンジンが止まると、再生の音も止まる(P3)。製品(`app/`・`lift/`)を直す Issue なので、直す場所は原因の場所に限る。

## 背景

系統ごとに、件数・本の ID・止まった時の報告の文面(実測)・見立て(推定)を書く。

- **系統 A: 外部の画像の取得が失敗すると止まる(3 本)**: `resilience/random-ops-05`・`resilience/random-ops-07`・`resilience/random-ops-09`(いずれも印 `new`。止まった後、道具が engine を起こし直して続けた)。止まった時の報告: `act1-net #5 InternetOpenUrlW scheme=https ... object=<パス>.png ... -> NULL, last error 12007 (decision 0132)`(終了コード 134。`returned from entry`)。見立て(推定): 外部の画像の取得が失敗した(12007)時、呼び出し元が NULL の返り値を扱わずに止まる。実行環境で、外部への接続が失敗したことが、きっかけと見ている。製品の側では、取得の失敗で止まらないことが必要。実行環境の接続の有無は、確認できない。
- **系統 B: Sytrus の demo 2 本を開くと止まる(2 本・印なし)**: `file/demo-product-demos-olbaid-sytrus-essentials`・`file/demo-product-demos-synth-dfr-overcome-sytrus`(DPR 1。終了コード 134。報告の最後の行: `fle: more than 32768 live critical sections`)。見立て(推定): demo を読む時、Sytrus の関連で、重要区間(critical section)の数が上限を超える。前回の run では、別の demo 2 本(`Vocodex`・`Mehran Abbasi - With You`)でも止まっており、同じ種類の問題と見ている(`#1420` は Mehran の Elastique の間接呼び出しで、別の原因)。
- **系統 C: Edit > #3 で止まる(1 本・印なし)と、乱操作で同じ報告が出る(1 本・印 `new`)**: `menus/edit-3-choose`(終了コード 134)と `resilience/random-ops-01`(終了コード 134)。報告: `fle: DefWindowProcW: message 0x304: the default processing is not answered (decision 0071: WM_NCCREATE, WM_NCCALCSIZE with wParam FALSE, WM_CREATE; decision 0119: WM_SETICON, WM_SETTEXT; decision 0123: the window messages)`。見立て(推定): Edit > #3 で開く窓(ドロップダウン)の作成の途中で、メッセージ 0x304 が既定の処理に渡り、その応答が無い。`menus/edit-3-choose` では、ドロップダウンが開いた後に止まった。
- **系統 D: File > Export の最後の形式で止まる(1 本・印 `new`)**: `file/export-last-format`(終了コード 5)。報告: `fle: pthread_mutex_lock: the mutex is held by this guest thread (taken through the wrapper): the libc's mutexes are not recursive, so a release was skipped`。見立て(推定): 同じ mutex を、同じスレッドが二度取り、解放が飛んだ。
- 関連: `file/z-persist-recent-list`(印 `#1409`、閉じている)も、この run で止まった(状態が `error`、報告の本文は無い)。この Issue には入れず、file 領域の Issue で扱う。
- 確認できない: 実行環境の接続が、系統 A の原因か。この run では、画像もエンジンのログも取っていない(`--engine-logs` を付けていない)。

## 読む物

- 系統 A: `lift/` と `app/` の外部取得の経路(`InternetOpenUrlW` の呼び出しの側と、戻り値の扱い)。`tools/e2e/lib/proxy.mjs`(外部への通信の扱い。読むだけ)。`tools/e2e/scenarios/resilience/random-ops-05.mjs` など(印 `new`)。
- 系統 B: `tools/e2e/scenarios/file/demo-product-demos-*.mjs`・`tools/e2e/lib/file.mjs`(`openDataFile`)。製品の demo の読み込みの経路。`pm-work/` の既存の記録(demo の停止の報告)。
- 系統 C: `tools/e2e/scenarios/menus/edit-3-choose.mjs`・`lib/menus.mjs`(`chooseItem`)。`lift/` の DefWindowProcW の経路(`decision 0071`・`0119`・`0123` の記録)。
- 系統 D: `tools/e2e/scenarios/file/export-last-format.mjs`。製品の書き出しの形式の経路(`pthread_mutex_lock` の呼び出し元)。
- 各系統の停止の報告は、`tools/e2e/lib/stop.mjs` の出力の形(`engineStop`)で読める。

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 系統ごとに、先に再現する: `node tools/e2e/run.mjs --only <ID> --displays 4k-dpr1 --engine-logs <DIR> --budget 30m` を、ビルドの後に、フォアグラウンドで走らせ、止まった時のエンジンのログを保存する(`<DIR>` は `.tmp-*`)。
2. 系統ごとに、止まる直前の呼び出しを特定する。製品の側の原因の場所を、報告に書く。
3. 系統ごとに直す。直し方は、止まらないことと、結果が正しいことの両方を満たす物にする。止まらないためだけに、結果を捨てない。`lift/` の変更は、そのまま `fe-build` の検査(`cargo test`・既存の試験)を通す。
4. 系統 A は、取得の失敗を、呼び出し元が扱える戻り値で返す(実行環境の接続が無くても止まらない)ことを、試験で確かめる。
5. 系統 B・C・D は、直した後に、該当の本が止まらないことを確かめる。直した後は、その本の印を外すか、`new` の印を Issue の番号(この Issue の番号)に変える(報告に書く)。
6. 受け入れを通す。通らない系統は、落ちた ID・表示・止まった報告の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --only resilience/random-ops-01,resilience/random-ops-05,resilience/random-ops-07,resilience/random-ops-09,menus/edit-3-choose,file/demo-product-demos-olbaid-sytrus-essentials,file/demo-product-demos-synth-dfr-overcome-sytrus,file/export-last-format --displays 4k-dpr1 --budget 60m --report-only > "$RUNNER_TEMP/stops.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);process.exit(L.every((r)=>r.error===null)&&S.length===8&&S.every((s)=>s.engineStops===0&&s.engineStuckRunning!==true&&s.skipped!==true&&s.error===null)?0:1)' "$RUNNER_TEMP/stops.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる(8 本すべてで、エンジンの停止が 0、skip が無い、error が無い)。
- `lift/` を変えた場合、`cargo test` が通る(該当の crate)。
- 系統 A の取得の失敗の試験が、直す前に落ち、直した後に通る(報告に、両方の行を書く)。
- 報告に: (a) 系統ごとの、止まった場所(呼び出し元と、止まる直前の関数)、(b) 系統ごとの直し方と、変えたファイル、(c) 8 本の出力の行(ID・ok・止まったか)、(d) 直せなかった系統と、その理由(確認できない物は「確認できない」)。
- 直した本の印は、報告に一覧で書く(外した・Issue の番号に変えた・`new` のまま)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 系統ごとに、やること 0〜6 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- 直すのは、止まる原因の場所だけにする。結果を変えずに止まりを隠す(例: 例外を握りつぶす・判定を外す)直し方はしない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- ビルドや E2E の長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- ビルドは 1 回にする。直した後の確かめは、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせる。
- 1 回で終わらない前提で、系統ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。`git add` は 1 つずつ、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- #1509(resilience の印の見直し)と #1420(Mehran の demo の Elastique)は、別の停止の原因。この Issue の受け入れには入れない。
- `resilience/random-ops-*` の印の扱いは、#1509 が見直す。この Issue では、止まりを直すことと、印の番号を正しく付けることだけを扱う。
