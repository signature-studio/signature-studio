---
draft: "185"
title: "ファイル選択の回答を待つ間も、ゲストの他のスレッド(音声のミキサー)が回り、回答が届く: 選択の待ちを、ホストへ制御を返す協調的な待ちにする"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: ["180"]
labels: ["perf", "audio"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

P3(再生中に音が途切れない)。main の今の版では、ファイル選択(`IFileOpenDialog::Show`)は、ページが回答するまでゲスト全体を止める(JSPI で同期的に待つ)。その間、音声のミキサーも止まる。また、ゲストが Sleep だけで時間を進める時、ホストの回答の message が処理されず、回答が届かない。この Issue で、選択の待ちを、ホストへ制御を返す協調的な待ちに変え、回答が届くことと、他のスレッドが回ることを確かめる。

## 背景

- main の今の版(`lift/harness/web/filedlg/fle_filedlg_lib.js`): `fle_filedlg_host_pick__async: true`。`fle_filedlg_host_pick` は、回答が来るまで中断する(JSPI)。回答の文字列は、その中断から戻って読まれる。
- 旧版の実測(ブランチの報告の記述): 97.8 MiB の評価用ファイルを回答した後、回答は約 544 秒後にワーカーで処理され、転送は 1.3／97.8 MiB で停滞した。ゲストのスレッドの切り替えは microtask の連鎖で続き、ホストの message の処理へ戻らなかった。
- ブランチの版(`pm-work/findings/perf-windows-browser/patches/04-filedlg-nonsuspending-poll.patch`): `fle_filedlg_host_pick` は、回答を待たずに 0 を返す(要求は保留する)。新しく `fle_filedlg_host_poll`(JSPI の非同期)を足す。回答が未だなら、`setTimeout(0)` でホストの 1 つのタスクへ制御を返してから 0 を返す。C 側(`fle_filedlg.c`)は、回答が NULL の間 `fle_thread_sleep("filedlg", 1)` を呼びながら `poll` を繰り返す。ブランチの試験(`spec/filedlg-bridge.test.mjs`)は、保留・取消しの 1 回だけの消費・読込の完了の待ち・ホストへの制御の返却を確かめる。
- 効果の根拠: 実測(旧版の停滞と、修正後の全量読込・非無音出力の確認。ブランチの報告)。ただし、音声の欠落の増分は、小規模なプロジェクトのダイアログの 10 秒間では検出されなかった(報告の記述)。

## 読む物

- `lift/harness/web/filedlg/fle_filedlg_lib.js`(`fle_filedlg_host_pick`・`fleFileDlgAnswer`・`FleFileDlg`)
- `lift/runtime/fle_filedlg.c`(`browser_picker`・`fle_filedlg_host_pick` の呼び出し・回答の読み)、`lift/runtime/fle_thread.h`(`fle_thread_sleep`)
- `spec/filedlg-bridge.test.mjs`(main の版。ブランチの版の試験は、この行を写す形で足す)
- `pm-work/findings/perf-windows-browser/patches/04-filedlg-nonsuspending-poll.patch`(ブランチの版。参考)

## やること

1. `fle_filedlg_lib.js`: `fle_filedlg_host_pick` は同期で、回答を待たずに 0 を返す(`S.pending` を立て、要求を Module の `fleFileDlg` へ送る)。既に保留中の選択があれば、`CANCEL` を返す。回答(`fleFileDlgAnswer`)は、今のとおり、ファイルを読んで `S.pending.result` に文字列を置く。
2. `fle_filedlg_host_poll`(JSPI の非同期): 結果が未だなら、`setTimeout(0)` でホストの 1 つのタスクへ制御を返す。結果があれば、その文字列を返し、`S.pending` を消す。結果が無ければ 0。
3. `fle_filedlg.c` の `browser_picker`: `fle_filedlg_host_pick` の後、回答が NULL の間、`fle_thread_sleep("filedlg", 1)` を呼びながら `fle_filedlg_host_poll` を繰り返す。`fle_thread_sleep` が無い時は、今のとおり止めて理由を出す。
4. 試験: `spec/filedlg-bridge.test.mjs` に、保留と取消しの 1 回だけの消費、読込の完了の待ち、ホストへの制御の返却(ゲストの sleep が仮想時間だけを進める時でも回答が届く)を足す。`wall_fpick1.rs` を動かす。

## 受け入れ

```accept
rm -rf .tmp-base && mkdir .tmp-base && git archive "$(git merge-base HEAD origin/main)" | tar -x -C .tmp-base && (cd .tmp-base && npm ci --prefix app && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle) && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node --test spec/filedlg-bridge.test.mjs && cargo test --release --locked --manifest-path lift/Cargo.toml -p fe-lift --test wall_fpick1 && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-185-boot.json && node -e 'const r=JSON.parse(require("fs").readFileSync(".tmp-185-boot.json","utf8"));process.exit(r.error===null&&r.state==="engine-idle"&&r.standIn===false?0:1)' && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > .tmp-185-flow.jsonl && node -e 'const L=require("fs").readFileSync(".tmp-185-flow.jsonl","utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' && node tools/ab/run.mjs p3 "$PWD/.tmp-base/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-base/app" "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/app" 3 > .tmp-185-ab-p3.json && node -e 'const s=JSON.parse(require("fs").readFileSync(".tmp-185-ab-p3.json","utf8"));process.exit(s.pass===true?0:1)'
```

判定の基準:
- `spec/filedlg-bridge.test.mjs` と `wall_fpick1` が通る。
- 起動の確認と流れ A〜D が全て合格(受け入れのとおり)。
- `tools/ab/run.mjs` の `p3` が、回帰なしで `pass` が true(この Issue の操作は再生中のダイアログを含まない。回帰の確認だけ)。
- 残す条件: 試験が、回答の届くことと、保留中の選択の消費が 1 回だけであることを示す。これが満たされないなら、取り消す。
- 報告に書く: 試験の出力の行、流れ A〜D の 4 行、`p3` の 6 回の run の値と中央値。「達成」と書かず、「受理」か「差し戻し」を、出力の行で書く。
- 実機の評価用の大きいファイル(リポジトリに無い)での確認は、この Issue の受け入れに入らない。報告に、未確認と書く。

## 作業の進め方

- engine のビルドは約 35 分。基準(変更前)と変更後の 2 回の engine のビルドが要る(受け入れのコマンドに入っている)。フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して、直接実行する。背景で起動して、終わるのを待たない。
- 作業は開始から 4 時間 30 分以内に終える。最初に `date -u` で開始を記録する。その時刻を過ぎたら、受け入れが通っていなくても作業ツリーを commit して、報告を書いて終える。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python を使わない。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 評価用のプロジェクト・ファイルは、リポジトリに入れない。
- sleep・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `lift/` の変更は、push で main の `deploy.yml` を動かす。報告に書く。

## 他のタスクとの関係

- blocked_by: #{{180}}(計測の道具)。
- 衝突: #1409(Recent。`fle_filedlg` の系を触る。失敗している Issue のため、blocked_by にしていない。重なったら作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、直して、受け入れをもう一度通す)。
- `spec/filedlg-bridge.test.mjs` は、main に無い。この Issue で新しく作る。
