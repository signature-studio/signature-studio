---
draft: "167"
title: "RegOpenKeyExW などの、鍵の経路に空の要素(先頭・末尾・連続の区切り)が在る呼び出しで、engine を止めない: Windows の返り値を返し、鍵の名前を記録する"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["verify"]
size: S
priority: Urgent
project: signature-studio/3
---

## 目的

鍵の経路に空の要素(先頭の `\`、末尾の `\`、連続の `\\`)が在る呼び出しで、engine を止めない。Windows が返す値を返し、どの鍵の経路かを、警告の 1 行で記録に残す。

この Issue の目標は次の 3 つ。
1. `RegOpenKeyExW`・`RegOpenKeyExA`・`RegCreateKeyExW`・`RegDeleteKeyW` の、空の要素を含む経路で、止まらず、資料で確かめた返り値を返すこと。
2. 1 経路につき 1 回、鍵の経路(先頭 120 字)を含む警告の行を出すこと。次の実機の記録で、どの鍵かが分かるように。
3. 同じ種類の停止(`lift/runtime/` の `fle_stop` の全箇所)を一覧にし、直す物と直さない物を分けること。直すのは、この Issue の鍵の経路の分だけ。

## 背景

- 実機の記録(`pm-work/findings/real-user-stops-2026-11-25.md` の 3.2。#1404 の集計。全て読むこと)。
  - 版 `2578ea1e23f6` で、2 セッション・17 記録。停止の文言は次の 1 行で、終わり方は SIGABRT。起動から 308 秒と 117 秒で止まった。
    `fle: RegOpenKeyExW: an empty component (leading, trailing or doubled backslash) in a key path, reached after the keys before it exist`
  - 呼び出し元の戻り番地は両方とも `0x5af85c`。**鍵の名前は記録に無いので、どの鍵かは分からない**。このため、警告の行が要る。
- 該当のコード(`lift/runtime/fle_reg.c`。コードの読み。実測ではない)。
  - `open_common` の、鍵の経路を先頭から辿る loop の中(341 行付近)が、この文言を出す。直前の注記(338–340 行)は、決定 0099 の暫定で、「Windows がここで何を返すかは、どのページにも無い」と書く。
  - `split_key`(280–297 行)の 288 行も、空の要素で止める(`allow_empty` が 0 の時)。`split_key` を呼ぶのは、`fle_imp_RegCreateKeyExW`(431 行。呼び出しは 464 行)と `fle_imp_RegDeleteKeyW`(928 行。呼び出しは 946 行)。
  - 今の順序: 空の要素より前の要素が無ければ、その時点で `ERROR_FILE_NOT_FOUND`(2)を返す。空の要素に着いた時だけ止まる。
- 資料の読み(作業者が確かめる。ここに書くのは、この Issue を作る時に読んだ物)。
  - Microsoft の旧サポート記事(題名「PRB: RegCreateKeyEx() Gives Error 161 Under Windows NT」): 先頭の `\` は、Windows NT 3.1 と Windows 95 では許されたが、NT 3.5 以降は許されず、エラー 161(`ERROR_BAD_PATHNAME`)になる、とする。
  - Raymond Chen の記事(題名「Why does RegOpenKey sometimes (but not always) fail if I use two backslashes instead of one?」、2012 年): 二重の `\` は、鍵の経路として正しくない。古い探索は、空の副鍵名の段で拒否する。報告の値は `ERROR_INVALID_ARGUMENT` と書かれている(値そのものは確かめていない)。キャッシュの経路は、区切りをまとめて通す。Windows Vista で両者の食い違いは解消された、とあるが、どちらに揃えたかは書かれていない。
  - 利用者の報告(Stack Overflow の複数の質問。題名に「error 161」を含む物)では、先頭の区切りで 161 になり、末尾の区切りでも 161 になる、とする物がある。
  - まとめ(見込み)。空の要素に着いた時の返り値は、**161(`ERROR_BAD_PATHNAME`)が有力**。前の要素が無ければ、今と同じ 2。末尾の区切りが通るかは、資料では確かめられていない(報告の多くは 161 とする)。作業者は、Wine のテスト(`dlls/advapi32/tests/registry.c` の、先頭・末尾・連続の区切りの期待値)などで確かめ、確かめた値を返す。確かめ切れない時は 161 を返し、確かめ切れなかったことと、その根拠を報告に書く。
- 既存の試験: `lift/crates/fe-lift/tests/reg.rs` の `undefined_input_stops_the_run`(543 行から)の `open-leading-backslash`・`open-trailing-backslash`(559・560 行)は、止まることを期待している。直すと、この 2 件は返り値の試験に替える。

## 読む物

- `lift/runtime/fle_reg.c`(全体。特に 276–303 行の `kpath`・`split_key`・`ret`、305–350 行の `open_common`、431 行付近の `fle_imp_RegCreateKeyExW`、928 行付近の `fle_imp_RegDeleteKeyW`)
- `lift/runtime/fle_env.c`(10–14 行の `fle_stop`。警告の出し方を合わせる)
- `lift/crates/fe-lift/tests/reg.rs`(`undefined_input_stops_the_run` の形と、`assert_aborts` の使い方)
- `pm-work/findings/real-user-stops-2026-11-25.md`(3.2)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 資料で、空の要素の返り値を確かめる(上の見込みを検証する)。確かめた資料は、題名と年で報告に書く。
2. **試験を先に、red で書く**(`reg.rs`)。先頭・末尾・連続の区切りで、止まらず、決めた返り値を返すこと。前の要素が無い時は 2 を返すこと。`open-leading-backslash`・`open-trailing-backslash` の期待を、返り値の試験に替える。`RegCreateKeyExW`・`RegDeleteKeyW` の空の要素も、同じ試験に入れる。
3. 直す。`open_common` の 341 行付近と、`split_key` の 288 行の停止を、決めた返り値(`ret(...)`)にする。呼び出しごとの返り値を、コードの注記に 1 行で書く。決定 0099 の注記も、確かめた資料に合わせて直す。
4. 警告。1 経路につき 1 回、次の形の 1 行を stderr に出す(止めない)。
   `fle: warn: RegOpenKeyExW: empty component in "<鍵の経路の先頭 120 字>" (caller 0x…)`
   - `fle_stop` と同じ `fle:` の形にし、`warn` の語を入れる。
   - 同じ経路の 2 回目は出さない(静的な集合で覚える)。
   - 呼び出し元の戻り番地は、`fle_call_guest` の境界の外なら、その値を出す。
5. **同種の停止の一覧**。`grep -rn 'fle_stop(' lift/runtime --include=*.c` の全て(この時点で 1,396 行)を、次の 2 つに分ける。
   - (a) 内部の不変条件(止めてよい)。
   - (b) Windows の挙動が未確認で、暫定として止めている(この Issue では直さない)。
   - `pm-work/findings/runtime-stop-sites.md` に表で出す。列は、ファイル・行・API・止める条件・分類・Windows の挙動の見当(分かれば)・実機の記録に出た回数(`real-user-stops-2026-11-25.md` の指紋に一致する物があれば)。
   - 表は、全ての `fle_stop` の箇所を覆う。まとめる時は、行を並べて書く。
   - **直さない**。
6. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift --test reg && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node -e 'const t=require("fs").readFileSync("pm-work/findings/runtime-stop-sites.md","utf8");process.exit(t.split("\n").filter((l)=>/^\|/.test(l)).length>=3?0:1)' && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle で、`elapsedMs` が 12,500 以下。
- `reg` の試験が全て通る(新しい試験を含む)。空の要素で止まらない。
- 流れ A〜D の 4 行が在り、各行が `error` null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。流れ B と D は `welcomeClosed` が true。4 行の `preBootBytes` の最大が 314,368,711 以下。
- `npm test --prefix app` が通る。
- `pm-work/findings/runtime-stop-sites.md` が在り、表の行(`|` で始まる行)が、見出しと区切りを除いて 1 行以上ある。
- `bash guard/scan.sh` が 0 件。
- 判定の `node -e` は、合成データ(通る組と落ちる組)で先に確かめてから使う。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く。
  1. 空の要素の返り値: 確かめた資料(題名と年)と、採った値。確かめ切れなかった時は、その根拠。
  2. 試験の red と green の出力の行。
  3. 警告の行の出力例(1 経路につき 1 回であること)。
  4. `runtime-stop-sites.md` の行数と、分類の内訳((a) と (b) の件数)。
  5. 変えたパス(`lift/` を変えると main の deploy が動く)。
  6. 流れ A〜D の 4 行と、`elapsedMs`・`preBootBytes` の値。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 警告は既定の動作として出す。オプトインにしない。
- ベンダー名・Cloudflare の識別子・URL を書かない。資料は、題名と年で書く。
- Python と OCR は使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 4 時間 30 分で、受け入れが通らなくても、作業ツリーを commit して、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 起動の間欠の停止や CDP の timeout で流れ A〜D が落ちたら、その行を報告に書いて commit して終える(再試行で続く)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- `blocked_by` は無い。
- `#{{166}}`(枠の探索の例外)とは、`lift/runtime/fle_seh.c` と `fle_reg.c` で触る場所が違う。
- `runtime-stop-sites.md` は、後で停止の見直しをする時の入力になる。この Issue では、表を作るまでで直さない。
