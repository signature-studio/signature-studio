---
draft: "240"
title: "E2E の audio: Wave の書き出しが保存されず、その後の再生・画の判定が 22 本落ちる(窓の後始末と、保存されない原因の切り分け)"
type: Task
milestone: "M1 描画と音声の分離(P3)"
parent: "133"
blocked_by: []
labels: ["verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

`audio`(84 本)の E2E で、印の無い失敗が 35 件ある。そのうち再生・画の判定の 22 件は、同じ shard の前にある Wave の書き出し(`audio/export-wav-*`)が保存されずに失敗した後に起きている。この Issue は、(1) 失敗した書き出しを後始末して、後続の判定が正しく読める状態にする、(2) 書き出しが保存されない原因を、製品か道具かに切り分ける。音の判定は P3 に直結するので、先に切り分ける。製品(`app/`・`lift/`)は、原因が製品と分かるまで変えない。

## 背景

- 全シナリオの E2E(`e2e-full` の最新の run。commit 24aeee99 の版)の結果(実測):
  - 書き出し 2 本(`audio/export-wav-length`・`audio/export-wav-nonsilent`)は `export none: not valid: no file was saved within 90000 ms`。90 秒待っても保存されなかった。
  - 同じ shard の後続で、再生の判定 22 本が落ちた(`the transport runs (time display moves): false`・`playing 6000 ms (transport false)`・`Space to the first non-silent quantum none ms`・`pause-resume`・`resume-after-stop`・`song-mode-plays` など)。
  - 画の判定 4 本は `picture changed false`(`loop-button-toggle`・`metronome-button-toggle`・`pattern-song-toggle`・`tempo-value-changes`)。
  - 書き出しの前後で差がある(実測): audio 2/3 の shard では、書き出しの前の `audio/device-playing-on-play` は transport が動いて合格した。書き出しの後の再生の判定は全部落ちた。書き出しを含まない audio 3/3 の shard では、同じ種類の判定がほぼ全部合格した。
  - 推定: `tools/e2e/lib/audio.mjs` の `exportWav` は、保存されなかった時、File > Export > Wave の窓(`TWAVRenderForm`)を閉じずに返す。窓が残って、以後の Space とクリックが製品に届かないと見ている。窓の表は記録されていないので、確認できない。
  - 取り込みが 0 件の 5 本: `audio/repeat-4096`・`repeat-8192`・`repeat-16384`・`repeat-32768`(`equal share null over 0 comparisons`・`capture valid null`)と `audio/finite-samples`(`non-finite samples null in null frames`)。
  - 音量の判定 2 本: `audio/master-fader-down-200`・`audio/master-fader-up-60`(`peak undefined -> undefined`)。
- 内訳: audio 1/3 が 18 件、audio 2/3 が 17 件(予算切れの skip 3 件を含む)。audio 3/3 は 0 件。
- 確認できない: 書き出しが保存されない原因(製品の書き出しの動きか、Start の位置か)。この run では画像もエンジンのログも取っていない(`--shots`・`--engine-logs` を使っていない)。

## 読む物

- `tools/e2e/lib/audio.mjs`: `exportWav`・`playStart`・`playStop`・`isPlaying`・`CHECKS` の `export-wav`・`play-rise`・`repeat` 系・`finite` 系
- `tools/e2e/lib/common.mjs`: `resetProject`・`closeNew`・`windowTable`
- `tools/e2e/scenarios/audio/*.mjs`(宣言・`covers`。書き出しの 2 本と、後続の再生の判定)
- `tools/e2e/lib/picture.mjs`・`tools/e2e/lib/detect.mjs`
- 製品: Wave の書き出しの窓(`TWAVRenderForm`)の動き(読むだけ)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 再現: ビルドの後、`node tools/e2e/run.mjs --area audio --shard 1/3 --displays 4k-dpr1 --engine-logs <DIR> --shots <DIR> --budget 35m` をフォアグラウンドで走らせる(`<DIR>` は `.tmp-*`)。書き出しの前後の窓の表とエンジンのログを保存し、書き出しの後に `TWAVRenderForm` が残っているかを確かめる。
2. 切り分け: 保存されない原因を、窓の表とログから確かめる。Start の位置(窓の矩形からの計算)が道具の誤りなら、道具を直す。製品の書き出しが動かないなら、製品の欠陥として報告に書く(製品の Issue は、この Issue では起票せず、報告に書く)。
3. 後始末: 書き出しが保存されずに終わった時、`TWAVRenderForm` を閉じ、窓の表で残っていないことを確かめてから返す。`exportWav` の結果に、閉じた後の `leftOpen` を必ず書く。
4. 後続の判定: 書き出しが失敗した後も、再生・画の判定が正しく読めることを、同じ shard で確かめる。
5. 取り込みの 0 件(`repeat-*`・`finite-samples`)の原因を確かめる。道具の欠陥(取り込みの開始の位置・待ち時間)なら直す。製品なら報告に書く。
6. 受け入れを通す。通らない時は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --area audio --shard 1/3 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/audio13.jsonl" && node tools/e2e/run.mjs --area audio --shard 2/3 --budget 35m --displays 4k-dpr1 --report-only > "$RUNNER_TEMP/audio23.jsonl" && node -e 'let n=0;for(const p of process.argv.slice(1)){const L=require("fs").readFileSync(p,"utf8").trim().split("\n").map((l)=>JSON.parse(l));for(const r of L){if(r.error!==null)process.exit(1);n+=r.scenarios.filter((s)=>s.expectFail===null&&s.ok!==true&&s.skipped!==true).length}}process.exit(n<=6?0:1)' "$RUNNER_TEMP/audio13.jsonl" "$RUNNER_TEMP/audio23.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる。
- audio 1/3 と 2/3 の run で、印の無い失敗(skipped を除く)が合計 6 本以下。
- 書き出しの 2 本の後で、再生の判定(`transport`・`playing`・`pause-resume`・`resume-*`・`song-mode-plays`)が、書き出しの前と同じ結果で読める(保存されない場合でも、後続が transport を読める)。
- 報告に: (a) 窓の表の前後の差(`TWAVRenderForm` が残ったか。後始末の後に残らないこと)、(b) 保存されない原因の切り分け(道具か製品か。根拠のログの行)、(c) 2 つの shard の出力の行(ID・ok・notes)、(d) 取り込みの 0 件 5 本の原因。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜6 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は、原因が製品と分かった場合でも、この Issue では直さない(報告に書く)。`.github/`・`spec/`・`ci/` は、直す物がある時だけ変える。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
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
- ビルドは 1 回にする。tools だけを直した後は、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせて直す。
- 1 回で終わらない前提で、段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。`git add` は 1 つずつ、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- この Issue は、#1509(topbar・page・resilience・perf)と、#1507(windows・settings)と、ファイルが重ならない。
- 製品の停止(エンジンが止まる 4 系統)は、#{{243}} で扱う。この Issue では、停止を見つけたら報告に書くだけにする。
